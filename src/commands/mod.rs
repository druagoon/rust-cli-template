mod completion;
#[cfg(feature = "config")]
mod config;
#[cfg(feature = "dev-tools")]
mod new;

// <generated-command-modules>
// </generated-command-modules>

use crate::prelude::*;

#[derive(clap::Subcommand, cli_derive::CliCommand, Debug)]
pub enum Command {
    Completion(completion::CompletionCmd),

    #[cfg(feature = "config")]
    #[command(subcommand)]
    Config(config::ConfigCmd),

    #[cfg(feature = "dev-tools")]
    New(new::NewCmd),
    // <generated-command-variants>
    // </generated-command-variants>
}
