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
// Enums for animation mode / direction / offset / rainbow fields
//
// These replace the old `pub const u8` blocks.  Each enum is #[repr(u8)] so it
// maps 1:1 to the flash-stored `u8` value.  `From<u8>` provides a safe fallback
// for stale flash data.  `display_name()` returns the OLED menu label strings.
// ──────────────────────────────────────────────────────────────────────────────

/// Background animation mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BgMode {
    Rotate = 0,
    Follow = 1,
    Solid = 2,
    SolidFade = 3,
    Off = 4,
}

impl From<u8> for BgMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Rotate,
            1 => Self::Follow,
            2 => Self::Solid,
            3 => Self::SolidFade,
            4 => Self::Off,
            _ => Self::Rotate, // safe fallback for unknown flash values
        }
    }
}

impl BgMode {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Rotate => "Rotate",
            Self::Follow => "Follow",
            Self::Solid => "Solid",
            Self::SolidFade => "SFade",
            Self::Off => "Off",
        }
    }
}

/// Foreground animation mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FgMode {
    Off = 0,
    Marquee = 1,
    MarqueeFixed = 2,
    MarqueeFade = 3,
    MarqueeFadeFixed = 4,
    VUMeter = 5,
}

impl From<u8> for FgMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Off,
            1 => Self::Marquee,
            2 => Self::MarqueeFixed,
            3 => Self::MarqueeFade,
            4 => Self::MarqueeFadeFixed,
            5 => Self::VUMeter,
            _ => Self::Off, // safe fallback for unknown flash values
        }
    }
}

impl FgMode {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Marquee => "Marq",
            Self::MarqueeFixed => "MrqFix",
            Self::MarqueeFade => "MrqFad",
            Self::MarqueeFadeFixed => "MrqFxF",
            Self::VUMeter => "VU",
        }
    }
}

/// Trigger animation mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrigMode {
    Off = 0,
    Pulse = 1,
    PulseFade = 2,
    PulseRainbow = 3,
    Shot = 4,
    ShotFade = 5,
    ShotRainbow = 6,
    Flash = 7,
    FlashFade = 8,
    FlashRainbow = 9,
}

impl From<u8> for TrigMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Off,
            1 => Self::Pulse,
            2 => Self::PulseFade,
            3 => Self::PulseRainbow,
            4 => Self::Shot,
            5 => Self::ShotFade,
            6 => Self::ShotRainbow,
            7 => Self::Flash,
            8 => Self::FlashFade,
            9 => Self::FlashRainbow,
            _ => Self::Off, // safe fallback for unknown flash values
        }
    }
}

impl TrigMode {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Pulse => "Pulse",
            Self::PulseFade => "PlsFad",
            Self::PulseRainbow => "PlsRnb",
            Self::Shot => "Shot",
            Self::ShotFade => "ShtFad",
            Self::ShotRainbow => "ShtRnb",
            Self::Flash => "Flash",
            Self::FlashFade => "FlsFad",
            Self::FlashRainbow => "FlsRnb",
        }
    }
}

/// Direction for any animation layer (background, foreground, trigger).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Fwd = 0,
    Stop = 1,
    Rev = 2,
}

impl From<u8> for Direction {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Fwd,
            1 => Self::Stop,
            2 => Self::Rev,
            _ => Self::Fwd, // safe fallback for unknown flash values
        }
    }
}

impl Direction {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Fwd => "Fwd",
            Self::Stop => "Stop",
            Self::Rev => "Rev",
        }
    }
}

/// Trigger starting offset.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrigOffset {
    Random = 0,
    Center = 1,
    Top = 2,
}

impl From<u8> for TrigOffset {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Random,
            1 => Self::Center,
            2 => Self::Top,
            _ => Self::Random, // safe fallback for unknown flash values
        }
    }
}

impl TrigOffset {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Random => "Random",
            Self::Center => "Center",
            Self::Top => "Top",
        }
    }
}

/// Rainbow palette index matching 1:1 with [`RAINBOW_SLICES`] and
/// [`RAINBOW_NAMES`](crate::menu_settings::RAINBOW_NAMES).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rainbow {
    Oklch = 0,
    OkWgt = 1,
    Ryb = 2,
    Ogp = 3,
    Rgb = 4,
    By = 5,
    Rb = 6,
    Black = 7,
    White = 8,
    Red = 9,
    Orange = 10,
    Yellow = 11,
    Lime = 12,
    Spring = 13,
    Cyan = 14,
    DpBlue = 15,
    Blue = 16,
    BlPurp = 17,
    Fuchsia = 18,
    DkPurp = 19,
}

impl From<u8> for Rainbow {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::Oklch,
            1 => Self::OkWgt,
            2 => Self::Ryb,
            3 => Self::Ogp,
            4 => Self::Rgb,
            5 => Self::By,
            6 => Self::Rb,
            7 => Self::Black,
            8 => Self::White,
            9 => Self::Red,
            10 => Self::Orange,
            11 => Self::Yellow,
            12 => Self::Lime,
            13 => Self::Spring,
            14 => Self::Cyan,
            15 => Self::DpBlue,
            16 => Self::Blue,
            17 => Self::BlPurp,
            18 => Self::Fuchsia,
            19 => Self::DkPurp,
            _ => Self::Oklch, // safe fallback for unknown flash values
        }
    }
}

impl Rainbow {
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Oklch => "Oklch",
            Self::OkWgt => "OkWgt",
            Self::Ryb => "RYB",
            Self::Ogp => "OGP",
            Self::Rgb => "RGB",
            Self::By => "BY",
            Self::Rb => "RB",
            Self::Black => "Black",
            Self::White => "White",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Lime => "Lime",
            Self::Spring => "Spring",
            Self::Cyan => "Cyan",
            Self::DpBlue => "DpBlue",
            Self::Blue => "Blue",
            Self::BlPurp => "BlPurp",
            Self::Fuchsia => "Fuchsia",
            Self::DkPurp => "DkPurp",
        }
    }
}
