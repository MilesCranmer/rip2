use clap::Parser;
use std::io;
use std::process::ExitCode;

use rip2::{args, completions, util};

fn main() -> ExitCode {
    let cli = args::Args::parse();
    let mut stream = io::stdout();
    let mode = util::ProductionMode;

    match &cli.command {
        Some(args::Commands::Completions { shell }) => {
            let result = completions::generate_shell_completions(shell, &mut io::stdout());
            if result.is_err() {
                eprintln!("{}", result.unwrap_err());
                return ExitCode::FAILURE;
            }
            return ExitCode::SUCCESS;
        }
        None => {}
    }

    if let Err(ref e) = rip2::run(cli, mode, &mut stream) {
        println!("Exception: {}", e);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
