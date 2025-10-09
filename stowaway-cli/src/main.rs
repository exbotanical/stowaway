mod cli;

use stowaway_lib::{
    engine::ExecutionFlags,
    logging::{init_logging, parse_log_format, parse_log_level, LogConfig, LogFormat},
    Stowaway,
};
use tracing::{error, Level};

fn main() {
    let command = cli::parse_args();

    let log_config = match &command {
        cli::CliCommand::Stow {
            log_level,
            log_format,
            verbose,
            quiet,
            ..
        }
        | cli::CliCommand::Unstow {
            log_level,
            log_format,
            verbose,
            quiet,
            ..
        } => create_log_config(log_level, log_format, *verbose, *quiet),
    };

    if let Err(e) = init_logging(log_config) {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    let stowaway = Stowaway::new();

    let result = match command {
        cli::CliCommand::Stow {
            source,
            target,
            dry_run,
            ..
        } => stowaway.run(
            &source,
            &target,
            ExecutionFlags {
                is_dry_run: dry_run,
            },
        ),
        cli::CliCommand::Unstow { dry_run, .. } => stowaway.unstow(dry_run),
    };

    if let Err(e) = result {
        error!(error = %e, "Operation failed");
        std::process::exit(1);
    }
}

fn create_log_config(
    log_level: &Option<String>,
    log_format: &Option<String>,
    verbose: bool,
    quiet: bool,
) -> LogConfig {
    let level = if quiet {
        Level::ERROR
    } else if verbose {
        Level::DEBUG
    } else if let Some(level_str) = log_level {
        match parse_log_level(level_str) {
            Ok(level) => level,
            Err(e) => {
                eprintln!("Invalid log level '{}': {}", level_str, e);
                std::process::exit(1);
            }
        }
    } else {
        Level::INFO
    };

    let format = if let Some(format_str) = log_format {
        match parse_log_format(format_str) {
            Ok(format) => format,
            Err(e) => {
                eprintln!("Invalid log format '{}': {}", format_str, e);
                std::process::exit(1);
            }
        }
    } else {
        LogFormat::Human
    };

    LogConfig::new(level).with_format(format)
}
