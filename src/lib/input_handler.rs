//! Input-handling module.
//!
//! Provides [`InputHandler`] — the core1-level orchestrator that receives
//! button/encoder state from core0 via SIO FIFO, detects changes, and
//! forwards events to [`MenuHandler`](crate::menu_handler::MenuHandler).

use crate::ButtonOffsets;
use crate::menu_handler::{MenuEvents, MenuHandler};
use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use strum::IntoEnumIterator;

// ──────────────────────────────────────────────────────────────────────────────
// ButtonEvents
// ──────────────────────────────────────────────────────────────────────────────

/// Events generated when a button's debounced state transitions.
pub(crate) enum ButtonEvents {
    Press(ButtonOffsets),
    Release(ButtonOffsets),
}

// ──────────────────────────────────────────────────────────────────────────────
// InputHandler
// ──────────────────────────────────────────────────────────────────────────────

/// Core1-level orchestrator for inputs.
///
/// Holds the current and previous 27-bit button-state bitmask, encoder counts
/// received from core0, and a [`MenuHandler`] that processes menu-related
/// button events and drives the OLED display.
pub struct InputHandler<'a, D> {
    pub menu_handler: MenuHandler<'a, D>,
    pub current_button_state: u32,
    pub previous_button_state: u32,
    pub encoder_p1_count: i32,
    pub encoder_p2_count: i32,
}

impl<'a, D: WriteOnlyDataCommand> InputHandler<'a, D> {
    /// Constructs a new `InputHandler` wrapping the given `MenuHandler`.
    pub fn new(menu_handler: MenuHandler<'a, D>) -> Self {
        Self {
            menu_handler,
            current_button_state: 0_u32,
            previous_button_state: 0_u32,
            encoder_p1_count: 0_i32,
            encoder_p2_count: 0_i32,
        }
    }

    /// Routes a single [`ButtonEvents`] to the menu handler.
    fn process_event(&mut self, event: ButtonEvents) {
        match event {
            ButtonEvents::Press(button) => {
                self.menu_handler.process_event(MenuEvents::Press(button));
            }
            ButtonEvents::Release(button) => debug!("RELEASE! {}", button as usize),
        };
    }

    /// Triggers a display refresh (increments frame counter and redraws).
    pub fn update_display(&mut self) {
        self.menu_handler.frames_rendered += 1;
        self.menu_handler.render_menu(
            self.current_button_state,
            self.encoder_p1_count,
            self.encoder_p2_count,
        );
    }

    /// Scans every button bit in `current_button_state` vs
    /// `previous_button_state` and fires press/release events when a
    /// transition is detected.
    pub fn detect_input_changes(&mut self) {
        let current_state = self.current_button_state;
        let previous_state = self.previous_button_state;
        for button in ButtonOffsets::iter() {
            let offset = button as usize;
            let current_pressed = (current_state >> offset) & 1 == 1;
            let previous_pressed = (previous_state >> offset) & 1 == 1;
            if current_pressed && !previous_pressed {
                self.process_event(ButtonEvents::Press(button));
            } else if !current_pressed && previous_pressed {
                self.process_event(ButtonEvents::Release(button));
            }
        }
    }
}
