use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

/// Reads the lines of a file and returns an iterator over the lines.
///
/// # Arguments
///
/// * `path` - A reference to a type that implements `AsRef<Path>`, representing
///   the path to the file.
///
/// # Returns
///
/// This function returns a `Result` containing an iterator over the lines of
/// the file, wrapped in an `io::Lines<io::BufReader<fs::File>>`.
///
/// # Errors
///
/// This function will return an error if the file cannot be opened.
///
/// # Examples
///
/// ```
/// use crate::utils::fs::read_lines;
///
/// if let Ok(lines) = read_lines("foo.txt") {
///     for line in lines {
///         if let Ok(content) = line {
///             println!("{}", content);
///         }
///     }
/// }
/// ```
pub fn read_lines<P: AsRef<Path>>(path: P) -> anyhow::Result<io::Lines<io::BufReader<fs::File>>> {
    let fp = fs::File::open(path)?;
    Ok(io::BufReader::new(fp).lines())
}

/// Sets the executable bit on a file.
///
/// # Arguments
///
/// * `path` - A reference to a type that implements `AsRef<Path>`, representing
///   the path to the file.
///
/// # Returns
///
/// This function returns `Ok(())` if the operation was successful.
///
/// # Platform-specific
///
/// This function is a no-op on Windows.
///
/// # Examples
///
/// ```
/// use crate::utils::fs::set_executable;
///
/// set_executable("foo")?;
/// ```
#[cfg(unix)]
pub fn set_executable<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_executable<P: AsRef<Path>>(_path: P) -> anyhow::Result<()> {
    Ok(())
}
