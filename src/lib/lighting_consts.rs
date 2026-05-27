//! Rainbow palette constants and default animation parameters for the IIDX lighting system.
//!
//! These are the two Oklch rainbows you requested as defaults:
//! - [`TWELVE_BIT_OKLCH_RAINBOW`] — evenly-spaced 48-color rainbow (default)
//! - [`TWELVE_BIT_OKLCH_RAINBOW_WEIGHTED`] — 42-color variant with more emphasis on certain regions

use lighting_controller as lc;
use rgb::RGB8;

/// Default 48-color Oklch rainbow — smooth, evenly distributed hues.
#[rustfmt::skip]
pub const TWELVE_BIT_OKLCH_RAINBOW: &[RGB8] = &[
    RGB8 { r: 137, g: 019, b: 120 },
    RGB8 { r: 137, g: 019, b: 120 },
    RGB8 { r: 147, g: 026, b: 111 },
    RGB8 { r: 156, g: 034, b: 102 },
    RGB8 { r: 164, g: 043, b: 094 },
    RGB8 { r: 170, g: 051, b: 085 },
    RGB8 { r: 179, g: 064, b: 088 },
    RGB8 { r: 187, g: 077, b: 092 },
    RGB8 { r: 196, g: 089, b: 096 },
    RGB8 { r: 203, g: 101, b: 101 },
    RGB8 { r: 214, g: 112, b: 093 },
    RGB8 { r: 224, g: 124, b: 084 },
    RGB8 { r: 231, g: 137, b: 075 },
    RGB8 { r: 236, g: 151, b: 066 },
    RGB8 { r: 242, g: 166, b: 047 },
    RGB8 { r: 245, g: 182, b: 022 },
    RGB8 { r: 243, g: 200, b: 000 },
    RGB8 { r: 236, g: 219, b: 000 },
    RGB8 { r: 217, g: 220, b: 028 },
    RGB8 { r: 196, g: 220, b: 049 },
    RGB8 { r: 174, g: 220, b: 067 },
    RGB8 { r: 151, g: 219, b: 083 },
    RGB8 { r: 133, g: 220, b: 097 },
    RGB8 { r: 115, g: 221, b: 111 },
    RGB8 { r: 094, g: 221, b: 124 },
    RGB8 { r: 068, g: 221, b: 136 },
    RGB8 { r: 044, g: 218, b: 153 },
    RGB8 { r: 023, g: 213, b: 166 },
    RGB8 { r: 017, g: 208, b: 177 },
    RGB8 { r: 032, g: 203, b: 186 },
    RGB8 { r: 013, g: 199, b: 191 },
    RGB8 { r: 000, g: 195, b: 195 },
    RGB8 { r: 000, g: 190, b: 199 },
    RGB8 { r: 000, g: 185, b: 202 },
    RGB8 { r: 000, g: 177, b: 203 },
    RGB8 { r: 000, g: 169, b: 204 },
    RGB8 { r: 000, g: 161, b: 204 },
    RGB8 { r: 000, g: 152, b: 203 },
    RGB8 { r: 013, g: 140, b: 201 },
    RGB8 { r: 028, g: 127, b: 197 },
    RGB8 { r: 040, g: 115, b: 192 },
    RGB8 { r: 050, g: 101, b: 186 },
    RGB8 { r: 069, g: 089, b: 182 },
    RGB8 { r: 082, g: 076, b: 175 },
    RGB8 { r: 093, g: 064, b: 165 },
    RGB8 { r: 101, g: 050, b: 152 },
    RGB8 { r: 111, g: 044, b: 147 },
    RGB8 { r: 120, g: 037, b: 139 },
    RGB8 { r: 129, g: 029, b: 130 },
];

/// 42-color Oklch rainbow — weighted toward certain color regions for visual punch.
#[rustfmt::skip]
pub const TWELVE_BIT_OKLCH_RAINBOW_WEIGHTED: &[RGB8] = &[
    RGB8 { r: 137, g: 019, b: 120 },
    RGB8 { r: 137, g: 019, b: 120 },
    RGB8 { r: 147, g: 026, b: 111 },
    RGB8 { r: 156, g: 034, b: 102 },
    RGB8 { r: 164, g: 043, b: 094 },
    RGB8 { r: 170, g: 051, b: 085 },
    RGB8 { r: 179, g: 064, b: 088 },
    RGB8 { r: 187, g: 077, b: 092 },
    RGB8 { r: 196, g: 089, b: 096 },
    RGB8 { r: 203, g: 101, b: 101 },
    RGB8 { r: 212, g: 110, b: 095 },
    RGB8 { r: 220, g: 119, b: 088 },
    RGB8 { r: 227, g: 129, b: 080 },
    RGB8 { r: 232, g: 140, b: 073 },
    RGB8 { r: 236, g: 151, b: 066 },
    RGB8 { r: 241, g: 161, b: 054 },
    RGB8 { r: 243, g: 171, b: 040 },
    RGB8 { r: 245, g: 182, b: 022 },
    RGB8 { r: 244, g: 194, b: 000 },
    RGB8 { r: 241, g: 207, b: 000 },
    RGB8 { r: 236, g: 219, b: 000 },
    RGB8 { r: 223, g: 220, b: 019 },
    RGB8 { r: 210, g: 220, b: 036 },
    RGB8 { r: 196, g: 220, b: 049 },
    RGB8 { r: 181, g: 220, b: 061 },
    RGB8 { r: 167, g: 220, b: 072 },
    RGB8 { r: 151, g: 219, b: 083 },
    RGB8 { r: 137, g: 220, b: 094 },
    RGB8 { r: 122, g: 221, b: 106 },
    RGB8 { r: 106, g: 221, b: 116 },
    RGB8 { r: 089, g: 221, b: 126 },
    RGB8 { r: 068, g: 221, b: 136 },
    RGB8 { r: 036, g: 216, b: 158 },
    RGB8 { r: 016, g: 210, b: 174 },
    RGB8 { r: 032, g: 203, b: 186 },
    RGB8 { r: 000, g: 195, b: 195 },
    RGB8 { r: 000, g: 185, b: 202 },
    RGB8 { r: 000, g: 152, b: 203 },
    RGB8 { r: 050, g: 101, b: 186 },
    RGB8 { r: 082, g: 076, b: 175 },
    RGB8 { r: 101, g: 050, b: 152 },
    RGB8 { r: 114, g: 042, b: 145 },
    RGB8 { r: 126, g: 032, b: 134 },
];
// ──────────────────────────────────────────────────────────────────────────────
// Source-of-truth arrays — single source for menu display & animation config
// ──────────────────────────────────────────────────────────────────────────────

/// Actual `&[RGB8]` slices indexed 1:1 with the rainbow names in
/// [`RAINBOW_NAMES`](crate::menu_settings::RAINBOW_NAMES).
pub const RAINBOW_SLICES: &[&[RGB8]] = &[
    TWELVE_BIT_OKLCH_RAINBOW,
    TWELVE_BIT_OKLCH_RAINBOW_WEIGHTED,
    lc::colors::R_RYB,
    lc::colors::R_OGP,
    lc::colors::R_RGB,
    lc::colors::R_BY,
    lc::colors::R_RB,
    lc::colors::R_BLACK,
    lc::colors::R_WHITE,
    lc::colors::R_RED,
    lc::colors::R_ORANGE,
    lc::colors::R_YELLOW,
    lc::colors::R_LIME,
    lc::colors::R_SPRING_GREEN,
    lc::colors::R_CYAN,
    lc::colors::R_DEEP_BLUE,
    lc::colors::R_BLUE,
    lc::colors::R_BLUE_PURPLE,
    lc::colors::R_FUCHSIA,
    lc::colors::R_DARK_PURPLE,
];

// ──────────────────────────────────────────────────────────────────────────────
// Named constants for animation mode/direction raw values
// These match the index positions in the corresponding *_NAMES arrays above
// and serve as a human-readable bridge between flash storage (`u8`) and the
// animation API match statements.
// ──────────────────────────────────────────────────────────────────────────────

pub const BG_ROTATE: u8 = 0;
pub const BG_FOLLOW: u8 = 1;
pub const BG_SOLID: u8 = 2;
pub const BG_SOLID_FADE: u8 = 3;
pub const BG_OFF: u8 = 4;

pub const FG_OFF: u8 = 0;
pub const FG_MARQUEE: u8 = 1;
pub const FG_MARQUEE_FIXED: u8 = 2;
pub const FG_MARQUEE_FADE: u8 = 3;
pub const FG_MARQUEE_FADE_FIXED: u8 = 4;
pub const FG_VU_METER: u8 = 5;

pub const TRIG_OFF: u8 = 0;
pub const TRIG_PULSE: u8 = 1;
pub const TRIG_PULSE_FADE: u8 = 2;
pub const TRIG_PULSE_RAINBOW: u8 = 3;
pub const TRIG_SHOT: u8 = 4;
pub const TRIG_SHOT_FADE: u8 = 5;
pub const TRIG_SHOT_RAINBOW: u8 = 6;
pub const TRIG_FLASH: u8 = 7;
pub const TRIG_FLASH_FADE: u8 = 8;
pub const TRIG_FLASH_RAINBOW: u8 = 9;

pub const DIR_FWD: u8 = 0;
pub const DIR_STOP: u8 = 1;
pub const DIR_REV: u8 = 2;

pub const OFFSET_RANDOM: u8 = 0;
pub const OFFSET_CENTER: u8 = 1;
pub const OFFSET_TOP: u8 = 2;
