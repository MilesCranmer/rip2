use clap::Parser;
use std::io;
use std::process::ExitCode;

use rip2::{args, util, completions};

fn main() -> ExitCode {
    let cli = args::Args::parse();

    match &cli.command {
        Some(args::Commands::Completions { shell }) => {
            completions::generate_shell_completions(shell);
            return ExitCode::SUCCESS;
        }
        None => {}
    }

    let mode = util::ProductionMode;
    let mut stream = io::stdout();

    if let Err(ref e) = rip2::run(cli, mode, &mut stream) {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
