# rust-cli-template

An opinionated Rust CLI project template for `cargo-generate`.

## Features

- Clap command routing backed by a small derive macro.
- Optional runtime configuration management through the default `config` feature.
- Development-only command scaffolding through the `dev-tools` feature.
- Shell completion generation.
- CI, multi-platform releases, checksums, and Homebrew formula updates.

## Generate a project

Install `cargo-generate` 0.23 or newer:

```shell
cargo install cargo-generate --locked
```

Generate a project:

```shell
cargo generate \
  --git https://github.com/druagoon/rust-cli-template \
  --name my-cli \
  --define github_owner=druagoon \
  --define description="My command-line application"
```

Override derived Homebrew values when necessary:

```shell
cargo generate \
  --git https://github.com/druagoon/rust-cli-template \
  --name my-cli \
  --define github_owner=example \
  --define homebrew_tap=example/homebrew-tools \
  --define homebrew_formula=my-cli
```

This repository contains Liquid source files and is not intended to be copied with GitHub's
native **Use this template** button.

## Template development

Run the same checks as CI:

```shell
cargo generate \
  --path . \
  --name smoke-cli \
  --define github_owner=example \
  --define description="Smoke test CLI"
```

The template requires `cargo-generate` 0.23.0 or newer.
