//! Native startup and GPU diagnostics. The Windows release build has no console,
//! so a file is needed to diagnose adapter/driver selection and fatal startup errors.
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::sync::{Mutex, OnceLock};

static LOGGER: StartupLogger = StartupLogger;
static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

struct StartupLogger;

impl log::Log for StartupLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |time| time.as_secs());
        let line = format!("{timestamp} [{}] {}: {}", record.level(), record.target(), record.args());
        if let Some(file) = LOG_FILE.get()
            && let Ok(mut file) = file.lock()
        {
            let _ = writeln!(file, "{line}");
            return;
        }
        eprintln!("{line}");
    }

    fn flush(&self) {
        if let Some(file) = LOG_FILE.get()
            && let Ok(mut file) = file.lock()
        {
            let _ = file.flush();
        }
    }
}

pub fn init() {
    if let Some(folder) = crate::platform::data_dir().map(|path| path.join("logs")) {
        if std::fs::create_dir_all(&folder).is_ok() {
            let path = folder.join("arclet-startup.log");
            // Bound log growth without deleting the last session's useful diagnostics.
            if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 4 * 1024 * 1024) {
                let previous = folder.join("arclet-startup.previous.log");
                let _ = std::fs::remove_file(&previous);
                let _ = std::fs::rename(&path, previous);
            }
            if let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) {
                let _ = LOG_FILE.set(Mutex::new(file));
            }
        }
    }

    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    // The logger receives wgpu/eframe errors too. A renderer panic must leave
    // a record even when the Windows subsystem has no attached terminal.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("Arclet panic: {info}\n{}", std::backtrace::Backtrace::force_capture());
        log::logger().flush();
        previous_hook(info);
    }));
    log::info!("Diagnostics enabled; WGPU_BACKEND={}", std::env::var("WGPU_BACKEND").unwrap_or_else(|_| "auto".into()));
}
