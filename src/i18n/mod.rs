//! Player texts: one catalog (`Texts`) per language, read through `tr()`.
//!
//! The catalog is a plain Rust struct: a language that forgets a text does not compile. Fixed texts are
//! `&'static str` fields; texts with values are `fn` fields (non-capturing closures), so each language keeps
//! its own word order and the compiler checks the values passed. Texts of the data tables (units, talents,
//! challenges…) are keyed by identifier; the tests check that every language covers every identifier.

pub mod fr;

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

    #[test]
    #[should_panic(expected = "no text for the unit \"nope\"")]
    fn find_panics_on_missing_text() {
        find::<&str>(&[("a", "A")], "nope", "unit");
    }
}
