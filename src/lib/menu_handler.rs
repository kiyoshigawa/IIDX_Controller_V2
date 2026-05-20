//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

use crate::{BUF_SIZE, ButtonCode, NUM_BUTTONS, OledDisplay};
use core::fmt::Write;
use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_9X18_BOLD},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text},
};

/// Y-offset at which the button layout graphic is drawn on the 64-px screen.
/// (63 − 26 − 2 + 1 = 36)
pub(crate) const BUTTON_GRAPHIC_ROW_HEIGHT: u8 = 36;

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
    Idle,
    TextMenu,
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

/// Drives the SSD1306 OLED display and menu logic for the IIDX deck.
pub struct MenuHandler<'a, D> {
    pub display: &'a mut OledDisplay<D>,
    line_bufs: [FmtBuf; 4],
    text_style: MonoTextStyle<'static, BinaryColor>,
    pub frames_rendered: u64,
    menu_mode: MenuMode,
}

impl<'a, D: WriteOnlyDataCommand> MenuHandler<'a, D> {
    pub fn new(display: &'a mut OledDisplay<D>) -> Self {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_9X18_BOLD)
            .text_color(BinaryColor::On)
            .build();
        Self {
            display,
            line_bufs: [FmtBuf::new(), FmtBuf::new(), FmtBuf::new(), FmtBuf::new()],
            text_style,
            frames_rendered: 0,
            menu_mode: MenuMode::Debug,
        }
    }

    pub fn process_event(&mut self, event: MenuEvents) {
        match event {
            MenuEvents::Press(button) => {
                debug!("MENU PRESS! {}", button as usize);
                match button {
                    ButtonCode::CcUp => self.menu_mode = MenuMode::PixelTest,
                    ButtonCode::CcSelect => self.menu_mode = MenuMode::Debug,
                    ButtonCode::CcDown => self.menu_mode = MenuMode::Debug,
                    ButtonCode::CcLeft => self.menu_mode = MenuMode::Debug,
                    ButtonCode::CcRight => self.menu_mode = MenuMode::Debug,
                    _ => {}
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
        match self.menu_mode {
            MenuMode::Debug => self.print_debug_display(
                current_combined_button_state,
                encoder_p1_count,
                encoder_p2_count,
            ),
            MenuMode::PixelTest => self.print_pixel_test(),
            MenuMode::Idle => {}
            MenuMode::TextMenu => {}
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
}
