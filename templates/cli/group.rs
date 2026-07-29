{% raw %}mod {{ child_module }};

use crate::prelude::*;

{% endraw %}#[derive(clap::Subcommand, {{ crate_name }}_derive::CliCommand, Debug)]{% raw %}
pub enum {{ group_type }} {
{{ child_variant }}}{% endraw %}
