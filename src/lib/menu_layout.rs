//! Menu layout types, data, and generation macros.
//!
//! Provides the type definitions that describe the menu structure, all static
//! menu level definitions, and the macros used to generate repetitive menus.
//! This module contains no runtime logic — it's pure type definitions and data.

use crate::ButtonCode;
use crate::menu_settings::{SettingKey, ValueKey};

// ── Display layout constants ───────────────────────────────────────

/// Maximum number of menu levels that can be nested on the stack.
pub(crate) const MAX_MENU_DEPTH: usize = 6;

/// All defined USB HID keyboard usage codes (0-231, skipping reserved range 165-223).
pub(crate) static VALID_KEYS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 224, 225, 226, 227, 228, 229, 230, 231,
];

// ── Menu types ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuMode {
    Debug,
    PixelTest,
}

#[derive(Clone, Copy)]
pub enum MenuEvents {
    Press(ButtonCode),
    LongPress(ButtonCode),
    Repeat(ButtonCode),
    Idle,
}

#[derive(Clone, Copy)]
pub(crate) enum Flip {
    NoFlip,
    FlipX,
    FlipY,
    FlipXY,
}

/// A generic editor bound to a specific value type.
#[derive(Clone, Copy)]
pub(crate) enum Editor {
    IntRange {
        value: u32,
        step: u32,
        min: u32,
        max: u32,
        divisor: u32,
        unit: &'static str,
    },
    OptionSelect {
        labels: &'static [&'static str],
        current: usize,
    },
}

/// Describes what to write back when an [`Editor`] is committed.
#[derive(Clone, Copy)]
pub(crate) enum Commit {
    Setting(SettingKey),
}

/// What happens when a menu option is activated.
#[derive(Clone, Copy)]
pub(crate) enum MenuAction {
    OpenSubmenu(&'static MenuLevel),
    GoBack,
    ShowDebugScreen,
    ShowPixelTest,
    EditValue(ValueKey),
    EditKeyBinding(ButtonCode),
    OpenWikiEdit(usize),
    SaveAndReboot,
    Discard,
    ResetDefaults,
    PerformReset,
    Reboot,
    ShowCustom,
    ReturnToCustom,
    ResetField(usize),
}

/// One option in a menu screen — a label and the action it triggers.
#[derive(Clone, Copy)]
pub(crate) struct MenuOption {
    pub(crate) label: &'static str,
    pub(crate) action: MenuAction,
}

/// One menu screen: a title and its list of options.
#[derive(Clone, Copy)]
pub(crate) struct MenuLevel {
    pub(crate) title: &'static str,
    pub(crate) options: &'static [MenuOption],
}

/// An entry on the navigation stack — keeps the position within one level.
#[derive(Clone, Copy)]
pub(crate) struct StackItem {
    pub(crate) level: &'static MenuLevel,
    pub(crate) cursor: usize,
}

/// Bounds, step size, and display metadata for a single setting type.
pub(crate) struct SettingMeta {
    pub(crate) step: u32,
    pub(crate) min: u32,
    pub(crate) max: u32,
    pub(crate) divisor: u32,
    pub(crate) unit: &'static str,
}

/// Which of the two choices in a `Prompt` is currently selected.
#[derive(Clone, Copy)]
pub(crate) enum PromptSide {
    First,
    Second,
}

/// One option in a `Prompt` — an action and its display label.
#[derive(Clone, Copy)]
pub(crate) struct PromptChoice {
    pub(crate) action: MenuAction,
    pub(crate) label: FmtBuf,
}

/// Fixed-size text buffer for building OLED display lines.
/// Avoids heap allocation in the no_std environment.
#[derive(Clone, Copy)]
pub(crate) struct FmtBuf {
    buf: [u8; crate::BUF_SIZE],
    ptr: usize,
}

impl FmtBuf {
    pub(crate) fn new() -> Self {
        Self {
            buf: [0; crate::BUF_SIZE],
            ptr: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.ptr = 0;
    }

    pub(crate) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.ptr]).unwrap_or("")
    }
}

impl core::fmt::Write for FmtBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let remaining = &mut self.buf[self.ptr..];
        let n = s.as_bytes().len().min(remaining.len());
        remaining[..n].copy_from_slice(&s.as_bytes()[..n]);
        self.ptr += n;
        Ok(())
    }
}

// ── Helper functions ──────────────────────────────────────────────

/// Increment or decrement `val` by `step`, clamping to [`min`, `max`].
pub(crate) fn clamp_step(val: u32, step: u32, min: u32, max: u32, up: bool) -> u32 {
    if up {
        val.saturating_add(step).clamp(min, max)
    } else {
        val.saturating_sub(step).clamp(min, max)
    }
}

/// Determine the flip orientation for a wiki direction arrow.
pub(crate) fn flip_for(code: ButtonCode) -> Flip {
    match code {
        ButtonCode::P1Positive => Flip::NoFlip,
        ButtonCode::P1Negative => Flip::FlipY,
        ButtonCode::P2Positive => Flip::FlipX,
        ButtonCode::P2Negative => Flip::FlipXY,
        _ => Flip::NoFlip,
    }
}

/// Convert a USB key code to its index in [`VALID_KEYS`].
pub(crate) fn key_index(key: u8) -> usize {
    VALID_KEYS.iter().position(|&k| k == key).unwrap_or(0)
}

/// Returns which item index (or `None` for `"--"`) appears on each of the 3
/// visible lines given a total item count and cursor position.
pub(crate) fn option_line_indices(total: usize, cursor: usize) -> [(usize, Option<usize>); 3] {
    match total {
        0 => [(1, None), (2, None), (3, None)],
        1 => [(1, None), (2, Some(0)), (3, None)],
        2 => {
            if cursor == 0 {
                [(1, None), (2, Some(0)), (3, Some(1))]
            } else {
                [(1, Some(0)), (2, Some(1)), (3, None)]
            }
        }
        _ => [
            (1, if cursor == 0 { None } else { Some(cursor - 1) }),
            (2, Some(cursor)),
            (
                3,
                if cursor == total - 1 {
                    None
                } else {
                    Some(cursor + 1)
                },
            ),
        ],
    }
}

// ── Menu generation macros ────────────────────────────────────────

/// Generate lighting submenus for a zone (BG, FG, or Trig).
/// Produces three `static MenuLevel`s: one for All (writes to both players),
/// one for P1, one for P2.
macro_rules! lighting_submenu_zone {
    ($all_name:ident, $p1_name:ident, $p2_name:ident,
     $title:expr,
     [$([$label:expr, $all_vk:ident, $player_vk:ident]),+ $(,)?]) => {
        static $all_name: MenuLevel = MenuLevel {
            title: $title,
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$all_vk),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
        static $p1_name: MenuLevel = MenuLevel {
            title: concat!("P1 ", $title),
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$player_vk(0)),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
        static $p2_name: MenuLevel = MenuLevel {
            title: concat!("P2 ", $title),
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::$player_vk(1)),
                }),+,
                MenuOption { label: "Back", action: MenuAction::GoBack },
            ],
        };
    };
}

lighting_submenu_zone!(
    BG_ALL_MENU,
    BG_P1_MENU,
    BG_P2_MENU,
    "BG",
    [
        ["Mode", AllBgMode, PlayerBgMode],
        ["Rainb", AllBgRainbow, PlayerBgRainbow],
        ["Spd", AllBgSpd, PlayerBgSpd],
        ["Subd", AllBgSubdiv, PlayerBgSubdiv],
    ]
);

lighting_submenu_zone!(
    FG_ALL_MENU,
    FG_P1_MENU,
    FG_P2_MENU,
    "FG",
    [
        ["Mode", AllFgMode, PlayerFgMode],
        ["Rainb", AllFgRainbow, PlayerFgRainbow],
        ["Spd", AllFgSpd, PlayerFgSpd],
        ["Subd", AllFgSubdiv, PlayerFgSubdiv],
        ["Step", AllFgStep, PlayerFgStep],
        ["Size", AllFgSize, PlayerFgSize],
    ]
);

lighting_submenu_zone!(
    TRIG_ALL_MENU,
    TRIG_P1_MENU,
    TRIG_P2_MENU,
    "Trig",
    [
        ["Mode", AllTrigMode, PlayerTrigMode],
        ["Rainb", AllTrigRainbow, PlayerTrigRainbow],
        ["Dir", AllTrigDir, PlayerTrigDir],
        ["Offset", AllTrigOffset, PlayerTrigOffset],
        ["Cycle", AllTrigDur, PlayerTrigDur],
        ["FdIn", AllTrigFdIn, PlayerTrigFdIn],
        ["FdOut", AllTrigFdOut, PlayerTrigFdOut],
        ["Size", AllTrigSize, PlayerTrigSize],
    ]
);

macro_rules! define_button_menus {
    ($(($code:ident, $label:expr)),* $(,)?) => {
        static DEBOUNCE_MENU: MenuLevel = MenuLevel {
            title: "Debounce",
            options: &[
                MenuOption {
                    label: "All",
                    action: MenuAction::EditValue(ValueKey::AllButtonDebounce),
                },
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditValue(ValueKey::ButtonDebounce(ButtonCode::$code)),
                }),*,
                MenuOption {
                    label: "Enc1",
                    action: MenuAction::EditValue(ValueKey::EncoderDebounce(0)),
                },
                MenuOption {
                    label: "Enc2",
                    action: MenuAction::EditValue(ValueKey::EncoderDebounce(1)),
                },
                MenuOption {
                    label: "Back",
                    action: MenuAction::GoBack,
                },
            ],
        };

        static KEYBIND_MENU: MenuLevel = MenuLevel {
            title: "Key Bindings",
            options: &[
                $(MenuOption {
                    label: $label,
                    action: MenuAction::EditKeyBinding(ButtonCode::$code),
                }),*,
                MenuOption {
                    label: "Enc1+",
                    action: MenuAction::EditKeyBinding(ButtonCode::P1Positive),
                },
                MenuOption {
                    label: "Enc1-",
                    action: MenuAction::EditKeyBinding(ButtonCode::P1Negative),
                },
                MenuOption {
                    label: "Enc2+",
                    action: MenuAction::EditKeyBinding(ButtonCode::P2Positive),
                },
                MenuOption {
                    label: "Enc2-",
                    action: MenuAction::EditKeyBinding(ButtonCode::P2Negative),
                },
                MenuOption {
                    label: "Back",
                    action: MenuAction::GoBack,
                },
            ],
        };
    };
}

define_button_menus! {
    (P1_1, "P1_1"), (P1_2, "P1_2"), (P1_3, "P1_3"), (P1_4, "P1_4"),
    (P1_5, "P1_5"), (P1_6, "P1_6"), (P1_7, "P1_7"),
    (P1Start, "P1St"), (P1Select, "P1Sl"),
    (P2_1, "P2_1"), (P2_2, "P2_2"), (P2_3, "P2_3"), (P2_4, "P2_4"),
    (P2_5, "P2_5"), (P2_6, "P2_6"), (P2_7, "P2_7"),
    (P2Start, "P2St"), (P2Select, "P2Sl"),
    (Escape, "Esc"),
    (CcUp, "CCUp"), (CcDown, "CCDn"), (CcLeft, "CCLt"), (CcRight, "CCRt"), (CcSelect, "CCSl"),
    (VolumeUp, "VUp"), (VolumeDown, "VDn"), (Mute, "Mute"),
}

// ── Top-level menu definitions ─────────────────────────────────────

pub(crate) static ROOT_MENU: MenuLevel = MenuLevel {
    title: "IIDX Menu",
    options: &[
        MenuOption {
            label: "Lighting",
            action: MenuAction::OpenSubmenu(&LIGHTING_MENU),
        },
        MenuOption {
            label: "Debug",
            action: MenuAction::OpenSubmenu(&DEBUG_MENU),
        },
        MenuOption {
            label: "Settings",
            action: MenuAction::OpenSubmenu(&SETTINGS_MENU),
        },
        MenuOption {
            label: "System",
            action: MenuAction::OpenSubmenu(&SYSTEM_MENU),
        },
    ],
};

pub(crate) static SETTINGS_MENU: MenuLevel = MenuLevel {
    title: "Settings",
    options: &[
        MenuOption {
            label: "Wiki Config",
            action: MenuAction::OpenSubmenu(&ENCODER_SENS_MENU),
        },
        MenuOption {
            label: "Key Bindings",
            action: MenuAction::OpenSubmenu(&KEYBIND_MENU),
        },
        MenuOption {
            label: "Debounce",
            action: MenuAction::OpenSubmenu(&DEBOUNCE_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static DEBUG_MENU: MenuLevel = MenuLevel {
    title: "Debug",
    options: &[
        MenuOption {
            label: "Debug Screen",
            action: MenuAction::ShowDebugScreen,
        },
        MenuOption {
            label: "Pixel Test",
            action: MenuAction::ShowPixelTest,
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static SYSTEM_MENU: MenuLevel = MenuLevel {
    title: "System",
    options: &[
        MenuOption {
            label: "Save+Reboot",
            action: MenuAction::SaveAndReboot,
        },
        MenuOption {
            label: "Show Custom",
            action: MenuAction::ShowCustom,
        },
        MenuOption {
            label: "ResetDefault",
            action: MenuAction::ResetDefaults,
        },
        MenuOption {
            label: "Reboot",
            action: MenuAction::Reboot,
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static ENCODER_SENS_MENU: MenuLevel = MenuLevel {
    title: "Wiki Config",
    options: &[
        MenuOption {
            label: "P1 Wiki",
            action: MenuAction::OpenWikiEdit(0),
        },
        MenuOption {
            label: "P2 Wiki",
            action: MenuAction::OpenWikiEdit(1),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static LIGHTING_MENU: MenuLevel = MenuLevel {
    title: "Lighting",
    options: &[
        MenuOption {
            label: "Both",
            action: MenuAction::OpenSubmenu(&BOTH_MENU),
        },
        MenuOption {
            label: "P1",
            action: MenuAction::OpenSubmenu(&P1_MENU),
        },
        MenuOption {
            label: "P2",
            action: MenuAction::OpenSubmenu(&P2_MENU),
        },
        MenuOption {
            label: "Global",
            action: MenuAction::OpenSubmenu(&GLOBAL_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static BOTH_MENU: MenuLevel = MenuLevel {
    title: "Both",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_ALL_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_ALL_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_ALL_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static P1_MENU: MenuLevel = MenuLevel {
    title: "P1",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_P1_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_P1_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_P1_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static P2_MENU: MenuLevel = MenuLevel {
    title: "P2",
    options: &[
        MenuOption {
            label: "BG",
            action: MenuAction::OpenSubmenu(&BG_P2_MENU),
        },
        MenuOption {
            label: "FG",
            action: MenuAction::OpenSubmenu(&FG_P2_MENU),
        },
        MenuOption {
            label: "Trig",
            action: MenuAction::OpenSubmenu(&TRIG_P2_MENU),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};

pub(crate) static GLOBAL_MENU: MenuLevel = MenuLevel {
    title: "Global",
    options: &[
        MenuOption {
            label: "Brght",
            action: MenuAction::EditValue(ValueKey::GlobalBrightness),
        },
        MenuOption {
            label: "Back",
            action: MenuAction::GoBack,
        },
    ],
};
