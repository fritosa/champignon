//! French texts.

use super::Texts;

pub static FR: Texts = Texts {
    num,
    dur,
    suffixes: [
        "", "K", "M", "B", "T", "Qa", "Qi", "Sx", "Sp", "Oc", "No", "Dc",
    ],
    hidden: "???",
    infinity: "∞",
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
