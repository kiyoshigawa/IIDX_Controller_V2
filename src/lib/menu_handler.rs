//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

use crate::{
    BUF_SIZE, ButtonCode, DEFAULT_BUTTON_DEBOUNCE_TICKS, FlashStoragePersistentMemory, NUM_BUTTONS,
    OledDisplay,
};
use core::cmp::min;
use core::fmt::Write;
use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_9X18_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text},
};

/// Y-offset at which the button layout graphic is drawn on the 64-px screen.
/// (63 − 26 − 2 + 1 = 36)
pub(crate) const BUTTON_GRAPHIC_ROW_HEIGHT: u8 = 36;

/// Maximum number of menu levels that can be nested on the stack.
const MAX_MENU_DEPTH: usize = 4;

/// Highest standard HID keyboard usage code cycled through in key binding editing.
const MAX_HID_KEY: usize = 101;

/// Visible character width of the OLED display with the 9px font.
const VISIBLE_WIDTH: usize = 13;

/// Y-origin of each of the 4 text lines on the OLED.
const LINE_Y: [i32; 4] = [0, 16, 32, 48];

/// Top-left corner of the frame-counter text in debug screen
const FRAME_COUNTER_POS: Point = Point::new(0, 0);
/// Left-middle position for the encoder 1 count in debug screen
const ENCODER1_LABEL_POS: Point = Point::new(0, 32);
/// Right-middle position for the encoder 2 count in debug screen
const ENCODER2_LABEL_POS: Point = Point::new(127, 32);

/// These are the 'on' pixels used for the wiki arrow graphic in the debug screen.
#[rustfmt::skip]
const ARROW_GRAPHIC_PIXELS: [(i32, i32); 12] = [
    (0,0), (1,0), (2,0),
    (0,1), (1,1),
    (0,2),        (2,2),
                        (3,3),
                            (4,4),
                            (4,5),
                                (5,6),
                                (5,7),
];

/// Anchor-point offsets for each wiki direction arrow
const P1_POSITIVE_ANCHOR: Point = Point::new(1, BUTTON_GRAPHIC_ROW_HEIGHT as i32 + 5);
const P1_NEGATIVE_ANCHOR: Point = Point::new(1, BUTTON_GRAPHIC_ROW_HEIGHT as i32 + 21);
const P2_POSITIVE_ANCHOR: Point = Point::new(126, BUTTON_GRAPHIC_ROW_HEIGHT as i32 + 5);
const P2_NEGATIVE_ANCHOR: Point = Point::new(126, BUTTON_GRAPHIC_ROW_HEIGHT as i32 + 21);

/// Binary pixel representation of the IIDX deck control layout.
#[rustfmt::skip]
pub(crate) const BUTTON_GRAPHIC: [u8; 16 * 26] = [
    0b00000000, 0b00000000, 0b00001111, 0b10000000, 0b00001111, 0b10000000, 0b01111100, 0b01111100, 0b01111100, 0b01111100, 0b00000001, 0b11110000, 0b00000001, 0b11110000, 0b00000000, 0b00000000, // Row 1
    0b11100000, 0b00000000, 0b00001000, 0b10000000, 0b00001000, 0b10000000, 0b01000100, 0b01000100, 0b01000100, 0b01000100, 0b00000001, 0b00010000, 0b00000001, 0b00010000, 0b00000000, 0b00000111, // Row 2
    0b00011000, 0b00000000, 0b00001000, 0b10000000, 0b00001000, 0b10000000, 0b01000100, 0b01000100, 0b01000100, 0b01000100, 0b00000001, 0b00010000, 0b00000001, 0b00010000, 0b00000000, 0b00011000, // Row 3
    0b00000110, 0b00000000, 0b00001000, 0b10000000, 0b00001000, 0b10000000, 0b01000100, 0b01000100, 0b01000100, 0b01000100, 0b00000001, 0b00010000, 0b00000001, 0b00010000, 0b00000000, 0b01100000, // Row 4
    0b00000001, 0b00000000, 0b00001111, 0b10000000, 0b00001111, 0b10000000, 0b01111100, 0b01111100, 0b01111100, 0b01111100, 0b00000001, 0b11110000, 0b00000001, 0b11110000, 0b00000000, 0b10000000, // Row 5
    0b00000000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000001, 0b00000000, // Row 6
    0b00000000, 0b01000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00000000, // Row 7
    0b00000000, 0b01000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000111, 0b11000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00000000, // Row 8
    0b00000000, 0b00100000, 0b00001111, 0b10001111, 0b10001111, 0b10000000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00000001, 0b11110001, 0b11110001, 0b11110000, 0b00000100, 0b00000000, // Row 9
    0b00000000, 0b00100000, 0b00001000, 0b10001000, 0b10001000, 0b10000000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00000001, 0b00010001, 0b00010001, 0b00010000, 0b00000100, 0b00000000, // Row 10
    0b00000000, 0b00010000, 0b00001000, 0b10001000, 0b10001000, 0b10000000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00000001, 0b00010001, 0b00010001, 0b00010000, 0b00001000, 0b00000000, // Row 11
    0b00000000, 0b00010000, 0b00001000, 0b10001000, 0b10001000, 0b10000000, 0b00000000, 0b00000111, 0b11000000, 0b00000000, 0b00000001, 0b00010001, 0b00010001, 0b00010000, 0b00001000, 0b00000000, // Row 12
    0b00000000, 0b00010000, 0b00001000, 0b10001000, 0b10001000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000001, 0b00010001, 0b00010001, 0b00010000, 0b00001000, 0b00000000, // Row 13
    0b10000000, 0b00010000, 0b00001000, 0b10001000, 0b10001000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000001, 0b00010001, 0b00010001, 0b00010000, 0b00001000, 0b00000001, // Row 14
    0b00000000, 0b00010000, 0b00001111, 0b10001111, 0b10001111, 0b10000000, 0b00000011, 0b11100111, 0b11001111, 0b10000000, 0b00000001, 0b11110001, 0b11110001, 0b11110000, 0b00001000, 0b00000000, // Row 15
    0b00000000, 0b00010000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00100100, 0b01001000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00001000, 0b00000000, // Row 16
    0b00000000, 0b00010000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00100100, 0b01001000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00001000, 0b00000000, // Row 17
    0b00000000, 0b00100000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000010, 0b00100100, 0b01001000, 0b10000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000100, 0b00000000, // Row 18
    0b00000000, 0b00100000, 0b11111000, 0b11111000, 0b11111000, 0b11111000, 0b00000011, 0b11100111, 0b11001111, 0b10000000, 0b00011111, 0b00011111, 0b00011111, 0b00011111, 0b00000100, 0b00000000, // Row 19
    0b00000000, 0b01000000, 0b10001000, 0b10001000, 0b10001000, 0b10001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010001, 0b00010001, 0b00010001, 0b00010001, 0b00000010, 0b00000000, // Row 20
    0b00000000, 0b01000000, 0b10001000, 0b10001000, 0b10001000, 0b10001000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010001, 0b00010001, 0b00010001, 0b00010001, 0b00000010, 0b00000000, // Row 21
    0b00000000, 0b10000000, 0b10001000, 0b10001000, 0b10001000, 0b10001000, 0b00000000, 0b00000111, 0b11000000, 0b00000000, 0b00010001, 0b00010001, 0b00010001, 0b00010001, 0b00000001, 0b00000000, // Row 22
    0b00000001, 0b00000000, 0b10001000, 0b10001000, 0b10001000, 0b10001000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00010001, 0b00010001, 0b00010001, 0b00010001, 0b00000000, 0b10000000, // Row 23
    0b00000110, 0b00000000, 0b10001000, 0b10001000, 0b10001000, 0b10001000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00010001, 0b00010001, 0b00010001, 0b00010001, 0b00000000, 0b01100000, // Row 24
    0b00011000, 0b00000000, 0b11111000, 0b11111000, 0b11111000, 0b11111000, 0b00000000, 0b00000100, 0b01000000, 0b00000000, 0b00011111, 0b00011111, 0b00011111, 0b00011111, 0b00000000, 0b00011000, // Row 25
    0b11100000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000111, 0b11000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000111, // Row 26
];

/// Static rectangle coordinates for each physical button's debug-indicator position.
#[rustfmt::skip]
pub(crate) const BUTTON_DEBUG_RECTANGLES: [Rectangle; NUM_BUTTONS] = [
    Rectangle::new(Point::new( 17, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 21, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new( 25, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 29, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new( 33, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 37, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new( 41, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 21, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 37, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 84, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 88, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new( 92, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 96, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new(100, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new(104, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  9), Size::new(3, 5)),
    Rectangle::new(Point::new(108, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 19), Size::new(3, 5)),
    Rectangle::new(Point::new( 88, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new(104, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 50, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 62, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  8), Size::new(3, 3)),
    Rectangle::new(Point::new( 62, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 22), Size::new(3, 3)),
    Rectangle::new(Point::new( 55, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 15), Size::new(3, 3)),
    Rectangle::new(Point::new( 69, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 15), Size::new(3, 3)),
    Rectangle::new(Point::new( 62, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) + 15), Size::new(3, 3)),
    Rectangle::new(Point::new( 58, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 66, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
    Rectangle::new(Point::new( 74, (BUTTON_GRAPHIC_ROW_HEIGHT as i32) +  1), Size::new(3, 3)),
];

/// Determines which rendering mode the OLED display is in, which effects how MenuEvents
/// will be handled.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuMode {
    Debug,
    PixelTest,
}

/// These are the events that can be called by outside sources to trigger
/// changes in the menu/display state.
pub enum MenuEvents {
    Press(ButtonCode),
}

/// Orientation to apply when drawing an encoder arrow graphic.
/// At the screen level: FlipX mirrors left↔right (P1 vs P2),
/// FlipY mirrors top↔bottom (Positive vs Negative).
#[derive(Clone, Copy)]
enum Flip {
    NoFlip,
    FlipX,
    FlipY,
    FlipXY,
}

/// Identifies a single configurable value in [`FlashStoragePersistentMemory`].
///
/// Decouples the static menu tree from the flash struct layout — the handler
/// translates each key to the appropriate field access internally.
#[derive(Clone, Copy, PartialEq)]
enum SettingKey {
    AllButtonDebounce,
    ButtonDebounce(ButtonCode),
    EncoderDebounce(usize),
    EncoderStepThreshold(usize),
    EncoderMoveTimeout(usize),
}

/// What happens when a menu option is activated
#[derive(Clone, Copy)]
enum MenuAction {
    OpenSubmenu(&'static MenuLevel),
    GoBack,
    /// Switch to a debug/test display mode (exit via specific keys).
    ShowDebugScreen,
    /// Switch to pixel-fill test mode (any key exits).
    ShowPixelTest,
    /// Enter in-place value adjustment for a named setting.
    EditSetting(SettingKey),
    /// Enter key-binding cycle for a physical button or encoder direction.
    EditKeyBinding(ButtonCode),
    /// Visible but non-functional — reserved for future features.
    None,
}

/// Top-level state machine controlling what the handler does with inputs
/// and how it renders the screen.
#[derive(Clone, Copy)]
enum MenuSubState {
    Browsing,
    EditingValue {
        setting: SettingKey,
        original_value: u32,
        working_value: u32,
    },
    EditingKeyBinding {
        button: ButtonCode,
        working_key_idx: usize,
        original_key_idx: usize,
    },
    DisplayMode(MenuMode),
}

/// Returns the [`Flip`] orientation required for a given encoder
/// [`ButtonCode`] so the arrow points in the correct direction.
fn flip_for(code: ButtonCode) -> Flip {
    match code {
        ButtonCode::P1Positive => Flip::NoFlip,
        ButtonCode::P1Negative => Flip::FlipY,
        ButtonCode::P2Positive => Flip::FlipX,
        ButtonCode::P2Negative => Flip::FlipXY,
        _ => Flip::NoFlip,
    }
}

static ROOT_MENU: MenuLevel = MenuLevel {
    title: "IIDX Menu",
    options: &[
        MenuOption {
            label: "Lighting",
            action: MenuAction::None,
        },
        MenuOption {
            label: "Settings",
            action: MenuAction::OpenSubmenu(&SETTINGS_MENU),
        },
        MenuOption {
            label: "Debug",
            action: MenuAction::OpenSubmenu(&DEBUG_MENU),
        },
    ],
};

static SETTINGS_MENU: MenuLevel = MenuLevel {
    title: "Settings",
    options: &[
        MenuOption {
            label: "Debounce",
            action: MenuAction::OpenSubmenu(&DEBOUNCE_MENU),
        },
        MenuOption {
            label: "Key Bindings",
            action: MenuAction::OpenSubmenu(&KEYBIND_MENU),
        },
        MenuOption {
            label: "Encoder Sensitivity",
            action: MenuAction::OpenSubmenu(&ENCODER_SENS_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static DEBUG_MENU: MenuLevel = MenuLevel {
    title: "Debug",
    options: &[
        MenuOption {
            label: "Debug Screen",
            action: MenuAction::ShowDebugScreen,
        },
        MenuOption {
            label: "Pixel Test",
            action: MenuAction::ShowPixelTest,
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static ENCODER_SENS_MENU: MenuLevel = MenuLevel {
    title: "Encoder Sens",
    options: &[
        MenuOption {
            label: "Enc1 Threshold",
            action: MenuAction::EditSetting(SettingKey::EncoderStepThreshold(0)),
        },
        MenuOption {
            label: "Enc1 Timeout",
            action: MenuAction::EditSetting(SettingKey::EncoderMoveTimeout(0)),
        },
        MenuOption {
            label: "Enc2 Threshold",
            action: MenuAction::EditSetting(SettingKey::EncoderStepThreshold(1)),
        },
        MenuOption {
            label: "Enc2 Timeout",
            action: MenuAction::EditSetting(SettingKey::EncoderMoveTimeout(1)),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static DEBOUNCE_MENU: MenuLevel = MenuLevel {
    title: "Debounce",
    options: &[
        MenuOption {
            label: "All",
            action: MenuAction::EditSetting(SettingKey::AllButtonDebounce),
        },
        MenuOption {
            label: "P1_1",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_1)),
        },
        MenuOption {
            label: "P1_2",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_2)),
        },
        MenuOption {
            label: "P1_3",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_3)),
        },
        MenuOption {
            label: "P1_4",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_4)),
        },
        MenuOption {
            label: "P1_5",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_5)),
        },
        MenuOption {
            label: "P1_6",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_6)),
        },
        MenuOption {
            label: "P1_7",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1_7)),
        },
        MenuOption {
            label: "P1Start",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1Start)),
        },
        MenuOption {
            label: "P1Select",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P1Select)),
        },
        MenuOption {
            label: "P2_1",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_1)),
        },
        MenuOption {
            label: "P2_2",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_2)),
        },
        MenuOption {
            label: "P2_3",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_3)),
        },
        MenuOption {
            label: "P2_4",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_4)),
        },
        MenuOption {
            label: "P2_5",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_5)),
        },
        MenuOption {
            label: "P2_6",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_6)),
        },
        MenuOption {
            label: "P2_7",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2_7)),
        },
        MenuOption {
            label: "P2Start",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2Start)),
        },
        MenuOption {
            label: "P2Select",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::P2Select)),
        },
        MenuOption {
            label: "Escape",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::Escape)),
        },
        MenuOption {
            label: "CcUp",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcUp)),
        },
        MenuOption {
            label: "CcDown",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcDown)),
        },
        MenuOption {
            label: "CcLeft",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcLeft)),
        },
        MenuOption {
            label: "CcRight",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcRight)),
        },
        MenuOption {
            label: "CcSelect",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcSelect)),
        },
        MenuOption {
            label: "VolumeUp",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::VolumeUp)),
        },
        MenuOption {
            label: "VolumeDown",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::VolumeDown)),
        },
        MenuOption {
            label: "Mute",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::Mute)),
        },
        MenuOption {
            label: "Enc1",
            action: MenuAction::EditSetting(SettingKey::EncoderDebounce(0)),
        },
        MenuOption {
            label: "Enc2",
            action: MenuAction::EditSetting(SettingKey::EncoderDebounce(1)),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static KEYBIND_MENU: MenuLevel = MenuLevel {
    title: "Key Bindings",
    options: &[
        MenuOption {
            label: "P1_1",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_1),
        },
        MenuOption {
            label: "P1_2",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_2),
        },
        MenuOption {
            label: "P1_3",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_3),
        },
        MenuOption {
            label: "P1_4",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_4),
        },
        MenuOption {
            label: "P1_5",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_5),
        },
        MenuOption {
            label: "P1_6",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_6),
        },
        MenuOption {
            label: "P1_7",
            action: MenuAction::EditKeyBinding(ButtonCode::P1_7),
        },
        MenuOption {
            label: "P1Start",
            action: MenuAction::EditKeyBinding(ButtonCode::P1Start),
        },
        MenuOption {
            label: "P1Select",
            action: MenuAction::EditKeyBinding(ButtonCode::P1Select),
        },
        MenuOption {
            label: "P2_1",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_1),
        },
        MenuOption {
            label: "P2_2",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_2),
        },
        MenuOption {
            label: "P2_3",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_3),
        },
        MenuOption {
            label: "P2_4",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_4),
        },
        MenuOption {
            label: "P2_5",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_5),
        },
        MenuOption {
            label: "P2_6",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_6),
        },
        MenuOption {
            label: "P2_7",
            action: MenuAction::EditKeyBinding(ButtonCode::P2_7),
        },
        MenuOption {
            label: "P2Start",
            action: MenuAction::EditKeyBinding(ButtonCode::P2Start),
        },
        MenuOption {
            label: "P2Select",
            action: MenuAction::EditKeyBinding(ButtonCode::P2Select),
        },
        MenuOption {
            label: "Escape",
            action: MenuAction::EditKeyBinding(ButtonCode::Escape),
        },
        MenuOption {
            label: "VolumeUp",
            action: MenuAction::EditKeyBinding(ButtonCode::VolumeUp),
        },
        MenuOption {
            label: "VolumeDown",
            action: MenuAction::EditKeyBinding(ButtonCode::VolumeDown),
        },
        MenuOption {
            label: "Mute",
            action: MenuAction::EditKeyBinding(ButtonCode::Mute),
        },
        MenuOption {
            label: "Enc1+",
            action: MenuAction::EditKeyBinding(ButtonCode::P1Positive),
        },
        MenuOption {
            label: "Enc1-",
            action: MenuAction::EditKeyBinding(ButtonCode::P1Negative),
        },
        MenuOption {
            label: "Enc2+",
            action: MenuAction::EditKeyBinding(ButtonCode::P2Positive),
        },
        MenuOption {
            label: "Enc2-",
            action: MenuAction::EditKeyBinding(ButtonCode::P2Negative),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

/// A tiny fixed-size buffer for formatting a single short line of text
/// (up to [`BUF_SIZE`] bytes) before drawing it to the OLED.
pub(crate) struct FmtBuf {
    buf: [u8; BUF_SIZE],
    ptr: usize,
}

impl FmtBuf {
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; BUF_SIZE],
            ptr: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.ptr = 0;
    }

    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[0..self.ptr]).unwrap()
    }
}

impl core::fmt::Write for FmtBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let rest_len = self.buf.len() - self.ptr;
        let len = if rest_len < s.len() {
            rest_len
        } else {
            s.len()
        };
        self.buf[self.ptr..(self.ptr + len)].copy_from_slice(&s.as_bytes()[0..len]);
        self.ptr += len;
        Ok(())
    }
}

/// A single selectable row in a menu level.
#[derive(Clone, Copy)]
struct MenuOption {
    label: &'static str,
    action: MenuAction,
}

/// One menu screen: a title and its list of options.
#[derive(Clone, Copy)]
struct MenuLevel {
    title: &'static str,
    options: &'static [MenuOption],
}

/// An entry on the navigation stack — keeps the position within one level.
#[derive(Clone, Copy)]
struct StackItem {
    level: &'static MenuLevel,
    cursor: usize,
}

/// Bounds, step size, and display metadata for a single setting type.
/// Used to adjust menu navigation settings for edit modes
struct SettingMeta {
    step: u32,
    min: u32,
    max: u32,
    divisor: u32,
    unit: &'static str,
}

impl SettingKey {
    fn meta(&self) -> SettingMeta {
        match self {
            Self::AllButtonDebounce | Self::ButtonDebounce(_) | Self::EncoderDebounce(_) => {
                SettingMeta {
                    step: 1_000,
                    min: 0,
                    max: 1_000_000,
                    divisor: 1_000,
                    unit: "ms",
                }
            }
            Self::EncoderStepThreshold(_) => SettingMeta {
                step: 1,
                min: 0,
                max: 100,
                divisor: 1,
                unit: "",
            },
            Self::EncoderMoveTimeout(_) => SettingMeta {
                step: 5_000,
                min: 0,
                max: 10_000_000,
                divisor: 1_000,
                unit: "ms",
            },
        }
    }
}

/// Drives the SSD1306 OLED display and menu logic for the IIDX deck.
pub struct MenuHandler<'a, D> {
    pub display: &'a mut OledDisplay<D>,
    line_bufs: [FmtBuf; 4],
    text_style: MonoTextStyle<'static, BinaryColor>,
    pub frames_rendered: u64,
    state: MenuSubState,
    stack: [Option<StackItem>; MAX_MENU_DEPTH],
    stack_depth: usize,
    /// RAM shadow of flash settings — modified in-place during editing,
    /// written to flash only when the user explicitly saves.
    pub settings: FlashStoragePersistentMemory,
    /// Independent value for "All" debounce — separate from any single button.
    all_debounce_value: u32,
}

impl<'a, D: WriteOnlyDataCommand> MenuHandler<'a, D> {
    pub fn new(display: &'a mut OledDisplay<D>, settings: FlashStoragePersistentMemory) -> Self {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X18_BOLD)
            .text_color(BinaryColor::On)
            .build();
        let mut stack = [None; MAX_MENU_DEPTH];
        stack[0] = Some(StackItem {
            level: &ROOT_MENU,
            cursor: 0,
        });
        Self {
            display,
            line_bufs: [FmtBuf::new(), FmtBuf::new(), FmtBuf::new(), FmtBuf::new()],
            text_style,
            frames_rendered: 0,
            state: MenuSubState::Browsing,
            stack,
            stack_depth: 1,
            settings,
            all_debounce_value: DEFAULT_BUTTON_DEBOUNCE_TICKS as u32,
        }
    }

    pub fn process_event(&mut self, event: MenuEvents) {
        match event {
            MenuEvents::Press(button) => {
                debug!(
                    "MENU PRESS! {} ({})",
                    button as usize,
                    match self.state {
                        MenuSubState::Browsing => "Browsing",
                        MenuSubState::EditingValue { .. } => "EditingValue",
                        MenuSubState::EditingKeyBinding { .. } => "EditingKeyBinding",
                        MenuSubState::DisplayMode(_) => "DisplayMode",
                    }
                );
                let current_state = self.state;
                match current_state {
                    MenuSubState::DisplayMode(MenuMode::Debug) => {
                        if button == ButtonCode::CcSelect {
                            self.state = MenuSubState::Browsing;
                        }
                    }
                    MenuSubState::DisplayMode(MenuMode::PixelTest) => {
                        self.state = MenuSubState::Browsing;
                    }
                    MenuSubState::Browsing => {
                        let level = self.current_level();
                        let cursor = self.current_cursor();
                        match button {
                            ButtonCode::CcUp => {
                                if cursor > 0 {
                                    *self.current_cursor_mut() -= 1;
                                }
                            }
                            ButtonCode::CcDown => {
                                if cursor + 1 < level.options.len() {
                                    *self.current_cursor_mut() += 1;
                                }
                            }
                            ButtonCode::CcSelect | ButtonCode::CcRight => {
                                self.execute_action(level.options[cursor].action);
                            }
                            ButtonCode::CcLeft => {
                                self.pop_level();
                            }
                            _ => {}
                        }
                    }
                    MenuSubState::EditingValue {
                        setting,
                        original_value,
                        working_value,
                    } => {
                        let meta = setting.meta();
                        match button {
                            ButtonCode::CcLeft => {
                                let new_val = working_value
                                    .saturating_sub(meta.step)
                                    .clamp(meta.min, meta.max);
                                if new_val != working_value {
                                    self.state = MenuSubState::EditingValue {
                                        setting,
                                        original_value,
                                        working_value: new_val,
                                    };
                                }
                            }
                            ButtonCode::CcRight => {
                                let new_val = working_value
                                    .saturating_add(meta.step)
                                    .clamp(meta.min, meta.max);
                                if new_val != working_value {
                                    self.state = MenuSubState::EditingValue {
                                        setting,
                                        original_value,
                                        working_value: new_val,
                                    };
                                }
                            }
                            ButtonCode::CcSelect => {
                                self.write_setting(setting, working_value);
                                self.state = MenuSubState::Browsing;
                            }
                            _ => {}
                        }
                    }
                    MenuSubState::EditingKeyBinding {
                        button: bind_button,
                        working_key_idx,
                        original_key_idx,
                    } => match button {
                        ButtonCode::CcLeft => {
                            let new_idx = working_key_idx.saturating_sub(1);
                            self.state = MenuSubState::EditingKeyBinding {
                                button: bind_button,
                                working_key_idx: new_idx,
                                original_key_idx,
                            };
                        }
                        ButtonCode::CcRight => {
                            let new_idx = min(working_key_idx + 1, MAX_HID_KEY);
                            self.state = MenuSubState::EditingKeyBinding {
                                button: bind_button,
                                working_key_idx: new_idx,
                                original_key_idx,
                            };
                        }
                        ButtonCode::CcSelect => {
                            self.write_key_binding(bind_button, working_key_idx as u8);
                            self.state = MenuSubState::Browsing;
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    pub fn render_menu(
        &mut self,
        current_combined_button_state: u64,
        encoder_p1_count: i32,
        encoder_p2_count: i32,
    ) {
        let current_state = self.state;
        match current_state {
            MenuSubState::DisplayMode(MenuMode::Debug) => self.print_debug_display(
                current_combined_button_state,
                encoder_p1_count,
                encoder_p2_count,
            ),
            MenuSubState::DisplayMode(MenuMode::PixelTest) => self.print_pixel_test(),
            MenuSubState::Browsing => {
                for line in &mut self.line_bufs {
                    line.reset();
                }
                self.render_browsing();
                self.draw_menu_text(None);
            }
            MenuSubState::EditingValue {
                setting,
                working_value,
                ..
            } => {
                for line in &mut self.line_bufs {
                    line.reset();
                }
                self.render_editing_value(setting, working_value);
                let cursor = self.current_cursor();
                let len = self.current_level().options.len();
                let hl = if cursor == 0 {
                    1
                } else if cursor == len - 1 {
                    3
                } else {
                    2
                };
                self.draw_menu_text(Some(hl));
            }
            MenuSubState::EditingKeyBinding {
                button,
                working_key_idx,
                ..
            } => {
                for line in &mut self.line_bufs {
                    line.reset();
                }
                self.render_editing_keybinding(button, working_key_idx);
                let cursor = self.current_cursor();
                let len = self.current_level().options.len();
                let hl = if cursor == 0 {
                    1
                } else if cursor == len - 1 {
                    3
                } else {
                    2
                };
                self.draw_menu_text(Some(hl));
            }
        }
    }

    fn print_debug_display(
        &mut self,
        current_combined_button_state: u64,
        encoder_p1_count: i32,
        encoder_p2_count: i32,
    ) {
        for line in &mut self.line_bufs {
            line.reset();
        }
        write!(self.line_bufs[0], "fc: {}", self.frames_rendered).unwrap();
        write!(self.line_bufs[1], "{}", encoder_p1_count).unwrap();
        write!(self.line_bufs[2], "{}", encoder_p2_count).unwrap();
        write!(self.line_bufs[3], "Not used").unwrap();

        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        // Frame counter (top-left)
        Text::with_baseline(
            self.line_bufs[0].as_str(),
            FRAME_COUNTER_POS,
            self.text_style,
            Baseline::Top,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 1 (left-middle)
        Text::with_alignment(
            self.line_bufs[1].as_str(),
            ENCODER1_LABEL_POS,
            self.text_style,
            Alignment::Left,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 2 (right-middle)
        Text::with_alignment(
            self.line_bufs[2].as_str(),
            ENCODER2_LABEL_POS,
            self.text_style,
            Alignment::Right,
        )
        .draw(self.display)
        .unwrap();

        // Button layout background
        self.draw_empty_button_graphic();

        // Pressed-button indicator dots and encoder arrows
        self.draw_pressed_buttons(current_combined_button_state);

        // displays current buffer on screen.
        self.display.flush().unwrap();
    }

    /// Draws the base IIDX button-layout image from [`BUTTON_GRAPHIC`].
    fn draw_empty_button_graphic(&mut self) {
        let raw = ImageRaw::<BinaryColor>::new(&BUTTON_GRAPHIC, 128);
        let image = Image::new(&raw, Point::new(0, BUTTON_GRAPHIC_ROW_HEIGHT as i32));
        image.draw(self.display).unwrap();
    }

    /// Draws filled rectangles for physical pressed buttons and arrow
    /// graphics for encoder logical buttons, based on the combined u64 state.
    fn draw_pressed_buttons(&mut self, combined_state: u64) {
        // Physical buttons (lower 32 bits)
        let physical_state = combined_state as u32;
        for (i, rect) in BUTTON_DEBUG_RECTANGLES.iter().enumerate() {
            if (physical_state >> i) & 1 == 1 {
                rect.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(self.display)
                    .unwrap();
            }
        }

        // Logical encoder buttons (bits 32–35)
        for code in &[
            ButtonCode::P1Positive,
            ButtonCode::P1Negative,
            ButtonCode::P2Positive,
            ButtonCode::P2Negative,
        ] {
            if (combined_state >> (*code as usize)) & 1 == 1 {
                self.draw_encoder_arrow(*code);
            }
        }
    }

    /// Draws the arrow graphic for an encoder logical button, applying the
    /// appropriate flip and positioning it at the correct anchor point.
    fn draw_encoder_arrow(&mut self, code: ButtonCode) {
        use embedded_graphics::Pixel;

        let anchor = match code {
            ButtonCode::P1Positive => P1_POSITIVE_ANCHOR,
            ButtonCode::P1Negative => P1_NEGATIVE_ANCHOR,
            ButtonCode::P2Positive => P2_POSITIVE_ANCHOR,
            ButtonCode::P2Negative => P2_NEGATIVE_ANCHOR,
            _ => return,
        };
        let flip = flip_for(code);

        for &(dx, dy) in &ARROW_GRAPHIC_PIXELS {
            let (tx, ty) = match flip {
                Flip::NoFlip => (dx, dy),
                Flip::FlipX => (-dx, dy),
                Flip::FlipY => (dx, -dy),
                Flip::FlipXY => (-dx, -dy),
            };
            Pixel(Point::new(anchor.x + tx, anchor.y + ty), BinaryColor::On)
                .draw(self.display)
                .unwrap();
        }
    }

    /// Fills the entire display white (all pixels on) as a quick pixel test.
    fn print_pixel_test(&mut self) {
        let color = embedded_graphics::pixelcolor::BinaryColor::On;
        self.display.clear(color).unwrap();
        self.display.flush().unwrap();
    }

    // ── Menu rendering ──

    /// Writes one menu line: prefix + label[ + " " + value_suffix].
    /// Label is truncated so the full line fits within VISIBLE_WIDTH.
    fn write_menu_line(buf: &mut FmtBuf, selected: bool, label: &str, value_suffix: Option<&str>) {
        buf.reset();
        let prefix = if selected { ">" } else { " " };
        let suffix_total = value_suffix.map_or(0, |s| 1 + s.as_bytes().len());
        let max_label = (VISIBLE_WIDTH as isize) - 1 - (suffix_total as isize);
        let max_label = if max_label < 0 { 0 } else { max_label as usize };
        let label = if label.as_bytes().len() > max_label {
            &label[..max_label]
        } else {
            label
        };
        write!(buf, "{}{}", prefix, label).unwrap();
        if let Some(suffix) = value_suffix {
            write!(buf, " {}", suffix).unwrap();
        }
    }

    /// Writes a value string into `buf` for the given option.
    /// Uses `override_val` when in edit mode so the live working value is shown.
    fn write_option_value(
        &self,
        option: &MenuOption,
        buf: &mut FmtBuf,
        override_val: Option<u32>,
    ) -> bool {
        match option.action {
            MenuAction::EditSetting(key) => {
                let val = override_val.unwrap_or_else(|| self.read_setting(key));
                let meta = key.meta();
                let display_val = val / meta.divisor;
                if meta.unit.is_empty() {
                    write!(buf, "{}", display_val).unwrap();
                } else {
                    write!(buf, "{} {}", display_val, meta.unit).unwrap();
                }
                true
            }
            MenuAction::EditKeyBinding(code) => {
                let key = override_val
                    .map(|v| v as u8)
                    .unwrap_or_else(|| self.read_key_binding(code));
                let hi = core::char::from_digit((key as usize >> 4) as u32, 16).unwrap_or('0');
                let lo = core::char::from_digit((key as usize & 0xF) as u32, 16).unwrap_or('0');
                write!(buf, "0x{}{}", hi, lo).unwrap();
                true
            }
            _ => false,
        }
    }

    /// Helper: renders the 3 visible option lines (lines 1–3) based on cursor position.
    /// `override_val` is `Some` on the selected line during editing to show the live value.
    fn render_options(&mut self, override_val: Option<u32>) {
        let level = self.current_level();
        let cursor = self.current_cursor();
        let len = level.options.len();

        let lines: [(usize, usize, bool); 3] = match cursor {
            0 => [(1, 0, true), (2, 1, false), (3, 2, false)],
            c if c == len - 1 => [
                (1, c.saturating_sub(2), false),
                (2, c.saturating_sub(1), false),
                (3, c, true),
            ],
            c => [(1, c - 1, false), (2, c, true), (3, c + 1, false)],
        };

        for &(buf_idx, opt_idx, selected) in &lines {
            if opt_idx >= len {
                continue;
            }
            let option = &level.options[opt_idx];
            let mut value_buf = FmtBuf::new();
            let vo = if selected { override_val } else { None };
            let has_value = self.write_option_value(option, &mut value_buf, vo);
            let suffix = if has_value {
                Some(value_buf.as_str())
            } else {
                None
            };
            Self::write_menu_line(&mut self.line_bufs[buf_idx], selected, option.label, suffix);
        }
    }

    fn render_browsing(&mut self) {
        write!(self.line_bufs[0], "{}", self.current_level().title).unwrap();
        self.render_options(None);
    }

    fn render_editing_value(&mut self, _setting: SettingKey, working_value: u32) {
        write!(self.line_bufs[0], "{}", self.current_level().title).unwrap();
        self.render_options(Some(working_value));
    }

    fn render_editing_keybinding(&mut self, _button: ButtonCode, working_key_idx: usize) {
        write!(self.line_bufs[0], "{}", self.current_level().title).unwrap();
        self.render_options(Some(working_key_idx as u32));
    }

    /// Draws the 4 FmtBuf lines to the OLED, a separator below the title,
    /// and optionally a highlighted box around one line (for edit mode).
    fn draw_menu_text(&mut self, highlight_line: Option<usize>) {
        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        for i in 0..4 {
            let s = self.line_bufs[i].as_str();
            if !s.is_empty() {
                Text::with_baseline(s, Point::new(0, LINE_Y[i]), self.text_style, Baseline::Top)
                    .draw(self.display)
                    .unwrap();
            }
        }

        if let Some(line) = highlight_line {
            Rectangle::new(Point::new(0, LINE_Y[line]), Size::new(128, 18))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(self.display)
                .unwrap();
        }

        Line::new(
            Point::new(0, LINE_Y[0] + 18),
            Point::new(127, LINE_Y[0] + 18),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(self.display)
        .unwrap();

        self.display.flush().unwrap();
    }

    // ── Navigation helpers ──

    fn execute_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::OpenSubmenu(level) => self.push_level(level),
            MenuAction::GoBack => self.pop_level(),
            MenuAction::None => {}
            MenuAction::ShowDebugScreen => self.state = MenuSubState::DisplayMode(MenuMode::Debug),
            MenuAction::ShowPixelTest => {
                self.state = MenuSubState::DisplayMode(MenuMode::PixelTest)
            }
            MenuAction::EditSetting(key) => {
                let value = self.read_setting(key);
                self.state = MenuSubState::EditingValue {
                    setting: key,
                    original_value: value,
                    working_value: value,
                };
            }
            MenuAction::EditKeyBinding(code) => {
                let key = self.read_key_binding(code);
                self.state = MenuSubState::EditingKeyBinding {
                    button: code,
                    working_key_idx: key as usize,
                    original_key_idx: key as usize,
                };
            }
        }
    }

    fn push_level(&mut self, level: &'static MenuLevel) {
        if self.stack_depth < MAX_MENU_DEPTH {
            self.stack[self.stack_depth] = Some(StackItem { level, cursor: 0 });
            self.stack_depth += 1;
        }
    }

    fn pop_level(&mut self) {
        if self.stack_depth > 1 {
            self.stack_depth -= 1;
            self.stack[self.stack_depth] = None;
        }
    }

    fn current_level(&self) -> &'static MenuLevel {
        self.stack[self.stack_depth - 1].unwrap().level
    }

    fn current_cursor(&self) -> usize {
        self.stack[self.stack_depth - 1].unwrap().cursor
    }

    fn current_cursor_mut(&mut self) -> &mut usize {
        &mut self.stack[self.stack_depth - 1].as_mut().unwrap().cursor
    }

    // ── Setting read/write ──

    fn read_setting(&self, key: SettingKey) -> u32 {
        match key {
            SettingKey::AllButtonDebounce => self.all_debounce_value,
            SettingKey::ButtonDebounce(code) => {
                self.settings.buttons[code as usize].debounce_ticks as u32
            }
            SettingKey::EncoderDebounce(idx) => self.settings.encoders[idx].debounce_ticks as u32,
            SettingKey::EncoderStepThreshold(idx) => {
                self.settings.encoders[idx].step_threshold as u32
            }
            SettingKey::EncoderMoveTimeout(idx) => {
                self.settings.encoders[idx].move_timeout_ticks as u32
            }
        }
    }

    fn write_setting(&mut self, key: SettingKey, value: u32) {
        match key {
            SettingKey::AllButtonDebounce => {
                self.all_debounce_value = value;
                for button in &mut self.settings.buttons {
                    button.debounce_ticks = value as u64;
                }
            }
            SettingKey::ButtonDebounce(code) => {
                self.settings.buttons[code as usize].debounce_ticks = value as u64;
            }
            SettingKey::EncoderDebounce(idx) => {
                self.settings.encoders[idx].debounce_ticks = value as u64;
            }
            SettingKey::EncoderStepThreshold(idx) => {
                self.settings.encoders[idx].step_threshold = value as i32;
            }
            SettingKey::EncoderMoveTimeout(idx) => {
                self.settings.encoders[idx].move_timeout_ticks = value as u64;
            }
        }
    }

    fn read_key_binding(&self, code: ButtonCode) -> u8 {
        let idx = code as usize;
        if idx < NUM_BUTTONS {
            self.settings.buttons[idx].key
        } else {
            match code {
                ButtonCode::P1Positive => self.settings.encoders[0].key_up,
                ButtonCode::P1Negative => self.settings.encoders[0].key_down,
                ButtonCode::P2Positive => self.settings.encoders[1].key_up,
                ButtonCode::P2Negative => self.settings.encoders[1].key_down,
                _ => 0,
            }
        }
    }

    fn write_key_binding(&mut self, code: ButtonCode, key: u8) {
        let idx = code as usize;
        if idx < NUM_BUTTONS {
            self.settings.buttons[idx].key = key;
        } else {
            match code {
                ButtonCode::P1Positive => self.settings.encoders[0].key_up = key,
                ButtonCode::P1Negative => self.settings.encoders[0].key_down = key,
                ButtonCode::P2Positive => self.settings.encoders[1].key_up = key,
                ButtonCode::P2Negative => self.settings.encoders[1].key_down = key,
                _ => {}
            }
        }
    }
}
