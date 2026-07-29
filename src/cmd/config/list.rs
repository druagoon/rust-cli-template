use std::fs;

use anyhow::Context;

use crate::conf::ConfigPaths;
use crate::prelude::*;

/// List configuration files.
#[derive(clap::Parser, Debug)]
pub struct ConfigListCmd {
    /// Only list files that exist.
    #[arg(long)]
    exists: bool,

    /// Print the content of each existing file.
    #[arg(long)]
    with_content: bool,
}

impl CliCommand for ConfigListCmd {
    fn run(&self) -> CliResult {
        let paths = ConfigPaths::discover()?;
        for path in paths.ordered() {
            if self.exists && !path.exists() {
                continue;
            }

            println!("{}", path.display());
            if self.with_content && path.exists() {
                let content = fs::read_to_string(path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                println!("{}", content.trim_end());
            }
        }
        Ok(())
    }
}
