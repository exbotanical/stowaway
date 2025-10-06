//! Logging configuration and utilities for Stowaway

use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

#[macro_export]
macro_rules! log_dryrun {
    ($($arg:tt)*) => {
        info!("[DRY RUN] {}", format_args!($($arg)*));
    };
}

/// Log output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable format for CLI usage
    Human,
    /// JSON format for programmatic use-cases
    Json,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Human
    }
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level filter
    pub level: Level,
    /// Output format
    pub format: LogFormat,
    /// Whether to show target module names
    pub show_target: bool,
    /// Whether to show timestamps
    pub show_time: bool,
    /// Whether to show thread names
    pub show_thread_names: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            format: LogFormat::Human,
            show_target: false,
            show_time: false,
            show_thread_names: false,
        }
    }
}

impl LogConfig {
    /// Creates a new log configuration with the specified level
    pub fn new(level: Level) -> Self {
        Self {
            level,
            ..Default::default()
        }
    }

    /// Sets the output format
    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    /// Enables target module names in output
    pub fn with_target(mut self) -> Self {
        self.show_target = true;
        self
    }

    /// Enables timestamps in output
    pub fn with_time(mut self) -> Self {
        self.show_time = true;
        self
    }

    /// Enables thread names in output
    pub fn with_thread_names(mut self) -> Self {
        self.show_thread_names = true;
        self
    }

    /// Creates a quiet configuration (errors only)
    pub fn quiet() -> Self {
        Self::new(Level::ERROR)
    }

    /// Creates a verbose configuration (debug level)
    pub fn verbose() -> Self {
        Self::new(Level::DEBUG).with_target()
    }

    /// Creates a trace configuration (maximum verbosity)
    pub fn trace() -> Self {
        Self::new(Level::TRACE).with_target().with_time()
    }
}

/// Initializes the global tracing subscriber with the given configuration
pub fn init_logging(config: LogConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(config.level.into())
        .from_env_lossy()
        .add_directive(format!("stowaway={}", config.level).parse()?)
        .add_directive(format!("stowaway_lib={}", config.level).parse()?);

    match config.format {
        LogFormat::Human => {
            let fmt_layer = fmt::layer()
                .with_target(config.show_target)
                .with_thread_names(config.show_thread_names)
                .with_span_events(FmtSpan::NONE)
                .compact();

            let fmt_layer = if config.show_time {
                fmt_layer.boxed()
            } else {
                fmt_layer.without_time().boxed()
            };

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .init();
        }
        LogFormat::Json => {
            let fmt_layer = fmt::layer()
                .json()
                .with_target(config.show_target)
                .with_thread_names(config.show_thread_names)
                .with_span_events(FmtSpan::NONE);

            let fmt_layer = if config.show_time {
                fmt_layer.boxed()
            } else {
                fmt_layer.without_time().boxed()
            };

            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt_layer)
                .init();
        }
    }

    Ok(())
}

/// Parses log level from string
pub fn parse_log_level(level: &str) -> Result<Level, String> {
    match level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" | "warning" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(format!("Invalid log level: {}", level)),
    }
}

/// Parses log format from string
pub fn parse_log_format(format: &str) -> Result<LogFormat, String> {
    match format.to_lowercase().as_str() {
        "human" | "text" => Ok(LogFormat::Human),
        "json" => Ok(LogFormat::Json),
        _ => Err(format!("Invalid log format: {}", format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, Level::INFO);
        assert_eq!(config.format, LogFormat::Human);
        assert!(!config.show_target);
        assert!(!config.show_time);
        assert!(!config.show_thread_names);
    }

    #[test]
    fn test_log_config_builder() {
        let config = LogConfig::new(Level::DEBUG)
            .with_format(LogFormat::Json)
            .with_target()
            .with_time()
            .with_thread_names();

        assert_eq!(config.level, Level::DEBUG);
        assert_eq!(config.format, LogFormat::Json);
        assert!(config.show_target);
        assert!(config.show_time);
        assert!(config.show_thread_names);
    }

    #[test]
    fn test_log_config_presets() {
        let quiet = LogConfig::quiet();
        assert_eq!(quiet.level, Level::ERROR);

        let verbose = LogConfig::verbose();
        assert_eq!(verbose.level, Level::DEBUG);
        assert!(verbose.show_target);

        let trace = LogConfig::trace();
        assert_eq!(trace.level, Level::TRACE);
        assert!(trace.show_target);
        assert!(trace.show_time);
    }

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("ERROR").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("warn").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("warning").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);

        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_parse_log_format() {
        assert_eq!(parse_log_format("human").unwrap(), LogFormat::Human);
        assert_eq!(parse_log_format("text").unwrap(), LogFormat::Human);
        assert_eq!(parse_log_format("json").unwrap(), LogFormat::Json);
        assert_eq!(parse_log_format("JSON").unwrap(), LogFormat::Json);

        assert!(parse_log_format("invalid").is_err());
    }
}
