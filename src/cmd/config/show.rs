use clap::ValueEnum;

use crate::conf::{ConfigPaths, merged_value};
use crate::prelude::*;

/// Show the merged configuration.
#[derive(clap::Parser, Debug)]
pub struct ConfigShowCmd {
    /// Output format.
    #[arg(long, value_enum, default_value = "toml")]
    format: OutputFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Toml,
    Json,
    Yaml,
}

impl ConfigShowCmd {
    fn render(&self, value: &toml::Value) -> anyhow::Result<String> {
        match self.format {
            OutputFormat::Toml => Ok(toml::to_string_pretty(value)?),
            OutputFormat::Json => Ok(serde_json::to_string_pretty(value)?),
            OutputFormat::Yaml => Ok(serde_yaml::to_string(value)?),
        }
    }
}

impl CliCommand for ConfigShowCmd {
    fn run(&self) -> CliResult {
        let paths = ConfigPaths::discover()?;
        let value = merged_value(&paths)?;
        println!("{}", self.render(&value)?.trim_end());
        Ok(())
    }
}
