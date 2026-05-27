//! Settings types and operations for the menu system.
//!
//! Defines `SettingKey`, `ValueKey`, `FieldDescriptor`, and all read/write/reset/format
//! operations on persistent settings.  The second `impl MenuHandler` block in this file
//! is merged by the compiler with the main block in `menu_handler.rs`.

use core::fmt::Write;

use defmt::debug;
use display_interface::WriteOnlyDataCommand;
use strum::IntoEnumIterator;

use crate::menu_handler::MenuHandler;
use crate::menu_layout::{Commit, Editor, FmtBuf, SettingMeta};
use crate::{
    BG_MODE_NAMES, ButtonCode, DIR_NAMES, FG_MODE_NAMES, FlashStoragePersistentMemory, NUM_BUTTONS,
    OFFSET_NAMES, Player, RAINBOW_NAMES, TRIG_MODE_NAMES,
};

// ── Setting key enums ─────────────────────────────────────────────

/// Keys whose value can be edited via [`Editor::IntRange`].  Maps 1:1 to
/// [`SettingKey`] entries that have a [`SettingMeta`].
#[derive(Clone, Copy)]
pub(crate) enum ValueKey {
    AllButtonDebounce,
    ButtonDebounce(ButtonCode),
    EncoderDebounce(Player),
    // Lighting
    AllBgMode,
    AllBgRainbow,
    AllBgSpd,
    AllBgSubdiv,
    AllFgMode,
    AllFgRainbow,
    AllFgSpd,
    AllFgSubdiv,
    AllFgStep,
    AllFgSize,
    AllTrigMode,
    AllTrigRainbow,
    AllTrigFdIn,
    AllTrigFdOut,
    AllTrigSize,
    AllTrigDir,
    AllTrigOffset,
    AllTrigDur,
    PlayerBgMode(Player),
    PlayerBgRainbow(Player),
    PlayerBgSpd(Player),
    PlayerBgSubdiv(Player),
    PlayerFgMode(Player),
    PlayerFgRainbow(Player),
    PlayerFgSpd(Player),
    PlayerFgSubdiv(Player),
    PlayerFgStep(Player),
    PlayerFgSize(Player),
    PlayerTrigMode(Player),
    PlayerTrigRainbow(Player),
    PlayerTrigFdIn(Player),
    PlayerTrigFdOut(Player),
    PlayerTrigSize(Player),
    PlayerTrigDir(Player),
    PlayerTrigOffset(Player),
    PlayerTrigDur(Player),
    GlobalBrightness,
}

/// Keys identifying which setting a menu option edits.  Each field in the
/// persistent config has both an `All*` variant (applies to both players)
/// and a `Player*(Player)` variant (applies to one player).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum SettingKey {
    AllButtonDebounce,
    ButtonDebounce(ButtonCode),
    EncoderDebounce(Player),
    EncoderStepThreshold(Player),
    EncoderMoveTimeout(Player),
    // Lighting — "All" variants apply to both players
    AllBgMode,
    AllBgRainbow,
    AllBgSpd,
    AllBgSubdiv,
    AllFgMode,
    AllFgRainbow,
    AllFgSpd,
    AllFgSubdiv,
    AllFgStep,
    AllFgSize,
    AllTrigMode,
    AllTrigRainbow,
    AllTrigFdIn,
    AllTrigFdOut,
    AllTrigSize,
    AllTrigDir,
    AllTrigOffset,
    AllTrigDur,
    // Lighting — per-player
    PlayerBgMode(Player),
    PlayerBgRainbow(Player),
    PlayerBgSpd(Player),
    PlayerBgSubdiv(Player),
    PlayerFgMode(Player),
    PlayerFgRainbow(Player),
    PlayerFgSpd(Player),
    PlayerFgSubdiv(Player),
    PlayerFgStep(Player),
    PlayerFgSize(Player),
    PlayerTrigMode(Player),
    PlayerTrigRainbow(Player),
    PlayerTrigFdIn(Player),
    PlayerTrigFdOut(Player),
    PlayerTrigSize(Player),
    PlayerTrigDir(Player),
    PlayerTrigOffset(Player),
    PlayerTrigDur(Player),
    GlobalBrightness,
}

impl From<ValueKey> for SettingKey {
    fn from(vk: ValueKey) -> Self {
        match vk {
            ValueKey::AllButtonDebounce => SettingKey::AllButtonDebounce,
            ValueKey::ButtonDebounce(c) => SettingKey::ButtonDebounce(c),
            ValueKey::EncoderDebounce(i) => SettingKey::EncoderDebounce(i),
            ValueKey::AllBgMode => SettingKey::AllBgMode,
            ValueKey::AllBgRainbow => SettingKey::AllBgRainbow,
            ValueKey::AllBgSpd => SettingKey::AllBgSpd,
            ValueKey::AllBgSubdiv => SettingKey::AllBgSubdiv,
            ValueKey::AllFgMode => SettingKey::AllFgMode,
            ValueKey::AllFgRainbow => SettingKey::AllFgRainbow,
            ValueKey::AllFgSpd => SettingKey::AllFgSpd,
            ValueKey::AllFgSubdiv => SettingKey::AllFgSubdiv,
            ValueKey::AllFgStep => SettingKey::AllFgStep,
            ValueKey::AllFgSize => SettingKey::AllFgSize,
            ValueKey::AllTrigMode => SettingKey::AllTrigMode,
            ValueKey::AllTrigRainbow => SettingKey::AllTrigRainbow,
            ValueKey::AllTrigFdIn => SettingKey::AllTrigFdIn,
            ValueKey::AllTrigFdOut => SettingKey::AllTrigFdOut,
            ValueKey::AllTrigSize => SettingKey::AllTrigSize,
            ValueKey::AllTrigDir => SettingKey::AllTrigDir,
            ValueKey::AllTrigOffset => SettingKey::AllTrigOffset,
            ValueKey::AllTrigDur => SettingKey::AllTrigDur,
            ValueKey::PlayerBgMode(p) => SettingKey::PlayerBgMode(p),
            ValueKey::PlayerBgRainbow(p) => SettingKey::PlayerBgRainbow(p),
            ValueKey::PlayerBgSpd(p) => SettingKey::PlayerBgSpd(p),
            ValueKey::PlayerBgSubdiv(p) => SettingKey::PlayerBgSubdiv(p),
            ValueKey::PlayerFgMode(p) => SettingKey::PlayerFgMode(p),
            ValueKey::PlayerFgRainbow(p) => SettingKey::PlayerFgRainbow(p),
            ValueKey::PlayerFgSpd(p) => SettingKey::PlayerFgSpd(p),
            ValueKey::PlayerFgSubdiv(p) => SettingKey::PlayerFgSubdiv(p),
            ValueKey::PlayerFgStep(p) => SettingKey::PlayerFgStep(p),
            ValueKey::PlayerFgSize(p) => SettingKey::PlayerFgSize(p),
            ValueKey::PlayerTrigMode(p) => SettingKey::PlayerTrigMode(p),
            ValueKey::PlayerTrigRainbow(p) => SettingKey::PlayerTrigRainbow(p),
            ValueKey::PlayerTrigFdIn(p) => SettingKey::PlayerTrigFdIn(p),
            ValueKey::PlayerTrigFdOut(p) => SettingKey::PlayerTrigFdOut(p),
            ValueKey::PlayerTrigSize(p) => SettingKey::PlayerTrigSize(p),
            ValueKey::PlayerTrigDir(p) => SettingKey::PlayerTrigDir(p),
            ValueKey::PlayerTrigOffset(p) => SettingKey::PlayerTrigOffset(p),
            ValueKey::PlayerTrigDur(p) => SettingKey::PlayerTrigDur(p),
            ValueKey::GlobalBrightness => SettingKey::GlobalBrightness,
        }
    }
}

macro_rules! lighting_meta_block {
    ($key:ident, [$([$all_vk:ident, $player_vk:ident, $step:expr, $min:expr, $max:expr, $divisor:expr, $unit:expr]),+ $(,)?]) => {
        {
            match $key {
                $(
                    Self::$all_vk | Self::$player_vk(_) => {
                        return SettingMeta {
                            step: $step, min: $min, max: $max,
                            divisor: $divisor, unit: $unit,
                        };
                    }
                )+
                _ => {}
            }
        }
    }
}

impl SettingKey {
    /// Return the adjustment metadata for numeric (IntRange) settings.
    pub(crate) fn meta(&self) -> SettingMeta {
        lighting_meta_block!(
            self,
            [
                [AllBgSpd, PlayerBgSpd, 1, 5, 600, 10, "s"],
                [AllBgSubdiv, PlayerBgSubdiv, 1, 1, 10, 1, ""],
                [AllFgSpd, PlayerFgSpd, 1, 5, 600, 10, "s"],
                [AllFgSubdiv, PlayerFgSubdiv, 1, 1, 10, 1, ""],
                [AllFgStep, PlayerFgStep, 1, 1, 200, 10, "s"],
                [AllFgSize, PlayerFgSize, 1, 1, 10, 1, "px"],
                [AllTrigFdIn, PlayerTrigFdIn, 50, 50, 2000, 1, "ms"],
                [AllTrigFdOut, PlayerTrigFdOut, 50, 50, 5000, 1, "ms"],
                [AllTrigSize, PlayerTrigSize, 1, 1, 10, 1, "px"],
                [AllTrigDur, PlayerTrigDur, 1, 1, 60, 1, "s"],
            ]
        );
        match self {
            Self::AllButtonDebounce | Self::ButtonDebounce(_) | Self::EncoderDebounce(_) => {
                SettingMeta {
                    step: 1_000,
                    min: 0,
                    max: 1_000_000,
                    divisor: 1_000,
                    unit: "ms",
                }
            }
            Self::EncoderStepThreshold(_) => SettingMeta {
                step: 1,
                min: 0,
                max: 100,
                divisor: 1,
                unit: "Steps",
            },
            Self::EncoderMoveTimeout(_) => SettingMeta {
                step: 5_000,
                min: 0,
                max: 10_000_000,
                divisor: 1_000,
                unit: "ms",
            },
            Self::GlobalBrightness => SettingMeta {
                step: 5,
                min: 0,
                max: 255,
                divisor: 1,
                unit: "",
            },
            // OptionSelect keys (modes, rainbows) — never read via meta()
            _ => SettingMeta {
                step: 1,
                min: 0,
                max: 1,
                divisor: 1,
                unit: "",
            },
        }
    }
}

/// Describes a single field in [`FlashStoragePersistentMemory`] that differs
/// from its factory-default value.
#[derive(Clone, Copy)]
pub(crate) enum FieldDescriptor {
    ButtonDebounce(usize),
    ButtonKey(usize),
    EncoderKeyUp(Player),
    EncoderKeyDown(Player),
    EncoderDebounce(Player),
    EncoderStepThreshold(Player),
    EncoderMoveTimeout(Player),
    // Lighting
    PlayerBgMode(Player),
    PlayerBgRainbow(Player),
    PlayerBgSpd(Player),
    PlayerBgSubdiv(Player),
    PlayerFgMode(Player),
    PlayerFgRainbow(Player),
    PlayerFgSpd(Player),
    PlayerFgSubdiv(Player),
    PlayerFgStep(Player),
    PlayerFgSize(Player),
    PlayerTrigMode(Player),
    PlayerTrigRainbow(Player),
    PlayerTrigFdIn(Player),
    PlayerTrigFdOut(Player),
    PlayerTrigSize(Player),
    PlayerTrigDir(Player),
    PlayerTrigOffset(Player),
    PlayerTrigDur(Player),
    PlayerBrightness(Player),
}

impl FieldDescriptor {
    /// Section title shown in the "Show Custom" screen for this field group.
    pub(crate) fn section_title(&self) -> &'static str {
        match self {
            Self::ButtonDebounce(_) => "Button Debounce",
            Self::ButtonKey(_) => "Button Keys",
            Self::EncoderKeyUp(_) | Self::EncoderKeyDown(_) => "Encoder Keys",
            Self::EncoderDebounce(_) => "Encoder Debounce",
            Self::EncoderStepThreshold(_) => "Encoder Threshold",
            Self::EncoderMoveTimeout(_) => "Encoder Timeout",
            Self::PlayerBgMode(p) => match p {
                Player::P1 => "P1 BG",
                Player::P2 => "P2 BG",
            },
            Self::PlayerFgMode(p) => match p {
                Player::P1 => "P1 FG",
                Player::P2 => "P2 FG",
            },
            Self::PlayerTrigMode(p) => match p {
                Player::P1 => "P1 Trig",
                Player::P2 => "P2 Trig",
            },
            Self::PlayerBrightness(p) => match p {
                Player::P1 => "P1 Bright",
                Player::P2 => "P2 Bright",
            },
            Self::PlayerBgRainbow(p) => match p {
                Player::P1 => "P1 BgRnb",
                Player::P2 => "P2 BgRnb",
            },
            Self::PlayerBgSpd(p) => match p {
                Player::P1 => "P1 BgSpd",
                Player::P2 => "P2 BgSpd",
            },
            Self::PlayerBgSubdiv(p) => match p {
                Player::P1 => "P1 BgSub",
                Player::P2 => "P2 BgSub",
            },
            Self::PlayerFgSubdiv(p) => match p {
                Player::P1 => "P1 FgSub",
                Player::P2 => "P2 FgSub",
            },
            Self::PlayerFgRainbow(p) => match p {
                Player::P1 => "P1 FgRnb",
                Player::P2 => "P2 FgRnb",
            },
            Self::PlayerFgSpd(p) => match p {
                Player::P1 => "P1 FgSpd",
                Player::P2 => "P2 FgSpd",
            },
            Self::PlayerFgStep(p) => match p {
                Player::P1 => "P1 FgStp",
                Player::P2 => "P2 FgStp",
            },
            Self::PlayerFgSize(p) => match p {
                Player::P1 => "P1 FgSz",
                Player::P2 => "P2 FgSz",
            },
            Self::PlayerTrigRainbow(p) => match p {
                Player::P1 => "P1 TrRnb",
                Player::P2 => "P2 TrRnb",
            },
            Self::PlayerTrigFdIn(p) => match p {
                Player::P1 => "P1 FdIn",
                Player::P2 => "P2 FdIn",
            },
            Self::PlayerTrigFdOut(p) => match p {
                Player::P1 => "P1 FdOut",
                Player::P2 => "P2 FdOut",
            },
            Self::PlayerTrigSize(p) => match p {
                Player::P1 => "P1 TrSz",
                Player::P2 => "P2 TrSz",
            },
            Self::PlayerTrigDir(p) => match p {
                Player::P1 => "P1 TrDr",
                Player::P2 => "P2 TrDr",
            },
            Self::PlayerTrigOffset(p) => match p {
                Player::P1 => "P1 TrOf",
                Player::P2 => "P2 TrOf",
            },
            Self::PlayerTrigDur(p) => match p {
                Player::P1 => "P1 TrDu",
                Player::P2 => "P2 TrDu",
            },
        }
    }
}

// ── Lighting field macros ─────────────────────────────────────────

macro_rules! lighting_read_block {
    ($self:ident, $key:ident, [$([$all_vk:ident, $player_vk:ident, $field:ident]),+ $(,)?]) => {
        {
            match $key {
                $(
                    SettingKey::$all_vk => {
                        return $self.settings.lighting.players[0].$field as u32;
                    }
                    SettingKey::$player_vk(p) => {
                        return $self.settings.lighting.players[p as usize].$field as u32;
                    }
                )+
                _ => {}
            }
        }
    }
}

macro_rules! lighting_write_block {
    ($self:ident, $key:ident, $value:ident, [$([$all_vk:ident, $player_vk:ident, $field:ident, $ty:ty]),+ $(,)?]) => {
        {
            match $key {
                $(
                    SettingKey::$all_vk => {
                        for p in $crate::Player::iter() {
                            $self.settings.lighting.players[p as usize].$field = $value as $ty;
                        }
                        return;
                    }
                    SettingKey::$player_vk(p) => {
                        $self.settings.lighting.players[p as usize].$field = $value as $ty;
                        return;
                    }
                )+
                _ => {}
            }
        }
    }
}

macro_rules! lighting_for_each_check {
    ($p:ident, $settings:expr, $defaults:expr, $f:ident, $count:ident,
     [$([$player_vk:ident, $field:ident]),+ $(,)?]) => {
        $(
            {
                let cur = $settings.lighting.players[$p as usize].$field as u64;
                let def = $defaults.lighting.players[$p as usize].$field as u64;
                if cur != def {
                    if !$f(FieldDescriptor::$player_vk($p), cur, def) {
                        return $count + 1;
                    }
                    $count += 1;
                }
            }
        )+
    }
}

macro_rules! lighting_reset_checks {
    ($self:ident, $p:ident, $idx:ident, $defaults:ident, $target_idx:ident, [$([$player_vk:ident, $field:ident]),+ $(,)?]) => {
        $(
            if $self.settings.lighting.players[$p as usize].$field
                != $defaults.lighting.players[$p as usize].$field
            {
                if $idx == $target_idx {
                    let key = SettingKey::$player_vk($p);
                    $self.write_setting(key, $defaults.lighting.players[$p as usize].$field as u32);
                    return;
                }
                $idx += 1;
            }
        )+
    }
}

// ── Standalone functions ──────────────────────────────────────────

/// Walk every field in [`FlashStoragePersistentMemory`] in a fixed order,
/// calling `f(field, current_value, default_value)` for each field that
/// differs from its factory default.
pub(crate) fn for_each_changed_field(
    settings: &FlashStoragePersistentMemory,
    defaults: &FlashStoragePersistentMemory,
    mut f: impl FnMut(FieldDescriptor, u64, u64) -> bool,
) -> usize {
    let mut count = 0;

    macro_rules! check {
        ($current:expr, $default:expr, $desc:expr) => {
            let cur = $current as u64;
            let def = $default as u64;
            if cur != def {
                if !f($desc, cur, def) {
                    return count + 1;
                }
                count += 1;
            }
        };
    }

    for b in 0..NUM_BUTTONS {
        check!(
            settings.buttons[b].debounce_ticks,
            defaults.buttons[b].debounce_ticks,
            FieldDescriptor::ButtonDebounce(b)
        );
    }
    for b in 0..NUM_BUTTONS {
        check!(
            settings.buttons[b].key,
            defaults.buttons[b].key,
            FieldDescriptor::ButtonKey(b)
        );
    }
    for e in Player::iter() {
        check!(
            settings.encoders[e as usize].key_up,
            defaults.encoders[e as usize].key_up,
            FieldDescriptor::EncoderKeyUp(e)
        );
        check!(
            settings.encoders[e as usize].key_down,
            defaults.encoders[e as usize].key_down,
            FieldDescriptor::EncoderKeyDown(e)
        );
        check!(
            settings.encoders[e as usize].debounce_ticks,
            defaults.encoders[e as usize].debounce_ticks,
            FieldDescriptor::EncoderDebounce(e)
        );
        check!(
            settings.encoders[e as usize].step_threshold,
            defaults.encoders[e as usize].step_threshold,
            FieldDescriptor::EncoderStepThreshold(e)
        );
        check!(
            settings.encoders[e as usize].move_timeout_ticks,
            defaults.encoders[e as usize].move_timeout_ticks,
            FieldDescriptor::EncoderMoveTimeout(e)
        );
    }

    // Lighting per-player fields
    for p in Player::iter() {
        lighting_for_each_check!(
            p,
            settings,
            defaults,
            f,
            count,
            [
                [PlayerBgMode, bg_mode],
                [PlayerBgRainbow, bg_rainbow],
                [PlayerBgSpd, bg_speed_ds],
                [PlayerBgSubdiv, bg_subdivisions],
                [PlayerFgMode, fg_mode],
                [PlayerFgRainbow, fg_rainbow],
                [PlayerFgSpd, fg_speed_ds],
                [PlayerFgSubdiv, fg_subdivisions],
                [PlayerFgStep, fg_step_ds],
                [PlayerFgSize, fg_px_per_group],
                [PlayerTrigMode, trig_mode],
                [PlayerTrigRainbow, trig_rainbow],
                [PlayerTrigFdIn, trig_fade_in_ms],
                [PlayerTrigFdOut, trig_fade_out_ms],
                [PlayerTrigSize, trig_width],
                [PlayerTrigDir, trig_dir],
                [PlayerTrigOffset, trig_offset],
                [PlayerTrigDur, trig_dur_s],
            ]
        );
    }
    // Global brightness
    check!(
        settings.lighting.brightness,
        defaults.lighting.brightness,
        FieldDescriptor::PlayerBrightness(Player::P1)
    );

    count
}

/// Build an [`Editor`] + [`Commit`] pair for a [`ValueKey`] by reading the
/// current value from `settings`.
pub(crate) fn build_editor(
    settings: &FlashStoragePersistentMemory,
    vk: ValueKey,
) -> (Editor, Commit) {
    let key: SettingKey = vk.into();
    match key {
        // OptionSelect keys
        SettingKey::AllBgMode | SettingKey::PlayerBgMode(_) => {
            let current = match key {
                SettingKey::AllBgMode => settings.lighting.players[0].bg_mode as usize,
                SettingKey::PlayerBgMode(p) => {
                    settings.lighting.players[p as usize].bg_mode as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: BG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllBgRainbow | SettingKey::PlayerBgRainbow(_) => {
            let current = match key {
                SettingKey::AllBgRainbow => settings.lighting.players[0].bg_rainbow as usize,
                SettingKey::PlayerBgRainbow(p) => {
                    settings.lighting.players[p as usize].bg_rainbow as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllFgMode | SettingKey::PlayerFgMode(_) => {
            let current = match key {
                SettingKey::AllFgMode => settings.lighting.players[0].fg_mode as usize,
                SettingKey::PlayerFgMode(p) => {
                    settings.lighting.players[p as usize].fg_mode as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: FG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllFgRainbow | SettingKey::PlayerFgRainbow(_) => {
            let current = match key {
                SettingKey::AllFgRainbow => settings.lighting.players[0].fg_rainbow as usize,
                SettingKey::PlayerFgRainbow(p) => {
                    settings.lighting.players[p as usize].fg_rainbow as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigMode | SettingKey::PlayerTrigMode(_) => {
            let current = match key {
                SettingKey::AllTrigMode => settings.lighting.players[0].trig_mode as usize,
                SettingKey::PlayerTrigMode(p) => {
                    settings.lighting.players[p as usize].trig_mode as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: TRIG_MODE_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigRainbow | SettingKey::PlayerTrigRainbow(_) => {
            let current = match key {
                SettingKey::AllTrigRainbow => settings.lighting.players[0].trig_rainbow as usize,
                SettingKey::PlayerTrigRainbow(p) => {
                    settings.lighting.players[p as usize].trig_rainbow as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: RAINBOW_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigDir | SettingKey::PlayerTrigDir(_) => {
            let current = match key {
                SettingKey::AllTrigDir => settings.lighting.players[0].trig_dir as usize,
                SettingKey::PlayerTrigDir(p) => {
                    settings.lighting.players[p as usize].trig_dir as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: DIR_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        SettingKey::AllTrigOffset | SettingKey::PlayerTrigOffset(_) => {
            let current = match key {
                SettingKey::AllTrigOffset => settings.lighting.players[0].trig_offset as usize,
                SettingKey::PlayerTrigOffset(p) => {
                    settings.lighting.players[p as usize].trig_offset as usize
                }
                _ => 0,
            };
            (
                Editor::OptionSelect {
                    labels: OFFSET_NAMES,
                    current,
                },
                Commit::Setting(key),
            )
        }
        // IntRange keys — use meta()
        _ => {
            let meta = key.meta();
            let value = match key {
                SettingKey::AllButtonDebounce => settings.buttons[0].debounce_ticks as u32,
                SettingKey::ButtonDebounce(code) => {
                    settings.buttons[code as usize].debounce_ticks as u32
                }
                SettingKey::EncoderDebounce(p) => {
                    settings.encoders[p as usize].debounce_ticks as u32
                }
                SettingKey::EncoderStepThreshold(p) => {
                    settings.encoders[p as usize].step_threshold as u32
                }
                SettingKey::EncoderMoveTimeout(p) => {
                    settings.encoders[p as usize].move_timeout_ticks as u32
                }
                SettingKey::AllBgSpd => settings.lighting.players[0].bg_speed_ds as u32,
                SettingKey::PlayerBgSpd(p) => {
                    settings.lighting.players[p as usize].bg_speed_ds as u32
                }
                SettingKey::AllBgSubdiv => settings.lighting.players[0].bg_subdivisions as u32,
                SettingKey::PlayerBgSubdiv(p) => {
                    settings.lighting.players[p as usize].bg_subdivisions as u32
                }
                SettingKey::AllFgSubdiv => settings.lighting.players[0].fg_subdivisions as u32,
                SettingKey::PlayerFgSubdiv(p) => {
                    settings.lighting.players[p as usize].fg_subdivisions as u32
                }
                SettingKey::AllFgSpd => settings.lighting.players[0].fg_speed_ds as u32,
                SettingKey::PlayerFgSpd(p) => {
                    settings.lighting.players[p as usize].fg_speed_ds as u32
                }
                SettingKey::AllFgStep => settings.lighting.players[0].fg_step_ds as u32,
                SettingKey::PlayerFgStep(p) => {
                    settings.lighting.players[p as usize].fg_step_ds as u32
                }
                SettingKey::AllFgSize => settings.lighting.players[0].fg_px_per_group as u32,
                SettingKey::PlayerFgSize(p) => {
                    settings.lighting.players[p as usize].fg_px_per_group as u32
                }
                SettingKey::AllTrigFdIn => settings.lighting.players[0].trig_fade_in_ms as u32,
                SettingKey::PlayerTrigFdIn(p) => {
                    settings.lighting.players[p as usize].trig_fade_in_ms as u32
                }
                SettingKey::AllTrigFdOut => settings.lighting.players[0].trig_fade_out_ms as u32,
                SettingKey::PlayerTrigFdOut(p) => {
                    settings.lighting.players[p as usize].trig_fade_out_ms as u32
                }
                SettingKey::AllTrigSize => settings.lighting.players[0].trig_width as u32,
                SettingKey::PlayerTrigSize(p) => {
                    settings.lighting.players[p as usize].trig_width as u32
                }
                SettingKey::AllTrigDur => settings.lighting.players[0].trig_dur_s as u32,
                SettingKey::PlayerTrigDur(p) => {
                    settings.lighting.players[p as usize].trig_dur_s as u32
                }
                SettingKey::GlobalBrightness => settings.lighting.brightness as u32,
                _ => 0,
            };
            let editor = Editor::IntRange {
                value,
                step: meta.step,
                min: meta.min,
                max: meta.max,
                divisor: meta.divisor,
                unit: meta.unit,
            };
            (editor, Commit::Setting(key))
        }
    }
}

/// Convert a USB HID key code to a short human-readable name.
pub(crate) fn key_name(key: u8) -> &'static str {
    match key {
        0 => "No Key",
        1 => "ErrRO",
        2 => "POST",
        3 => "Err",
        4 => "A",
        5 => "B",
        6 => "C",
        7 => "D",
        8 => "E",
        9 => "F",
        10 => "G",
        11 => "H",
        12 => "I",
        13 => "J",
        14 => "K",
        15 => "L",
        16 => "M",
        17 => "N",
        18 => "O",
        19 => "P",
        20 => "Q",
        21 => "R",
        22 => "S",
        23 => "T",
        24 => "U",
        25 => "V",
        26 => "W",
        27 => "X",
        28 => "Y",
        29 => "Z",
        30 => "K1",
        31 => "K2",
        32 => "K3",
        33 => "K4",
        34 => "K5",
        35 => "K6",
        36 => "K7",
        37 => "K8",
        38 => "K9",
        39 => "K0",
        40 => "Enter",
        41 => "Esc",
        42 => "BkSpc",
        43 => "Tab",
        44 => "Spc",
        45 => "-_",
        46 => "=+",
        47 => "{",
        48 => "}",
        49 => "\\",
        50 => "#",
        51 => ";",
        52 => "'",
        53 => "Grave",
        54 => ",<",
        55 => ".>",
        56 => "/",
        57 => "Caps",
        58 => "F1",
        59 => "F2",
        60 => "F3",
        61 => "F4",
        62 => "F5",
        63 => "F6",
        64 => "F7",
        65 => "F8",
        66 => "F9",
        67 => "F10",
        68 => "F11",
        69 => "F12",
        70 => "PrtSc",
        71 => "ScrLk",
        72 => "Paus",
        73 => "Ins",
        74 => "Hom",
        75 => "PgUp",
        76 => "Del",
        77 => "End",
        78 => "PgDn",
        79 => "RArr",
        80 => "LArr",
        81 => "DArr",
        82 => "UArr",
        83 => "NLck",
        84 => "Kp/",
        85 => "Kp*",
        86 => "Kp-",
        87 => "Kp+",
        88 => "KEnt",
        89 => "Kp1",
        90 => "Kp2",
        91 => "Kp3",
        92 => "Kp4",
        93 => "Kp5",
        94 => "Kp6",
        95 => "Kp7",
        96 => "Kp8",
        97 => "Kp9",
        98 => "Kp0",
        99 => "Kp.",
        100 => "NUS\\",
        101 => "App",
        102 => "Pow",
        103 => "KpEq",
        104 => "F13",
        105 => "F14",
        106 => "F15",
        107 => "F16",
        108 => "F17",
        109 => "F18",
        110 => "F19",
        111 => "F20",
        112 => "F21",
        113 => "F22",
        114 => "F23",
        115 => "F24",
        116 => "Exe",
        117 => "Help",
        118 => "Men",
        119 => "Sel",
        120 => "Stp",
        121 => "Agn",
        122 => "Und",
        123 => "Cut",
        124 => "Cop",
        125 => "Pst",
        126 => "Fin",
        127 => "Mute",
        128 => "VolUp",
        129 => "VolDn",
        130 => "LCaps",
        131 => "LNumL",
        132 => "LScrL",
        133 => "KpCom",
        134 => "KpEqS",
        153 => "AltEr",
        154 => "SysRq",
        159 => "Sep",
        162 => "ClrAg",
        163 => "Props",
        224 => "LCTRL",
        225 => "LSHFT",
        226 => "LALT",
        227 => "LGUI",
        228 => "RCTRL",
        229 => "RSHFT",
        230 => "RALT",
        231 => "RGUI",
        _ => "?",
    }
}

// ── Encoder wiki editing state ────────────────────────────────────

/// Encoder wiki-editing state.
#[derive(Clone, Copy)]
pub(crate) struct WikiEditState {
    pub(crate) encoder: Player,
    pub(crate) selected: usize,
    pub(crate) editing: bool,
    pub(crate) working_threshold: u32,
    pub(crate) working_timeout: u32,
}

// ── MenuHandler settings impl ─────────────────────────────────────

impl<'a, D: WriteOnlyDataCommand> MenuHandler<'a, D> {
    /// Read the current value of a setting by key.
    pub(crate) fn read_setting(&self, key: SettingKey) -> u32 {
        lighting_read_block!(
            self,
            key,
            [
                [AllBgMode, PlayerBgMode, bg_mode],
                [AllBgRainbow, PlayerBgRainbow, bg_rainbow],
                [AllBgSpd, PlayerBgSpd, bg_speed_ds],
                [AllBgSubdiv, PlayerBgSubdiv, bg_subdivisions],
                [AllFgMode, PlayerFgMode, fg_mode],
                [AllFgRainbow, PlayerFgRainbow, fg_rainbow],
                [AllFgSpd, PlayerFgSpd, fg_speed_ds],
                [AllFgSubdiv, PlayerFgSubdiv, fg_subdivisions],
                [AllFgStep, PlayerFgStep, fg_step_ds],
                [AllFgSize, PlayerFgSize, fg_px_per_group],
                [AllTrigMode, PlayerTrigMode, trig_mode],
                [AllTrigRainbow, PlayerTrigRainbow, trig_rainbow],
                [AllTrigFdIn, PlayerTrigFdIn, trig_fade_in_ms],
                [AllTrigFdOut, PlayerTrigFdOut, trig_fade_out_ms],
                [AllTrigSize, PlayerTrigSize, trig_width],
                [AllTrigDir, PlayerTrigDir, trig_dir],
                [AllTrigOffset, PlayerTrigOffset, trig_offset],
                [AllTrigDur, PlayerTrigDur, trig_dur_s],
            ]
        );
        match key {
            SettingKey::AllButtonDebounce => self.all_debounce_value,
            SettingKey::ButtonDebounce(code) => {
                self.settings.buttons[code as usize].debounce_ticks as u32
            }
            SettingKey::EncoderDebounce(p) => {
                self.settings.encoders[p as usize].debounce_ticks as u32
            }
            SettingKey::EncoderStepThreshold(p) => {
                self.settings.encoders[p as usize].step_threshold as u32
            }
            SettingKey::EncoderMoveTimeout(p) => {
                self.settings.encoders[p as usize].move_timeout_ticks as u32
            }
            SettingKey::GlobalBrightness => self.settings.lighting.brightness as u32,
            _ => {
                debug!("read_setting: unexpected key");
                0
            }
        }
    }

    /// Write a value to a setting by key.
    pub(crate) fn write_setting(&mut self, key: SettingKey, value: u32) {
        self.settings_changed = true;
        self.prompt_answered_since_change = false;
        lighting_write_block!(
            self,
            key,
            value,
            [
                [AllBgMode, PlayerBgMode, bg_mode, u8],
                [AllBgRainbow, PlayerBgRainbow, bg_rainbow, u8],
                [AllBgSpd, PlayerBgSpd, bg_speed_ds, u16],
                [AllBgSubdiv, PlayerBgSubdiv, bg_subdivisions, u8],
                [AllFgMode, PlayerFgMode, fg_mode, u8],
                [AllFgRainbow, PlayerFgRainbow, fg_rainbow, u8],
                [AllFgSpd, PlayerFgSpd, fg_speed_ds, u16],
                [AllFgSubdiv, PlayerFgSubdiv, fg_subdivisions, u8],
                [AllFgStep, PlayerFgStep, fg_step_ds, u16],
                [AllFgSize, PlayerFgSize, fg_px_per_group, u8],
                [AllTrigMode, PlayerTrigMode, trig_mode, u8],
                [AllTrigRainbow, PlayerTrigRainbow, trig_rainbow, u8],
                [AllTrigFdIn, PlayerTrigFdIn, trig_fade_in_ms, u16],
                [AllTrigFdOut, PlayerTrigFdOut, trig_fade_out_ms, u16],
                [AllTrigSize, PlayerTrigSize, trig_width, u8],
                [AllTrigDir, PlayerTrigDir, trig_dir, u8],
                [AllTrigOffset, PlayerTrigOffset, trig_offset, u8],
                [AllTrigDur, PlayerTrigDur, trig_dur_s, u8],
            ]
        );
        match key {
            SettingKey::AllButtonDebounce => {
                self.all_debounce_value = value;
                for button in &mut self.settings.buttons {
                    button.debounce_ticks = value as u64;
                }
            }
            SettingKey::ButtonDebounce(code) => {
                self.settings.buttons[code as usize].debounce_ticks = value as u64;
            }
            SettingKey::EncoderDebounce(p) => {
                self.settings.encoders[p as usize].debounce_ticks = value as u64;
            }
            SettingKey::EncoderStepThreshold(p) => {
                self.settings.encoders[p as usize].step_threshold = value as i32;
            }
            SettingKey::EncoderMoveTimeout(p) => {
                self.settings.encoders[p as usize].move_timeout_ticks = value as u64;
            }
            SettingKey::GlobalBrightness => self.settings.lighting.brightness = value as u8,
            _ => unreachable!(),
        }
    }

    /// Read the current USB key binding for a button code.
    pub(crate) fn read_key_binding(&self, code: ButtonCode) -> u8 {
        let idx = code as usize;
        if idx < NUM_BUTTONS {
            self.settings.buttons[idx].key
        } else {
            match code {
                ButtonCode::P1Positive => self.settings.encoders[Player::P1 as usize].key_up,
                ButtonCode::P1Negative => self.settings.encoders[Player::P1 as usize].key_down,
                ButtonCode::P2Positive => self.settings.encoders[Player::P2 as usize].key_up,
                ButtonCode::P2Negative => self.settings.encoders[Player::P2 as usize].key_down,
                _ => 0,
            }
        }
    }

    /// Write a USB key binding for a button code.
    pub(crate) fn write_key_binding(&mut self, code: ButtonCode, key: u8) {
        let idx = code as usize;
        if idx < NUM_BUTTONS {
            self.settings.buttons[idx].key = key;
        } else {
            match code {
                ButtonCode::P1Positive => self.settings.encoders[Player::P1 as usize].key_up = key,
                ButtonCode::P1Negative => {
                    self.settings.encoders[Player::P1 as usize].key_down = key
                }
                ButtonCode::P2Positive => self.settings.encoders[Player::P2 as usize].key_up = key,
                ButtonCode::P2Negative => {
                    self.settings.encoders[Player::P2 as usize].key_down = key
                }
                _ => {}
            }
        }
        self.settings_changed = true;
        self.prompt_answered_since_change = false;
    }

    /// Execute a [`Commit`] by writing `value` into the appropriate field.
    pub(crate) fn commit_edit(&mut self, commit: Commit, value: u32) {
        match commit {
            Commit::Setting(key) => self.write_setting(key, value),
        }
    }

    /// Count how many settings differ from factory defaults.
    pub(crate) fn count_changes(&self) -> usize {
        let defaults = crate::flash_storage::FlashStoragePersistentMemory::default();
        for_each_changed_field(&self.settings, &defaults, |_, _, _| true)
    }

    /// Reset the `target_idx`th changed field back to its factory-default value.
    pub(crate) fn reset_field_to_default(&mut self, target_idx: usize) {
        let defaults = crate::flash_storage::FlashStoragePersistentMemory::default();
        let mut idx = 0_usize;

        for b in 0..NUM_BUTTONS {
            if self.settings.buttons[b].debounce_ticks != defaults.buttons[b].debounce_ticks {
                if idx == target_idx {
                    let key = SettingKey::ButtonDebounce(ButtonCode::from_repr(b).unwrap());
                    self.write_setting(key, defaults.buttons[b].debounce_ticks as u32);
                    return;
                }
                idx += 1;
            }
        }
        for b in 0..NUM_BUTTONS {
            if self.settings.buttons[b].key != defaults.buttons[b].key {
                if idx == target_idx {
                    let code = ButtonCode::from_repr(b).unwrap();
                    self.write_key_binding(code, defaults.buttons[b].key);
                    return;
                }
                idx += 1;
            }
        }
        for e in Player::iter() {
            let e_idx = e as usize;
            if self.settings.encoders[e_idx].key_up != defaults.encoders[e_idx].key_up {
                if idx == target_idx {
                    let code = match e {
                        Player::P1 => ButtonCode::P1Positive,
                        Player::P2 => ButtonCode::P2Positive,
                    };
                    self.write_key_binding(code, defaults.encoders[e_idx].key_up);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e_idx].key_down != defaults.encoders[e_idx].key_down {
                if idx == target_idx {
                    let code = match e {
                        Player::P1 => ButtonCode::P1Negative,
                        Player::P2 => ButtonCode::P2Negative,
                    };
                    self.write_key_binding(code, defaults.encoders[e_idx].key_down);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e_idx].debounce_ticks
                != defaults.encoders[e_idx].debounce_ticks
            {
                if idx == target_idx {
                    let key = SettingKey::EncoderDebounce(e);
                    self.write_setting(key, defaults.encoders[e_idx].debounce_ticks as u32);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e_idx].step_threshold
                != defaults.encoders[e_idx].step_threshold
            {
                if idx == target_idx {
                    let key = SettingKey::EncoderStepThreshold(e);
                    self.write_setting(key, defaults.encoders[e_idx].step_threshold as u32);
                    return;
                }
                idx += 1;
            }
            if self.settings.encoders[e_idx].move_timeout_ticks
                != defaults.encoders[e_idx].move_timeout_ticks
            {
                if idx == target_idx {
                    let key = SettingKey::EncoderMoveTimeout(e);
                    self.write_setting(key, defaults.encoders[e_idx].move_timeout_ticks as u32);
                    return;
                }
                idx += 1;
            }
        }
        // Lighting fields
        for p in Player::iter() {
            lighting_reset_checks!(
                self,
                p,
                idx,
                defaults,
                target_idx,
                [
                    [PlayerBgMode, bg_mode],
                    [PlayerBgRainbow, bg_rainbow],
                    [PlayerBgSpd, bg_speed_ds],
                    [PlayerBgSubdiv, bg_subdivisions],
                    [PlayerFgMode, fg_mode],
                    [PlayerFgRainbow, fg_rainbow],
                    [PlayerFgSpd, fg_speed_ds],
                    [PlayerFgSubdiv, fg_subdivisions],
                    [PlayerFgStep, fg_step_ds],
                    [PlayerFgSize, fg_px_per_group],
                    [PlayerTrigMode, trig_mode],
                    [PlayerTrigRainbow, trig_rainbow],
                    [PlayerTrigFdIn, trig_fade_in_ms],
                    [PlayerTrigFdOut, trig_fade_out_ms],
                    [PlayerTrigSize, trig_width],
                    [PlayerTrigDir, trig_dir],
                    [PlayerTrigOffset, trig_offset],
                    [PlayerTrigDur, trig_dur_s],
                ]
            );
        }
        // Global brightness
        if self.settings.lighting.brightness != defaults.lighting.brightness {
            if idx == target_idx {
                self.write_setting(
                    SettingKey::GlobalBrightness,
                    defaults.lighting.brightness as u32,
                );
                return;
            }
            idx += 1;
        }
        let _ = idx;
    }
}

/// Format a single changed setting into separate label and value buffers for the
/// labeled-edit display. This is a free function so it can be called from closures
/// without aliasing conflicts with `MenuHandler`'s `&mut self` methods.
pub(crate) fn format_change_item(
    settings: &crate::flash_storage::FlashStoragePersistentMemory,
    defaults: &crate::flash_storage::FlashStoragePersistentMemory,
    target_idx: usize,
    label_buf: &mut FmtBuf,
    value_buf: &mut FmtBuf,
) {
    let mut idx = 0_usize;
    for_each_changed_field(settings, defaults, |field, cur, _def| {
        if idx != target_idx {
            idx += 1;
            return true;
        }
        match field {
            FieldDescriptor::ButtonDebounce(b) => {
                let code = ButtonCode::from_repr(b).expect("index out of range");
                write!(label_buf, "{} Debounce", code.short_label()).ok();
                write!(value_buf, "{} ms", cur / 1_000).ok();
            }
            FieldDescriptor::ButtonKey(b) => {
                let code = ButtonCode::from_repr(b).expect("index out of range");
                write!(label_buf, "{} Key", code.short_label()).ok();
                write!(value_buf, "{}", key_name(cur as u8)).ok();
            }
            FieldDescriptor::EncoderKeyUp(e) => {
                let n = match e {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Positive Key", n).ok();
                write!(value_buf, "{}", key_name(cur as u8)).ok();
            }
            FieldDescriptor::EncoderKeyDown(e) => {
                let n = match e {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Negative Key", n).ok();
                write!(value_buf, "{}", key_name(cur as u8)).ok();
            }
            FieldDescriptor::EncoderDebounce(e) => {
                let n = match e {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Enc Debounce", n).ok();
                write!(value_buf, "{} ms", cur / 1_000).ok();
            }
            FieldDescriptor::EncoderStepThreshold(e) => {
                let n = match e {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Threshold", n).ok();
                write!(value_buf, "{} Steps", cur).ok();
            }
            FieldDescriptor::EncoderMoveTimeout(e) => {
                let n = match e {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Timeout", n).ok();
                write!(value_buf, "{} ms", cur / 1_000).ok();
            }
            FieldDescriptor::PlayerBgMode(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} BG Mode", n).ok();
                write!(value_buf, "{}", BG_MODE_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerBgRainbow(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} BG Rainbow", n).ok();
                write!(value_buf, "{}", RAINBOW_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerBgSpd(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} BG Speed", n).ok();
                write!(value_buf, "{} s", cur / 10).ok();
            }
            FieldDescriptor::PlayerBgSubdiv(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} BG Subdiv", n).ok();
                write!(value_buf, "{}", cur).ok();
            }
            FieldDescriptor::PlayerFgMode(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Mode", n).ok();
                write!(value_buf, "{}", FG_MODE_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerFgRainbow(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Rainbow", n).ok();
                write!(value_buf, "{}", RAINBOW_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerFgSpd(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Speed", n).ok();
                write!(value_buf, "{} s", cur / 10).ok();
            }
            FieldDescriptor::PlayerFgSubdiv(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Subdiv", n).ok();
                write!(value_buf, "{}", cur).ok();
            }
            FieldDescriptor::PlayerFgStep(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Step", n).ok();
                write!(value_buf, "{} s", cur / 10).ok();
            }
            FieldDescriptor::PlayerFgSize(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} FG Size", n).ok();
                write!(value_buf, "{} px", cur).ok();
            }
            FieldDescriptor::PlayerTrigMode(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Mode", n).ok();
                write!(value_buf, "{}", TRIG_MODE_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerTrigRainbow(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Rainbow", n).ok();
                write!(value_buf, "{}", RAINBOW_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerTrigFdIn(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Fade In", n).ok();
                write!(value_buf, "{} ms", cur).ok();
            }
            FieldDescriptor::PlayerTrigFdOut(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Fade Out", n).ok();
                write!(value_buf, "{} ms", cur).ok();
            }
            FieldDescriptor::PlayerTrigSize(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Size", n).ok();
                write!(value_buf, "{} px", cur).ok();
            }
            FieldDescriptor::PlayerTrigDir(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Dir", n).ok();
                write!(value_buf, "{}", DIR_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerTrigOffset(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Off", n).ok();
                write!(value_buf, "{}", OFFSET_NAMES[cur as usize]).ok();
            }
            FieldDescriptor::PlayerTrigDur(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Trig Cycle", n).ok();
                write!(value_buf, "{} s", cur).ok();
            }
            FieldDescriptor::PlayerBrightness(p) => {
                let n = match p {
                    Player::P1 => 1,
                    Player::P2 => 2,
                };
                write!(label_buf, "P{} Brightness", n).ok();
                write!(value_buf, "{}", cur).ok();
            }
        }
        false
    });
}
