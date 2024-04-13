use clap::{Parser, Subcommand};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// File or directory to remove
    pub targets: Vec<PathBuf>,

    /// Directory where deleted files rest
    #[arg(long)]
    pub graveyard: Option<PathBuf>,

    /// Permanently deletes the graveyard
    #[arg(short, long)]
    pub decompose: bool,

    /// Prints files that were deleted
    /// in the current working directory
    #[arg(short, long)]
    pub seance: bool,

    /// Restore the specified
    /// files or the last file
    /// if none are specified
    #[arg(short, long, num_args = 0)]
    pub unbury: Option<Vec<PathBuf>>,

    /// Print some info about TARGET before
    /// burying
    #[arg(short, long)]
    pub inspect: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate shell completions file
    /// for the specified shell
    Completions {
        /// The shell to generate completions for
        #[arg(value_name = "SHELL")]
        shell: String,
    },
}

struct IsDefault {
    graveyard: bool,
    decompose: bool,
    seance: bool,
    unbury: bool,
    inspect: bool,
    completions: bool,
}

impl IsDefault {
    fn new(cli: &Args) -> IsDefault {
        let defaults = Args::default();
        IsDefault {
            graveyard: cli.graveyard == defaults.graveyard,
            decompose: cli.decompose == defaults.decompose,
            seance: cli.seance == defaults.seance,
            unbury: cli.unbury == defaults.unbury,
            inspect: cli.inspect == defaults.inspect,
            completions: cli.command.is_none(),
        }
    }
}

#[allow(clippy::nonminimal_bool)]
pub fn validate_args(cli: &Args) -> Result<(), Error> {
    let defaults = IsDefault::new(cli);

    // [completions] can only be used by itself
    if !defaults.completions
        && !(defaults.graveyard
            && defaults.decompose
            && defaults.seance
            && defaults.unbury
            && defaults.inspect)
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "--completions can only be used by itself",
        ));
    }
    if !defaults.decompose && !(defaults.seance && defaults.unbury && defaults.inspect) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "-d,--decompose can only be used with --graveyard",
        ));
    }

    Ok(())
}
