use std::io;

use clap_complete::{Generator, Shell};

use crate::prelude::*;

/// Generate a shell completion script.
#[derive(clap::Parser, Debug)]
pub struct CompletionCmd {
    #[arg(value_enum)]
    shell: Shell,
}

impl CliCommand for CompletionCmd {
    fn run(&self) -> CliResult {
        let command = crate::cli::Cli::build();
        self.shell.generate(&command, &mut io::stdout());
        Ok(())
    }
}
