use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use serde_json::{Map, Value};

const DEFAULT_WINDOW_WIDTH: f64 = 1280.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct WindowBounds {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub window_bounds: WindowBounds,
    pub managed_mode: bool,
    pub close_to_tray: bool,
    pub show_on_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_bounds: WindowBounds {
                width: DEFAULT_WINDOW_WIDTH,
                height: DEFAULT_WINDOW_HEIGHT,
            },
            managed_mode: false,
            close_to_tray: true,
            show_on_startup: true,
        }
    }
}

#[derive(Debug)]
pub struct SettingsError {
    operation: &'static str,
    path: PathBuf,
    source: io::Error,
}

impl SettingsError {
    fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to {} settings at {}: {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
struct State {
    settings: Settings,
    record: Map<String, Value>,
}

/// Thread-safe settings persistence compatible with the upstream `settings.json` format.
#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
    state: Mutex<State>,
}

impl SettingsStore {
    /// Loads a settings file. Missing, unreadable, malformed, or non-object files use defaults,
    /// matching the upstream application's recovery behavior.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, SettingsError> {
        let path = path.as_ref().to_path_buf();
        let record = load_record(&path);
        let settings = settings_from_record(&record);

        Ok(Self {
            path,
            state: Mutex::new(State { settings, record }),
        })
    }

    /// Returns an owned, internally consistent view of the current settings.
    pub fn snapshot(&self) -> Settings {
        self.lock_state().settings.clone()
    }

    /// Persists a mutation atomically before making it observable to other threads.
    pub fn update<F>(&self, mutation: F) -> Result<Settings, SettingsError>
    where
        F: FnOnce(&mut Settings),
    {
        let mut state = self.lock_state();
        let mut next = state.settings.clone();
        mutation(&mut next);
        normalize_settings(&mut next);

        let mut next_record = state.record.clone();
        write_settings_to_record(&mut next_record, &next);
        atomic_write(&self.path, &next_record)?;

        state.settings = next.clone();
        state.record = next_record;
        Ok(next)
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn load_record(path: &Path) -> Map<String, Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
        .and_then(|value| match value {
            Value::Object(record) => Some(record),
            _ => None,
        })
        .unwrap_or_default()
}

fn settings_from_record(record: &Map<String, Value>) -> Settings {
    let mut settings = Settings {
        window_bounds: window_bounds(record.get("windowBounds")).unwrap_or(WindowBounds {
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
        }),
        managed_mode: boolean(record.get("managedMode")).unwrap_or(false),
        close_to_tray: boolean(record.get("closeToTray")).unwrap_or(true),
        show_on_startup: boolean(record.get("showOnStartup")).unwrap_or(true),
    };
    normalize_settings(&mut settings);
    settings
}

fn window_bounds(value: Option<&Value>) -> Option<WindowBounds> {
    let Value::Object(bounds) = value? else {
        return None;
    };

    let width = bounds.get("width")?.as_f64()?;
    let height = bounds.get("height")?.as_f64()?;
    (width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0)
        .then_some(WindowBounds { width, height })
}

fn boolean(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

fn normalize_settings(settings: &mut Settings) {
    if !settings.window_bounds.width.is_finite()
        || settings.window_bounds.width <= 0.0
        || !settings.window_bounds.height.is_finite()
        || settings.window_bounds.height <= 0.0
    {
        settings.window_bounds = WindowBounds {
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
        };
    }
}

fn write_settings_to_record(record: &mut Map<String, Value>, settings: &Settings) {
    record.insert(
        "windowBounds".to_owned(),
        serde_json::json!({
            "width": settings.window_bounds.width,
            "height": settings.window_bounds.height,
        }),
    );
    record.insert("managedMode".to_owned(), Value::Bool(settings.managed_mode));
    record.insert(
        "closeToTray".to_owned(),
        Value::Bool(settings.close_to_tray),
    );
    record.insert(
        "showOnStartup".to_owned(),
        Value::Bool(settings.show_on_startup),
    );
    record.remove("proxyProfileId");
}

fn atomic_write(path: &Path, record: &Map<String, Value>) -> Result<(), SettingsError> {
    let parent = parent_dir(path);
    if !parent.exists() {
        fs::create_dir_all(parent)
            .map_err(|error| SettingsError::io("create settings directory", parent, error))?;
    }

    let bytes = serde_json::to_vec(record).map_err(|error| {
        SettingsError::io(
            "encode",
            path,
            io::Error::new(io::ErrorKind::InvalidData, error),
        )
    })?;
    let (temporary_path, mut temporary_file) = create_temporary_file(parent, path)?;

    let result =
        write_temporary_file(&mut temporary_file, &bytes, &temporary_path).and_then(|()| {
            drop(temporary_file);
            fs::rename(&temporary_path, path)
                .map_err(|error| SettingsError::io("replace", path, error))
        });

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    result
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn create_temporary_file(
    parent: &Path,
    settings_path: &Path,
) -> Result<(PathBuf, File), SettingsError> {
    let file_name = settings_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("settings.json");

    for _ in 0..32 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(SettingsError::io("create temporary file for", path, error)),
        }
    }

    Err(SettingsError::io(
        "create temporary file for",
        settings_path,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a unique temporary settings file",
        ),
    ))
}

fn write_temporary_file(
    file: &mut File,
    bytes: &[u8],
    temporary_path: &Path,
) -> Result<(), SettingsError> {
    file.write_all(bytes)
        .map_err(|error| SettingsError::io("write", temporary_path, error))?;
    file.sync_all()
        .map_err(|error| SettingsError::io("sync", temporary_path, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_write_discards_obsolete_proxy_selection_without_losing_other_settings() {
        let directory = std::env::temp_dir().join(format!(
            "telesram-settings-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(
            &path,
            r#"{"managedMode":true,"proxyProfileId":"old-profile","userOwned":{"theme":"dark"}}"#,
        )
        .unwrap();

        let store = SettingsStore::load(&path).unwrap();
        assert!(store.snapshot().managed_mode);
        store
            .update(|settings| settings.show_on_startup = false)
            .unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved.get("proxyProfileId"), None);
        assert_eq!(saved["userOwned"], serde_json::json!({"theme": "dark"}));
        assert_eq!(saved["managedMode"], true);
        assert_eq!(saved["showOnStartup"], false);

        let reloaded = SettingsStore::load(&path).unwrap();
        assert!(reloaded.snapshot().managed_mode);
        assert!(!reloaded.snapshot().show_on_startup);
        fs::remove_dir_all(directory).unwrap();
    }
}
