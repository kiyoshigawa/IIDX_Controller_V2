//! Input-handling module.
//!
//! Provides [`InputHandler`] — the core1-level orchestrator that receives
//! button/encoder state from core0 via SIO FIFO, detects changes, and
//! forwards events to [`MenuHandler`](crate::menu_handler::MenuHandler).

use crate::lighting_handler::{LightingEvent, LightingHandler};
use crate::menu_handler::{MenuEvents, MenuHandler};
use crate::{ButtonCode, EncoderDirection};
use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use strum::IntoEnumIterator;

/// Time a button must be held before a LongPress event fires (1 second at 1 µs/tick).
const DEFAULT_LONG_PRESS_DELAY_TICKS: u64 = 1_000_000;

/// Interval between Repeat events after the long press fires (200 ms).
const DEFAULT_REPEAT_INTERVAL_TICKS: u64 = 200_000;

/// Time of inactivity on CC buttons before sending a menu idle event (30 seconds).
const MENU_IDLE_TIMEOUT_TICKS: u64 = 30_000_000;

/// Per-button hold-tracking state for long-press and repeat detection.
#[derive(Clone, Copy)]
struct ButtonHoldState {
    press_start_tick: u64,
    long_press_fired: bool,
    last_repeat_tick: u64,
}

impl ButtonHoldState {
    const fn default() -> Self {
        Self {
            press_start_tick: 0,
            long_press_fired: false,
            last_repeat_tick: 0,
        }
    }
}

/// Events generated when a button's debounced state transitions.
pub(crate) enum ButtonEvents {
    Press(ButtonCode),
    Release(ButtonCode),
    LongPress(ButtonCode),
    Repeat(ButtonCode),
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
    /// Per-button hold-tracking state, indexed by ButtonCode as usize.
    hold_states: [ButtonHoldState; 36],
    /// Tick of the last CC button press (used for menu idle timeout).
    last_cc_press_tick: u64,
    /// Whether the idle event has already been sent for the current idle period.
    idle_event_sent: bool,
    /// Previous encoder counts for change detection.
    last_encoder_p1_count: i32,
    last_encoder_p2_count: i32,
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
            hold_states: [ButtonHoldState::default(); 36],
            last_cc_press_tick: 0,
            idle_event_sent: false,
            last_encoder_p1_count: 0_i32,
            last_encoder_p2_count: 0_i32,
        }
    }

    /// Routes events based on a change in button state.
    fn process_event(&mut self, event: ButtonEvents) {
        match event {
            ButtonEvents::Press(button) => {
                debug!("btn: {} pressed", button as usize);
                self.menu_handler.process_event(MenuEvents::Press(button));
            }
            ButtonEvents::Release(button) => debug!("btn: {} released", button as usize),
            ButtonEvents::LongPress(button) => {
                debug!("btn: {} long-press", button as usize);
                self.menu_handler
                    .process_event(MenuEvents::LongPress(button));
            }
            ButtonEvents::Repeat(button) => {
                debug!("btn: {} repeat", button as usize);
                self.menu_handler.process_event(MenuEvents::Repeat(button));
            }
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
    ///
    /// `current_tick` is the current timer tick value (used for long-press/repeat timing).
    pub fn detect_input_changes(&mut self, current_tick: u64) {
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

        // ── Pass 1: Uniform bit-scan over every ButtonCode for transitions ──
        let current = self.current_combined_button_state;
        let previous = self.previous_combined_button_state;
        for code in ButtonCode::iter() {
            let offset = code as usize;
            let pressed = (current >> offset) & 1 == 1;
            let was_pressed = (previous >> offset) & 1 == 1;
            if pressed && !was_pressed {
                self.hold_states[offset] = ButtonHoldState {
                    press_start_tick: current_tick,
                    long_press_fired: false,
                    last_repeat_tick: 0,
                };
                self.process_event(ButtonEvents::Press(code));
                if matches!(
                    code,
                    ButtonCode::CcUp
                        | ButtonCode::CcDown
                        | ButtonCode::CcLeft
                        | ButtonCode::CcRight
                        | ButtonCode::CcSelect
                ) {
                    self.last_cc_press_tick = current_tick;
                    self.idle_event_sent = false;
                }
            } else if !pressed && was_pressed {
                self.hold_states[offset] = ButtonHoldState::default();
                self.process_event(ButtonEvents::Release(code));
            }
        }

        // ── Pass 2: Check held buttons for long-press and repeat ──
        for code in ButtonCode::iter() {
            let offset = code as usize;
            let pressed = (current >> offset) & 1 == 1;
            if !pressed {
                continue;
            }
            let state = &mut self.hold_states[offset];
            if state.press_start_tick == 0 {
                // Button was already held before we started tracking; ignore.
                continue;
            }
            if !state.long_press_fired
                && current_tick.wrapping_sub(state.press_start_tick)
                    >= DEFAULT_LONG_PRESS_DELAY_TICKS
            {
                state.long_press_fired = true;
                state.last_repeat_tick = current_tick;
                self.process_event(ButtonEvents::LongPress(code));
            } else if state.long_press_fired
                && current_tick.wrapping_sub(state.last_repeat_tick)
                    >= DEFAULT_REPEAT_INTERVAL_TICKS
            {
                state.last_repeat_tick = current_tick;
                self.process_event(ButtonEvents::Repeat(code));
            }
        }

        // ── Menu idle timeout ──
        if !self.idle_event_sent
            && self.last_cc_press_tick != 0
            && current_tick.wrapping_sub(self.last_cc_press_tick) >= MENU_IDLE_TIMEOUT_TICKS
        {
            self.idle_event_sent = true;
            self.menu_handler.process_event(MenuEvents::Idle);
        }
    }

    /// Check encoder counts for changes and forward events to the lighting handler.
    /// Called from the core1 loop after `detect_input_changes()`.
    pub fn process_encoder_events(&mut self, lighting_handler: &mut LightingHandler) {
        if self.encoder_p1_count != self.last_encoder_p1_count {
            self.last_encoder_p1_count = self.encoder_p1_count;
            lighting_handler.handle_event(LightingEvent::EncoderMoved {
                player: 0,
                count: self.encoder_p1_count,
            });
        }
        if self.encoder_p2_count != self.last_encoder_p2_count {
            self.last_encoder_p2_count = self.encoder_p2_count;
            lighting_handler.handle_event(LightingEvent::EncoderMoved {
                player: 1,
                count: self.encoder_p2_count,
            });
        }
    }

    /// Check gameplay and encoder-logical buttons for rising edges and forward
    /// [`LightingEvent::ButtonPressed`] to the lighting handler.
    pub fn process_button_events(&mut self, lighting_handler: &mut LightingHandler) {
        let current = self.current_combined_button_state;
        let previous = self.previous_combined_button_state;

        // P1 gameplay (codes 0..=6) + encoder logicals (32..=33)
        for offset in [0, 1, 2, 3, 4, 5, 6, 32, 33] {
            if ((current >> offset) & 1 == 1) && ((previous >> offset) & 1 == 0) {
                lighting_handler.handle_event(LightingEvent::ButtonPressed { player: 0 });
            }
        }

        // P2 gameplay (codes 9..=15) + encoder logicals (34..=35)
        for offset in [9, 10, 11, 12, 13, 14, 15, 34, 35] {
            if ((current >> offset) & 1 == 1) && ((previous >> offset) & 1 == 0) {
                lighting_handler.handle_event(LightingEvent::ButtonPressed { player: 1 });
            }
        }
    }
}
