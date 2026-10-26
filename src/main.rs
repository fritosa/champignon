//! Champignon atomique — an incremental game in the terminal.
//!
//! Launch: champignon (link ~/.local/bin/champignon → the release binary) or cargo run --release
//! (--reset to erase all games, --mute for no sound).
//!
//! At startup, if games exist, a menu offers to continue one of them or to create a new one.

use std::io::{self, BufRead, Write};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use champignon::game::{Game, MsgKind};
use champignon::saves::{self, MAIN_SLOT, MAX_SLOTS, Saves};
use champignon::screens::{self, INTRO};
use champignon::sound::{Sound, SoundSink};
use champignon::term::{self, Screen, TermScreen};
use champignon::ui::{Flow, UI};

const FRAME_DURATION: f64 = 0.05;
const AUTOSAVE_INTERVAL: f64 = 30.0;
const MAX_TICK: f64 = 1.0; // after a sleep, the game doesn't catch up on all the elapsed time at once
const YES: [&str; 4] = ["o", "oui", "y", "yes"];

fn log_purged(g: &mut Game, purged: &[u32]) {
    let names: Vec<String> = purged
        .iter()
        .map(|&n| {
            if n == MAIN_SLOT {
                "partie principale".to_string()
            } else {
                format!("partie {n}")
            }
        })
        .collect();
    let prefix = if purged.len() > 1 {
        "Sauvegardes trop anciennes supprimées : "
    } else {
        "Sauvegarde trop ancienne supprimée : "
    };
    g.log(format!("{prefix}{}", names.join(", ")), MsgKind::Bad, None);
}

/// Plays until the player quits (or a signal arrives). Returns the path of the saved game, None if
/// the player left the menu without choosing one.
fn run(saves: &Saves, purged: &[u32], mute: bool) -> io::Result<Option<PathBuf>> {
    let mut scr = TermScreen::new()?;
    let sound = Sound::new(!mute);
    let slots = saves.save_slots();
    // With no readable game, start on the first free slot (main checked that one exists).
    let path = if slots.is_empty() {
        saves.free_slot().map(|n| saves.slot_path(n))
    } else {
        screens::save_menu(&mut scr, saves, &slots, Some(&sound))
    };
    let Some(path) = path else { return Ok(None) };
    let mut g = match Game::load(&path) {
        Some(g) => g,
        None => {
            let g = Game::default();
            if screens::intro(&mut scr, Some(&sound), INTRO, 0.0, "commencer").is_err() {
                return Ok(None); // Ctrl+C before the game even started: nothing to save
            }
            g
        }
    };
    if !purged.is_empty() {
        log_purged(&mut g, purged);
    }
    let mut ui = UI::new(scr, g, Some(Box::new(sound) as Box<dyn SoundSink>));
    if term::can_set_title() {
        ui.title = Some(String::new()); // Some enables the title (None disables it)
    }
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| play(&mut ui, &path)));
    // Even after a panic (the hook already restored the terminal); before dropping ui otherwise.
    let saved = ui.g.save(&path);
    if let Err(payload) = outcome {
        panic::resume_unwind(payload);
    }
    saved?;
    Ok(Some(path))
}

/// Game loop, until Q, Ctrl+C or a signal (read_key then gives Key::Interrupt).
fn play(ui: &mut UI<TermScreen>, path: &Path) {
    let start = Instant::now();
    let clock = || start.elapsed().as_secs_f64();
    let (mut last, mut last_save) = (clock(), clock());
    'game: loop {
        let now = clock();
        let dt = (now - last).min(MAX_TICK);
        last = now;
        while let Some(key) = ui.scr.read_key(Some(Duration::ZERO)) {
            if ui.handle(key) == Flow::Quit {
                break 'game;
            }
        }
        ui.g.tick(dt);
        ui.draw(dt);
        if ui.show_story().is_err() {
            break; // blocking full screen: outside of draw
        }
        if now - last_save > AUTOSAVE_INTERVAL {
            let _ = ui.g.save(path);
            last_save = now;
        }
        thread::sleep(Duration::from_secs_f64(
            (FRAME_DURATION - (clock() - now)).max(0.0),
        ));
    }
}

/// Erases all games after confirmation. Returns false if the player backs out.
/// answer: the line typed by the player, None on end of input.
fn reset(saves: &Saves, answer: impl FnOnce() -> Option<String>, out: &mut impl Write) -> bool {
    let files = saves.all_files();
    if files.is_empty() {
        let _ = writeln!(out, "Aucune partie à effacer.");
        return true;
    }
    let list: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
    let _ = writeln!(out, "Parties à effacer :\n  {}", list.join("\n  "));
    let _ = write!(out, "Effacer définitivement ? [o/N] ");
    let _ = out.flush();
    match answer() {
        Some(a) if YES.contains(&a.trim().to_lowercase().as_str()) => {}
        Some(_) => {
            let _ = writeln!(out, "Rien n'a été effacé.");
            return false;
        }
        None => {
            let _ = writeln!(out, "\nRien n'a été effacé.");
            return false;
        }
    }
    let failed = saves::delete(&files);
    if !failed.is_empty() {
        let _ = writeln!(out, "Impossible d'effacer :\n  {}", failed.join("\n  "));
        return false;
    }
    let _ = writeln!(out, "Parties effacées.");
    true
}

/// Never overwrite an unreadable file or one from a newer version: refuse to start instead.
fn no_usable_slot(saves: &Saves) -> bool {
    saves.save_slots().is_empty() && saves.free_slot().is_none()
}

fn read_line() -> Option<String> {
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(n) if n > 0 => Some(line),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let saves = Saves::from_env();
    if args.iter().any(|a| a == "--reset") && !reset(&saves, read_line, &mut io::stdout()) {
        return;
    }
    let purged = saves.purge_obsolete();
    if no_usable_slot(&saves) {
        println!(
            "Aucune partie utilisable et aucun emplacement libre ({MAX_SLOTS} fichiers) : \
             supprime un fichier de sauvegarde ou lance champignon --reset."
        );
        return;
    }
    // Terminal closed or kill: every screen quits like with Ctrl+C and the game is saved.
    for sig in [
        signal_hook::consts::SIGHUP,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ] {
        let _ = signal_hook::flag::register(sig, Arc::clone(term::interrupt_flag()));
    }
    term::install_panic_hook();
    let mute = args.iter().any(|a| a == "--mute");
    match run(&saves, &purged, mute) {
        // let _: after SIGHUP stdout is gone and println! would panic.
        Ok(Some(path)) => {
            let _ = writeln!(io::stdout(), "Partie sauvegardée dans {}", path.display());
        }
        Ok(None) => {}
        Err(e) => {
            term::restore_terminal();
            eprintln!("Erreur : {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempSaves {
        saves: Saves,
        files: Vec<PathBuf>,
    }

    impl TempSaves {
        fn new() -> TempSaves {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("champignon-main-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            let saves = Saves::new(&dir);
            let mut files = vec![saves.slot_path(1), saves.slot_path(2), saves.slot_path(7)];
            files.push(PathBuf::from(format!(
                "{}.tmp",
                saves.slot_path(1).display()
            )));
            for p in &files {
                fs::write(p, "{}").unwrap();
            }
            TempSaves { saves, files }
        }

        fn reset(&self, answer: Option<&str>) -> bool {
            reset(&self.saves, || answer.map(String::from), &mut Vec::new())
        }

        fn all_exist(&self) -> bool {
            self.files.iter().all(|p| p.exists())
        }
    }

    impl Drop for TempSaves {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.saves.dir);
        }
    }

    #[test]
    fn confirmed_deletes_every_slot() {
        let t = TempSaves::new();
        assert!(t.reset(Some("o\n")));
        assert!(t.files.iter().all(|p| !p.exists()));
    }

    #[test]
    fn refused_keeps_everything() {
        let t = TempSaves::new();
        for answer in [Some(""), Some("n"), None] {
            assert!(!t.reset(answer), "{answer:?}");
            assert!(t.all_exist(), "{answer:?}");
        }
    }

    #[test]
    fn other_files_untouched() {
        let t = TempSaves::new();
        let other = t.saves.dir.join("autre.json");
        fs::write(&other, "{}").unwrap();
        t.reset(Some("o"));
        assert!(other.exists());
    }

    #[test]
    fn nothing_to_delete() {
        let t = TempSaves::new();
        t.reset(Some("o"));
        assert!(t.reset(Some("o")));
    }

    #[test]
    fn failed_delete_does_not_start() {
        let t = TempSaves::new();
        // A non-empty directory where a save goes: it cannot be removed as a file.
        let blocked = t.saves.slot_path(3);
        fs::create_dir_all(blocked.join("x")).unwrap();
        assert!(!t.reset(Some("o")));
    }

    #[test]
    fn refuses_to_start_instead_of_overwriting() {
        let t = TempSaves::new();
        assert!(!no_usable_slot(&t.saves));
        for n in Saves::slot_numbers() {
            fs::write(t.saves.slot_path(n), "illisible").unwrap();
        }
        assert!(no_usable_slot(&t.saves));
    }
}
