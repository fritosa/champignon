//! Save files: game slots, format version, reading and atomic writing.
//!
//! Only deals with JSON objects: their content is the business of Game::save_data / Game::from_save.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Raw content of a save.
pub type RawSave = Map<String, Value>;

/// Environment variable that moves the save directory (tests, several installs).
pub const DIR_ENV: &str = "CHAMPIGNON_SAVE_DIR";
pub const MAX_SLOTS: u32 = 99;
// Format version. On a format change, increment SAVE_VERSION. If supporting older saves becomes
// too complicated, also raise MIN_SAVE_VERSION: older files are then deleted at startup
// (purge_obsolete) instead of being converted.
pub const SAVE_VERSION: i64 = 1;
pub const MIN_SAVE_VERSION: i64 = 1;

/// Save directory: $CHAMPIGNON_SAVE_DIR, otherwise ~/.champignon.
pub fn default_dir() -> PathBuf {
    dir_from(std::env::var_os(DIR_ENV), std::env::var_os("HOME"))
}

/// default_dir from the values of the environment variables.
pub fn dir_from(env_dir: Option<OsString>, home: Option<OsString>) -> PathBuf {
    match env_dir.filter(|d| !d.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.unwrap_or_else(|| ".".into())).join(".champignon"),
    }
}

/// Version of a raw save (0 if missing or invalid).
pub fn version(d: &RawSave) -> i64 {
    d.get("version").and_then(Value::as_i64).unwrap_or(0)
}

/// Save readable by this version: neither too old nor written by a newer version
/// (which must not be rewritten with a lower number, or a newer version would purge it).
pub fn supported(d: &RawSave) -> bool {
    (MIN_SAVE_VERSION..=SAVE_VERSION).contains(&version(d))
}

/// Raw content of a save, None if missing or unreadable.
pub fn read(path: &Path) -> Option<RawSave> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text).ok()? {
        Value::Object(d) => Some(d),
        _ => None,
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(".tmp");
    PathBuf::from(p)
}

/// Atomic write (.tmp then rename), creating the directory if needed.
pub fn write(path: &Path, d: &Value) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let tmp = tmp_path(path);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&serde_json::to_vec(d)?)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        delete(&[tmp]); // the real save is intact; don't leave a half-written file behind
    }
    result
}

/// Deletes the files; returns those that could not be deleted, with the reason.
pub fn delete(paths: &[PathBuf]) -> Vec<String> {
    let mut failed = Vec::new();
    for p in paths {
        match fs::remove_file(p) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                let reason = e.to_string();
                let reason = reason.split(" (os error").next().unwrap_or_default();
                failed.push(format!("{} ({reason})", p.display()));
            }
            _ => {}
        }
    }
    failed
}

/// The game slots of a save directory.
#[derive(Clone, Debug)]
pub struct Saves {
    pub dir: PathBuf,
    /// Oldest readable version (MIN_SAVE_VERSION; changed by tests only).
    pub min_version: i64,
    /// Current version (SAVE_VERSION; changed by tests only).
    pub version: i64,
}

impl Saves {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Saves {
            dir: dir.into(),
            min_version: MIN_SAVE_VERSION,
            version: SAVE_VERSION,
        }
    }

    /// Slots of the default directory (default_dir).
    pub fn from_env() -> Self {
        Saves::new(default_dir())
    }

    pub fn slot_numbers() -> std::ops::RangeInclusive<u32> {
        1..=MAX_SLOTS
    }

    /// File of game n.
    pub fn slot_path(&self, n: u32) -> PathBuf {
        self.dir.join(format!("save_{n}.json"))
    }

    /// Save too old for this version of the game.
    pub fn obsolete(&self, d: &RawSave) -> bool {
        version(d) < self.min_version
    }

    pub fn supported(&self, d: &RawSave) -> bool {
        (self.min_version..=self.version).contains(&version(d))
    }

    /// Readable games: list of (number, path, raw data), in slot order.
    pub fn save_slots(&self) -> Vec<(u32, PathBuf, RawSave)> {
        Self::slot_numbers()
            .filter_map(|n| {
                let path = self.slot_path(n);
                let d = read(&path).filter(|d| self.supported(d))?;
                Some((n, path, d))
            })
            .collect()
    }

    /// First number without a file (an unreadable file is never overwritten), None if all are taken.
    pub fn free_slot(&self) -> Option<u32> {
        Self::slot_numbers().find(|&n| !self.slot_path(n).exists())
    }

    /// Deletes saves that are too old. Returns the numbers of the deleted games.
    pub fn purge_obsolete(&self) -> Vec<u32> {
        let mut gone = Vec::new();
        for n in Self::slot_numbers() {
            let path = self.slot_path(n);
            // Missing or damaged file (None): never deleted.
            if read(&path).is_some_and(|d| self.obsolete(&d)) {
                delete(&[path.clone(), tmp_path(&path)]);
                if !path.exists() {
                    gone.push(n);
                }
            }
        }
        gone
    }

    /// All existing game files, including .tmp and unreadable files.
    pub fn all_files(&self) -> Vec<PathBuf> {
        Self::slot_numbers()
            .flat_map(|n| {
                let path = self.slot_path(n);
                let tmp = tmp_path(&path);
                [path, tmp]
            })
            .filter(|p| p.exists())
            .collect()
    }
}

/// Unique temporary directory for a test, removed when dropped.
#[cfg(test)]
pub(crate) struct TempDir(pub PathBuf);

#[cfg(test)]
impl TempDir {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("champignon-tests-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        TempDir(dir)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, tests::exploded_game};
    use serde_json::json;

    struct Slots {
        _tmp: TempDir,
        saves: Saves,
    }

    fn setup() -> Slots {
        let tmp = TempDir::new();
        let saves = Saves::new(tmp.0.clone());
        Slots { _tmp: tmp, saves }
    }

    impl Slots {
        fn write(&self, n: u32, content: &str) {
            fs::write(self.saves.slot_path(n), content).unwrap();
        }
    }

    const V1: &str = r#"{"version": 1}"#;

    #[test]
    fn paths() {
        let s = setup();
        assert_eq!(s.saves.slot_path(1), s.saves.dir.join("save_1.json"));
        assert_eq!(s.saves.slot_path(2), s.saves.dir.join("save_2.json"));
    }

    #[test]
    fn default_dir_follows_env() {
        assert_eq!(
            dir_from(Some("/x/y".into()), Some("/home/a".into())),
            PathBuf::from("/x/y")
        );
        assert_eq!(
            dir_from(None, Some("/home/a".into())),
            PathBuf::from("/home/a/.champignon")
        );
        assert_eq!(
            dir_from(Some("".into()), Some("/home/a".into())),
            PathBuf::from("/home/a/.champignon")
        );
    }

    #[test]
    fn empty() {
        let s = setup();
        assert!(s.saves.save_slots().is_empty());
        assert_eq!(s.saves.free_slot(), Some(1));
    }

    #[test]
    fn listing_and_free_slot() {
        let s = setup();
        s.write(1, r#"{"level": 4, "version": 1}"#);
        s.write(3, V1);
        s.write(4, "corrompu");
        let slots = s.saves.save_slots();
        assert_eq!(slots.iter().map(|(n, _, _)| *n).collect::<Vec<_>>(), [1, 3]);
        assert_eq!(
            Value::Object(slots[0].2.clone()),
            json!({"level": 4, "version": 1})
        );
        assert_eq!(s.saves.free_slot(), Some(2));
    }

    #[test]
    fn main_reused_when_deleted() {
        let s = setup();
        s.write(2, V1);
        assert_eq!(s.saves.free_slot(), Some(1));
    }

    #[test]
    fn unreadable_file_never_reused() {
        let s = setup();
        s.write(1, "corrompu");
        s.write(2, "[]");
        assert!(s.saves.save_slots().is_empty());
        assert_eq!(s.saves.free_slot(), Some(3));
    }

    #[test]
    fn full() {
        let s = setup();
        for n in Saves::slot_numbers() {
            s.write(n, V1);
        }
        assert_eq!(s.saves.save_slots().len(), MAX_SLOTS as usize);
        assert_eq!(s.saves.free_slot(), None);
    }

    #[test]
    fn current_version_written() {
        let s = setup();
        exploded_game().save(&s.saves.slot_path(1)).unwrap();
        assert_eq!(version(&s.saves.save_slots()[0].2), SAVE_VERSION);
        assert!(s.saves.purge_obsolete().is_empty());
    }

    #[test]
    fn obsolete_purged() {
        let mut s = setup();
        s.write(1, V1);
        s.write(2, r#"{"version": 1, "level": 3}"#);
        s.write(3, r#"{"level": 3}"#); // no version
        s.write(4, r#"{"version": "2"}"#);
        s.write(5, "corrompu");
        fs::write(tmp_path(&s.saves.slot_path(2)), "{}").unwrap();
        s.saves.min_version = 2;
        s.saves.version = 2;
        assert!(!s.saves.supported(&read(&s.saves.slot_path(2)).unwrap()));
        assert!(s.saves.save_slots().is_empty());
        assert_eq!(s.saves.purge_obsolete(), [1, 2, 3, 4]);
        for n in 1..=4 {
            assert!(!s.saves.slot_path(n).exists(), "{n}");
        }
        assert!(!tmp_path(&s.saves.slot_path(2)).exists());
        assert!(s.saves.slot_path(5).exists()); // unreadable: never deleted
        assert_eq!(s.saves.free_slot(), Some(1));
    }

    #[test]
    fn newer_version_kept() {
        let s = setup();
        s.write(1, r#"{"version": 99}"#);
        assert!(s.saves.purge_obsolete().is_empty());
        assert!(s.saves.slot_path(1).exists());
        // Not offered, loaded or reused: it is never rewritten with a lower version.
        assert!(s.saves.save_slots().is_empty());
        assert!(Game::load(&s.saves.slot_path(1)).is_none());
        assert_eq!(s.saves.free_slot(), Some(2));
    }

    #[test]
    fn atomic_write() {
        let s = setup();
        let main = s.saves.slot_path(1);
        write(&main, &json!({"level": 2})).unwrap();
        assert_eq!(Value::Object(read(&main).unwrap()), json!({"level": 2}));
        assert!(!tmp_path(&main).exists());
    }

    #[test]
    fn write_creates_directory() {
        let s = setup();
        let path = s.saves.dir.join("sub").join("save_1.json");
        write(&path, &json!({"level": 2})).unwrap();
        assert!(read(&path).is_some());
    }

    #[test]
    fn failed_write_leaves_no_tmp() {
        let s = setup();
        // A non-empty directory in place of the save: the final rename fails.
        let main = s.saves.slot_path(1);
        fs::create_dir(&main).unwrap();
        fs::write(main.join("x"), "").unwrap();
        assert!(write(&main, &json!({"level": 2})).is_err());
        assert!(!tmp_path(&main).exists());
        assert!(main.join("x").exists());
    }

    #[test]
    fn all_files_and_delete() {
        let s = setup();
        s.write(1, V1);
        s.write(3, "corrompu");
        fs::write(tmp_path(&s.saves.slot_path(2)), "{}").unwrap();
        let mut files = s.saves.all_files();
        assert_eq!(
            files,
            [
                s.saves.slot_path(1),
                tmp_path(&s.saves.slot_path(2)),
                s.saves.slot_path(3)
            ]
        );
        files.push(s.saves.slot_path(9));
        assert!(delete(&files).is_empty()); // missing file: ignored
        assert!(s.saves.all_files().is_empty());
    }
}
