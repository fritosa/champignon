//! Player texts: one catalog (`Texts`) per language, read through `tr()`.
//!
//! The catalog is a plain Rust struct: a language that forgets a text does not compile. Fixed texts are
//! `&'static str` fields; texts with values are `fn` fields (non-capturing closures), so each language keeps
//! its own word order and the compiler checks the values passed. Texts of the data tables (units, talents,
//! challenges…) are keyed by identifier; the tests check that every language covers every identifier.

pub mod app;
pub mod data;
pub mod fr;
pub mod msg;
pub mod screens;
pub mod ui;

use app::AppTexts;
use data::DataTexts;
use msg::MsgTexts;
use screens::ScreenTexts;
use ui::UiTexts;

/// Every text shown to the player, in one language.
pub struct Texts {
    /// Number in a sentence (French: decimal comma, at most 3 decimals, thousands separated by spaces).
    pub num: fn(f64) -> String,
    /// Percentage (15 %), the number already formatted.
    pub percent: fn(&str) -> String,
    /// Duration for humans (42 s, 2 min 05 s, 2 h 01 min).
    pub dur: fn(i64) -> String,
    /// Decimal separator of the short numbers (game::fmt) and of `fixed`.
    pub decimal: char,
    /// Suffixes of the short numbers (game::fmt), one per power of 1000.
    pub suffixes: [&'static str; SUFFIX_COUNT],
    /// Shown instead of a hidden value: numbers during the Brouillard de guerre challenge, locked tab name.
    pub hidden: &'static str,
    /// Endless value or duration.
    pub infinity: &'static str,
    /// Between the items of a list in a sentence ("Mitose, Lueur verte").
    pub list_join: &'static str,
    /// Typography of a paragraph before it is wrapped (French: non-breaking spaces before : ; ! ? and
    /// inside « », so that no mark is left alone at a line edge).
    pub typography: fn(&str) -> String,
    /// Texts of the data tables.
    pub data: DataTexts,
    /// Messages of the game.
    pub msg: MsgTexts,
    /// Texts of the program outside the game screens.
    pub app: AppTexts,
    /// Texts of the game screen.
    pub ui: UiTexts,
    /// Texts of the full screens outside a game: introduction, stories, start menu.
    pub screens: ScreenTexts,
}

/// Powers of 1000 with a suffix in game::fmt (beyond: scientific notation).
pub const SUFFIX_COUNT: usize = 12;

/// Every language, with its code: the tests check them all.
pub static LANGS: &[(&str, &Texts)] = &[("fr", &fr::FR)];

/// Texts of the current language.
pub fn tr() -> &'static Texts {
    #[cfg(test)]
    if let Some(texts) = TEST_LANG.get() {
        return texts;
    }
    &fr::FR
}

/// `x` with `decimals` decimals, with the decimal separator of the current language (1,5).
pub fn fixed(x: f64, decimals: usize) -> String {
    localize_decimal(format!("{x:.decimals$}"))
}

/// Number written by Rust (decimal point) with the decimal separator of the current language.
pub fn localize_decimal(s: String) -> String {
    match tr().decimal {
        '.' => s,
        sep => s.replace('.', sep.encode_utf8(&mut [0; 4])),
    }
}

/// Number in a sentence, in the current language.
pub fn num(x: f64) -> String {
    (tr().num)(x)
}

#[cfg(test)]
thread_local! {
    static TEST_LANG: std::cell::Cell<Option<&'static Texts>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with `texts` as the current language (on this test thread only).
#[cfg(test)]
pub fn with_lang<R>(texts: &'static Texts, f: impl FnOnce() -> R) -> R {
    let before = TEST_LANG.replace(Some(texts));
    let result = f();
    TEST_LANG.set(before);
    result
}

/// Text of `id` in a table of a language (panics on a missing text: the tests check every table).
pub fn find<'a, T>(table: &'a [(&str, T)], id: &str, what: &str) -> &'a T {
    table
        .iter()
        .find(|(k, _)| *k == id)
        .map(|(_, t)| t)
        .unwrap_or_else(|| panic!("no text for the {what} {id:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_uses_the_decimal_separator() {
        assert_eq!(fixed(1.5, 1), "1,5");
        assert_eq!(fixed(3.0, 0), "3");
    }

    #[test]
    fn test_lang_is_per_thread_and_restored() {
        let (_, fr) = LANGS[0];
        with_lang(fr, || assert!(std::ptr::eq(tr(), fr)));
        assert!(std::ptr::eq(tr(), &fr::FR));
    }

    /// The ids of a text table are exactly `ids`, without duplicates.
    fn same_ids<'a, T>(
        lang: &str,
        what: &str,
        table: &[(&str, T)],
        ids: impl IntoIterator<Item = &'a str>,
    ) {
        let mut got: Vec<&str> = table.iter().map(|(k, _)| *k).collect();
        let mut want: Vec<&str> = ids.into_iter().collect();
        got.sort_unstable();
        want.sort_unstable();
        assert_eq!(got, want, "{lang}: texts of the {what}");
    }

    #[test]
    fn every_table_entry_has_its_text() {
        use crate::data::{self, TalentText};
        use crate::events;
        for (lang, texts) in LANGS {
            let d = &texts.data;
            same_ids(lang, "units", d.units, data::UNITS.iter().map(|u| u.id));
            let talents = data::TALENTS
                .iter()
                .filter(|t| matches!(t.text, TalentText::Table(_)))
                .map(|t| t.id);
            same_ids(lang, "talents", d.talents, talents);
            same_ids(lang, "branches", d.branches, data::BRANCHES);
            let ptalents = data::PTALENTS.iter().map(|p| p.id);
            same_ids(lang, "prestige talents", d.ptalents, ptalents);
            let stalents = data::STALENTS.iter().map(|p| p.id);
            same_ids(lang, "supernova talents", d.stalents, stalents);
            let challenges = data::CHALLENGES.iter().map(|c| c.id);
            same_ids(lang, "challenges", d.challenges, challenges);
            same_ids(lang, "skins", d.skins, data::SKINS);
            let mutations = events::MUTATIONS.iter().map(|m| m.id);
            same_ids(lang, "mutations", d.mutations, mutations);
            let synergies = events::SYNERGIES.iter().map(|s| s.id);
            same_ids(lang, "synergies", d.synergies, synergies);
            let achievements = events::ACHIEVEMENTS.iter().map(|a| a.id);
            same_ids(lang, "achievements", d.achievements, achievements);
            let enemies = events::ENEMIES.iter().map(|e| e.id);
            same_ids(lang, "enemies", d.enemies, enemies);
            same_ids(lang, "chest bonuses", d.chests, events::CHEST_REWARDS);
        }
    }

    /// Each template quotes every value of its entry, and renders (a bad placeholder or a missing value
    /// panics).
    #[test]
    fn every_template_quotes_its_values() {
        use crate::data::{self, Desc, TalentText};
        use crate::events;
        let exploded = crate::game::tests::exploded_game();
        for (lang, texts) in LANGS {
            let d = &texts.data;
            let mut descs: Vec<(Desc, &str)> = Vec::new();
            for t in data::TALENTS.iter() {
                if let TalentText::Table(desc) = t.text {
                    descs.push((desc, d.talent(t.id).desc));
                }
            }
            for p in data::PTALENTS.iter().chain(&data::STALENTS) {
                descs.push((p.desc, d.perk(p.id).desc));
            }
            for c in &data::CHALLENGES {
                descs.push((c.desc, d.challenge(c.id).desc));
                descs.push((c.reward, d.challenge(c.id).reward));
            }
            for m in &events::MUTATIONS {
                descs.push((m.effect, d.mutation(m.id).desc));
            }
            for s in &events::SYNERGIES {
                descs.push((s.effect, d.synergy(s.id).desc));
            }
            for kind in events::EnemyKind::ALL {
                descs.push((
                    events::effect_strength(&exploded, kind),
                    d.enemy(kind.id()).desc,
                ));
            }
            for (desc, template) in descs {
                assert!(
                    template.matches('{').count() >= desc.len,
                    "{lang}: {template}"
                );
                let _ = desc.render(template);
            }
            with_lang(texts, || {
                for t in data::TALENTS.iter() {
                    assert!(
                        !t.name().is_empty() && !t.desc().is_empty(),
                        "{lang}: {}",
                        t.id
                    );
                }
            });
        }
    }

    /// Big titles of the messages: every letter (accents folded) exists in the big font.
    #[test]
    fn titles_use_the_big_font() {
        use crate::art::{BIG_FONT, plain};
        for (lang, texts) in LANGS {
            let titles = &texts.msg.titles;
            for title in titles
                .all()
                .into_iter()
                .map(String::from)
                .chain([(titles.wave)(12)])
            {
                for c in plain(&title).chars() {
                    assert!(
                        BIG_FONT.iter().any(|(k, _)| *k == c),
                        "{lang}: {c:?} in {title:?}"
                    );
                }
            }
        }
    }

    /// Literals that look like text but are not shown in the game.
    const NOT_PLAYER_TEXT: [&str; 2] = [
        " (os error",          // cut off the end of a system error message (saves::delete)
        "Champignon atomique", // name of the audio stream in the system mixer (sound.rs), a proper noun
    ];

    /// String literals of the code before its test module, with the code just before each one; comments are
    /// skipped.
    fn literals(src: &str) -> Vec<(String, String)> {
        let code = src
            .find("#[cfg(test)]\nmod tests")
            .or_else(|| src.find("#[cfg(test)]\npub(crate) mod tests"))
            .map_or(src, |end| &src[..end]);
        let chars: Vec<char> = code.chars().collect();
        let mut found = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '/' if chars.get(i + 1) == Some(&'/') => {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                }
                // Character literal ('"', '\'', 'x'), not a lifetime.
                '\'' if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') => {
                    i += 3
                }
                '\'' if chars.get(i + 2) == Some(&'\'') => i += 2,
                '"' => {
                    // Raw string (r"…", r#"…"#): no escapes, closed by a quote and as many #.
                    let hashes = chars[..i].iter().rev().take_while(|&&c| c == '#').count();
                    let raw = i > hashes && chars[i - hashes - 1] == 'r';
                    let closing = |at: usize| {
                        chars[at] == '"'
                            && (!raw || (1..=hashes).all(|k| chars.get(at + k) == Some(&'#')))
                    };
                    let before: String = chars[i.saturating_sub(200)..i].iter().collect();
                    let mut text = String::new();
                    i += 1;
                    while i < chars.len() && !closing(i) {
                        if chars[i] == '\\' && !raw {
                            i += 1;
                        }
                        if let Some(&c) = chars.get(i) {
                            text.push(c);
                        }
                        i += 1;
                    }
                    found.push((before, text));
                }
                _ => {}
            }
            i += 1;
        }
        found
    }

    /// A word with an accent, or two words separated by a space: French (or English) prose. A single word
    /// without accent is not caught (it cannot be told from an id). Format placeholders (`{name:>8}`)
    /// are not words.
    fn looks_like_prose(text: &str) -> bool {
        let mut plain = String::new();
        let mut depth = 0u32;
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                // {{ and }} are literal braces.
                '{' | '}' if depth == 0 && chars.peek() == Some(&c) => {
                    chars.next();
                    plain.push(c);
                }
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => plain.push(c),
                _ => {}
            }
        }
        let accented = |c: char| c.is_alphabetic() && !c.is_ascii();
        // Identifiers (snake_case) are not words.
        let words = plain
            .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '_')
            .filter(|w| !w.contains('_') && w.chars().filter(|c| c.is_alphabetic()).count() >= 2)
            .count();
        plain.chars().any(accented) || words >= 2 && plain.contains(' ')
    }

    /// Player texts live in the catalog: no prose literal elsewhere, except developer messages (expect,
    /// panic) and NOT_PLAYER_TEXT.
    #[test]
    fn no_player_text_outside_i18n() {
        // Outside the catalog and the binaries `sim` and `apercu` (developer tools).
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        crate::sound::tests::rust_sources(&src, &mut files);
        files.retain(|f| !f.starts_with(src.join("i18n")) && !f.starts_with(src.join("bin")));
        assert!(files.len() > 10, "sources not found in {}", src.display());
        let mut stray = Vec::new();
        for file in files {
            let source = std::fs::read_to_string(&file).expect("readable source");
            let file = file
                .strip_prefix(&src)
                .unwrap_or(&file)
                .display()
                .to_string();
            for (before, text) in literals(&source) {
                // The statement the literal belongs to: developer message if it is in one of these calls.
                let statement = before.rsplit([';', '{', '}']).next().unwrap_or_default();
                let developer = [
                    "expect(",
                    "panic!(",
                    "unreachable!(",
                    "assert!(",
                    "assert_eq!(",
                    "assert_ne!(",
                ]
                .iter()
                .any(|call| statement.contains(call));
                if looks_like_prose(&text)
                    && !developer
                    && !NOT_PLAYER_TEXT.contains(&text.as_str())
                {
                    stray.push(format!("{file}: {text:?}"));
                }
            }
        }
        assert!(
            stray.is_empty(),
            "texts to move to src/i18n/:\n{}",
            stray.join("\n")
        );
    }

    #[test]
    fn prose_detection() {
        assert!(looks_like_prose("Défi"));
        assert!(looks_like_prose("Fusion partielle : +{} ☢"));
        assert!(!looks_like_prose(" ×{}"));
        assert!(!looks_like_prose("u_{}"));
        assert!(!looks_like_prose("msg_bad"));
        assert!(!looks_like_prose("{:<nw$} {r}/{:<2}"));
        assert!(looks_like_prose("{n} unités"));
        assert!(!looks_like_prose("libpulse-simple.so.0"));
        assert!(!looks_like_prose("{{}} {x}"));
        assert!(looks_like_prose("{{Défi}}"));
        assert!(looks_like_prose("} deux mots"));
        let lits = literals("let a = '\"'; // \"comment\"\nf(\"un \\\"deux\\\"\", r#\"x\"#);");
        let texts: Vec<&str> = lits.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts, ["un \"deux\"", "x"]);
        let lits = literals(r##"['\'', '"', '\''];f(r"a\", "b", r#"c"d"#);"##);
        let texts: Vec<&str> = lits.iter().map(|(_, t)| t.as_str()).collect();
        assert_eq!(texts, ["a\\", "b", "c\"d"]);
    }

    #[test]
    #[should_panic(expected = "no text for the unit \"nope\"")]
    fn find_panics_on_missing_text() {
        find::<&str>(&[("a", "A")], "nope", "unit");
    }
}
