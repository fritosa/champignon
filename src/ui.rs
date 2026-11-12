//! Game interface: panel rendering, animations and key handling, on any `Screen`.

use std::collections::HashMap;
use std::fmt;

use crate::art;
use crate::data::{self, UNITS};
use crate::events::{self, ACHIEVEMENTS, SYNERGIES};
use crate::game::{self, AUDIO_MODES, Game, MsgKind, RunReport, VOLUME_STEP, need};
use crate::i18n::ui::KeyHelp;
pub use crate::i18n::ui::TAB_COUNT;
use crate::i18n::{self, tr};
use crate::rng::{RandomExt, Rng, StdRandom};
use crate::screens::{Interrupted, STORY_GUARD, center, fdiv, head, intro, slice, story, wrap};
use crate::sound::SoundSink;
use crate::term::{Color, Key, Screen, Style, width};

/// Seconds for a challenge monster to cross the field.
pub const MONSTER_CROSSING: f64 = 12.0;
/// Minimum delay between two plays of the same sound.
pub const SFX_GAP: [(&str, f64); 7] = [
    ("level", 0.6),
    ("hit", 0.08),
    ("achievement", 0.6),
    ("crit", 0.15),
    ("buy", 0.12),
    ("fizzle", 0.5),
    ("jam", 0.15),
];
pub const DEFAULT_SFX_GAP: f64 = 0.05;
pub const MSG_TIME: f64 = 6.0;
pub const BIG_TIME: f64 = 3.0; // title in big letters (important messages: see BIG_EARLY_EXIT)
/// Screen shake duration per sound (the boom flash lasts 0.8 s).
pub const SHAKE: [(&str, f64); 4] = [
    ("boom", 1.3),
    ("perfect", 0.6),
    ("prestige", 0.6),
    ("supernova", 1.5),
];
pub const QTY: [Qty; 4] = [Qty::N(1), Qty::N(10), Qty::N(100), Qty::Max];
pub const MIN_W: i32 = 60;
pub const MIN_H: i32 = 20;
pub const MAX_FLOATS: usize = 6; // floating numbers shown at once
const MAX_PARTICLES: usize = 60;
const LONG_AGO: f64 = -9.0; // timestamp of "never": older than any delay of the interface
const TITLE_INTERVAL: f64 = 1.0; // seconds between two terminal title updates
const CONFIRM_TIME: f64 = 3.0; // seconds to press Enter again on a prestige, challenge or supernova
const PAGE_ROWS: usize = 5; // selection jump of PageUp / PageDown
const EPIC_CAP_DIV: f64 = 2.0; // epic music once the level reaches cap / EPIC_CAP_DIV
const PRESTIGE_TAB: usize = 2; // index of the tab, "???" until the prestige is unlocked

// Blink periods (seconds per on/off phase).
const BLINK_OVERHEAT: f64 = 0.2; // mushroom frame and stats line during an overheat
const BLINK_FLASH: f64 = 0.1; // explosion flash, fresh messages
const BLINK_PROMPT: f64 = 0.6; // "Appuie sur…" invitations
const BLINK_BIG_TITLE: f64 = 0.15; // big title of an important message
const BLINK_ALERT: f64 = 0.3; // inspector and enemy alerts, inspector waving
const BLINK_LEVEL_MAX: f64 = 0.5;
const BLINK_PLANE: f64 = 0.25; // letter of the supply plane
const BLINK_GAUGE_READY: f64 = 0.4; // critical mass ready

// Layout of the game screen.
const LEFT_MIN_W: i32 = 30; // mushroom panel: at least this wide…
const LEFT_SHARE: f64 = 0.45; // …or this share of the terminal width…
// …or all but this many columns: a wide terminal widens the mushroom, the tabs keep room for the key help.
const RIGHT_MAX_W: i32 = 84;
const STATS_H: i32 = 10;
const PANEL_PAD: i32 = 2; // columns between a panel edge and its text (border + space)
const INITIAL_TAB_W: i32 = 40; // inner tab width before the first draw
const SEL_PREFIX_W: i32 = 2; // "▶ " before every tab row
const POPUP_MARGIN: i32 = 4; // columns kept around a popup narrower than the screen

// Messages under the mushroom panel edge.
const MSG_SHOWN: usize = 3; // latest messages stacked
const MSG_MAX_LINES: usize = 2; // lines of a wrapped message
const MSG_MIN_W: i32 = 10; // wrap width on a tiny panel
const MSG_WRAP_MARGIN: i32 = 4; // columns of the panel not used by message text
const MSG_FLASH_TIME: f64 = 0.6; // a new message blinks in reverse video this long
const MSG_FADE: f64 = 1.0; // last seconds of a message, shown dimmer
const BIG_EARLY_EXIT: f64 = 1.0; // the big title of an important message goes this long before its line
const BIG_FLASH_TIME: f64 = 1.0; // the big title of an important message flashes this long

// Mushroom panel: scenery and animations.
const SHAKE_MIN_W: i32 = 4; // narrower, the panel does not shake
const STEM_GAP_MIN: i32 = 3; // columns kept clear on each side of the stem…
const STEM_GAP_DIV: i32 = 6; // …or the foot width / STEM_GAP_DIV
const ARMY_GAP_MIN: i32 = 2; // same for the army sprites
const ARMY_GAP_DIV: i32 = 10;
const SKY_GROUND_MARGIN: i32 = 3; // rows above the ground without sky objects
const SKY_MIN_H: i32 = 2;
const SKY_MIN_W: i32 = 10;
const SKY_SEED_STRIDE: i32 = 1000; // sky seed: iw * SKY_SEED_STRIDE + ground (same sky for a same size)
const SKY_STARS_LEVEL: u32 = 100; // clouds below, stars from there
const SKY_PLANETS_LEVEL: u32 = 1000;
const CLOUD: &str = "(~~)";
const CLOUD_SPACING: i32 = 25; // one cloud per CLOUD_SPACING columns
const CLOUD_SPEED: f64 = 0.5; // columns/s of the first cloud…
const CLOUD_SPEED_STEP: f64 = 0.3; // …each next one is faster by this much
const WRAP_MARGIN: i32 = 6; // columns past the left edge before a drifting sprite reappears on the right
const STAR_DENSITY: i32 = 30; // one star per STAR_DENSITY cells of sky
const BIG_STAR_EVERY: i32 = 7;
const TWINKLE_SPEED: f64 = 2.0; // twinkle steps per second
const TWINKLE_EVERY: i64 = 9; // one star in TWINKLE_EVERY is lit
const ARMY_COPIES: u32 = 3; // sprites per unit type at most…
const ARMY_COPY_BASE: u64 = 10; // …the k-th from ARMY_COPY_BASE^k units
const ARMY_COPY_PHASE: f64 = 0.3; // animation offset (s) between copies
const ARMY_UPPER_ROW: i32 = 5; // rows above the ground of the second row of sprites…
const ARMY_UPPER_MIN: i32 = 12; // …which needs at least this many rows above the ground
const BOMBERS_SHOWN: u64 = 2;
const BOMBER_SPEED: f64 = 8.0; // columns/s
const BOMBER_SPACING: i32 = 23; // columns between two bombers
const BOMBER_TOP: i32 = 3; // bottom row of the first bomber…
const BOMBER_ROW_STEP: i32 = 3; // …and of each next one, lower
const PARTICLE_RATE_BASE: f64 = 0.4; // projectiles/s of a unit type: base…
const PARTICLE_RATE_LOG: f64 = 0.6; // …+ this × log2(1 + count)…
const PARTICLE_RATE_MAX: f64 = 5.0; // …at most this
const PARTICLE_SKY_ROW: i32 = 2; // bombs fall from this row
const PARTICLE_PEAK_MIN: f64 = 2.0; // height of a projectile arc (rows)
const PARTICLE_PEAK_MAX: f64 = 5.0;
const PARTICLE_SPREAD: i64 = 2; // impact columns around the stem center
const PARTICLE_DUR_MIN: f64 = 0.7; // flight duration (s)
const PARTICLE_DUR_MAX: f64 = 1.3;
const IMPACT_END: f64 = 1.2; // impact star shown until this share of the flight duration
const FLOAT_TIME: f64 = 0.9; // seconds a floating number stays
const FLOAT_SPEED: f64 = 4.0; // rows/s
const FLOAT_SPREAD: i64 = 4; // random horizontal offset of a floating number
const PLANE_EXIT: i32 = 8; // the plane flies until this many columns past the left edge
const PLANE_LABEL_GAP: i32 = 2; // columns from the plane to its letter
const INSPECTOR_START: i32 = 6; // columns from the right edge where the inspector appears
const HIPPIES_RUSH: f64 = 4.0; // protesters walk this much faster, then stage their sit-in
const DRONE_ALTITUDE: i32 = 2; // sky enemies: rows above their sprite top

// Statistics panel.
const COMBO_BAR_MAX: i32 = 20;
const COMBO_LABEL_W: i32 = 16; // "Combo ×1.0 " and some room
const GAUGE_MIN_W: i32 = 5;

// Tabs.
const TAB_LABEL_SHORT: i32 = 5; // characters kept of the tab names when they don't fit…
const TAB_LABEL_TINY: i32 = 3; // …then fewer
const UNIT_NAME_MIN_W: i32 = 8;
const UNIT_COLUMNS_W: i32 = 28; // "▶ " + count, production and cost columns of a unit row
const NAME_MIN_W: i32 = 10; // talent names
const TALENT_COLUMNS_W: i32 = 16; // "▶ " + rank and cost columns of a talent row
const TREE_COLUMNS_W: i32 = 18; // same for the prestige and supernova trees
const STAT_NAME_MIN_W: i32 = 6;
const STAT_VALUE_MIN_W: i32 = 10;
const STAT_ROW_EXTRA_W: i32 = 4; // "▶ ", space between the columns, free last column
// Heading rules (──), shortened so that the headings end about the same column.
const RULE: usize = 20;
const RULE_COUNTED: usize = 10; // headings with a counter in them
const RULE_STATS: usize = 15;
const RULE_LAST_RUN: usize = 12;
const SPRITE_MIN_TAB_W: i32 = 36; // narrower, no unit sprite next to the description
const SPRITE_GAP: i32 = 2; // columns between the description and the sprite
const MIN_LIST_H: i32 = 5; // rows kept for the list above the description
const MIN_DESC_H: i32 = 2;

// Overlays.
const QTE_W: i32 = 44;
const QTE_H: i32 = 6;
const QTE_ROW_DIV: i32 = 3; // the QTE box is a 1/QTE_ROW_DIV of the way down the panel
const QTE_RESULT_H: i32 = 3;
const REPORT_W: i32 = 72;
const REPORT_CHROME_H: i32 = 4; // borders, blank row and prompt of the end screen
const REPORT_LABEL_GAP: i32 = 2; // columns between labels and values
const REPORT_VALUE_MIN_W: i32 = 10;
const REPORT_WRAP_MARGIN: i32 = 6; // columns of the box not used by values
const REPORT_VALUE_LINES: usize = 2; // lines of a wrapped value
const REPORT_CHESTS_SHOWN: usize = 6; // latest chests listed
const POPUP_W: i32 = 70; // chest and mutation choices
const POPUP_CHROME_H: i32 = 4; // borders, intro row and bottom blank row
const OPTION_ROW_H: i32 = 2; // name, then description
const OPTION_DESC_X: i32 = 5; // description indent in the popup
const NEWS_SPEED: f64 = 12.0; // columns/s of the news ticker
pub const NEWS_MARGIN: i32 = 2; // columns of the news line outside the label and the ticker

fn sfx_gap(key: &str) -> f64 {
    SFX_GAP
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(DEFAULT_SFX_GAP, |(_, gap)| *gap)
}

fn shake(name: &str) -> Option<f64> {
    SHAKE.iter().find(|(k, _)| *k == name).map(|(_, d)| *d)
}

/// Player-facing name of an audio mode.
pub fn audio_label(mode: &str) -> &'static str {
    let ui = &tr().ui;
    match mode {
        "all" => ui.audio_all,
        "sfx" => ui.audio_sfx,
        _ => ui.audio_off,
    }
}

/// Key help of the Infos tab: (keys, what they do), in the current language.
pub fn key_help() -> KeyHelp {
    let modes: Vec<&str> = AUDIO_MODES.iter().map(|m| audio_label(m)).collect();
    (tr().ui.key_help)(&modes, VOLUME_STEP)
}

/// Quantity bought by Enter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qty {
    N(u64),
    Max,
}

impl fmt::Display for Qty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Qty::N(n) => write!(f, "{n}"),
            Qty::Max => write!(f, "{}", tr().ui.qty_max),
        }
    }
}

/// What Enter does on a tab row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Unit(&'static str),
    Talent(&'static str),
    PTalent(&'static str),
    STalent(&'static str),
    Prestige,
    Challenge(&'static str),
    Abandon,
    /// Turns the double challenge mode on or off.
    DoubleMode,
    /// Shows or hides the challenges done at every rank.
    ToggleDone,
    /// Shows or hides the challenge stories to read again.
    ToggleStories,
    Supernova,
    Skin,
    Intro,
    Story(&'static str),
}

/// A tab row. sprite: (unit id, unlocked) to draw its art next to the description.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub text: String,
    pub style: Style,
    pub action: Option<Action>,
    pub desc: String,
    pub sel: bool,
    pub sprite: Option<(&'static str, bool)>,
}

/// Row folding or unfolding a list (`open`: unfolded): label, arrow, and what Enter shows or hides.
fn fold_row(open: bool, label: &str, style: Style, action: Action, what: &str) -> Row {
    let arrow = if open { '▾' } else { '▸' };
    Row::new(
        format!("{label} {arrow}"),
        style,
        Some(action),
        (tr().ui.fold)(open, what),
    )
}

impl Row {
    /// Selectable row.
    pub fn new(
        text: impl Into<String>,
        style: Style,
        action: Option<Action>,
        desc: impl Into<String>,
    ) -> Row {
        Row {
            text: text.into(),
            style,
            action,
            desc: desc.into(),
            sel: true,
            sprite: None,
        }
    }

    /// Title or text row, skipped by the selection.
    pub fn label(text: impl Into<String>, style: Style) -> Row {
        Row {
            sel: false,
            ..Row::new(text, style, None, "")
        }
    }

    fn with_sprite(self, uid: &'static str, unlocked: bool) -> Row {
        Row {
            sprite: Some((uid, unlocked)),
            ..self
        }
    }
}

/// Unit projectile flying toward the mushroom.
#[derive(Clone, Debug)]
pub struct Particle {
    sx: i32,
    sy: i32,
    tx: i32,
    ty: i32,
    peak: f64,
    age: f64,
    dur: f64,
    ch: &'static str,
    col: &'static str,
}

/// Text rising above the mushroom.
#[derive(Clone, Debug)]
pub struct Float {
    pub text: String,
    pub age: f64,
    pub dx: i32,
    pub style: Style,
}

/// What the main loop does after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    /// Q or Ctrl+C: save and quit.
    Quit,
}

type Line = (String, Style);

pub struct UI<S: Screen> {
    pub scr: S,
    pub g: Game,
    pub sound: Option<Box<dyn SoundSink>>,
    pub sfx_last: HashMap<&'static str, f64>,
    pub qty: Qty,
    /// Double challenge mode: Enter on a challenge picks it, then Enter on a second one starts both.
    pub double: bool,
    /// First challenge picked in double mode.
    pub double_pick: Option<&'static str>,
    /// Challenges done at every rank listed in the Prestige tab (folded by default).
    pub show_done: bool,
    /// Challenge stories listed in the Infos tab (folded by default).
    pub show_stories: bool,
    pub shake_until: f64,
    /// Last terminal title; None: title disabled (main enables it with Some("")).
    pub title: Option<String>,
    pub title_at: f64,
    pub tab: usize,
    pub sel: [usize; TAB_COUNT],
    pub scroll: [usize; TAB_COUNT],
    pub particles: Vec<Particle>,
    pub floats: Vec<Float>,
    pub spawn_acc: HashMap<&'static str, f64>,
    pub news: &'static str,
    pub news_off: f64,
    /// Row waiting for its confirmation press, and until when.
    pub confirm: Option<(Action, f64)>,
    pub t: f64,
    pub h: i32,
    pub w: i32,
    pub tab_w: i32,
    pub rng: Box<dyn Rng>,
    /// Seconds of keys ignored at the start of a challenge story (STORY_GUARD).
    pub story_guard: f64,
}

impl<S: Screen> UI<S> {
    pub fn new(scr: S, game: Game, sound: Option<Box<dyn SoundSink>>) -> UI<S> {
        UI {
            scr,
            g: game,
            sound,
            sfx_last: HashMap::new(),
            qty: Qty::N(1),
            double: false,
            double_pick: None,
            show_done: false,
            show_stories: false,
            shake_until: 0.0,
            title: None,
            title_at: LONG_AGO,
            tab: 0,
            sel: [0; TAB_COUNT],
            scroll: [0; TAB_COUNT],
            particles: Vec::new(),
            floats: Vec::new(),
            spawn_acc: HashMap::new(),
            news: "",
            news_off: 0.0,
            confirm: None,
            t: 0.0,
            h: 0,
            w: 0,
            tab_w: INITIAL_TAB_W,
            rng: Box::new(StdRandom::new()),
            story_guard: STORY_GUARD,
        }
    }

    // --- Low level -----------------------------------------------------------------

    /// Style of a named color ("red"…"grey", "flash", "alert", "msg_<kind>").
    pub fn c(&self, name: &str) -> Style {
        Style::named(name)
    }

    pub fn blink(&self, period: f64) -> bool {
        (self.t / period) as i64 % 2 == 0
    }

    /// Writes clipped to the screen: what sticks out on the left or the right is cut.
    pub fn put(&mut self, y: i32, x: i32, s: &str, style: Style) {
        if y < 0 || y >= self.h || x >= self.w || s.is_empty() {
            return;
        }
        // Borrowed cuts (no allocation: this runs for every cell of the mushroom and the sprites).
        let byte_at =
            |s: &str, n: i32| s.char_indices().nth(n as usize).map_or(s.len(), |(i, _)| i);
        let (s, x) = if x < 0 {
            (&s[byte_at(s, -x)..], 0)
        } else {
            (s, x)
        };
        let s = &s[..byte_at(s, self.w - x)];
        if !s.is_empty() {
            self.scr.put(y, x, s, style);
        }
    }

    fn put_char(&mut self, y: i32, x: i32, ch: char, style: Style) {
        self.put(y, x, ch.encode_utf8(&mut [0; 4]), style);
    }

    pub fn frame(&mut self, y: i32, x: i32, h: i32, w: i32, title: &str, style: Style) {
        if h < 2 || w < 2 {
            return;
        }
        let rule = "─".repeat((w - 2) as usize);
        self.put(y, x, &format!("┌{rule}┐"), style);
        for i in 1..h - 1 {
            self.put(y + i, x, "│", style);
            self.put(y + i, x + w - 1, "│", style);
        }
        self.put(y + h - 1, x, &format!("└{rule}┘"), style);
        if !title.is_empty() {
            self.put(y, x + PANEL_PAD, &format!(" {title} "), style.bold());
        }
    }

    pub fn bar(frac: f64, width: i32, full: &str, empty: &str) -> String {
        let n = ((frac * f64::from(width)).round_ties_even() as i32).clamp(0, width.max(0));
        full.repeat(n as usize) + &empty.repeat((width - n).max(0) as usize)
    }

    // --- Rendering ---------------------------------------------------------------------

    pub fn draw(&mut self, dt: f64) {
        self.t += dt;
        self.play_sfx();
        (self.h, self.w) = self.scr.size();
        self.scr.erase();
        if self.w < MIN_W || self.h < MIN_H {
            let ui = &tr().ui;
            let size = (ui.too_small)(self.w, self.h, MIN_W, MIN_H);
            self.put(0, 0, &size, Style::PLAIN);
            let state = (ui.too_small_state)(self.g.level, &game::fmt(self.g.radiation));
            self.put(1, 0, &state, Style::PLAIN);
            self.scr.refresh();
            return;
        }
        let left_w = LEFT_MIN_W
            .max((f64::from(self.w) * LEFT_SHARE) as i32)
            .max(self.w - RIGHT_MAX_W);
        let right_w = self.w - left_w;
        let stats_h = STATS_H;
        let news_y = self.h - 1;
        self.draw_mushroom(0, 0, news_y, left_w, dt);
        self.draw_stats(0, left_w, stats_h, right_w);
        self.draw_tab(stats_h, left_w, news_y - stats_h, right_w);
        self.draw_news(news_y, dt);
        self.draw_qte(0, 0, news_y, left_w);
        self.draw_mutation();
        self.draw_chest();
        self.draw_report();
        self.scr.refresh();
        self.update_title();
    }

    /// Pending challenge story (start or success): full screen, shown once.
    pub fn show_story(&mut self) -> Result<(), Interrupted> {
        if let Some((id, part)) = self.g.pending_story.take()
            && let Some(pages) = story(id, part)
        {
            intro(
                &mut self.scr,
                self.sound.as_deref(),
                pages,
                self.story_guard,
                tr().screens.resume,
            )?;
        }
        Ok(())
    }

    fn sfx(&mut self, name: &'static str) {
        self.g.sfx.push_back(name);
    }

    pub fn music_track(&self) -> &'static str {
        let g = &self.g;
        if g.inspector.is_some()
            || !g.enemies.is_empty()
            || g.qte.is_some()
            || g.overheated()
            || g.frozen()
        {
            return "tension";
        }
        if g.challenge.is_some()
            || (g.exploded && f64::from(g.level) >= f64::from(g.cap()) / EPIC_CAP_DIV)
        {
            return "epic";
        }
        "calm"
    }

    /// Drains g.sfx: audio settings copied to the sound output, screen shake, one play per sound and frame,
    /// at most one per SFX_GAP.
    pub fn play_sfx(&mut self) {
        if let Some(sound) = &self.sound {
            sound.set_muted(self.g.audio == "off");
            sound.set_music_on(self.g.audio == "all");
            sound.set_volume(self.g.volume as f32 / 100.0);
            sound.set_music(self.music_track());
        }
        let mut played: Vec<&'static str> = Vec::new();
        while let Some(name) = self.g.sfx.pop_front() {
            if let Some(d) = shake(name) {
                self.shake_until = self.shake_until.max(self.t + d);
            }
            let key = if name.starts_with("click") {
                "click"
            } else {
                name
            }; // press variants share one delay
            let last = self.sfx_last.get(key).copied().unwrap_or(LONG_AGO);
            let too_soon = self.t - last < sfx_gap(key);
            let Some(sound) = &self.sound else { continue };
            if played.contains(&key) || too_soon {
                continue;
            }
            played.push(key);
            self.sfx_last.insert(key, self.t);
            sound.play(name);
        }
    }

    pub fn draw_mushroom(&mut self, y0: i32, x0: i32, h: i32, w: i32, dt: f64) {
        let mut border = self.c("grey");
        if self.g.overheated() && self.blink(BLINK_OVERHEAT) {
            border = self.c("red").bold();
        } else if self.g.frozen() {
            border = self.c("blue").bold();
        }
        self.frame(y0, x0, h, w, tr().ui.mushroom_title, border);
        let (iy, mut ix, ih, mut iw) = (y0 + 1, x0 + 1, h - 2, w - 2);
        let ground = ih - 1;

        if self.g.time < self.g.flash_until {
            for y in 0..ih {
                let line: String = (0..iw)
                    .map(|_| *self.rng.choice(&['█', '▓', '▒', '░']))
                    .collect();
                let style = if self.blink(BLINK_FLASH) {
                    self.c("flash")
                } else {
                    self.c("white").bold()
                };
                self.put(iy + y, ix, &line, style);
            }
            self.draw_banners(iy, ix, iw, None);
            return;
        }

        if self.t < self.shake_until && iw > SHAKE_MIN_W {
            let shift = *self.rng.choice(&[0, 1]);
            ix += shift;
            iw -= 1; // the lost width keeps the shift inside the frame
        }
        self.put(
            iy + ground,
            ix,
            &"▁".repeat(iw.max(0) as usize),
            self.c("grey"),
        );
        if !self.g.exploded {
            for (y, x, ch, col) in art::bomb(iw, ih) {
                self.put_char(iy + y, ix + x, ch, Style::of(col));
            }
            if self.blink(BLINK_PROMPT) {
                let msg = tr().ui.press_space;
                let style = self.c("yellow").bold();
                self.put(iy + ih / 2, ix + fdiv(iw - width(msg), 2), msg, style);
            }
            self.draw_banners(iy, ix, iw, None);
            return;
        }

        self.draw_sky(iy, ix, iw, ground);
        let size = art::size_fraction(f64::from(self.g.level), f64::from(self.g.cap()));
        let twin = self.g.has_mut("twin");
        for (y, x, ch, col) in art::mushroom(iw, ih, size, self.t, self.g.skin, twin) {
            if 0 <= y && y < ih && 0 <= x && x < iw {
                self.put_char(iy + y, ix + x, ch, Style::of(col));
            }
        }

        let foot_w = art::mushroom_width(iw, ih, size, twin);
        let gap = STEM_GAP_MIN.max(foot_w / STEM_GAP_DIV);
        self.draw_army(iy, ix, iw, ground, ARMY_GAP_MIN.max(foot_w / ARMY_GAP_DIV));
        let banners = self.banner_lines(iw);
        self.update_particles(
            iy,
            ix,
            iw,
            ih,
            ground,
            size,
            gap,
            dt,
            1 + banners.len() as i32,
        );
        self.draw_challenge_monsters(iy, ix, iw, ground);
        self.draw_inspector(iy, ix, iw, ground, (iw - INSPECTOR_START).min(iw / 2 + gap));
        self.draw_enemy(iy, ix, iw, ground, gap);
        self.draw_plane(iy, ix, iw);
        self.draw_banners(iy, ix, iw, Some(banners));
    }

    /// Title in big letters (3 lines of equal width), or in spaced letters if it does not fit.
    pub fn big_lines(&self, title: &str, kind: MsgKind, iw: i32, flashing: bool) -> Vec<Line> {
        let mut style = Style::of(kind.color()).bold();
        if flashing && self.blink(BLINK_BIG_TITLE) {
            style = self.c("white").bold();
        }
        let rows = art::big_text(title);
        let w = rows.iter().map(|r| width(r)).max().unwrap_or(0);
        if w == 0 {
            return vec![(head(title, iw - 2), style)];
        }
        if w > iw - 2 {
            let spaced: Vec<String> = art::plain(title).chars().map(String::from).collect();
            return vec![(spaced.join(" "), style)];
        }
        rows.iter()
            .map(|r| (format!("{r:<0$}", w as usize), style))
            .collect()
    }

    /// Backdrop that evolves with the level: clouds, then stars, then planets (fixed positions per size).
    pub fn draw_sky(&mut self, iy: i32, ix: i32, iw: i32, ground: i32) {
        let (lvl, top) = (self.g.level, ground - SKY_GROUND_MARGIN);
        if top < SKY_MIN_H || iw < SKY_MIN_W {
            return;
        }
        let mut rng = StdRandom::seeded((iw * SKY_SEED_STRIDE + ground) as u64);
        let grey = self.c("grey");
        if lvl < SKY_STARS_LEVEL {
            for i in 0..1.max(iw / CLOUD_SPACING) {
                let y = rng.randint(1, i64::from(top)) as i32;
                let x0 = rng.randint(0, i64::from(iw));
                let x = ((x0 as f64 + self.t * (CLOUD_SPEED + CLOUD_SPEED_STEP * f64::from(i)))
                    as i64)
                    .rem_euclid(i64::from(iw + WRAP_MARGIN)) as i32
                    - WRAP_MARGIN;
                let cloud = slice(CLOUD, (-x).max(0), width(CLOUD).min(iw - x).max(0));
                self.put(iy + y, ix + x.max(0), &cloud, grey);
            }
            return;
        }
        for i in 0..iw * top / STAR_DENSITY {
            let y = rng.randint(1, i64::from(top)) as i32;
            let x = rng.randint(0, i64::from(iw - 1)) as i32;
            let ch = if i % BIG_STAR_EVERY == 0 { "✦" } else { "·" };
            let style = if (i64::from(i) + (self.t * TWINKLE_SPEED) as i64) % TWINKLE_EVERY == 0 {
                self.c("white")
            } else {
                grey
            };
            self.put(iy + y, ix + x, ch, style);
        }
        if lvl >= SKY_PLANETS_LEVEL {
            for (ch, col) in [("◯", "cyan"), ("♄", "yellow"), ("☾", "white")] {
                let y = rng.randint(1, i64::from(top)) as i32;
                let x = rng.randint(0, i64::from(iw - 1)) as i32;
                let style = self.c(col).bold();
                self.put(iy + y, ix + x, ch, style);
            }
        }
    }

    /// Level in the terminal title, at most once per second and only when it changes.
    pub fn update_title(&mut self) {
        let Some(title) = &self.title else { return };
        if self.t - self.title_at < TITLE_INTERVAL {
            return;
        }
        let text = (tr().ui.term_title)(&self.num(f64::from(self.g.level)));
        if text != *title {
            self.scr.set_title(&text);
            self.title = Some(text);
            self.title_at = self.t;
        }
    }

    pub fn draw_banners(&mut self, iy: i32, ix: i32, iw: i32, lines: Option<Vec<Line>>) {
        let lines = lines.unwrap_or_else(|| self.banner_lines(iw));
        for (i, (text, style)) in lines.into_iter().enumerate() {
            let text = head(&format!(" {text} "), iw);
            self.put(
                iy + 1 + i as i32,
                ix + fdiv(iw - width(&text), 2).max(0),
                &text,
                style,
            );
        }
    }

    fn alert_attr(&self, calm: &str) -> Style {
        if self.blink(BLINK_ALERT) {
            self.c("alert")
        } else {
            self.c(calm)
        }
    }

    /// Active alerts then latest messages (most recent on top), stacked under the panel edge.
    pub fn banner_lines(&self, iw: i32) -> Vec<Line> {
        let (g, ui) = (&self.g, &tr().ui);
        let mut lines = Vec::new();
        if g.inspector.is_some() {
            lines.push((ui.inspector_alert.to_string(), self.alert_attr("msg_info")));
        }
        if !g.enemies.is_empty() {
            let who = match g.enemies.as_slice() {
                [one] => one.kind.spec().name().to_string(),
                many => (ui.enemies)(many.len()),
            };
            lines.push(((ui.enemy_alert)(&who), self.alert_attr("msg_bad")));
        }
        if g.frozen() {
            let left = g.freeze_until - g.time;
            lines.push(((ui.frozen_banner)(left), self.c("msg_loot")));
        }
        let recent: Vec<_> = g
            .messages
            .iter()
            .filter(|m| g.time - m.time < MSG_TIME)
            .collect();
        let recent = &recent[recent.len().saturating_sub(MSG_SHOWN)..];
        for (i, m) in recent.iter().rev().enumerate() {
            let kind = m.kind;
            let age = g.time - m.time;
            let important = matches!(kind, MsgKind::Epic | MsgKind::Star);
            if let Some(title) = m.title.as_deref().filter(|t| !t.is_empty())
                && i == 0
                && age
                    < if important {
                        MSG_TIME - BIG_EARLY_EXIT
                    } else {
                        BIG_TIME
                    }
            {
                lines.extend(self.big_lines(title, kind, iw, important && age < BIG_FLASH_TIME));
            }
            let mut style = Style::of(Color::Msg(kind));
            if age < MSG_FLASH_TIME && self.blink(BLINK_FLASH) {
                style = style.toggle_reverse();
            } else if age > MSG_TIME - MSG_FADE {
                style = Style::of(kind.color()).bold();
            }
            let mut parts = wrap(&m.text, MSG_MIN_W.max(iw - MSG_WRAP_MARGIN));
            parts.truncate(MSG_MAX_LINES);
            let w = parts.iter().map(|p| width(p)).max().unwrap_or(0);
            lines.extend(parts.iter().map(|p| (center(p, w), style)));
        }
        lines
    }

    /// Sprite whose last line is at `bottom`, clipped to the panel edges.
    /// Opaque: inner spaces erase the backdrop (readable in front of the dust).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_sprite(
        &mut self,
        iy: i32,
        ix: i32,
        iw: i32,
        bottom: i32,
        x: i32,
        rows: &[String],
        style: Style,
        opaque: bool,
    ) {
        let top = bottom - rows.len() as i32 + 1;
        for (i, line) in rows.iter().enumerate() {
            let y = top + i as i32;
            let body = if opaque { width(line.trim()) } else { 0 };
            let indent = width(line) - width(line.trim_start());
            for (j, ch) in line.chars().enumerate() {
                let j = j as i32;
                let inside = opaque && body > 0 && indent <= j && j < indent + body;
                if (ch != ' ' || inside) && 0 <= x + j && x + j < iw && y >= 0 {
                    self.put_char(iy + y, ix + x + j, ch, style);
                }
            }
        }
    }

    /// One sprite per owned unit type, a 2nd from 10 units and a 3rd from 100 if there is room.
    /// Placed from the edges toward the center, leaving the stem clear; those on the right face left.
    /// Bombers fly separately.
    pub fn draw_army(&mut self, iy: i32, ix: i32, iw: i32, ground: i32, gap: i32) {
        let owned: Vec<_> = UNITS
            .iter()
            // A unit crossing the field as a challenge monster is not drawn in the army as well.
            .filter(|u| {
                self.g.units[u.id] > 0
                    && u.id != "bomber"
                    && !u.challenge.is_some_and(|(c, _)| self.g.in_challenge(c))
            })
            .collect();
        let wanted: Vec<_> = (0..ARMY_COPIES)
            .flat_map(|copy| owned.iter().map(move |u| (*u, copy)))
            .filter(|(u, copy)| self.g.units[u.id] >= ARMY_COPY_BASE.pow(*copy))
            .collect();
        let mut bottoms = vec![ground - 1];
        if ground >= ARMY_UPPER_MIN {
            bottoms.push(ground - ARMY_UPPER_ROW);
        }
        // (bottom line, on the right side)
        let slots: Vec<(i32, bool)> = bottoms
            .iter()
            .flat_map(|&b| [(b, false), (b, true)])
            .collect();
        let mut cursor: Vec<i32> = slots
            .iter()
            .map(|&(_, right)| if right { iw - 1 } else { 1 })
            .collect();
        let mut next_slot = 0;
        for (u, copy) in wanted {
            let frames = art::unit_art(u.id).expect("every unit has art");
            let w = art::sprite_width(frames);
            for tries in 0..slots.len() {
                let k = (next_slot + tries) % slots.len();
                let (bottom, right) = slots[k];
                let x = if right { cursor[k] - w } else { cursor[k] };
                if (!right && x + w <= iw / 2 - gap) || (right && x >= iw / 2 + gap) {
                    cursor[k] = if right { x - 1 } else { x + w + 1 };
                    let rows =
                        art::sprite(frames, self.t + f64::from(copy) * ARMY_COPY_PHASE, right);
                    let style = self.c(u.color).bold();
                    self.draw_sprite(iy, ix, iw, bottom, x, &rows, style, true);
                    next_slot = (next_slot + tries + 1) % slots.len();
                    break;
                }
            }
        }
        let frames = art::unit_art("bomber").expect("bombers have art");
        for i in 0..self.g.units["bomber"].min(BOMBERS_SHOWN) as i32 {
            let x = ((self.t * BOMBER_SPEED + f64::from(i * BOMBER_SPACING)) as i64)
                .rem_euclid(i64::from(iw + WRAP_MARGIN)) as i32
                - WRAP_MARGIN;
            let rows = art::sprite(frames, self.t, false);
            let style = self.c("cyan").bold();
            self.draw_sprite(
                iy,
                ix,
                iw,
                BOMBER_TOP + BOMBER_ROW_STEP * i,
                x,
                &rows,
                style,
                false,
            );
        }
    }

    /// Unit projectiles toward the mushroom, then floating numbers above its head.
    #[allow(clippy::too_many_arguments)]
    pub fn update_particles(
        &mut self,
        iy: i32,
        ix: i32,
        iw: i32,
        ih: i32,
        ground: i32,
        size: f64,
        gap: i32,
        dt: f64,
        min_row: i32,
    ) {
        let top = ground - art::mushroom_height(ih, size);
        if !self.g.frozen() {
            for u in &UNITS {
                let n = self.g.units[u.id];
                if n == 0 {
                    continue;
                }
                let per_second = (PARTICLE_RATE_BASE + (1.0 + n as f64).log2() * PARTICLE_RATE_LOG)
                    .min(PARTICLE_RATE_MAX);
                let mut acc = self.spawn_acc.get(u.id).copied().unwrap_or(0.0) + dt * per_second;
                while acc >= 1.0 && self.particles.len() < MAX_PARTICLES {
                    acc -= 1.0;
                    let rng = &mut self.rng;
                    let (sx, sy, peak) = if u.id == "bomber" {
                        (rng.randint(0, i64::from(iw - 1)), PARTICLE_SKY_ROW, 0.0)
                    } else {
                        let left = rng.random() < 0.5;
                        let sx = if left {
                            rng.randint(1, i64::from(iw / 2 - gap))
                        } else {
                            rng.randint(i64::from(iw / 2 + gap), i64::from(iw - 2))
                        };
                        (
                            sx,
                            ground - 1,
                            rng.uniform(PARTICLE_PEAK_MIN, PARTICLE_PEAK_MAX),
                        )
                    };
                    let tx = iw / 2 + rng.randint(-PARTICLE_SPREAD, PARTICLE_SPREAD) as i32;
                    let ty = rng
                        .randint(i64::from((top + 1).min(ground - 2)), i64::from(ground - 2))
                        as i32;
                    let dur = rng.uniform(PARTICLE_DUR_MIN, PARTICLE_DUR_MAX);
                    self.particles.push(Particle {
                        sx: sx as i32,
                        sy,
                        tx,
                        ty,
                        peak,
                        age: 0.0,
                        dur,
                        ch: u.proj,
                        col: u.color,
                    });
                }
                self.spawn_acc.insert(u.id, acc.min(1.0));
            }
        }
        let mut alive = Vec::new();
        for mut p in std::mem::take(&mut self.particles) {
            p.age += dt;
            let flight = p.age / p.dur;
            if flight >= IMPACT_END {
                continue;
            }
            if flight >= 1.0 {
                let style = self.c("yellow").bold();
                self.put(iy + p.ty, ix + p.tx, "✸", style);
            } else {
                let x = (f64::from(p.sx) + f64::from(p.tx - p.sx) * flight) as i32;
                let arc = 4.0 * p.peak * flight * (1.0 - flight);
                let y = (f64::from(p.sy) + f64::from(p.ty - p.sy) * flight - arc) as i32;
                if 0 <= y && y < ih && 0 <= x && x < iw {
                    let style = self.c(p.col).bold();
                    self.put(iy + y, ix + x, p.ch, style);
                }
            }
            alive.push(p);
        }
        self.particles = alive;

        let mut floats = Vec::new();
        for mut fl in std::mem::take(&mut self.floats) {
            fl.age += dt;
            if fl.age < FLOAT_TIME {
                let y = (f64::from(top) - 1.0 - fl.age * FLOAT_SPEED) as i32;
                let len = width(&fl.text);
                let x = iw / 2 + fl.dx - len / 2;
                self.put(
                    iy + min_row.max(y),
                    ix + (iw - len).min(x).max(0),
                    &fl.text,
                    fl.style,
                );
                floats.push(fl);
            }
        }
        self.floats = floats;
    }

    pub fn draw_plane(&mut self, iy: i32, ix: i32, iw: i32) {
        let Some(p) = self.g.plane else { return };
        let progress = (self.g.time - p.start) / p.dur;
        let x = (f64::from(iw) - progress * f64::from(iw + PLANE_EXIT)) as i32;
        let style = self.c("white").bold();
        self.put(iy, ix + x.max(0), "✈", style);
        let label = format!("[{}]", p.letter.to_uppercase());
        let style = if self.blink(BLINK_PLANE) {
            self.c("yellow")
        } else {
            self.c("cyan")
        }
        .bold();
        let lx = x + PLANE_LABEL_GAP;
        if 0 <= lx && lx <= iw - width(&label) {
            self.put(iy, ix + lx, &label, style);
        }
    }

    pub fn draw_inspector(&mut self, iy: i32, ix: i32, iw: i32, ground: i32, target: i32) {
        let Some(ins) = self.g.inspector else { return };
        let progress = ((self.g.time - ins.start) / ins.dur).min(1.0);
        let start = iw - INSPECTOR_START;
        let x = (f64::from(start) - f64::from(start - target) * progress) as i32;
        let sign = tr().ui.inspector_sign;
        let x = x.min(iw - width(sign)).max(0);
        let style = self.c("magenta").bold();
        let arms = if self.blink(BLINK_ALERT) {
            " o/"
        } else {
            "\\o "
        };
        self.put(iy + ground - 2, ix + x + 1, arms, style);
        self.put(iy + ground - 1, ix + x, sign, style);
        let left = events::inspector_hits(&self.g).saturating_sub(ins.hits);
        let hits = "x".repeat(ins.hits as usize) + &"·".repeat(left as usize);
        let style = self.c("red");
        self.put(iy + ground - 3, ix + x, &hits, style);
    }

    /// During a challenge that unlocks a unit (moussaka, nanars), its monster crosses the field from right to
    /// left, again and again, in front of the army.
    pub fn draw_challenge_monsters(&mut self, iy: i32, ix: i32, iw: i32, ground: i32) {
        let monsters = self.g.active_challenges().into_iter();
        for (i, u) in monsters.filter_map(data::challenge_unit).enumerate() {
            let frames = art::unit_art(u.id).expect("every unit has art");
            let w = art::sprite_width(frames);
            // A double challenge with two monsters: the second one half a crossing behind.
            let lap = (self.t / MONSTER_CROSSING + i as f64 * 0.5).fract();
            let x = iw - (lap * f64::from(iw + w)) as i32;
            let rows = art::sprite(frames, self.t, true);
            let style = self.c(u.color).bold();
            self.draw_sprite(iy, ix, iw, ground - 1, x, &rows, style, true);
        }
    }

    /// Enemies coming from the left (drones through the sky), remaining hits above each one.
    /// The front one stops at the foot of the mushroom, the others queue up behind it.
    pub fn draw_enemy(&mut self, iy: i32, ix: i32, iw: i32, ground: i32, gap: i32) {
        for sky in [false, true] {
            let mut target = iw / 2 - gap;
            let mut enemies: Vec<_> = self
                .g
                .enemies
                .iter()
                .filter(|e| e.kind.spec().sky == sky)
                .map(|e| (*e, events::enemy_progress(&self.g, e)))
                .collect();
            enemies.sort_by(|a, b| b.1.total_cmp(&a.1));
            for (en, progress) in enemies {
                let frames = art::enemy_art(en.kind.id()).expect("every enemy has art");
                let w = art::sprite_width(frames);
                target = (target - w).max(0);
                let mut progress = progress.clamp(0.0, 1.0);
                if en.kind == events::EnemyKind::Hippies {
                    progress = (progress * HIPPIES_RUSH).min(1.0);
                }
                let x = (f64::from(-w) + f64::from(target + w) * progress) as i32;
                target = target.min(x) - 1;
                let rows = art::sprite(frames, self.t, false);
                let bottom = if sky {
                    DRONE_ALTITUDE + rows.len() as i32
                } else {
                    ground - 1
                };
                let style = self.c("red").bold();
                self.draw_sprite(iy, ix, iw, bottom, x, &rows, style, true);
                if x + w > 0 {
                    let left = events::enemy_hits(&self.g, en.kind).saturating_sub(en.hits);
                    let hits = "x".repeat(en.hits as usize) + &"·".repeat(left as usize);
                    let style = self.c("yellow");
                    self.put(iy + bottom - rows.len() as i32, ix + x.max(0), &hits, style);
                }
            }
        }
    }

    /// Formatted number, hidden during the Fog of War challenge.
    pub fn num(&self, x: f64) -> String {
        if self.g.in_challenge("blind") {
            tr().hidden.to_string()
        } else {
            game::fmt(x)
        }
    }

    pub fn draw_stats(&mut self, y0: i32, x0: i32, h: i32, w: i32) {
        let grey = self.c("grey");
        let ui = &tr().ui;
        self.frame(y0, x0, h, w, ui.stats_title, grey);
        let (iy, ix, iw) = (y0 + 1, x0 + PANEL_PAD, w - 2 * PANEL_PAD);
        let g = &self.g;
        let mut lines: Vec<Vec<Line>> = Vec::new();

        let head_text = (ui.level)(
            &self.num(f64::from(g.level)),
            &game::fmt(f64::from(g.cap())),
        );
        let mut tag = String::new();
        if let Some(name) = g.challenge_label() {
            tag = (ui.challenge_tag)(&name);
            if let Some(time) = g.run_time_limit() {
                let left = (time - g.run_time).max(0.0);
                tag += &format!(
                    " {:.0}:{:02.0}",
                    (left / 60.0).floor(),
                    left.rem_euclid(60.0)
                );
            }
        } else if g.prestige_count > 0 {
            tag = (ui.prestige_tag)(g.prestige_count);
        }
        let pad = " ".repeat(1.max(iw - width(&head_text) - width(&tag)) as usize);
        lines.push(vec![
            (head_text, self.c("white").bold()),
            (pad, Style::PLAIN),
            (tag, self.c("magenta")),
        ]);

        if g.level >= g.cap() {
            let style = if self.blink(BLINK_LEVEL_MAX) {
                self.c("green").bold()
            } else {
                self.c("green")
            };
            lines.push(vec![(ui.level_max.to_string(), style)]);
        } else {
            let frac = if g.exploded {
                g.progress / need(g.level.max(1))
            } else {
                0.0
            };
            let shown = if g.in_challenge("blind") {
                tr().hidden.to_string()
            } else {
                percent((frac * 100.0) as i64)
            };
            // Right-aligned on the width of 100 %, so that the line doesn't move.
            let full = percent_width();
            let percent = format!(" {shown:>w$}", w = full as usize);
            lines.push(vec![
                (
                    // Room on the right for the percentage and its space.
                    Self::bar(frac, iw - 1 - full, "█", "░"),
                    self.c("orange"),
                ),
                (percent, Style::PLAIN),
            ]);
        }

        let prod = (ui.production)(&self.num(g.production()));
        let (state, state_style) = if g.frozen() {
            (ui.frozen.to_string(), self.c("blue"))
        } else {
            (prod, self.c("green"))
        };
        lines.push(vec![
            (
                format!("☢ {}   ", self.num(g.radiation)),
                self.c("yellow").bold(),
            ),
            (state, state_style),
        ]);
        let auto = if g.tr("t_auto") > 0 {
            format!("   {}", (ui.auto)(g.tr("t_auto")))
        } else {
            String::new()
        };
        let click_stats = (ui.click_stats)(
            &self.num(g.click_value(true)),
            (g.crit_chance() * 100.0) as i64,
        );
        lines.push(vec![(click_stats + &auto, Style::PLAIN)]);
        let stars = if g.supernova_unlocked() {
            format!("   ✦ {}", game::fmt(g.stars as f64))
        } else {
            String::new()
        };
        lines.push(vec![
            ((ui.pt)(g.pt), self.c("cyan").bold()),
            (
                format!("   ⚛ {}", game::fmt(g.isotopes as f64)),
                self.c("magenta").bold(),
            ),
            (
                format!("   ★ {}/{}", g.achievements.len(), ACHIEVEMENTS.len()),
                self.c("yellow"),
            ),
            (stars, self.c("magenta").bold()),
        ]);

        if g.overheated() {
            let style = if self.blink(BLINK_OVERHEAT) {
                self.c("alert")
            } else {
                self.c("red").bold()
            };
            lines.push(vec![((ui.overheat)(g.overheat_until - g.time), style)]);
        } else {
            let steps = (g.max_combo() as i32 - 1).max(1);
            let step_w = 1.max(fdiv(COMBO_BAR_MAX.min(iw - COMBO_LABEL_W), steps));
            let filled = (g.combo * f64::from(step_w)) as i32;
            lines.push(vec![
                ((ui.combo)(g.combo_mult()) + " ", Style::PLAIN),
                ("■".repeat(filled.max(0) as usize), self.c("red").bold()),
                (
                    "□".repeat((steps * step_w - filled).max(0) as usize),
                    self.c("grey"),
                ),
            ]);
        }

        if g.qte.is_some() {
            lines.push(vec![(ui.gauge_running.to_string(), self.c("red").bold())]);
        } else if g.crit_gauge >= 1.0 {
            let style = if self.blink(BLINK_GAUGE_READY) {
                self.c("green").bold()
            } else {
                self.c("yellow")
            };
            lines.push(vec![(ui.gauge_ready.to_string(), style)]);
        } else {
            lines.push(vec![
                (format!("{} ", ui.gauge), Style::PLAIN),
                (
                    // Room for the label, then the percentage, each followed or preceded by a space.
                    Self::bar(
                        g.crit_gauge,
                        GAUGE_MIN_W.max(iw - width(ui.gauge) - 2 - percent_width()),
                        "█",
                        "░",
                    ),
                    self.c("red"),
                ),
                (
                    format!(" {}", percent((g.crit_gauge * 100.0) as i64)),
                    Style::PLAIN,
                ),
            ]);
        }

        if !g.buffs.is_empty() {
            let txt: Vec<String> = g
                .buffs
                .iter()
                .map(|b| (ui.buff)(&events::buff_name(b.kind, b.mult), b.until - g.time))
                .collect();
            lines.push(vec![(txt.join("  "), self.c("yellow").bold())]);
        } else if !g.mutations.is_empty() {
            let names: Vec<&str> = g
                .mutations
                .iter()
                .map(|m| events::mutation(m).name())
                .collect();
            lines.push(vec![(
                (ui.mutations)(&names.join(tr().list_join)),
                self.c("green"),
            )]);
        }

        lines.truncate((h - 2).max(0) as usize);
        for (i, segs) in lines.into_iter().enumerate() {
            let mut x = ix;
            for (text, style) in segs {
                let text = head(&text, (ix + iw - x).max(0));
                self.put(iy + i as i32, x, &text, style);
                x += width(&text);
            }
        }
    }

    // --- Tabs ----------------------------------------------------------------------------

    pub fn tab_has_news(&self, i: usize) -> bool {
        let g = &self.g;
        match i {
            0 => UNITS.iter().any(|u| g.can_buy_unit(u.id)),
            1 => data::TALENTS.iter().any(|t| {
                g.talent_available(t.id) && g.talent_allowed(t.id) && g.pt >= g.talent_cost(t.id)
            }),
            2 => {
                g.can_prestige()
                    || g.can_supernova()
                    || (g.prestige_unlocked()
                        && data::PTALENTS
                            .iter()
                            .any(|p| g.isotopes as f64 >= g.ptalent_cost(p.id)))
                    || data::STALENTS
                        .iter()
                        .any(|s| g.stars as f64 >= g.stalent_cost(s.id))
            }
            _ => false,
        }
    }

    pub fn rows(&self) -> Vec<Row> {
        match self.tab {
            0 => self.unit_rows(),
            1 => self.talent_rows(),
            2 => self.prestige_rows(),
            3 => self.achievement_rows(),
            _ => self.info_rows(),
        }
    }

    /// Challenge row: ranks done, next rank (if the max level allows it) or all ranks done.
    fn challenge_row(&self, c: &'static data::Challenge) -> Row {
        let (g, ui) = (&self.g, &tr().ui);
        let done = g.challenge_rank(c.id);
        let all = data::CHALLENGE_RANKS;
        let cur = if g.in_challenge(c.id) {
            ui.in_progress
        } else {
            ""
        };
        let mark = if done == all { '✔' } else { '·' };
        let picked = if self.double_pick == Some(c.id) {
            "◆ "
        } else {
            ""
        };
        let text = format!(
            "{mark} {picked}{}{cur}",
            (ui.challenge)(c.name(), done, all)
        );
        let reward = (ui.reward)(&c.reward_text());
        if done == all {
            return Row::new(text, self.c("green"), None, (ui.all_ranks_done)(&reward));
        }
        let rank = data::RANK_NAMES[done as usize];
        if self.double && data::DOUBLE_SOLO.contains(&c.id) {
            return Row::new(text, self.c("grey"), None, (ui.solo_only)(&reward));
        }
        if !g.challenge_available(c.id) {
            let needed = crate::game::fmt(g.challenge_target(c.id) as f64);
            return Row::new(
                text,
                self.c("grey"),
                None,
                (ui.rank_locked)(rank, &needed, data::ptalent("p_cap").name(), &reward),
            );
        }
        let note = if g.can_prestige() {
            ui.note_prestige
        } else {
            ui.note_no_gain
        };
        Row::new(
            text,
            if done > 0 {
                self.c("yellow")
            } else {
                self.c("white")
            },
            Some(Action::Challenge(c.id)),
            (ui.rank_desc)(rank, &c.desc_at(done), &reward, note),
        )
    }

    pub fn unit_rows(&self) -> Vec<Row> {
        let (g, ui) = (&self.g, &tr().ui);
        let mut rows = Vec::new();
        for c in g.active_challenges() {
            let warning = match c {
                "moussaka" => Some((ui.moussaka)(g.moussaka_size())),
                "pacifist" => Some(ui.pacifist.to_string()),
                "mono" => Some(ui.mono.to_string()),
                "strike" if g.on_strike() => Some(ui.strike.to_string()),
                "savior" => Some(ui.savior.to_string()),
                "devil" if g.devil_waiting() => Some(ui.devil.to_string()),
                _ => None,
            };
            if let Some(text) = warning {
                rows.push(Row::label(text, self.c("magenta")));
            }
        }
        let nw = UNIT_NAME_MIN_W.max(self.tab_w - UNIT_COLUMNS_W);
        let mut teased = false;
        for u in &UNITS {
            let uid = u.id;
            if g.unit_unlocked(uid) {
                let n = g.units[uid];
                let k = match self.qty {
                    Qty::Max => 1.max(g.affordable(uid)),
                    Qty::N(k) => k.min(g.max_bulk()),
                };
                let cost = g.unit_cost_n(uid, k);
                let text = format!(
                    "{:<nw$} x{n:<4} {:>7}{} {:>7} ☢",
                    head(u.name(), nw),
                    self.num(g.unit_prod(uid) * n as f64),
                    ui.per_second,
                    game::fmt(cost),
                    nw = nw as usize
                );
                let ok = g.can_buy_unit(uid) && g.radiation >= cost;
                let style = if ok {
                    self.c("white").bold()
                } else {
                    self.c("grey")
                };
                let mut desc = (ui.unit_desc)(u.name(), &self.num(g.unit_prod(uid)), k);
                if let Some(nxt) = g.next_milestone(uid) {
                    desc += &(ui.milestone)(nxt, &i18n::num(g.milestone_mult()));
                }
                if !ok && g.unit_allowed(uid) && !g.in_challenge("blind") {
                    let prod = g.production();
                    desc += &if prod > 0.0 {
                        (ui.wait)(&dur((cost - g.radiation) / prod))
                    } else {
                        (ui.missing)(&game::fmt(cost - g.radiation))
                    };
                }
                rows.push(
                    Row::new(text, style, Some(Action::Unit(uid)), desc).with_sprite(uid, true),
                );
            } else if !teased {
                teased = true;
                let (text, desc) = match u
                    .challenge
                    .map(|(c, _)| c)
                    .filter(|&c| g.challenge_rank(c) == 0)
                {
                    Some(c) => (
                        ui.teaser_challenge.to_string(),
                        (ui.teaser_challenge_desc)(data::challenge(c).name()),
                    ),
                    None => (
                        (ui.teaser_talent)(u.unlock_name()),
                        ui.teaser_talent_desc.to_string(),
                    ),
                };
                rows.push(Row::new(text, self.c("grey"), None, desc).with_sprite(uid, false));
            }
        }
        rows
    }

    pub fn talent_rows(&self) -> Vec<Row> {
        let (g, ui) = (&self.g, &tr().ui);
        let mut rows = vec![Row::label((ui.talent_points)(g.pt), self.c("cyan").bold())];
        for bid in data::BRANCHES {
            let label = tr().data.branch(bid);
            rows.push(Row::label(
                format!("── {label} {}", "─".repeat(RULE)),
                self.c("cyan"),
            ));
            for t in data::TALENTS.iter().filter(|t| t.branch == bid) {
                let (tid, r, avail) = (t.id, g.tr(t.id), g.talent_available(t.id));
                if bid == "inf" && r == 0 && !avail {
                    continue;
                }
                let cost = if r >= t.max {
                    ui.max_rank.to_string()
                } else {
                    (ui.pt_cost)(g.talent_cost(tid))
                };
                let indent = if t.depth > 0 {
                    "  ".repeat(t.depth as usize - 1) + "└ "
                } else {
                    String::new()
                };
                let name = indent + t.name();
                let nw = NAME_MIN_W.max(self.tab_w - TALENT_COLUMNS_W);
                let text = format!(
                    "{:<nw$} {r}/{:<2} {cost:>6}",
                    head(&name, nw),
                    t.max,
                    nw = nw as usize
                );
                let style = if r >= t.max {
                    self.c("green")
                } else if !g.talent_allowed(tid) {
                    self.c("grey")
                } else if avail && g.pt >= g.talent_cost(tid) {
                    self.c("yellow").bold()
                } else if avail {
                    self.c("white")
                } else {
                    self.c("grey")
                };
                let mut desc = format!("{}{}", t.desc(), self.qty_note());
                if !g.talent_allowed(tid) {
                    desc = (ui.ascete)(&desc);
                }
                if !avail && r < t.max {
                    let reqs: Vec<String> = t
                        .req
                        .iter()
                        .map(|(q, n)| format!("{} {n}", data::talent(q).name()))
                        .collect();
                    desc += &(ui.requires)(&reqs.join(tr().list_join));
                }
                rows.push(Row::new(text, style, Some(Action::Talent(tid)), desc));
            }
        }
        rows
    }

    pub fn prestige_rows(&self) -> Vec<Row> {
        let (g, ui) = (&self.g, &tr().ui);
        let cap = game::fmt(f64::from(g.cap()));
        if !g.prestige_unlocked() {
            let grey = self.c("grey");
            let mut rows = vec![
                Row::label(ui.locked, self.c("red").bold()),
                Row::label((ui.locked_reach)(&cap), grey),
            ];
            rows.extend(ui.locked_lines.iter().map(|line| Row::label(*line, grey)));
            return rows;
        }
        let mut rows = Vec::new();
        if g.can_prestige() {
            rows.push(Row::new(
                (ui.prestige_button)(g.prestige_gain()),
                self.c("green").bold(),
                Some(Action::Prestige),
                (ui.prestige_desc)(
                    ui.reset_desc,
                    &i18n::num((g.surplus_mult() * 100.0).floor() / 100.0),
                    &i18n::num(crate::game::SURPLUS_BONUS),
                ),
            ));
        } else {
            rows.push(Row::new(
                (ui.prestige_reach)(&cap),
                self.c("grey"),
                None,
                ui.reset_desc,
            ));
        }
        if g.prestige_count > 0 {
            rows.push(Row::label(
                format!("── {} {}", ui.challenges_rule, "─".repeat(RULE)),
                self.c("cyan"),
            ));
            if g.double_unlocked() {
                let style = if self.double {
                    self.c("yellow")
                } else {
                    self.c("white")
                };
                rows.push(Row::new(
                    (ui.double_mode)(self.double),
                    style,
                    Some(Action::DoubleMode),
                    ui.double_desc,
                ));
            }
            if g.challenge.is_some() {
                rows.push(Row::new(
                    ui.abandon,
                    self.c("red"),
                    Some(Action::Abandon),
                    ui.abandon_desc,
                ));
            }
            // Challenges done at every rank are folded behind one row (Enter shows them).
            let all_done = |c: &&data::Challenge| g.challenge_rank(c.id) >= data::CHALLENGE_RANKS;
            let done = data::CHALLENGES.iter().filter(all_done).count();
            if done > 0 {
                rows.push(fold_row(
                    self.show_done,
                    &(ui.done_fold)(done),
                    self.c("green"),
                    Action::ToggleDone,
                    ui.done_fold_what,
                ));
            }
            let shown = |c: &&data::Challenge| self.show_done || !all_done(c);
            // Classic challenges first, then those that unlock a unit (their monster), under their own rule.
            let (monsters, classics): (Vec<_>, Vec<_>) = data::CHALLENGES
                .iter()
                .filter(shown)
                .partition(|c| data::challenge_unit(c.id).is_some());
            for c in classics {
                rows.push(self.challenge_row(c));
            }
            // No empty section when all the monster challenges are folded.
            if !monsters.is_empty() {
                rows.push(Row::label(
                    format!("── {} {}", ui.monsters_rule, "─".repeat(RULE)),
                    self.c("cyan"),
                ));
            }
            for c in monsters {
                rows.push(self.challenge_row(c));
            }
        }
        let skin_name = data::skin_name(g.skin);
        rows.push(Row::new(
            (ui.skin)(skin_name, g.skins.len(), data::SKINS.len()),
            self.c("magenta"),
            Some(Action::Skin),
            (ui.skin_desc)(data::PLASMA_PRESTIGES),
        ));
        rows.push(Row::label(
            format!(
                "── {} {}",
                (ui.prestige_tree)(&game::fmt(g.isotopes as f64)),
                "─".repeat(RULE_COUNTED)
            ),
            self.c("cyan"),
        ));
        let max_cap = (ui.max_cap)(&cap);
        rows.extend(self.tree_rows(
            &data::PTALENTS,
            Action::PTalent,
            Game::pr,
            Game::ptalent_cost,
            g.isotopes as f64,
            "⚛",
            &|pid| {
                if pid == "p_cap" {
                    max_cap.clone()
                } else {
                    String::new()
                }
            },
        ));
        if g.supernova_unlocked() {
            rows.extend(self.supernova_rows());
        }
        rows
    }

    pub fn supernova_rows(&self) -> Vec<Row> {
        let (g, ui) = (&self.g, &tr().ui);
        let mut rows = vec![Row::label(
            format!(
                "── {} {}",
                (ui.supernova_tree)(&game::fmt(g.stars as f64)),
                "─".repeat(RULE_COUNTED)
            ),
            self.c("magenta").bold(),
        )];
        let desc = (ui.supernova_desc)(
            data::SUPERNOVA_DIVISOR,
            data::ptalent("p_cap").name(),
            data::stalent("s_keep").name(),
        );
        if g.can_supernova() {
            rows.push(Row::new(
                (ui.supernova_button)(g.supernova_gain()),
                self.c("magenta").bold(),
                Some(Action::Supernova),
                desc,
            ));
        } else {
            rows.push(Row::new(
                (ui.supernova_reach)(&game::fmt(f64::from(data::MAX_CAP))),
                self.c("grey"),
                None,
                desc,
            ));
        }
        rows.extend(self.tree_rows(
            &data::STALENTS,
            Action::STalent,
            Game::sr,
            Game::stalent_cost,
            g.stars as f64,
            "✦",
            &|_| String::new(),
        ));
        rows
    }

    /// Tree paid in isotopes or stars: name, rank, next rank cost; yellow if affordable.
    #[allow(clippy::too_many_arguments)]
    fn tree_rows(
        &self,
        talents: &'static [data::Perk],
        kind: fn(&'static str) -> Action,
        rank: fn(&Game, &str) -> u32,
        cost: fn(&Game, &str) -> f64,
        budget: f64,
        unit: &str,
        extra_desc: &dyn Fn(&str) -> String,
    ) -> Vec<Row> {
        let nw = NAME_MIN_W.max(self.tab_w - TREE_COLUMNS_W);
        talents
            .iter()
            .map(|t| {
                let r = rank(&self.g, t.id);
                let maxed = r >= t.max;
                let c = cost(&self.g, t.id);
                let price = if maxed {
                    tr().ui.max_rank.to_string()
                } else {
                    format!("{} {unit}", game::fmt(c))
                };
                let text = format!(
                    "{:<nw$} {r}/{:<2} {price:>8}",
                    head(t.name(), nw),
                    t.max,
                    nw = nw as usize
                );
                let style = if maxed {
                    self.c("green")
                } else if budget >= c {
                    self.c("yellow").bold()
                } else {
                    self.c("white")
                };
                let desc = format!("{}{}{}", t.desc(), extra_desc(t.id), self.qty_note());
                Row::new(text, style, Some(kind(t.id)), desc)
            })
            .collect()
    }

    pub fn achievement_rows(&self) -> Vec<Row> {
        let (g, ui) = (&self.g, &tr().ui);
        let n = g.achievements.len();
        let mut rows = vec![Row::label(
            (ui.achievements)(
                n,
                ACHIEVEMENTS.len(),
                &i18n::num(data::percent(events::ACHIEVEMENT_BONUS) * n as f64),
            ),
            self.c("yellow").bold(),
        )];
        for a in &ACHIEVEMENTS {
            let done = g.achievements.contains(a.id);
            rows.push(Row::new(
                format!("{} {}", if done { '★' } else { '·' }, a.name()),
                if done {
                    self.c("yellow")
                } else {
                    self.c("grey")
                },
                None,
                a.desc(),
            ));
        }
        let found = &g.synergies_found;
        rows.push(Row::label(
            format!(
                "── {} {}",
                (ui.synergies_rule)(found.len(), SYNERGIES.len()),
                "─".repeat(RULE_COUNTED)
            ),
            self.c("magenta"),
        ));
        for s in &SYNERGIES {
            let pair = format!(
                "{} + {}",
                events::mutation(s.pair.0).name(),
                events::mutation(s.pair.1).name()
            );
            if found.contains(s.id) {
                let active = if g.has_syn(s.id) {
                    ui.synergy_active
                } else {
                    ""
                };
                rows.push(Row::new(
                    format!("✦ {}{active}", s.name()),
                    self.c("magenta").bold(),
                    None,
                    (ui.synergy_desc)(&pair, &s.effect()),
                ));
            } else {
                rows.push(Row::new(
                    format!("· {}", tr().hidden),
                    self.c("grey"),
                    None,
                    ui.synergy_unknown_desc,
                ));
            }
        }
        rows
    }

    pub fn qty_note(&self) -> String {
        match self.qty {
            Qty::N(1) => String::new(),
            Qty::Max => tr().ui.qty_all.to_string(),
            Qty::N(k) => (tr().ui.qty_up_to)(k),
        }
    }

    pub fn info_rows(&self) -> Vec<Row> {
        let (g, st, ui) = (&self.g, &self.g.stats, &tr().ui);
        let (head_style, key, grey) = (
            self.c("cyan").bold(),
            self.c("yellow").bold(),
            self.c("white"),
        );
        let mut rows = vec![Row::label(
            format!("── {} {}", ui.keys_rule, "─".repeat(RULE)),
            head_style,
        )];
        for (k, what) in key_help() {
            rows.push(Row::new(
                clip(&format!("{k:<9} {what}"), self.tab_w - SEL_PREFIX_W),
                key,
                None,
                what,
            ));
        }
        rows.push(Row::label(
            format!("── {} {}", ui.stats_rule, "─".repeat(RULE_STATS)),
            head_style,
        ));
        let count = |n: u64| game::fmt(n as f64);
        let labels = &ui.stats;
        let stats = [
            (labels.total_time, dur(st.total_time)),
            (labels.run_time, dur(g.run_time)),
            (
                labels.best_prestige,
                st.best_prestige_time.map_or("—".to_string(), dur),
            ),
            (labels.prestiges, count(g.prestige_count)),
            (labels.supernovas, count(g.supernova_count)),
            (labels.max_level, self.num(f64::from(st.max_level))),
            (labels.clicks, count(st.clicks)),
            (labels.overheats, count(st.overheats)),
            (labels.perfects, count(st.perfects)),
            (labels.drops, count(st.drops)),
            (labels.inspectors, count(st.inspectors)),
            (labels.enemies, count(st.enemies)),
            (labels.frozen, count(st.frozen)),
            (labels.mutations, count(st.mutations)),
            (
                labels.challenge_ranks,
                format!(
                    "{}/{}",
                    g.challenges_done.values().sum::<u32>(),
                    data::CHALLENGES.len() as u32 * data::CHALLENGE_RANKS
                ),
            ),
            (
                labels.skins,
                format!("{}/{}", g.skins.len(), data::SKINS.len()),
            ),
            (
                labels.sound,
                format!("{} · {}", audio_label(g.audio), percent(g.volume)),
            ),
            (
                labels.synergies,
                format!("{}/{}", g.synergies_found.len(), SYNERGIES.len()),
            ),
        ];
        for (label, value) in stats {
            let vw = STAT_VALUE_MIN_W.max(width(&value));
            let nw = STAT_NAME_MIN_W.max(self.tab_w - STAT_ROW_EXTRA_W - vw);
            let text = format!(
                "{:<nw$} {value:>vw$}",
                head(label, nw),
                nw = nw as usize,
                vw = vw as usize
            );
            rows.push(Row::new(text, grey, None, (ui.label_value)(label, &value)));
        }
        rows.push(Row::new(
            ui.intro,
            self.c("magenta"),
            Some(Action::Intro),
            ui.intro_desc,
        ));
        // The stories of the challenges done, folded behind one row.
        let read: Vec<&data::Challenge> = data::CHALLENGES
            .iter()
            .filter(|c| g.challenges_done.contains_key(c.id))
            .collect();
        if !read.is_empty() {
            rows.push(fold_row(
                self.show_stories,
                &(ui.stories_fold)(read.len()),
                self.c("magenta"),
                Action::ToggleStories,
                ui.stories_fold_what,
            ));
        }
        if self.show_stories {
            for ch in read {
                rows.push(Row::new(
                    format!("  {}", (ui.story)(ch.name())),
                    self.c("magenta"),
                    Some(Action::Story(ch.id)),
                    (ui.story_desc)(ch.name()),
                ));
            }
        }
        if let Some(r) = &g.last_run {
            rows.push(Row::label(
                format!("── {} {}", ui.last_run_rule, "─".repeat(RULE_LAST_RUN)),
                head_style,
            ));
            for (label, value) in report_lines(r) {
                let text = (ui.label_value)(label, &value);
                rows.push(Row::new(
                    clip(&text, self.tab_w - SEL_PREFIX_W),
                    grey,
                    None,
                    text,
                ));
            }
        }
        rows
    }

    pub fn draw_tab(&mut self, y0: i32, x0: i32, h: i32, w: i32) {
        self.tab_w = w - 2 * PANEL_PAD;
        let grey = self.c("grey");
        self.frame(y0, x0, h, w, "", grey);
        self.draw_tab_labels(y0, x0, w);
        self.draw_tab_help(y0 + h - 1, x0, w);
        self.draw_tab_body(y0 + 1, x0 + PANEL_PAD, h - 2, w - 2 * PANEL_PAD);
    }

    /// Tab names on the top edge, shortened until they fit; • if something there is affordable.
    pub fn draw_tab_labels(&mut self, y: i32, x0: i32, w: i32) {
        let mut names: Vec<String> = tr()
            .ui
            .tabs
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if i == PRESTIGE_TAB && !self.g.prestige_unlocked() {
                    tr().hidden.to_string()
                } else {
                    n.to_string()
                }
            })
            .collect();
        if self.qty != Qty::N(1) {
            names[0] += &format!(" ×{}", self.qty);
        }
        let dots: Vec<&str> = (0..names.len())
            .map(|i| if self.tab_has_news(i) { "•" } else { "" })
            .collect();
        let formats: [fn(usize, &str, &str) -> String; 4] = [
            |i, n, dot| format!(" {}:{n}{dot} ", i + 1),
            |i, n, dot| format!("{}:{}{dot}", i + 1, head(n, TAB_LABEL_SHORT)),
            |i, n, dot| format!("{}:{}{dot}", i + 1, head(n, TAB_LABEL_TINY)),
            |i, _, dot| format!("{}{dot}", i + 1),
        ];
        let mut labels = Vec::new();
        for fmt_label in formats {
            labels = names
                .iter()
                .enumerate()
                .map(|(i, n)| fmt_label(i, n, dots[i]))
                .collect();
            if labels.iter().map(|lb| width(lb) + 1).sum::<i32>() < w - PANEL_PAD {
                break;
            }
        }
        let mut x = x0 + PANEL_PAD;
        for (i, label) in labels.iter().enumerate() {
            let style = if i == self.tab {
                Style::PLAIN.reverse().bold()
            } else {
                self.c("white")
            };
            self.put(y, x, label, style);
            x += width(label) + 1;
        }
    }

    /// Key help on the bottom edge, less and less detailed as width shrinks.
    pub fn draw_tab_help(&mut self, y: i32, x0: i32, w: i32) {
        let ui = &tr().ui;
        let (mut sound_label, audible) = match &self.sound {
            Some(s) if s.ok() => match self.g.audio {
                "all" => ("♪", true),
                "sfx" => (ui.sound_on, true),
                _ => (ui.sound_off, false),
            },
            _ => (ui.sound_unavailable, false),
        };
        let with_volume;
        if audible && self.g.volume < game::MAX_VOLUME {
            with_volume = format!("{sound_label} {}", percent(self.g.volume));
            sound_label = &with_volume;
        }
        let variants = (ui.tab_help)(&self.qty.to_string(), sound_label);
        let help = variants
            .iter()
            .find(|h| width(h) <= w - 2)
            .unwrap_or(&variants[variants.len() - 1]);
        let grey = self.c("grey");
        self.put(
            y,
            x0 + 1.max(fdiv(w - width(help), 2)),
            &head(help, w - 2),
            grey,
        );
    }

    /// Scrolling list of the tab rows, then under a rule the description of the selected row
    /// (with the unit sprite on the right if it fits).
    pub fn draw_tab_body(&mut self, iy: i32, ix: i32, ih: i32, iw: i32) {
        let rows = self.rows();
        let selectable: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].sel).collect();
        let mut cur = None;
        if !selectable.is_empty() {
            self.sel[self.tab] = self.sel[self.tab].min(selectable.len() - 1);
            cur = Some(selectable[self.sel[self.tab]]);
        }
        let mut desc = cur.map_or(String::new(), |c| rows[c].desc.clone());
        let mut desc_style = Style::PLAIN;
        let confirming = self.confirm.is_some_and(|(_, until)| until > self.t);
        if confirming {
            desc = tr().ui.confirm.to_string();
            desc_style = self.c("red").bold();
        }
        let sprite = if confirming {
            None
        } else {
            cur.and_then(|c| rows[c].sprite)
        };
        let (lines, art_rows, art_style) = self.desc_layout(&desc, sprite, ih, iw);
        let list_h = 1.max(ih - 1 - lines.len() as i32);
        let sc = self.scroll_to(cur, &selectable, list_h as usize);
        let end = rows.len().min(sc + list_h as usize);
        for (i, r) in rows[sc.min(end)..end].iter().enumerate() {
            let idx = sc + i;
            let (prefix, style) = if Some(idx) == cur {
                ("▶ ", r.style.reverse())
            } else {
                ("  ", r.style)
            };
            self.put(
                iy + i as i32,
                ix,
                &head(&format!("{prefix}{}", r.text), iw),
                style,
            );
        }
        let grey = self.c("grey");
        if sc > 0 {
            self.put(iy, ix + iw - 1, "↑", grey);
        }
        if sc + (list_h as usize) < rows.len() {
            self.put(iy + list_h - 1, ix + iw - 1, "↓", grey);
        }

        self.put(iy + list_h, ix, &"─".repeat(iw.max(0) as usize), grey);
        for (i, line) in lines.iter().enumerate() {
            self.put(iy + list_h + 1 + i as i32, ix, line, desc_style);
        }
        if !art_rows.is_empty() {
            let w = width(&art_rows[0]);
            let bottom = art_rows.len() as i32 - 1;
            self.draw_sprite(
                iy + list_h + 1,
                ix + iw - w,
                w,
                bottom,
                0,
                &art_rows,
                art_style,
                false,
            );
        }
    }

    /// Description lines (at least 2, the list keeps 5), padded with blank lines up to the sprite height;
    /// the sprite is dropped if it does not fit. Returns (lines, sprite, style).
    pub fn desc_layout(
        &self,
        desc: &str,
        sprite: Option<(&'static str, bool)>,
        ih: i32,
        iw: i32,
    ) -> (Vec<String>, Vec<String>, Style) {
        let (mut art_rows, mut art_style) = (Vec::new(), Style::PLAIN);
        if let Some((uid, unlocked)) = sprite
            && iw >= SPRITE_MIN_TAB_W
        {
            art_rows = art::sprite(
                art::unit_art(uid).expect("every unit has art"),
                self.t,
                false,
            );
            art_style = if unlocked {
                self.c(data::unit(uid).color).bold()
            } else {
                self.c("grey")
            };
        }
        let mut text_w = iw - art_rows.first().map_or(0, |r| width(r) + SPRITE_GAP);
        let max_desc_h = ih - 1 - MIN_LIST_H;
        let mut lines = wrap(desc, text_w);
        let mut n_desc = MIN_DESC_H.max((lines.len().max(art_rows.len()) as i32).min(max_desc_h));
        if art_rows.len() as i32 > n_desc {
            art_rows.clear();
            text_w = iw;
            lines = wrap(desc, iw);
            n_desc = MIN_DESC_H.max((lines.len() as i32).min(max_desc_h));
        }
        let n = n_desc as usize;
        if lines.len() > n {
            let last = head(&lines[n - 1], text_w - 1) + "…";
            lines.truncate(n - 1);
            lines.push(last);
        }
        lines.resize(n.max(lines.len()), String::new());
        (lines, art_rows, art_style)
    }

    /// Tab scroll that keeps the selected row visible (at the very top for the first one).
    pub fn scroll_to(&mut self, cur: Option<usize>, selectable: &[usize], list_h: usize) -> usize {
        let Some(cur) = cur else { return 0 };
        let mut sc = self.scroll[self.tab];
        if cur < sc {
            sc = cur;
        } else if cur >= sc + list_h {
            sc = cur + 1 - list_h;
        }
        if Some(&cur) == selectable.first() {
            sc = 0;
        }
        self.scroll[self.tab] = sc;
        sc
    }

    // --- Overlays and banner -----------------------------------------------------------

    pub fn clear(&mut self, y: i32, x: i32, h: i32, w: i32) {
        let blank = " ".repeat(w.max(0) as usize);
        for i in 0..h {
            self.put(y + i, x, &blank, Style::PLAIN);
        }
    }

    /// Empty centered framed window; returns its top-left corner.
    pub fn popup(&mut self, bh: i32, bw: i32, title: &str, style: Style) -> (i32, i32) {
        let (y, x) = (fdiv(self.h - bh, 2), fdiv(self.w - bw, 2));
        self.clear(y, x, bh, bw);
        self.frame(y, x, bh, bw, title, style);
        (y, x)
    }

    pub fn draw_qte(&mut self, y0: i32, x0: i32, h: i32, w: i32) {
        let bw = (w - POPUP_MARGIN).min(QTE_W);
        let y = y0 + h / QTE_ROW_DIV;
        let x = x0 + fdiv(w - bw, 2);
        if let Some(q) = self.g.qte {
            self.clear(y, x, QTE_H, bw);
            let red = self.c("red").bold();
            self.frame(y, x, QTE_H, bw, tr().ui.qte_title, red);
            let n = bw - 2 * PANEL_PAD;
            for i in 0..n {
                let off_center = ((f64::from(i) + 0.5) / f64::from(n) - q.center).abs();
                let (ch, col) = if off_center <= q.green {
                    ("█", "green")
                } else if off_center <= q.yellow {
                    ("▓", "yellow")
                } else {
                    ("░", "red")
                };
                let style = self.c(col);
                self.put(y + 3, x + PANEL_PAD + i, ch, style);
            }
            let cx = x + PANEL_PAD + (n - 1).min((events::qte_pos(&q) * f64::from(n)) as i32);
            let white = self.c("white").bold();
            self.put(y + 2, cx, "▼", white);
            self.put(y + 1, x + PANEL_PAD, &head(tr().ui.qte_prompt, n), white);
        } else if let Some(res) = self.g.qte_result.clone()
            && self.g.time < res.until
        {
            let style = self.c(res.color).bold();
            self.frame(y, x, QTE_RESULT_H, bw, "", style);
            self.put(
                y + 1,
                x + 1,
                &" ".repeat((bw - 2).max(0) as usize),
                Style::PLAIN,
            );
            let tx = x + 1.max(fdiv(bw - width(&res.text), 2));
            self.put(y + 1, tx, &head(&res.text, bw - 2), style);
        }
    }

    pub fn draw_report(&mut self) {
        if !self.g.report_ready {
            return;
        }
        let Some(r) = self.g.last_run.clone() else {
            return;
        };
        let lines = report_lines(&r);
        let bw = (self.w - POPUP_MARGIN).min(REPORT_W);
        let lw = lines
            .iter()
            .map(|(label, _)| width(label))
            .max()
            .unwrap_or(0)
            + REPORT_LABEL_GAP;
        let mut rows: Vec<(String, String)> = Vec::new();
        let value_w = REPORT_VALUE_MIN_W.max(bw - REPORT_WRAP_MARGIN - lw);
        for (label, value) in lines {
            let mut wrapped = wrap(&value, value_w);
            if wrapped.is_empty() {
                wrapped.push(String::new());
            }
            if wrapped.len() > REPORT_VALUE_LINES {
                wrapped.truncate(REPORT_VALUE_LINES);
                let last = wrapped.pop().unwrap_or_default();
                wrapped.push(head(&last, value_w - 1) + "…");
            }
            let mut wrapped = wrapped.into_iter();
            rows.push((label.to_string(), wrapped.next().unwrap_or_default()));
            rows.extend(wrapped.map(|more| (String::new(), more)));
        }
        let bh = (self.h - 2).min(rows.len() as i32 + REPORT_CHROME_H);
        let room = bh - REPORT_CHROME_H;
        if rows.len() as i32 > room && room > 0 {
            rows.truncate((room - 1) as usize);
            rows.push(("…".to_string(), String::new()));
        }
        let (y, x) = (fdiv(self.h - bh, 2).max(0), fdiv(self.w - bw, 2).max(0));
        self.clear(y, x, bh, bw);
        let ui = &tr().ui;
        let (title, col) = match r.kind {
            game::ReportKind::Supernova => {
                ((ui.report_supernova)(self.g.supernova_count), "magenta")
            }
            game::ReportKind::Prestige => ((ui.report_prestige)(r.number), "green"),
        };
        let style = self.c(col).bold();
        self.frame(y, x, bh, bw, &title, style);
        let (cyan, white) = (self.c("cyan"), self.c("white").bold());
        for (i, (label, value)) in rows.iter().take(room.max(0) as usize).enumerate() {
            self.put(y + 1 + i as i32, x + PANEL_PAD, label, cyan);
            self.put(
                y + 1 + i as i32,
                x + PANEL_PAD + lw,
                &head(value, bw - 2 * PANEL_PAD - lw),
                white,
            );
        }
        let msg = ui.report_prompt;
        let style = if self.blink(BLINK_PROMPT) {
            self.c("yellow").bold()
        } else {
            self.c("yellow")
        };
        self.put(
            y + bh - 2,
            x + 1.max(fdiv(bw - width(msg), 2)),
            &head(msg, bw - 2),
            style,
        );
    }

    pub fn draw_chest(&mut self) {
        let Some(opts) = self.g.pending_chest.clone().filter(|o| !o.is_empty()) else {
            return;
        };
        let bw = (self.w - POPUP_MARGIN).min(POPUP_W);
        let cyan = self.c("cyan").bold();
        let (y, x) = self.popup(
            POPUP_CHROME_H + OPTION_ROW_H * opts.len() as i32,
            bw,
            tr().ui.chest_title,
            cyan,
        );
        let white = self.c("white").bold();
        self.put(
            y + 1,
            x + PANEL_PAD,
            &(tr().ui.chest_choose)(opts.len()),
            white,
        );
        for (i, kind) in opts.iter().enumerate() {
            let (name, mut desc) = events::chest_text(&self.g, kind);
            let ok = events::chest_allowed(&self.g, kind);
            if !ok {
                desc = tr().ui.chest_useless.to_string();
            }
            let (name_style, desc_style) = if ok {
                (self.c("yellow").bold(), self.c("white"))
            } else {
                (self.c("grey"), self.c("grey"))
            };
            let row = y + 2 + OPTION_ROW_H * i as i32;
            self.put(
                row,
                x + PANEL_PAD,
                &head(&format!("{}. {name}", i + 1), bw - 2 * PANEL_PAD),
                name_style,
            );
            self.put(
                row + 1,
                x + OPTION_DESC_X,
                &head(&desc, bw - OPTION_DESC_X - PANEL_PAD),
                desc_style,
            );
        }
    }

    pub fn draw_mutation(&mut self) {
        let Some(opts) = self.g.pending_mutation.clone().filter(|o| !o.is_empty()) else {
            return;
        };
        if self.g.pending_chest.as_ref().is_some_and(|c| !c.is_empty()) {
            return;
        }
        let bw = (self.w - POPUP_MARGIN).min(POPUP_W);
        let green = self.c("green").bold();
        let (y, x) = self.popup(
            POPUP_CHROME_H + OPTION_ROW_H * opts.len() as i32,
            bw,
            tr().ui.mutation_title,
            green,
        );
        let white = self.c("white").bold();
        let intro = (tr().ui.mutation_choose)(opts.len());
        self.put(y + 1, x + PANEL_PAD, &intro, white);
        for (i, mid) in opts.iter().enumerate() {
            let m = events::mutation(mid);
            let label = format!("{}. {}", i + 1, m.name());
            let row = y + 2 + OPTION_ROW_H * i as i32;
            let yellow = self.c("yellow").bold();
            self.put(
                row,
                x + PANEL_PAD,
                &head(&label, bw - 2 * PANEL_PAD),
                yellow,
            );
            if let Some(syn) = events::synergy_completed_by(&self.g, mid) {
                let hint = if self.g.synergies_found.contains(syn.id) {
                    format!(" ★ {}", syn.name())
                } else {
                    format!(" ★ {}", tr().ui.synergy_unknown)
                };
                let magenta = self.c("magenta").bold();
                let lw = width(&label);
                self.put(
                    row,
                    x + PANEL_PAD + lw,
                    &head(&hint, (bw - 2 * PANEL_PAD - lw).max(0)),
                    magenta,
                );
            }
            let desc_style = self.c("white");
            self.put(
                row + 1,
                x + OPTION_DESC_X,
                &head(&m.effect(), bw - OPTION_DESC_X - PANEL_PAD),
                desc_style,
            );
        }
    }

    pub fn draw_news(&mut self, y: i32, dt: f64) {
        let label = &format!(" {} ", tr().ui.news);
        let alert = self.c("alert");
        self.put(y, 0, label, alert);
        let ww = self.w - width(label) - NEWS_MARGIN;
        self.news_off += dt * NEWS_SPEED;
        if self.news.is_empty() || self.news_off > f64::from(width(self.news) + ww) {
            self.news = events::pick_news(&mut self.g);
            self.news_off = 0.0;
        }
        let pad = " ".repeat(ww.max(0) as usize);
        let text = format!("{pad}{}{pad}", self.news);
        let start = self.news_off as i32;
        let white = self.c("white");
        self.put(y, width(label) + 1, &slice(&text, start, start + ww), white);
    }

    // --- Keys ----------------------------------------------------------------------------

    pub fn handle(&mut self, key: Key) -> Flow {
        if key == Key::Interrupt {
            return Flow::Quit;
        }
        let too_small = self.w != 0 && (self.w < MIN_W || self.h < MIN_H);
        let report_shown = self.g.report_ready && self.g.last_run.is_some() && !too_small;
        if report_shown && key != Key::Resize {
            self.g.report_ready = false; // the key closes the end screen, with no other effect
            self.sfx("tick");
            return Flow::Continue;
        }
        let digit = match key {
            Key::Char(c @ '0'..='9') => Some(c as usize - '0' as usize),
            _ => None,
        };
        let pending_mutations = self.g.pending_mutation.as_ref().map_or(0, Vec::len);
        let g = &mut self.g;
        match key {
            Key::Char(' ') => {
                let (before, lvl) = (g.radiation, g.level);
                let fizzles = g.sfx.iter().filter(|&&s| s == "fizzle").count();
                g.click(true);
                let gained = g.radiation - before;
                let fizzled = g.sfx.iter().filter(|&&s| s == "fizzle").count() > fizzles;
                let dx = self.rng.randint(-FLOAT_SPREAD, FLOAT_SPREAD) as i32;
                if gained > 0.0 {
                    let crit = self.g.last_crit == self.g.time;
                    let gained = self.num(gained);
                    let text = if crit {
                        (tr().ui.crit)(&gained)
                    } else {
                        format!("+{gained}")
                    };
                    let style = if crit {
                        self.c("red")
                    } else {
                        self.c("yellow")
                    }
                    .bold();
                    self.add_float(text, style, dx);
                } else if fizzled {
                    let style = self.c("grey").bold();
                    self.add_float(tr().ui.fizzle.to_string(), style, dx);
                }
                if self.g.level > lvl && lvl > 0 {
                    let style = self.c("green").bold();
                    let level = self.num(f64::from(self.g.level));
                    self.add_float((tr().ui.level_up)(&level), style, 0);
                }
            }
            Key::Char('q' | 'Q') => return Flow::Quit,
            Key::Char('f' | 'F') => events::crit_press(g),
            Key::Char('x' | 'X') => events::chase(g),
            Key::Char('1'..='9') if g.pending_chest.as_ref().is_some_and(|c| !c.is_empty()) => {
                if !events::choose_chest(g, digit.unwrap_or(1) - 1) {
                    self.sfx("nope");
                }
            }
            Key::Char('1'..='9') if digit.is_some_and(|d| d <= pending_mutations) => {
                events::choose_mutation(g, digit.unwrap_or(1) - 1);
            }
            Key::Char('0') => {
                let i = AUDIO_MODES.iter().position(|m| *m == g.audio).unwrap_or(0);
                g.audio = AUDIO_MODES[(i + 1) % AUDIO_MODES.len()];
                let muted = g.audio == "off";
                let msg = (tr().ui.sound)(audio_label(g.audio), muted);
                g.log(
                    msg,
                    if muted {
                        game::MsgKind::Bad
                    } else {
                        game::MsgKind::Info
                    },
                    None,
                );
            }
            Key::Char(c @ ('+' | '=' | '-')) => {
                let (volume, before) = (tr().ui.volume, g.volume);
                if g.change_volume(if c == '-' { -1 } else { 1 }) {
                    g.sfx.push_back("tick"); // preview at the new volume
                }
                // A single volume banner: otherwise 20 presses flush the useful messages from the history.
                // The previous banner, if it is the latest message, shows the volume before this press.
                if g.messages.back().is_some_and(|m| m.text == volume(before)) {
                    g.messages.pop_back();
                }
                let msg = volume(g.volume);
                g.log(msg, game::MsgKind::Info, None);
            }
            Key::Char('1'..='5') => {
                self.tab = digit.unwrap_or(1) - 1;
                self.sfx("tick");
            }
            Key::Tab | Key::Right => {
                self.tab = (self.tab + 1) % TAB_COUNT;
                self.sfx("tick");
            }
            Key::BackTab | Key::Left => {
                self.tab = (self.tab + TAB_COUNT - 1) % TAB_COUNT;
                self.sfx("tick");
            }
            Key::Up | Key::Char('k') => {
                self.sel[self.tab] = self.sel[self.tab].saturating_sub(1);
                self.sfx("tick");
            }
            Key::Down | Key::Char('j') => {
                self.sel[self.tab] += 1;
                self.sfx("tick");
            }
            Key::PageUp => self.sel[self.tab] = self.sel[self.tab].saturating_sub(PAGE_ROWS),
            Key::PageDown => self.sel[self.tab] += PAGE_ROWS,
            Key::Enter => return self.activate(),
            Key::Char('m' | 'M') => {
                let i = QTY.iter().position(|q| *q == self.qty).unwrap_or(0);
                self.qty = QTY[(i + 1) % QTY.len()];
                self.sfx("tick");
            }
            Key::Char(c)
                if (c as u32) < 256
                    && c.to_lowercase().all(|l| events::DROP_LETTERS.contains(l)) =>
            {
                let letter = c.to_lowercase().next().unwrap_or(c);
                if events::catch(g, letter) {
                    let style = self.c("cyan").bold();
                    self.add_float(tr().ui.chest_caught.to_string(), style, 0);
                }
            }
            Key::Resize => self.scr.clear(),
            _ => {}
        }
        Flow::Continue
    }

    /// Text rising above the mushroom (see update_particles).
    pub fn add_float(&mut self, text: String, style: Style, dx: i32) {
        self.floats.push(Float {
            text,
            age: 0.0,
            dx,
            style,
        });
        let excess = self.floats.len().saturating_sub(MAX_FLOATS);
        self.floats.drain(..excess);
    }

    pub fn selected_action(&self) -> Option<Action> {
        let selectable: Vec<Row> = self.rows().into_iter().filter(|r| r.sel).collect();
        if selectable.is_empty() {
            return None;
        }
        selectable[self.sel[self.tab].min(selectable.len() - 1)].action
    }

    /// Enter on the selected row. Quit if Ctrl+C was pressed on a story screen.
    pub fn activate(&mut self) -> Flow {
        let Some(action) = self.selected_action() else {
            return Flow::Continue;
        };
        let g = &mut self.g;
        match action {
            Action::Unit(uid) => {
                let bought = match self.qty {
                    Qty::Max => g.buy_unit_max(uid),
                    Qty::N(k) => g.buy_unit_n(uid, k.min(g.max_bulk())),
                };
                if bought > 0 {
                    self.sfx("buy");
                } else {
                    self.sfx("nope");
                }
            }
            Action::Talent(id) | Action::PTalent(id) | Action::STalent(id) => {
                let buy: fn(&mut Game, &str) -> bool = match action {
                    Action::Talent(_) => Game::buy_talent,
                    Action::PTalent(_) => Game::buy_ptalent,
                    _ => Game::buy_stalent,
                };
                let limit = match self.qty {
                    Qty::Max => u64::MAX,
                    Qty::N(k) => k,
                };
                let mut n = 0;
                while n < limit && buy(g, id) {
                    n += 1;
                }
                if n > 0 {
                    self.sfx("talent");
                } else {
                    self.sfx("nope");
                }
            }
            Action::Abandon => g.abandon_challenge(),
            Action::DoubleMode => {
                self.double = !self.double;
                self.double_pick = None;
            }
            Action::ToggleDone => self.show_done = !self.show_done,
            Action::ToggleStories => self.show_stories = !self.show_stories,
            Action::Challenge(id) if self.double && self.double_pick.is_none_or(|p| p == id) => {
                // First pick, or a second Enter on it to drop it.
                self.double_pick = if self.double_pick == Some(id) {
                    None
                } else {
                    Some(id)
                };
            }
            Action::Intro => {
                let texts = &tr().screens;
                if intro(
                    &mut self.scr,
                    self.sound.as_deref(),
                    texts.intro,
                    0.0,
                    texts.start,
                )
                .is_err()
                {
                    return Flow::Quit;
                }
            }
            Action::Story(id) => {
                for part in ["start", "end"] {
                    if let Some(pages) = story(id, part)
                        && intro(
                            &mut self.scr,
                            self.sound.as_deref(),
                            pages,
                            0.0,
                            tr().screens.resume,
                        )
                        .is_err()
                    {
                        return Flow::Quit;
                    }
                }
            }
            Action::Skin => {
                let owned: Vec<&'static str> = data::SKINS
                    .into_iter()
                    .filter(|id| g.skins.contains(id))
                    .collect();
                if let Some(first) = owned.first() {
                    g.skin = match owned.iter().position(|s| *s == g.skin) {
                        Some(i) => owned[(i + 1) % owned.len()],
                        None => first,
                    };
                }
            }
            Action::Prestige | Action::Challenge(_) | Action::Supernova => {
                if self
                    .confirm
                    .is_some_and(|(a, until)| a == action && until > self.t)
                {
                    self.confirm = None;
                    let ok = match (action, self.double_pick) {
                        (Action::Supernova, _) => g.supernova(),
                        (Action::Challenge(id), Some(first)) if self.double => {
                            g.prestige_double(first, id)
                        }
                        (Action::Challenge(id), _) => g.prestige(Some(id)),
                        _ => g.prestige(None),
                    };
                    if ok {
                        self.double_pick = None;
                        self.particles.clear();
                        self.spawn_acc.clear();
                    }
                } else {
                    self.confirm = Some((action, self.t + CONFIRM_TIME));
                }
            }
        }
        Flow::Continue
    }
}

/// Text cut to `w` characters, including the « … ».
pub fn clip(text: &str, w: i32) -> String {
    if width(text) <= w {
        text.to_string()
    } else {
        head(text, w - 1) + "…"
    }
}

/// Percentage in the current language (15 %).
fn percent(n: impl fmt::Display) -> String {
    (tr().percent)(&n.to_string())
}

/// Columns of the widest percentage (100 %) in the current language.
fn percent_width() -> i32 {
    width(&percent(100))
}

/// Duration for humans: 42 s, 2 min 05 s, 2 h 01 min (∞ if it never ends).
pub fn dur(sec: f64) -> String {
    if sec == f64::INFINITY {
        return tr().infinity.to_string();
    }
    (tr().dur)(sec as i64)
}

/// (label, value) rows of a finished game summary.
pub fn report_lines(r: &RunReport) -> Vec<(&'static str, String)> {
    let fmt = game::fmt;
    let labels = &tr().ui.report;
    let star = if r.kind == game::ReportKind::Supernova {
        '✦'
    } else {
        '⚛'
    };
    let mut lines = vec![
        (labels.duration, dur(r.time)),
        (
            labels.level,
            format!("{} / {}", fmt(f64::from(r.level)), fmt(f64::from(r.cap))),
        ),
        (labels.gain, format!("+{} {star}", fmt(r.gain as f64))),
        (labels.radiation, format!("{} ☢", fmt(r.earned))),
        (
            labels.clicks,
            (labels.clicks_value)(&fmt(r.clicks as f64), &fmt(r.crits as f64)),
        ),
        (labels.best_click, format!("{} ☢", fmt(r.best_click))),
        (labels.perfects, fmt(r.perfects as f64)),
        (labels.inspectors, fmt(r.inspectors as f64)),
        (labels.units, fmt(r.units as f64)),
    ];
    // The report lists ids (checked on load): their names are those of the current language.
    let or_none = |names: Vec<&str>, none: &str| {
        if names.is_empty() {
            none.to_string()
        } else {
            names.join(tr().list_join)
        }
    };
    let mutations = r
        .mutations
        .iter()
        .map(|m| events::mutation(m).name())
        .collect();
    lines.push((labels.mutations, or_none(mutations, labels.none)));
    let synergies = r
        .synergies
        .iter()
        .map(|s| events::synergy(s).name())
        .collect();
    lines.push((labels.synergies, or_none(synergies, labels.none)));
    let chests = &r.chests;
    let latest = &chests[chests.len().saturating_sub(REPORT_CHESTS_SHOWN)..];
    let mut shown = or_none(
        latest.iter().map(|c| tr().data.chest(c)).collect(),
        labels.no_chest,
    );
    if chests.len() > REPORT_CHESTS_SHOWN {
        shown += &format!(" (+{})", chests.len() - REPORT_CHESTS_SHOWN);
    }
    lines.push((labels.chests, shown));
    lines
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::events::EnemyKind;
    use crate::game::tests::{exploded_game, exploded_game_with, fixed};
    use crate::game::{Enemy, Inspector};
    use crate::term::{BufferScreen, ColorName};

    /// What the UI asked of the sound output.
    #[derive(Debug, Default)]
    pub(crate) struct SoundLog {
        pub played: Vec<String>,
        pub music: Option<String>,
        pub muted: bool,
        pub music_on: bool,
        pub volume: f32,
    }

    /// Sound output that records the calls; clones share the same log.
    #[derive(Clone)]
    pub(crate) struct FakeSound {
        pub ok: bool,
        pub log: Rc<RefCell<SoundLog>>,
    }

    impl FakeSound {
        pub fn new() -> FakeSound {
            let log = SoundLog {
                music_on: true,
                volume: 1.0,
                ..SoundLog::default()
            };
            FakeSound {
                ok: true,
                log: Rc::new(RefCell::new(log)),
            }
        }

        pub fn played(&self) -> Vec<String> {
            self.log.borrow().played.clone()
        }
    }

    impl SoundSink for FakeSound {
        fn ok(&self) -> bool {
            self.ok
        }
        fn play(&self, name: &str) {
            self.log.borrow_mut().played.push(name.to_string());
        }
        fn set_music(&self, track: &str) {
            self.log.borrow_mut().music = Some(track.to_string());
        }
        fn set_muted(&self, muted: bool) {
            self.log.borrow_mut().muted = muted;
        }
        fn set_music_on(&self, on: bool) {
            self.log.borrow_mut().music_on = on;
        }
        fn set_volume(&self, volume: f32) {
            self.log.borrow_mut().volume = volume;
        }
    }

    /// UI on a 30×100 test screen, with a seeded rng.
    fn bare_ui_with(g: Game, sound: Option<&FakeSound>) -> UI<BufferScreen> {
        let sound = sound.map(|s| Box::new(s.clone()) as Box<dyn SoundSink>);
        let mut u = UI::new(BufferScreen::new(30, 100), g, sound);
        u.rng = Box::new(StdRandom::seeded(0));
        u
    }

    fn bare_ui() -> UI<BufferScreen> {
        bare_ui_with(exploded_game(), None)
    }

    /// Fresh test screen of h×w (refuses writes off screen) on which the UI draws.
    fn sized(u: &mut UI<BufferScreen>, h: i32, w: i32) {
        u.scr = BufferScreen::new(h, w);
        u.h = h;
        u.w = w;
    }

    fn texts(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|(t, _)| t.clone()).collect()
    }

    fn written(u: &UI<BufferScreen>, needle: &str) -> bool {
        u.scr.writes.iter().any(|(_, _, s)| s.contains(needle))
    }

    fn row_texts(rows: &[Row]) -> Vec<String> {
        rows.iter().map(|r| r.text.clone()).collect()
    }

    fn ch(c: char) -> Key {
        Key::Char(c)
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // --- Sound -----------------------------------------------------------------------

    #[test]
    fn sfx_dedupe_per_frame() {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        for name in ["click0", "click3", "level", "level", "tick"] {
            u.g.sfx.push_back(name);
        }
        u.play_sfx();
        assert_eq!(sound.played(), ["click0", "level", "tick"]);
        assert!(u.g.sfx.is_empty());
    }

    #[test]
    fn sfx_throttle() {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        u.g.sfx.push_back("level");
        u.play_sfx();
        u.t = 0.1;
        u.g.sfx.push_back("level");
        u.play_sfx();
        assert_eq!(sound.played(), ["level"]);
        u.t = sfx_gap("level");
        u.g.sfx.push_back("level");
        u.play_sfx();
        assert_eq!(sound.played(), ["level", "level"]);
    }

    #[test]
    fn sfx_without_sound() {
        let mut u = bare_ui();
        u.g.sfx.push_back("boom");
        u.play_sfx();
        assert!(u.g.sfx.is_empty());
    }

    #[test]
    fn throttled_sounds_exist() {
        let bank = crate::sound::build();
        for (name, _) in SFX_GAP {
            assert!(bank.contains_key(name), "{name}");
        }
        for (name, _) in SHAKE {
            assert!(bank.contains_key(name), "{name}");
        }
    }

    #[test]
    fn audio_modes_synced() {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        for (mode, muted, music) in [
            ("all", false, true),
            ("sfx", false, false),
            ("off", true, false),
        ] {
            u.g.audio = mode;
            u.play_sfx();
            let log = sound.log.borrow();
            assert_eq!((log.muted, log.music_on), (muted, music), "{mode}");
        }
    }

    #[test]
    fn audio_key_cycles() {
        let mut u = bare_ui();
        let mut seen = Vec::new();
        for _ in 0..4 {
            u.handle(ch('0'));
            seen.push(u.g.audio);
        }
        assert_eq!(seen, ["sfx", "off", "all", "sfx"]);
    }

    #[test]
    fn music_track() {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        u.play_sfx();
        assert_eq!(sound.log.borrow().music.as_deref(), Some("calm"));
        u.g.level = u.g.cap();
        u.play_sfx();
        assert_eq!(sound.log.borrow().music.as_deref(), Some("epic"));
        u.g.inspector = Some(Inspector {
            start: 0.0,
            dur: 20.0,
            hits: 0,
        });
        u.play_sfx();
        assert_eq!(sound.log.borrow().music.as_deref(), Some("tension"));
    }

    // --- Banners ---------------------------------------------------------------------

    fn message(text: &str, time: f64, kind: game::MsgKind) -> game::Message {
        game::Message {
            text: text.to_string(),
            time,
            kind,
            title: None,
        }
    }

    #[test]
    fn banner_order_and_expiry() {
        use game::MsgKind::*;
        let mut g = exploded_game();
        g.time = 100.0;
        for (text, time, kind) in [
            ("vieux", 90.0, Info),
            ("a", 97.0, Good),
            ("b", 98.0, Bad),
            ("c", 99.0, Loot),
            ("d", 99.5, Star),
        ] {
            g.messages.push_back(message(text, time, kind));
        }
        let lines = bare_ui_with(g, None).banner_lines(60);
        let texts: Vec<String> = lines.iter().map(|(t, _)| t.trim().to_string()).collect();
        assert_eq!(texts, ["d", "c", "b"]);
    }

    #[test]
    fn banner_alerts_first() {
        let mut g = exploded_game();
        g.inspector = Some(Inspector {
            start: g.time,
            dur: 20.0,
            hits: 0,
        });
        g.freeze_until = g.time + 10.0;
        g.log("coucou", game::MsgKind::Info, None);
        let lines = texts(&bare_ui_with(g, None).banner_lines(60));
        assert!(lines[0].contains("inspecteur"));
        assert!(lines[1].contains("GELÉE"));
        assert_eq!(lines[2], "coucou");
    }

    #[test]
    fn banner_long_message_wrapped_in_block() {
        let mut g = exploded_game();
        g.log(
            "un message vraiment beaucoup trop long pour tenir sur une seule ligne du panneau",
            game::MsgKind::Info,
            None,
        );
        let lines = texts(&bare_ui_with(g, None).banner_lines(40));
        assert_eq!(lines.len(), 2);
        assert_eq!(width(&lines[0]), width(&lines[1]));
        assert!(lines.iter().all(|t| width(t) <= 36));
    }

    #[test]
    fn banner_fade_drops_background() {
        let mut g = exploded_game();
        g.log("fin", game::MsgKind::Bad, None);
        g.time += MSG_TIME - 0.5;
        let u = bare_ui_with(g, None);
        assert_eq!(u.banner_lines(60)[0].1, Style::of(ColorName::Red).bold());
    }

    // --- Quantity --------------------------------------------------------------------

    fn soldier_ui() -> UI<BufferScreen> {
        let mut u = bare_ui();
        u.g.talents.insert("u_soldier", 1);
        u
    }

    #[test]
    fn m_cycles() {
        let mut u = soldier_ui();
        let mut seen = Vec::new();
        for _ in 0..5 {
            u.handle(ch('m'));
            seen.push(u.qty);
        }
        assert_eq!(
            seen,
            [Qty::N(10), Qty::N(100), Qty::Max, Qty::N(1), Qty::N(10)]
        );
    }

    #[test]
    fn buy_ten() {
        let mut u = soldier_ui();
        u.qty = Qty::N(10);
        u.g.radiation = u.g.unit_cost_n("soldier", 10);
        u.activate();
        assert_eq!(u.g.units["soldier"], 10);
        assert_eq!(u.g.sfx.back(), Some(&"buy"));
    }

    #[test]
    fn buy_ten_refused() {
        let mut u = soldier_ui();
        u.qty = Qty::N(10);
        u.g.radiation = u.g.unit_cost_n("soldier", 9);
        u.activate();
        assert_eq!(u.g.units["soldier"], 0);
        assert_eq!(u.g.sfx.back(), Some(&"nope"));
    }

    #[test]
    fn buy_max() {
        let mut u = soldier_ui();
        u.qty = Qty::Max;
        u.g.radiation = u.g.unit_cost_n("soldier", 7) + 1.0;
        u.activate();
        assert_eq!(u.g.units["soldier"], 7);
    }

    #[test]
    fn row_shows_cost_and_wait() {
        let mut u = soldier_ui();
        u.qty = Qty::N(10);
        u.g.units["mortar"] = 0;
        u.g.units["soldier"] = 2;
        let r = &u.unit_rows()[0];
        assert!(r.text.contains(&game::fmt(u.g.unit_cost_n("soldier", 10))));
        assert!(r.desc.contains("×10"));
        assert!(r.desc.contains("d'attente"));
    }

    #[test]
    fn talent_ranks() {
        let mut u = soldier_ui();
        u.tab = 1;
        u.qty = Qty::N(100);
        u.g.pt = 1000;
        u.sel[1] = 0; // first selectable talent: Détonateur renforcé (max 10)
        u.activate();
        assert_eq!(u.g.tr("t_click"), 10);
    }

    // --- Supernova -------------------------------------------------------------------

    fn supernova_ui() -> UI<BufferScreen> {
        let mut u = bare_ui();
        u.g.prestige_count = 1;
        u.g.ptalents.insert("p_cap", 4);
        u.g.level = u.g.cap();
        u.tab = 2;
        u
    }

    fn select(u: &mut UI<BufferScreen>, pick: fn(&Action) -> bool) {
        let rows: Vec<Row> = u.prestige_rows().into_iter().filter(|r| r.sel).collect();
        u.sel[2] = rows
            .iter()
            .position(|r| r.action.as_ref().is_some_and(pick))
            .expect("row present");
    }

    #[test]
    fn supernova_section_shown() {
        let u = supernova_ui();
        let texts = row_texts(&u.prestige_rows());
        assert!(texts.iter().any(|t| t.contains("SUPERNOVA :")));
        assert!(texts.iter().any(|t| t.contains("Fusion stellaire")));
    }

    #[test]
    fn supernova_hidden_before_unlock() {
        let mut u = supernova_ui();
        u.g.ptalents.insert("p_cap", 3);
        assert!(
            !row_texts(&u.prestige_rows())
                .iter()
                .any(|t| t.contains("SUPERNOVA"))
        );
    }

    #[test]
    fn supernova_double_confirm() {
        let mut u = supernova_ui();
        select(&mut u, |a| *a == Action::Supernova);
        u.activate();
        assert_eq!(u.g.supernova_count, 0);
        select(&mut u, |a| *a == Action::Supernova);
        u.activate();
        assert_eq!(u.g.supernova_count, 1);
    }

    /// Game time well past the explosion flash.
    const LONG_AFTER_EXPLOSION: f64 = 500.0;

    #[test]
    fn challenge_monster_crosses_the_field() {
        let mut u = bare_ui();
        u.g.time = LONG_AFTER_EXPLOSION;
        u.t = MONSTER_CROSSING / 2.0; // mid-field
        u.draw(0.0);
        // Rubber croc's backs (,^^^^^,; '^' alone is also dust) and the leftmost column holding one.
        let back = |u: &UI<BufferScreen>| {
            let cols: Vec<usize> = u
                .scr
                .rows()
                .iter()
                .filter_map(|r| {
                    let line: String = r.iter().map(|c| c.0).collect();
                    line.find(",^^^^^,").map(|b| line[..b].chars().count())
                })
                .collect();
            (cols.len(), cols.into_iter().min())
        };
        let (before, _) = back(&u);
        u.g.challenge = Some("crocodile");
        u.draw(0.0);
        let (during, mid) = back(&u);
        assert!(during > before);
        u.t += MONSTER_CROSSING / 4.0; // a quarter of a crossing later, further left
        u.draw(0.0);
        let (_, later) = back(&u);
        assert!(later.is_some() && later < mid);
    }

    #[test]
    fn owned_monster_drawn_once() {
        let mut u = bare_ui();
        u.g.time = LONG_AFTER_EXPLOSION;
        u.t = MONSTER_CROSSING / 2.0;
        u.g.challenge = Some("crocodile");
        u.g.units["croc"] = 1;
        u.draw(0.0);
        let backs = u
            .scr
            .rows()
            .iter()
            .filter(|row| {
                row.iter()
                    .map(|c| c.0)
                    .collect::<String>()
                    .contains(",^^^^^,")
            })
            .count();
        assert_eq!(backs, 1);
    }

    #[test]
    fn done_challenges_folded() {
        let mut u = supernova_ui();
        u.g.challenges_done.insert("race", data::CHALLENGE_RANKS);
        let texts = row_texts(&u.prestige_rows());
        assert!(
            texts
                .iter()
                .any(|t| t.starts_with("✔ 1 défi(s) terminé(s)"))
        );
        assert!(!texts.iter().any(|t| t.contains("Défi Course")));
        select(&mut u, |a| *a == Action::ToggleDone);
        u.activate();
        assert!(
            row_texts(&u.prestige_rows())
                .iter()
                .any(|t| t.contains("Défi Course"))
        );
    }

    #[test]
    fn monster_challenges_have_their_section() {
        let u = supernova_ui();
        let texts = row_texts(&u.prestige_rows());
        let header = texts
            .iter()
            .position(|t| t.starts_with("── MONSTRES"))
            .unwrap();
        let moussaka = texts
            .iter()
            .position(|t| t.contains("moussaka géante"))
            .unwrap();
        let race = texts
            .iter()
            .position(|t| t.contains("Défi Course"))
            .unwrap();
        assert!(race < header && header < moussaka);
        // All the monster challenges done and folded: no empty section.
        let mut u = u;
        for c in data::CHALLENGES.iter() {
            if data::challenge_unit(c.id).is_some() {
                u.g.challenges_done.insert(c.id, data::CHALLENGE_RANKS);
            }
        }
        assert!(
            !row_texts(&u.prestige_rows())
                .iter()
                .any(|t| t.starts_with("── MONSTRES"))
        );
    }

    #[test]
    fn double_challenge_picked_then_started() {
        let mut u = supernova_ui();
        u.g.level = 1;
        assert!(
            !row_texts(&u.prestige_rows())
                .iter()
                .any(|t| t.starts_with("Double défi"))
        );
        for c in data::CHALLENGES.iter().skip(3).take(data::DOUBLE_UNLOCK) {
            u.g.challenges_done.insert(c.id, 1);
        }
        select(&mut u, |a| *a == Action::DoubleMode);
        u.activate();
        assert!(u.double);
        assert!(
            !u.prestige_rows()
                .iter()
                .any(|r| r.action == Some(Action::Challenge("pacifist")))
        );
        select(&mut u, |a| *a == Action::Challenge("race"));
        u.activate();
        assert_eq!(u.double_pick, Some("race"));
        assert!(
            row_texts(&u.prestige_rows())
                .iter()
                .any(|t| t.contains("◆ Défi Course"))
        );
        select(&mut u, |a| *a == Action::Challenge("onearm"));
        u.activate(); // confirmation
        assert_eq!(u.g.challenge, None);
        select(&mut u, |a| *a == Action::Challenge("onearm"));
        u.activate();
        assert_eq!(
            (u.g.challenge, u.g.challenge2),
            (Some("race"), Some("onearm"))
        );
        assert_eq!(u.double_pick, None);
    }

    #[test]
    fn confirmation_expires() {
        let mut u = supernova_ui();
        select(&mut u, |a| *a == Action::Supernova);
        u.activate();
        u.t += 3.5;
        u.activate();
        assert_eq!(u.g.supernova_count, 0); // the first press was too old: this one asks again
    }

    #[test]
    fn buy_star_talent() {
        let mut u = supernova_ui();
        u.g.stars = 5;
        select(&mut u, |a| matches!(a, Action::STalent(_)));
        u.activate();
        assert_eq!(u.g.sr("s_prod"), 1);
        assert_eq!(u.g.sfx.back(), Some(&"talent"));
    }

    // --- Effects ---------------------------------------------------------------------

    #[test]
    fn shake_set_even_without_sound() {
        let mut u = bare_ui();
        u.g.sfx.push_back("perfect");
        u.play_sfx();
        assert!(approx(u.shake_until, shake("perfect").unwrap()));
    }

    #[test]
    fn shake_not_shortened() {
        let mut u = bare_ui();
        u.g.sfx.push_back("supernova");
        u.g.sfx.push_back("perfect");
        u.play_sfx();
        assert!(approx(u.shake_until, shake("supernova").unwrap()));
    }

    #[test]
    fn title_disabled_by_default() {
        let mut u = bare_ui();
        u.update_title();
        assert_eq!(u.scr.title, None);
    }

    #[test]
    fn title_only_on_change() {
        let mut u = bare_ui();
        u.title = Some(String::new());
        u.t = 5.0;
        u.update_title();
        assert!(u.scr.title.take().is_some());
        u.g.level = 42;
        u.t = 5.5;
        u.update_title(); // under a second: nothing
        assert_eq!(u.scr.title, None);
        u.t = 7.0;
        u.update_title();
        assert!(u.scr.title.take().is_some_and(|t| t.contains("Niv. 42")));
        u.t = 9.0;
        u.update_title(); // same level: nothing
        assert_eq!(u.scr.title, None);
    }

    #[test]
    fn title_masked_when_blind() {
        let mut u = bare_ui();
        u.title = Some(String::new());
        u.t = 5.0;
        u.g.challenge = Some("blind");
        u.update_title();
        assert!(u.scr.title.as_deref().is_some_and(|t| t.contains("???")));
    }

    #[test]
    fn numbers_masked() {
        let mut u = bare_ui();
        assert_eq!(u.num(1500.0), "1,50K");
        u.g.challenge = Some("blind");
        assert_eq!(u.num(1500.0), "???");
    }

    // --- Tab bar ---------------------------------------------------------------------

    fn draw_tab_bar(w: i32, qty: Qty, audio: &'static str) -> Vec<(i32, i32, String)> {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        sized(&mut u, 24, w);
        u.qty = qty;
        u.g.audio = audio;
        u.g.talents.insert("u_soldier", 1);
        u.g.radiation = 1e9; // "•" marks on every tab
        u.draw_tab(0, 0, 14, w);
        u.scr.writes.clone()
    }

    #[test]
    fn all_tabs_visible_and_inside_box() {
        for w in [26, 30, 36, 44, 60, 80] {
            let writes = draw_tab_bar(w, Qty::Max, "all");
            let top: Vec<_> = writes
                .iter()
                .filter(|(y, _, s)| *y == 0 && !s.trim_matches(|c| "─┌┐ ".contains(c)).is_empty())
                .collect();
            let labels: String = top.iter().map(|(_, _, s)| s.as_str()).collect();
            for i in 1..=5 {
                assert!(labels.contains(&i.to_string()), "{w}");
            }
            assert!(top.iter().all(|(_, x, s)| x + width(s) < w), "{w}"); // not on the right edge
        }
    }

    #[test]
    fn help_line_complete_variant() {
        for w in [30, 44, 60, 80] {
            let writes = draw_tab_bar(w, Qty::N(1), "all");
            let bottom: Vec<_> = writes
                .iter()
                .filter(|(y, _, s)| *y == 13 && s.contains("aide"))
                .collect();
            assert!(!bottom.is_empty(), "{w}");
            let help = bottom[0].2.trim();
            assert!(help.ends_with('Q') || help.ends_with("aide"), "{w} {help}");
        }
    }

    #[test]
    fn help_line_without_sound() {
        let mut u = bare_ui();
        sized(&mut u, 24, 80);
        u.draw_tab(0, 0, 14, 80);
        assert!(written(&u, "0 son indispo"));
    }

    #[test]
    fn talent_desc_mentions_quantity() {
        let mut u = bare_ui();
        u.tab_w = 40;
        assert!(!u.talent_rows()[2].desc.contains("Entrée"));
        u.qty = Qty::Max;
        assert!(u.talent_rows()[2].desc.contains("tous les rangs"));
        u.qty = Qty::N(10);
        assert!(u.talent_rows()[2].desc.contains("10 rangs"));
    }

    // --- Fog of war ------------------------------------------------------------------

    #[test]
    fn unit_rows_masked() {
        let mut u = bare_ui();
        u.g.challenge = Some("blind");
        u.g.talents.insert("u_soldier", 1);
        u.g.units["soldier"] = 10;
        let r = &u.unit_rows()[0];
        assert!(r.text.contains("???/s"));
        assert!(!r.desc.contains("attente"));
        assert!(!r.desc.contains("manque"));
        assert!(
            row_texts(&u.info_rows())
                .iter()
                .any(|t| t.contains("Niveau max") && t.contains("???"))
        );
    }

    #[test]
    fn infinite_cost_and_wait() {
        assert_eq!(dur(f64::INFINITY), "∞");
        assert_eq!(game::fmt(f64::INFINITY), "∞");
        let mut u = bare_ui();
        u.g.talents.insert("u_soldier", 1);
        u.g.units["soldier"] = 6000;
        u.g.talents.insert("u_mortar", 1);
        u.g.units["mortar"] = 1;
        assert!(u.unit_rows()[0].text.contains('∞'));
    }

    // --- Chest and mutation keys -----------------------------------------------------

    #[test]
    fn chest_keys_win_over_mutation_and_tabs() {
        let mut u = bare_ui();
        u.g.pending_mutation = Some(vec!["spores", "twin", "acid"]);
        u.g.pending_chest = Some(vec!["gauge", "frenzy", "pt"]);
        u.handle(ch('1'));
        assert_eq!(u.g.pending_chest, None);
        assert_eq!(u.g.crit_gauge, 1.0);
        assert!(u.g.mutations.is_empty()); // the mutation waits its turn
        assert_eq!(u.tab, 0);
    }

    #[test]
    fn hidden_mutation_not_picked() {
        let mut u = bare_ui();
        u.g.pending_mutation = Some(vec!["spores", "twin", "acid", "fog", "glow"]);
        u.g.pending_chest = Some(vec!["gauge", "frenzy", "pt"]);
        u.handle(ch('4'));
        assert!(u.g.mutations.is_empty());
        assert_eq!(u.g.pending_chest, Some(vec!["gauge", "frenzy", "pt"]));
        assert_eq!(u.g.sfx.back(), Some(&"nope"));
        assert_eq!(u.tab, 0);
    }

    #[test]
    fn unusable_choice_refused() {
        let mut u = bare_ui();
        u.g.pending_chest = Some(vec!["gauge", "frenzy", "pt"]);
        u.g.crit_gauge = 1.0;
        u.handle(ch('1'));
        assert!(u.g.pending_chest.is_some());
        assert_eq!(u.g.sfx.back(), Some(&"nope"));
    }

    #[test]
    fn unknown_title_glyphs() {
        let mut g = exploded_game();
        g.log("x", game::MsgKind::Good, Some("☢☢"));
        assert_eq!(texts(&bare_ui_with(g, None).banner_lines(60))[0], "☢☢");
    }

    #[test]
    fn chest_overlay_fits() {
        for (lang, texts) in i18n::LANGS {
            i18n::with_lang(texts, || {
                let mut u = bare_ui();
                u.g.pending_chest = Some(vec!["gauge", "frenzy", "pt", "lump"]);
                sized(&mut u, MIN_H, MIN_W);
                u.draw_chest();
                assert!(written(&u, texts.ui.chest_title), "{lang}");
                assert!(written(&u, "4. "), "{lang}");
            });
        }
    }

    #[test]
    fn mutation_overlay_fits() {
        for (lang, texts) in i18n::LANGS {
            i18n::with_lang(texts, || {
                let mut u = bare_ui();
                // Hints of both kinds: a known synergy (spores) and an unknown one (acid).
                u.g.mutations = vec!["mitosis", "fallout"];
                u.g.synergies_found.insert("proliferation");
                u.g.pending_mutation = Some(vec!["spores", "twin", "acid"]);
                sized(&mut u, MIN_H, MIN_W);
                u.draw_mutation();
                assert!(written(&u, texts.ui.mutation_title), "{lang}");
                assert!(written(&u, "3. "), "{lang}");
            });
        }
    }

    // --- End of game report ----------------------------------------------------------

    fn finished() -> UI<BufferScreen> {
        let mut u = bare_ui();
        u.g.mutations = vec!["acid", "fallout"];
        u.g.level = u.g.cap();
        u.g.prestige(None);
        u
    }

    #[test]
    fn report_any_key_closes_and_is_swallowed() {
        let mut u = finished();
        u.handle(ch(' '));
        assert!(!u.g.report_ready);
        assert!(!u.g.exploded); // SPACE didn't detonate the bomb
        u.handle(ch(' '));
        assert!(u.g.exploded);
    }

    #[test]
    fn report_resize_keeps_report() {
        let mut u = finished();
        u.scr.put(0, 0, "x", Style::PLAIN);
        u.handle(Key::Resize);
        assert!(u.g.report_ready);
        assert!(u.scr.writes.is_empty()); // the screen was cleared
    }

    #[test]
    fn report_lines_content() {
        let mut u = finished();
        let r = u.g.last_run.clone().unwrap();
        let lines: HashMap<_, _> = report_lines(&r).into_iter().collect();
        assert!(lines["Gain"].contains('⚛'));
        assert_eq!(lines["Synergies"], "Orage acide");
        assert_eq!(lines["Coffres"], "aucun");
        let run = u.g.last_run.as_mut().unwrap();
        run.chests = vec!["pt".to_string(); 9];
        let lines: HashMap<_, _> = report_lines(run).into_iter().collect();
        assert!(lines["Coffres"].contains("(+3)"));
    }

    #[test]
    fn draw_report_fits() {
        for (lang, texts) in i18n::LANGS {
            for (w, h) in [(MIN_W, MIN_H), (80, 24), (140, 40)] {
                i18n::with_lang(texts, || {
                    let mut u = finished();
                    sized(&mut u, h, w);
                    u.draw_report();
                    let title = (texts.ui.report_prestige)(1);
                    assert!(written(&u, &title), "{lang} {w}");
                });
            }
        }
    }

    #[test]
    fn report_long_values_get_ellipsis() {
        let mut u = finished();
        u.g.last_run.as_mut().unwrap().mutations =
            events::MUTATIONS.iter().map(|m| m.id.to_string()).collect();
        sized(&mut u, 20, 60);
        u.draw_report();
        assert!(u.scr.writes.iter().any(|(_, _, s)| s.ends_with('…')));
    }

    #[test]
    fn report_key_not_swallowed_when_invisible() {
        let mut u = finished();
        u.w = 40; // "Terminal trop petit": the end screen is not drawn
        u.h = 10;
        u.handle(ch(' '));
        assert!(u.g.exploded);
    }

    #[test]
    fn infos_tab_shows_last_run() {
        let u = finished();
        let texts = row_texts(&u.info_rows());
        assert!(texts.iter().any(|t| t.contains("DERNIÈRE PARTIE")));
        assert!(
            texts
                .iter()
                .filter(|t| !t.contains("──"))
                .all(|t| width(t) <= u.tab_w)
        );
    }

    // --- Synergies -------------------------------------------------------------------

    #[test]
    fn synergy_collection_in_achievements() {
        let mut u = bare_ui();
        u.g.synergies_found.insert("storm");
        u.g.mutations = vec!["acid", "fallout"];
        let texts = row_texts(&u.achievement_rows());
        assert!(texts.iter().any(|t| t.contains("SYNERGIES (1/6)")));
        assert!(texts.iter().any(|t| t.contains("Orage acide (active)")));
        assert_eq!(texts.iter().filter(|t| t.contains("???")).count(), 5);
    }

    #[test]
    fn mutation_hint() {
        let mut u = bare_ui();
        u.g.mutations = vec!["spores"];
        u.g.pending_mutation = Some(vec!["mitosis", "glow", "fog"]);
        sized(&mut u, 24, 80);
        u.draw_mutation();
        assert!(written(&u, "synergie inconnue"));
        u.g.synergies_found.insert("proliferation");
        sized(&mut u, 24, 80);
        u.draw_mutation();
        assert!(written(&u, "★ Prolifération"));
    }

    // --- Big titles ------------------------------------------------------------------

    #[test]
    fn newest_message_gets_big_title() {
        let mut g = exploded_game();
        g.log("Coffre : Frénésie !", game::MsgKind::Loot, Some("COFFRE"));
        let lines = texts(&bare_ui_with(g, None).banner_lines(60));
        assert_eq!(lines.len(), 4); // 3 title lines + the banner
        assert!(
            lines[..3]
                .iter()
                .all(|t| t.contains('█') || t.contains('▀'))
        );
        assert!(lines[..3].iter().all(|t| width(t) == width(&lines[0]))); // same width: centering lines up
        assert_eq!(lines[3], "Coffre : Frénésie !");
    }

    #[test]
    fn big_title_expires_unless_important() {
        let mut g = exploded_game();
        g.log("Coffre", game::MsgKind::Loot, Some("COFFRE"));
        g.time += BIG_TIME + 0.1;
        assert_eq!(bare_ui_with(g, None).banner_lines(60).len(), 1);
        let mut g = exploded_game();
        g.log("Coffre", game::MsgKind::Loot, Some("COFFRE"));
        g.time += BIG_TIME + 0.1;
        g.log("Succès", game::MsgKind::Star, Some("SUCCÈS !"));
        g.time += BIG_TIME + 0.1;
        let lines = texts(&bare_ui_with(g, None).banner_lines(60));
        assert_eq!(lines.len(), 3 + 1); // title still there; "Coffre" (6.2 s) has expired
        assert_eq!(lines[3], "Succès");
        let mut g = exploded_game();
        g.log("Succès", game::MsgKind::Star, Some("SUCCÈS !"));
        g.time += MSG_TIME - BIG_EARLY_EXIT + 0.1;
        let lines = texts(&bare_ui_with(g, None).banner_lines(60));
        assert_eq!(lines, ["Succès"]); // the title has gone, its line stays
    }

    #[test]
    fn only_newest_is_big() {
        let mut g = exploded_game();
        g.log("a", game::MsgKind::Good, Some("UN"));
        g.log("b", game::MsgKind::Good, Some("DEUX"));
        assert_eq!(bare_ui_with(g, None).banner_lines(60).len(), 3 + 2);
    }

    #[test]
    fn too_wide_falls_back_to_spaced() {
        let mut g = exploded_game();
        g.log("x", game::MsgKind::Epic, Some("SUPERNOVA"));
        assert_eq!(
            texts(&bare_ui_with(g, None).banner_lines(20))[0],
            "S U P E R N O V A"
        );
    }

    // --- Infos tab -------------------------------------------------------------------

    #[test]
    fn key_5_opens_infos() {
        let mut u = bare_ui();
        u.handle(ch('5'));
        assert_eq!(tr().ui.tabs[u.tab], "Infos");
    }

    #[test]
    fn mutation_keys_take_priority() {
        let mut u = bare_ui();
        u.g.pending_mutation = Some(vec!["spores", "twin", "acid", "fog", "glow"]);
        u.handle(ch('5'));
        assert_eq!(u.tab, 0);
        assert!(u.g.mutations.contains(&"glow"));
    }

    #[test]
    fn infos_content() {
        let mut u = bare_ui();
        u.g.stats.clicks = 1234;
        u.g.stats.total_time = 3725.0;
        u.g.stats.best_prestige_time = None;
        let rows = u.info_rows();
        let texts: Vec<&String> = rows.iter().filter(|r| r.sel).map(|r| &r.text).collect();
        assert!(texts.iter().any(|t| t.contains("ESPACE")));
        assert!(
            texts
                .iter()
                .any(|t| t.contains("Appuis sur ESPACE") && t.contains("1,23K"))
        );
        assert!(texts.iter().any(|t| t.contains("1 h 02 min")));
        assert!(
            texts
                .iter()
                .any(|t| t.contains("Meilleur prestige") && t.contains('—'))
        );
        assert!(texts.iter().all(|t| width(t) <= 40));
    }

    #[test]
    fn infos_rows_scrollable() {
        assert!(bare_ui().info_rows().iter().any(|r| r.sel));
    }

    #[test]
    fn durations() {
        assert_eq!(dur(42.0), "42 s");
        assert_eq!(dur(125.0), "2 min 05 s");
        assert_eq!(dur(7260.0), "2 h 01 min");
    }

    // --- Nanarland and stories -------------------------------------------------------

    #[test]
    fn fizzle_float() {
        let mut u = bare_ui_with(exploded_game_with(fixed(0.0)), None);
        u.g.challenge = Some("nanar");
        u.handle(ch(' '));
        assert_eq!(u.floats.last().unwrap().text, "pschitt…");
    }

    #[test]
    fn floats_capped() {
        let mut u = bare_ui();
        for i in 0..MAX_FLOATS + 3 {
            u.add_float(i.to_string(), Style::PLAIN, 0);
        }
        assert_eq!(u.floats.len(), MAX_FLOATS);
        assert_eq!(u.floats.last().unwrap().text, (MAX_FLOATS + 2).to_string());
    }

    #[test]
    fn story_shown_once_then_cleared() {
        let mut u = bare_ui();
        u.scr.keys.extend([Some(ch('a')), Some(ch('b'))]);
        u.g.pending_story = Some(("nanar", "start"));
        u.story_guard = 0.0;
        assert_eq!(u.show_story(), Ok(()));
        assert_eq!(u.g.pending_story, None);
        assert!(u.scr.keys.is_empty()); // the story screen consumed its two keys
        assert!(written(&u, "Zédé"));
    }

    #[test]
    fn story_interrupted() {
        let mut u = bare_ui();
        u.scr.keys.push_back(Some(Key::Interrupt));
        u.g.pending_story = Some(("nanar", "start"));
        assert_eq!(u.show_story(), Err(Interrupted));
    }

    #[test]
    fn unknown_story_shows_nothing() {
        let mut u = bare_ui();
        u.scr.keys.push_back(Some(ch('a')));
        u.g.pending_story = Some(("nope", "end"));
        assert_eq!(u.show_story(), Ok(()));
        assert_eq!(u.g.pending_story, None);
        assert_eq!(u.scr.keys.len(), 1);
    }

    #[test]
    fn replay_story_plays_its_parts() {
        let mut u = bare_ui();
        u.g.challenges_done.insert("nanar", 1);
        u.tab = 4;
        assert!(
            !row_texts(&u.info_rows())
                .iter()
                .any(|t| t.contains("Revoir le défi"))
        ); // folded
        u.show_stories = true;
        let rows: Vec<Row> = u.info_rows().into_iter().filter(|r| r.sel).collect();
        u.sel[4] = rows
            .iter()
            .position(|r| r.text.contains("Revoir le défi"))
            .unwrap();
        u.scr
            .keys
            .extend([Some(ch('a')), Some(ch('b')), Some(ch('a')), Some(ch('b'))]);
        assert_eq!(u.activate(), Flow::Continue);
        assert!(u.scr.keys.is_empty()); // start then end: two story screens
        assert!(written(&u, "GÉNÉRIQUE"));
    }

    #[test]
    fn replay_row_after_success() {
        let mut u = bare_ui();
        let label = "Revoir le défi Nanarland";
        assert!(
            !row_texts(&u.info_rows())
                .iter()
                .any(|t| t.starts_with(label))
        );
        u.g.challenges_done.insert("nanar", 1);
        u.show_stories = true; // folded behind « Histoires des défis » by default
        assert!(
            row_texts(&u.info_rows())
                .iter()
                .any(|t| t.trim_start().starts_with(label))
        );
    }

    // --- Volume and audio keys -------------------------------------------------------

    #[test]
    fn volume_plus_minus() {
        let sound = FakeSound::new();
        let mut u = bare_ui_with(exploded_game(), Some(&sound));
        u.handle(ch('-'));
        u.handle(ch('-'));
        assert_eq!(u.g.volume, 90);
        u.handle(ch('+'));
        assert_eq!(u.g.volume, 95);
        u.play_sfx();
        assert!((sound.log.borrow().volume - 0.95).abs() < 1e-6);
        assert!(sound.played().contains(&"tick".to_string()));
        assert_eq!(u.g.messages.back().unwrap().text, "Volume : 95 %");
        assert_eq!(
            u.g.messages
                .iter()
                .filter(|m| m.text.starts_with("Volume : "))
                .count(),
            1
        );
    }

    #[test]
    fn zero_cycles_and_announces() {
        let mut u = bare_ui();
        let mut seen = Vec::new();
        for _ in 0..3 {
            u.handle(ch('0'));
            seen.push((u.g.audio, u.g.messages.back().unwrap().text.clone()));
        }
        assert_eq!(
            seen.iter().map(|s| s.0).collect::<Vec<_>>(),
            ["sfx", "off", "all"]
        );
        assert_eq!(seen[1].1, "Son : muet (0 pour le remettre)");
    }

    #[test]
    fn quit_keys() {
        let mut u = bare_ui();
        assert_eq!(u.handle(ch('q')), Flow::Quit);
        assert_eq!(u.handle(Key::Interrupt), Flow::Quit);
        let mut u = finished();
        assert_eq!(u.handle(Key::Interrupt), Flow::Quit); // even on the end screen
    }

    // --- Field -----------------------------------------------------------------------

    /// The mushroom panel with the whole army and every enemy stays on screen.
    #[test]
    fn army_and_enemies_inside_screen() {
        for w in [30, 45, 80, 160] {
            for kind in EnemyKind::ALL {
                let mut u = bare_ui();
                sized(&mut u, 24, w);
                u.g.level = 1_000_000; // longest hit counters
                for n in u.g.units.0.iter_mut() {
                    *n = 150;
                }
                let time = u.g.time;
                u.g.enemies = vec![
                    Enemy {
                        kind,
                        start: time - 3.0,
                        dur: 10.0,
                        hits: 1,
                    },
                    Enemy {
                        kind,
                        start: time - 1.0,
                        dur: 10.0,
                        hits: 0,
                    },
                ];
                for t in [0.0, 0.7] {
                    u.t = t;
                    u.draw_mushroom(0, 0, 23, w, 0.05);
                }
                assert!(!u.scr.writes.is_empty());
            }
        }
    }

    #[test]
    fn enemy_banner() {
        let mut u = bare_ui();
        u.g.enemies = vec![Enemy {
            kind: EnemyKind::Drone,
            start: 0.0,
            dur: 15.0,
            hits: 0,
        }];
        assert!(u.banner_lines(40)[0].0.contains("Drone espion"));
        assert_eq!(u.music_track(), "tension");
        u.g.enemies.push(Enemy {
            kind: EnemyKind::Saboteur,
            start: 0.0,
            dur: 12.0,
            hits: 0,
        });
        assert!(u.banner_lines(40)[0].0.contains("2 ennemis"));
    }

    #[test]
    fn challenge_reward_visible() {
        let mut u = bare_ui();
        u.g.prestige_count = 1;
        u.tab = 2;
        u.sel[2] = 1; // first challenge
        sized(&mut u, 30, 60);
        u.draw_tab(10, 0, 19, 60);
        let text: String = u.scr.writes.iter().map(|w| w.2.as_str()).collect();
        assert!(text.contains("Récompense"));
    }

    #[test]
    fn ascete_no_talent_dot() {
        let mut u = bare_ui();
        u.g.challenge = Some("ascete");
        for t in ["u_soldier", "u_mortar", "u_artillery"] {
            u.g.talents.insert(t, 1);
        }
        u.g.pt = 2; // enough for t_click (1 PT) but not the next unlock (8 PT)
        assert!(!u.tab_has_news(1));
    }

    // --- Unit sprite -----------------------------------------------------------------

    fn draw_units(w: i32, h: i32, sel: usize) -> Vec<(i32, i32, String)> {
        let mut u = bare_ui();
        u.g.talents.insert("u_soldier", 1);
        u.sel[0] = sel;
        sized(&mut u, 30, w);
        u.draw_tab(10, 0, h, w);
        u.scr.writes.clone()
    }

    /// Single characters written on line y right of x0 (the sprite is drawn character by character).
    fn art_line(writes: &[(i32, i32, String)], y: i32) -> String {
        let mut cells: Vec<_> = writes
            .iter()
            .filter(|(wy, wx, s)| *wy == y && *wx >= 60 && width(s) == 1 && s != "│")
            .collect();
        cells.sort();
        cells.iter().map(|(_, _, s)| s.as_str()).collect()
    }

    #[test]
    fn selected_unit_sprite_beside_desc() {
        let writes = draw_units(80, 19, 0);
        let lines: Vec<String> = [25, 26, 27].iter().map(|&y| art_line(&writes, y)).collect();
        assert_eq!(lines, ["o", "/|=", "/\\"]);
        assert!(writes.contains(&(26, 75, "/".to_string()))); // flush with the right edge of the description area
        let desc = &writes.iter().find(|w| w.0 == 25 && w.1 == 2).unwrap().2;
        assert!(2 + width(desc) <= 75 - 2); // the text doesn't run under the drawing
    }

    #[test]
    fn locked_unit_silhouette() {
        let writes = draw_units(80, 19, 1); // "???" line: the mortar to unlock
        assert_eq!(art_line(&writes, 27), "[_]");
    }

    #[test]
    fn no_sprite_when_narrow_or_short() {
        for (w, h) in [(36, 19), (80, 8)] {
            let writes = draw_units(w, h, 0);
            assert!(!writes.iter().any(|w| w.2 == "="), "{w} {h}");
        }
    }

    // --- Whole screen ----------------------------------------------------------------

    /// At their widest, the tabs show every key help line whole.
    #[test]
    fn key_help_fits_the_widest_tab() {
        for (lang, texts) in i18n::LANGS {
            i18n::with_lang(texts, || {
                let mut u = bare_ui();
                u.scr = BufferScreen::new(30, 240);
                u.draw(0.0);
                // The rule, then a row per key: unclipped, it ends with the whole text.
                for (row, (_, what)) in u.info_rows()[1..].iter().zip(key_help()) {
                    assert!(row.text.ends_with(&what), "{lang}: {}", row.text);
                }
            });
        }
    }

    /// On a wide terminal, the tabs stop growing: the extra columns go to the mushroom panel.
    #[test]
    fn wide_terminal_widens_the_mushroom_panel() {
        let title = format!(" {} ", tr().ui.stats_title);
        // 45 % of the width up to 152 columns, all but RIGHT_MAX_W from 153.
        for (w, left_w) in [
            (100, 45),
            (152, 68),
            (153, 153 - RIGHT_MAX_W),
            (240, 240 - RIGHT_MAX_W),
        ] {
            let mut u = bare_ui();
            u.scr = BufferScreen::new(30, w);
            u.draw(0.0);
            let at = (0, left_w + PANEL_PAD, title.clone());
            assert!(u.scr.writes.contains(&at), "{w}");
        }
    }

    /// Full frames at the minimum size and larger, with overlays, never write off screen.
    #[test]
    fn full_draw_fits() {
        for (lang, texts) in i18n::LANGS {
            i18n::with_lang(texts, || full_draws_fit(lang));
        }
    }

    fn full_draws_fit(lang: &str) {
        for (h, w) in [(MIN_H, MIN_W), (24, 80), (40, 140), (65, 240), (10, 30)] {
            let mut u = bare_ui();
            u.scr = BufferScreen::new(h, w);
            u.g.talents.insert("u_soldier", 1);
            u.g.units["soldier"] = 12;
            u.g.pending_mutation = Some(vec!["spores", "twin", "acid"]);
            u.g.plane = Some(game::Plane {
                letter: 'a',
                start: u.g.time,
                dur: 10.0,
            });
            u.g.crit_gauge = 1.0;
            events::crit_press(&mut u.g);
            for _ in 0..30 {
                u.draw(0.05);
            }
            assert!(u.scr.refreshes > 0, "{lang}");
        }
    }
}
