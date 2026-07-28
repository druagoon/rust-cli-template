use crate::prelude::*;

/// Describe this command.
#[derive(clap::Parser, Debug)]
pub struct {{ command_type }} {}

impl CliCommand for {{ command_type }} {
    fn run(&self) -> CliResult {
        Ok(())
    }
}
