mod cli;
mod commands;
#[cfg(feature = "config")]
mod config;
mod error;
mod macros;
mod prelude;

fn main() {
    self::cli::Cli::exec();
}
