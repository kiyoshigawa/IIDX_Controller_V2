//! Lighting-handling module.
//!
//! Provides [`LightingHandler`] — the core1-level manager that owns both
//! per-player [`Animation`]s, the shared colour buffer, the gamma/brightness
//! pipeline, and event handling.
//!
//! # Layout
//!
//! The 58-LED physical strip is split into two logical halves of 29 LEDs each.
//! Player 1 occupies indices 0–28, Player 2 occupies 29–57.  Each half is
//! driven by an independent [`Animation`] with its own background, foreground,
//! and trigger state.

use crate::flash_storage::PlayerAnimConfig;
use crate::led_strip::{LEDS_PER_SIDE, NUM_LEDS};
use crate::{NUM_PLAYERS, Player};
use embedded_time::rate::Hertz;
use lighting_controller::animations::Animatable;
use lighting_controller::{self as lc, animations, utility};
use rgb::RGB8;
use smart_leds::colors::BLACK;
use strum::IntoEnumIterator;

// ──────────────────────────────────────────────────────────────────────────────
// Gamma correction
// ──────────────────────────────────────────────────────────────────────────────

/// Gamma-correction lookup table (standard 2.2 gamma, stored in flash).
const GAMMA_TABLE: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

/// Default LED brightness level (0–255).
pub const DEFAULT_BRIGHTNESS: u8 = 200;

// ──────────────────────────────────────────────────────────────────────────────
// LightingHandler
// ──────────────────────────────────────────────────────────────────────────────

/// Owns the LED strip colour buffer, both per-player animations, and the
/// gamma/brightness pipeline.
pub struct LightingHandler {
    /// The frame buffer written to the WS2812 strip each cycle.
    pub color_buffer: [RGB8; NUM_LEDS],

    /// Per-player animations, indexed by Player.
    animations: [animations::Animation<'static, LEDS_PER_SIDE>; NUM_PLAYERS],

    /// Current brightness level (0–255).
    brightness: u8,

    /// Frame rate used for duration → frame-count conversions.
    frame_rate: Hertz,

    /// Shadow copy of per-player lighting config for on-the-fly trigger param construction.
    player_cfg: [PlayerAnimConfig; NUM_PLAYERS],

    /// Last-applied config for efficient change detection during live sync.
    last_applied_cfg: crate::flash_storage::LightingConfig,

    /// Alternates trigger direction on each button press per player.
    trigger_dir_toggle: [bool; NUM_PLAYERS],
}

impl LightingHandler {
    /// Create a new handler with two per-player animations, both initialised
    /// with the given rainbow and frame rate.
    pub fn new(frame_rate: Hertz, rainbow: &'static [RGB8]) -> Self {
        let p1 = animations::Animation::<LEDS_PER_SIDE>::new(
            lc::default_animations::ANI_DEFAULT,
            frame_rate,
        )
        .set_translation_array(utility::default_translation_array::<LEDS_PER_SIDE>(0))
        .set_bg_rainbow(rainbow, animations::RainbowDir::Forward)
        .set_bg_subdivisions(2)
        .set_trig_incremental_rainbow(lc::colors::R_BLACK, animations::RainbowDir::Forward)
        .set_trig_fade_rainbow(lc::colors::R_BLACK, animations::RainbowDir::Forward);

        let mut p2 = animations::Animation::<LEDS_PER_SIDE>::new(
            lc::default_animations::ANI_DEFAULT,
            frame_rate,
        )
        .set_translation_array(utility::default_translation_array::<LEDS_PER_SIDE>(
            LEDS_PER_SIDE,
        ))
        .set_bg_rainbow(rainbow, animations::RainbowDir::Forward)
        .set_bg_subdivisions(2)
        .set_bg_direction(animations::Direction::Negative);

        p2.set_offset(
            animations::AnimationType::Background,
            animations::MAX_OFFSET / 2,
        );
        p2.update_trig_incremental_rainbow(lc::colors::R_BLACK, animations::RainbowDir::Forward);
        p2.update_trig_fade_rainbow(lc::colors::R_BLACK, animations::RainbowDir::Forward);

        Self {
            color_buffer: [BLACK; NUM_LEDS],
            animations: [p1, p2],
            brightness: DEFAULT_BRIGHTNESS,
            frame_rate,
            player_cfg: [PlayerAnimConfig::default(); NUM_PLAYERS],
            last_applied_cfg: crate::flash_storage::LightingConfig::default(),
            trigger_dir_toggle: [false; NUM_PLAYERS],
        }
    }

    /// Drive both animations forward one frame and copy their segment data
    /// into the shared colour buffer.
    pub fn update(&mut self) {
        for p in Player::iter() {
            let a = &mut self.animations[p as usize];
            a.update();
            let seg = a.segment();
            let trans = a.translation_array();
            for (&i, &c) in trans.iter().zip(seg.iter()) {
                self.color_buffer[i] = c;
            }
        }
    }

    /// Apply gamma correction and brightness scaling **in-place** on the
    /// colour buffer.
    pub fn apply_gamma_brightness(&mut self) {
        let scale = self.brightness as u16;

        for led in self.color_buffer.iter_mut() {
            led.r = GAMMA_TABLE[led.r as usize];
            led.g = GAMMA_TABLE[led.g as usize];
            led.b = GAMMA_TABLE[led.b as usize];
            led.r = ((led.r as u16 * scale + 127) / 255) as u8;
            led.g = ((led.g as u16 * scale + 127) / 255) as u8;
            led.b = ((led.b as u16 * scale + 127) / 255) as u8;
        }
    }

    /// Convenience: update animations, gamma-correct, and write to the strip
    /// in one call.
    pub fn update_and_write<CH, SM>(
        &mut self,
        strip: &mut crate::led_strip::DmaLedStrip<CH, SM>,
        now: u64,
    ) where
        CH: rp235x_hal::dma::SingleChannel,
        SM: rp235x_hal::pio::ValidStateMachine,
    {
        self.update();
        self.apply_gamma_brightness();
        strip.write_frame(&self.color_buffer, now);
    }

    /// The frame rate used internally for animation timing.
    pub fn frame_rate(&self) -> Hertz {
        self.frame_rate
    }

    /// Set the global brightness (0–255).
    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    /// Apply per-player lighting config from flash settings.
    pub fn apply_config(&mut self, cfg: &crate::flash_storage::LightingConfig) {
        self.brightness = cfg.brightness;
        self.player_cfg = cfg.players;
        self.last_applied_cfg = *cfg;
        for (i, pcfg) in cfg.players.iter().enumerate() {
            let player = if i == 0 { Player::P1 } else { Player::P2 };
            self.apply_player_config(player, pcfg);
        }
    }

    /// push animation config changes made to the live animations
    pub fn sync_config(&mut self, cfg: &crate::flash_storage::LightingConfig) {
        if self.last_applied_cfg == *cfg {
            return;
        }
        self.apply_config(cfg);
    }

    fn apply_player_config(&mut self, player: Player, pcfg: &PlayerAnimConfig) {
        let anim: &mut dyn Animatable = &mut self.animations[player as usize];

        // BG direction from config
        let bg_dir = match pcfg.bg_dir {
            crate::DIR_STOP => animations::Direction::Stopped,
            crate::DIR_REV => animations::Direction::Negative,
            _ => animations::Direction::Positive,
        };
        // FG direction from config
        let fg_dir = match pcfg.fg_dir {
            crate::DIR_STOP => animations::Direction::Stopped,
            crate::DIR_REV => animations::Direction::Negative,
            _ => animations::Direction::Positive,
        };

        match pcfg.bg_mode {
            crate::BG_ROTATE | crate::BG_FOLLOW => {
                // Rotate or Follow — FillRainbowRotate with config direction
                anim.update_bg_mode(animations::background::Mode::FillRainbowRotate);
                anim.update_bg_direction(bg_dir);
            }
            crate::BG_SOLID => anim.update_bg_mode(animations::background::Mode::Solid),
            crate::BG_SOLID_FADE => anim.update_bg_mode(animations::background::Mode::SolidFade),
            crate::BG_OFF => anim.update_bg_mode(animations::background::Mode::NoBackground),
            _ => {}
        }

        // BG rainbow & subdivisions
        let bg_rainbow = crate::lighting_consts::RAINBOW_SLICES
            .get(pcfg.bg_rainbow as usize)
            .copied()
            .unwrap_or(crate::lighting_consts::TWELVE_BIT_OKLCH_RAINBOW);
        anim.update_bg_rainbow(bg_rainbow, animations::RainbowDir::Forward);
        anim.update_bg_subdivisions(pcfg.bg_subdivisions as usize);

        // BG speed
        let bg_ns = (pcfg.bg_speed_ds as u64) * 100_000_000;
        anim.update_bg_duration_ns(bg_ns, self.frame_rate);

        // FG mode
        match pcfg.fg_mode {
            crate::FG_OFF => anim.update_fg_mode(animations::foreground::Mode::NoForeground),
            crate::FG_MARQUEE => anim.update_fg_mode(animations::foreground::Mode::MarqueeSolid),
            crate::FG_MARQUEE_FIXED => {
                anim.update_fg_mode(animations::foreground::Mode::MarqueeSolidFixed)
            }
            crate::FG_MARQUEE_FADE => {
                anim.update_fg_mode(animations::foreground::Mode::MarqueeFade)
            }
            crate::FG_MARQUEE_FADE_FIXED => {
                anim.update_fg_mode(animations::foreground::Mode::MarqueeFadeFixed)
            }
            crate::FG_VU_METER => anim.update_fg_mode(animations::foreground::Mode::VUMeter),
            _ => {}
        }

        // FG rainbow & subdivisions
        let fg_rainbow = crate::lighting_consts::RAINBOW_SLICES
            .get(pcfg.fg_rainbow as usize)
            .copied()
            .unwrap_or(crate::lighting_consts::TWELVE_BIT_OKLCH_RAINBOW);
        anim.update_fg_rainbow(fg_rainbow, animations::RainbowDir::Forward);
        anim.update_fg_subdivisions(pcfg.fg_subdivisions as usize);

        // FG speed & step
        let fg_ns = (pcfg.fg_speed_ds as u64) * 100_000_000;
        anim.update_fg_duration_ns(fg_ns, self.frame_rate);
        let step_ns = (pcfg.fg_step_ds as u64) * 100_000_000;
        anim.update_fg_step_time_ns(step_ns, self.frame_rate);
        anim.update_fg_pixels_per_pixel_group(pcfg.fg_px_per_group as usize);

        // FG direction from config
        anim.update_fg_direction(fg_dir);

        // Trigger rainbows
        let trig_rainbow = crate::lighting_consts::RAINBOW_SLICES
            .get(pcfg.trig_rainbow as usize)
            .copied()
            .unwrap_or(crate::lighting_consts::TWELVE_BIT_OKLCH_RAINBOW);
        anim.update_trig_incremental_rainbow(trig_rainbow, animations::RainbowDir::Forward);
        anim.update_trig_fade_rainbow(trig_rainbow, animations::RainbowDir::Forward);

        // Trigger cycle duration
        let dur_ns = (pcfg.trig_dur_s as u64) * 1_000_000_000;
        anim.update_trig_duration_ns(dur_ns, self.frame_rate);
    }

    /// Process a [`LightingEvent`] — called from the core1 loop when SIO FIFO
    /// data indicates an encoder position change or button press.
    pub fn handle_event(&mut self, event: LightingEvent) {
        match event {
            LightingEvent::EncoderMoved { player, count } => {
                // Only apply encoder offset in Follow mode.
                if self.player_cfg[player as usize].bg_mode != crate::BG_FOLLOW {
                    return;
                }
                const STEPS_PER_REV: i32 = 2400;
                let normalized = count.rem_euclid(STEPS_PER_REV);
                let offset = (normalized * (u16::MAX as i32)) / STEPS_PER_REV;
                self.animations[player as usize]
                    .set_offset(animations::AnimationType::Background, offset as u16);
            }
            LightingEvent::DirectionChanged { player, direction } => {
                // Only respond in Follow mode.
                if self.player_cfg[player as usize].bg_mode != crate::BG_FOLLOW {
                    return;
                }
                let dir = match direction {
                    crate::EncoderDirection::Positive => animations::Direction::Positive,
                    crate::EncoderDirection::Negative => animations::Direction::Negative,
                    crate::EncoderDirection::Stopped => return,
                };
                self.animations[player as usize].update_bg_direction(dir);
            }
            LightingEvent::ButtonPressed { player } => {
                let p_idx = player as usize;
                let cfg = &self.player_cfg[p_idx];
                if cfg.trig_mode == crate::TRIG_OFF {
                    return;
                }
                // Trigger direction: config sets starting direction, toggles each fire
                let dir = match (cfg.trig_dir, self.trigger_dir_toggle[p_idx]) {
                    (crate::DIR_FWD, false) => animations::Direction::Positive,
                    (crate::DIR_FWD, true) => animations::Direction::Negative,
                    (crate::DIR_STOP, _) => animations::Direction::Stopped,
                    (crate::DIR_REV, false) => animations::Direction::Negative,
                    (crate::DIR_REV, true) => animations::Direction::Positive,
                    _ => animations::Direction::Positive,
                };
                if cfg.trig_dir != crate::DIR_STOP {
                    self.trigger_dir_toggle[p_idx] = !self.trigger_dir_toggle[p_idx];
                }
                // Trigger offset: Random, Center, or Top
                let offset = match cfg.trig_offset {
                    crate::OFFSET_CENTER => animations::MAX_OFFSET / 2,
                    crate::OFFSET_TOP => 0,
                    _ => 0, // Random — overridden by init functions
                };
                let params = animations::trigger::Parameters {
                    mode: match cfg.trig_mode {
                        crate::TRIG_PULSE => animations::trigger::Mode::ColorPulse,
                        crate::TRIG_PULSE_FADE => animations::trigger::Mode::ColorPulseFade,
                        crate::TRIG_PULSE_RAINBOW => animations::trigger::Mode::ColorPulseRainbow,
                        crate::TRIG_SHOT => animations::trigger::Mode::ColorShot,
                        crate::TRIG_SHOT_FADE => animations::trigger::Mode::ColorShotFade,
                        crate::TRIG_SHOT_RAINBOW => animations::trigger::Mode::ColorShotRainbow,
                        crate::TRIG_FLASH => animations::trigger::Mode::Flash,
                        crate::TRIG_FLASH_FADE => animations::trigger::Mode::FlashFade,
                        crate::TRIG_FLASH_RAINBOW => animations::trigger::Mode::FlashRainbow,
                        _ => animations::trigger::Mode::ColorPulse,
                    },
                    direction: dir,
                    fade_in_time_ns: (cfg.trig_fade_in_ms as u64) * 1_000_000,
                    fade_out_time_ns: (cfg.trig_fade_out_ms as u64) * 1_000_000,
                    starting_offset: offset,
                    pixels_per_pixel_group: cfg.trig_width as usize,
                };
                self.animations[player as usize].trigger(&params, self.frame_rate);
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// LightingEvent
// ──────────────────────────────────────────────────────────────────────────────

/// Events forwarded to [`LightingHandler`] from the core1 loop in response
/// to SIO FIFO input data or button press notifications.
/// Events generated by the input handler to control the lighting system.
pub enum LightingEvent {
    /// Encoder position changed for the given player.
    EncoderMoved { player: Player, count: i32 },
    /// Encoder spin direction changed.
    DirectionChanged {
        player: Player,
        direction: crate::EncoderDirection,
    },
    /// A gameplay or encoder-derived button was pressed.
    ButtonPressed { player: Player },
}
