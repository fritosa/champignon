//! Full-screen screens outside a game: introduction and challenge stories (typewriter text), game
//! selection menu at launch. They drive the `Screen` themselves.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::game::{fmt, wall_time};
use crate::saves::{RawSave, Saves};
use crate::sound::SoundSink;
use crate::term::{Key, Screen, Style, width};

pub const TITLE: &str = "☢  CHAMPIGNON ATOMIQUE  ☢";

// --- Introduction -----------------------------------------------------------------

pub const INTRO: &[&str] = &[
    "ANNÉE 1962 ¾. QUELQUE PART ENTRE DEUX FRONTIÈRES. (STOCK-SHOT D'UN DÉSERT, PUIS D'UNE FORÊT, PUIS DU \
     MÊME DÉSERT.)",
    "Le professeur Amédée Morille, mycologue, artificier amateur et ceinture noire de pétanque, fait LA \
     découverte : les champignons atomiques sont VIVANTS ! Ils mangent de la radiation. « C'est \
     impossible ! », s'écrie son assistante, face à la mauvaise caméra.",
    "Une nuit d'orage (tournée en plein jour, filtre bleu), le professeur disparaît en zone interdite. On ne \
     retrouve que sa casquette et une bombe au fond de TON jardin, avec un post-it : « Arrose-la. »",
    "Te voici jardinier en chef. Le ministère envoie des soldats « pour aider », l'ONU des inspecteurs « pour \
     vérifier ». Saboteurs, drones en fil de pêche et manifestants débarquent sans invitation. Le voisin porte \
     plainte ; sa perruque change de couleur entre deux plans.",
    "Ta mission : le faire pousser jusqu'aux étoiles. Le professeur aurait été fier. Probablement : son \
     doublage était inaudible.",
    "CET ÉTÉ... APPUIE SUR ESPACE.",
];
pub const INTRO_SPEED: f64 = 80.0; // characters/s
pub const STORY_GUARD: f64 = 1.0; // seconds of keys ignored by a challenge story (players are still hammering SPACE)
const TYPE_FRAME: Duration = Duration::from_millis(30);

/// Challenge stories, shown like the introduction at launch ("start") and on success ("end").
pub const STORIES: &[((&str, &str), &[&str])] = &[
    (
        ("nanar", "start"),
        &[
            "NANARLAND — UN FILM DE GÉRARD ZÉDÉ",
            "Gérard Zédé, producteur des « Tomates tueuses du Larzac » et de « Mon voisin le yéti 3 : le retour \
             du yéti 2 », a vu ton champignon dans le journal. Il a eu une illumination. Et peut-être aussi une \
             légère insolation.",
            "Son prochain chef-d'œuvre s'appellera « LE CHAMPIGNON QUI VOULAIT CONQUÉRIR LE MONDE (ET UN PEU LA \
             BELGIQUE) ». Budget total : 312 francs, un sandwich au pâté et une caméra empruntée au beau-frère.",
            "Les explosions seront bruitées à la bouche par son neveu, qui est enrhumé : une fois sur trois, ça fait \
             pschitt. La moitié de ton armée est réquisitionnée comme figurants et ne produit plus que des \
             autographes.",
            "Ton rôle : faire monter le champignon jusqu'au niveau 100 avant la fin du tournage. Silence... \
             moteur... ça tourne ! (La perche est dans le champ. Personne ne la retirera.)",
        ],
    ),
    (
        ("nanar", "end"),
        &[
            "NANARLAND — GÉNÉRIQUE DE FIN",
            "Le film sort un mardi pluvieux dans la salle des fêtes de Saint-Glinglin-sur-Marne. Quatorze \
             spectateurs. À la scène la plus dramatique, les quatorze pleurent de rire. Le lendemain, ils \
             reviennent avec leurs amis.",
            "Les critiques sont unanimes : « raté de A à Z », « le perchman mérite un César », « on y retourne \
             demain ». Six mois plus tard, la séance de minuit affiche complet tous les vendredis, et la salle \
             entière crie « PSCHITT ! » à chaque explosion. Ce n'est pas un navet : c'est un nanar, et on l'aime.",
            "Ton champignon est devenu CULTE : ses détonations critiques frappent désormais ×15. Gérard t'offre \
             les décors en carton-pâte, à porter comme skin.",
            "Tout en bas du générique, une ligne intrigue les fans : « Lézard géant : A. Morille ». Le professeur \
             ne répond toujours pas aux lettres.",
        ],
    ),
];

/// Story of a challenge part ("start" or "end"), if it has one.
pub fn story(challenge: &str, part: &str) -> Option<&'static [&'static str]> {
    STORIES
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

/// French-style non-breaking spaces: no « or » : ; ! ? left orphaned at a line edge.
pub fn nbsp(text: &str) -> String {
    let text = text.replace("« ", "«\u{a0}");
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let before_mark = chars.get(i + 1).is_some_and(|n| "»:;!?".contains(*n));
        out.push(if c == ' ' && before_mark { '\u{a0}' } else { c });
    }
    out
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
/// then: end of the prompt « Appuie sur une touche pour … ».
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
    loop {
        let (h, w) = scr.size();
        let bw = (w - 4).clamp(10, 70);
        let mut lines: Vec<(String, bool)> = Vec::new();
        for (i, par) in pages.iter().enumerate() {
            lines.extend(wrap(&nbsp(par), bw).into_iter().map(|line| (line, i == 0)));
            lines.push((String::new(), false));
        }
        lines.truncate((h - 3).max(1) as usize); // overflow is neither typed nor waited for
        let total: i32 = lines.iter().map(|(line, _)| width(line)).sum();
        let before = shown as i32;
        shown = f64::from(total).min((clock() - start) * INTRO_SPEED);
        if let Some(sound) = sound
            && shown as i32 / 6 != before / 6
            && shown < f64::from(total)
        {
            sound.play("tick");
        }
        scr.erase();
        let y = fdiv(h - lines.len() as i32 - 4, 2).max(0);
        let x = fdiv(w - bw, 2).max(0);
        menu_put(scr, y, fdiv(w - width(TITLE), 2).max(0), TITLE, bold);
        let mut left = shown as i32;
        for (i, (line, is_bold)) in lines.iter().enumerate() {
            if y + 2 + i as i32 >= h - 1 {
                break;
            }
            let style = if *is_bold { bold } else { Style::PLAIN };
            menu_put(scr, y + 2 + i as i32, x, &head(line, left), style);
            left = (left - width(line)).max(0);
        }
        let done = shown >= f64::from(total);
        let msg = if done {
            format!("Appuie sur une touche pour {then}")
        } else {
            "Une touche pour tout afficher".to_string()
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

/// How long ago `wall` was, `now` being the current wall-clock time.
pub fn ago(wall: f64, now: f64) -> String {
    let s = (now - wall).max(0.0);
    for (unit, n) in [("j", 86400.0), ("h", 3600.0), ("min", 60.0)] {
        if s >= n {
            return format!("il y a {} {unit}", (s / n).floor() as i64);
        }
    }
    "à l'instant".to_string()
}

/// Game selection at launch. Returns the save path, or None to quit.
pub fn save_menu(
    scr: &mut dyn Screen,
    saves: &Saves,
    slots: &[(u32, PathBuf, RawSave)],
    sound: Option<&dyn SoundSink>,
) -> Option<PathBuf> {
    let now = wall_time();
    let mut entries: Vec<(String, String, Option<PathBuf>)> = Vec::new();
    for (n, path, d) in slots {
        let name = if *n == 1 {
            "Partie principale".to_string()
        } else {
            format!("Partie {n}")
        };
        let [level, isotopes, prestiges, wall] =
            ["level", "isotopes", "prestige_count", "wall_time"].map(|k| number_or_zero(d, k));
        let mut detail = format!(
            "niveau {} · ⚛ {} · prestige ×{}",
            fmt(level),
            fmt(isotopes),
            prestiges as i64
        );
        if wall != 0.0 {
            detail += &format!(" · {}", ago(wall, now));
        }
        entries.push((format!("Continuer : {name}"), detail, Some(path.clone())));
    }
    let free = saves.free_slot();
    entries.push((
        "+ Nouvelle partie".to_string(),
        if free.is_some() {
            "Recommence de zéro dans une sauvegarde séparée.".to_string()
        } else {
            "Plus d'emplacement libre : supprime un fichier de sauvegarde.".to_string()
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
        let visible = fdiv(h - 6, 2).max(1);
        let y = (fdiv(h, 2) - count - 3)
            .min(h - 6 - 2 * count.min(visible))
            .max(0);
        top = top.min(sel).max(sel - visible + 1);
        menu_put(scr, y, fdiv(w - width(TITLE), 2).max(0), TITLE, bold);
        let q = "Quelle partie lancer ?";
        menu_put(scr, y + 2, fdiv(w - width(q), 2).max(0), q, Style::PLAIN);
        let x = (fdiv(w, 2) - 30).max(0);
        for (row, i) in (top..count.min(top + visible)).enumerate() {
            let (label, detail, _) = &entries[i as usize];
            let ly = y + 4 + 2 * row as i32;
            let (mark, style) = if i == sel {
                ('▶', bold.reverse())
            } else {
                (' ', bold)
            };
            menu_put(scr, ly, x, &format!(" {mark} {label} "), style);
            menu_put(scr, ly + 1, x + 4, detail, Style::PLAIN.dim());
        }
        if top > 0 {
            menu_put(scr, y + 3, x, "↑", Style::PLAIN);
        }
        if top + visible < count {
            menu_put(scr, h - 2, x, "↓", Style::PLAIN);
        }
        let help = "↑↓ choisir · Entrée valider · Q quitter";
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
            intro(&mut scr, Some(&sound), INTRO, 0.0, "commencer"),
            Ok(())
        );
        assert!(scr.keys.is_empty());
        assert!(written(&scr, "Morille"));
        assert!(written(&scr, "commencer"));
    }

    #[test]
    fn intro_small_screen() {
        let mut scr = BufferScreen::with_keys(8, 20, keys("ab"));
        assert_eq!(intro(&mut scr, None, INTRO, 0.0, "commencer"), Ok(()));
    }

    #[test]
    fn intro_interrupted() {
        let mut scr = BufferScreen::with_keys(30, 100, [Some(Key::Interrupt)]);
        assert_eq!(
            intro(&mut scr, None, INTRO, 60.0, "commencer"),
            Err(Interrupted)
        );
    }

    #[test]
    fn texts_fit_80x24() {
        // The default terminal: the last paragraph (intro or story) must stay visible.
        for pages in std::iter::once(INTRO).chain(STORIES.iter().map(|(_, p)| *p)) {
            let mut scr = BufferScreen::with_keys(24, 80, keys("ab"));
            intro(&mut scr, None, pages, 0.0, "commencer").unwrap();
            let last_par = nbsp(pages[pages.len() - 1]);
            let last = last_par.split_whitespace().last().unwrap();
            assert!(written(&scr, last), "{}", pages[0]);
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
        for ((cid, part), _) in STORIES {
            assert!(data::find_challenge(cid).is_some(), "{cid}");
            assert!(["start", "end"].contains(part));
        }
        assert!(story("nanar", "end").is_some());
    }
}
