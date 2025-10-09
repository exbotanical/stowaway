use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;

#[derive(Debug)]
pub enum CliCommand {
    Stow {
        source: PathBuf,
        target: PathBuf,
        dry_run: bool,
        log_level: Option<String>,
        log_format: Option<String>,
        verbose: bool,
        quiet: bool,
    },
    Unstow {
        dry_run: bool,
        log_level: Option<String>,
        log_format: Option<String>,
        verbose: bool,
        quiet: bool,
    },
}

pub fn build_cli() -> Command {
    let logging_args = vec![
        Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Enable verbose logging (debug level)")
            .action(ArgAction::SetTrue),
        Arg::new("quiet")
            .short('q')
            .long("quiet")
            .help("Enable quiet mode (errors only)")
            .action(ArgAction::SetTrue),
        Arg::new("log-level")
            .long("log-level")
            .value_name("LEVEL")
            .help("Set log level (error, warn, info, debug, trace)")
            .conflicts_with_all(["verbose", "quiet"]),
        Arg::new("log-format")
            .long("log-format")
            .value_name("FORMAT")
            .help("Set log format (human, json)")
            .default_value("human"),
    ];

    Command::new("stowaway")
        .version("0.1.0")
        .about("A modern GNU Stow replacement with variable interpolation")
        .subcommand_required(true)
        .subcommand(
            Command::new("stow")
                .about("Stow dotfiles from source to target directory")
                .arg(
                    Arg::new("source")
                        .short('s')
                        .long("source")
                        .value_name("DIR")
                        .help("Source directory containing dotfiles")
                        .required(true),
                )
                .arg(
                    Arg::new("target")
                        .short('t')
                        .long("target")
                        .value_name("DIR")
                        .help("Target directory for symlinks")
                        .required(true),
                )
                .arg(
                    Arg::new("dry-run")
                        .short('n')
                        .long("dry-run")
                        .help("Show what would be done without making changes")
                        .action(ArgAction::SetTrue),
                )
                .args(logging_args.clone()),
        )
        .subcommand(
            Command::new("unstow")
                .about("Remove all symlinks from the current stow version")
                .arg(
                    Arg::new("dry-run")
                        .short('n')
                        .long("dry-run")
                        .help("Show what would be removed without making changes")
                        .action(ArgAction::SetTrue),
                )
                .args(logging_args.clone()),
        )
}

pub fn parse_args() -> CliCommand {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("stow", sub_matches)) => CliCommand::Stow {
            source: PathBuf::from(sub_matches.get_one::<String>("source").unwrap()),
            target: PathBuf::from(sub_matches.get_one::<String>("target").unwrap()),
            dry_run: sub_matches.get_flag("dry-run"),
            log_level: sub_matches.get_one::<String>("log-level").cloned(),
            log_format: sub_matches.get_one::<String>("log-format").cloned(),
            verbose: sub_matches.get_flag("verbose"),
            quiet: sub_matches.get_flag("quiet"),
        },

        Some(("unstow", sub_matches)) => CliCommand::Unstow {
            dry_run: sub_matches.get_flag("dry-run"),
            log_level: sub_matches.get_one::<String>("log-level").cloned(),
            log_format: sub_matches.get_one::<String>("log-format").cloned(),
            verbose: sub_matches.get_flag("verbose"),
            quiet: sub_matches.get_flag("quiet"),
        },

        _ => unreachable!("Subcommand is required"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stow_command_basic() {
        let cmd = build_cli();
        let matches = cmd
            .try_get_matches_from(vec![
                "stowaway",
                "stow",
                "--source",
                "/home/user/dotfiles",
                "--target",
                "/home/user",
            ])
            .unwrap();

        let result = match matches.subcommand() {
            Some(("stow", sub_matches)) => CliCommand::Stow {
                source: PathBuf::from(sub_matches.get_one::<String>("source").unwrap()),
                target: PathBuf::from(sub_matches.get_one::<String>("target").unwrap()),
                dry_run: sub_matches.get_flag("dry-run"),
                log_level: sub_matches.get_one::<String>("log-level").cloned(),
                log_format: sub_matches.get_one::<String>("log-format").cloned(),
                verbose: sub_matches.get_flag("verbose"),
                quiet: sub_matches.get_flag("quiet"),
            },
            _ => panic!("Expected stow subcommand"),
        };

        match result {
            CliCommand::Stow {
                source,
                target,
                dry_run,
                ..
            } => {
                assert_eq!(source, PathBuf::from("/home/user/dotfiles"));
                assert_eq!(target, PathBuf::from("/home/user"));
                assert!(!dry_run);
            }
            _ => panic!("Expected Stow command"),
        }
    }

    #[test]
    fn test_parse_stow_command_with_flags() {
        let cmd = build_cli();
        let matches = cmd
            .try_get_matches_from(vec![
                "stowaway",
                "stow",
                "--source",
                "/home/user/dotfiles",
                "--target",
                "/home/user",
                "--dry-run",
            ])
            .unwrap();

        let result = match matches.subcommand() {
            Some(("stow", sub_matches)) => CliCommand::Stow {
                source: PathBuf::from(sub_matches.get_one::<String>("source").unwrap()),
                target: PathBuf::from(sub_matches.get_one::<String>("target").unwrap()),
                dry_run: sub_matches.get_flag("dry-run"),
                log_level: sub_matches.get_one::<String>("log-level").cloned(),
                log_format: sub_matches.get_one::<String>("log-format").cloned(),
                verbose: sub_matches.get_flag("verbose"),
                quiet: sub_matches.get_flag("quiet"),
            },
            _ => panic!("Expected stow subcommand"),
        };

        match result {
            CliCommand::Stow {
                source,
                target,
                dry_run,
                ..
            } => {
                assert_eq!(source, PathBuf::from("/home/user/dotfiles"));
                assert_eq!(target, PathBuf::from("/home/user"));
                assert!(dry_run);
            }
            _ => panic!("Expected Stow command"),
        }
    }

    #[test]
    fn test_parse_stow_command_short_flags() {
        let cmd = build_cli();
        let matches = cmd
            .try_get_matches_from(vec![
                "stowaway",
                "stow",
                "-s",
                "/home/user/dotfiles",
                "-t",
                "/home/user",
                "-n",
                "-f",
            ])
            .unwrap();

        let result = match matches.subcommand() {
            Some(("stow", sub_matches)) => CliCommand::Stow {
                source: PathBuf::from(sub_matches.get_one::<String>("source").unwrap()),
                target: PathBuf::from(sub_matches.get_one::<String>("target").unwrap()),
                dry_run: sub_matches.get_flag("dry-run"),
                log_level: sub_matches.get_one::<String>("log-level").cloned(),
                log_format: sub_matches.get_one::<String>("log-format").cloned(),
                verbose: sub_matches.get_flag("verbose"),
                quiet: sub_matches.get_flag("quiet"),
            },
            _ => panic!("Expected stow subcommand"),
        };

        match result {
            CliCommand::Stow {
                source,
                target,
                dry_run,
                ..
            } => {
                assert_eq!(source, PathBuf::from("/home/user/dotfiles"));
                assert_eq!(target, PathBuf::from("/home/user"));
                assert!(dry_run);
            }
            _ => panic!("Expected Stow command"),
        }
    }

    #[test]
    fn test_stow_command_missing_required_args() {
        let cmd = build_cli();

        // Missing source
        let result =
            cmd.clone()
                .try_get_matches_from(vec!["stowaway", "stow", "--target", "/home/user"]);
        assert!(result.is_err());

        // Missing target
        let result = cmd.clone().try_get_matches_from(vec![
            "stowaway",
            "stow",
            "--source",
            "/home/user/dotfiles",
        ]);
        assert!(result.is_err());

        // Missing both
        let result = cmd.clone().try_get_matches_from(vec!["stowaway", "stow"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_subcommand_fails() {
        let cmd = build_cli();
        let result = cmd.try_get_matches_from(vec!["stowaway"]);
        assert!(result.is_err());
    }
}
