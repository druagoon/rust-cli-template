use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::conf::{ConfigPaths, DEFAULT_CONFIG};
use crate::prelude::*;

/// Generate a configuration file.
#[derive(clap::Parser, Debug)]
pub struct ConfigGenerateCmd {
    /// Generate the project-local configuration.
    #[arg(long)]
    local: bool,

    /// Back up and overwrite an existing configuration.
    #[arg(long)]
    force: bool,
}

impl ConfigGenerateCmd {
    fn destination(&self, paths: &ConfigPaths) -> PathBuf {
        if self.local { paths.local.clone() } else { paths.user.clone() }
    }

    fn generate(&self, destination: &Path) -> anyhow::Result<()> {
        if destination.exists() && !self.force {
            anyhow::bail!(
                "configuration already exists: {}; use --force to overwrite it",
                destination.display()
            );
        }

        if destination.exists() {
            let backup = destination.with_extension("bak");
            fs::copy(destination, &backup).with_context(|| {
                format!("failed to back up {} to {}", destination.display(), backup.display())
            })?;
            println!("backed up configuration to {}", backup.display());
        }

        let parent =
            destination.parent().context("configuration destination has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        fs::write(destination, DEFAULT_CONFIG)
            .with_context(|| format!("failed to write {}", destination.display()))?;
        println!("generated configuration at {}", destination.display());
        Ok(())
    }
}

impl CliCommand for ConfigGenerateCmd {
    fn run(&self) -> CliResult {
        let paths = ConfigPaths::discover()?;
        self.generate(&self.destination(&paths))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> anyhow::Result<Self> {
            let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cli-generate-test-{}-{id}", std::process::id()));
            fs::create_dir_all(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn force_generation_creates_a_backup() -> anyhow::Result<()> {
        let directory = TestDirectory::new()?;
        let destination = directory.0.join("config.toml");
        fs::write(&destination, "old = true\n")?;

        ConfigGenerateCmd { local: false, force: true }.generate(&destination)?;

        assert_eq!(fs::read_to_string(&destination)?, DEFAULT_CONFIG);
        assert_eq!(fs::read_to_string(destination.with_extension("bak"))?, "old = true\n");
        Ok(())
    }
}
