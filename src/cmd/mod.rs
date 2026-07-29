mod completion;
#[cfg(feature = "config")]
mod config;
#[cfg(feature = "dev-tools")]
mod new;

use crate::prelude::*;

#[derive(clap::Subcommand, {{ crate_name }}_derive::CliCommand, Debug)]
pub enum Command {
    Completion(completion::CompletionCmd),

    #[cfg(feature = "config")]
    #[command(subcommand)]
    Config(config::ConfigCmd),

    #[cfg(feature = "dev-tools")]
    New(new::NewCmd),
}
