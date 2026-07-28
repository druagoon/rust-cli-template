{{ modules_start }}
mod {{ child_module }};
{{ modules_end }}

use crate::prelude::*;

#[derive(clap::Subcommand, cli_derive::CliCommand, Debug)]
pub enum {{ group_type }} {
    {{ variants_start }}
{{ child_variant }}    {{ variants_end }}
}
