//! Diagnostic logs live on TV flash and Sera runs around the clock, so an
//! append-only log must never grow without bound. Past `MAX_BYTES` the file
//! becomes `<name>.prev.log` (one backup, replacing the older one) and a fresh
//! file is started: at most ~2 MiB per log on disk.
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

pub const MAX_BYTES: u64 = 1024 * 1024;

/// Opens `path` for appending, rotating it first when it is over the cap.
pub fn append(path: impl AsRef<Path>) -> std::io::Result<File> {
    let path = path.as_ref();
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(path, backup(path));
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// "sera-demux.log" → "sera-demux.prev.log", as the native metrics log does.
fn backup(path: &Path) -> PathBuf {
    path.with_extension("prev.log")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rotates_once_over_the_cap_and_keeps_one_backup() {
        let directory = std::env::temp_dir().join(format!("sera-logs-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("sera-test.log");
        assert_eq!(backup(&path), directory.join("sera-test.prev.log"));

        append(&path).unwrap().write_all(b"small\n").unwrap();
        append(&path).unwrap().write_all(b"more\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"small\nmore\n");
        assert!(!backup(&path).exists());

        std::fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        append(&path).unwrap().write_all(b"fresh\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"fresh\n");
        assert_eq!(
            std::fs::metadata(backup(&path)).unwrap().len(),
            MAX_BYTES + 1
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
