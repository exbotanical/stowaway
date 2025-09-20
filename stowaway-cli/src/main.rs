mod cli;

use stowaway_lib::Stowaway;

fn main() {
    let command = cli::parse_args();

    let stowaway = Stowaway::new();

    let result = match command {
        cli::CliCommand::Stow { source, target, dry_run, force: _ } => {
            stowaway.run(&source, &target, dry_run)
        }
        cli::CliCommand::Rollback { hash } => {
            stowaway.rollback(&hash)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
