//! French texts.

mod app;
mod data;
mod msg;
mod screens;
mod stories;
mod ui;

use super::Texts;

pub static FR: Texts = Texts {
    num,
    dur,
    suffixes: [
        "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc",
    ],
    hidden: "???",
    infinity: "∞",
    list_join: ", ",
    typography,
    data: data::DATA,
    msg: msg::MSG,
    app: app::APP,
    ui: ui::UI,
    screens: screens::SCREENS,
};

/// Decimal comma, at most 3 decimals without trailing zeros, thousands separated by spaces
/// (1,5 · 0,1 · 1 000 000).
fn num(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let (sign, digits) = int.split_at(usize::from(int.starts_with('-')));
    let mut grouped = String::from(sign);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(c);
    }
    if frac.is_empty() {
        grouped
    } else {
        format!("{grouped},{frac}")
    }
}

fn dur(sec: i64) -> String {
    if sec >= 3600 {
        format!("{} h {:02} min", sec / 3600, sec % 3600 / 60)
    } else if sec >= 60 {
        format!("{} min {:02} s", sec / 60, sec % 60)
    } else {
        format!("{sec} s")
    }
}

/// Non-breaking spaces: no « or » : ; ! ? left orphaned at a line edge.
fn typography(text: &str) -> String {
    let text = text.replace("« ", "«\u{a0}");
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        let before_mark = chars.get(i + 1).is_some_and(|n| "»:;!?".contains(*n));
        out.push(if c == ' ' && before_mark { '\u{a0}' } else { c });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_numbers() {
        assert_eq!(num(100.0), "100");
        assert_eq!(num(0.1), "0,1");
        assert_eq!(num(1_000_000.0), "1 000 000");
        assert_eq!(num(-1234.25), "-1 234,25");
    }
}
