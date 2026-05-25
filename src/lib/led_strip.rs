//! WS2812 LED strip drivers for RP2350B (rp235x-hal).
//!
//! Provides two drivers for WS2812 (or compatible) addressable LED strips:
//!
//! * [`Ws2812`] — blocking, writes directly to the PIO TX FIFO.
//! * [`DmaLedStrip`] — non-blocking, uses DMA + double buffering.
//!
//! The PIO program and timing constants are shared between both drivers.
//! The code in this module is based on the
//! [ws2812-pio-rs](https://github.com/rp-rs/ws2812-pio-rs) project but has
//! been rewritten to work with the `rp235x-hal` 0.4 API and the `pio` 0.3
//! assembler rather than depending on the upstream crate's dependency chain.
//!
//! # Usage
//!
//! ## Blocking (simple, for testing)
//!
//! ```ignore
//! use iidx_controller_v2::led_strip::Ws2812;
//! use smart_leds::SmartLedsWrite;
//!
//! let (mut pio, sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
//! let mut strip = Ws2812::new(27, &mut pio, sm0, clocks.system_clock.freq());
//! strip.write(iterator_of_rgb8).unwrap();
//! ```
//!
//! ## Non-blocking DMA (for production)
//!
//! ```ignore
//! use iidx_controller_v2::led_strip::{DmaLedStrip, NUM_LEDS};
//!
//! let (mut pio, sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
//! // Build the PIO state machine via Ws2812 or manually, extract Tx ...
//! let mut led = DmaLedStrip::new(channel, tx);
//! led.write_frame(&[RGB8::new(255, 0, 0); NUM_LEDS], timer.get_counter().ticks());
//! ```

use fugit::HertzU32;
use rp235x_hal::dma::SingleChannel;
use rp235x_hal::dma::single_buffer::{Config as DmaConfig, Transfer as DmaTransfer};
use rp235x_hal::pio::{
    self as hal_pio, Buffers, PIOBuilder, PIOExt, PinDir, ShiftDirection, StateMachineIndex, Tx,
    UninitStateMachine, ValidStateMachine,
};
use smart_leds::{RGB8, SmartLedsWrite};

// ── WS2812 timing constants ────────────────────────────────────────────────
//
// The WS2812 protocol encodes each bit as:
//   T1 (start)  = 3 PIO cycles — side-set high
//   T2 (data)   = 3 PIO cycles — high for "1", low for "0"
//   T3 (stop)   = 4 PIO cycles — side-set low
//   ─────────────────────────
//   Total       = 10 cycles/bit
//
// At a PIO clock of 8 MHz the effective bit rate is 800 kHz, matching the
// WS2812 specification.  The clock divider is computed from the system clock
// (typically 125 MHz).

const T1: u8 = 3;
const T2: u8 = 3;
const T3: u8 = 4;
const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32; // 10
const WS2812_BIT_FREQ: HertzU32 = HertzU32::kHz(800);

/// Number of LEDs in the strip. Change this to match your hardware.
pub const NUM_LEDS: usize = 58;

/// LEDs per wiki side — half the strip, used as the const generic for per-player animations.
pub const LEDS_PER_SIDE: usize = NUM_LEDS / 2;

/// Minimum WS2812 RESET gap in timer ticks (1 tick = 1 µs).
const RESET_TICKS: u64 = 60;

/// Which of the two DMA buffers is currently in use.
#[derive(Clone, Copy, PartialEq)]
enum WhichBuf {
    A,
    B,
}

impl WhichBuf {
    fn other(&self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

/// Static DMA buffers — core1 is the sole writer, so no locking is needed.
static mut DMA_BUF_A: [u32; NUM_LEDS] = [0; NUM_LEDS];
static mut DMA_BUF_B: [u32; NUM_LEDS] = [0; NUM_LEDS];

// ── Blocking driver (phase 1) ──────────────────────────────────────────────

/// Driver for a chain of WS2812 (or compatible) addressable LEDs, driven by
/// a single PIO state machine with a side-set output pin.
///
/// The `write()` method busy-waits until every word has entered the PIO TX
/// FIFO.  This is a simple blocking implementation — the PIO still handles
/// all the serial timing autonomously while the CPU waits for FIFO space.
///
/// The driver implements [`SmartLedsWrite`] so it can be used directly with
/// any iterator of [`RGB8`] values (e.g. from `smart_leds::gamma`,
/// `smart_leds::brightness`, or the lighting controller crate).
pub struct Ws2812<P: PIOExt, SM: StateMachineIndex> {
    tx: Tx<(P, SM)>,
}

impl<P: PIOExt, SM: StateMachineIndex> Ws2812<P, SM> {
    /// Create a new WS2812 driver on the given PIO and state machine.
    ///
    /// * `pin_id`     — GPIO pin number for the data line (side-set pin).
    /// * `pio`        — Initialised PIO block (e.g. `PIO1`).
    /// * `sm`         — Uninitialised state machine to claim.
    /// * `clock_freq` — System clock frequency for the PIO clock divider
    ///                  (typically `clocks.system_clock.freq()` = 125 MHz).
    pub fn new(
        pin_id: u8,
        pio: &mut hal_pio::PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        clock_freq: HertzU32,
    ) -> Self {
        let tx = setup_ws2812_pio(pin_id, pio, sm, clock_freq);
        Self { tx }
    }
}

impl<P: PIOExt, SM: StateMachineIndex> SmartLedsWrite for Ws2812<P, SM> {
    type Color = RGB8;
    type Error = ();

    /// Write an iterator of [`RGB8`] colours to the LED chain.
    ///
    /// This method busy-waits until every word has been accepted by the PIO
    /// TX FIFO.
    ///
    /// The WS2812 expects data in **GRB** order (Green, Red, Blue), so each
    /// 24-bit colour word is assembled as `(g << 24) | (r << 16) | (b << 8)`.
    ///
    /// **Important:** You must wait at least 60 µs between successive writes
    /// (the WS2812 RESET period).  The frame timer in `main.rs` enforces this
    /// via the `LED_FRAME_TICKS` constant.
    fn write<T, J>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = J>,
        J: Into<Self::Color>,
    {
        for item in iterator {
            let color: RGB8 = item.into();
            let word =
                (u32::from(color.g) << 24) | (u32::from(color.r) << 16) | (u32::from(color.b) << 8);

            while !self.tx.write(word) {
                cortex_m::asm::nop();
            }
        }
        Ok(())
    }
}

// ── Non-blocking DMA driver (phase 2) ──────────────────────────────────────

/// Helper to build and start the WS2812 PIO program on a given PIO block and
/// state machine.  Returns the [`Tx`] handle, which can be used either with
/// [`Ws2812`] or with [`DmaLedStrip`].
pub fn setup_ws2812_pio<P: PIOExt, SM: StateMachineIndex>(
    pin_id: u8,
    pio: &mut hal_pio::PIO<P>,
    sm: UninitStateMachine<(P, SM)>,
    clock_freq: HertzU32,
) -> Tx<(P, SM)> {
    let program = pio::pio_asm!(
        ".side_set 1 opt",
        "",
        ".wrap_target",
        "bitloop:",
        "    out x, 1       side 0 [3]",
        "    jmp !x, do_zero side 1 [2]",
        "    jmp bitloop     side 1 [2]",
        "do_zero:",
        "    nop            side 0 [2]",
        ".wrap"
    );

    let installed = pio.install(&program.program).unwrap();

    let pio_freq = WS2812_BIT_FREQ * CYCLES_PER_BIT;
    let int_part: u32 = clock_freq / pio_freq;
    let rem_hz = clock_freq.to_Hz() - int_part * pio_freq.to_Hz();
    let frac_part = ((rem_hz * 256) / pio_freq.to_Hz()) as u8;
    let int_val: u16 = if int_part == 65536 {
        0
    } else {
        int_part as u16
    };

    let (mut sm, _, tx) = PIOBuilder::from_installed_program(installed)
        .buffers(Buffers::OnlyTx)
        .side_set_pin_base(pin_id)
        .out_shift_direction(ShiftDirection::Left)
        .autopull(true)
        .pull_threshold(24)
        .clock_divisor_fixed_point(int_val, frac_part)
        .build(sm);

    sm.set_pindirs([(pin_id, PinDir::Output)]);
    sm.start();

    tx
}

/// Non-blocking WS2812 driver using DMA + double buffering.
///
/// `write_frame()` converts RGB data to GRB u32 words in an internal buffer
/// and fires a single-shot DMA transfer to the PIO TX FIFO.  The method
/// returns immediately — the DMA engine feeds the PIO autonomously while
/// core1 continues with other work.
///
/// Double buffering means one buffer can be populated with the next frame's
/// data while the previous frame is still being transferred.
///
/// # Frame timing and the RESET gap
///
/// The WS2812 protocol requires a ≥60 µs low period (RESET) between frames.
/// The PIO naturally provides this by stalling on an empty TX FIFO after
/// each frame.  `write_frame()` enforces a minimum gap based on the known
/// serialisation duration (`NUM_LEDS × 30 µs`).
pub struct DmaLedStrip<CH: SingleChannel, SM: ValidStateMachine> {
    channel: Option<CH>,
    tx: Option<Tx<SM>>,
    active: WhichBuf,
    last_frame_tick: u64,
    transfer: Option<DmaTransfer<CH, &'static mut [u32], Tx<SM>>>,
}

impl<CH: SingleChannel, SM: ValidStateMachine> DmaLedStrip<CH, SM> {
    /// Create a new DMA-driven WS2812 driver.
    ///
    /// * `channel` — A DMA channel (e.g. from `dma.split()`).
    /// * `tx`      — The PIO TX FIFO handle from `PIOBuilder::build()`.
    pub fn new(channel: CH, tx: Tx<SM>) -> Self {
        Self {
            channel: Some(channel),
            tx: Some(tx),
            active: WhichBuf::A,
            last_frame_tick: 0,
            transfer: None,
        }
    }

    /// Send a frame of `NUM_LEDS` colours to the LED strip.
    ///
    /// This method converts `colors` to GRB-ordered u32 words in an internal
    /// buffer, starts a DMA transfer to the PIO TX FIFO, and returns
    /// immediately.  If a previous DMA transfer is still in flight when this
    /// method is called, it blocks until that transfer completes (this is
    /// the only possible blocking point and should be rare).
    ///
    /// * `colors` — Slice of [`RGB8`] values, typically `NUM_LEDS` long.
    ///   If shorter, the remaining LEDs are set to black (off).
    /// * `now`    — Current timer tick value (1 tick = 1 µs), used to
    ///   enforce the WS2812 RESET gap.
    pub fn write_frame(&mut self, colors: &[RGB8], now: u64) {
        // ── Reclaim channel and tx from any previous transfer ────
        if let Some(t) = self.transfer.take() {
            let (ch, _old_buf, tx) = t.wait();
            self.channel = Some(ch);
            self.tx = Some(tx);
        }

        // ── Enforce ≥60 µs RESET gap ────────────────────────────
        let serial_duration = NUM_LEDS as u64 * 30; // 30 µs per LED
        let pio_finished = self.last_frame_tick.wrapping_add(serial_duration);
        let min_next = pio_finished.wrapping_add(RESET_TICKS);
        while now < min_next {
            cortex_m::asm::nop();
        }
        self.last_frame_tick = now;

        // ── Take channel and tx (must exist here) ───────────────
        let ch = self.channel.take().expect("channel taken");
        let tx = self.tx.take().expect("tx taken");

        // ── Fill the inactive buffer with GRB words ─────────────
        let inactive = self.active.other();
        let buf: &'static mut [u32] = unsafe {
            match inactive {
                WhichBuf::A => {
                    let ptr = core::ptr::addr_of_mut!(DMA_BUF_A) as *mut u32;
                    core::slice::from_raw_parts_mut(ptr, NUM_LEDS)
                }
                WhichBuf::B => {
                    let ptr = core::ptr::addr_of_mut!(DMA_BUF_B) as *mut u32;
                    core::slice::from_raw_parts_mut(ptr, NUM_LEDS)
                }
            }
        };

        let n = colors.len().min(NUM_LEDS);
        for (i, &c) in colors[..n].iter().enumerate() {
            buf[i] = (u32::from(c.g) << 24) | (u32::from(c.r) << 16) | (u32::from(c.b) << 8);
        }
        // Zero out any remaining LEDs beyond the input length.
        for i in n..NUM_LEDS {
            buf[i] = 0;
        }

        // ── Fire DMA transfer (instant) ─────────────────────────
        let config = DmaConfig::new(ch, buf, tx);
        self.active = inactive;
        self.transfer = Some(config.start());
    }
}
