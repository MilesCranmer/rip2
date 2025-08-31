use clap::CommandFactory;
use clap_complete::{generate, Shell};
use clap_complete_nushell::Nushell;
use std::io::{Error, ErrorKind, Result, Write};
use std::str::FromStr;

use crate::args;

pub fn generate_shell_completions(shell_s: &str, buf: &mut dyn Write) -> Result<()> {
    if "nu" == shell_s || "nushell" == shell_s {
        let shell = Nushell;
        generate(shell, &mut args::Args::command(), "rip", buf);
    } else {
        let tryshell = Shell::from_str(shell_s);
        if tryshell.is_err() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Invalid shell specification: {shell_s}. Available shells: bash, elvish, fish, powershell, zsh, nushell"
                )
            ));
        }
        generate(tryshell.unwrap(), &mut args::Args::command(), "rip", buf);
    }
    Ok(())
}
