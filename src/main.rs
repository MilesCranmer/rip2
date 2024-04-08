use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;

use rip::{args, util};
use std::io::stdout;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = args::Args::parse();

    if let Some(shell) = cli.completions.as_deref() {
        if ["nu", "nushell"].contains(&shell) {
            let shell = Nushell;
            generate(shell, &mut args::Args::command(), "rip", &mut stdout());
        } else {
            let shell = Shell::from_str(shell, true).unwrap_or_else(|_| {
                eprintln!("Invalid shell specification: {}", shell);
                std::process::exit(1);
            });
            generate(shell, &mut args::Args::command(), "rip", &mut stdout());
        }
        return ExitCode::SUCCESS;
    }

    if let Err(ref e) = rip::run(cli, util::ProductionMode) {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
