//! Library crate for IIDX Controller V2
//!
//! Shared types, constants, input handling, and display/menu logic
//! for the IIDX deck controller firmware.

#![no_std]

use rp235x_hal::gpio::{DynPinId, FunctionSioInput, Pin, PullDown};
use ssd1306::{Ssd1306, mode::BufferedGraphicsMode, prelude::*};
use strum::EnumIter;
use usbd_human_interface_device::page::Keyboard;

/// Identifies which of the two players/sides.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, EnumIter)]
pub enum Player {
    #[default]
    P1 = 0,
    P2 = 1,
}

pub mod flash_storage;
pub mod input_handler;
pub mod led_strip;
pub mod lighting_consts;
pub mod lighting_handler;
pub mod lighting_presets;
pub mod menu_handler;
pub mod menu_layout;
pub mod menu_settings;

pub use flash_storage::FLASH_STORAGE_BASE_ADDR;
pub use flash_storage::{ButtonConfig, EncoderConfig, FlashStoragePersistentMemory};
pub use flash_storage::{LightingConfig, PlayerAnimConfig};
pub use lighting_consts::*;

/// Type alias for the OLED display used throughout this project.
pub type OledDisplay<D> = Ssd1306<D, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

// ──────────────────────────────────────────────────────────────────────────────
// General configuration constants
// ──────────────────────────────────────────────────────────────────────────────

/// The frequency of the external clock crystal on the board (12 MHz).
pub const EXTERNAL_XTAL_FREQ: u32 = 12_000_000;

/// The number of GPIO pins being used as buttons (both keyboard and control center).
/// If you go higher than 27, you need to update how the SIO FIFO sends work between cores,
/// or the 31-27 bits will be eatn by the headers.
pub const NUM_BUTTONS: usize = 27;

/// Number of encoders.
pub const NUM_ENCODERS: usize = 2;

/// Number of players (sides of the LED strip).
pub const NUM_PLAYERS: usize = 2;

/// Logical buttons (not actual physical single-button-to-GPIO) start at this bit position in the
/// combined u64 state variable on core1. Physical button bits occupy 0–31.
const LOGICAL_BUTTON_OFFSET: usize = 32;

/// USB device tick interval (1 ms per USB spec).
pub const USB_TICK_INTERVAL_TICKS: u64 = 1_000;

/// Keyboard HID report send rate in timer ticks.
pub const USB_SEND_INTERVAL_TICKS: u64 = 1_000;

/// Heartbeat LED toggle rate for core0 (~4 Hz).
pub const CORE0_HEARTBEAT_RATE: u64 = 1_000_000 / 4;

/// Heartbeat LED toggle rate for core1 (~3 Hz).
pub const CORE1_HEARTBEAT_RATE: u64 = 1_000_000 / 3;

/// Min. time in ticks between LED strip refreshes (~144 Hz).
pub const LED_FRAME_TICKS: u64 = 6_944;

/// Idle timeout for encoder counts. If no input change occurs within this
/// window, the device performs a system reset.
pub const IDLE_RESET_TIMEOUT_TICKS: u64 = 10_000_000 * 900;

/// Minimum ticks between OLED screen refreshes (~10 Hz).
pub const SCREEN_REFRESH_TICKS: u64 = 100_000;

// ──────────────────────────────────────────────────────────────────────────────
// Inter-core FIFO protocol and flash storage headers
// ──────────────────────────────────────────────────────────────────────────────

/// Header for the current-button-state word sent from core0 to core1.
pub const CURRENT_BUTTON_STATE_HEADER: u32 = 0b10100;

/// Header for encoder P1 count sent from core0 to core1.
pub const ENCODER_P1_COUNT_HEADER: u32 = 0b10110;

/// Header for encoder P2 count sent from core0 to core1.
pub const ENCODER_P2_COUNT_HEADER: u32 = 0b10111;

/// Header for the encoder-direction word sent from core0 to core1.
/// Packs both encoder directions into the lowest 4 payload bits.
/// If you need more physical buttons form the spare pins, you can encode
/// them in this u32, but you'll need to rework the SIO FIFO logic on both cores
pub const ENCODER_DIRECTION_HEADER: u32 = 0b10101;

/// Every input source has a unique code that doubles as a bit offset
/// into the combined u64 state word on core1.  Physical button codes
/// (0–31) index the lower half; logical (encoder-derived) button codes
/// (32+) index the upper half, so the same bit-scanning logic handles both.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, EnumIter, strum::FromRepr)]
pub enum ButtonCode {
    // Physical buttons (0–31, currently 0–26 are wired)
    P1_1 = 0,
    P1_2 = 1,
    P1_3 = 2,
    P1_4 = 3,
    P1_5 = 4,
    P1_6 = 5,
    P1_7 = 6,
    P1Start = 7,
    P1Select = 8,
    P2_1 = 9,
    P2_2 = 10,
    P2_3 = 11,
    P2_4 = 12,
    P2_5 = 13,
    P2_6 = 14,
    P2_7 = 15,
    P2Start = 16,
    P2Select = 17,
    Escape = 18,
    CcUp = 19,
    CcDown = 20,
    CcLeft = 21,
    CcRight = 22,
    CcSelect = 23,
    VolumeUp = 24,
    VolumeDown = 25,
    Mute = 26,
    // Logical encoder-derived buttons (32+)
    P1Positive = LOGICAL_BUTTON_OFFSET + 0,
    P1Negative = LOGICAL_BUTTON_OFFSET + 1,
    P2Positive = LOGICAL_BUTTON_OFFSET + 2,
    P2Negative = LOGICAL_BUTTON_OFFSET + 3,
}

impl ButtonCode {
    /// Short human-readable label for this button (OLED-safe length).
    pub fn short_label(&self) -> &'static str {
        match self {
            Self::P1_1 => "P1_1",
            Self::P1_2 => "P1_2",
            Self::P1_3 => "P1_3",
            Self::P1_4 => "P1_4",
            Self::P1_5 => "P1_5",
            Self::P1_6 => "P1_6",
            Self::P1_7 => "P1_7",
            Self::P1Start => "P1_ST",
            Self::P1Select => "P1_SL",
            Self::P2_1 => "P2_1",
            Self::P2_2 => "P2_2",
            Self::P2_3 => "P2_3",
            Self::P2_4 => "P2_4",
            Self::P2_5 => "P2_5",
            Self::P2_6 => "P2_6",
            Self::P2_7 => "P2_7",
            Self::P2Start => "P2_ST",
            Self::P2Select => "P2_SL",
            Self::Escape => "Esc",
            Self::CcUp => "CC_UP",
            Self::CcDown => "CC_DN",
            Self::CcLeft => "CC_LT",
            Self::CcRight => "CC_RT",
            Self::CcSelect => "CC_SL",
            Self::VolumeUp => "V_UP",
            Self::VolumeDown => "V_DN",
            Self::Mute => "Mute",
            Self::P1Positive => "Wiki1+",
            Self::P1Negative => "Wiki1-",
            Self::P2Positive => "Wiki2+",
            Self::P2Negative => "Wiki2-",
        }
    }
}

/// Direction of turntable rotation.
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum EncoderDirection {
    Stopped = 0b00,
    Positive = 0b01,
    Negative = 0b10,
}

// used to pull direction from fifo encoded word data on core1
impl From<u8> for EncoderDirection {
    fn from(v: u8) -> Self {
        match v & 0b11 {
            0b01 => Self::Positive,
            0b10 => Self::Negative,
            _ => Self::Stopped,
        }
    }
}

/// Trait abstracting over the PIO Rx FIFO types so that [`EncoderState`]
/// can hold a reference to either SM0's or SM1's receiver in a single
/// homogeneous array.
pub trait PioRxReader {
    fn is_empty(&self) -> bool;
    fn read(&mut self) -> Option<u32>;
}

use rp235x_hal::pio::{Rx as HalRx, ValidStateMachine};

impl<SM: ValidStateMachine> PioRxReader for HalRx<SM> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn read(&mut self) -> Option<u32> {
        self.read()
    }
}

/// Per-button debounced state, including the associated GPIO pin, USB keyboard
/// mapping, and timing information for debounce.
pub struct ButtonState {
    pub name: &'static str,
    pub pin: Pin<DynPinId, FunctionSioInput, PullDown>,
    pub last_update_ticks: u64,
    pub debounce_ticks: u64,
    pub key: Option<Keyboard>,
    pub is_pressed: bool,
    pub was_pressed: bool,
}

impl ButtonState {
    /// Creates a new `ButtonState` with the given debounce ticks and
    /// initial unpressed state.
    pub fn new(
        name: &'static str,
        pin: Pin<DynPinId, FunctionSioInput, PullDown>,
        key: Option<Keyboard>,
        debounce_ticks: u64,
    ) -> Self {
        Self {
            name,
            pin,
            key,
            last_update_ticks: 0,
            debounce_ticks,
            is_pressed: false,
            was_pressed: false,
        }
    }
}

/// Per-encoder state: raw position, step-threshold anchor, direction,
/// move-timeout timing, the USB keys for each direction, and a reference
/// to the PIO Rx FIFO used to read raw quadrature ticks.
pub struct EncoderState<'a> {
    pub name: &'static str,
    pub key_up: Option<Keyboard>,
    pub key_down: Option<Keyboard>,
    pub count: i32,
    pub direction: EncoderDirection,
    /// Last position that crossed the step threshold. Used as the
    /// comparison anchor for delta calculations.
    pub anchor_count: i32,
    /// Timer tick stamp of the last threshold-crossing event.
    pub last_move_ticks: u64,
    /// Timer tick stamp of the last PIO FIFO read. Used for debounce calc.
    pub last_update_ticks: u64,
    /// Minimum ticks between accepting a new PIO sample.
    pub debounce_ticks: u64,
    /// Minimum delta before direction registers (hysteresis threshold).
    pub step_threshold: i32,
    /// Timer ticks of inactivity before releasing the encoder key.
    pub move_timeout_ticks: u64,
    /// Reference to this encoder's PIO Rx FIFO for reading raw counts.
    pub rx: &'a mut dyn PioRxReader,
}

impl<'a> EncoderState<'a> {
    /// Creates a new `EncoderState` with the given name, key bindings,
    /// PIO Rx FIFO reference, and threshold values.
    pub fn new(
        name: &'static str,
        key_up: Option<Keyboard>,
        key_down: Option<Keyboard>,
        rx: &'a mut dyn PioRxReader,
        debounce_ticks: u64,
        step_threshold: i32,
        move_timeout_ticks: u64,
    ) -> Self {
        Self {
            name,
            key_up,
            key_down,
            count: 0,
            anchor_count: 0,
            direction: EncoderDirection::Stopped,
            last_move_ticks: 0,
            last_update_ticks: 0,
            debounce_ticks,
            step_threshold,
            move_timeout_ticks,
            rx,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Inter-core communication helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Packs a 5-bit header into the top bits of a 27-bit payload word.
pub fn add_header_to_word(header: u32, word: u32) -> u32 {
    ((header & 0x1F) << 27) | (word & 0x07FF_FFFF)
}

/// Unpacks a word previously packed by [`add_header_to_word`].
/// Returns `(header, payload)`.
pub fn extract_header_from_word(encoded: u32) -> (u32, u32) {
    let header = (encoded >> 27) & 0x1F;
    let word = encoded & 0x07FF_FFFF;
    (header, word)
}

/// Sign-extends a 27-bit two's-complement value to a full `i32`.
pub fn sign_extend_27bit(value: u32) -> i32 {
    if value & 0x0400_0000 != 0 {
        (value | 0xF800_0000) as i32
    } else {
        value as i32
    }
}

/// Encodes the current and previous button states into two 27-bit bitmasks.
/// Returns `(current_state, previous_state)`.
pub fn encode_button_state(buttons: &[ButtonState]) -> (u32, u32) {
    let mut current_state = 0_u32;
    let mut previous_state = 0_u32;

    for (position, button) in buttons.iter().enumerate() {
        let mut is_pressed = button.is_pressed as u32;
        is_pressed = is_pressed << position;
        current_state |= is_pressed;

        let mut was_pressed = button.was_pressed as u32;
        was_pressed = was_pressed << position;
        previous_state |= was_pressed;
    }

    (current_state, previous_state)
}
