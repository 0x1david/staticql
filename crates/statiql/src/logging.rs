use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

static GLOBAL_LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);
static LOGGER_INITIALIZED: OnceLock<()> = OnceLock::new();
static HAS_ERROR_OCCURRED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Always = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Always => "ALWAYS",
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }

    fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Always => "\x1b[1;37m", // Bold White
            LogLevel::Error => "\x1b[31m",    // Red
            LogLevel::Warn => "\x1b[33m",     // Yellow
            LogLevel::Info => "\x1b[32m",     // Green
            LogLevel::Debug => "\x1b[36m",    // Cyan
        }
    }
}

pub struct Logger;
impl Logger {
    pub fn init(level: LogLevel) {
        LOGGER_INITIALIZED.get_or_init(|| {
            GLOBAL_LOG_LEVEL.store(level as u8, Ordering::Relaxed);
        });
    }

    pub fn should_log(level: LogLevel) -> bool {
        let current_level = GLOBAL_LOG_LEVEL.load(Ordering::Relaxed);
        (level as u8) <= current_level
    }

    pub fn log_message(level: LogLevel, message: &str, file: &str, line: u32) {
        if Self::should_log(level) {
            println!(
                "{}[{}] {}:{} - {}\x1b[0m",
                level.color_code(),
                level.as_str(),
                file,
                line,
                message
            );
        }

        if matches!(level, LogLevel::Error) {
            HAS_ERROR_OCCURRED.store(true, Ordering::Relaxed);
        }
    }

    pub fn has_error_occurred() -> bool {
        HAS_ERROR_OCCURRED.load(Ordering::Relaxed)
    }

    pub fn exit_code() -> i32 {
        if Self::has_error_occurred() { 1 } else { 0 }
    }
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::Logger::log_message(
            $level,
            &format!($($arg)*),
            file!(),
            line!()
        )
    };
}

#[macro_export]
macro_rules! always_log {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Always, $($arg)*)
    };
}

#[macro_export]
macro_rules! exception {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Error, $($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Error, $($arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Warn, $($arg)*)
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Info, $($arg)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log!($crate::LogLevel::Debug, $($arg)*)
    };
}
