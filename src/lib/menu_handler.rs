//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

use crate::lighting_consts::{
    BG_MODE_NAMES, DIR_NAMES, FG_MODE_NAMES, OFFSET_NAMES, RAINBOW_NAMES, TRIG_MODE_NAMES,
};
use crate::{
    BUF_SIZE, ButtonCode, DEFAULT_BUTTON_DEBOUNCE_TICKS, FlashStoragePersistentMemory, NUM_BUTTONS,
    NUM_ENCODERS, OledDisplay,
};
use core::fmt::Write;
use core::sync::atomic::Ordering;
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
const MAX_MENU_DEPTH: usize = 6;

/// Width of a single character in the FONT_9X18_BOLD font (pixels).
const CHAR_W: i32 = 9;

/// OLED display width in pixels.
const DISPLAY_W: u32 = 128;

/// Rightmost pixel column on the display.
const DISPLAY_R: i32 = (DISPLAY_W - 1) as i32;

/// All defined USB HID keyboard usage codes (0-231, skipping reserved range 165-223).
static VALID_KEYS: &[u8] = &[
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
static ARROW_GRAPHIC_PIXELS: [(i32, i32); 12] = [
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
static BUTTON_GRAPHIC: [u8; 16 * 26] = [
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
static BACK_ARROW_PIXELS: [(i32, i32); 48] = [
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
static BUTTON_DEBUG_RECTANGLES: [Rectangle; NUM_BUTTONS] = [
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
    // Lighting — "All" variants apply to both players
    AllBgMode,
    AllBgRainbow,
    AllBgSpd,
    AllBgSubdiv,
    AllFgMode,
    AllFgRainbow,
    AllFgSpd,
    AllFgSubdiv,
    AllFgStep,
    AllFgSize,
    AllTrigMode,
    AllTrigRainbow,
    AllTrigFdIn,
    AllTrigFdOut,
    AllTrigSize,
    AllTrigDir,
    AllTrigOffset,
    AllTrigDur,
    // Lighting — per-player
    PlayerBgMode(usize),
    PlayerBgRainbow(usize),
    PlayerBgSpd(usize),
    PlayerBgSubdiv(usize),
    PlayerFgMode(usize),
    PlayerFgRainbow(usize),
    PlayerFgSpd(usize),
    PlayerFgSubdiv(usize),
    PlayerFgStep(usize),
    PlayerFgSize(usize),
    PlayerTrigMode(usize),
    PlayerTrigRainbow(usize),
    PlayerTrigFdIn(usize),
    PlayerTrigFdOut(usize),
    PlayerTrigSize(usize),
    PlayerTrigDir(usize),
    PlayerTrigOffset(usize),
    PlayerTrigDur(usize),
    GlobalBrightness,
}

/// A generic editor bound to a specific value type.  New editor types (e.g.
/// pattern selection or bool toggles) can be uncommented here without
/// touching [`MenuSubState`] or the event-dispatch machinery.
#[derive(Clone, Copy)]
enum Editor {
    IntRange {
        value: u32,
        step: u32,
        min: u32,
        max: u32,
        divisor: u32,
        unit: &'static str,
    },
    OptionSelect {
        labels: &'static [&'static str],
        current: usize,
    },
    // BoolToggle(bool),
}

/// Describes what to write back when an [`Editor`] is committed.
#[derive(Clone, Copy)]
enum Commit {
    Setting(SettingKey),
    // NoOp,
}

/// Keys whose value can be edited via [`Editor::IntRange`].  Maps 1:1 to
/// [`SettingKey`] entries that have a [`SettingMeta`].
#[derive(Clone, Copy)]
enum ValueKey {
    AllButtonDebounce,
    ButtonDebounce(ButtonCode),
    EncoderDebounce(usize),
    // Lighting
    AllBgMode,
    AllBgRainbow,
    AllBgSpd,
    AllBgSubdiv,
    AllFgMode,
    AllFgRainbow,
    AllFgSpd,
    AllFgSubdiv,
    AllFgStep,
    AllFgSize,
    AllTrigMode,
    AllTrigRainbow,
    AllTrigFdIn,
    AllTrigFdOut,
    AllTrigSize,
    AllTrigDir,
    AllTrigOffset,
    AllTrigDur,
    PlayerBgMode(usize),
    PlayerBgRainbow(usize),
    PlayerBgSpd(usize),
    PlayerBgSubdiv(usize),
    PlayerFgMode(usize),
    PlayerFgRainbow(usize),
    PlayerFgSpd(usize),
    PlayerFgSubdiv(usize),
    PlayerFgStep(usize),
    PlayerFgSize(usize),
    PlayerTrigMode(usize),
    PlayerTrigRainbow(usize),
    PlayerTrigFdIn(usize),
    PlayerTrigFdOut(usize),
    PlayerTrigSize(usize),
    PlayerTrigDir(usize),
    PlayerTrigOffset(usize),
    PlayerTrigDur(usize),
    GlobalBrightness,
}

impl From<ValueKey> for SettingKey {
    fn from(vk: ValueKey) -> Self {
        match vk {
            ValueKey::AllButtonDebounce => SettingKey::AllButtonDebounce,
            ValueKey::ButtonDebounce(c) => SettingKey::ButtonDebounce(c),
            ValueKey::EncoderDebounce(i) => SettingKey::EncoderDebounce(i),
            ValueKey::AllBgMode => SettingKey::AllBgMode,
            ValueKey::AllBgRainbow => SettingKey::AllBgRainbow,
            ValueKey::AllBgSpd => SettingKey::AllBgSpd,
            ValueKey::AllBgSubdiv => SettingKey::AllBgSubdiv,
            ValueKey::AllFgMode => SettingKey::AllFgMode,
            ValueKey::AllFgRainbow => SettingKey::AllFgRainbow,
            ValueKey::AllFgSpd => SettingKey::AllFgSpd,
            ValueKey::AllFgSubdiv => SettingKey::AllFgSubdiv,
            ValueKey::AllFgStep => SettingKey::AllFgStep,
            ValueKey::AllFgSize => SettingKey::AllFgSize,
            ValueKey::AllTrigMode => SettingKey::AllTrigMode,
            ValueKey::AllTrigRainbow => SettingKey::AllTrigRainbow,
            ValueKey::AllTrigFdIn => SettingKey::AllTrigFdIn,
            ValueKey::AllTrigFdOut => SettingKey::AllTrigFdOut,
            ValueKey::AllTrigSize => SettingKey::AllTrigSize,
            ValueKey::AllTrigDir => SettingKey::AllTrigDir,
            ValueKey::AllTrigOffset => SettingKey::AllTrigOffset,
            ValueKey::AllTrigDur => SettingKey::AllTrigDur,
            ValueKey::PlayerBgMode(p) => SettingKey::PlayerBgMode(p),
            ValueKey::PlayerBgRainbow(p) => SettingKey::PlayerBgRainbow(p),
            ValueKey::PlayerBgSpd(p) => SettingKey::PlayerBgSpd(p),
            ValueKey::PlayerBgSubdiv(p) => SettingKey::PlayerBgSubdiv(p),
            ValueKey::PlayerFgMode(p) => SettingKey::PlayerFgMode(p),
            ValueKey::PlayerFgRainbow(p) => SettingKey::PlayerFgRainbow(p),
            ValueKey::PlayerFgSpd(p) => SettingKey::PlayerFgSpd(p),
            ValueKey::PlayerFgSubdiv(p) => SettingKey::PlayerFgSubdiv(p),
            ValueKey::PlayerFgStep(p) => SettingKey::PlayerFgStep(p),
            ValueKey::PlayerFgSize(p) => SettingKey::PlayerFgSize(p),
            ValueKey::PlayerTrigMode(p) => SettingKey::PlayerTrigMode(p),
            ValueKey::PlayerTrigRainbow(p) => SettingKey::PlayerTrigRainbow(p),
            ValueKey::PlayerTrigFdIn(p) => SettingKey::PlayerTrigFdIn(p),
            ValueKey::PlayerTrigFdOut(p) => SettingKey::PlayerTrigFdOut(p),
            ValueKey::PlayerTrigSize(p) => SettingKey::PlayerTrigSize(p),
            ValueKey::PlayerTrigDir(p) => SettingKey::PlayerTrigDir(p),
            ValueKey::PlayerTrigOffset(p) => SettingKey::PlayerTrigOffset(p),
            ValueKey::PlayerTrigDur(p) => SettingKey::PlayerTrigDur(p),
            ValueKey::GlobalBrightness => SettingKey::GlobalBrightness,
        }
    }
}

/// What happens when a menu option is activated
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum MenuAction {
    OpenSubmenu(&'static MenuLevel),
    GoBack,
    /// Switch to a debug/test display mode (exit via specific keys).
    ShowDebugScreen,
    /// Switch to pixel-fill test mode (any key exits).
    ShowPixelTest,
    /// Enter in-place value adjustment for a named setting.
    EditValue(ValueKey),
    /// Enter key-binding cycle for a physical button or encoder direction.
    EditKeyBinding(ButtonCode),
    /// Open the dedicated encoder-editing screen for wiki sensitivity.
    OpenWikiEdit(usize),
    /// Save current settings to flash and reboot the chip.
    SaveAndReboot,
    /// Dismiss the save prompt without saving.
    Discard,
    /// Reset all settings to factory defaults, with confirmation prompt.
    ResetDefaults,
    /// Internal — perform the actual reset after user confirms.
    PerformReset,
    /// Reboot the chip without saving or clearing settings.
    Reboot,
    /// Show a list of all settings that differ from factory defaults.
    ShowCustom,
    /// Internal — return to the `ShowCustom` menu (no-op for prompts).
    ReturnToCustom,
    /// Internal — reset one specific field to its default value.
    ResetField(usize),
    /// Visible but non-functional
    None,
}

/// Describes a single field in [`FlashStoragePersistentMemory`] that differs
/// from its factory-default value. Used by [`for_each_changed_field`] to
/// unify the iteration pattern across counting, formatting, and display.
#[derive(Clone, Copy)]
enum FieldDescriptor {
    ButtonDebounce(usize),
    ButtonKey(usize),
    EncoderKeyUp(usize),
    EncoderKeyDown(usize),
    EncoderDebounce(usize),
    EncoderStepThreshold(usize),
    EncoderMoveTimeout(usize),
    // Lighting
    PlayerBgMode(usize),
    PlayerBgRainbow(usize),
    PlayerBgSpd(usize),
    PlayerBgSubdiv(usize),
    PlayerFgMode(usize),
    PlayerFgRainbow(usize),
    PlayerFgSpd(usize),
    PlayerFgSubdiv(usize),
    PlayerFgStep(usize),
    PlayerFgSize(usize),
    PlayerTrigMode(usize),
    PlayerTrigRainbow(usize),
    PlayerTrigFdIn(usize),
    PlayerTrigFdOut(usize),
    PlayerTrigSize(usize),
    PlayerTrigDir(usize),
    PlayerTrigOffset(usize),
    PlayerTrigDur(usize),
    PlayerBrightness(usize),
}

impl FieldDescriptor {
    fn section_title(&self) -> &'static str {
        match self {
            Self::ButtonDebounce(_) => "Button Debounce",
            Self::ButtonKey(_) => "Button Keys",
            Self::EncoderKeyUp(_) | Self::EncoderKeyDown(_) => "Encoder Keys",
            Self::EncoderDebounce(_) => "Encoder Debounce",
            Self::EncoderStepThreshold(_) => "Encoder Threshold",
            Self::EncoderMoveTimeout(_) => "Encoder Timeout",
            Self::PlayerBgMode(p) => {
                if *p == 0 {
                    "P1 BG"
                } else {
                    "P2 BG"
                }
            }
            Self::PlayerFgMode(p) => {
                if *p == 0 {
                    "P1 FG"
                } else {
                    "P2 FG"
                }
            }
            Self::PlayerTrigMode(p) => {
                if *p == 0 {
                    "P1 Trig"
                } else {
                    "P2 Trig"
                }
            }
            Self::PlayerBrightness(p) => {
                if *p == 0 {
                    "P1 Bright"
                } else {
                    "P2 Bright"
                }
            }
            Self::PlayerBgRainbow(p) => {
                if *p == 0 {
                    "P1 BgRnb"
                } else {
                    "P2 BgRnb"
                }
            }
            Self::PlayerBgSpd(p) => {
                if *p == 0 {
                    "P1 BgSpd"
                } else {
                    "P2 BgSpd"
                }
            }
            Self::PlayerBgSubdiv(p) => {
                if *p == 0 {
                    "P1 BgSub"
                } else {
                    "P2 BgSub"
                }
            }
            Self::PlayerFgSubdiv(p) => {
                if *p == 0 {
                    "P1 FgSub"
                } else {
                    "P2 FgSub"
                }
            }
            Self::PlayerFgRainbow(p) => {
                if *p == 0 {
                    "P1 FgRnb"
                } else {
                    "P2 FgRnb"
                }
            }
            Self::PlayerFgSpd(p) => {
                if *p == 0 {
                    "P1 FgSpd"
                } else {
                    "P2 FgSpd"
                }
            }
            Self::PlayerFgStep(p) => {
                if *p == 0 {
                    "P1 FgStp"
                } else {
                    "P2 FgStp"
                }
            }
            Self::PlayerFgSize(p) => {
                if *p == 0 {
                    "P1 FgSz"
                } else {
                    "P2 FgSz"
                }
            }
            Self::PlayerTrigRainbow(p) => {
                if *p == 0 {
                    "P1 TrRnb"
                } else {
                    "P2 TrRnb"
                }
            }
            Self::PlayerTrigFdIn(p) => {
                if *p == 0 {
                    "P1 FdIn"
                } else {
                    "P2 FdIn"
                }
            }
            Self::PlayerTrigFdOut(p) => {
                if *p == 0 {
                    "P1 FdOut"
                } else {
                    "P2 FdOut"
                }
            }
            Self::PlayerTrigSize(p) => {
                if *p == 0 {
                    "P1 TrSz"
                } else {
                    "P2 TrSz"
                }
            }
            Self::PlayerTrigDir(p) => {
                if *p == 0 {
                    "P1 TrDr"
                } else {
                    "P2 TrDr"
                }
            }
            Self::PlayerTrigOffset(p) => {
                if *p == 0 {
                    "P1 TrOf"
                } else {
                    "P2 TrOf"
                }
            }
            Self::PlayerTrigDur(p) => {
                if *p == 0 {
                    "P1 TrDu"
                } else {
                    "P2 TrDu"
                }
            }
        }
    }
}

// ── Lighting field macros ─────────────────────────────────────────
/// Generate a block expression for `read_setting` that handles all lighting
/// fields. Returns early if the key matches a lighting variant.
macro_rules! lighting_read_block {
    ($self:ident, $key:ident, [$([$all_vk:ident, $player_vk:ident, $field:ident]),+ $(,)?]) => {
        {
            match $key {
                $(
                    SettingKey::$all_vk => {
                        return $self.settings.lighting.players[0].$field as u32;
                    }
                    SettingKey::$player_vk(p) => {
                        return $self.settings.lighting.players[p].$field as u32;
                    }
                )+
                _ => {}
            }
        }
    }
}

/// Generate a block expression for `write_setting` that handles all lighting
/// fields. Modifies in place when the key matches a lighting variant.
macro_rules! lighting_write_block {
    ($self:ident, $key:ident, $value:ident, [$([$all_vk:ident, $player_vk:ident, $field:ident, $ty:ty]),+ $(,)?]) => {
        {
            match $key {
                $(
                    SettingKey::$all_vk => {
                        for p in 0..2 {
                            $self.settings.lighting.players[p].$field = $value as $ty;
                        }
                        return;
                    }
                    SettingKey::$player_vk(p) => {
                        $self.settings.lighting.players[p].$field = $value as $ty;
                        return;
                    }
                )+
                _ => {}
            }
        }
    }
}

/// Generate a block expression for `SettingKey::meta()` that handles all lighting
/// fields. Returns early with metadata if the key matches a lighting variant.
macro_rules! lighting_meta_block {
    ($key:ident, [$([$all_vk:ident, $player_vk:ident, $step:expr, $min:expr, $max:expr, $divisor:expr, $unit:expr]),+ $(,)?]) => {
        {
            match $key {
                $(
                    Self::$all_vk | Self::$player_vk(_) => {
                        return SettingMeta {
                            step: $step, min: $min, max: $max,
                            divisor: $divisor, unit: $unit,
                        };
                    }
                )+
                _ => {}
            }
        }
    }
}

/// Generate `check!()`-equivalent blocks inside `for_each_changed_field` for lighting
/// per-player fields.
macro_rules! lighting_for_each_check {
    ($p:ident, $settings:expr, $defaults:expr, $f:ident, $count:ident,
     [$([$player_vk:ident, $field:ident]),+ $(,)?]) => {
        $(
            {
                let cur = $settings.lighting.players[$p].$field as u64;
                let def = $defaults.lighting.players[$p].$field as u64;
                if cur != def {
                    if !$f(FieldDescriptor::$player_vk($p), cur, def) {
                        return $count + 1;
                    }
                    $count += 1;
                }
            }
        )+
    }
}

/// Generate `if`-blocks inside `reset_field_to_default` for lighting
/// per-player fields.  `$p` and `$idx` must be variables from the enclosing scope.
macro_rules! lighting_reset_checks {
    ($self:ident, $p:ident, $idx:ident, $defaults:ident, $target_idx:ident, [$([$player_vk:ident, $field:ident]),+ $(,)?]) => {
        $(
            if $self.settings.lighting.players[$p].$field
                != $defaults.lighting.players[$p].$field
            {
                if $idx == $target_idx {
                    let key = SettingKey::$player_vk($p);
                    $self.write_setting(key, $defaults.lighting.players[$p].$field as u32);
                    return;
                }
                $idx += 1;
            }
        )+
    }
}

/// Walk every field in [`FlashStoragePersistentMemory`] in a fixed order,
/// calling `f(field, current_value, default_value)` for each field that
/// differs from its factory default.  `f` returns `true` to continue or
/// `false` to stop early.
///
/// Returns the total number of changed fields (the number of calls made to `f`
/// unless iteration was stopped early).
fn for_each_changed_field(
    settings: &FlashStoragePersistentMemory,
    defaults: &FlashStoragePersistentMemory,
    mut f: impl FnMut(FieldDescriptor, u64, u64) -> bool,
) -> usize {
    let mut count = 0;

    macro_rules! check {
        ($current:expr, $default:expr, $desc:expr) => {
            let cur = $current as u64;
            let def = $default as u64;
            if cur != def {
                if !f($desc, cur, def) {
                    return count + 1;
                }
                count += 1;
            }
        };
    }

    for b in 0..NUM_BUTTONS {
        check!(
            settings.buttons[b].debounce_ticks,
            defaults.buttons[b].debounce_ticks,
            FieldDescriptor::ButtonDebounce(b)
        );
    }
    for b in 0..NUM_BUTTONS {
        check!(
            settings.buttons[b].key,
            defaults.buttons[b].key,
            FieldDescriptor::ButtonKey(b)
        );
    }
    for e in 0..NUM_ENCODERS {
        check!(
            settings.encoders[e].key_up,
            defaults.encoders[e].key_up,
            FieldDescriptor::EncoderKeyUp(e)
        );
        check!(
            settings.encoders[e].key_down,
            defaults.encoders[e].key_down,
            FieldDescriptor::EncoderKeyDown(e)
        );
        check!(
            settings.encoders[e].debounce_ticks,
            defaults.encoders[e].debounce_ticks,
            FieldDescriptor::EncoderDebounce(e)
        );
        check!(
            settings.encoders[e].step_threshold,
            defaults.encoders[e].step_threshold,
            FieldDescriptor::EncoderStepThreshold(e)
        );
        check!(
            settings.encoders[e].move_timeout_ticks,
            defaults.encoders[e].move_timeout_ticks,
            FieldDescriptor::EncoderMoveTimeout(e)
        );
    }

    // Lighting per-player fields
    for p in 0..2_usize {
        lighting_for_each_check!(
            p,
            settings,
            defaults,
            f,
            count,
            [
                [PlayerBgMode, bg_mode],
                [PlayerBgRainbow, bg_rainbow],
                [PlayerBgSpd, bg_speed_ds],
                [PlayerBgSubdiv, bg_subdivisions],
                [PlayerFgMode, fg_mode],
                [PlayerFgRainbow, fg_rainbow],
                [PlayerFgSpd, fg_speed_ds],
                [PlayerFgSubdiv, fg_subdivisions],
                [PlayerFgStep, fg_step_ds],
                [PlayerFgSize, fg_px_per_group],
                [PlayerTrigMode, trig_mode],
                [PlayerTrigRainbow, trig_rainbow],
                [PlayerTrigFdIn, trig_fade_in_ms],
                [PlayerTrigFdOut, trig_fade_out_ms],
                [PlayerTrigSize, trig_width],
                [PlayerTrigDir, trig_dir],
                [PlayerTrigOffset, trig_offset],
                [PlayerTrigDur, trig_dur_s],
            ]
        );
    }
    // Global brightness
    check!(
        settings.lighting.brightness,
        defaults.lighting.brightness,
        FieldDescriptor::PlayerBrightness(0)
    );

    count
}

/// Which of the two choices in a `Prompt` is currently selected.
#[derive(Clone, Copy)]
enum PromptSide {
    First,
    Second,
}

/// One option in a `Prompt` — an action and its display label.
#[derive(Clone, Copy)]
struct PromptChoice {
    action: MenuAction,
    label: FmtBuf,
}

/// Encoder wiki-editing state, extracted into its own struct to avoid
/// verbose field-by-field reconstruction inside match arms.
//
// `original_*` fields are preserved through `..*w` struct-update syntax,
// not read individually, hence the dead_code allow.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct WikiEditState {
    encoder: usize,
    selected: usize,
    editing: bool,
    working_threshold: u32,
    working_timeout: u32,
    original_threshold: u32,
    original_timeout: u32,
}

/// Top-level state machine controlling what the handler does with inputs
/// and how it renders the screen.
#[derive(Clone, Copy)]
enum MenuSubState {
    Browsing,
    Editing {
        editor: Editor,
        commit: Commit,
    },
    EditingKeyBinding {
        button: ButtonCode,
        working_key_idx: usize,
        original_key_idx: usize,
    },
    WikiEdit(WikiEditState),
    DisplayMode(MenuMode),
    IdleMode,
    /// General-purpose confirmation prompt with up to 3 lines of text
    /// and two selectable choices.
    Prompt {
        lines: [FmtBuf; 3],
        choices: [PromptChoice; 2],
        selection: PromptSide,
    },
    /// Scrollable list of settings that differ from factory defaults.
    ShowCustom {
        cursor: usize,
    },
}

/// Increment or decrement `val` by `step`, clamping to [`min`, `max`].
fn clamp_step(val: u32, step: u32, min: u32, max: u32, up: bool) -> u32 {
    if up {
        val.saturating_add(step).clamp(min, max)
    } else {
        val.saturating_sub(step).clamp(min, max)
    }
}

/// Build an [`Editor`] + [`Commit`] pair for a [`ValueKey`] by reading the
/// current value from `settings` and looking up the adjustment metadata.
fn build_editor(settings: &FlashStoragePersistentMemory, vk: ValueKey) -> (Editor, Commit) {
    let key: SettingKey = vk.into();
    match key {
        // OptionSelect keys — return early
        SettingKey::AllBgMode => {
            let current = settings.lighting.players[0].bg_mode as usize;
            (
                Editor::OptionSelect {
                    labels: BG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerBgMode(p) => {
            let current = settings.lighting.players[p].bg_mode as usize;
            (
                Editor::OptionSelect {
                    labels: BG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllBgRainbow => {
            let current = settings.lighting.players[0].bg_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerBgRainbow(p) => {
            let current = settings.lighting.players[p].bg_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllFgMode => {
            let current = settings.lighting.players[0].fg_mode as usize;
            (
                Editor::OptionSelect {
                    labels: FG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerFgMode(p) => {
            let current = settings.lighting.players[p].fg_mode as usize;
            (
                Editor::OptionSelect {
                    labels: FG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllFgRainbow => {
            let current = settings.lighting.players[0].fg_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerFgRainbow(p) => {
            let current = settings.lighting.players[p].fg_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigMode => {
            let current = settings.lighting.players[0].trig_mode as usize;
            (
                Editor::OptionSelect {
                    labels: TRIG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerTrigMode(p) => {
            let current = settings.lighting.players[p].trig_mode as usize;
            (
                Editor::OptionSelect {
                    labels: TRIG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigRainbow => {
            let current = settings.lighting.players[0].trig_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerTrigRainbow(p) => {
            let current = settings.lighting.players[p].trig_rainbow as usize;
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigDir => {
            let current = settings.lighting.players[0].trig_dir as usize;
            (
                Editor::OptionSelect {
                    labels: DIR_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerTrigDir(p) => {
            let current = settings.lighting.players[p].trig_dir as usize;
            (
                Editor::OptionSelect {
                    labels: DIR_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigOffset => {
            let current = settings.lighting.players[0].trig_offset as usize;
            (
                Editor::OptionSelect {
                    labels: OFFSET_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::PlayerTrigOffset(p) => {
            let current = settings.lighting.players[p].trig_offset as usize;
            (
                Editor::OptionSelect {
                    labels: OFFSET_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        // IntRange keys — fall through to meta-based editor
        _ => {
            let meta = key.meta();
            let value = match key {
                SettingKey::AllButtonDebounce => settings.buttons[0].debounce_ticks as u32,
                SettingKey::ButtonDebounce(code) => {
                    settings.buttons[code as usize].debounce_ticks as u32
                }
                SettingKey::EncoderDebounce(idx) => settings.encoders[idx].debounce_ticks as u32,
                SettingKey::EncoderStepThreshold(idx) => {
                    settings.encoders[idx].step_threshold as u32
                }
                SettingKey::EncoderMoveTimeout(idx) => {
                    settings.encoders[idx].move_timeout_ticks as u32
                }
                SettingKey::AllBgSpd => settings.lighting.players[0].bg_speed_ds as u32,
                SettingKey::AllBgSubdiv => settings.lighting.players[0].bg_subdivisions as u32,
                SettingKey::AllFgSubdiv => settings.lighting.players[0].fg_subdivisions as u32,
                SettingKey::AllFgSpd => settings.lighting.players[0].fg_speed_ds as u32,
                SettingKey::AllFgStep => settings.lighting.players[0].fg_step_ds as u32,
                SettingKey::AllFgSize => settings.lighting.players[0].fg_px_per_group as u32,
                SettingKey::AllTrigFdIn => settings.lighting.players[0].trig_fade_in_ms as u32,
                SettingKey::AllTrigFdOut => settings.lighting.players[0].trig_fade_out_ms as u32,
                SettingKey::AllTrigSize => settings.lighting.players[0].trig_width as u32,
                SettingKey::AllTrigDur => settings.lighting.players[0].trig_dur_s as u32,
                SettingKey::PlayerBgSpd(p) => settings.lighting.players[p].bg_speed_ds as u32,
                SettingKey::PlayerBgSubdiv(p) => {
                    settings.lighting.players[p].bg_subdivisions as u32
                }
                SettingKey::PlayerFgSubdiv(p) => {
                    settings.lighting.players[p].fg_subdivisions as u32
                }
                SettingKey::PlayerFgSpd(p) => settings.lighting.players[p].fg_speed_ds as u32,
                SettingKey::PlayerFgStep(p) => settings.lighting.players[p].fg_step_ds as u32,
                SettingKey::PlayerFgSize(p) => settings.lighting.players[p].fg_px_per_group as u32,
                SettingKey::PlayerTrigFdIn(p) => {
                    settings.lighting.players[p].trig_fade_in_ms as u32
                }
                SettingKey::PlayerTrigFdOut(p) => {
                    settings.lighting.players[p].trig_fade_out_ms as u32
                }
                SettingKey::PlayerTrigSize(p) => settings.lighting.players[p].trig_width as u32,
                SettingKey::PlayerTrigDur(p) => settings.lighting.players[p].trig_dur_s as u32,
                SettingKey::GlobalBrightness => settings.lighting.brightness as u32,
                // These will never be hit because they return early above,
                // but the match must be exhaustive:
                SettingKey::AllBgMode
                | SettingKey::PlayerBgMode(_)
                | SettingKey::AllBgRainbow
                | SettingKey::PlayerBgRainbow(_)
                | SettingKey::AllFgMode
                | SettingKey::PlayerFgMode(_)
                | SettingKey::AllFgRainbow
                | SettingKey::PlayerFgRainbow(_)
                | SettingKey::AllTrigMode
                | SettingKey::PlayerTrigMode(_)
                | SettingKey::AllTrigRainbow
                | SettingKey::PlayerTrigRainbow(_)
                | SettingKey::AllTrigDir
                | SettingKey::PlayerTrigDir(_)
                | SettingKey::AllTrigOffset
                | SettingKey::PlayerTrigOffset(_) => 0,
            };
            let editor = Editor::IntRange {
                value,
                step: meta.step,
                min: meta.min,
                max: meta.max,
                divisor: meta.divisor,
                unit: meta.unit,
            };
            (editor, Commit::Setting(key))
        }
    }
}

/// Execute a [`Commit`] by writing `value` into the appropriate field.
impl<'a, D: WriteOnlyDataCommand> MenuHandler<'a, D> {
    fn commit_edit(&mut self, commit: Commit, value: u32) {
        match commit {
            Commit::Setting(key) => self.write_setting(key, value),
            // Commit::NoOp => {}
        }
    }
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

/// Returns which item index (or `None` for `"--"`) appears on each of the 3
/// visible lines given a total item count and cursor position.
/// Returned tuples are `(line_y_index, Option<item_index>)` where `line_y_index`
/// is 1–3 (matching 0-based `label_bufs` index = `line_y_index - 1`).
fn option_line_indices(total: usize, cursor: usize) -> [(usize, Option<usize>); 3] {
    match total {
        0 => [(1, None), (2, None), (3, None)],
        1 => [(1, None), (2, Some(0)), (3, None)],
        2 => {
            if cursor == 0 {
                [(1, None), (2, Some(0)), (3, Some(1))]
            } else {
                [(1, Some(0)), (2, Some(1)), (3, None)]
            }
        }
        _ => [
            (1, if cursor == 0 { None } else { Some(cursor - 1) }),
            (2, Some(cursor)),
            (
                3,
                if cursor == total - 1 {
                    None
                } else {
                    Some(cursor + 1)
                },
            ),
        ],
    }
}

/// Generate lighting submenus for a zone (BG, FG, or Trig).
/// Produces three `static MenuLevel`s: one for All (writes to both players),
/// one for P1, one for P2.
macro_rules! lighting_submenu_zone {
    ($all_name:ident, $p1_name:ident, $p2_name:ident,
     $title:expr,
     [$([$label:expr, $all_vk:ident, $player_vk:ident]),+ $(,)?]) => {
        static $all_name: MenuLevel = MenuLevel {
            title: $title,
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$all_vk),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
        static $p1_name: MenuLevel = MenuLevel {
            title: concat!("P1 ", $title),
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$player_vk(0)),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
        static $p2_name: MenuLevel = MenuLevel {
            title: concat!("P2 ", $title),
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$player_vk(1)),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
    };
}

static ROOT_MENU: MenuLevel = MenuLevel {
    title: "IIDX Menu",
    options: &[
        MenuOption {
            label: "Lighting",
            action: MenuAction::OpenSubmenu(&LIGHTING_MENU),
        },
        MenuOption {
            label: "Debug",
            action: MenuAction::OpenSubmenu(&DEBUG_MENU),
        },
        MenuOption {
            label: "Settings",
            action: MenuAction::OpenSubmenu(&SETTINGS_MENU),
        },
        MenuOption {
            label: "System",
            action: MenuAction::OpenSubmenu(&SYSTEM_MENU),
        },
    ],
};

static SETTINGS_MENU: MenuLevel = MenuLevel {
    title: "Settings",
    options: &[
        MenuOption {
            label: "Wiki Config",
            action: MenuAction::OpenSubmenu(&ENCODER_SENS_MENU),
        },
        MenuOption {
            label: "Key Bindings",
            action: MenuAction::OpenSubmenu(&KEYBIND_MENU),
        },
        MenuOption {
            label: "Debounce",
            action: MenuAction::OpenSubmenu(&DEBOUNCE_MENU),
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

static SYSTEM_MENU: MenuLevel = MenuLevel {
    title: "System",
    options: &[
        MenuOption {
            label: "Save+Reboot",
            action: MenuAction::SaveAndReboot,
        },
        MenuOption {
            label: "Show Custom",
            action: MenuAction::ShowCustom,
        },
        MenuOption {
            label: "ResetDefault",
            action: MenuAction::ResetDefaults,
        },
        MenuOption {
            label: "Reboot",
            action: MenuAction::Reboot,
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

static LIGHTING_MENU: MenuLevel = MenuLevel {
    title: "Lighting",
    options: &[
        MenuOption {
            label: "Both",
            action: MenuAction::OpenSubmenu(&BOTH_MENU),
        },
        MenuOption {
            label: "P1",
            action: MenuAction::OpenSubmenu(&P1_MENU),
        },
        MenuOption {
            label: "P2",
            action: MenuAction::OpenSubmenu(&P2_MENU),
        },
        MenuOption {
            label: "Global",
            action: MenuAction::OpenSubmenu(&GLOBAL_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static BOTH_MENU: MenuLevel = MenuLevel {
    title: "Both",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_ALL_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_ALL_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_ALL_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static P1_MENU: MenuLevel = MenuLevel {
    title: "P1",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_P1_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_P1_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_P1_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

static P2_MENU: MenuLevel = MenuLevel {
    title: "P2",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_P2_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_P2_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_P2_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

lighting_submenu_zone!(
    BG_ALL_MENU,
    BG_P1_MENU,
    BG_P2_MENU,
    "BG",
    [
        ["Mode", AllBgMode, PlayerBgMode],
        ["Rainb", AllBgRainbow, PlayerBgRainbow],
        ["Spd", AllBgSpd, PlayerBgSpd],
        ["Subd", AllBgSubdiv, PlayerBgSubdiv],
    ]
);

lighting_submenu_zone!(
    FG_ALL_MENU,
    FG_P1_MENU,
    FG_P2_MENU,
    "FG",
    [
        ["Mode", AllFgMode, PlayerFgMode],
        ["Rainb", AllFgRainbow, PlayerFgRainbow],
        ["Spd", AllFgSpd, PlayerFgSpd],
        ["Subd", AllFgSubdiv, PlayerFgSubdiv],
        ["Step", AllFgStep, PlayerFgStep],
        ["Size", AllFgSize, PlayerFgSize],
    ]
);

lighting_submenu_zone!(
    TRIG_ALL_MENU,
    TRIG_P1_MENU,
    TRIG_P2_MENU,
    "Trig",
    [
        ["Mode", AllTrigMode, PlayerTrigMode],
        ["Rainb", AllTrigRainbow, PlayerTrigRainbow],
        ["Dir", AllTrigDir, PlayerTrigDir],
        ["Offset", AllTrigOffset, PlayerTrigOffset],
        ["Cycle", AllTrigDur, PlayerTrigDur],
        ["FdIn", AllTrigFdIn, PlayerTrigFdIn],
        ["FdOut", AllTrigFdOut, PlayerTrigFdOut],
        ["Size", AllTrigSize, PlayerTrigSize],
    ]
);

static GLOBAL_MENU: MenuLevel = MenuLevel {
    title: "Global",
    options: &[
        MenuOption {
            label: "Brght",
            action: MenuAction::EditValue(ValueKey::GlobalBrightness),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

macro_rules! define_button_menus {
    ($(($code:ident, $label:expr)),* $(,)?) => {
        static DEBOUNCE_MENU: MenuLevel = MenuLevel {
            title: "Debounce",
            options: &[
                MenuOption {
                    label: "All",
                    action: MenuAction::EditValue(ValueKey::AllButtonDebounce),
                },
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::ButtonDebounce(ButtonCode::$code)),
                }),*,
                MenuOption {
                    label: "Enc1",
                    action: MenuAction::EditValue(ValueKey::EncoderDebounce(0)),
                },
                MenuOption {
                    label: "Enc2",
                    action: MenuAction::EditValue(ValueKey::EncoderDebounce(1)),
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
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditKeyBinding(ButtonCode::$code),
                }),*,
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
    };
}

define_button_menus! {
    // Labels derived from ButtonCode::short_label() — single source of truth
    (P1_1, "P1_1"),
    (P1_2, "P1_2"),
    (P1_3, "P1_3"),
    (P1_4, "P1_4"),
    (P1_5, "P1_5"),
    (P1_6, "P1_6"),
    (P1_7, "P1_7"),
    (P1Start, "P1_St"),
    (P1Select, "P1_Sl"),
    (P2_1, "P2_1"),
    (P2_2, "P2_2"),
    (P2_3, "P2_3"),
    (P2_4, "P2_4"),
    (P2_5, "P2_5"),
    (P2_6, "P2_6"),
    (P2_7, "P2_7"),
    (P2Start, "P2_St"),
    (P2Select, "P2_Sl"),
    (Escape, "Esc"),
    (CcUp, "CC_Up"),
    (CcDown, "CC_Dn"),
    (CcLeft, "CC_Lt"),
    (CcRight, "CC_Rt"),
    (CcSelect, "CC_Sl"),
    (VolumeUp, "V_Up"),
    (VolumeDown, "V_Dn"),
    (Mute, "Mute"),
}

/// A tiny fixed-size buffer for formatting a single short line of text
/// (up to [`BUF_SIZE`] bytes) before drawing it to the OLED.
#[derive(Clone, Copy)]
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
        lighting_meta_block!(
            self,
            [
                [AllBgSpd, PlayerBgSpd, 1, 5, 600, 10, "s"],
                [AllBgSubdiv, PlayerBgSubdiv, 1, 1, 10, 1, ""],
                [AllFgSpd, PlayerFgSpd, 1, 5, 600, 10, "s"],
                [AllFgSubdiv, PlayerFgSubdiv, 1, 1, 10, 1, ""],
                [AllFgStep, PlayerFgStep, 1, 1, 200, 10, "s"],
                [AllFgSize, PlayerFgSize, 1, 1, 10, 1, "px"],
                [AllTrigFdIn, PlayerTrigFdIn, 50, 50, 2000, 1, "ms"],
                [AllTrigFdOut, PlayerTrigFdOut, 50, 50, 5000, 1, "ms"],
                [AllTrigSize, PlayerTrigSize, 1, 1, 10, 1, "px"],
                [AllTrigDur, PlayerTrigDur, 1, 1, 60, 1, "s"],
            ]
        );
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
            Self::GlobalBrightness => SettingMeta {
                step: 5,
                min: 0,
                max: 255,
                divisor: 1,
                unit: "",
            },
            // All the OptionSelect keys (modes, rainbows) fall through to a dummy meta
            // since they are never read via meta() — write_option_value handles them
            _ => SettingMeta {
                step: 1,
                min: 0,
                max: 1,
                divisor: 1,
                unit: "",
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
    /// True when any setting has been written to the RAM copy since boot.
    pub settings_changed: bool,
    /// Set to true when the user confirms "Save & Reboot".
    pub pending_reboot: bool,
    /// True once the save prompt has been shown and dismissed since the last change.
    /// Prevents the prompt from re-appearing on every back-navigation within a change batch.
    prompt_answered_since_change: bool,
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
            settings_changed: false,
            pending_reboot: false,
            prompt_answered_since_change: false,
            stack,
            stack_depth: 1,
            settings,
            all_debounce_value: DEFAULT_BUTTON_DEBOUNCE_TICKS as u32,
            back_arrow_y: None,
        }
    }

    /// If settings have changed since the last prompt dismissal, show the save
    /// prompt; otherwise run `fallback`.
    fn prompt_if_unsaved(&mut self, fallback: impl FnOnce(&mut Self)) {
        if self.settings_changed && !self.prompt_answered_since_change {
            self.state = Self::build_prompt(
                ["Save changes", "to flash and", "reboot?"],
                MenuAction::SaveAndReboot,
                MenuAction::Discard,
            );
        } else {
            fallback(self);
        }
    }

    /// Construct a `Prompt` with two choices: "Yes" (left, non-default) and "No"
    /// (right, pre-selected = default).
    fn build_prompt(
        lines: [&str; 3],
        yes_action: MenuAction,
        no_action: MenuAction,
    ) -> MenuSubState {
        let mut buf_lines = [FmtBuf::new(); 3];
        for (i, text) in lines.iter().enumerate() {
            core::write!(buf_lines[i], "{}", text).ok();
        }
        let mut yes_label = FmtBuf::new();
        core::write!(yes_label, "Yes").ok();
        let mut no_label = FmtBuf::new();
        core::write!(no_label, "No").ok();
        MenuSubState::Prompt {
            lines: buf_lines,
            choices: [
                PromptChoice {
                    action: yes_action,
                    label: yes_label,
                },
                PromptChoice {
                    action: no_action,
                    label: no_label,
                },
            ],
            selection: PromptSide::Second,
        }
    }

    pub fn process_event(&mut self, event: MenuEvents) {
        match event {
            MenuEvents::Press(button) => self.dispatch_press(button),
            MenuEvents::LongPress(button) => self.dispatch_long_press(button),
            MenuEvents::Repeat(button) => self.dispatch_repeat(button),
            MenuEvents::Idle => self.dispatch_idle(),
        }
    }

    fn dispatch_press(&mut self, button: ButtonCode) {
        match self.state {
            MenuSubState::Prompt { .. } => self.on_prompt_press(button),
            MenuSubState::ShowCustom { cursor } => self.on_custom_press(cursor, button),
            MenuSubState::IdleMode => self.on_idle_press(button),
            MenuSubState::DisplayMode(MenuMode::Debug) => {}
            MenuSubState::DisplayMode(MenuMode::PixelTest) => self.on_pixel_press(button),
            MenuSubState::Browsing => self.on_browsing_press(button),
            MenuSubState::Editing { .. } => self.on_editing_press(button),
            MenuSubState::EditingKeyBinding { .. } => self.on_keybinding_press(button),
            MenuSubState::WikiEdit(ref w) => {
                let wiki_state = *w;
                self.on_wiki_press(&wiki_state, button);
            }
        }
    }

    fn dispatch_long_press(&mut self, button: ButtonCode) {
        match self.state {
            MenuSubState::IdleMode => self.on_idle_long_press(button),
            MenuSubState::DisplayMode(MenuMode::Debug) => self.on_debug_long_press(button),
            MenuSubState::DisplayMode(MenuMode::PixelTest) => self.on_pixel_long_press(button),
            MenuSubState::Prompt { .. } => {} // press-and-hold must not re-trigger prompt selection
            MenuSubState::ShowCustom { .. } => {
                self.process_event(MenuEvents::Press(button));
            }
            _ => self.handle_repeat_as_press(button),
        }
    }

    fn dispatch_repeat(&mut self, button: ButtonCode) {
        match self.state {
            MenuSubState::Prompt { .. } => {} // repeat must not re-trigger prompt selection
            MenuSubState::ShowCustom { .. } => {
                self.process_event(MenuEvents::Press(button));
            }
            MenuSubState::IdleMode
                if matches!(
                    button,
                    ButtonCode::CcUp
                        | ButtonCode::CcDown
                        | ButtonCode::CcLeft
                        | ButtonCode::CcRight
                        | ButtonCode::CcSelect
                ) =>
            {
                debug!("menu: exit idle");
                self.state = self.saved_state;
            }
            _ => self.handle_repeat_as_press(button),
        }
    }

    fn dispatch_idle(&mut self) {
        if !matches!(
            self.state,
            MenuSubState::IdleMode | MenuSubState::DisplayMode(_)
        ) {
            debug!("menu: idle");
            self.saved_state = self.state;
            self.state = MenuSubState::IdleMode;
        }
    }

    // ── Press handlers ──

    fn on_prompt_press(&mut self, button: ButtonCode) {
        if let MenuSubState::Prompt {
            lines,
            choices,
            selection,
        } = self.state
        {
            match button {
                ButtonCode::CcUp
                | ButtonCode::CcDown
                | ButtonCode::CcLeft
                | ButtonCode::CcRight => {
                    let new_sel = match selection {
                        PromptSide::First => PromptSide::Second,
                        PromptSide::Second => PromptSide::First,
                    };
                    self.state = MenuSubState::Prompt {
                        lines,
                        choices,
                        selection: new_sel,
                    };
                }
                ButtonCode::CcSelect => {
                    self.prompt_answered_since_change = true;
                    let action = match selection {
                        PromptSide::First => choices[0].action,
                        PromptSide::Second => choices[1].action,
                    };
                    self.execute_action(action);
                }
                _ => {}
            }
        }
    }

    fn on_custom_press(&mut self, cursor: usize, button: ButtonCode) {
        let change_count = self.count_changes();
        if change_count == 0 {
            self.state = MenuSubState::Browsing;
            return;
        }
        match button {
            ButtonCode::CcLeft => {
                if self.stack_depth > 1 {
                    self.prompt_if_unsaved(|s| {
                        s.state = MenuSubState::Browsing;
                    });
                } else {
                    self.state = MenuSubState::Browsing;
                }
            }
            ButtonCode::CcUp => {
                let new_cursor = (cursor + change_count - 1) % change_count;
                self.state = MenuSubState::ShowCustom { cursor: new_cursor };
            }
            ButtonCode::CcDown => {
                let new_cursor = (cursor + 1) % change_count;
                self.state = MenuSubState::ShowCustom { cursor: new_cursor };
            }
            ButtonCode::CcRight | ButtonCode::CcSelect => {
                self.state = Self::build_prompt(
                    ["Reset value", "to default", "state?"],
                    MenuAction::ResetField(cursor),
                    MenuAction::ReturnToCustom,
                );
            }
            _ => {}
        }
    }

    fn on_idle_press(&mut self, button: ButtonCode) {
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

    fn on_pixel_press(&mut self, _button: ButtonCode) {
        debug!("menu: exit pixel test");
        self.state = MenuSubState::Browsing;
    }

    fn on_browsing_press(&mut self, button: ButtonCode) {
        let level = self.current_level();
        let cursor = self.current_cursor();
        match button {
            ButtonCode::CcUp => {
                let len = level.options.len();
                let new_cursor = (cursor + len - 1) % len;
                debug!("menu: cursor up \u{2192} {}", new_cursor);
                *self.current_cursor_mut() = new_cursor;
            }
            ButtonCode::CcDown => {
                let len = level.options.len();
                let new_cursor = (cursor + 1) % len;
                debug!("menu: cursor down \u{2192} {}", new_cursor);
                *self.current_cursor_mut() = new_cursor;
            }
            ButtonCode::CcSelect | ButtonCode::CcRight => {
                self.execute_action(level.options[cursor].action);
            }
            ButtonCode::CcLeft => {
                if self.stack_depth > 1 {
                    self.prompt_if_unsaved(|s| s.pop_level());
                } else {
                    self.pop_level();
                }
            }
            _ => {}
        }
    }

    fn on_editing_press(&mut self, button: ButtonCode) {
        // Work on a local copy to avoid borrowing conflicts with self calls.
        let mut editing = match self.state {
            MenuSubState::Editing { editor, commit } => Some((editor, commit)),
            _ => None,
        };
        if let Some((ref mut editor, commit)) = editing {
            let mut do_commit = None;
            match editor {
                Editor::IntRange {
                    value,
                    step,
                    min,
                    max,
                    divisor,
                    unit,
                } => match button {
                    ButtonCode::CcLeft | ButtonCode::CcDown => {
                        let new_val = clamp_step(*value, *step, *min, *max, false);
                        if new_val != *value {
                            debug!("menu: edit {}\u{2193} {}", new_val / *divisor, unit);
                            *value = new_val;
                        }
                    }
                    ButtonCode::CcRight | ButtonCode::CcUp => {
                        let new_val = clamp_step(*value, *step, *min, *max, true);
                        if new_val != *value {
                            debug!("menu: edit {}\u{2191} {}", new_val / *divisor, unit);
                            *value = new_val;
                        }
                    }
                    ButtonCode::CcSelect => {
                        debug!("menu: commit {} {}", *value / *divisor, unit);
                        do_commit = Some((commit, *value));
                    }
                    _ => {}
                },
                Editor::OptionSelect { labels, current } => match button {
                    ButtonCode::CcUp | ButtonCode::CcRight => {
                        *current = (*current + 1) % labels.len();
                    }
                    ButtonCode::CcDown | ButtonCode::CcLeft => {
                        *current = (*current + labels.len() - 1) % labels.len();
                    }
                    ButtonCode::CcSelect => {
                        do_commit = Some((commit, *current as u32));
                    }
                    _ => {}
                },
                // Editor::BoolToggle(v) => ...,
            }
            if let Some((commit, val)) = do_commit {
                self.commit_edit(commit, val);
                self.state = MenuSubState::Browsing;
            } else {
                // Write the modified editor back so the display updates.
                self.state = MenuSubState::Editing {
                    editor: *editor,
                    commit,
                };
            }
        }
    }

    fn on_keybinding_press(&mut self, button: ButtonCode) {
        // Work on a local copy to avoid borrowing conflicts with self calls.
        let mut kb = match self.state {
            MenuSubState::EditingKeyBinding {
                button: b,
                working_key_idx,
                original_key_idx,
            } => Some((b, working_key_idx, original_key_idx)),
            _ => None,
        };
        if let Some((bind_button, ref mut working_key_idx, original_key_idx)) = kb {
            match button {
                ButtonCode::CcLeft | ButtonCode::CcDown => {
                    let count = VALID_KEYS.len();
                    let new_idx = (*working_key_idx + count - 1) % count;
                    debug!("menu: key bind \u{2190} {}", VALID_KEYS[new_idx]);
                    *working_key_idx = new_idx;
                    self.state = MenuSubState::EditingKeyBinding {
                        button: bind_button,
                        working_key_idx: new_idx,
                        original_key_idx,
                    };
                }
                ButtonCode::CcRight | ButtonCode::CcUp => {
                    let count = VALID_KEYS.len();
                    let new_idx = (*working_key_idx + 1) % count;
                    debug!("menu: key bind \u{2192} {}", VALID_KEYS[new_idx]);
                    *working_key_idx = new_idx;
                    self.state = MenuSubState::EditingKeyBinding {
                        button: bind_button,
                        working_key_idx: new_idx,
                        original_key_idx,
                    };
                }
                ButtonCode::CcSelect => {
                    let key = VALID_KEYS[*working_key_idx];
                    debug!("menu: bind commit {}", key);
                    self.write_key_binding(bind_button, key);
                    self.state = MenuSubState::Browsing;
                }
                _ => {}
            }
        }
    }

    fn on_wiki_press(&mut self, w: &WikiEditState, button: ButtonCode) {
        if w.editing {
            let val = if w.selected == 0 {
                w.working_timeout
            } else {
                w.working_threshold
            };
            let key = if w.selected == 0 {
                SettingKey::EncoderMoveTimeout(w.encoder)
            } else {
                SettingKey::EncoderStepThreshold(w.encoder)
            };
            let meta = key.meta();
            match button {
                ButtonCode::CcLeft | ButtonCode::CcDown => {
                    let new_val = clamp_step(val, meta.step, meta.min, meta.max, false);
                    self.state = MenuSubState::WikiEdit(WikiEditState {
                        editing: true,
                        working_timeout: if w.selected == 0 {
                            new_val
                        } else {
                            w.working_timeout
                        },
                        working_threshold: if w.selected == 0 {
                            w.working_threshold
                        } else {
                            new_val
                        },
                        ..*w
                    });
                }
                ButtonCode::CcRight | ButtonCode::CcUp => {
                    let new_val = clamp_step(val, meta.step, meta.min, meta.max, true);
                    self.state = MenuSubState::WikiEdit(WikiEditState {
                        editing: true,
                        working_timeout: if w.selected == 0 {
                            new_val
                        } else {
                            w.working_timeout
                        },
                        working_threshold: if w.selected == 0 {
                            w.working_threshold
                        } else {
                            new_val
                        },
                        ..*w
                    });
                }
                ButtonCode::CcSelect => {
                    let sk = SettingKey::EncoderStepThreshold(w.encoder);
                    self.write_setting(sk, w.working_threshold);
                    let sk = SettingKey::EncoderMoveTimeout(w.encoder);
                    self.write_setting(sk, w.working_timeout);
                    debug!("menu: wiki commit");
                    self.state = MenuSubState::WikiEdit(WikiEditState {
                        editing: false,
                        ..*w
                    });
                }
                _ => {}
            }
        } else {
            match button {
                ButtonCode::CcUp | ButtonCode::CcDown => {
                    let new_sel = (w.selected + 1) % 2;
                    debug!(
                        "menu: wiki select {}",
                        if new_sel == 0 { "timeout" } else { "threshold" }
                    );
                    self.state = MenuSubState::WikiEdit(WikiEditState {
                        selected: new_sel,
                        ..*w
                    });
                }
                ButtonCode::CcSelect | ButtonCode::CcRight => {
                    debug!("menu: wiki edit start");
                    self.state = MenuSubState::WikiEdit(WikiEditState {
                        editing: true,
                        ..*w
                    });
                }
                ButtonCode::CcLeft => {
                    self.prompt_if_unsaved(|s| {
                        debug!("menu: wiki exit");
                        s.state = MenuSubState::Browsing;
                    });
                }
                _ => {}
            }
        }
    }

    // ── Long-press handlers ──

    fn on_idle_long_press(&mut self, button: ButtonCode) {
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

    fn on_debug_long_press(&mut self, button: ButtonCode) {
        if button == ButtonCode::CcSelect || button == ButtonCode::CcLeft {
            debug!("menu: exit debug screen (long-press)");
            self.state = MenuSubState::Browsing;
        }
    }

    fn on_pixel_long_press(&mut self, _button: ButtonCode) {
        debug!("menu: exit pixel test");
        self.state = MenuSubState::Browsing;
    }

    /// Common handling for LongPress and Repeat events: delegate direction
    /// buttons to `Press` where appropriate (always for Up/Down, only when
    /// in an editing substate for Left/Right).
    fn handle_repeat_as_press(&mut self, button: ButtonCode) {
        let is_editing = matches!(
            self.state,
            MenuSubState::Editing { .. }
                | MenuSubState::EditingKeyBinding { .. }
                | MenuSubState::WikiEdit(..)
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

    pub fn render_menu(
        &mut self,
        current_combined_button_state: u64,
        encoder_p1_count: i32,
        encoder_p2_count: i32,
    ) {
        match self.state {
            MenuSubState::Prompt { .. } => self.render_prompt(),
            MenuSubState::ShowCustom { cursor } => {
                self.render_show_custom(cursor, current_combined_button_state)
            }
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
            MenuSubState::Browsing => self.render_browsing_screen(),
            MenuSubState::Editing { .. } => self.render_editing_screen(),
            MenuSubState::EditingKeyBinding { .. } => self.render_keybinding_screen(),
            MenuSubState::WikiEdit(ref w) => self.render_wiki_edit(
                w.encoder,
                w.selected,
                w.editing,
                w.working_threshold,
                w.working_timeout,
            ),
        }
    }

    fn render_prompt(&mut self) {
        if let MenuSubState::Prompt {
            lines,
            choices,
            selection,
        } = self.state
        {
            self.display.clear(BinaryColor::Off).unwrap();
            for (i, line) in lines.iter().enumerate() {
                let s = line.as_str();
                if !s.is_empty() {
                    Text::with_baseline(
                        s,
                        Point::new(0, LINE_Y[i]),
                        self.text_style,
                        Baseline::Top,
                    )
                    .draw(self.display)
                    .unwrap();
                }
            }
            Text::with_baseline(
                choices[0].label.as_str(),
                Point::new(2, LINE_Y[3]),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
            let mut right_text = Text::with_alignment(
                choices[1].label.as_str(),
                Point::new(DISPLAY_R - 2, LINE_Y[3]),
                self.text_style,
                Alignment::Right,
            );
            right_text.text_style.baseline = Baseline::Top;
            right_text.draw(self.display).unwrap();
            let label = choices[match selection {
                PromptSide::First => 0,
                PromptSide::Second => 1,
            }]
            .label
            .as_str();
            let label_width = label.as_bytes().len() as i32 * CHAR_W;
            let (box_x, box_w) = match selection {
                PromptSide::First => (1, label_width + 2),
                PromptSide::Second => (DISPLAY_R - 2 - label_width, label_width + 2),
            };
            Rectangle::new(
                Point::new(box_x, LINE_Y[3] + 1),
                Size::new(box_w as u32, 15),
            )
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(self.display)
            .unwrap();
            self.display.flush().unwrap();
        }
    }

    fn render_browsing_screen(&mut self) {
        self.render_browsing();
        self.draw_menu_text(None);
    }

    fn render_editing_screen(&mut self) {
        if let MenuSubState::Editing { ref editor, .. } = self.state {
            write!(self.title_buf, "{}", self.current_level().title).unwrap();
            match editor {
                Editor::OptionSelect { current, .. } => {
                    self.render_options(Some(*current as u32));
                }
                Editor::IntRange { value, .. } => {
                    self.render_options(Some(*value));
                }
            }
            self.draw_menu_text(Some(2));
        }
    }

    fn render_keybinding_screen(&mut self) {
        if let MenuSubState::EditingKeyBinding {
            button,
            working_key_idx,
            ..
        } = self.state
        {
            self.render_editing_keybinding(button, working_key_idx);
            self.draw_menu_text(Some(2));
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
            "{}P{} Timeout",
            if selected == 0 { ">" } else { " " },
            encoder + 1
        )
        .unwrap();
        write!(
            self.debug_bufs[2],
            "{}P{} Threshold",
            if selected == 1 { ">" } else { " " },
            encoder + 1
        )
        .unwrap();

        // Values on value_bufs (right-aligned, even line indices)
        write!(self.value_bufs[0], "{} {}", m_val, m_meta.unit).unwrap();
        write!(self.value_bufs[1], "{} {}", t_val, t_meta.unit).unwrap();

        // Draw
        let color = embedded_graphics::pixelcolor::BinaryColor::Off;
        self.display.clear(color).unwrap();

        // Unsaved-changes indicator
        if self.settings_changed {
            Text::with_baseline(
                "*",
                Point::new(DISPLAY_R - CHAR_W, 0),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
        }

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
            MenuAction::EditValue(vk) => {
                let key: SettingKey = vk.into();
                let val = override_val.unwrap_or_else(|| self.read_setting(key));
                match vk {
                    ValueKey::AllBgMode | ValueKey::PlayerBgMode(_) => {
                        write!(buf, "{}", BG_MODE_NAMES[val as usize]).unwrap();
                    }
                    ValueKey::AllFgMode | ValueKey::PlayerFgMode(_) => {
                        write!(buf, "{}", FG_MODE_NAMES[val as usize]).unwrap();
                    }
                    ValueKey::AllTrigMode | ValueKey::PlayerTrigMode(_) => {
                        write!(buf, "{}", TRIG_MODE_NAMES[val as usize]).unwrap();
                    }
                    ValueKey::AllTrigDir | ValueKey::PlayerTrigDir(_) => {
                        write!(buf, "{}", DIR_NAMES[val as usize]).unwrap();
                    }
                    ValueKey::AllTrigOffset | ValueKey::PlayerTrigOffset(_) => {
                        write!(buf, "{}", OFFSET_NAMES[val as usize]).unwrap();
                    }
                    ValueKey::AllBgRainbow
                    | ValueKey::PlayerBgRainbow(_)
                    | ValueKey::AllFgRainbow
                    | ValueKey::PlayerFgRainbow(_)
                    | ValueKey::AllTrigRainbow
                    | ValueKey::PlayerTrigRainbow(_) => {
                        write!(buf, "{}", RAINBOW_NAMES[val as usize]).unwrap();
                    }
                    _ => {
                        let meta = key.meta();
                        let display_val = val / meta.divisor;
                        if meta.divisor > 1 && meta.step < meta.divisor {
                            let frac = val % meta.divisor;
                            write!(buf, "{}.{} {}", display_val, frac, meta.unit).unwrap();
                        } else if meta.unit.is_empty() {
                            write!(buf, "{}", display_val).unwrap();
                        } else {
                            write!(buf, "{} {}", display_val, meta.unit).unwrap();
                        }
                    }
                }
                true
            }
            MenuAction::EditKeyBinding(code) => {
                let key = override_val
                    .map(|v| v as u8)
                    .unwrap_or_else(|| self.read_key_binding(code));
                write!(buf, "{}", key_name(key)).unwrap();
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

        for &(buf_idx, opt_idx) in &option_line_indices(len, cursor) {
            let li = buf_idx - 1; // label/value index (0-based)
            if let Some(opt_idx) = opt_idx {
                let option = &level.options[opt_idx];

                // Write label with prefix
                let prefix = if buf_idx == 2 { ">" } else { " " };
                if matches!(option.action, MenuAction::GoBack) {
                    write!(self.label_bufs[li], "{}", prefix).unwrap();
                    self.back_arrow_y = Some(LINE_Y[buf_idx]);
                } else {
                    let max_label = VISIBLE_WIDTH - 1;
                    let label = if option.label.as_bytes().len() > max_label {
                        &option.label[..max_label]
                    } else {
                        option.label
                    };
                    write!(self.label_bufs[li], "{}{}", prefix, label).unwrap();

                    let mut temp = FmtBuf::new();
                    let vo = if buf_idx == 2 { override_val } else { None };
                    if self.write_option_value(option, &mut temp, vo) {
                        self.value_bufs[li].reset();
                        write!(self.value_bufs[li], "{}", temp.as_str()).unwrap();
                    }
                }
            } else {
                write!(self.label_bufs[li], "--").unwrap();
            }
        }
    }

    fn render_browsing(&mut self) {
        write!(self.title_buf, "{}", self.current_level().title).unwrap();
        self.render_options(None);
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

        // Unsaved-changes indicator
        if self.settings_changed {
            Text::with_baseline(
                "*",
                Point::new(DISPLAY_R - CHAR_W, 0),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
        }

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

        // Reset per-frame buffers for the next render pass.
        self.back_arrow_y = None;
        self.title_buf.reset();
        for line in &mut self.label_bufs {
            line.reset();
        }
        for line in &mut self.value_bufs {
            line.reset();
        }
    }

    // ── Navigation helpers ──

    fn execute_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::OpenSubmenu(level) => {
                debug!("menu: enter {}", level.title);
                self.push_level(level);
            }
            MenuAction::GoBack => {
                if self.stack_depth > 1 {
                    self.prompt_if_unsaved(|s| {
                        debug!("menu: back");
                        s.pop_level();
                    });
                } else {
                    debug!("menu: back");
                    self.pop_level();
                }
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
            MenuAction::EditValue(vk) => {
                let (editor, commit) = build_editor(&self.settings, vk);
                debug!("menu: edit setting");
                self.state = MenuSubState::Editing { editor, commit };
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
                self.state = MenuSubState::WikiEdit(WikiEditState {
                    encoder,
                    selected: 0,
                    editing: false,
                    working_threshold: t_val,
                    working_timeout: m_val,
                    original_threshold: t_val,
                    original_timeout: m_val,
                });
            }
            MenuAction::SaveAndReboot => {
                use crate::flash_storage::*;
                self.show_rebooting_screen();
                // Signal both cores to enter a safe RAM spin-loop.
                FLASH_PREPARE_FLAG.store(true, Ordering::SeqCst);
                // Wait for core0 to acknowledge it's spinning.
                while !FLASH_CORE0_READY.load(Ordering::SeqCst) {
                    core::hint::spin_loop();
                }
                // Safe to write flash — core0 isn't fetching from XIP.
                unsafe {
                    write_storage(&self.settings);
                }
                // Release core0 and signal it to reboot.
                FLASH_PREPARE_FLAG.store(false, Ordering::SeqCst);
                FLASH_PENDING_REBOOT.store(true, Ordering::SeqCst);
                // Core0 handles the `sys_reset()` — wait here.
                loop {
                    core::hint::spin_loop();
                }
            }
            MenuAction::Discard => {
                self.pop_level();
                self.state = MenuSubState::Browsing;
            }
            MenuAction::ResetDefaults => {
                self.state = Self::build_prompt(
                    ["This restores", "ALL changes", "back to base!"],
                    MenuAction::PerformReset,
                    MenuAction::Discard,
                );
            }
            MenuAction::PerformReset => {
                use crate::flash_storage::*;
                self.show_rebooting_screen();
                FLASH_PREPARE_FLAG.store(true, Ordering::SeqCst);
                while !FLASH_CORE0_READY.load(Ordering::SeqCst) {
                    core::hint::spin_loop();
                }
                unsafe {
                    clear_storage(&self.settings);
                }
                FLASH_PREPARE_FLAG.store(false, Ordering::SeqCst);
                FLASH_PENDING_REBOOT.store(true, Ordering::SeqCst);
                loop {
                    core::hint::spin_loop();
                }
            }
            MenuAction::Reboot => {
                use crate::flash_storage::*;
                self.show_rebooting_screen();
                FLASH_PENDING_REBOOT.store(true, Ordering::SeqCst);
                loop {
                    core::hint::spin_loop();
                }
            }
            MenuAction::ShowCustom => {
                self.state = MenuSubState::ShowCustom { cursor: 0 };
            }
            MenuAction::ResetField(idx) => {
                self.reset_field_to_default(idx);
                self.state = MenuSubState::ShowCustom { cursor: 0 };
            }
            MenuAction::ReturnToCustom => {
                self.state = MenuSubState::ShowCustom { cursor: 0 };
            }
        }
    }

    /// Count how many settings differ from factory defaults.
    fn count_changes(&self) -> usize {
        let defaults = crate::flash_storage::FlashStoragePersistentMemory::default();
        for_each_changed_field(&self.settings, &defaults, |_, _, _| true)
    }

    /// Reset the `target_idx`th changed field back to its factory-default value.
    fn reset_field_to_default(&mut self, target_idx: usize) {
        let defaults = crate::flash_storage::FlashStoragePersistentMemory::default();
        let mut idx = 0_usize;

        for b in 0..NUM_BUTTONS {
            if self.settings.buttons[b].debounce_ticks != defaults.buttons[b].debounce_ticks {
                if idx == target_idx {
                    let key = SettingKey::ButtonDebounce(ButtonCode::from_repr(b).unwrap());
                    self.write_setting(key, defaults.buttons[b].debounce_ticks as u32);
                    return;
                }
                idx += 1;
            }
        }
        for b in 0..NUM_BUTTONS {
            if self.settings.buttons[b].key != defaults.buttons[b].key {
                if idx == target_idx {
                    let code = ButtonCode::from_repr(b).unwrap();
                    self.write_key_binding(code, defaults.buttons[b].key);
                    return;
                }
                idx += 1;
            }
        }
        for e in 0..NUM_ENCODERS {
            if self.settings.encoders[e].key_up != defaults.encoders[e].key_up {
                if idx == target_idx {
                    let code = if e == 0 {
                        ButtonCode::P1Positive
                    } else {
                        ButtonCode::P2Positive
                    };
                    self.write_key_binding(code, defaults.encoders[e].key_up);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e].key_down != defaults.encoders[e].key_down {
                if idx == target_idx {
                    let code = if e == 0 {
                        ButtonCode::P1Negative
                    } else {
                        ButtonCode::P2Negative
                    };
                    self.write_key_binding(code, defaults.encoders[e].key_down);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e].debounce_ticks != defaults.encoders[e].debounce_ticks {
                if idx == target_idx {
                    let key = SettingKey::EncoderDebounce(e);
                    self.write_setting(key, defaults.encoders[e].debounce_ticks as u32);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e].step_threshold != defaults.encoders[e].step_threshold {
                if idx == target_idx {
                    let key = SettingKey::EncoderStepThreshold(e);
                    self.write_setting(key, defaults.encoders[e].step_threshold as u32);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e].move_timeout_ticks
                != defaults.encoders[e].move_timeout_ticks
            {
                if idx == target_idx {
                    let key = SettingKey::EncoderMoveTimeout(e);
                    self.write_setting(key, defaults.encoders[e].move_timeout_ticks as u32);
                    return;
                }
                idx += 1;
            }
        }
        // Lighting fields
        for p in 0..2 {
            lighting_reset_checks!(
                self,
                p,
                idx,
                defaults,
                target_idx,
                [
                    [PlayerBgMode, bg_mode],
                    [PlayerBgRainbow, bg_rainbow],
                    [PlayerBgSpd, bg_speed_ds],
                    [PlayerBgSubdiv, bg_subdivisions],
                    [PlayerFgMode, fg_mode],
                    [PlayerFgRainbow, fg_rainbow],
                    [PlayerFgSpd, fg_speed_ds],
                    [PlayerFgSubdiv, fg_subdivisions],
                    [PlayerFgStep, fg_step_ds],
                    [PlayerFgSize, fg_px_per_group],
                    [PlayerTrigMode, trig_mode],
                    [PlayerTrigRainbow, trig_rainbow],
                    [PlayerTrigFdIn, trig_fade_in_ms],
                    [PlayerTrigFdOut, trig_fade_out_ms],
                    [PlayerTrigSize, trig_width],
                    [PlayerTrigDir, trig_dir],
                    [PlayerTrigOffset, trig_offset],
                    [PlayerTrigDur, trig_dur_s],
                ]
            );
        }
        // Global brightness
        if self.settings.lighting.brightness != defaults.lighting.brightness {
            if idx == target_idx {
                self.write_setting(
                    SettingKey::GlobalBrightness,
                    defaults.lighting.brightness as u32,
                );
                return;
            }
            idx += 1;
        }
        let _ = idx;
    }

    /// Render the show-custom screen: three lines of changed settings.
    fn render_show_custom(&mut self, cursor: usize, _state: u64) {
        let defaults = crate::flash_storage::FlashStoragePersistentMemory::default();
        // ── Count total changes ──
        let total = self.count_changes();

        self.display.clear(BinaryColor::Off).unwrap();

        if total == 0 {
            Text::with_baseline(
                "No custom",
                Point::new(0, LINE_Y[1]),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
            Text::with_baseline(
                "settings",
                Point::new(0, LINE_Y[2]),
                self.text_style,
                Baseline::Top,
            )
            .draw(self.display)
            .unwrap();
            self.display.flush().unwrap();
            return;
        }

        // ── Determine section title from cursor position ──
        let mut section = "?";
        let idx = core::cell::Cell::new(0_usize);
        for_each_changed_field(&self.settings, &defaults, |field, _, _| {
            if idx.get() == cursor {
                section = field.section_title();
                return false;
            }
            idx.set(idx.get() + 1);
            true
        });
        self.title_buf.reset();
        core::write!(self.title_buf, "{}", section).ok();

        // ── Draw title and current item ──
        // Use label_bufs[0..2] for the three visible lines.
        for buf in &mut self.label_bufs {
            buf.reset();
        }

        // Fill the three visible lines using the shared helper.
        for &(buf_idx, opt_idx) in &option_line_indices(total, cursor) {
            let mut buf = self.label_bufs[buf_idx - 1];
            if let Some(idx) = opt_idx {
                self.format_change_item(&defaults, idx, &mut buf);
            } else {
                core::write!(buf, "--").ok();
            }
            self.label_bufs[buf_idx - 1] = buf;
        }

        // Title
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

        // Three option lines at y=16,32,48
        for i in 0..3_usize {
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
        }

        // Highlight box on the middle line (cursor is the second of three shown)
        let hl = self.label_bufs[1].as_str();
        let hl_len = hl.as_bytes().len() as i32 * CHAR_W;
        Rectangle::new(
            Point::new(0, LINE_Y[2] + 1),
            Size::new(hl_len as u32 + 2, 15),
        )
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(self.display)
        .unwrap();

        self.display.flush().unwrap();
    }

    /// Write a single line of the show-custom list into `buf` for the `target_idx`th change.
    fn format_change_item(
        &self,
        defaults: &crate::flash_storage::FlashStoragePersistentMemory,
        target_idx: usize,
        buf: &mut FmtBuf,
    ) {
        let mut idx = 0_usize;
        for_each_changed_field(&self.settings, defaults, |field, cur, _def| {
            if idx != target_idx {
                idx += 1;
                return true;
            }
            match field {
                FieldDescriptor::ButtonDebounce(b) => {
                    let code = ButtonCode::from_repr(b).expect("button index out of range");
                    write!(buf, "{} db: {} ms", code.short_label(), cur / 1_000).ok();
                }
                FieldDescriptor::ButtonKey(b) => {
                    let code = ButtonCode::from_repr(b).expect("button index out of range");
                    write!(buf, "{}: {}", code.short_label(), key_name(cur as u8)).ok();
                }
                FieldDescriptor::EncoderKeyUp(e) => {
                    let name = if e == 0 { "P1Up" } else { "P2Up" };
                    write!(buf, "{}: {}", name, key_name(cur as u8)).ok();
                }
                FieldDescriptor::EncoderKeyDown(e) => {
                    let name = if e == 0 { "P1Dn" } else { "P2Dn" };
                    write!(buf, "{}: {}", name, key_name(cur as u8)).ok();
                }
                FieldDescriptor::EncoderDebounce(e) => {
                    let name = if e == 0 { "P1Edb" } else { "P2Edb" };
                    write!(buf, "{}: {} ms", name, cur / 1_000).ok();
                }
                FieldDescriptor::EncoderStepThreshold(e) => {
                    let name = if e == 0 { "P1Eth" } else { "P2Eth" };
                    write!(buf, "{}: {} Steps", name, cur).ok();
                }
                FieldDescriptor::EncoderMoveTimeout(e) => {
                    let name = if e == 0 { "P1Etm" } else { "P2Etm" };
                    write!(buf, "{}: {} ms", name, cur / 1_000).ok();
                }
                FieldDescriptor::PlayerBgMode(p) => {
                    let name = if p == 0 { "P1BgM" } else { "P2BgM" };
                    write!(buf, "{}: {}", name, BG_MODE_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerBgRainbow(p) => {
                    let name = if p == 0 { "P1BgR" } else { "P2BgR" };
                    write!(buf, "{}: {}", name, RAINBOW_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerBgSpd(p) => {
                    let name = if p == 0 { "P1BgS" } else { "P2BgS" };
                    write!(buf, "{}: {} s", name, cur / 10).ok();
                }
                FieldDescriptor::PlayerBgSubdiv(p) => {
                    let name = if p == 0 { "P1BgSd" } else { "P2BgSd" };
                    write!(buf, "{}: {}", name, cur).ok();
                }
                FieldDescriptor::PlayerFgMode(p) => {
                    let name = if p == 0 { "P1FgM" } else { "P2FgM" };
                    write!(buf, "{}: {}", name, FG_MODE_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerFgRainbow(p) => {
                    let name = if p == 0 { "P1FgR" } else { "P2FgR" };
                    write!(buf, "{}: {}", name, RAINBOW_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerFgSpd(p) => {
                    let name = if p == 0 { "P1FgS" } else { "P2FgS" };
                    write!(buf, "{}: {} s", name, cur / 10).ok();
                }
                FieldDescriptor::PlayerFgSubdiv(p) => {
                    let name = if p == 0 { "P1FgSd" } else { "P2FgSd" };
                    write!(buf, "{}: {}", name, cur).ok();
                }
                FieldDescriptor::PlayerFgStep(p) => {
                    let name = if p == 0 { "P1FgSt" } else { "P2FgSt" };
                    write!(buf, "{}: {} s", name, cur / 10).ok();
                }
                FieldDescriptor::PlayerFgSize(p) => {
                    let name = if p == 0 { "P1FgSz" } else { "P2FgSz" };
                    write!(buf, "{}: {} px", name, cur).ok();
                }
                FieldDescriptor::PlayerTrigMode(p) => {
                    let name = if p == 0 { "P1TrM" } else { "P2TrM" };
                    write!(buf, "{}: {}", name, TRIG_MODE_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerTrigRainbow(p) => {
                    let name = if p == 0 { "P1TrR" } else { "P2TrR" };
                    write!(buf, "{}: {}", name, RAINBOW_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerTrigFdIn(p) => {
                    let name = if p == 0 { "P1FdI" } else { "P2FdI" };
                    write!(buf, "{}:{}ms", name, cur).ok();
                }
                FieldDescriptor::PlayerTrigFdOut(p) => {
                    let name = if p == 0 { "P1FdO" } else { "P2FdO" };
                    write!(buf, "{}:{}ms", name, cur).ok();
                }
                FieldDescriptor::PlayerTrigSize(p) => {
                    let name = if p == 0 { "P1TrSz" } else { "P2TrSz" };
                    write!(buf, "{}: {} px", name, cur).ok();
                }
                FieldDescriptor::PlayerTrigDir(p) => {
                    let name = if p == 0 { "P1TrDr" } else { "P2TrDr" };
                    write!(buf, "{}: {}", name, DIR_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerTrigOffset(p) => {
                    let name = if p == 0 { "P1TrOf" } else { "P2TrOf" };
                    write!(buf, "{}:{}", name, OFFSET_NAMES[cur as usize]).ok();
                }
                FieldDescriptor::PlayerTrigDur(p) => {
                    let name = if p == 0 { "P1TrDu" } else { "P2TrDu" };
                    write!(buf, "{}: {} s", name, cur).ok();
                }
                FieldDescriptor::PlayerBrightness(p) => {
                    let name = if p == 0 { "P1Brt" } else { "P2Brt" };
                    write!(buf, "{}: {}", name, cur).ok();
                }
            }
            false
        });
    }

    /// Display \"Rebooting...\" on the OLED before a system reset.
    fn show_rebooting_screen(&mut self) {
        self.display.clear(BinaryColor::Off).unwrap();
        Text::with_baseline(
            "Rebooting...",
            Point::new(18, LINE_Y[2]),
            self.text_style,
            Baseline::Top,
        )
        .draw(self.display)
        .unwrap();
        self.display.flush().unwrap();
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
        lighting_read_block!(
            self,
            key,
            [
                [AllBgMode, PlayerBgMode, bg_mode],
                [AllBgRainbow, PlayerBgRainbow, bg_rainbow],
                [AllBgSpd, PlayerBgSpd, bg_speed_ds],
                [AllBgSubdiv, PlayerBgSubdiv, bg_subdivisions],
                [AllFgMode, PlayerFgMode, fg_mode],
                [AllFgRainbow, PlayerFgRainbow, fg_rainbow],
                [AllFgSpd, PlayerFgSpd, fg_speed_ds],
                [AllFgSubdiv, PlayerFgSubdiv, fg_subdivisions],
                [AllFgStep, PlayerFgStep, fg_step_ds],
                [AllFgSize, PlayerFgSize, fg_px_per_group],
                [AllTrigMode, PlayerTrigMode, trig_mode],
                [AllTrigRainbow, PlayerTrigRainbow, trig_rainbow],
                [AllTrigFdIn, PlayerTrigFdIn, trig_fade_in_ms],
                [AllTrigFdOut, PlayerTrigFdOut, trig_fade_out_ms],
                [AllTrigSize, PlayerTrigSize, trig_width],
                [AllTrigDir, PlayerTrigDir, trig_dir],
                [AllTrigOffset, PlayerTrigOffset, trig_offset],
                [AllTrigDur, PlayerTrigDur, trig_dur_s],
            ]
        );
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
            SettingKey::GlobalBrightness => self.settings.lighting.brightness as u32,
            _ => {
                debug!("read_setting: unexpected key");
                0
            }
        }
    }

    fn write_setting(&mut self, key: SettingKey, value: u32) {
        lighting_write_block!(
            self,
            key,
            value,
            [
                [AllBgMode, PlayerBgMode, bg_mode, u8],
                [AllBgRainbow, PlayerBgRainbow, bg_rainbow, u8],
                [AllBgSpd, PlayerBgSpd, bg_speed_ds, u16],
                [AllBgSubdiv, PlayerBgSubdiv, bg_subdivisions, u8],
                [AllFgMode, PlayerFgMode, fg_mode, u8],
                [AllFgRainbow, PlayerFgRainbow, fg_rainbow, u8],
                [AllFgSpd, PlayerFgSpd, fg_speed_ds, u16],
                [AllFgSubdiv, PlayerFgSubdiv, fg_subdivisions, u8],
                [AllFgStep, PlayerFgStep, fg_step_ds, u16],
                [AllFgSize, PlayerFgSize, fg_px_per_group, u8],
                [AllTrigMode, PlayerTrigMode, trig_mode, u8],
                [AllTrigRainbow, PlayerTrigRainbow, trig_rainbow, u8],
                [AllTrigFdIn, PlayerTrigFdIn, trig_fade_in_ms, u16],
                [AllTrigFdOut, PlayerTrigFdOut, trig_fade_out_ms, u16],
                [AllTrigSize, PlayerTrigSize, trig_width, u8],
                [AllTrigDir, PlayerTrigDir, trig_dir, u8],
                [AllTrigOffset, PlayerTrigOffset, trig_offset, u8],
                [AllTrigDur, PlayerTrigDur, trig_dur_s, u8],
            ]
        );
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
            SettingKey::GlobalBrightness => self.settings.lighting.brightness = value as u8,
            _ => unreachable!(),
        }
        self.settings_changed = true;
        self.prompt_answered_since_change = false;
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
        self.settings_changed = true;
        self.prompt_answered_since_change = false;
    }
}

/// Convert a USB HID key code to a short human-readable name (OLED-safe length).
fn key_name(key: u8) -> &'static str {
    match key {
        0 => "No Key",
        1 => "ErrRO",
        2 => "POST",
        3 => "Err",
        4 => "A",
        5 => "B",
        6 => "C",
        7 => "D",
        8 => "E",
        9 => "F",
        10 => "G",
        11 => "H",
        12 => "I",
        13 => "J",
        14 => "K",
        15 => "L",
        16 => "M",
        17 => "N",
        18 => "O",
        19 => "P",
        20 => "Q",
        21 => "R",
        22 => "S",
        23 => "T",
        24 => "U",
        25 => "V",
        26 => "W",
        27 => "X",
        28 => "Y",
        29 => "Z",
        30 => "K1",
        31 => "K2",
        32 => "K3",
        33 => "K4",
        34 => "K5",
        35 => "K6",
        36 => "K7",
        37 => "K8",
        38 => "K9",
        39 => "K0",
        40 => "Enter",
        41 => "Esc",
        42 => "BkSpc",
        43 => "Tab",
        44 => "Spc",
        45 => "-_",
        46 => "=+",
        47 => "{",
        48 => "}",
        49 => "\\",
        50 => "#",
        51 => ";",
        52 => "'",
        53 => "Grave",
        54 => ",<",
        55 => ".>",
        56 => "/",
        57 => "Caps",
        58 => "F1",
        59 => "F2",
        60 => "F3",
        61 => "F4",
        62 => "F5",
        63 => "F6",
        64 => "F7",
        65 => "F8",
        66 => "F9",
        67 => "F10",
        68 => "F11",
        69 => "F12",
        70 => "PrtSc",
        71 => "ScrLk",
        72 => "Paus",
        73 => "Ins",
        74 => "Hom",
        75 => "PgUp",
        76 => "Del",
        77 => "End",
        78 => "PgDn",
        79 => "RArr",
        80 => "LArr",
        81 => "DArr",
        82 => "UArr",
        83 => "NLck",
        84 => "Kp/",
        85 => "Kp*",
        86 => "Kp-",
        87 => "Kp+",
        88 => "KEnt",
        89 => "Kp1",
        90 => "Kp2",
        91 => "Kp3",
        92 => "Kp4",
        93 => "Kp5",
        94 => "Kp6",
        95 => "Kp7",
        96 => "Kp8",
        97 => "Kp9",
        98 => "Kp0",
        99 => "Kp.",
        100 => "NUS\\",
        101 => "App",
        102 => "Pow",
        103 => "KpEq",
        104 => "F13",
        105 => "F14",
        106 => "F15",
        107 => "F16",
        108 => "F17",
        109 => "F18",
        110 => "F19",
        111 => "F20",
        112 => "F21",
        113 => "F22",
        114 => "F23",
        115 => "F24",
        116 => "Exe",
        117 => "Help",
        118 => "Men",
        119 => "Sel",
        120 => "Stp",
        121 => "Agn",
        122 => "Und",
        123 => "Cut",
        124 => "Cop",
        125 => "Pst",
        126 => "Fin",
        127 => "Mute",
        128 => "VolUp",
        129 => "VolDn",
        130 => "LCaps",
        131 => "LNumL",
        132 => "LScrL",
        133 => "KpCom",
        134 => "KpEqS",
        153 => "AltEr",
        154 => "SysRq",
        159 => "Sep",
        162 => "ClrAg",
        163 => "Props",
        224 => "LCTRL",
        225 => "LSHFT",
        226 => "LALT",
        227 => "LGUI",
        228 => "RCTRL",
        229 => "RSHFT",
        230 => "RALT",
        231 => "RGUI",
        _ => "?",
    }
}
