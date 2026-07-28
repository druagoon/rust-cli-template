use std::path::PathBuf;

use serde::Deserialize;

use crate::utils::path::expand_tilde;

/// Deserializes a `PathBuf` from a string, expanding any tilde (`~`) to the
/// user's home directory.
///
/// # Arguments
///
/// * `deserializer` - The deserializer to use for deserializing the string.
///
/// # Returns
///
/// * `Result<Option<PathBuf>, D::Error>` - Returns an `Ok(Some(PathBuf))` if
///   the string is not empty, with the tilde expanded to the user's home
///   directory. Returns `Ok(None)` if the string is empty.
///
/// # Errors
///
/// * Returns an error if the string cannot be deserialized.
///
/// # Example
///
/// ```rust
/// use std::path::PathBuf;
///
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Config {
///     #[serde(deserialize_with = "deserialize_path")]
///     path: Option<PathBuf>,
/// }
/// ```
#[allow(dead_code)]
pub fn deserialize_path<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut path = None;
    let value = String::deserialize(deserializer)?;
    if !value.is_empty() {
        path = Some(expand_tilde(&value));
    }
    Ok(path)
}
