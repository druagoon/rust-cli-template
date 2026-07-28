{% raw %}{{ modules_start }}
mod {{ child_module }};
{{ modules_end }}

use crate::prelude::*;

{% endraw %}#[derive(clap::Subcommand, {{ crate_name }}_derive::CliCommand, Debug)]{% raw %}
pub enum {{ group_type }} {
    {{ variants_start }}
{{ child_variant }}    {{ variants_end }}
}{% endraw %}
