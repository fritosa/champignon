//! Screen layer replacing curses: styles, keys, the `Screen` trait, the real terminal (`TermScreen`, crossterm)
//! and an in-memory grid (`BufferScreen`) for the tests and the SVG preview.
//!
//! Coordinates are (y, x) like curses, 1 column per `char` (like Python's `len()`). Clipping to the screen is
//! the caller's job (`UI::put`): `BufferScreen` refuses anything off screen, `TermScreen` ignores it.

use std::collections::VecDeque;
use std::io::{self, BufWriter, IsTerminal, Stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once, OnceLock};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{
    self as cstyle, Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, execute, queue, terminal};

pub use crate::game::MsgKind;

// --- Colors and styles ------------------------------------------------------------

/// Named foreground colors (the curses pairs on the default background).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorName {
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Magenta,
    White,
    Grey,
}

impl ColorName {
    pub const ALL: [ColorName; 9] = [
        ColorName::Red,
        ColorName::Orange,
        ColorName::Yellow,
        ColorName::Green,
        ColorName::Cyan,
        ColorName::Blue,
        ColorName::Magenta,
        ColorName::White,
        ColorName::Grey,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ColorName::Red => "red",
            ColorName::Orange => "orange",
            ColorName::Yellow => "yellow",
            ColorName::Green => "green",
            ColorName::Cyan => "cyan",
            ColorName::Blue => "blue",
            ColorName::Magenta => "magenta",
            ColorName::White => "white",
            ColorName::Grey => "grey",
        }
    }

    pub fn from_name(name: &str) -> Option<ColorName> {
        ColorName::ALL.into_iter().find(|c| c.name() == name)
    }
}

/// Banner colors of a game message kind.
impl MsgKind {
    /// Color of the kind (MSG_COLORS): text color of its big titles, background of its banners.
    pub fn color(self) -> ColorName {
        match self {
            MsgKind::Info => ColorName::White,
            MsgKind::Good => ColorName::Green,
            MsgKind::Bad => ColorName::Red,
            MsgKind::Loot => ColorName::Cyan,
            MsgKind::Star => ColorName::Yellow,
            MsgKind::Epic => ColorName::Magenta,
        }
    }
}

/// A terminal palette entry: black or one of the named colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ink {
    Black,
    Named(ColorName),
}

/// A color pair: a named color on the default background, or one of the special pairs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    Named(ColorName),
    /// Black on yellow.
    Flash,
    /// White on red, bold.
    Alert,
    /// Message banner `msg_<kind>`: black text (white on red and magenta) on the kind's color, bold.
    Msg(MsgKind),
}

impl Color {
    /// Names used by the interface: "red"…"grey", "flash", "alert", "msg_<kind>".
    pub fn from_name(name: &str) -> Option<Color> {
        match name {
            "flash" => Some(Color::Flash),
            "alert" => Some(Color::Alert),
            _ => match name.strip_prefix("msg_") {
                Some(kind) => MsgKind::from_name(kind).map(Color::Msg),
                None => ColorName::from_name(name).map(Color::Named),
            },
        }
    }

    pub fn fg(self) -> Ink {
        match self {
            Color::Named(c) => Ink::Named(c),
            Color::Flash => Ink::Black,
            Color::Alert => Ink::Named(ColorName::White),
            Color::Msg(kind) => match kind.color() {
                ColorName::Red | ColorName::Magenta => Ink::Named(ColorName::White),
                _ => Ink::Black,
            },
        }
    }

    /// None: the terminal's default background.
    pub fn bg(self) -> Option<Ink> {
        match self {
            Color::Named(_) => None,
            Color::Flash => Some(Ink::Named(ColorName::Yellow)),
            Color::Alert => Some(Ink::Named(ColorName::Red)),
            Color::Msg(kind) => Some(Ink::Named(kind.color())),
        }
    }

    /// The special pairs carry the bold attribute (like `curses.color_pair(n) | A_BOLD`).
    pub fn implies_bold(self) -> bool {
        matches!(self, Color::Alert | Color::Msg(_))
    }
}

impl From<ColorName> for Color {
    fn from(c: ColorName) -> Color {
        Color::Named(c)
    }
}

/// Display attributes of a cell (replaces the curses attributes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub color: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub reverse: bool,
}

impl Style {
    /// No color, no attribute (curses attribute 0).
    pub const PLAIN: Style = Style {
        color: None,
        bold: false,
        dim: false,
        reverse: false,
    };

    /// A color, bold included for the special pairs that carry it.
    pub fn of(color: impl Into<Color>) -> Style {
        let color = color.into();
        Style {
            color: Some(color),
            bold: color.implies_bold(),
            ..Style::PLAIN
        }
    }

    /// Like `Style::of`, by name (`Color::from_name`); an unknown name gives the plain style.
    pub fn named(name: &str) -> Style {
        Color::from_name(name).map_or(Style::PLAIN, Style::of)
    }

    pub fn bold(self) -> Style {
        Style { bold: true, ..self }
    }

    pub fn dim(self) -> Style {
        Style { dim: true, ..self }
    }

    pub fn reverse(self) -> Style {
        Style {
            reverse: true,
            ..self
        }
    }

    /// Flips the reverse attribute (`attr ^= A_REVERSE`).
    pub fn toggle_reverse(self) -> Style {
        Style {
            reverse: !self.reverse,
            ..self
        }
    }
}

impl From<ColorName> for Style {
    fn from(c: ColorName) -> Style {
        Style::of(c)
    }
}

impl From<Color> for Style {
    fn from(c: Color) -> Style {
        Style::of(c)
    }
}

// --- Keys and the Screen trait ----------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Enter,
    Resize,
    /// Ctrl+C (raw mode turns it into a key instead of SIGINT): quit with a save, like KeyboardInterrupt.
    Interrupt,
}

/// Translates a terminal event into a game key (None: ignored event).
pub fn key_from_event(ev: &Event) -> Option<Key> {
    match ev {
        Event::Resize(..) => Some(Key::Resize),
        Event::Key(k) => key_from_key_event(k),
        _ => None,
    }
}

fn key_from_key_event(k: &KeyEvent) -> Option<Key> {
    if k.kind == KeyEventKind::Release {
        return None;
    }
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    Some(match k.code {
        KeyCode::Char('c') | KeyCode::Char('C') if ctrl => Key::Interrupt,
        KeyCode::Char('j' | 'm') if ctrl => Key::Enter, // ^J and ^M: newline and carriage return
        KeyCode::Char('i') if ctrl => Key::Tab,
        KeyCode::Char(_) if ctrl => return None,
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        _ => return None,
    })
}

/// Number of columns taken by a string (1 per `char`).
pub fn width(s: &str) -> i32 {
    s.chars().count() as i32
}

pub trait Screen {
    /// (height, width) in cells.
    fn size(&self) -> (i32, i32);
    /// Writes `s` at (y, x); the caller keeps it on screen (see `UI::put`).
    fn put(&mut self, y: i32, x: i32, s: &str, style: Style);
    /// Blanks the next frame.
    fn erase(&mut self);
    /// Like `erase`, and the next refresh redraws everything.
    fn clear(&mut self);
    /// Shows the frame drawn since the last erase.
    fn refresh(&mut self);
    /// Next key; waits at most `timeout` (None: waits for a key). None: no key in time.
    fn read_key(&mut self, timeout: Option<Duration>) -> Option<Key>;
    /// Terminal title (no-op where the terminal does not support it).
    fn set_title(&mut self, title: &str);
}

type Grid = Vec<Vec<(char, Style)>>;

fn blank_grid(h: i32, w: i32) -> Grid {
    vec![vec![(' ', Style::PLAIN); w.max(0) as usize]; h.max(0) as usize]
}

// --- In-memory screen ------------------------------------------------------------

/// Grid of (char, Style) with scripted keys: the test screen and the preview's canvas.
/// Refuses (panics on) any write off screen, so tests catch drawing that overflows.
pub struct BufferScreen {
    h: i32,
    w: i32,
    cells: Grid,
    /// Strings written since the last erase, as (y, x, text).
    pub writes: Vec<(i32, i32, String)>,
    /// Scripted keys; None plays a timeout. Reading past the end panics (like `FakeScreen`).
    pub keys: VecDeque<Option<Key>>,
    pub title: Option<String>,
    pub refreshes: usize,
}

impl BufferScreen {
    pub fn new(h: i32, w: i32) -> BufferScreen {
        BufferScreen {
            h,
            w,
            cells: blank_grid(h, w),
            writes: Vec::new(),
            keys: VecDeque::new(),
            title: None,
            refreshes: 0,
        }
    }

    /// A screen whose `read_key` returns these keys in order (None: a timeout).
    pub fn with_keys(h: i32, w: i32, keys: impl IntoIterator<Item = Option<Key>>) -> BufferScreen {
        let mut scr = BufferScreen::new(h, w);
        scr.keys.extend(keys);
        scr
    }

    pub fn push_key(&mut self, key: Key) {
        self.keys.push_back(Some(key));
    }

    /// New size; the content is blanked.
    pub fn resize(&mut self, h: i32, w: i32) {
        self.h = h;
        self.w = w;
        self.erase();
    }

    pub fn cell(&self, y: i32, x: i32) -> (char, Style) {
        self.cells[y as usize][x as usize]
    }

    pub fn style_at(&self, y: i32, x: i32) -> Style {
        self.cell(y, x).1
    }

    /// Text of row y, trailing spaces included.
    pub fn row_text(&self, y: i32) -> String {
        self.cells[y as usize].iter().map(|c| c.0).collect()
    }

    /// All rows, separated by newlines.
    pub fn text(&self) -> String {
        (0..self.h)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn contains(&self, needle: &str) -> bool {
        (0..self.h).any(|y| self.row_text(y).contains(needle))
    }

    /// Cells as rows of (char, Style), top to bottom.
    pub fn rows(&self) -> &[Vec<(char, Style)>] {
        &self.cells
    }
}

impl Screen for BufferScreen {
    fn size(&self) -> (i32, i32) {
        (self.h, self.w)
    }

    fn put(&mut self, y: i32, x: i32, s: &str, style: Style) {
        assert!(
            0 <= y && y < self.h && 0 <= x && x + width(s) <= self.w,
            "write off screen: ({y}, {x}, {s:?}) on {}x{}",
            self.h,
            self.w
        );
        let row = &mut self.cells[y as usize];
        for (i, ch) in s.chars().enumerate() {
            row[x as usize + i] = (ch, style);
        }
        self.writes.push((y, x, s.to_string()));
    }

    fn erase(&mut self) {
        self.cells = blank_grid(self.h, self.w);
        self.writes.clear();
    }

    fn clear(&mut self) {
        self.erase();
    }

    fn refresh(&mut self) {
        self.refreshes += 1;
    }

    fn read_key(&mut self, _timeout: Option<Duration>) -> Option<Key> {
        self.keys.pop_front().expect("no more scripted keys")
    }

    fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_string());
    }
}

// --- Real terminal ---------------------------------------------------------------

/// xterm sequences: push the terminal title, then clear it and restore the old one.
const PUSH_TITLE: &str = "\x1b[22;0t";
const RESTORE_TITLE: &str = "\x1b]0;\x07\x1b[23;0t";

/// How often a blocking `read_key` looks at `interrupt_flag`.
const SIGNAL_CHECK: Duration = Duration::from_millis(100);

/// Set by the signals (main registers it): `TermScreen::read_key` then returns `Key::Interrupt`.
pub fn interrupt_flag() -> &'static Arc<AtomicBool> {
    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// Terminal set up (raw mode…) and not restored yet; shared with the panic hook.
static ACTIVE: AtomicBool = AtomicBool::new(false);
static TITLE_PUSHED: AtomicBool = AtomicBool::new(false);

/// Whether the terminal title can be set: a real terminal that understands the xterm sequences.
pub fn can_set_title() -> bool {
    let term = std::env::var("TERM").unwrap_or_else(|_| "dumb".to_string());
    !matches!(term.as_str(), "dumb" | "linux") && io::stdout().is_terminal()
}

/// 256 colors available (orange and grey get their own shades).
fn has_256_colors() -> bool {
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    matches!(colorterm.as_str(), "truecolor" | "24bit") || term.contains("256color")
}

/// Puts the terminal back as it was (idempotent): raw mode off, main screen, cursor, title.
pub fn restore_terminal() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let mut out = io::stdout();
    if TITLE_PUSHED.swap(false, Ordering::SeqCst) {
        let _ = write!(out, "{RESTORE_TITLE}");
    }
    let _ = execute!(
        out,
        SetAttribute(Attribute::Reset),
        cursor::Show,
        terminal::LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();
}

/// Restores the terminal before the panic message is printed (otherwise lost in the alternate screen).
pub fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous(info);
        }));
    });
}

/// The real terminal (crossterm), double buffered: `refresh` only writes the cells that changed.
pub struct TermScreen {
    out: BufWriter<Stdout>,
    h: i32,
    w: i32,
    /// Frame being drawn.
    back: Grid,
    /// What the terminal shows; None: unknown, everything is redrawn.
    front: Option<Grid>,
    many_colors: bool,
    titled: bool,
    title: String,
}

impl TermScreen {
    /// Raw mode, alternate screen, hidden cursor; the terminal is restored on drop.
    pub fn new() -> io::Result<TermScreen> {
        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let mut out = BufWriter::new(io::stdout());
        let titled = can_set_title();
        let setup = (|| {
            if titled {
                write!(out, "{PUSH_TITLE}")?;
                TITLE_PUSHED.store(true, Ordering::SeqCst);
            }
            execute!(
                out,
                terminal::EnterAlternateScreen,
                cursor::Hide,
                terminal::Clear(terminal::ClearType::All)
            )
        })();
        if let Err(e) = setup {
            restore_terminal();
            return Err(e);
        }
        let (w, h) = terminal::size().unwrap_or((80, 24));
        let (h, w) = (i32::from(h), i32::from(w));
        Ok(TermScreen {
            out,
            h,
            w,
            back: blank_grid(h, w),
            front: None,
            many_colors: has_256_colors(),
            titled,
            title: String::new(),
        })
    }

    /// Follows the terminal size; a new size blanks both buffers.
    fn sync_size(&mut self) {
        if let Ok((w, h)) = terminal::size() {
            let (h, w) = (i32::from(h), i32::from(w));
            if (h, w) != (self.h, self.w) {
                self.h = h;
                self.w = w;
                self.back = blank_grid(h, w);
                self.front = None;
            }
        }
    }

    fn ink(&self, ink: Ink) -> cstyle::Color {
        use cstyle::Color as C;
        match ink {
            Ink::Black => C::Black,
            Ink::Named(c) => match c {
                ColorName::Red => C::DarkRed,
                ColorName::Orange if self.many_colors => C::AnsiValue(208),
                ColorName::Orange | ColorName::Yellow => C::DarkYellow,
                ColorName::Green => C::DarkGreen,
                ColorName::Cyan => C::DarkCyan,
                ColorName::Blue => C::DarkBlue,
                ColorName::Magenta => C::DarkMagenta,
                ColorName::Grey if self.many_colors => C::AnsiValue(244),
                ColorName::White | ColorName::Grey => C::Grey, // ANSI white (7)
            },
        }
    }

    /// Resets the attributes then applies the style (default colors when it has none).
    fn apply(&mut self, style: Style) -> io::Result<()> {
        queue!(self.out, SetAttribute(Attribute::Reset))?;
        let grey_fallback = !self.many_colors && style.color == Some(Color::Named(ColorName::Grey));
        if style.bold {
            queue!(self.out, SetAttribute(Attribute::Bold))?;
        }
        if style.dim || grey_fallback {
            queue!(self.out, SetAttribute(Attribute::Dim))?;
        }
        if style.reverse {
            queue!(self.out, SetAttribute(Attribute::Reverse))?;
        }
        if let Some(color) = style.color {
            let fg = self.ink(color.fg());
            queue!(self.out, SetForegroundColor(fg))?;
            if let Some(bg) = color.bg() {
                let bg = self.ink(bg);
                queue!(self.out, SetBackgroundColor(bg))?;
            }
        }
        Ok(())
    }

    fn draw(&mut self) -> io::Result<()> {
        let mut current: Option<Style> = None;
        let mut cursor_at: Option<(usize, usize)> = None;
        for y in 0..self.back.len() {
            for x in 0..self.back[y].len() {
                let cell = self.back[y][x];
                if self.front.as_ref().is_some_and(|f| f[y][x] == cell) {
                    continue;
                }
                if cursor_at != Some((y, x)) {
                    queue!(self.out, cursor::MoveTo(x as u16, y as u16))?;
                }
                if current != Some(cell.1) {
                    self.apply(cell.1)?;
                    current = Some(cell.1);
                }
                queue!(self.out, Print(cell.0))?;
                cursor_at = Some((y, x + 1));
            }
        }
        queue!(self.out, SetAttribute(Attribute::Reset))?;
        self.out.flush()?;
        match &mut self.front {
            Some(front) => front.clone_from(&self.back), // reuses the rows' allocations
            None => self.front = Some(self.back.clone()),
        }
        Ok(())
    }
}

impl Screen for TermScreen {
    fn size(&self) -> (i32, i32) {
        (self.h, self.w)
    }

    fn put(&mut self, y: i32, x: i32, s: &str, style: Style) {
        if y < 0 || y >= self.h {
            return;
        }
        let row = &mut self.back[y as usize];
        for (i, ch) in s.chars().enumerate() {
            let col = x + i as i32;
            if col >= self.w {
                break;
            }
            if col >= 0 {
                row[col as usize] = (ch, style);
            }
        }
    }

    fn erase(&mut self) {
        self.sync_size();
        for row in &mut self.back {
            row.fill((' ', Style::PLAIN));
        }
    }

    fn clear(&mut self) {
        self.erase();
        self.front = None;
        let _ = queue!(
            self.out,
            SetAttribute(Attribute::Reset),
            terminal::Clear(terminal::ClearType::All)
        );
    }

    fn refresh(&mut self) {
        // A failed write (terminal gone) is not fatal: the game still saves on exit.
        if self.draw().is_err() {
            self.front = None;
        }
    }

    /// A signal (`interrupt_flag`) or a terminal that can no longer be read gives `Key::Interrupt`, so that
    /// every screen, even a blocking one, quits like with Ctrl+C.
    fn read_key(&mut self, timeout: Option<Duration>) -> Option<Key> {
        let deadline = timeout.map(|t| Instant::now() + t);
        loop {
            if interrupt_flag().load(Ordering::SeqCst) {
                return Some(Key::Interrupt);
            }
            let left = deadline.map_or(SIGNAL_CHECK, |d| {
                d.saturating_duration_since(Instant::now())
                    .min(SIGNAL_CHECK)
            });
            match event::poll(left) {
                Ok(true) => {}
                Ok(false) if deadline.is_some_and(|d| Instant::now() >= d) => return None,
                Ok(false) => continue,
                Err(_) => return Some(Key::Interrupt),
            }
            let Ok(ev) = event::read() else {
                return Some(Key::Interrupt);
            };
            if let Some(key) = key_from_event(&ev) {
                if key == Key::Resize {
                    self.sync_size();
                }
                return Some(key);
            }
        }
    }

    fn set_title(&mut self, title: &str) {
        if !self.titled || title == self.title {
            return;
        }
        self.title = title.to_string();
        let written = write!(self.out, "\x1b]0;{title}\x07").and_then(|_| self.out.flush());
        if written.is_err() {
            self.titled = false;
        }
    }
}

impl Drop for TermScreen {
    fn drop(&mut self) {
        let _ = self.out.flush();
        restore_terminal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventState, MouseEvent, MouseEventKind};

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn buffer_put_and_read_back() {
        let mut scr = BufferScreen::new(3, 10);
        scr.put(1, 2, "abc", Style::of(ColorName::Red).bold());
        assert_eq!(scr.row_text(1), "  abc     ");
        assert_eq!(scr.cell(1, 3), ('b', Style::of(ColorName::Red).bold()));
        assert_eq!(scr.style_at(1, 5), Style::PLAIN);
        assert!(scr.contains("abc"));
        assert_eq!(scr.writes, vec![(1, 2, "abc".to_string())]);
        assert_eq!(scr.text().lines().count(), 3);
        scr.erase();
        assert!(!scr.contains("abc"));
        assert!(scr.writes.is_empty());
    }

    #[test]
    fn buffer_counts_one_column_per_char() {
        let mut scr = BufferScreen::new(1, 4);
        scr.put(0, 0, "☢é─…", Style::PLAIN);
        assert_eq!(scr.row_text(0), "☢é─…");
    }

    #[test]
    fn buffer_accepts_up_to_the_edges() {
        let mut scr = BufferScreen::new(2, 5);
        scr.put(1, 0, "abcde", Style::PLAIN);
        scr.put(0, 4, "x", Style::PLAIN);
        scr.put(0, 5, "", Style::PLAIN);
        assert_eq!(scr.row_text(1), "abcde");
    }

    #[test]
    #[should_panic(expected = "off screen")]
    fn buffer_refuses_overflow() {
        BufferScreen::new(2, 5).put(0, 3, "abc", Style::PLAIN);
    }

    #[test]
    #[should_panic(expected = "off screen")]
    fn buffer_refuses_negative_x() {
        BufferScreen::new(2, 5).put(0, -1, "a", Style::PLAIN);
    }

    #[test]
    #[should_panic(expected = "off screen")]
    fn buffer_refuses_row_below() {
        BufferScreen::new(2, 5).put(2, 0, "a", Style::PLAIN);
    }

    #[test]
    fn buffer_scripted_keys_and_resize() {
        let mut scr =
            BufferScreen::with_keys(30, 100, [Some(Key::Char('a')), None, Some(Key::Enter)]);
        assert_eq!(scr.size(), (30, 100));
        assert_eq!(scr.read_key(None), Some(Key::Char('a')));
        assert_eq!(scr.read_key(Some(Duration::ZERO)), None);
        assert_eq!(scr.read_key(None), Some(Key::Enter));
        scr.push_key(Key::Resize);
        assert_eq!(scr.read_key(None), Some(Key::Resize));
        scr.resize(20, 60);
        assert_eq!(scr.size(), (20, 60));
        assert_eq!(scr.row_text(19).len(), 60);
        scr.set_title("☢ Niv. 1");
        assert_eq!(scr.title.as_deref(), Some("☢ Niv. 1"));
    }

    #[test]
    #[should_panic(expected = "no more scripted keys")]
    fn buffer_without_keys_left_panics() {
        BufferScreen::new(5, 5).read_key(None);
    }

    #[test]
    fn color_names_round_trip() {
        for c in ColorName::ALL {
            assert_eq!(ColorName::from_name(c.name()), Some(c));
            assert_eq!(Color::from_name(c.name()), Some(Color::Named(c)));
        }
        for k in MsgKind::ALL {
            assert_eq!(
                Color::from_name(&format!("msg_{}", k.name())),
                Some(Color::Msg(k))
            );
        }
        assert_eq!(Color::from_name("flash"), Some(Color::Flash));
        assert_eq!(Color::from_name("alert"), Some(Color::Alert));
        assert_eq!(Color::from_name("msg_nope"), None);
        assert_eq!(Style::named("nope"), Style::PLAIN);
    }

    #[test]
    fn special_pairs() {
        assert_eq!(Color::Flash.fg(), Ink::Black);
        assert_eq!(Color::Flash.bg(), Some(Ink::Named(ColorName::Yellow)));
        assert!(!Style::of(Color::Flash).bold);
        assert_eq!(Color::Alert.fg(), Ink::Named(ColorName::White));
        assert_eq!(Color::Alert.bg(), Some(Ink::Named(ColorName::Red)));
        assert!(Style::named("alert").bold);
        // Banners: white text on red and magenta, black elsewhere, always bold.
        assert_eq!(Color::Msg(MsgKind::Bad).fg(), Ink::Named(ColorName::White));
        assert_eq!(Color::Msg(MsgKind::Epic).fg(), Ink::Named(ColorName::White));
        assert_eq!(Color::Msg(MsgKind::Star).fg(), Ink::Black);
        assert_eq!(
            Color::Msg(MsgKind::Loot).bg(),
            Some(Ink::Named(ColorName::Cyan))
        );
        assert!(Style::of(Color::Msg(MsgKind::Info)).bold);
        assert_eq!(Color::Named(ColorName::Grey).bg(), None);
    }

    #[test]
    fn style_builders() {
        let s = Style::of(ColorName::Green).bold().dim();
        assert!(s.bold && s.dim && !s.reverse);
        assert!(s.toggle_reverse().reverse);
        assert!(!s.reverse().toggle_reverse().reverse);
        assert_eq!(
            Style::from(ColorName::Cyan),
            Style::of(Color::Named(ColorName::Cyan))
        );
        assert_eq!(Style::default(), Style::PLAIN);
    }

    #[test]
    fn keys_from_events() {
        let none = KeyModifiers::NONE;
        assert_eq!(
            key_from_event(&press(KeyCode::Char('x'), none)),
            Some(Key::Char('x'))
        );
        assert_eq!(
            key_from_event(&press(KeyCode::Char('Q'), KeyModifiers::SHIFT)),
            Some(Key::Char('Q'))
        );
        assert_eq!(
            key_from_event(&press(KeyCode::Enter, none)),
            Some(Key::Enter)
        );
        assert_eq!(
            key_from_event(&press(KeyCode::Char('m'), KeyModifiers::CONTROL)),
            Some(Key::Enter)
        );
        assert_eq!(key_from_event(&press(KeyCode::Tab, none)), Some(Key::Tab));
        assert_eq!(
            key_from_event(&press(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Key::BackTab)
        );
        assert_eq!(
            key_from_event(&press(KeyCode::PageDown, none)),
            Some(Key::PageDown)
        );
        assert_eq!(key_from_event(&press(KeyCode::Left, none)), Some(Key::Left));
        assert_eq!(
            key_from_event(&press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Key::Interrupt)
        );
        assert_eq!(
            key_from_event(&press(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(key_from_event(&press(KeyCode::Esc, none)), None);
        assert_eq!(key_from_event(&Event::Resize(80, 24)), Some(Key::Resize));
        let release = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: none,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        assert_eq!(key_from_event(&Event::Key(release)), None);
        let mouse = MouseEvent {
            kind: MouseEventKind::Moved,
            column: 0,
            row: 0,
            modifiers: none,
        };
        assert_eq!(key_from_event(&Event::Mouse(mouse)), None);
    }

    #[test]
    fn width_counts_chars() {
        assert_eq!(width("Gelé ☢"), 6);
        assert_eq!(width(""), 0);
    }
}
