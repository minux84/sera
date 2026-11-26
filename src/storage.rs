use crate::model::Preferences;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn default_path(webos: bool) -> Result<PathBuf, String> {
    if webos {
        #[cfg(all(feature = "webos", unix))]
        return tv_path();
        #[cfg(not(all(feature = "webos", unix)))]
        return Err("TV storage requires a webOS build".into());
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
        .ok_or("Set HOME, XDG_CONFIG_HOME or --preferences for native preferences")?;
    Ok(base.join("sera-native/preferences.json"))
}

#[cfg(all(feature = "webos", unix))]
fn tv_path() -> Result<PathBuf, String> {
    use std::{
        io::Write,
        os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    };

    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let legacy = executable.with_file_name("preferences.json");
    let id = crate::platform::APP_ID;
    // These volumes persist across boots. The app-specific directory also
    // survives replacement of the installed payload on an ordinary update.
    // Production native jails expose internal storage instead of developer.
    let developer = PathBuf::from("/media/developer")
        .join(id)
        .join("preferences.json");
    let shared_developer = PathBuf::from("/media/developer/temp")
        .join(id)
        .join("preferences.json");
    let internal = PathBuf::from("/media/internal")
        .join(format!(".{id}"))
        .join("preferences.json");
    let mut candidates = if executable.starts_with("/media/cryptofs") {
        [internal, developer, shared_developer]
    } else {
        [developer, shared_developer, internal]
    };
    // Keep a previously used destination even if another candidate becomes
    // writable after a reboot; do not accidentally restart with fresh settings.
    candidates.sort_by_key(|path| !path.exists());
    let mut failures = Vec::new();
    for path in candidates {
        let directory = path.parent().ok_or("Invalid TV storage directory")?;
        // Create only this app's leaf, not a missing system mount hierarchy.
        let created = fs::DirBuilder::new().mode(0o700).create(directory);
        if let Err(error) = created
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            failures.push(format!("{}: {error}", directory.display()));
            continue;
        }
        let metadata = match fs::symlink_metadata(directory) {
            Ok(metadata) => metadata,
            Err(error) => {
                failures.push(format!("{}: {error}", directory.display()));
                continue;
            }
        };
        // Do not silently reuse a symlink or root-created debug directory as
        // another process's private app storage.
        if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
            failures.push(format!(
                "{} is not owned by the app user",
                directory.display()
            ));
            continue;
        }
        let probe = directory.join(format!(".write-probe-{}", std::process::id()));
        let writable = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&probe)?;
            let result = file
                .write_all(b"Sera storage probe")
                .and_then(|_| file.sync_all());
            drop(file);
            // Only remove a probe that this process created successfully.
            let cleanup = fs::remove_file(&probe);
            result.and(cleanup)
        })();
        if let Err(error) = writable {
            failures.push(format!("{}: {error}", directory.display()));
            continue;
        }
        if !path.exists() {
            // Import JSON from the old path in THIS installation, or else from
            // Luma, which Sera replaces. Never browser storage or any other app ID.
            match fs::read(&legacy) {
                Ok(bytes) => {
                    let mut preferences: Preferences =
                        serde_json::from_slice(&bytes).map_err(|e| {
                            format!(
                                "Existing preferences {} could not be read: {e}",
                                legacy.display()
                            )
                        })?;
                    migrate_selection(&bytes, &mut preferences);
                    preferences.sanitize();
                    save(&path, &preferences)?;
                    eprintln!(
                        "Sera preferences migrated from {} to {}",
                        legacy.display(),
                        path.display()
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    import_from_luma(&path, directory);
                }
                Err(error) => {
                    return Err(format!(
                        "Existing preferences {} could not be read: {error}",
                        legacy.display()
                    ));
                }
            }
            let old_cache = crate::weather_service::cache_path(&legacy);
            let cache = crate::weather_service::cache_path(&path);
            if !cache.exists()
                && let Some(reading) = crate::weather_service::load_cache(&old_cache)
                && let Err(error) = crate::weather_service::save_cache(&cache, &reading)
            {
                eprintln!("Sera weather cache migration: {error}");
            }
        }
        eprintln!("Sera persistent storage: {}", directory.display());
        return Ok(path);
    }
    Err(format!(
        "No writable persistent TV storage found: {}",
        failures.join("; ")
    ))
}

/// The launcher Sera was forked from. Its settings are imported once, into a
/// Sera that has none yet.
pub const LUMA_APP_ID: &str = "com.denisefremov.luma";

/// Files Luma kept beside its preferences that Sera reads under the same names.
pub const LUMA_FILES: [&str; 4] = [
    "preferences.json",
    "preferences.weather.json",
    "ha-auth.json",
    "preferences.artwork",
];

/// First Sera start on a TV that had Luma: bring its settings over. Best
/// effort: on any failure Sera simply starts with defaults, as before.
#[cfg(all(feature = "webos", unix))]
fn import_from_luma(path: &Path, directory: &Path) {
    let id = LUMA_APP_ID;
    let sources = [
        PathBuf::from("/media/developer").join(id),
        PathBuf::from("/media/developer/temp").join(id),
        PathBuf::from("/media/internal").join(format!(".{id}")),
    ];
    // Stat the directory itself: Luma created it 0700 under its own user, so
    // its files may not even be visible to Sera.
    for source in sources
        .iter()
        .filter(|source| fs::symlink_metadata(source).is_ok_and(|m| m.is_dir()))
    {
        // Root (Homebrew Channel) can read Luma's private files and hand Sera
        // a copy it owns; without root, read them directly where permitted.
        let staging = directory.join(".luma-import");
        let staged = crate::platform::root_copy(
            source,
            &staging,
            &LUMA_FILES,
            unsafe { libc::geteuid() },
            unsafe { libc::getegid() },
        )
        .map_err(|error| eprintln!("Sera Luma import without root: {error}"))
        .is_ok();
        let result = import_luma(if staged { &staging } else { source }, path);
        if staged && let Err(error) = fs::remove_dir_all(&staging) {
            eprintln!(
                "Sera Luma import: could not remove {}: {error}",
                staging.display()
            );
        }
        match result {
            Ok(()) => {
                eprintln!("Sera settings imported from Luma: {}", source.display());
                return;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => eprintln!("Sera Luma import from {}: {error}", source.display()),
        }
    }
}

/// Imports a Luma storage directory into a Sera that has no preferences yet:
/// the preferences file (sanitized like any load), then best effort the
/// weather cache, the Home Assistant sign-in and custom app artwork. Fails
/// only when the preferences themselves cannot be imported.
pub fn import_luma(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    if destination.exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            "Sera already has preferences",
        ));
    }
    let bytes = fs::read(source.join("preferences.json"))?;
    let mut preferences: Preferences = serde_json::from_slice(&bytes)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Luma preferences: {e}")))?;
    migrate_selection(&bytes, &mut preferences);
    preferences.sanitize();
    save(destination, &preferences).map_err(Error::other)?;

    let weather = crate::weather_service::cache_path(destination);
    if !weather.exists()
        && let Some(cache) =
            crate::weather_service::load_cache(&source.join("preferences.weather.json"))
        && let Err(error) = crate::weather_service::save_cache(&weather, &cache)
    {
        eprintln!("Sera Luma import, weather cache: {error}");
    }
    let auth = crate::smart_home::auth_path(destination);
    if let Err(error) = crate::smart_home::import_auth(&source.join("ha-auth.json"), &auth) {
        eprintln!("Sera Luma import, Home Assistant sign-in: {error}");
    }
    if let Err(error) = copy_artwork(
        &source.join("preferences.artwork"),
        &destination.with_extension("artwork"),
    ) {
        eprintln!("Sera Luma import, app artwork: {error}");
    }
    Ok(())
}

/// Copies regular files only (no symlinks, no subfolders) and never replaces
/// artwork Sera already has.
fn copy_artwork(from: &Path, to: &Path) -> std::io::Result<()> {
    let entries = match fs::read_dir(from) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    fs::create_dir_all(to)?;
    let mut failure = None;
    for entry in entries {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if !entry.file_type()?.is_file() || target.exists() {
            continue;
        }
        if let Err(error) = fs::copy(entry.path(), &target) {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub fn load(path: &Path) -> Result<Preferences, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let mut preferences: Preferences = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Preferences could not be read: {e}"))?;
            migrate_selection(&bytes, &mut preferences);
            preferences.sanitize();
            Ok(preferences)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
        Err(e) => Err(format!("Preferences could not be read: {e}")),
    }
}

pub fn save(path: &Path, preferences: &Preferences) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not prepare preferences directory {}: {e}",
                parent.display()
            )
        })?;
    }
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(preferences)
        .map_err(|e| format!("Could not encode preferences: {e}"))?;
    // TVs are often switched off at the wall. Without fsync the rename can
    // reach flash before the data does, leaving an empty preferences file.
    fs::File::create(&temporary)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(&bytes)?;
            file.sync_all()
        })
        .map_err(|e| {
            format!(
                "Could not write preferences temporary file {}: {e}",
                temporary.display()
            )
        })?;
    fs::rename(&temporary, path)
        .map_err(|e| format!("Could not replace preferences file {}: {e}", path.display()))?;
    sync_directory(path);
    Ok(())
}

/// Makes the rename itself durable. Best effort: not every platform can
/// open a directory for syncing, and the data is already on disk.
fn sync_directory(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
        && let Ok(directory) = fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Startup load for the launcher itself: it must always come up, so an
/// unreadable or corrupt preferences file falls back to defaults. A corrupt
/// file is kept aside as `preferences.json.corrupt` for diagnosis and is not
/// overwritten by the next save.
pub fn load_or_recover(path: &Path) -> Preferences {
    match load(path) {
        Ok(preferences) => preferences,
        Err(error) => {
            eprintln!("{error}; starting with default preferences");
            if path.is_file() {
                let aside = path.with_extension("json.corrupt");
                match fs::rename(path, &aside) {
                    Ok(()) => eprintln!("Unreadable preferences kept as {}", aside.display()),
                    Err(e) => eprintln!("Could not set unreadable preferences aside: {e}"),
                }
            }
            Preferences::default()
        }
    }
}

/// A missing modern field (not an explicitly empty set) activates one-time migration.
fn migrate_selection(bytes: &[u8], preferences: &mut Preferences) {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) {
        // Migrate the original five paired presets only if no independent choice was saved.
        if value.get("heading_font").is_none() && value.get("text_font").is_none() {
            let old = value
                .get("font_pair")
                .and_then(|v| v.as_str())
                .unwrap_or("cinema");
            use crate::typography::{HeadingFont as H, TextFont as T};
            let (h, t) = match old {
                "editorial" => (H::Fraunces, T::Inter),
                "geometric" => (H::Unbounded, T::Manrope),
                "classic" | "original" => (H::Inter, T::Inter),
                _ => (H::Archivo, T::Inter),
            };
            preferences.heading_font = h;
            preferences.text_font = t;
        }
    }
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes)
        && value.get("video_excluded").is_none()
    {
        let permitted = crate::video_catalog::filtered(preferences.video_sources_mask);
        for (i, video) in crate::video_catalog::all().iter().enumerate() {
            if !permitted.contains(&i) {
                preferences.video_excluded.insert(video.id.clone());
            }
        }
    }
}
