/// Logging setup for MCP applications
use log::LevelFilter;

/// Initialize logging for MCP applications
pub fn init_logging(level: LogLevel) {
    let level_filter = match level {
        LogLevel::Debug => LevelFilter::Debug,
        LogLevel::Info => LevelFilter::Info,
        LogLevel::Warn => LevelFilter::Warn,
        LogLevel::Error => LevelFilter::Error,
    };

    let _ = env_logger::Builder::from_default_env()
        .filter_level(level_filter)
        .try_init();
}

/// Log levels
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Macro for debug logging
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*);
    };
}

/// Macro for info logging
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log::info!($($arg)*);
    };
}

/// Macro for warning logging
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log::warn!($($arg)*);
    };
}

/// Macro for error logging
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log::error!($($arg)*);
    };
}

/// Logger utility struct
pub struct Logger;

impl Logger {
    /// Log a debug message
    pub fn debug(msg: &str) {
        log::debug!("{}", msg);
    }

    /// Log an info message
    pub fn info(msg: &str) {
        log::info!("{}", msg);
    }

    /// Log a warning message
    pub fn warn(msg: &str) {
        log::warn!("{}", msg);
    }

    /// Log an error message
    pub fn error(msg: &str) {
        log::error!("{}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_creation() {
        let _ = LogLevel::Debug;
        let _ = LogLevel::Info;
        let _ = LogLevel::Warn;
        let _ = LogLevel::Error;
    }

    #[test]
    fn test_logger_creation() {
        let _ = Logger;
    }
}
