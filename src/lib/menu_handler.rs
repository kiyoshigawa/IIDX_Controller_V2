//! Display/menu-handling module.
//!
//! Provides [`MenuHandler`] — the OLED display driver and menu-state machine
//! that renders all OLED display output and controls the persistent settings
//! for the controller.

use crate::{
    BUF_SIZE, BUTTON_DEBUG_RECTANGLES, BUTTON_GRAPHIC, BUTTON_GRAPHIC_ROW_HEIGHT, ButtonCode,
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
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text},
};

// ──────────────────────────────────────────────────────────────────────────────
// MenuMode
// ──────────────────────────────────────────────────────────────────────────────

/// Determines which rendering mode the OLED display is in.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuMode {
    Idle,
    TextMenu,
    Debug,
    PixelTest,
}

// ──────────────────────────────────────────────────────────────────────────────
// MenuEvents
// ──────────────────────────────────────────────────────────────────────────────

/// Menu-related events emitted when a control-center button is pressed.
pub enum MenuEvents {
    Press(ButtonCode),
}

// ──────────────────────────────────────────────────────────────────────────────
// FmtBuf
// ──────────────────────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────────────────────
// MenuHandler
// ──────────────────────────────────────────────────────────────────────────────

/// Drives the SSD1306 OLED display for the IIDX deck.
///
/// Manages a set of four [`FmtBuf`] lines, a text style, and a current
/// [`MenuMode`].  Call [`render_menu`](MenuHandler::render_menu) from the
/// main loop to redraw the screen.
pub struct MenuHandler<'a, D> {
    pub display: &'a mut OledDisplay<D>,
    line_bufs: [FmtBuf; 4],
    text_style: MonoTextStyle<'static, BinaryColor>,
    pub frames_rendered: u64,
    menu_mode: MenuMode,
}

impl<'a, D: WriteOnlyDataCommand> MenuHandler<'a, D> {
    /// Constructs a new `MenuHandler` with the given display.
    ///
    /// Owns its own text buffers and style internally; starts in
    /// [`MenuMode::Debug`] by default.
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

    /// Handles a [`MenuEvents`] by switching the menu mode.
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

    /// Entry point for rendering the display.
    ///
    /// Dispatches to the appropriate render function based on the current
    /// [`MenuMode`].
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

    // ── Private render helpers ────────────────────────────────────────────

    /// Prints the full debug overlay: frame counter, encoder counts, the
    /// button-layout graphic, and pressed-button indicator dots.
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
            Point::new(0, 0),
            self.text_style,
            Baseline::Top,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 1 (left-middle)
        Text::with_alignment(
            self.line_bufs[1].as_str(),
            Point::new(0, 32),
            self.text_style,
            Alignment::Left,
        )
        .draw(self.display)
        .unwrap();

        // Encoder 2 (right-middle)
        Text::with_alignment(
            self.line_bufs[2].as_str(),
            Point::new(127, 32),
            self.text_style,
            Alignment::Right,
        )
        .draw(self.display)
        .unwrap();

        // Button layout background
        self.draw_empty_button_graphic();

        // Pressed-button indicator dots (only physical buttons, lower 32 bits)
        self.draw_pressed_buttons(current_combined_button_state as u32);

        self.display.flush().unwrap();
    }

    /// Draws the base IIDX button-layout image from [`BUTTON_GRAPHIC`].
    fn draw_empty_button_graphic(&mut self) {
        let raw = ImageRaw::<BinaryColor>::new(&BUTTON_GRAPHIC, 128);
        let image = Image::new(&raw, Point::new(0, BUTTON_GRAPHIC_ROW_HEIGHT as i32));
        image.draw(self.display).unwrap();
    }

    /// Draws filled rectangles for each pressed button based on the 27-bit
    /// encoded button state. Bits 0–26 correspond to the 27 rectangles in
    /// [`BUTTON_DEBUG_RECTANGLES`].
    fn draw_pressed_buttons(&mut self, state: u32) {
        for (i, rect) in BUTTON_DEBUG_RECTANGLES.iter().enumerate() {
            if (state >> i) & 1 == 1 {
                rect.into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(self.display)
                    .unwrap();
            }
        }
    }

    /// Fills the entire display white (all pixels on) as a quick pixel test.
    fn print_pixel_test(&mut self) {
        Rectangle::new(Point::new(0, 0), Size::new(127, 63))
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
            .draw(self.display)
            .unwrap();

        self.display.flush().unwrap();
    }
}
