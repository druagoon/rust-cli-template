use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::include_template;

pub const DEFAULT_CONFIG: &str = include_template!("config/default.toml");
const CONFIG_DIRECTORY: &str = ".config";
const CONFIG_FILENAME: &str = "config.toml";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigPaths {
    pub local: PathBuf,
    pub user: PathBuf,
}

impl ConfigPaths {
    pub fn discover() -> anyhow::Result<Self> {
        let current_directory =
            std::env::current_dir().context("failed to resolve the current directory")?;
        let home_directory = PathBuf::from(shellexpand::tilde("~").as_ref());
        Ok(Self::from_roots(&current_directory, &home_directory))
    }

    pub fn from_roots(current_directory: &Path, home_directory: &Path) -> Self {
        let suffix = Path::new(CONFIG_DIRECTORY).join(clap::crate_name!()).join(CONFIG_FILENAME);
        Self { local: current_directory.join(&suffix), user: home_directory.join(suffix) }
    }

    pub fn ordered(&self) -> [&Path; 2] {
        [&self.local, &self.user]
    }
}

#[allow(dead_code)]
pub fn load<T>() -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let paths = ConfigPaths::discover()?;
    load_from_paths(&paths)
}

pub(crate) fn load_from_paths<T>(paths: &ConfigPaths) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    build(paths)?.try_deserialize::<T>().context("failed to deserialize the merged configuration")
}

pub(crate) fn merged_value(paths: &ConfigPaths) -> anyhow::Result<toml::Value> {
    load_from_paths(paths)
}

fn build(paths: &ConfigPaths) -> anyhow::Result<::config::Config> {
    ::config::Config::builder()
        .add_source(::config::File::from_str(DEFAULT_CONFIG, ::config::FileFormat::Toml))
        .add_source(::config::File::from(paths.user.clone()).required(false))
        .add_source(::config::File::from(paths.local.clone()).required(false))
        .build()
        .context("failed to merge configuration sources")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde::Deserialize;

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct TestConfig {
        app: AppConfig,
    }

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    struct AppConfig {
        name: String,
        retries: u8,
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> anyhow::Result<Self> {
            let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("cli-config-test-{}-{id}", std::process::id()));
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
    fn local_configuration_overrides_user_configuration() -> anyhow::Result<()> {
        let directory = TestDirectory::new()?;
        let current = directory.0.join("project");
        let home = directory.0.join("home");
        let paths = ConfigPaths::from_roots(&current, &home);

        fs::create_dir_all(paths.user.parent().context("missing user config parent")?)?;
        fs::create_dir_all(paths.local.parent().context("missing local config parent")?)?;
        fs::write(&paths.user, "[app]\nname = \"user\"\nretries = 2\n")?;
        fs::write(&paths.local, "[app]\nname = \"local\"\n")?;

        let config = load_from_paths::<TestConfig>(&paths)?;
        assert_eq!(config, TestConfig { app: AppConfig { name: "local".to_owned(), retries: 2 } });
        Ok(())
    }
}
