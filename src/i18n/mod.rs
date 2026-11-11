//! Player texts: one catalog (`Texts`) per language, read through `tr()`.
//!
//! The catalog is a plain Rust struct: a language that forgets a text does not compile. Fixed texts are
//! `&'static str` fields; texts with values are `fn` fields (non-capturing closures), so each language keeps
//! its own word order and the compiler checks the values passed. Texts of the data tables (units, talents,
//! challenges…) are keyed by identifier; the tests check that every language covers every identifier.

pub mod data;
pub mod fr;

use data::DataTexts;

/// Every text shown to the player, in one language.
pub struct Texts {
    /// Number in a sentence (French: decimal comma, at most 3 decimals, thousands separated by spaces).
    pub num: fn(f64) -> String,
    /// Duration for humans (42 s, 2 min 05 s, 2 h 01 min).
    pub dur: fn(i64) -> String,
    /// Suffixes of the short numbers (game::fmt), one per power of 1000.
    pub suffixes: [&'static str; SUFFIX_COUNT],
    /// Shown instead of a hidden value: numbers during the Brouillard de guerre challenge, locked tab name.
    pub hidden: &'static str,
    /// Endless value or duration.
    pub infinity: &'static str,
    /// Texts of the data tables.
    pub data: DataTexts,
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
        }
    }

    /// Each template quotes every value of its entry, and renders (a bad placeholder or a missing value
    /// panics).
    #[test]
    fn every_template_quotes_its_values() {
        use crate::data::{self, Desc, TalentText};
        use crate::events;
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

    #[test]
    #[should_panic(expected = "no text for the unit \"nope\"")]
    fn find_panics_on_missing_text() {
        find::<&str>(&[("a", "A")], "nope", "unit");
    }
}
