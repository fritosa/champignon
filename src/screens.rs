//! Full-screen screens outside a game: introduction and challenge stories (typewriter text), game
//! selection menu at launch. They drive the `Screen` themselves.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::game::{fmt, wall_time};
use crate::i18n::screens::{AgoUnit, Pages};
use crate::i18n::tr;
use crate::saves::{MAIN_SLOT, RawSave, Saves};
use crate::sound::SoundSink;
use crate::term::{Key, Screen, Style, width};

pub const INTRO_SPEED: f64 = 80.0; // characters/s
pub const STORY_GUARD: f64 = 1.0; // seconds of keys ignored by a challenge story (players are still hammering SPACE)
const TYPE_FRAME: Duration = Duration::from_millis(30);
const TICK_EVERY: i32 = 6; // typewriter sound every TICK_EVERY characters
const TEXT_MARGIN: i32 = 4; // columns kept around the typed text…
const TEXT_MIN_W: i32 = 10; // …whose width stays within these bounds
const TEXT_MAX_W: i32 = 70;
const TEXT_TOP: i32 = 2; // rows of the title and the blank line above the text
const TEXT_CHROME_H: i32 = 4; // rows centered with the text: title, blank line, two below

/// Story of a challenge part ("start" or "end"), if it has one, in the current language.
pub fn story(challenge: &str, part: &str) -> Option<Pages> {
    tr().screens
        .stories
        .iter()
        .find(|((c, p), _)| *c == challenge && *p == part)
        .map(|(_, pages)| *pages)
}

/// Ctrl+C on a full screen: the caller quits (with a save during a game).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupted;

/// Seconds since an arbitrary fixed point (like time.monotonic).
pub fn monotonic() -> f64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_secs_f64()
}

// --- Text helpers -------------------------------------------------------------------

/// Python's floor division for a positive divisor.
pub(crate) fn fdiv(a: i32, b: i32) -> i32 {
    a.div_euclid(b)
}

/// Characters [a, b) of s, clamped like a Python slice with non-negative bounds.
pub(crate) fn slice(s: &str, a: i32, b: i32) -> String {
    let a = a.max(0) as usize;
    let b = b.max(0) as usize;
    if b <= a {
        return String::new();
    }
    s.chars().skip(a).take(b - a).collect()
}

/// The first n characters of s (s[:n]; a negative n counts from the end, like Python).
pub(crate) fn head(s: &str, n: i32) -> String {
    if n < 0 {
        return slice(s, 0, width(s) + n);
    }
    s.chars().take(n as usize).collect()
}

/// str.center: the extra space of an odd margin goes left when the width is odd (CPython's rule).
pub(crate) fn center(s: &str, w: i32) -> String {
    let marg = w - width(s);
    if marg <= 0 {
        return s.to_string();
    }
    let left = marg / 2 + (marg & w & 1);
    format!(
        "{}{s}{}",
        " ".repeat(left as usize),
        " ".repeat((marg - left) as usize)
    )
}

/// Whitespace for textwrap: ASCII only, so non-breaking spaces never break a line.
fn is_wrap_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ')
}

/// Letter in the sense of Python's `[^\d\W]` (word character that is not a digit).
fn is_letter(c: Option<&char>) -> bool {
    c.is_some_and(|&c| c.is_alphabetic() || c == '_')
}

/// Chunks like textwrap's wordsep_re: whitespace runs, words, and hyphenated words cut after their hyphens.
fn wrap_chunks(chars: &[char]) -> Vec<String> {
    let at = |i: isize| if i < 0 { None } else { chars.get(i as usize) };
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        if is_wrap_space(chars[i]) {
            while i < chars.len() && is_wrap_space(chars[i]) {
                i += 1;
            }
        } else {
            i += 1; // at least one character before a hyphen break
            while i < chars.len() && !is_wrap_space(chars[i]) {
                let j = i as isize;
                if chars[i] == '-' {
                    let behind = (is_letter(at(j - 2)) && is_letter(at(j - 1)))
                        || (is_letter(at(j - 3))
                            && at(j - 2) == Some(&'-')
                            && is_letter(at(j - 1)));
                    let ahead = is_letter(at(j + 1))
                        && (is_letter(at(j + 2))
                            || (at(j + 2) == Some(&'-') && is_letter(at(j + 3))));
                    if behind && ahead {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
        }
        chunks.push(chars[start..i].iter().collect());
    }
    chunks
}

/// textwrap.wrap(text, w) with its default options (long words broken, hyphens as break points).
pub fn wrap(text: &str, w: i32) -> Vec<String> {
    let w = w.max(1) as usize;
    let chars: Vec<char> = text
        .chars()
        .map(|c| if is_wrap_space(c) { ' ' } else { c })
        .collect();
    let blank = |s: &str| s.chars().all(is_wrap_space);
    let len = |s: &str| s.chars().count();
    let mut chunks = wrap_chunks(&chars);
    chunks.reverse();
    let mut lines = Vec::new();
    while !chunks.is_empty() {
        let mut line: Vec<String> = Vec::new();
        let mut cur = 0;
        if !lines.is_empty() && chunks.last().is_some_and(|c| blank(c)) {
            chunks.pop();
        }
        while let Some(chunk) = chunks.last() {
            if cur + len(chunk) > w {
                break;
            }
            cur += len(chunk);
            line.push(chunks.pop().unwrap_or_default());
        }
        if let Some(chunk) = chunks.last_mut()
            && len(chunk) > w
        {
            // Long word: cut it, after its last hyphen that fits if there is one.
            let space_left = w - cur;
            let cs: Vec<char> = chunk.chars().collect();
            let mut end = space_left;
            if let Some(hyphen) = cs[..space_left].iter().rposition(|&c| c == '-')
                && hyphen > 0
                && cs[..hyphen].iter().any(|&c| c != '-')
            {
                end = hyphen + 1;
            }
            line.push(cs[..end].iter().collect());
            *chunk = cs[end..].iter().collect();
        }
        if line.last().is_some_and(|c| blank(c)) {
            line.pop();
        }
        if !line.is_empty() {
            lines.push(line.concat());
        }
    }
    lines
}

/// Lenient write: clips at the edge, never on the last column (nor anywhere on a tiny window).
fn menu_put(scr: &mut dyn Screen, y: i32, x: i32, text: &str, style: Style) {
    let (h, w) = scr.size();
    if 0 <= y && y < h && 0 <= x && x < w - 1 && !text.is_empty() {
        scr.put(y, x, &head(text, w - 1 - x), style);
    }
}

// --- Typewriter screen ------------------------------------------------------------

/// Typewriter text (introduction, or a challenge story). One key shows everything, the next one
/// closes. The first paragraph is bold. Keys during the first guard seconds are ignored.
/// then: what the key does once everything is shown (`ScreenTexts::start` or `resume`).
pub fn intro(
    scr: &mut dyn Screen,
    sound: Option<&dyn SoundSink>,
    pages: &[&str],
    guard: f64,
    then: &str,
) -> Result<(), Interrupted> {
    intro_with_clock(scr, sound, pages, guard, then, &mut monotonic)
}

/// `intro` with an injectable clock (seconds, monotonic).
pub fn intro_with_clock(
    scr: &mut dyn Screen,
    sound: Option<&dyn SoundSink>,
    pages: &[&str],
    guard: f64,
    then: &str,
    clock: &mut dyn FnMut() -> f64,
) -> Result<(), Interrupted> {
    let mut shown = 0.0;
    let mut start = clock();
    let opened = start;
    let bold = Style::PLAIN.bold();
    let texts = &tr().screens;
    loop {
        let (h, w) = scr.size();
        let bw = (w - TEXT_MARGIN).clamp(TEXT_MIN_W, TEXT_MAX_W);
        let mut lines: Vec<(String, bool)> = Vec::new();
        for (i, par) in pages.iter().enumerate() {
            lines.extend(
                wrap(&(tr().typography)(par), bw)
                    .into_iter()
                    .map(|line| (line, i == 0)),
            );
            lines.push((String::new(), false));
        }
        lines.truncate((h - TEXT_TOP - 1).max(1) as usize); // overflow is neither typed nor waited for
        let total: i32 = lines.iter().map(|(line, _)| width(line)).sum();
        let before = shown as i32;
        shown = f64::from(total).min((clock() - start) * INTRO_SPEED);
        if let Some(sound) = sound
            && shown as i32 / TICK_EVERY != before / TICK_EVERY
            && shown < f64::from(total)
        {
            sound.play("tick");
        }
        scr.erase();
        let y = fdiv(h - lines.len() as i32 - TEXT_CHROME_H, 2).max(0);
        let x = fdiv(w - bw, 2).max(0);
        menu_put(
            scr,
            y,
            fdiv(w - width(texts.title), 2).max(0),
            texts.title,
            bold,
        );
        let mut left = shown as i32;
        for (i, (line, is_bold)) in lines.iter().enumerate() {
            if y + TEXT_TOP + i as i32 >= h - 1 {
                break;
            }
            let style = if *is_bold { bold } else { Style::PLAIN };
            menu_put(scr, y + TEXT_TOP + i as i32, x, &head(line, left), style);
            left = (left - width(line)).max(0);
        }
        let done = shown >= f64::from(total);
        let msg = if done {
            (texts.press_key)(then)
        } else {
            texts.show_all.to_string()
        };
        let style = if done { bold } else { Style::PLAIN.dim() };
        menu_put(scr, h - 1, fdiv(w - width(&msg), 2).max(0), &msg, style);
        scr.refresh();
        let key = scr.read_key(Some(TYPE_FRAME));
        if key == Some(Key::Interrupt) {
            return Err(Interrupted);
        }
        if matches!(key, None | Some(Key::Resize)) || clock() - opened < guard {
            continue;
        }
        if done {
            return Ok(());
        }
        start = clock() - f64::from(total) / INTRO_SPEED;
    }
}

// --- Start menu ------------------------------------------------------------------

/// A numeric field of a raw save, 0 if missing or of another type.
fn number_or_zero(d: &RawSave, key: &str) -> f64 {
    d.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Units of `ago`, largest first: (name, seconds).
const AGO_UNITS: [(AgoUnit, f64); 3] = [
    (AgoUnit::Days, 86400.0),
    (AgoUnit::Hours, 3600.0),
    (AgoUnit::Minutes, 60.0),
];

// Start menu layout: header (title, blank, question, blank), 2 rows per game, ↓ and help rows.
const MENU_HEADER_H: i32 = 4;
const MENU_CHROME_H: i32 = MENU_HEADER_H + 2;
const MENU_ROW_H: i32 = 2; // label, then details
const MENU_QUESTION_ROW: i32 = 2;
const MENU_HALF_W: i32 = 30; // the list starts this many columns left of the center
const MENU_DETAIL_X: i32 = 4; // details indent under a label

/// How long ago `wall` was, `now` being the current wall-clock time.
pub fn ago(wall: f64, now: f64) -> String {
    let s = (now - wall).max(0.0);
    let texts = &tr().screens;
    for (unit, n) in AGO_UNITS {
        if s >= n {
            return (texts.ago)((s / n).floor() as i64, unit);
        }
    }
    texts.just_now.to_string()
}

/// Game selection at launch. Returns the save path, or None to quit.
pub fn save_menu(
    scr: &mut dyn Screen,
    saves: &Saves,
    slots: &[(u32, PathBuf, RawSave)],
    sound: Option<&dyn SoundSink>,
) -> Option<PathBuf> {
    let now = wall_time();
    let texts = &tr().screens;
    let mut entries: Vec<(String, String, Option<PathBuf>)> = Vec::new();
    for (n, path, d) in slots {
        let name = if *n == MAIN_SLOT {
            texts.main_slot.to_string()
        } else {
            (texts.slot)(*n)
        };
        let [level, isotopes, prestiges, wall] =
            ["level", "isotopes", "prestige_count", "wall_time"].map(|k| number_or_zero(d, k));
        let mut detail = (texts.game_detail)(&fmt(level), &fmt(isotopes), prestiges as i64);
        if wall != 0.0 {
            detail += &format!(" · {}", ago(wall, now));
        }
        entries.push(((texts.continue_game)(&name), detail, Some(path.clone())));
    }
    let free = saves.free_slot();
    entries.push((
        texts.new_game.to_string(),
        if free.is_some() {
            texts.new_game_desc.to_string()
        } else {
            texts.no_free_slot.to_string()
        },
        free.map(|n| saves.slot_path(n)),
    ));
    let count = entries.len() as i32;
    let (mut sel, mut top) = (0i32, 0i32);
    let bold = Style::PLAIN.bold();
    loop {
        let (h, w) = scr.size();
        scr.erase();
        // Header (4 lines), scrolling list (2 lines per game), help on the last line.
        let visible = fdiv(h - MENU_CHROME_H, MENU_ROW_H).max(1);
        let y = (fdiv(h, 2) - count - MENU_CHROME_H / 2)
            .min(h - MENU_CHROME_H - MENU_ROW_H * count.min(visible))
            .max(0);
        top = top.min(sel).max(sel - visible + 1);
        menu_put(
            scr,
            y,
            fdiv(w - width(texts.title), 2).max(0),
            texts.title,
            bold,
        );
        let q = texts.question;
        menu_put(
            scr,
            y + MENU_QUESTION_ROW,
            fdiv(w - width(q), 2).max(0),
            q,
            Style::PLAIN,
        );
        let x = (fdiv(w, 2) - MENU_HALF_W).max(0);
        for (row, i) in (top..count.min(top + visible)).enumerate() {
            let (label, detail, _) = &entries[i as usize];
            let ly = y + MENU_HEADER_H + MENU_ROW_H * row as i32;
            let (mark, style) = if i == sel {
                ('▶', bold.reverse())
            } else {
                (' ', bold)
            };
            menu_put(scr, ly, x, &format!(" {mark} {label} "), style);
            menu_put(scr, ly + 1, x + MENU_DETAIL_X, detail, Style::PLAIN.dim());
        }
        if top > 0 {
            menu_put(scr, y + MENU_HEADER_H - 1, x, "↑", Style::PLAIN);
        }
        if top + visible < count {
            menu_put(scr, h - 2, x, "↓", Style::PLAIN);
        }
        let help = texts.menu_help;
        menu_put(
            scr,
            h - 1,
            fdiv(w - width(help), 2).max(0),
            help,
            Style::PLAIN.dim(),
        );
        scr.refresh();
        match scr.read_key(None) {
            Some(Key::Up | Key::Char('k')) => sel = (sel - 1).rem_euclid(count),
            Some(Key::Down | Key::Char('j') | Key::Tab) => sel = (sel + 1).rem_euclid(count),
            Some(Key::Enter) => {
                if let Some(path) = &entries[sel as usize].2 {
                    return Some(path.clone());
                }
                if let Some(sound) = sound {
                    sound.play("nope");
                }
                continue;
            }
            Some(Key::Char('q' | 'Q') | Key::Interrupt) => return None,
            _ => continue,
        }
        if let Some(sound) = sound {
            sound.play("tick");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::data;
    use crate::game::Game;
    use crate::i18n;
    use crate::saves::TempDir;
    use crate::term::BufferScreen;
    use crate::ui::tests::FakeSound;

    fn keys(chars: &str) -> Vec<Option<Key>> {
        chars.chars().map(|c| Some(Key::Char(c))).collect()
    }

    fn written(scr: &BufferScreen, needle: &str) -> bool {
        scr.writes.iter().any(|(_, _, s)| s.contains(needle))
    }

    #[test]
    fn ago_units() {
        let now = 1_000_000.0;
        assert_eq!(ago(now - 30.0, now), "à l'instant");
        assert_eq!(ago(now - 125.0, now), "il y a 2 min");
        assert_eq!(ago(now - 7300.0, now), "il y a 2 h");
        assert_eq!(ago(now - 3.0 * 86400.0 - 5.0, now), "il y a 3 j");
    }

    #[test]
    fn wrap_like_textwrap() {
        assert_eq!(wrap("un deux trois", 7), ["un deux", "trois"]);
        assert_eq!(wrap("  a  b ", 10), ["  a  b"]);
        assert_eq!(
            wrap("Pots-de-vin pour tous", 6),
            ["Pots-", "de-vin", "pour", "tous"]
        );
        assert_eq!(wrap("a-b-cd ef", 3), ["a-", "b-", "cd", "ef"]);
        assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("x carton-pâte", 9), ["x carton-", "pâte"]);
        assert_eq!(wrap("a\u{a0}b c", 3), ["a\u{a0}b", "c"]);
        assert!(wrap("", 5).is_empty());
        assert_eq!(center("ab", 5), "  ab ");
        assert_eq!(center("ab", 6), "  ab  ");
        assert_eq!(center("abc", 6), " abc  ");
    }

    struct MenuFixture {
        _dir: TempDir,
        saves: Saves,
        slots: Vec<(u32, PathBuf, RawSave)>,
    }

    impl MenuFixture {
        fn new(games: u32) -> MenuFixture {
            let dir = TempDir::new();
            let saves = Saves::new(&dir.0);
            for n in 1..=games {
                Game::default().save(&saves.slot_path(n)).unwrap();
            }
            let slots = saves.save_slots();
            MenuFixture {
                _dir: dir,
                saves,
                slots,
            }
        }

        fn menu(
            &self,
            keys: Vec<Option<Key>>,
            size: (i32, i32),
        ) -> (Option<PathBuf>, BufferScreen) {
            let mut scr = BufferScreen::with_keys(size.0, size.1, keys);
            let sound = FakeSound::new();
            let path = save_menu(&mut scr, &self.saves, &self.slots, Some(&sound));
            (path, scr)
        }
    }

    #[test]
    fn menu_enter_continues_main() {
        let f = MenuFixture::new(2);
        let (path, _) = f.menu(vec![Some(Key::Enter)], (30, 100));
        assert_eq!(path, Some(f.saves.slot_path(1)));
    }

    #[test]
    fn menu_navigate_to_second() {
        let f = MenuFixture::new(2);
        let (path, _) = f.menu(
            vec![Some(Key::Down), Some(Key::Char('z')), Some(Key::Enter)],
            (30, 100),
        );
        assert_eq!(path, Some(f.saves.slot_path(2)));
    }

    #[test]
    fn menu_new_game_uses_free_slot() {
        let f = MenuFixture::new(2);
        let (path, scr) = f.menu(vec![Some(Key::Up), Some(Key::Enter)], (30, 100)); // wraps up to "Nouvelle partie"
        assert_eq!(path, Some(f.saves.slot_path(3)));
        assert!(written(&scr, "Nouvelle partie"));
        assert!(written(&scr, "Continuer : Partie 2"));
    }

    #[test]
    fn menu_quit() {
        let f = MenuFixture::new(2);
        assert_eq!(f.menu(keys("q"), (30, 100)).0, None);
        assert_eq!(f.menu(vec![Some(Key::Interrupt)], (30, 100)).0, None);
    }

    #[test]
    fn menu_escape_does_not_quit() {
        let f = MenuFixture::new(2);
        let (path, _) = f.menu(vec![Some(Key::Char('\x1b')), Some(Key::Enter)], (30, 100));
        assert_eq!(path, Some(f.saves.slot_path(1)));
    }

    #[test]
    fn menu_tiny_terminals() {
        let f = MenuFixture::new(2);
        for size in [(5, 20), (24, 3), (8, 1), (1, 80), (2, 2)] {
            let (path, _) = f.menu(vec![Some(Key::Enter)], size);
            assert_eq!(path, Some(f.saves.slot_path(1)), "{size:?}");
        }
    }

    #[test]
    fn menu_scrolls_to_selection() {
        let f = MenuFixture::new(15);
        let (path, scr) = f.menu(vec![Some(Key::Up), Some(Key::Enter)], (24, 80));
        assert_eq!(path, Some(f.saves.slot_path(16)));
        assert!(written(&scr, "Nouvelle partie"));
        assert!(!written(&scr, "Partie principale")); // scrolled off screen
    }

    #[test]
    fn menu_bad_field_types() {
        let mut f = MenuFixture::new(1);
        let raw = serde_json::json!({"level": "5", "isotopes": null, "prestige_count": [], "wall_time": null});
        let Value::Object(d) = raw else {
            unreachable!()
        };
        f.slots = vec![(1, f.saves.slot_path(1), d)];
        let (path, scr) = f.menu(vec![Some(Key::Enter)], (30, 100));
        assert_eq!(path, Some(f.saves.slot_path(1)));
        assert!(written(&scr, "niveau 0"));
    }

    #[test]
    fn menu_all_slots_taken() {
        let f = MenuFixture::new(2);
        for n in 3..=crate::saves::MAX_SLOTS {
            fs::write(f.saves.slot_path(n), "abîmé").unwrap(); // unreadable, but taken
        }
        let mut scr = BufferScreen::with_keys(30, 100, vec![Some(Key::Up), Some(Key::Enter)]);
        scr.keys.extend(keys("q"));
        let sound = FakeSound::new();
        assert_eq!(save_menu(&mut scr, &f.saves, &f.slots, Some(&sound)), None);
        assert!(sound.played().contains(&"nope".to_string()));
        assert!(written(&scr, "Plus d'emplacement libre"));
    }

    #[test]
    fn intro_first_key_reveals_second_closes() {
        let mut scr = BufferScreen::with_keys(30, 100, [None]);
        scr.keys.extend(keys("ab"));
        let sound = FakeSound::new();
        assert_eq!(
            intro(&mut scr, Some(&sound), tr().screens.intro, 0.0, "commencer"),
            Ok(())
        );
        assert!(scr.keys.is_empty());
        assert!(written(&scr, "Morille"));
        assert!(written(&scr, "commencer"));
    }

    #[test]
    fn intro_small_screen() {
        let mut scr = BufferScreen::with_keys(8, 20, keys("ab"));
        assert_eq!(
            intro(&mut scr, None, tr().screens.intro, 0.0, "commencer"),
            Ok(())
        );
    }

    #[test]
    fn intro_interrupted() {
        let mut scr = BufferScreen::with_keys(30, 100, [Some(Key::Interrupt)]);
        assert_eq!(
            intro(&mut scr, None, tr().screens.intro, 60.0, "commencer"),
            Err(Interrupted)
        );
    }

    #[test]
    fn texts_fit_80x24() {
        // The default terminal: the last paragraph (intro or story) must stay visible, in every language.
        for (lang, texts) in i18n::LANGS {
            let s = &texts.screens;
            for pages in std::iter::once(s.intro).chain(s.stories.iter().map(|(_, p)| *p)) {
                let mut scr = BufferScreen::with_keys(24, 80, keys("ab"));
                i18n::with_lang(texts, || intro(&mut scr, None, pages, 0.0, s.start)).unwrap();
                let last_par = (texts.typography)(pages[pages.len() - 1]);
                let last = last_par.split_whitespace().last().unwrap();
                assert!(written(&scr, last), "{lang}: {}", pages[0]);
            }
        }
    }

    #[test]
    fn story_ignores_keys_at_first() {
        // The player is still hammering SPACE when the story opens: those presses don't close it.
        let tick = Cell::new(0.0);
        let mut clock = || {
            tick.set(tick.get() + 0.3);
            tick.get()
        };
        let mut scr = BufferScreen::with_keys(30, 100, keys("    "));
        let pages = story("nanar", "end").unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| {
            intro_with_clock(&mut scr, None, pages, 60.0, "continuer", &mut clock)
        }));
        assert!(result.is_err()); // still open when the keys run out
        assert!(scr.keys.is_empty()); // all 4 presses were read (and ignored)
        assert!(!written(&scr, "continuer") && !written(&scr, "commencer")); // text not fully shown yet
    }

    #[test]
    fn stories_match_challenges() {
        for (lang, texts) in i18n::LANGS {
            for ((cid, part), _) in texts.screens.stories {
                assert!(data::find_challenge(cid).is_some(), "{lang}: {cid}");
                assert!(["start", "end"].contains(part), "{lang}: {cid}");
            }
        }
    }

    #[test]
    fn every_challenge_has_both_stories() {
        for (lang, texts) in i18n::LANGS {
            i18n::with_lang(texts, || {
                for c in &data::CHALLENGES {
                    assert!(story(c.id, "start").is_some(), "{lang}: {}", c.id);
                    assert!(story(c.id, "end").is_some(), "{lang}: {}", c.id);
                }
            });
        }
    }
}
