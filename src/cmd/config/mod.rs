mod generate;
mod list;
mod show;

use crate::prelude::*;

/// Manage local and user configuration.
#[derive(clap::Subcommand, {{ crate_name }}_derive::CliCommand, Debug)]
pub enum ConfigCmd {
    Generate(generate::ConfigGenerateCmd),
    List(list::ConfigListCmd),
    Show(show::ConfigShowCmd),
}
