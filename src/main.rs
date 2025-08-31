use clap::{Args as _, Command, FromArgMatches as _};
use std::env;
use std::io;
use std::process::ExitCode;

use rip2::args::Commands;
use rip2::{args, completions, util};

fn main() -> ExitCode {
    let base_cmd = Command::new("rip");
    let cmd = args::Args::augment_args(base_cmd);
    let cli = args::Args::from_arg_matches(&cmd.get_matches()).unwrap();

    match &cli.command {
        Some(Commands::Completions { shell }) => {
            let result = completions::generate_shell_completions(shell, &mut io::stdout());
            if result.is_err() {
                eprintln!("{}", result.unwrap_err());
                return ExitCode::FAILURE;
            }
        }
        Some(Commands::Graveyard { seance }) => {
            let graveyard = rip2::get_graveyard(None);
            if *seance {
                let cwd = env::current_dir().expect("Failed to get current directory");
                let gravepath = util::join_absolute(
                    graveyard,
                    dunce::canonicalize(cwd).expect("Failed to get current directory"),
                );
                print!("{}", gravepath.display());
            } else {
                print!("{}", graveyard.display());
            }
        }
        None => {
            let mut stream = io::stdout();
            let mode = util::ProductionMode;

            ////////////////////////////////////////////////////////////
            // Main code ///////////////////////////////////////////////
            let result = rip2::run(cli, mode, &mut stream);
            ////////////////////////////////////////////////////////////

            if let Err(ref e) = result {
                println!("Exception: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}
