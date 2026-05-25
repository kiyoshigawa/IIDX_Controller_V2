//! Lighting-handling module.
//!
//! Provides [`LightingHandler`] — the core1-level manager for the LED strip
//! colour buffer, gamma correction, and brightness scaling.
//!
//! The actual [`lighting_controller::LightingController`] and its
//! [`Animation`]s live in `main.rs` (owned as local variables in the core1
//! closure).  This module owns the colour buffer and the gamma/brightness
//! pipeline, and will grow to own the animations and controller in
//! Phase 2 (per-player split) and handle [`LightingEvent`]s in Phase 3+.

use crate::led_strip::NUM_LEDS;
use rgb::RGB8;
use smart_leds::colors::BLACK;

// ──────────────────────────────────────────────────────────────────────────────
// Gamma correction
// ──────────────────────────────────────────────────────────────────────────────

/// Gamma-correction lookup table (standard 2.2 gamma, stored in flash).
///
/// Maps an 8-bit linear channel value to its gamma-corrected 8-bit output.
/// This is the same table used in the original Arduino/Teensy controller code.
static GAMMA_TABLE: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

/// Default LED brightness level (0–255).  Start moderately — can be adjusted
/// via menu in Phase 5.
pub const DEFAULT_BRIGHTNESS: u8 = 255;

// ──────────────────────────────────────────────────────────────────────────────
// LightingHandler
// ──────────────────────────────────────────────────────────────────────────────

/// Owns the LED strip colour buffer and handles gamma/brightness correction.
///
/// **Phase 1:** Just the buffer and pipeline — the animation and
/// [`lighting_controller::LightingController`] live in `main.rs`.
///
/// **Phase 2+:** Will own the per-player [`Animation`]s and the
/// [`LightingController`], handle [`LightingEvent`]s, and expose settings
/// for the menu system.
pub struct LightingHandler {
    /// The frame buffer written to the WS2812 strip each cycle.
    pub color_buffer: [RGB8; NUM_LEDS],
    /// Current brightness level (0–255).
    brightness: u8,
}

impl LightingHandler {
    /// Create a new `LightingHandler` with all LEDs off at default brightness.
    pub fn new() -> Self {
        Self {
            color_buffer: [BLACK; NUM_LEDS],
            brightness: DEFAULT_BRIGHTNESS,
        }
    }

    /// Create a new `LightingHandler` with a custom brightness level.
    pub fn with_brightness(brightness: u8) -> Self {
        Self {
            color_buffer: [BLACK; NUM_LEDS],
            brightness,
        }
    }

    /// Apply gamma correction and brightness scaling **in-place** on the
    /// colour buffer.
    ///
    /// Must be called before [`write_frame`] because the DMA driver accepts
    /// a `&[RGB8]` slice, not an iterator — we cannot chain gamma/brightness
    /// iterator adaptors at the write site.
    pub fn apply_gamma_brightness(&mut self) {
        let scale = self.brightness as u16;

        for led in self.color_buffer.iter_mut() {
            // Gamma correction via lookup table
            led.r = GAMMA_TABLE[led.r as usize];
            led.g = GAMMA_TABLE[led.g as usize];
            led.b = GAMMA_TABLE[led.b as usize];

            // Brightness scaling with rounding
            led.r = ((led.r as u16 * scale + 127) / 255) as u8;
            led.g = ((led.g as u16 * scale + 127) / 255) as u8;
            led.b = ((led.b as u16 * scale + 127) / 255) as u8;
        }
    }

    /// Convenience: gamma-correct, then immediately write to the LED strip.
    ///
    /// Equivalent to calling [`apply_gamma_brightness`] then
    /// [`DmaLedStrip::write_frame`].
    pub fn write_gamma_corrected<CH, SM>(
        &mut self,
        strip: &mut crate::led_strip::DmaLedStrip<CH, SM>,
        now: u64,
    ) where
        CH: rp235x_hal::dma::SingleChannel,
        SM: rp235x_hal::pio::ValidStateMachine,
    {
        self.apply_gamma_brightness();
        strip.write_frame(&self.color_buffer, now);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// LightingEvent — placeholder for Phases 3+
// ──────────────────────────────────────────────────────────────────────────────

/// Events emitted by [`InputHandler`](crate::input_handler::InputHandler) for
/// the lighting system to react to.
///
/// Currently unused (Phase 1 just runs a background animation).  Phases 3 and 4
/// will add `EncoderMoved` and `ButtonPressed` variants respectively.
#[allow(dead_code)]
pub enum LightingEvent {
    /// Placeholder — will grow in later phases.
    _Placeholder,
}
