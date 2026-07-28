use std::path::PathBuf;

/// Expands a tilde (`~`) in the given string to the user's home directory.
///
/// # Arguments
///
/// * `s` - A string slice that may contain a tilde (`~`) representing the
///   user's home directory.
///
/// # Returns
///
/// A `PathBuf` with the tilde expanded to the full path of the user's home
/// directory.
///
/// # Examples
///
/// ```
/// let path = expand_tilde("~/.config");
/// assert!(path.starts_with(std::env::home_dir().unwrap()));
/// ```
#[allow(unused)]
pub fn expand_tilde(s: &str) -> PathBuf {
    let p = shellexpand::tilde(s);
    PathBuf::from(p.as_ref())
}
