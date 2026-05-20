//! Input-handling module.
//!
//! Provides [`InputHandler`] — the core1-level orchestrator that receives
//! button/encoder state from core0 via SIO FIFO, detects changes, and
//! forwards events to [`MenuHandler`](crate::menu_handler::MenuHandler).

use crate::menu_handler::{MenuEvents, MenuHandler};
use crate::{ButtonCode, EncoderDirection};
use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use strum::IntoEnumIterator;

/// Events generated when a button's debounced state transitions.
pub(crate) enum ButtonEvents {
    Press(ButtonCode),
    Release(ButtonCode),
}

/// This will take data from the physical inputs that was passed through the SIO FIFO buffers
/// from core0 to core1, then it will process that data to decide on which input states have 
/// changed, and depending on what buttons changed and how, it will fire off event calls to
/// other state machines like the LED Strip or MenuHelper
pub struct InputHandler<'a, D> {
    pub menu_handler: MenuHandler<'a, D>,
    /// Physical button bitmask received from core0 (bits 0–26 are wired buttons):
    pub current_button_state: u32,
    /// Raw encoder counts received from core0 (for display/lighting updates):
    pub encoder_p1_count: i32,
    pub encoder_p2_count: i32,
    /// Decoded encoder directions (used to build the logical bits):
    pub encoder_p1_direction: EncoderDirection,
    pub encoder_p2_direction: EncoderDirection,
    /// Combined state: lower 32 bits = current_button_state, upper 32 bits
    /// = logical (encoder-derived) buttons:
    pub current_combined_button_state: u64,
    pub previous_combined_button_state: u64,
}

impl<'a, D: WriteOnlyDataCommand> InputHandler<'a, D> {
    pub fn new(menu_handler: MenuHandler<'a, D>) -> Self {
        Self {
            menu_handler,
            current_button_state: 0_u32,
            encoder_p1_count: 0_i32,
            encoder_p2_count: 0_i32,
            encoder_p1_direction: EncoderDirection::Stopped,
            encoder_p2_direction: EncoderDirection::Stopped,
            current_combined_button_state: 0_u64,
            previous_combined_button_state: 0_u64,
        }
    }

    /// Routes events based on a change in button state.
    fn process_event(&mut self, event: ButtonEvents) {
        match event {
            ButtonEvents::Press(button) => {
                self.menu_handler.process_event(MenuEvents::Press(button));
            }
            ButtonEvents::Release(button) => debug!("RELEASE! {}", button as usize),
        };
    }

    /// Triggers a display refresh on the oled
    pub fn update_display(&mut self) {
        self.menu_handler.frames_rendered += 1;
        self.menu_handler.render_menu(
            self.current_combined_button_state,
            self.encoder_p1_count,
            self.encoder_p2_count,
        );
    }

    /// Builds the combined u64 state from the separate physical and logical
    /// pieces, then scans every [`ButtonCode`] bit to detect transitions and
    /// triggers events when button states change.
    pub fn detect_input_changes(&mut self) {
        // ── Build combined state from source-of-truth fields ──
        let mut combined = self.current_button_state as u64;
        match self.encoder_p1_direction {
            EncoderDirection::Positive => combined |= 1_u64 << (ButtonCode::P1Positive as usize),
            EncoderDirection::Negative => combined |= 1_u64 << (ButtonCode::P1Negative as usize),
            EncoderDirection::Stopped => {} // no logical button pressed
        }
        match self.encoder_p2_direction {
            EncoderDirection::Positive => combined |= 1_u64 << (ButtonCode::P2Positive as usize),
            EncoderDirection::Negative => combined |= 1_u64 << (ButtonCode::P2Negative as usize),
            EncoderDirection::Stopped => {} // no logical button pressed
        }
        self.current_combined_button_state = combined;

        // ── Uniform bit-scan over every ButtonCode ──
        let current = self.current_combined_button_state;
        let previous = self.previous_combined_button_state;
        for code in ButtonCode::iter() {
            let offset = code as usize;
            let pressed = (current >> offset) & 1 == 1;
            let was_pressed = (previous >> offset) & 1 == 1;
            if pressed && !was_pressed {
                self.process_event(ButtonEvents::Press(code));
            } else if !pressed && was_pressed {
                self.process_event(ButtonEvents::Release(code));
            }
        }
    }
}
