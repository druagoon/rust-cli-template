mod cli;
mod cmd;
#[cfg(feature = "config")]
mod conf;
mod consts;
mod de;
mod error;
mod macros;
mod prelude;
mod utils;

fn main() {
    self::cli::Cli::exec();
}
