use clap::CommandFactory;
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;
use std::io::{self, Write};
use std::str::FromStr;

use crate::args;

pub fn generate_shell_completions(shell_s: &str, buf: &mut dyn Write) {
    if "nu" == shell_s || "nushell" == shell_s {
        let shell = Nushell;
        generate(shell, &mut args::Args::command(), "rip", buf);
    } else {
        let shell = Shell::from_str(shell_s).unwrap_or_else(|_| {
            eprintln!(
                "Invalid shell specification: {}. Available shells: bash, elvish, fish, powershell, zsh, nushell",
                shell_s
            );
            std::process::exit(1);
        });
        generate(shell, &mut args::Args::command(), "rip", buf);
    }
}
