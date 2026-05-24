//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

use crate::{
    BUF_SIZE, ButtonCode, DEFAULT_BUTTON_DEBOUNCE_TICKS, FlashStoragePersistentMemory, NUM_BUTTONS,
    OledDisplay,
};
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

/// Width of a single character in the FONT_9X18_BOLD font (pixels).
const CHAR_W: i32 = 9;

/// OLED display width in pixels.
const DISPLAY_W: u32 = 128;

/// Rightmost pixel column on the display.
const DISPLAY_R: i32 = (DISPLAY_W - 1) as i32;

/// All defined USB HID keyboard usage codes (0-231, skipping reserved range 165-223).
const VALID_KEYS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 224, 225, 226, 227, 228, 229, 230, 231,
];

/// Visible character width of the OLED display with the 9px font.
const VISIBLE_WIDTH: usize = 13;

/// Y-origin of each of the 4 text lines on the OLED.
const LINE_Y: [i32; 4] = [0, 16, 32, 48];

/// Top-left corner of the frame-counter text in debug screen
const FRAME_COUNTER_POS: Point = Point::new(0, 0);
/// Left-middle position for the encoder 1 count in debug screen
const ENCODER1_LABEL_POS: Point = Point::new(0, 32);
/// Right-middle position for the encoder 2 count in debug screen
const ENCODER2_LABEL_POS: Point = Point::new(DISPLAY_R, 32);

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

/// Left-pointing back-arrow glyph for menu "Back" options.
/// Pixels relative to the top-left of the glyph bounding box.
#[rustfmt::skip]
const BACK_ARROW_PIXELS: [(i32, i32); 48] = [
                                                                                        (12,0),(13,0),
                         ( 3,1),                                                        (12,1),(13,1),
                  ( 2,2),( 3,2),                                                        (12,2),(13,2),
           ( 1,3),( 2,3),( 3,3),                                                        (12,3),(13,3),
    ( 0,4),( 1,4),( 2,4),( 3,4),( 4,4),( 5,4),( 6,4),( 7,4),( 8,4),( 9,4),(10,4),(11,4),(12,4),(13,4),
    ( 0,5),( 1,5),( 2,5),( 3,5),( 4,5),( 5,5),( 6,5),( 7,5),( 8,5),( 9,5),(10,5),(11,5),(12,5),(13,5),
           ( 1,6),( 2,6),( 3,6),
                  ( 2,7),( 3,7),
                         ( 3,8),
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
    LongPress(ButtonCode),
    Repeat(ButtonCode),
    Idle,
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
    /// Open the dedicated encoder-editing screen for wiki sensitivity.
    OpenWikiEdit(usize),
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
    WikiEdit {
        encoder: usize,
        selected: usize,
        editing: bool,
        working_threshold: u32,
        working_timeout: u32,
        original_threshold: u32,
        original_timeout: u32,
    },
    DisplayMode(MenuMode),
    IdleMode,
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

/// Compact human-readable key name for a USB HID keyboard usage code (0-101).
/// Returns the index of `key` within [`VALID_KEYS`].
fn key_index(key: u8) -> usize {
    VALID_KEYS.iter().position(|&k| k == key).unwrap_or(0)
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
            label: "Wiki Config",
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
    title: "Wiki Config",
    options: &[
        MenuOption {
            label: "P1 Wiki",
            action: MenuAction::OpenWikiEdit(0),
        },
        MenuOption {
            label: "P2 Wiki",
            action: MenuAction::OpenWikiEdit(1),
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
            label: "P1Sel",
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
            label: "P2Sel",
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
            label: "CcSel",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::CcSelect)),
        },
        MenuOption {
            label: "VolUp",
            action: MenuAction::EditSetting(SettingKey::ButtonDebounce(ButtonCode::VolumeUp)),
        },
        MenuOption {
            label: "VolDn",
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
            label: "P1Sel",
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
            label: "P2Sel",
            action: MenuAction::EditKeyBinding(ButtonCode::P2Select),
        },
        MenuOption {
            label: "Escape",
            action: MenuAction::EditKeyBinding(ButtonCode::Escape),
        },
        MenuOption {
            label: "VolUp",
            action: MenuAction::EditKeyBinding(ButtonCode::VolumeUp),
        },
        MenuOption {
            label: "VolDn",
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
                unit: "Steps",
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
    debug_bufs: [FmtBuf; 4],
    title_buf: FmtBuf,
    label_bufs: [FmtBuf; 3],
    value_bufs: [FmtBuf; 3],
    text_style: MonoTextStyle<'static, BinaryColor>,
    pub frames_rendered: u64,
    state: MenuSubState,
    saved_state: MenuSubState,
    stack: [Option<StackItem>; MAX_MENU_DEPTH],
    stack_depth: usize,
    /// RAM shadow of flash settings — modified in-place during editing,
    /// written to flash only when the user explicitly saves.
    pub settings: FlashStoragePersistentMemory,
    /// Independent value for "All" debounce — separate from any single button.
    all_debounce_value: u32,
    /// Y position of the back-arrow glyph, if any option on screen has `GoBack`.
    back_arrow_y: Option<i32>,
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
            debug_bufs: [FmtBuf::new(), FmtBuf::new(), FmtBuf::new(), FmtBuf::new()],
            title_buf: FmtBuf::new(),
            label_bufs: [FmtBuf::new(), FmtBuf::new(), FmtBuf::new()],
            value_bufs: [FmtBuf::new(), FmtBuf::new(), FmtBuf::new()],
            text_style,
            frames_rendered: 0,
            state: MenuSubState::IdleMode,
            saved_state: MenuSubState::Browsing,
            stack,
            stack_depth: 1,
            settings,
            all_debounce_value: DEFAULT_BUTTON_DEBOUNCE_TICKS as u32,
            back_arrow_y: None,
        }
    }

    pub fn process_event(&mut self, event: MenuEvents) {
        match event {
            MenuEvents::Press(button) => {
                let current_state = self.state;
                match current_state {
                    MenuSubState::IdleMode => {
                        if matches!(
                            button,
                            ButtonCode::CcUp
                                | ButtonCode::CcDown
                                | ButtonCode::CcLeft
                                | ButtonCode::CcRight
                                | ButtonCode::CcSelect
                        ) {
                            debug!("menu: exit idle");
                            self.state = self.saved_state;
                        }
                    }
                    MenuSubState::DisplayMode(MenuMode::Debug) => {
                        // Short press does nothing in debug mode.
                        // Exit requires a long press (handled in LongPress arm).
                    }
                    MenuSubState::DisplayMode(MenuMode::PixelTest) => {
                        debug!("menu: exit pixel test");
                        self.state = MenuSubState::Browsing;
                    }
                    MenuSubState::Browsing => {
                        let level = self.current_level();
                        let cursor = self.current_cursor();
                        match button {
                            ButtonCode::CcUp => {
                                let len = level.options.len();
                                let new_cursor = (cursor + len - 1) % len;
                                debug!("menu: cursor up → {}", new_cursor);
                                *self.current_cursor_mut() = new_cursor;
                            }
                            ButtonCode::CcDown => {
                                let len = level.options.len();
                                let new_cursor = (cursor + 1) % len;
                                debug!("menu: cursor down → {}", new_cursor);
                                *self.current_cursor_mut() = new_cursor;
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
                            ButtonCode::CcLeft | ButtonCode::CcDown => {
                                let new_val = working_value
                                    .saturating_sub(meta.step)
                                    .clamp(meta.min, meta.max);
                                if new_val != working_value {
                                    debug!(
                                        "menu: edit {}\u{2193} {}",
                                        new_val / meta.divisor,
                                        meta.unit
                                    );
                                    self.state = MenuSubState::EditingValue {
                                        setting,
                                        original_value,
                                        working_value: new_val,
                                    };
                                }
                            }
                            ButtonCode::CcRight | ButtonCode::CcUp => {
                                let new_val = working_value
                                    .saturating_add(meta.step)
                                    .clamp(meta.min, meta.max);
                                if new_val != working_value {
                                    debug!(
                                        "menu: edit {}\u{2191} {}",
                                        new_val / meta.divisor,
                                        meta.unit
                                    );
                                    self.state = MenuSubState::EditingValue {
                                        setting,
                                        original_value,
                                        working_value: new_val,
                                    };
                                }
                            }
                            ButtonCode::CcSelect => {
                                debug!(
                                    "menu: commit {} {}",
                                    working_value / meta.divisor,
                                    meta.unit
                                );
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
                        ButtonCode::CcLeft | ButtonCode::CcDown => {
                            let count = VALID_KEYS.len();
                            let new_idx = (working_key_idx + count - 1) % count;
                            debug!("menu: key bind ← {}", VALID_KEYS[new_idx]);
                            self.state = MenuSubState::EditingKeyBinding {
                                button: bind_button,
                                working_key_idx: new_idx,
                                original_key_idx,
                            };
                        }
                        ButtonCode::CcRight | ButtonCode::CcUp => {
                            let count = VALID_KEYS.len();
                            let new_idx = (working_key_idx + 1) % count;
                            debug!("menu: key bind → {}", VALID_KEYS[new_idx]);
                            self.state = MenuSubState::EditingKeyBinding {
                                button: bind_button,
                                working_key_idx: new_idx,
                                original_key_idx,
                            };
                        }
                        ButtonCode::CcSelect => {
                            let key = VALID_KEYS[working_key_idx];
                            debug!("menu: bind commit {}", key);
                            self.write_key_binding(bind_button, key);
                            self.state = MenuSubState::Browsing;
                        }
                        _ => {}
                    },
                    MenuSubState::WikiEdit {
                        encoder,
                        selected,
                        editing,
                        working_threshold,
                        working_timeout,
                        original_threshold,
                        original_timeout,
                    } => {
                        if editing {
                            let val = if selected == 0 {
                                working_threshold
                            } else {
                                working_timeout
                            };
                            let key = if selected == 0 {
                                SettingKey::EncoderStepThreshold(encoder)
                            } else {
                                SettingKey::EncoderMoveTimeout(encoder)
                            };
                            let meta = key.meta();
                            match button {
                                ButtonCode::CcLeft | ButtonCode::CcDown => {
                                    let new_val =
                                        val.saturating_sub(meta.step).clamp(meta.min, meta.max);
                                    if selected == 0 {
                                        self.state = MenuSubState::WikiEdit {
                                            encoder,
                                            selected,
                                            editing: true,
                                            working_threshold: new_val,
                                            working_timeout,
                                            original_threshold,
                                            original_timeout,
                                        };
                                    } else {
                                        self.state = MenuSubState::WikiEdit {
                                            encoder,
                                            selected,
                                            editing: true,
                                            working_threshold,
                                            working_timeout: new_val,
                                            original_threshold,
                                            original_timeout,
                                        };
                                    }
                                }
                                ButtonCode::CcRight | ButtonCode::CcUp => {
                                    let new_val =
                                        val.saturating_add(meta.step).clamp(meta.min, meta.max);
                                    if selected == 0 {
                                        self.state = MenuSubState::WikiEdit {
                                            encoder,
                                            selected,
                                            editing: true,
                                            working_threshold: new_val,
                                            working_timeout,
                                            original_threshold,
                                            original_timeout,
                                        };
                                    } else {
                                        self.state = MenuSubState::WikiEdit {
                                            encoder,
                                            selected,
                                            editing: true,
                                            working_threshold,
                                            working_timeout: new_val,
                                            original_threshold,
                                            original_timeout,
                                        };
                                    }
                                }
                                ButtonCode::CcSelect => {
                                    let sk = SettingKey::EncoderStepThreshold(encoder);
                                    self.write_setting(sk, working_threshold);
                                    let sk = SettingKey::EncoderMoveTimeout(encoder);
                                    self.write_setting(sk, working_timeout);
                                    debug!("menu: wiki commit");
                                    self.state = MenuSubState::WikiEdit {
                                        encoder,
                                        selected,
                                        editing: false,
                                        working_threshold,
                                        working_timeout,
                                        original_threshold,
                                        original_timeout,
                                    };
                                }
                                _ => {}
                            }
                        } else {
                            match button {
                                ButtonCode::CcUp | ButtonCode::CcDown => {
                                    let new_sel = (selected + 1) % 2;
                                    debug!(
                                        "menu: wiki select {}",
                                        if new_sel == 0 { "threshold" } else { "timeout" }
                                    );
                                    self.state = MenuSubState::WikiEdit {
                                        encoder,
                                        selected: new_sel,
                                        editing: false,
                                        working_threshold,
                                        working_timeout,
                                        original_threshold,
                                        original_timeout,
                                    };
                                }
                                ButtonCode::CcSelect | ButtonCode::CcRight => {
                                    debug!("menu: wiki edit start");
                                    self.state = MenuSubState::WikiEdit {
                                        encoder,
                                        selected,
                                        editing: true,
                                        working_threshold,
                                        working_timeout,
                                        original_threshold,
                                        original_timeout,
                                    };
                                }
                                ButtonCode::CcLeft => {
                                    debug!("menu: wiki exit");
                                    self.state = MenuSubState::Browsing;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            MenuEvents::LongPress(button) => {
                let current_state = self.state;
                match current_state {
                    MenuSubState::IdleMode => {
                        if matches!(
                            button,
                            ButtonCode::CcUp
                                | ButtonCode::CcDown
                                | ButtonCode::CcLeft
                                | ButtonCode::CcRight
                                | ButtonCode::CcSelect
                        ) {
                            debug!("menu: exit idle");
                            self.state = self.saved_state;
                        }
                    }
                    MenuSubState::DisplayMode(MenuMode::Debug) => {
                        if button == ButtonCode::CcSelect || button == ButtonCode::CcLeft {
                            debug!("menu: exit debug screen (long-press)");
                            self.state = MenuSubState::Browsing;
                        }
                    }
                    MenuSubState::DisplayMode(MenuMode::PixelTest) => {
                        debug!("menu: exit pixel test");
                        self.state = MenuSubState::Browsing;
                    }
                    _ => match button {
                        ButtonCode::CcUp | ButtonCode::CcDown => {
                            self.process_event(MenuEvents::Press(button));
                        }
                        ButtonCode::CcLeft | ButtonCode::CcRight
                            if matches!(
                                self.state,
                                MenuSubState::EditingValue { .. }
                                    | MenuSubState::EditingKeyBinding { .. }
                                    | MenuSubState::WikiEdit { .. }
                            ) =>
                        {
                            self.process_event(MenuEvents::Press(button));
                        }
                        _ => {}
                    },
                }
            }
            MenuEvents::Repeat(button) => {
                if matches!(self.state, MenuSubState::IdleMode)
                    && matches!(
                        button,
                        ButtonCode::CcUp
                            | ButtonCode::CcDown
                            | ButtonCode::CcLeft
                            | ButtonCode::CcRight
                            | ButtonCode::CcSelect
                    )
                {
                    debug!("menu: exit idle");
                    self.state = self.saved_state;
                    return;
                }
                let is_editing = matches!(
                    self.state,
                    MenuSubState::EditingValue { .. }
                        | MenuSubState::EditingKeyBinding { .. }
                        | MenuSubState::WikiEdit { .. }
                );
                match button {
                    ButtonCode::CcUp | ButtonCode::CcDown => {
                        self.process_event(MenuEvents::Press(button));
                    }
                    ButtonCode::CcLeft | ButtonCode::CcRight if is_editing => {
                        self.process_event(MenuEvents::Press(button));
                    }
                    _ => {}
                }
            }
            MenuEvents::Idle => {
                if !matches!(self.state, MenuSubState::IdleMode) {
                    debug!("menu: idle");
                    self.saved_state = self.state;
                    self.state = MenuSubState::IdleMode;
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
            MenuSubState::IdleMode => self.print_debug_display(
                current_combined_button_state,
                encoder_p1_count,
                encoder_p2_count,
            ),
            MenuSubState::DisplayMode(MenuMode::Debug) => self.print_debug_display(
                current_combined_button_state,
                encoder_p1_count,
                encoder_p2_count,
            ),
            MenuSubState::DisplayMode(MenuMode::PixelTest) => self.print_pixel_test(),
            MenuSubState::Browsing => {
                self.back_arrow_y = None;
                self.title_buf.reset();
                for line in &mut self.label_bufs {
                    line.reset();
                }
                for line in &mut self.value_bufs {
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
                self.back_arrow_y = None;
                self.title_buf.reset();
                for line in &mut self.label_bufs {
                    line.reset();
                }
                for line in &mut self.value_bufs {
                    line.reset();
                }
                self.render_editing_value(setting, working_value);
                self.draw_menu_text(Some(2));
            }
            MenuSubState::EditingKeyBinding {
                button,
                working_key_idx,
                ..
            } => {
                self.back_arrow_y = None;
                self.title_buf.reset();
                for line in &mut self.label_bufs {
                    line.reset();
                }
                for line in &mut self.value_bufs {
                    line.reset();
                }
                self.render_editing_keybinding(button, working_key_idx);
                self.draw_menu_text(Some(2));
            }
            MenuSubState::WikiEdit {
                encoder,
                selected,
                editing,
                working_threshold,
                working_timeout,
                ..
            } => {
                self.render_wiki_edit(
                    encoder,
                    selected,
                    editing,
                    working_threshold,
                    working_timeout,
                );
            }
        }
    }

    fn print_debug_display(
        &mut self,
        current_combined_button_state: u64,
        encoder_p1_count: i32,
        encoder_p2_count: i32,
    ) {
        for line in &mut self.debug_bufs {
            line.reset();
        }
        write!(self.debug_bufs[0], "fc: {}", self.frames_rendered).unwrap();
        write!(self.debug_bufs[1], "{}", encoder_p1_count).unwrap();
        write!(self.debug_bufs[2], "{}", encoder_p2_count).unwrap();
        write!(self.debug_bufs[3], "Not used").unwrap();

        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        // Frame counter (top-left)
        Text::with_baseline(
            self.debug_bufs[0].as_str(),
            FRAME_COUNTER_POS,
            self.text_style,
            Baseline::Top,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 1 (left-middle)
        Text::with_alignment(
            self.debug_bufs[1].as_str(),
            ENCODER1_LABEL_POS,
            self.text_style,
            Alignment::Left,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 2 (right-middle)
        Text::with_alignment(
            self.debug_bufs[2].as_str(),
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
        let raw = ImageRaw::<BinaryColor>::new(&BUTTON_GRAPHIC, DISPLAY_W);
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

    /// Renders the dedicated wiki-edit screen: two parameters displayed vertically
    /// with labels on even lines and indented values on odd lines.
    fn render_wiki_edit(
        &mut self,
        encoder: usize,
        selected: usize,
        editing: bool,
        working_threshold: u32,
        working_timeout: u32,
    ) {
        for buf in &mut self.debug_bufs {
            buf.reset();
        }
        for buf in &mut self.value_bufs {
            buf.reset();
        }

        let t_meta = SettingKey::EncoderStepThreshold(0).meta();
        let m_meta = SettingKey::EncoderMoveTimeout(0).meta();
        let t_val = working_threshold / t_meta.divisor;
        let m_val = working_timeout / m_meta.divisor;

        // Labels on debug_bufs (left-aligned, odd line indices)
        write!(
            self.debug_bufs[0],
            "{}P{} Threshold",
            if selected == 0 { ">" } else { " " },
            encoder + 1
        )
        .unwrap();
        write!(
            self.debug_bufs[2],
            "{}P{} Timeout",
            if selected == 1 { ">" } else { " " },
            encoder + 1
        )
        .unwrap();

        // Values on value_bufs (right-aligned, even line indices)
        write!(self.value_bufs[0], "{} {}", t_val, t_meta.unit).unwrap();
        write!(self.value_bufs[1], "{} {}", m_val, m_meta.unit).unwrap();

        // Draw
        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        for (i, y) in [(0usize, 0usize), (2, 2)] {
            let s = self.debug_bufs[i].as_str();
            if !s.is_empty() {
                Text::with_baseline(s, Point::new(0, LINE_Y[y]), self.text_style, Baseline::Top)
                    .draw(self.display)
                    .unwrap();
            }
        }

        for (vi, ly) in [(0usize, 1usize), (1, 3)] {
            let v = self.value_bufs[vi].as_str();
            if !v.is_empty() {
                let mut text = Text::with_alignment(
                    v,
                    Point::new(124, LINE_Y[ly]),
                    self.text_style,
                    Alignment::Right,
                );
                text.text_style.baseline = Baseline::Top;
                text.draw(self.display).unwrap();
            }
        }

        if editing {
            let value_idx = if selected == 0 { 0 } else { 1 };
            let v = self.value_bufs[value_idx].as_str();
            let vlen = v.as_bytes().len();
            // Box from pixel 127 to 2px left of the value's first character
            let box_left = 123 - (vlen as i32 * CHAR_W);
            let box_left = if box_left < 0 { 0 } else { box_left };
            let target_line = if selected == 0 { 1 } else { 3 };
            Rectangle::new(
                Point::new(box_left, LINE_Y[target_line]),
                Size::new((DISPLAY_R - box_left) as u32, 16),
            )
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(self.display)
            .unwrap();
        }

        self.display.flush().unwrap();
    }

    // ── Menu rendering ──

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
                // Use the library's Debug name, overriding only long ones
                match key {
                    0 => write!(buf, "No Key"),
                    1 => write!(buf, "ErrRO"),
                    2 => write!(buf, "POST"),
                    3 => write!(buf, "Err"),
                    30..=39 => {
                        let digit = if key == 39 { 0 } else { key - 30 + 1 };
                        write!(buf, "K{}", digit)
                    }
                    40 => write!(buf, "Enter"),
                    42 => write!(buf, "BkSpc"),
                    47 => write!(buf, "{{"),
                    48 => write!(buf, "}}"),
                    49 => write!(buf, "\\"),
                    50 => write!(buf, "#"),
                    51 => write!(buf, ";"),
                    52 => write!(buf, "'"),
                    56 => write!(buf, "/"),
                    57 => write!(buf, "Caps"),
                    70 => write!(buf, "PrtSc"),
                    71 => write!(buf, "ScrLk"),
                    75 => write!(buf, "PgUp"),
                    76 => write!(buf, "Del"),
                    78 => write!(buf, "PgDn"),
                    79 => write!(buf, "RArr"),
                    80 => write!(buf, "LArr"),
                    81 => write!(buf, "DArr"),
                    82 => write!(buf, "UArr"),
                    83 => write!(buf, "NLck"),
                    84 => write!(buf, "Kp/"),
                    85 => write!(buf, "Kp*"),
                    86 => write!(buf, "Kp-"),
                    87 => write!(buf, "Kp+"),
                    88 => write!(buf, "KEnt"),
                    89..=97 => write!(buf, "Kp{}", key - 88),
                    98 => write!(buf, "Kp0"),
                    99 => write!(buf, "Kp."),
                    100 => write!(buf, "NUS\\"),
                    101 => write!(buf, "App"),
                    103 => write!(buf, "KpEq"),
                    127 => write!(buf, "Mute"),
                    128 => write!(buf, "VolUp"),
                    129 => write!(buf, "VolDn"),
                    130 => write!(buf, "LCaps"),
                    131 => write!(buf, "LNumL"),
                    132 => write!(buf, "LScrL"),
                    133 => write!(buf, "KpCom"),
                    134 => write!(buf, "KpEqS"),
                    153 => write!(buf, "AltEr"),
                    154 => write!(buf, "SysRq"),
                    159 => write!(buf, "Sep"),
                    162 => write!(buf, "ClrAg"),
                    163 => write!(buf, "Props"),
                    224 => write!(buf, "LCTRL"),
                    225 => write!(buf, "LSHFT"),
                    226 => write!(buf, "LALT"),
                    227 => write!(buf, "LGUI"),
                    228 => write!(buf, "RCTRL"),
                    229 => write!(buf, "RSHFT"),
                    230 => write!(buf, "RALT"),
                    231 => write!(buf, "RGUI"),
                    _ => write!(
                        buf,
                        "{:?}",
                        usbd_human_interface_device::page::Keyboard::from(key)
                    ),
                }
                .unwrap();
                true
            }
            _ => false,
        }
    }

    /// Renders the 3 visible option lines — cursor always on the middle line (li=1),
    /// with prev/next wrapping around when there are at least 3 options.
    fn render_options(&mut self, override_val: Option<u32>) {
        let level = self.current_level();
        let cursor = self.current_cursor();
        let len = level.options.len();
        let prev = (cursor + len - 1) % len;
        let next = (cursor + 1) % len;

        let lines: [(usize, usize, bool); 3] = if len >= 3 {
            [(1, prev, false), (2, cursor, true), (3, next, false)]
        } else if len == 2 {
            // Show the "other" option below when at top, above when at bottom
            if cursor == 0 {
                [(2, 0, true), (3, 1, false), (1, usize::MAX, false)]
            } else {
                [(1, 0, false), (2, 1, true), (3, usize::MAX, false)]
            }
        } else {
            // len == 1 — only the cursor line
            [(2, 0, true), (1, usize::MAX, false), (3, usize::MAX, false)]
        };

        for &(buf_idx, opt_idx, selected) in &lines {
            if opt_idx >= len {
                continue;
            }
            let option = &level.options[opt_idx];
            let li = buf_idx - 1; // label/value index (0-based)

            // Write label with prefix
            let prefix = if selected { ">" } else { " " };
            if matches!(option.action, MenuAction::GoBack) {
                // Back arrow — write only the selection prefix
                write!(self.label_bufs[li], "{}", prefix).unwrap();
                self.back_arrow_y = Some(LINE_Y[buf_idx]);
            } else {
                // Normal label
                let max_label = VISIBLE_WIDTH - 1;
                let label = if option.label.as_bytes().len() > max_label {
                    &option.label[..max_label]
                } else {
                    option.label
                };
                write!(self.label_bufs[li], "{}{}", prefix, label).unwrap();

                // Write value into a temp buffer, then copy to value_bufs
                let mut temp = FmtBuf::new();
                let vo = if selected { override_val } else { None };
                if self.write_option_value(option, &mut temp, vo) {
                    self.value_bufs[li].reset();
                    write!(self.value_bufs[li], "{}", temp.as_str()).unwrap();
                }
            }
        }
    }

    fn render_browsing(&mut self) {
        write!(self.title_buf, "{}", self.current_level().title).unwrap();
        self.render_options(None);
    }

    fn render_editing_value(&mut self, _setting: SettingKey, working_value: u32) {
        write!(self.title_buf, "{}", self.current_level().title).unwrap();
        self.render_options(Some(working_value));
    }

    fn render_editing_keybinding(&mut self, _button: ButtonCode, working_key_idx: usize) {
        write!(self.title_buf, "{}", self.current_level().title).unwrap();
        // Convert array index back to actual key code for the override
        let key = VALID_KEYS[working_key_idx] as u32;
        self.render_options(Some(key));
    }

    /// Draws title (left), labels (left), values (right), highlight box, and separator.
    fn draw_menu_text(&mut self, highlight_line: Option<usize>) {
        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        // Title — left-aligned
        let title = self.title_buf.as_str();
        if !title.is_empty() {
            Text::with_baseline(
                title,
                Point::new(0, LINE_Y[0]),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
        }

        // Option labels — left-aligned on lines 1–3
        for i in 0..3 {
            let s = self.label_bufs[i].as_str();
            if !s.is_empty() {
                Text::with_baseline(
                    s,
                    Point::new(0, LINE_Y[i + 1]),
                    self.text_style,
                    Baseline::Top,
                )
                .draw(self.display)
                .unwrap();
            }
            // Values — right-aligned on lines 1–3
            let v = self.value_bufs[i].as_str();
            if !v.is_empty() {
                let mut text = Text::with_alignment(
                    v,
                    Point::new(124, LINE_Y[i + 1]),
                    self.text_style,
                    Alignment::Right,
                );
                text.text_style.baseline = Baseline::Top;
                text.draw(self.display).unwrap();
            }
        }

        if let Some(line) = highlight_line {
            let v = self.value_bufs[line - 1].as_str();
            let value_len = v.as_bytes().len();
            // Box from pixel 127 to 2px left of the value's first character
            let box_left = 123 - (value_len as i32 * CHAR_W);
            let box_left = if box_left < 0 { 0 } else { box_left };
            Rectangle::new(
                Point::new(box_left, LINE_Y[line] + 1),
                Size::new((DISPLAY_R - box_left) as u32, 16),
            )
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(self.display)
            .unwrap();
        }

        if let Some(y) = self.back_arrow_y {
            for &(dx, dy) in &BACK_ARROW_PIXELS {
                Pixel(Point::new(CHAR_W + dx, y + dy + 6), BinaryColor::On)
                    .draw(self.display)
                    .unwrap();
            }
            // "Back" label 3 px right of the arrow's right edge
            Text::with_baseline(
                "Back",
                Point::new(CHAR_W + 14 + 3, y),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
        }

        Line::new(
            Point::new(0, LINE_Y[0] + 16),
            Point::new(DISPLAY_R, LINE_Y[0] + 16),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(self.display)
        .unwrap();

        self.display.flush().unwrap();
    }

    // ── Navigation helpers ──

    fn execute_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::OpenSubmenu(level) => {
                debug!("menu: enter {}", level.title);
                self.push_level(level);
            }
            MenuAction::GoBack => {
                debug!("menu: back");
                self.pop_level();
            }
            MenuAction::None => {}
            MenuAction::ShowDebugScreen => {
                debug!("menu: debug screen");
                self.state = MenuSubState::DisplayMode(MenuMode::Debug);
            }
            MenuAction::ShowPixelTest => {
                debug!("menu: pixel test");
                self.state = MenuSubState::DisplayMode(MenuMode::PixelTest);
            }
            MenuAction::EditSetting(key) => {
                let value = self.read_setting(key);
                debug!("menu: edit setting");
                self.state = MenuSubState::EditingValue {
                    setting: key,
                    original_value: value,
                    working_value: value,
                };
            }
            MenuAction::EditKeyBinding(code) => {
                let key = self.read_key_binding(code);
                let idx = key_index(key);
                debug!("menu: edit key bind");
                self.state = MenuSubState::EditingKeyBinding {
                    button: code,
                    working_key_idx: idx,
                    original_key_idx: idx,
                };
            }
            MenuAction::OpenWikiEdit(encoder) => {
                let t_val = self.read_setting(SettingKey::EncoderStepThreshold(encoder));
                let m_val = self.read_setting(SettingKey::EncoderMoveTimeout(encoder));
                debug!("menu: wiki edit P{}", encoder + 1);
                self.state = MenuSubState::WikiEdit {
                    encoder,
                    selected: 0,
                    editing: false,
                    working_threshold: t_val,
                    working_timeout: m_val,
                    original_threshold: t_val,
                    original_timeout: m_val,
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
