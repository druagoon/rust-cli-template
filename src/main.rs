mod cli;
mod commands;
#[cfg(feature = "config")]
mod config;
mod consts;
mod de;
mod error;
mod macros;
mod prelude;
mod utils;

fn main() {
    self::cli::Cli::exec();
}
