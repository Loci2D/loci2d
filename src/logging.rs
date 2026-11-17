use chrono::Local;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// Severity levels for structured Loci2D server logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Off => "OFF  ",
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => LogLevel::Off,
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            _ => LogLevel::Trace,
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "silent" => Ok(LogLevel::Off),
            "error" | "err" => Ok(LogLevel::Error),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            other => Err(format!(
                "Invalid log level '{}' (expected: off, error, warn, info, debug, trace)",
                other
            )),
        }
    }
}

static MAX_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);
static LOG_FILE: OnceLock<Mutex<Option<BufWriter<File>>>> = OnceLock::new();

/// Sets the global active maximum log level.
pub fn set_level(level: LogLevel) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Returns the currently active maximum log level.
#[inline]
pub fn level() -> LogLevel {
    LogLevel::from_u8(MAX_LEVEL.load(Ordering::Relaxed))
}

/// Returns `true` if a message at `msg_level` should be emitted under the active threshold.
#[inline]
pub fn enabled(msg_level: LogLevel) -> bool {
    msg_level != LogLevel::Off && (msg_level as u8) <= MAX_LEVEL.load(Ordering::Relaxed)
}

/// Initializes log level from `LOCI_LOG` or `RUST_LOG` environment variables (defaults to `Info`).
pub fn init_from_env() {
    let raw = std::env::var("LOCI_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .ok();
    if let Some(val) = raw
        && let Ok(parsed) = LogLevel::parse(&val)
    {
        set_level(parsed);
    }
}

/// Enables optional tee logging to a file on disk (appending or creating cleanly).
pub fn set_log_file<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    if let Some(parent) = path.as_ref().parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let cell = LOG_FILE.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(BufWriter::new(file));
    Ok(())
}

/// Flushes any buffered log file output to disk.
pub fn flush() {
    if let Some(cell) = LOG_FILE.get() {
        let mut guard = cell.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *guard {
            let _ = writer.flush();
        }
    }
}

#[doc(hidden)]
pub fn emit(msg_level: LogLevel, subsystem: &str, tick: Option<u64>, args: std::fmt::Arguments<'_>) {
    if !enabled(msg_level) {
        return;
    }

    let ts = Local::now().format("%H:%M:%S%.3f");
    let line = match tick {
        Some(t) => format!(
            "[{}][{}][{}][tick={:05}] {}",
            ts,
            msg_level.as_str(),
            subsystem,
            t,
            args
        ),
        None => format!("[{}][{}][{}] {}", ts, msg_level.as_str(), subsystem, args),
    };

    if matches!(msg_level, LogLevel::Error | LogLevel::Warn) {
        eprintln!("{}", line);
    } else {
        println!("{}", line);
    }

    if let Some(cell) = LOG_FILE.get() {
        let mut guard = cell.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *guard {
            let _ = writeln!(writer, "{}", line);
            if matches!(msg_level, LogLevel::Error | LogLevel::Warn | LogLevel::Info) {
                let _ = writer.flush();
            }
        }
    }
}

#[macro_export]
macro_rules! log_error {
    ($subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Error) {
            $crate::logging::emit($crate::logging::LogLevel::Error, $subsystem, None, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Warn) {
            $crate::logging::emit($crate::logging::LogLevel::Warn, $subsystem, None, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Info) {
            $crate::logging::emit($crate::logging::LogLevel::Info, $subsystem, None, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    (tick = $tick:expr, $subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Debug) {
            $crate::logging::emit($crate::logging::LogLevel::Debug, $subsystem, Some($tick), format_args!($($arg)+));
        }
    };
    ($subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Debug) {
            $crate::logging::emit($crate::logging::LogLevel::Debug, $subsystem, None, format_args!($($arg)+));
        }
    };
}

#[macro_export]
macro_rules! log_trace {
    (tick = $tick:expr, $subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Trace) {
            $crate::logging::emit($crate::logging::LogLevel::Trace, $subsystem, Some($tick), format_args!($($arg)+));
        }
    };
    ($subsystem:expr, $($arg:tt)+) => {
        if $crate::logging::enabled($crate::logging::LogLevel::Trace) {
            $crate::logging::emit($crate::logging::LogLevel::Trace, $subsystem, None, format_args!($($arg)+));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing_and_ordering() {
        assert_eq!(LogLevel::parse("error").unwrap(), LogLevel::Error);
        assert_eq!(LogLevel::parse("WARN").unwrap(), LogLevel::Warn);
        assert_eq!(LogLevel::parse("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::parse("Debug").unwrap(), LogLevel::Debug);
        assert_eq!(LogLevel::parse("trace").unwrap(), LogLevel::Trace);
        assert_eq!(LogLevel::parse("off").unwrap(), LogLevel::Off);
        assert!(LogLevel::parse("invalid").is_err());

        assert!(LogLevel::Off < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }
}
