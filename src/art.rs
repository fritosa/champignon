//! Procedural ASCII generation of the atomic mushroom.
//!
//! Each function returns a list of cells (y, x, character, color) relative to the panel; the UI turns the
//! color into a style. Widths count 1 column per `char`.

use crate::term::ColorName::{self, Blue, Cyan, Green, Grey, Magenta, Orange, Red, White, Yellow};

/// (y, x, character, color), relative to the panel.
pub type Cell = (i32, i32, char, ColorName);

pub const PALETTES: &[(&str, &[ColorName])] = &[
    (
        "classic",
        &[Yellow, Orange, Red, Orange, Yellow, White, Red],
    ),
    ("toxic", &[Green, Green, Yellow, Cyan, Green]),
    ("plasma", &[Magenta, Cyan, Blue, White, Magenta]),
    ("gold", &[Yellow, Orange, Yellow, White, Yellow]),
    ("ghost", &[White, Grey, White, Grey, Cyan]),
    ("neon", &[Magenta, Cyan, Magenta, Blue, Cyan]),
    ("ice", &[Cyan, White, Blue, Cyan, White]),
    ("lava", &[Red, Orange, Red, Yellow, Red]),
    ("camo", &[Green, Grey, Yellow, Green, Grey]),
    ("crystal", &[Cyan, Magenta, White, Blue, Cyan]),
    ("obsidian", &[Grey, Magenta, Grey, Blue, White]),
    ("carton", &[Orange, Yellow, Grey, Orange, White]),
];
pub const RAINBOW: &[ColorName] = &[Red, Orange, Yellow, Green, Cyan, Blue, Magenta];
const DENSE: &[char] = &['@', '#', '%', '&'];
const MID: &[char] = &['*', '%', '+', '='];
const LIGHT: &[char] = &['.', ':', '\'', '*'];

const BOMB: [&str; 3] = [r"  _/\_  ", r" |BOMB| ", r" |____| "];

/// Palette of a skin (None for `rainbow` and `mario`, drawn by special cases, and for unknown skins).
pub fn palette(skin: &str) -> Option<&'static [ColorName]> {
    PALETTES
        .iter()
        .find(|(name, _)| *name == skin)
        .map(|(_, p)| *p)
}

/// Whether the mushroom knows how to draw this skin.
pub fn has_skin(skin: &str) -> bool {
    palette(skin).is_some() || skin == "rainbow" || skin == "mario"
}

/// 16-bit pseudo-random value, stable for a given cell and frame.
fn noise(x: i32, y: i32, frame: i64) -> usize {
    let n = i64::from(x).wrapping_mul(73856093)
        ^ i64::from(y).wrapping_mul(19349663)
        ^ frame.wrapping_mul(83492791);
    (n & 0xFFFF) as usize
}

fn pick<T: Copy>(seq: &[T], n: usize) -> T {
    seq[n % seq.len()]
}

/// Relative size (0..1): grows with the level, maxed at the run's max level.
pub fn size_fraction(level: f64, cap: f64) -> f64 {
    if level <= 0.0 {
        return 0.0;
    }
    0.15 + 0.85 * (1.0 + level).ln() / (1.0 + cap).ln()
}

pub fn bomb(w: i32, h: i32) -> Vec<Cell> {
    let ground = h - 1;
    let top = ground - BOMB.len() as i32;
    let left = w.div_euclid(2) - (BOMB[0].chars().count() as i32).div_euclid(2);
    let mut cells = Vec::new();
    for (i, row) in BOMB.iter().enumerate() {
        let color = if i == 0 { Grey } else { White };
        for (j, ch) in row.chars().enumerate() {
            if ch != ' ' {
                cells.push((top + i as i32, left + j as i32, ch, color));
            }
        }
    }
    cells
}

/// Total height of a size-f mushroom in a panel of height h.
pub fn mushroom_height(h: i32, f: f64) -> i32 {
    // Python's round(): ties to even
    4.max((f64::from(h - 2) * f).round_ties_even() as i32)
}

/// (total height, cap width) of a size-f mushroom; twins share the width.
fn dimensions(w: i32, h: i32, f: f64, twin: bool) -> (i32, i32) {
    let height = mushroom_height(h, f);
    let mut cap_w = (w - 2).min(5.max((f64::from(height) * 2.4) as i32));
    if twin {
        cap_w = cap_w.min((w - 4).div_euclid(2));
    }
    (height, cap_w)
}

/// Shape of one mushroom, shared by its parts.
struct Shape<'a> {
    cx: i32,
    cap_h: i32,
    cap_w: i32,
    frame: i64,
    t: f64,
    skin: &'a str,
    mario: bool,
}

/// Mushroom of size f (0..1) in a w×h panel, ground on the last line.
pub fn mushroom(w: i32, h: i32, f: f64, t: f64, skin: &str, twin: bool) -> Vec<Cell> {
    let ground = h - 1;
    let frame = (t * 6.0) as i64;
    let (height, cap_w) = dimensions(w, h, f, twin);
    let cap_h = 2.max((f64::from(height) * 0.42) as i32);
    let stem_h = height - cap_h;
    let half_w = w.div_euclid(2);
    let centers = if twin {
        vec![
            half_w - cap_w.div_euclid(2) - 1,
            half_w + cap_w.div_euclid(2) + 1,
        ]
    } else {
        vec![half_w]
    };
    let mut cells = Vec::new();
    for cx in centers {
        let mario = skin == "mario";
        let shape = Shape {
            cx,
            cap_h,
            cap_w,
            frame: if mario { 0 } else { frame },
            t,
            skin,
            mario,
        };
        let top = ground - height;
        cap(&mut cells, &shape, top);
        stem(&mut cells, &shape, top + cap_h, stem_h, height);
        dust(&mut cells, &shape, ground, height);
    }
    cells
}

fn color(skin: &str, n: usize, row: i32, t: f64) -> ColorName {
    if skin == "rainbow" {
        let i = (i64::from(row) + (t * 4.0) as i64).rem_euclid(RAINBOW.len() as i64);
        return RAINBOW[i as usize];
    }
    pick(palette(skin).unwrap_or(PALETTES[0].1), n >> 3)
}

/// Cap: a dome, wider at the bottom, with a slightly tucked-in lower rim.
fn cap(cells: &mut Vec<Cell>, s: &Shape, top: i32) {
    for i in 0..s.cap_h {
        let rel = f64::from(i + 1) / f64::from(s.cap_h);
        let mut width = f64::from(s.cap_w) * (0.4 + 0.6 * (rel * std::f64::consts::PI / 2.0).sin());
        let is_rim = i == s.cap_h - 1 && s.cap_h > 2;
        if is_rim {
            width *= 0.9;
        }
        let half = 1.max((width / 2.0) as i32);
        let y = top + i;
        for x in s.cx - half..=s.cx + half {
            let n = noise(x, y, s.frame);
            let dist = f64::from((x - s.cx).abs()) / (f64::from(half) + 0.5);
            if s.mario {
                let spot = (x + 2 * y).rem_euclid(5) == 0 && dist < 0.85;
                cells.push((
                    y,
                    x,
                    if spot { 'O' } else { '#' },
                    if spot { White } else { Red },
                ));
                continue;
            }
            let chars = if dist < 0.5 {
                DENSE
            } else if dist < 0.85 {
                MID
            } else {
                LIGHT
            };
            cells.push((y, x, pick(chars, n), color(s.skin, n, i, s.t)));
        }
    }
}

/// Stem: widens toward the bottom, with a condensation ring on large mushrooms.
fn stem(cells: &mut Vec<Cell>, s: &Shape, top: i32, stem_h: i32, height: i32) {
    let stem_w = 1.max((f64::from(s.cap_w) * 0.16) as i32) | 1; // odd, to stay centered on cx
    let ring_row = (height >= 10).then(|| (f64::from(stem_h) * 0.35) as i32);
    let cx = s.cx;
    for j in 0..stem_h {
        let y = top + j;
        let half = (stem_w + (2.0 * f64::from(j) / f64::from(1.max(stem_h))) as i32).div_euclid(2);
        if Some(j) == ring_row {
            ring(
                cells,
                cx,
                y,
                (half + 2).max((f64::from(s.cap_w) * 0.3) as i32),
                s.frame,
            );
            continue;
        }
        for x in cx - half..=cx + half {
            let n = noise(x, y, s.frame);
            if s.mario {
                let edge = x == cx - half || x == cx + half;
                cells.push((y, x, if edge { '|' } else { ' ' }, White));
            } else if x == cx - half && half != 0 {
                cells.push((y, x, '(', Orange));
            } else if x == cx + half && half != 0 {
                cells.push((y, x, ')', Orange));
            } else {
                cells.push((
                    y,
                    x,
                    pick(&['!', '|', ':', '|'], n),
                    color(s.skin, n, s.cap_h + j, s.t),
                ));
            }
        }
    }
    if s.mario && stem_h >= 2 && stem_w >= 3 {
        let y = top + stem_h.div_euclid(2);
        for (k, ch) in "•‿•".chars().enumerate() {
            cells.push((y, cx - 1 + k as i32, ch, White));
        }
    }
}

fn ring(cells: &mut Vec<Cell>, cx: i32, y: i32, half: i32, frame: i64) {
    for x in cx - half..=cx + half {
        let phase = i64::from(x) + frame;
        let ch = if x == cx - half {
            '('
        } else if x == cx + half {
            ')'
        } else {
            ['~', '=', '-'][phase.rem_euclid(3) as usize]
        };
        cells.push((
            y,
            x,
            ch,
            if phase.rem_euclid(4) != 0 {
                White
            } else {
                Cyan
            },
        ));
    }
}

/// Dust cloud at the base, frayed at the edges.
fn dust(cells: &mut Vec<Cell>, s: &Shape, ground: i32, height: i32) {
    let mut rows = vec![(ground - 1, 1.3)];
    if height >= 7 {
        rows.push((ground - 2, 0.8));
    }
    let cx = s.cx;
    for (y, scale) in rows {
        let half = 2.max((f64::from(s.cap_w) * scale / 2.0) as i32);
        for x in cx - half..=cx + half {
            let n = noise(x, y, s.frame);
            let frayed = n.is_multiple_of(5) && f64::from((x - cx).abs()) > f64::from(half) * 0.7;
            if frayed {
                continue;
            }
            let color = if !n.is_multiple_of(3) { Grey } else { Orange };
            cells.push((y, x, pick(&['~', '^', '°', '*', '~'], n), color));
        }
    }
}

/// Approximate width taken on the ground (to place the units and the inspector).
pub fn mushroom_width(w: i32, h: i32, f: f64, twin: bool) -> i32 {
    let cap_w = dimensions(w, h, f, twin).1;
    if twin {
        return (w - 2).min(2 * cap_w + 2);
    }
    (w - 2).min((f64::from(cap_w) * 1.3) as i32)
}

// --- Units and enemies ------------------------------------------------------------
// Two frames per sprite (animation), lines bottom-aligned, facing right.

pub type Frames = [&'static [&'static str]; 2];

pub const UNIT_ART: &[(&str, Frames)] = &[
    ("soldier", [&[" o ", "/|=", "/ \\"], &[" o ", "/|=", " | "]]),
    ("mortar", [&["  /", " / ", "[_]"], &["  *", " / ", "[_]"]]),
    (
        "artillery",
        [&["    _", " __//", "(o)_)"], &["    °", " __//", "(o)_)"]],
    ),
    (
        "tank",
        [
            &["  __   ", " [__]==", "(oooo) "],
            &["  __   ", " [__]==", "(OoOo) "],
        ],
    ),
    ("bomber", [&["  _\\ ", "=[__>"], &["  _\\ ", "-[__>"]]),
    (
        "sub",
        [&["   _|_  ", "~<_____)"], &["   _|_  ", "<_____)~"]],
    ),
    (
        "icbm",
        [&[" /\\ ", " |!|", "_|_|_"], &[" /\\ ", " |!|", "_|_|_"]],
    ),
    (
        "reactor",
        [&[" .-. ", "(@@@)", "|___|"], &[" .-. ", "(@*@)", "|___|"]],
    ),
    (
        "blackhole",
        [&[" .-. ", "( @ )", " `-' "], &[" .-. ", "( O )", " `-' "]],
    ),
];

pub const ENEMY_ART: &[(&str, Frames)] = &[
    (
        "saboteur",
        [&["  o $", " /|/ ", " / \\ "], &["  o $", " /|/ ", "  |  "]],
    ),
    ("drone", [&["-=-=-", " [o] "], &["=-=-=", " [o] "]]),
    (
        "hippies",
        [
            &["[PAIX]  ", " \\o/ \\o/", "  |   | ", " / \\ / \\"],
            &["[PAIX]  ", " \\o/ \\o/", "  |   | ", "  |   | "],
        ],
    ),
    (
        "commando",
        [&[" .o. ", "/|==-", "/ \\  "], &[" .o. ", "/|==-", " |   "]],
    ),
];

fn find_art(table: &'static [(&str, Frames)], id: &str) -> Option<&'static Frames> {
    table.iter().find(|(name, _)| *name == id).map(|(_, f)| f)
}

pub fn unit_art(id: &str) -> Option<&'static Frames> {
    find_art(UNIT_ART, id)
}

pub fn enemy_art(kind: &str) -> Option<&'static Frames> {
    find_art(ENEMY_ART, kind)
}

fn mirrored(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '<' => '>',
        '>' => '<',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '/' => '\\',
        '\\' => '/',
        '`' => '\'',
        '\'' => '`',
        c => c,
    }
}

/// Current frame of a sprite (2 frames/s), padded to equal widths, flipped horizontally if mirror.
pub fn sprite(frames: &Frames, t: f64, mirror: bool) -> Vec<String> {
    let rows = frames[((t * 2.0) as i64).rem_euclid(frames.len() as i64) as usize];
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|r| {
            let padded = format!("{r:<w$}");
            if mirror {
                padded.chars().rev().map(mirrored).collect()
            } else {
                padded
            }
        })
        .collect()
}

pub fn sprite_width(frames: &Frames) -> i32 {
    frames
        .iter()
        .flat_map(|f| f.iter())
        .map(|r| r.chars().count() as i32)
        .max()
        .unwrap_or(0)
}

// --- Big-letter font (3 lines, half blocks) ------------------------------------------

pub const BIG_ROWS: usize = 3;
pub const BIG_FONT: &[(char, [&str; BIG_ROWS])] = &[
    ('A', ["▄▀▄", "█▀█", "▀ ▀"]),
    ('B', ["█▀▄", "█▀▄", "▀▀ "]),
    ('C', ["▄▀▀", "█  ", " ▀▀"]),
    ('D', ["█▀▄", "█ █", "▀▀ "]),
    ('E', ["█▀▀", "█▀ ", "▀▀▀"]),
    ('F', ["█▀▀", "█▀ ", "▀  "]),
    ('G', ["█▀▀", "█ ▄", "▀▀▀"]),
    ('H', ["█ █", "█▀█", "▀ ▀"]),
    ('I', ["▀█▀", " █ ", "▀▀▀"]),
    ('J', ["  █", "  █", "▀▀ "]),
    ('K', ["█ █", "█▀▄", "▀ ▀"]),
    ('L', ["█  ", "█  ", "▀▀▀"]),
    ('M', ["█▄ ▄█", "█ ▀ █", "▀   ▀"]),
    ('N', ["█▄ █", "█ ▀█", "▀  ▀"]),
    ('O', ["█▀█", "█ █", "▀▀▀"]),
    ('P', ["█▀█", "█▀▀", "▀  "]),
    ('Q', ["█▀█", "█ █", "▀▀▄"]),
    ('R', ["█▀█", "█▀▄", "▀ ▀"]),
    ('S', ["█▀▀", "▀▀█", "▀▀▀"]),
    ('T', ["▀█▀", " █ ", " ▀ "]),
    ('U', ["█ █", "█ █", "▀▀▀"]),
    ('V', ["█ █", "█ █", " ▀ "]),
    ('W', ["█ █ █", "█ █ █", " ▀ ▀ "]),
    ('X', ["█ █", "▄▀▄", "▀ ▀"]),
    ('Y', ["█ █", " █ ", " ▀ "]),
    ('Z', ["▀▀█", "▄▀ ", "▀▀▀"]),
    ('0', ["█▀█", "█ █", "▀▀▀"]),
    ('1', ["▄█ ", " █ ", "▀▀▀"]),
    ('2', ["▀▀█", "█▀▀", "▀▀▀"]),
    ('3', ["▀▀█", " ▀█", "▀▀▀"]),
    ('4', ["█ █", "▀▀█", "  ▀"]),
    ('5', ["█▀▀", "▀▀█", "▀▀▀"]),
    ('6', ["█▀▀", "█▀█", "▀▀▀"]),
    ('7', ["▀▀█", "  █", "  ▀"]),
    ('8', ["█▀█", "█▀█", "▀▀▀"]),
    ('9', ["█▀█", "▀▀█", "▀▀▀"]),
    ('!', ["█", "▀", "▀"]),
    ('?', ["▀▀█", " █▀", " ▀ "]),
    ('.', [" ", " ", "▀"]),
    (':', [" ", "▀", "▀"]),
    ('+', ["   ", "▄█▄", " ▀ "]),
    ('-', ["   ", "▀▀▀", "   "]),
    ('×', ["   ", "▀▄▀", "▀ ▀"]),
    ('\'', ["▀", " ", " "]),
    (' ', [" ", " ", " "]),
];

fn big_glyph(c: char) -> Option<&'static [&'static str; BIG_ROWS]> {
    BIG_FONT.iter().find(|(k, _)| *k == c).map(|(_, g)| g)
}

/// Base letter of an accented uppercase letter (what NFD followed by dropping the marks gives).
fn strip_accent(c: char) -> char {
    match c {
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' | 'Ă' | 'Ą' => 'A',
        'Ç' | 'Ć' | 'Ĉ' | 'Ċ' | 'Č' => 'C',
        'Ď' => 'D',
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => 'E',
        'Ĝ' | 'Ğ' | 'Ġ' | 'Ģ' => 'G',
        'Ĥ' => 'H',
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ĩ' | 'Ī' | 'Ĭ' | 'Į' | 'İ' => 'I',
        'Ĵ' => 'J',
        'Ķ' => 'K',
        'Ĺ' | 'Ļ' | 'Ľ' => 'L',
        'Ñ' | 'Ń' | 'Ņ' | 'Ň' => 'N',
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ō' | 'Ŏ' | 'Ő' => 'O',
        'Ŕ' | 'Ŗ' | 'Ř' => 'R',
        'Ś' | 'Ŝ' | 'Ş' | 'Š' => 'S',
        'Ţ' | 'Ť' => 'T',
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ũ' | 'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => 'U',
        'Ŵ' => 'W',
        'Ý' | 'Ÿ' | 'Ŷ' => 'Y',
        'Ź' | 'Ż' | 'Ž' => 'Z',
        c => c,
    }
}

/// Uppercase without accents (É → E, Œ → OE) for the big font.
pub fn plain(text: &str) -> String {
    text.to_uppercase()
        .replace('Œ', "OE")
        .replace('Æ', "AE")
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c)) // combining marks
        .map(strip_accent)
        .collect()
}

/// Three lines of big letters; unknown characters are skipped.
pub fn big_text(text: &str) -> [String; BIG_ROWS] {
    let glyphs: Vec<_> = plain(text).chars().filter_map(big_glyph).collect();
    std::array::from_fn(|row| {
        let line: String = glyphs.iter().map(|g| format!("{} ", g[row])).collect();
        line.trim_end().to_string()
    })
}

pub fn big_width(text: &str) -> i32 {
    big_text(text)
        .iter()
        .map(|r| r.chars().count() as i32)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNITS: [&str; 9] = [
        "soldier",
        "mortar",
        "artillery",
        "tank",
        "bomber",
        "sub",
        "icbm",
        "reactor",
        "blackhole",
    ];
    const ENEMIES: [&str; 4] = ["saboteur", "drone", "hippies", "commando"];

    #[test]
    fn big_text_folds_accents_and_skips_unknown() {
        let rows = big_text("Gelé !");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows, big_text(&"GELE!".replace('!', " !")));
        assert_eq!(big_text("☢"), ["", "", ""]);
        assert_eq!(big_text("cœur"), big_text("COEUR"));
        assert_eq!(plain("Éclair à l'œil"), "ECLAIR A L'OEIL");
        assert_eq!(plain("☢ e\u{301}"), "☢ E");
    }

    #[test]
    fn big_font_glyphs_have_equal_rows() {
        for (ch, glyph) in BIG_FONT {
            let w = glyph[0].chars().count();
            assert!(glyph.iter().all(|r| r.chars().count() == w), "{ch}");
        }
    }

    #[test]
    fn big_width_of_known_text() {
        // A (3) + space + B (3), the trailing space trimmed
        assert_eq!(big_width("ab"), 7);
        assert_eq!(big_width(""), 0);
        assert_eq!(big_width("M"), 5);
    }

    #[test]
    fn every_unit_and_enemy_has_art() {
        for id in UNITS {
            assert!(unit_art(id).is_some(), "{id}");
        }
        for kind in ENEMIES {
            assert!(enemy_art(kind).is_some(), "{kind}");
        }
        assert_eq!(UNIT_ART.len(), UNITS.len());
        assert_eq!(ENEMY_ART.len(), ENEMIES.len());
    }

    #[test]
    fn sprite_frames_have_the_same_height() {
        for (id, frames) in UNIT_ART.iter().chain(ENEMY_ART) {
            assert_eq!(frames[0].len(), frames[1].len(), "{id}");
        }
    }

    #[test]
    fn sprites_are_aligned_and_mirrored() {
        for (id, frames) in UNIT_ART.iter().chain(ENEMY_ART) {
            for t in [0.0, 0.6] {
                for mirror in [false, true] {
                    let rows = sprite(frames, t, mirror);
                    let w = rows[0].chars().count();
                    assert!(rows.iter().all(|r| r.chars().count() == w), "{id}");
                    assert!(w as i32 <= sprite_width(frames), "{id}");
                }
            }
        }
        let tank = unit_art("tank").unwrap();
        assert_eq!(sprite(tank, 0.0, false)[1], " [__]==");
        assert_eq!(sprite(tank, 0.0, true)[1], "==[__] ");
        assert_eq!(sprite(tank, 0.6, false)[2], "(OoOo) ");
        assert_eq!(sprite(unit_art("soldier").unwrap(), 0.0, true)[2], "/ \\");
        assert_eq!(
            sprite(unit_art("blackhole").unwrap(), 0.0, true)[2],
            " `-' "
        );
        assert_eq!(sprite_width(enemy_art("hippies").unwrap()), 8);
    }

    #[test]
    fn known_skins() {
        for skin in [
            "classic", "toxic", "rainbow", "mario", "plasma", "gold", "ghost", "neon", "ice",
            "lava", "camo", "crystal", "obsidian", "carton",
        ] {
            assert!(has_skin(skin), "{skin}");
        }
        assert!(!has_skin("nope"));
        assert!(palette("rainbow").is_none());
    }

    #[test]
    fn mushroom_stands_on_the_ground_and_grows() {
        for skin in ["classic", "rainbow", "mario", "unknown"] {
            for twin in [false, true] {
                for f in [0.15, 0.5, 1.0] {
                    let (w, h) = (60, 20);
                    let cells = mushroom(w, h, f, 3.7, skin, twin);
                    assert!(!cells.is_empty());
                    // The dust may overflow the sides (the UI clips), never the ground.
                    assert!(cells.iter().all(|&(y, _, _, _)| y < h));
                    let top = cells.iter().map(|c| c.0).min().unwrap();
                    assert_eq!(top, h - 1 - mushroom_height(h, f), "{skin} {twin} {f}");
                }
            }
        }
        assert!(mushroom_width(80, 24, 1.0, false) > mushroom_width(80, 24, 0.2, false));
        assert!(size_fraction(10.0, 100.0) < size_fraction(50.0, 100.0));
        assert_eq!(size_fraction(0.0, 100.0), 0.0);
        assert!((size_fraction(100.0, 100.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mushroom_skins_and_mario_face() {
        let rainbow = mushroom(60, 24, 1.0, 0.0, "rainbow", false);
        let colors: std::collections::HashSet<_> = rainbow.iter().map(|c| c.3).collect();
        assert!(RAINBOW.iter().all(|c| colors.contains(c)));
        let mario = mushroom(60, 24, 1.0, 0.0, "mario", false);
        let face: String = mario
            .iter()
            .filter(|c| "•‿".contains(c.2))
            .map(|c| c.2)
            .collect();
        assert_eq!(face, "•‿•");
        // Mario does not flicker: frame 0 whatever t.
        assert_eq!(mario, mushroom(60, 24, 1.0, 5.0, "mario", false));
    }

    #[test]
    fn python_rounding_and_noise() {
        assert_eq!(mushroom_height(7, 0.5), 4); // round(2.5) == 2 → floor of 4
        assert_eq!(mushroom_height(13, 0.5), 6); // round(5.5) == 6
        assert_eq!(mushroom_height(19, 0.5), 8); // round(8.5) == 8
        assert_eq!(
            noise(-3, 2, 1),
            ((-3i64 * 73856093) ^ (2 * 19349663) ^ 83492791) as usize & 0xFFFF
        );
    }

    #[test]
    fn bomb_is_centered_on_the_ground() {
        let cells = bomb(20, 10);
        assert_eq!(cells.iter().map(|c| c.0).max(), Some(8));
        assert!(cells.iter().any(|c| c.2 == 'B'));
        assert_eq!(cells.iter().map(|c| c.1).min(), Some(10 - 4 + 1));
    }
}
