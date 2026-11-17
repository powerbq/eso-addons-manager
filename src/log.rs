use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use iced::futures::channel::mpsc;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

const DEFAULT_FILTER: &str = "warn,app=info,eso_addons_manager_core=info";

pub const DIRECTORY: &str = "logs";
const LOG_NAME: &str = "app";
const LOG_SUFFIX: &str = ".log";
const KEEP_FILES: usize = 10;

pub fn setup() -> (mpsc::UnboundedReceiver<String>, FileLog) {
    let (sender, receiver) = mpsc::unbounded();
    let file = FileLog::new(PathBuf::from(DIRECTORY));

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER)))
        .with(fmt::layer())
        .with(fmt::layer().with_ansi(false).with_writer(Channel(sender)))
        .with(fmt::layer().with_ansi(false).with_writer(file.writer()))
        .init();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("unnamed");
        tracing::error!("panic on thread '{name}': {info}");
        default_hook(info);
    }));

    (receiver, file)
}

pub fn banner() {
    tracing::info!(
        "eso-addons-manager {} starting on {} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

/// Runtime-switchable file output for the log, shared with the writing layer.
#[derive(Clone)]
pub struct FileLog {
    state: Arc<Mutex<State>>,
}

struct State {
    directory: PathBuf,
    enabled: bool,
    rotated: bool,
    writer: Option<File>,
}

impl FileLog {
    fn new(directory: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                directory,
                enabled: false,
                rotated: false,
                writer: None,
            })),
        }
    }

    fn writer(&self) -> Writer {
        Writer(self.state.clone())
    }

    /// Apply the saved setting at start-up (the session log is still empty).
    pub fn configure(&self, enabled: bool) {
        if enabled {
            self.lock().enable("");
        }
    }

    /// Turn file writing on or off; when turning on, seed the file with the
    /// whole session so far so the file matches the Log tab.
    pub fn set_enabled(&self, enabled: bool, seed: &str) {
        let mut state = self.lock();
        if enabled && !state.enabled {
            state.enable(seed);
        } else if !enabled && state.enabled {
            state.disable();
        }
    }

    /// Whether any log file exists on disk.
    pub fn has_logs(&self) -> bool {
        let state = self.lock();
        fs::read_dir(&state.directory)
            .is_ok_and(|entries| entries.flatten().any(|entry| is_log(&entry.path())))
    }

    /// Delete every log file and stop writing.
    pub fn clear_all_and_disable(&self) {
        let mut state = self.lock();
        state.disable();
        state.remove_logs(None);
    }

    /// Delete every log file except the one being written now.
    pub fn clear_others(&self) {
        let state = self.lock();
        let current = state.directory.join(format!("{LOG_NAME}{LOG_SUFFIX}"));
        state.remove_logs(Some(&current));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().expect("log state is not poisoned")
    }
}

impl State {
    fn enable(&mut self, seed: &str) {
        match self.open() {
            Ok(mut file) => {
                if !seed.is_empty() {
                    let _ = file.write_all(seed.as_bytes());
                    let _ = file.write_all(b"\n");
                }
                self.writer = Some(file);
                self.enabled = true;
            }
            Err(error) => {
                tracing::warn!("cannot open the log file: {error}");
                self.writer = None;
            }
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
        self.writer = None;
    }

    /// The current run rotates once (its first time on); later it reuses and
    /// overwrites the same file instead of rotating again.
    fn open(&mut self) -> io::Result<File> {
        fs::create_dir_all(&self.directory)?;
        if !self.rotated {
            self.rotate();
            self.rotated = true;
        }
        File::create(self.directory.join(format!("{LOG_NAME}{LOG_SUFFIX}")))
    }

    fn rotate(&self) {
        let current = self.directory.join(format!("{LOG_NAME}{LOG_SUFFIX}"));
        if !current.exists() {
            return;
        }
        let numbered = |n: usize| self.directory.join(format!("{LOG_NAME}.{n}{LOG_SUFFIX}"));

        let _ = fs::remove_file(numbered(KEEP_FILES - 1));
        for n in (1..KEEP_FILES - 1).rev() {
            let from = numbered(n);
            if from.exists() {
                let _ = fs::rename(from, numbered(n + 1));
            }
        }
        let _ = fs::rename(&current, numbered(1));
    }

    fn remove_logs(&self, keep: Option<&Path>) {
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if is_log(&path) && keep != Some(path.as_path()) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

fn is_log(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(LOG_NAME) && name.ends_with(LOG_SUFFIX))
}

struct Writer(Arc<Mutex<State>>);

impl<'a> MakeWriter<'a> for Writer {
    type Writer = Sink;

    fn make_writer(&'a self) -> Self::Writer {
        Sink(self.0.clone())
    }
}

struct Sink(Arc<Mutex<State>>);

impl io::Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Ok(mut state) = self.0.lock()
            && state.enabled
            && let Some(writer) = state.writer.as_mut()
        {
            let _ = writer.write_all(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut state) = self.0.lock()
            && let Some(writer) = state.writer.as_mut()
        {
            let _ = writer.flush();
        }
        Ok(())
    }
}

struct Channel(mpsc::UnboundedSender<String>);

impl<'a> MakeWriter<'a> for Channel {
    type Writer = Record;

    fn make_writer(&'a self) -> Self::Writer {
        Record {
            sender: self.0.clone(),
            bytes: Vec::new(),
        }
    }
}

struct Record {
    sender: mpsc::UnboundedSender<String>,
    bytes: Vec<u8>,
}

impl io::Write for Record {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        let record = String::from_utf8_lossy(&self.bytes);
        let record = record.trim_end();
        if !record.is_empty() {
            let _ = self.sender.unbounded_send(record.to_owned());
        }
    }
}
