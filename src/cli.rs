use clap::{CommandFactory, Parser};

use crate::commands::Command;
use crate::error::CliResult;

pub trait CliCommand {
    fn run(&self) -> CliResult;
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(bin_name = clap::crate_name!())]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

impl Cli {
    pub fn exec() {
        let cli = Self::parse();
        if let Err(err) = cli.run() {
            eprintln!("{err:?}");
            std::process::exit(1);
        }
    }

    pub fn build() -> clap::Command {
        let mut command = Self::command();
        command.build();
        command
    }

    fn init_logging(&self) {
        let level = self.verbose.log_level_filter();
        env_logger::Builder::new().filter_level(level).init();
        log::debug!("initialized logging at {level}");
    }
}

impl CliCommand for Cli {
    fn run(&self) -> CliResult {
        self.init_logging();
        match &self.command {
            Some(command) => command.run(),
            None => {
                Self::command().print_long_help()?;
                println!();
                Ok(())
            }
        }
    }
}
