//! Pre-configured lighting presets — 20 per-player `PlayerAnimConfig` consts
//! forming 10 preset slots with distinct background rainbows.
//!
//! Each slot pairs a P1 config (rainbow 0–9) with a P2 config (rainbow 10–19).
//! To customise any preset, edit the corresponding const below.

use crate::{
    BgMode, Direction, FgMode, LightingConfig, PlayerAnimConfig, Rainbow, TrigMode, TrigOffset,
};

// ── Preset 0 ────────────────────────────────────────────────────────

pub const PRESET_0_P1: PlayerAnimConfig = PlayerAnimConfig {
    bg_mode: BgMode::Follow as u8,
    bg_rainbow: Rainbow::Oklch as u8,
    bg_subdivisions: 2,
    bg_speed_ds: 50,
    bg_dir: Direction::Fwd as u8,
    fg_mode: FgMode::Off as u8,
    fg_rainbow: Rainbow::Rgb as u8,
    fg_subdivisions: 1,
    fg_speed_ds: 50,
    fg_step_ds: 4,
    fg_leds_per_group: 1,
    fg_dir: Direction::Fwd as u8,
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_0_P2: PlayerAnimConfig = PlayerAnimConfig {
    bg_mode: BgMode::Follow as u8,
    bg_rainbow: Rainbow::Oklch as u8,
    bg_subdivisions: 2,
    bg_speed_ds: 50,
    bg_dir: Direction::Fwd as u8,
    fg_mode: FgMode::Off as u8,
    fg_rainbow: Rainbow::Rgb as u8,
    fg_subdivisions: 1,
    fg_speed_ds: 50,
    fg_step_ds: 4,
    fg_leds_per_group: 1,
    fg_dir: Direction::Fwd as u8,
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 1 ────────────────────────────────────────────────────────

pub const PRESET_1_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_1_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 2 ────────────────────────────────────────────────────────

pub const PRESET_2_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_2_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 3 ────────────────────────────────────────────────────────

pub const PRESET_3_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_3_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 4 ────────────────────────────────────────────────────────

pub const PRESET_4_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_4_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 5 ────────────────────────────────────────────────────────

pub const PRESET_5_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_5_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 6 ────────────────────────────────────────────────────────

pub const PRESET_6_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_6_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 7 ────────────────────────────────────────────────────────

pub const PRESET_7_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_7_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 8 ────────────────────────────────────────────────────────

pub const PRESET_8_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_8_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset 9 ────────────────────────────────────────────────────────

pub const PRESET_9_P1: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

pub const PRESET_9_P2: PlayerAnimConfig = PlayerAnimConfig {
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
    trig_mode: TrigMode::ShotRainbow as u8,
    trig_rainbow: Rainbow::Ryb as u8,
    trig_fade_in_ms: 50,
    trig_fade_out_ms: 200,
    trig_width_in_leds: 3,
    trig_dir: Direction::Fwd as u8,
    trig_offset: TrigOffset::Center as u8,
    trig_dur_ds: 1,
};

// ── Preset lookup ───────────────────────────────────────────────────

/// Returns the default [`LightingConfig`] for a given preset slot index (0–9).
pub fn default_preset(idx: usize) -> LightingConfig {
    let (p1, p2) = match idx {
        0 => (PRESET_0_P1, PRESET_0_P2),
        1 => (PRESET_1_P1, PRESET_1_P2),
        2 => (PRESET_2_P1, PRESET_2_P2),
        3 => (PRESET_3_P1, PRESET_3_P2),
        4 => (PRESET_4_P1, PRESET_4_P2),
        5 => (PRESET_5_P1, PRESET_5_P2),
        6 => (PRESET_6_P1, PRESET_6_P2),
        7 => (PRESET_7_P1, PRESET_7_P2),
        8 => (PRESET_8_P1, PRESET_8_P2),
        9 => (PRESET_9_P1, PRESET_9_P2),
        _ => (PRESET_0_P1, PRESET_0_P2),
    };
    LightingConfig {
        players: [p1, p2],
        brightness: 200,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Dump helpers — convert u8 → Rust enum variant name strings for defmt output
// ──────────────────────────────────────────────────────────────────────────────

fn fmt_bg_mode(v: u8) -> &'static str {
    match BgMode::from(v) {
        BgMode::Rotate => "Rotate",
        BgMode::Follow => "Follow",
        BgMode::Solid => "Solid",
        BgMode::SolidFade => "SolidFade",
        BgMode::Off => "Off",
    }
}

fn fmt_fg_mode(v: u8) -> &'static str {
    match FgMode::from(v) {
        FgMode::Off => "Off",
        FgMode::Marquee => "Marquee",
        FgMode::MarqueeFixed => "MarqueeFixed",
        FgMode::MarqueeFade => "MarqueeFade",
        FgMode::MarqueeFadeFixed => "MarqueeFadeFixed",
        FgMode::VUMeter => "VUMeter",
    }
}

fn fmt_trig_mode(v: u8) -> &'static str {
    match TrigMode::from(v) {
        TrigMode::Off => "Off",
        TrigMode::Pulse => "Pulse",
        TrigMode::PulseFade => "PulseFade",
        TrigMode::PulseRainbow => "PulseRainbow",
        TrigMode::Shot => "Shot",
        TrigMode::ShotFade => "ShotFade",
        TrigMode::ShotRainbow => "ShotRainbow",
        TrigMode::Flash => "Flash",
        TrigMode::FlashFade => "FlashFade",
        TrigMode::FlashRainbow => "FlashRainbow",
    }
}

fn fmt_dir(v: u8) -> &'static str {
    match Direction::from(v) {
        Direction::Fwd => "Fwd",
        Direction::Stop => "Stop",
        Direction::Rev => "Rev",
    }
}

fn fmt_trig_offset(v: u8) -> &'static str {
    match TrigOffset::from(v) {
        TrigOffset::Random => "Random",
        TrigOffset::Center => "Center",
        TrigOffset::Top => "Top",
    }
}

fn fmt_rainbow(v: u8) -> &'static str {
    match Rainbow::from(v) {
        Rainbow::Oklch => "Oklch",
        Rainbow::OkWgt => "OkWgt",
        Rainbow::Ryb => "Ryb",
        Rainbow::Ogp => "Ogp",
        Rainbow::Rgb => "Rgb",
        Rainbow::By => "By",
        Rainbow::Rb => "Rb",
        Rainbow::Black => "Black",
        Rainbow::White => "White",
        Rainbow::Red => "Red",
        Rainbow::Orange => "Orange",
        Rainbow::Yellow => "Yellow",
        Rainbow::Lime => "Lime",
        Rainbow::Spring => "Spring",
        Rainbow::Cyan => "Cyan",
        Rainbow::DpBlue => "DpBlue",
        Rainbow::Blue => "Blue",
        Rainbow::BlPurp => "BlPurp",
        Rainbow::Fuchsia => "Fuchsia",
        Rainbow::DkPurp => "DkPurp",
    }
}

/// Print a [`PlayerAnimConfig`] as valid Rust over defmt, ready to paste
/// into a preset const definition.  The entire block uses \r\n so only one
/// `[INFO]` tag appears in the terminal output.
pub fn dump_player_config(label: &str, cfg: &PlayerAnimConfig) {
    defmt::info!(
        "\r\n\
         \r\n\
         // {}\r\n\
         PlayerAnimConfig {{\r\n\
             bg_mode: BgMode::{} as u8,\r\n\
             bg_rainbow: Rainbow::{} as u8,\r\n\
             bg_subdivisions: {=u8},\r\n\
             bg_speed_ds: {=u16},\r\n\
             bg_dir: Direction::{} as u8,\r\n\
             fg_mode: FgMode::{} as u8,\r\n\
             fg_rainbow: Rainbow::{} as u8,\r\n\
             fg_subdivisions: {=u8},\r\n\
             fg_speed_ds: {=u16},\r\n\
             fg_step_ds: {=u16},\r\n\
             fg_leds_per_group: {=u8},\r\n\
             fg_dir: Direction::{} as u8,\r\n\
             trig_mode: TrigMode::{} as u8,\r\n\
             trig_rainbow: Rainbow::{} as u8,\r\n\
             trig_fade_in_ms: {=u16},\r\n\
             trig_fade_out_ms: {=u16},\r\n\
             trig_width_in_leds: {=u8},\r\n\
             trig_dir: Direction::{} as u8,\r\n\
             trig_offset: TrigOffset::{} as u8,\r\n\
             trig_dur_ds: {=u8},\r\n\
         }}\r\n",
        label,
        fmt_bg_mode(cfg.bg_mode),
        fmt_rainbow(cfg.bg_rainbow),
        cfg.bg_subdivisions,
        cfg.bg_speed_ds,
        fmt_dir(cfg.bg_dir),
        fmt_fg_mode(cfg.fg_mode),
        fmt_rainbow(cfg.fg_rainbow),
        cfg.fg_subdivisions,
        cfg.fg_speed_ds,
        cfg.fg_step_ds,
        cfg.fg_leds_per_group,
        fmt_dir(cfg.fg_dir),
        fmt_trig_mode(cfg.trig_mode),
        fmt_rainbow(cfg.trig_rainbow),
        cfg.trig_fade_in_ms,
        cfg.trig_fade_out_ms,
        cfg.trig_width_in_leds,
        fmt_dir(cfg.trig_dir),
        fmt_trig_offset(cfg.trig_offset),
        cfg.trig_dur_ds,
    );
}
