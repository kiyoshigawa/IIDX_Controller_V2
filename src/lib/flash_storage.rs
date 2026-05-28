//! Flash storage types and programming helpers.
//!
//! Provides the persistent configuration struct (`FlashStoragePersistentMemory`),
//! its constituent config types, and the low-level `write_storage` / `clear_storage`
//! functions that program the on-chip flash via the RP2350's rom_data API.

use core::sync::atomic::{AtomicBool, Ordering};
use rp235x_hal as hal;
use usbd_human_interface_device::page::Keyboard;

use crate::lighting_presets::default_preset;
use crate::{BgMode, Direction, FgMode, NUM_BUTTONS, NUM_ENCODERS, Rainbow, TrigMode, TrigOffset};

// ──────────────────────────────────────────────────────────────────────────────
// Flash address layout
// ──────────────────────────────────────────────────────────────────────────────

/// Address in memory of the flash storage dedicated to persistent memory on the chip.
/// Configured in memory.x as 'STORAGE' — currently 1024 KiB.
pub const FLASH_STORAGE_BASE_ADDR: u32 = 0x10F00000;

/// Flash byte offset (relative to start of flash) for the storage region.
/// FLASH_STORAGE_BASE_ADDR = 0x10F00000 => offset = 0x00F00000
pub const FLASH_STORAGE_OFFSET: u32 = 0x00F00000;

/// Magic value written to the first word of flash to indicate initialised storage.
const FLASH_HEADER: u32 = 0xA5A5A5A5;

/// Bitwise inverse of [`FLASH_HEADER`]; the pair together provide a robust
/// "is-initialised" check against erased (0xFF) flash.
const FLASH_HEADER_INV: u32 = 0x5A5A5A5A;

/// Magic word written to the last 8 bytes of the struct as a version sentinel.
/// When the struct layout or size changes, this sentinel lands at a different
/// byte offset from the start, so the old bytes at that position won't match.
/// This provides an automatic self-check against struct shape changes without
/// per-field migration logic.
const FLASH_FOOTER: u32 = 0xAAAAAAAA;

/// Bitwise inverse of [`FLASH_FOOTER`].
const FLASH_FOOTER_INV: u32 = 0x55555555;

/// RP2350 flash sector size (4 KB). Used for erase operations.
pub(crate) const FLASH_SECTOR_SIZE: u32 = 4096;

/// RP2350 flash page size (256 B). Used for program operations.
pub(crate) const FLASH_PAGE_SIZE: u32 = 256;

/// Flags passed to the ROM erase API (standard erase mode).
pub(crate) const FLASH_ERASE_FLAGS: u8 = 0x20;

/// Atomic flag to prevent concurrent flash writes from both cores.
pub static FLASH_WRITE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Set by the core that wants to write flash, signalling both cores to
/// enter a safe spin-loop in RAM before the erase/program cycle begins.
pub static FLASH_PREPARE_FLAG: AtomicBool = AtomicBool::new(false);

/// Set by core0 once it has acknowledged [`FLASH_PREPARE_FLAG`] and is spinning.
pub static FLASH_CORE0_READY: AtomicBool = AtomicBool::new(false);

/// Set by core1 once it has acknowledged [`FLASH_PREPARE_FLAG`] and is spinning.
pub static FLASH_CORE1_READY: AtomicBool = AtomicBool::new(false);

/// Set by core1 after a successful flash write to signal core0 to call `sys_reset()`.
/// Core0 checks this flag after exiting the safe loop.
pub static FLASH_PENDING_REBOOT: AtomicBool = AtomicBool::new(false);

// ──────────────────────────────────────────────────────────────────────────────
// Default encoder timing constants (only used by FlashStoragePersistentMemory)
// ──────────────────────────────────────────────────────────────────────────────

/// Default button debounce time in timer ticks (1,000,000 ticks per second).
const DEFAULT_BUTTON_DEBOUNCE_TICKS: u64 = 10_000;

/// Default encoder debounce time in timer ticks (1,000,000 ticks per second).
const DEFAULT_ENCODER_DEBOUNCE_TICKS: u64 = 1_000;

/// Default minimum encoder delta before direction registers (hysteresis threshold).
const DEFAULT_ENCODER_STEP_THRESHOLD: i32 = 20;

/// Default timer ticks of inactivity before releasing the turntable key (100 ms).
const DEFAULT_ENCODER_MOVE_TIMEOUT_TICKS: u64 = 100_000;

// ──────────────────────────────────────────────────────────────────────────────
// Configuration structs (mirrored in flash)
// ──────────────────────────────────────────────────────────────────────────────

/// Per-button configuration stored in flash.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ButtonConfig {
    /// USB HID key code (0 = NoEventIndicated, meaning no key).
    pub key: u8,
    /// Debounce time in timer ticks (1,000,000 ticks per second).
    pub debounce_ticks: u64,
}

/// Per-encoder configuration stored in flash.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EncoderConfig {
    /// USB HID key code for clockwise rotation.
    pub key_up: u8,
    /// USB HID key code for counter-clockwise rotation.
    pub key_down: u8,
    /// Debounce time in timer ticks.
    pub debounce_ticks: u64,
    /// Step threshold (hysteresis) for direction detection.
    pub step_threshold: i32,
    /// Timer ticks of inactivity before releasing the encoder key.
    pub move_timeout_ticks: u64,
}

/// Number of lighting presets stored in flash.
pub const NUM_PRESETS: usize = 10;

/// Per-player animation configuration stored in flash.
/// Each field is an index into the corresponding `lighting_consts::*_NAMES` array.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default)]
pub struct PlayerAnimConfig {
    // BG settings
    pub bg_mode: u8,    // index into BG_MODE_NAMES
    pub bg_rainbow: u8, // index into RAINBOW_NAMES
    pub bg_subdivisions: u8,
    pub bg_speed_ds: u16, // deciseconds (÷10 → seconds)
    pub bg_dir: u8,       // 0=Positive, 1=Stopped, 2=Negative

    // FG settings
    pub fg_mode: u8,    // index into FG_MODE_NAMES
    pub fg_rainbow: u8, // index into RAINBOW_NAMES
    pub fg_subdivisions: u8,
    pub fg_speed_ds: u16, // deciseconds
    pub fg_step_ds: u16,  // deciseconds
    pub fg_leds_per_group: u8,
    pub fg_dir: u8, // 0=Positive, 1=Stopped, 2=Negative

    // Trigger settings
    pub trig_mode: u8,    // index into TRIG_MODE_NAMES
    pub trig_rainbow: u8, // index into RAINBOW_NAMES
    pub trig_fade_in_ms: u16,
    pub trig_fade_out_ms: u16,
    pub trig_width_in_leds: u8,
    pub trig_dir: u8,    // starting direction for alternation
    pub trig_offset: u8, // 0=Random, 1=Center, 2=Top
    pub trig_dur_ds: u8, // trigger cycle duration in deciseconds
}

/// Lighting configuration stored in flash.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Default)]
pub struct LightingConfig {
    pub players: [PlayerAnimConfig; 2], // [0]=P1, [1]=P2
    pub brightness: u8,
}

/// The full persistent memory structure laid out at [`FLASH_STORAGE_BASE_ADDR`].
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlashStoragePersistentMemory {
    pub header: u32,
    pub header_inv: u32,
    pub buttons: [ButtonConfig; NUM_BUTTONS],
    pub encoders: [EncoderConfig; NUM_ENCODERS],
    pub lighting: LightingConfig,
    pub presets: [LightingConfig; NUM_PRESETS],
    pub active_preset: u8,
    pub footer: u32,
    pub footer_inv: u32,
}

impl FlashStoragePersistentMemory {
    /// Returns `true` if the flash header words match the expected magic values.
    pub fn has_been_written(&self) -> bool {
        self.header == FLASH_HEADER && self.header_inv == FLASH_HEADER_INV
    }

    /// Returns `true` if the struct layout and size match the current version.
    ///
    /// Checks the trailing sentinel words. If these don't match, the struct
    /// size or field layout has changed since the data was written, and
    /// defaults should be rewritten. This is the single source of truth for
    /// detecting format changes — no per-field migration checks needed.
    pub fn has_valid_layout(&self) -> bool {
        self.footer == FLASH_FOOTER && self.footer_inv == FLASH_FOOTER_INV
    }

    /// Factory-default configuration matching the original template key bindings.
    pub fn default() -> Self {
        // Build the button config array using the current default key mappings.
        // key=0 means None (no key mapped); they are set explicitly below.
        let mut buttons = [ButtonConfig {
            key: 0,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; NUM_BUTTONS];

        // Physical buttons with default key bindings
        buttons[0] = ButtonConfig {
            key: Keyboard::Z as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_1
        buttons[1] = ButtonConfig {
            key: Keyboard::S as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_2
        buttons[2] = ButtonConfig {
            key: Keyboard::X as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_3
        buttons[3] = ButtonConfig {
            key: Keyboard::D as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_4
        buttons[4] = ButtonConfig {
            key: Keyboard::C as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_5
        buttons[5] = ButtonConfig {
            key: Keyboard::F as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_6
        buttons[6] = ButtonConfig {
            key: Keyboard::V as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_7
        buttons[7] = ButtonConfig {
            key: Keyboard::Grave as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_Start
        buttons[8] = ButtonConfig {
            key: Keyboard::Keyboard1 as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P1_Select
        buttons[9] = ButtonConfig {
            key: Keyboard::M as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_1
        buttons[10] = ButtonConfig {
            key: Keyboard::K as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_2
        buttons[11] = ButtonConfig {
            key: Keyboard::Comma as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_3
        buttons[12] = ButtonConfig {
            key: Keyboard::L as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_4
        buttons[13] = ButtonConfig {
            key: Keyboard::Dot as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_5
        buttons[14] = ButtonConfig {
            key: Keyboard::Semicolon as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_6
        buttons[15] = ButtonConfig {
            key: Keyboard::ForwardSlash as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_7
        buttons[16] = ButtonConfig {
            key: Keyboard::DeleteBackspace as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_Start
        buttons[17] = ButtonConfig {
            key: Keyboard::Equal as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // P2_Select
        buttons[18] = ButtonConfig {
            key: Keyboard::Escape as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // Escape
        // Center-console buttons have no default USB key, but still need debounce ticks.
        buttons[19] = ButtonConfig {
            key: Keyboard::NoEventIndicated as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // CC_Up
        buttons[20] = ButtonConfig {
            key: Keyboard::NoEventIndicated as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // CC_Down
        buttons[21] = ButtonConfig {
            key: Keyboard::NoEventIndicated as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // CC_Left
        buttons[22] = ButtonConfig {
            key: Keyboard::NoEventIndicated as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // CC_Right
        buttons[23] = ButtonConfig {
            key: Keyboard::NoEventIndicated as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // CC_Select
        buttons[24] = ButtonConfig {
            key: Keyboard::VolumeUp as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // Volume_Up
        buttons[25] = ButtonConfig {
            key: Keyboard::VolumeDown as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // Volume_Down
        buttons[26] = ButtonConfig {
            key: Keyboard::Mute as u8,
            debounce_ticks: DEFAULT_BUTTON_DEBOUNCE_TICKS,
        }; // Mute

        let encoders = [
            EncoderConfig {
                key_up: Keyboard::LeftShift as u8,
                key_down: Keyboard::LeftControl as u8,
                debounce_ticks: DEFAULT_ENCODER_DEBOUNCE_TICKS,
                step_threshold: DEFAULT_ENCODER_STEP_THRESHOLD,
                move_timeout_ticks: DEFAULT_ENCODER_MOVE_TIMEOUT_TICKS,
            },
            EncoderConfig {
                key_up: Keyboard::RightControl as u8,
                key_down: Keyboard::RightShift as u8,
                debounce_ticks: DEFAULT_ENCODER_DEBOUNCE_TICKS,
                step_threshold: DEFAULT_ENCODER_STEP_THRESHOLD,
                move_timeout_ticks: DEFAULT_ENCODER_MOVE_TIMEOUT_TICKS,
            },
        ];

        let p1_default = PlayerAnimConfig {
            bg_mode: BgMode::Follow as u8,
            bg_rainbow: Rainbow::Oklch as u8,
            bg_subdivisions: 1,
            bg_speed_ds: 50,
            bg_dir: Direction::Fwd as u8,
            fg_mode: FgMode::Off as u8,
            fg_rainbow: Rainbow::Rgb as u8,
            fg_subdivisions: 1,
            fg_speed_ds: 50,
            fg_step_ds: 4,
            fg_leds_per_group: 1,
            fg_dir: Direction::Fwd as u8,
            trig_mode: TrigMode::Pulse as u8,
            trig_rainbow: Rainbow::Black as u8,
            trig_fade_in_ms: 100,
            trig_fade_out_ms: 500,
            trig_width_in_leds: 3,
            trig_dir: Direction::Fwd as u8,
            trig_offset: TrigOffset::Random as u8,
            trig_dur_ds: 10,
        };
        let players = [p1_default; 2];
        let lighting = LightingConfig {
            players,
            brightness: 200,
        };

        // Build the preset array from lighting_presets module.
        let presets = [
            default_preset(0),
            default_preset(1),
            default_preset(2),
            default_preset(3),
            default_preset(4),
            default_preset(5),
            default_preset(6),
            default_preset(7),
            default_preset(8),
            default_preset(9),
        ];

        Self {
            header: FLASH_HEADER,
            header_inv: FLASH_HEADER_INV,
            buttons,
            encoders,
            lighting,
            presets,
            active_preset: 0,
            footer: FLASH_FOOTER,
            footer_inv: FLASH_FOOTER_INV,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Flash programming helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Write the full [`FlashStoragePersistentMemory`] struct to flash, overwriting
/// the storage region.  Erases the necessary sectors first, then programs each
/// 256-byte page.
///
/// Uses an atomic guard (`FLASH_WRITE_IN_PROGRESS`) to prevent concurrent
/// calls from both cores.  If the guard is already claimed, the function logs
/// a warning and returns immediately without writing.
pub unsafe fn write_storage(storage: &FlashStoragePersistentMemory) {
    // Atomic guard: if someone else is already writing, skip.
    if FLASH_WRITE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        defmt::info!("write_storage: another write is already in progress, skipping.");
        return;
    }

    let struct_size = core::mem::size_of::<FlashStoragePersistentMemory>();
    let struct_ptr = storage as *const FlashStoragePersistentMemory as *const u8;

    // Step 1: Erase enough 4 KB sectors to cover the entire struct.
    let erase_size =
        ((struct_size as u32 + (FLASH_SECTOR_SIZE - 1)) / FLASH_SECTOR_SIZE) * FLASH_SECTOR_SIZE;
    unsafe {
        hal::rom_data::flash_range_erase(
            FLASH_STORAGE_OFFSET,
            erase_size as usize,
            FLASH_SECTOR_SIZE,
            FLASH_ERASE_FLAGS,
        );
    }

    // Step 2: Program the struct data one page at a time.
    let mut remaining = struct_size;
    let mut src = struct_ptr;
    let mut flash_offs = FLASH_STORAGE_OFFSET;
    while remaining > 0 {
        let mut page_buf = [0xFFu8; FLASH_PAGE_SIZE as usize];
        let chunk_len = core::cmp::min(remaining, FLASH_PAGE_SIZE as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(src, page_buf.as_mut_ptr(), chunk_len);
        }
        unsafe {
            hal::rom_data::flash_range_program(
                flash_offs,
                page_buf.as_ptr(),
                FLASH_PAGE_SIZE as usize,
            );
        }
        remaining -= chunk_len;
        src = unsafe { src.add(chunk_len) };
        flash_offs += FLASH_PAGE_SIZE;
    }

    // Step 3: Flush the XIP cache so subsequent XIP reads see the new data.
    unsafe {
        hal::rom_data::flash_flush_cache();
    }

    // Release the atomic guard
    FLASH_WRITE_IN_PROGRESS.store(false, Ordering::Release);
}

/// Erase the persistent storage region, restoring flash to an uninitialised state.
/// On the next boot the device will detect fresh storage and re-write the defaults.
///
/// The erase size is calculated from `core::mem::size_of_val(storage)` so it
/// automatically covers the full struct regardless of future growth.
#[allow(dead_code)]
pub unsafe fn clear_storage(storage: &FlashStoragePersistentMemory) {
    // Atomic guard: if someone else is already writing, skip.
    if FLASH_WRITE_IN_PROGRESS.swap(true, Ordering::AcqRel) {
        defmt::info!("clear_storage: another write is already in progress, skipping.");
        return;
    }

    let struct_size = core::mem::size_of_val(storage);
    let erase_size =
        ((struct_size as u32 + (FLASH_SECTOR_SIZE - 1)) / FLASH_SECTOR_SIZE) * FLASH_SECTOR_SIZE;

    unsafe {
        hal::rom_data::flash_range_erase(
            FLASH_STORAGE_OFFSET,
            erase_size as usize,
            FLASH_SECTOR_SIZE,
            FLASH_ERASE_FLAGS,
        );
    }

    // Flush the XIP cache so subsequent reads see the erased (0xFF) state.
    unsafe {
        hal::rom_data::flash_flush_cache();
    }

    FLASH_WRITE_IN_PROGRESS.store(false, Ordering::Release);
    defmt::info!("Persistent storage cleared. Next boot will re-initialise defaults.");
}
