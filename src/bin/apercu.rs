//! Generates apercu.svg: a color capture of the game screen for the README (cargo run --bin apercu).
//!
//! The real UI rendering runs on an in-memory screen (`BufferScreen`) that records each character and its
//! style; the grid is then turned into SVG text, rectangles and paths.

use std::fmt::Write as _;
use std::path::Path;

use champignon::data;
use champignon::events::{self, EnemyKind};
use champignon::game::{Enemy, Game, Plane, need};
use champignon::rng::StdRandom;
use champignon::sound::SoundSink;
use champignon::term::{BufferScreen, ColorName, Ink, Style};
use champignon::ui::{self, Float, UI};

const W: i32 = 100;
const H: i32 = 30;
const FONT_SIZE: i32 = 14;
const CELL_W: f64 = 8.4;
const CELL_H: i32 = 17;
const MARGIN: i32 = 10;
const BASELINE_OFFSET: i32 = 4; // from the cell bottom to the text baseline
const SETTLE_FRAMES: usize = 40;
const SETTLE_DT: f64 = 0.05; // seconds per settling frame
const CORNER_RADIUS: i32 = 8;
const STROKE_WIDTH: &str = "1.2"; // border lines, in px
const DIM_ATTR: &str = " opacity=\"0.6\""; // how the dim attribute looks
const BG: &str = "#1d1f21";
const FG: &str = "#c5c8c6";
/// Fixed seeds: the game's draws and the interface's, so that the capture never changes by itself.
const SCENE_SEED: u64 = 4;
const UI_SEED: u64 = 1;
/// Captured situation, beyond the plain field values of `scene`: share of the level done, share of the
/// commando's time elapsed, plane on screen for PLANE_AGE seconds (out of events::PLANE_DURATION).
const LEVEL_PROGRESS: f64 = 0.6;
const ENEMY_ELAPSED: f64 = 0.65;
const PLANE_AGE: f64 = 2.0;

/// Fake "available" sound, so the help bar shows the sound mode.
struct Sound;

impl SoundSink for Sound {
    fn ok(&self) -> bool {
        true
    }
    fn play(&self, _name: &str) {}
    fn set_music(&self, _track: &str) {}
    fn set_muted(&self, _muted: bool) {}
    fn set_music_on(&self, _on: bool) {}
    fn set_volume(&self, _volume: f32) {}
}

/// Palette of a 256-color terminal (orange is color 208, grey 244).
fn color(ink: Ink) -> &'static str {
    match ink {
        Ink::Black => "#1d1f21",
        Ink::Named(c) => match c {
            ColorName::Red => "#e5534b",
            ColorName::Green => "#8ec07c",
            ColorName::Yellow => "#f0c674",
            ColorName::Blue => "#81a2be",
            ColorName::Magenta => "#c397d8",
            ColorName::Cyan => "#70c0b1",
            ColorName::White => "#eaeaea",
            ColorName::Orange => "#f28c28",
            ColorName::Grey => "#808080",
        },
    }
}

/// Border strokes drawn as SVG lines (u: up, d: down, l: left, r: right): as glyphs, the font does not fill
/// the full line height and vertical borders turn into dotted lines.
fn box_arms(c: char) -> Option<&'static str> {
    Some(match c {
        '│' => "ud",
        '─' => "lr",
        '┌' => "dr",
        '┐' => "dl",
        '└' => "ur",
        '┘' => "ul",
        '├' => "udr",
        '┤' => "udl",
        '┬' => "lrd",
        '┴' => "lru",
        '┼' => "udlr",
        _ => return None,
    })
}

/// Same flaw for the mushroom's full blocks: drawn as rectangles (top, bottom as a fraction of a line).
fn block(c: char) -> Option<(f64, f64)> {
    Some(match c {
        '█' => (0.0, 1.0),
        '▀' => (0.0, 0.5),
        '▄' => (0.5, 1.0),
        '▁' => (0.875, 1.0),
        _ => return None,
    })
}

fn scene() -> Game {
    let mut g = Game::new(Box::new(StdRandom::seeded(SCENE_SEED)));
    g.click(true);
    g.exploded = true;
    g.challenge = Some("nanar");
    g.level = 72;
    g.isotopes = 41;
    g.pt = 4;
    g.radiation = 42e6;
    g.progress = LEVEL_PROGRESS * need(g.level);
    let counts = [
        ("soldier", 140),
        ("mortar", 95),
        ("artillery", 60),
        ("tank", 32),
        ("bomber", 12),
        ("sub", 8),
        ("icbm", 3),
        ("reactor", 1),
    ];
    for (uid, n) in counts {
        g.talents.insert(data::talent(&format!("u_{uid}")).id, 1);
        g.units[uid] = n;
    }
    g.talents.insert(data::talent("u_blackhole").id, 1);
    g.mutations = vec!["spores", "fallout"];
    g.combo = 1.4;
    g.crit_gauge = 0.62;
    g.time = 500.0;
    let dur = events::enemy_duration(&g, EnemyKind::Commando);
    g.enemies = vec![Enemy {
        kind: EnemyKind::Commando,
        start: g.time - dur * ENEMY_ELAPSED,
        dur,
        hits: 0,
    }];
    g.plane = Some(Plane {
        letter: 'R',
        start: g.time - PLANE_AGE,
        dur: events::PLANE_DURATION,
    });
    g.messages.clear();
    g.sfx.clear();
    g
}

/// Splits a line into (start, end, style) segments of the same style.
fn same_attr_runs(row: &[(char, Style)]) -> Vec<(usize, usize, Style)> {
    let mut runs = Vec::new();
    let mut x = 0;
    while x < row.len() {
        let style = row[x].1;
        let mut end = x;
        while end < row.len() && row[end].1 == style {
            end += 1;
        }
        runs.push((x, end, style));
        x = end;
    }
    runs
}

/// (text, background or None) colors of a style.
fn colors(style: Style) -> (&'static str, Option<&'static str>) {
    let fg = style.color.map_or(FG, |c| color(c.fg()));
    let bg = style.color.and_then(|c| c.bg()).map(color);
    if style.reverse {
        (bg.unwrap_or(BG), Some(fg))
    } else {
        (fg, bg)
    }
}

/// SVG path for border stroke c, i-th cell of the segment starting at (px, py).
fn box_path(arms: &str, px: f64, py: f64, i: usize) -> String {
    let cell_h = f64::from(CELL_H);
    let x0 = px + i as f64 * CELL_W;
    let (cx, cy) = (px + (i as f64 + 0.5) * CELL_W, py + cell_h / 2.0);
    let end = |k: char| match k {
        'u' => (cx, py),
        'd' => (cx, py + cell_h),
        'l' => (x0, cy),
        _ => (x0 + CELL_W, cy),
    };
    let mut arms = arms.chars();
    // The first two arms form a single polyline through the center: solid corner, no notch.
    let (ax, ay) = end(arms.next().expect("two arms"));
    let (bx, by) = end(arms.next().expect("two arms"));
    let mut d = format!("M{ax:.1} {ay:.1}L{cx:.1} {cy:.1}L{bx:.1} {by:.1}");
    for k in arms {
        let (ex, ey) = end(k);
        write!(d, "M{cx:.1} {cy:.1}L{ex:.1} {ey:.1}").expect("write to String");
    }
    d
}

/// Like Python's html.escape (quotes included).
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

fn svg(cells: &[Vec<(char, Style)>]) -> String {
    let mut out = vec![
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{}\" \
             font-family=\"DejaVu Sans Mono, Menlo, Consolas, monospace\" font-size=\"{FONT_SIZE}\">",
            f64::from(W) * CELL_W + f64::from(2 * MARGIN),
            H * CELL_H + 2 * MARGIN
        ),
        format!("<rect width=\"100%\" height=\"100%\" rx=\"{CORNER_RADIUS}\" fill=\"{BG}\"/>"),
    ];
    for (y, row) in cells.iter().enumerate() {
        for (x, end, style) in same_attr_runs(row) {
            let chars: Vec<char> = row[x..end].iter().map(|c| c.0).collect();
            let (fg, bg) = colors(style);
            let px = f64::from(MARGIN) + x as f64 * CELL_W;
            let py = MARGIN + y as i32 * CELL_H;
            let width = (end - x) as f64 * CELL_W;
            if let Some(bg) = bg {
                out.push(format!(
                    "<rect x=\"{px:.1}\" y=\"{py}\" width=\"{width:.1}\" height=\"{CELL_H}\" fill=\"{bg}\"/>"
                ));
            }
            let dim = if style.dim { DIM_ATTR } else { "" };
            let mut d = String::new();
            for (i, &c) in chars.iter().enumerate() {
                if let Some(arms) = box_arms(c) {
                    d += &box_path(arms, px, f64::from(py), i);
                } else if let Some((top, bottom)) = block(c) {
                    let x0 = px + i as f64 * CELL_W;
                    let cell_h = f64::from(CELL_H);
                    out.push(format!(
                        "<rect x=\"{x0:.1}\" y=\"{:.1}\" width=\"{CELL_W:.1}\" height=\"{:.1}\" fill=\"{fg}\"{dim}/>",
                        f64::from(py) + top * cell_h,
                        (bottom - top) * cell_h
                    ));
                }
            }
            if !d.is_empty() {
                // A single path per segment: lighter file, and no darker joins in dim mode.
                out.push(format!(
                    "<path d=\"{d}\" stroke=\"{fg}\" stroke-width=\"{STROKE_WIDTH}\" fill=\"none\"{dim}/>"
                ));
            }
            let text: String = chars
                .iter()
                .map(|&c| {
                    if box_arms(c).is_some() || block(c).is_some() {
                        ' '
                    } else {
                        c
                    }
                })
                .collect();
            if !text.trim().is_empty() {
                let bold = if style.bold {
                    " font-weight=\"bold\""
                } else {
                    ""
                };
                out.push(format!(
                    "<text x=\"{px:.1}\" y=\"{}\" fill=\"{fg}\"{bold}{dim} xml:space=\"preserve\" \
                     textLength=\"{width:.1}\" lengthAdjust=\"spacingAndGlyphs\">{}</text>",
                    py + CELL_H - BASELINE_OFFSET,
                    escape(&text)
                ));
            }
        }
    }
    out.push("</svg>".to_string());
    out.join("\n")
}

fn main() {
    let mut u = UI::new(BufferScreen::new(H, W), scene(), Some(Box::new(Sound)));
    u.rng = Box::new(StdRandom::seeded(UI_SEED));
    for _ in 0..SETTLE_FRAMES {
        u.draw(SETTLE_DT);
    }
    // Nanarland challenge: a press that fizzles, and a botched-shoot title set at the start of the banner.
    u.floats.push(Float {
        text: "pschitt…".to_string(),
        age: 0.0,
        dx: -3,
        style: u.c("grey").bold(),
    });
    u.news = events::NEWS_NANAR[1];
    u.news_off =
        f64::from(u.w) - " FLASH INFO ".chars().count() as f64 - f64::from(ui::NEWS_MARGIN);
    u.draw(0.0);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("apercu.svg");
    std::fs::write(path, svg(u.scr.rows())).expect("write apercu.svg");
    println!("apercu.svg écrit");
}
