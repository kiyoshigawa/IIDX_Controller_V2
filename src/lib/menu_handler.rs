//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

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

use crate::menu_layout::MenuOption;
use crate::menu_layout::{
    Commit, Editor, Flip, FmtBuf, MAX_MENU_DEPTH, MenuAction, MenuEvents, MenuLevel, MenuMode,
    PromptChoice, PromptSide, ROOT_MENU, StackItem, VALID_KEYS, clamp_step, flip_for, key_index,
    option_line_indices,
};
use crate::menu_settings::WikiEditState;
use crate::menu_settings::{SettingKey, ValueKey, build_editor, for_each_changed_field, key_name};
use crate::{
    BG_MODE_NAMES, ButtonCode, DEFAULT_BUTTON_DEBOUNCE_TICKS, DIR_NAMES, FG_MODE_NAMES,
    FlashStoragePersistentMemory, NUM_BUTTONS, OFFSET_NAMES, OledDisplay, RAINBOW_NAMES,
    TRIG_MODE_NAMES,
};

/// Y-offset at which the button layout graphic is drawn on the 64-px screen.
/// (63 − 26 − 2 + 1 = 36)
pub(crate) const BUTTON_GRAPHIC_ROW_HEIGHT: u8 = 36;

/// Width of a single character in the FONT_9X18_BOLD font (pixels).
const CHAR_W: i32 = 9;

/// OLED display width in pixels.
const DISPLAY_W: u32 = 128;

/// Rightmost pixel column on the display.
const DISPLAY_R: i32 = (DISPLAY_W - 1) as i32;

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
    pub(crate) all_debounce_value: u32,
    /// True when any setting has been written to the RAM copy since boot.
    pub settings_changed: bool,
    /// Set to true when the user confirms "Save & Reboot".
    pub pending_reboot: bool,
    /// True once the save prompt has been shown and dismissed since the last change.
    /// Prevents the prompt from re-appearing on every back-navigation within a change batch.
    pub(crate) prompt_answered_since_change: bool,
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
}
