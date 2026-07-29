.DEFAULT_GOAL := help

SHELL := bash

.PHONY: check
check: ## Check the default feature set
	cargo check --locked --workspace --all-targets

.PHONY: check-features
check-features: ## Check every supported feature combination
	cargo check --locked --no-default-features
	cargo check --locked --no-default-features --features config
	cargo check --locked --no-default-features --features dev-tools
	cargo check --locked --all-features

.PHONY: test
test: ## Run tests with all features
	cargo test --locked --all-features

.PHONY: clippy
clippy: ## Run Clippy with all features
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

.PHONY: fmt
fmt: ## Format Rust sources
	cargo fmt --all

.PHONY: check-fmt
check-fmt: ## Check Rust formatting
	cargo fmt --all -- --check

.PHONY: check-toml
check-toml: ## Check TOML formatting
	taplo format --check

.PHONY: check-deps
check-deps: ## Check unused dependencies
# 	@echo "Checking for unused dependencies..."
	cargo machete
	cargo shear

.PHONY: fix-deps
fix-deps: ## Check unused dependencies and fix
	cargo machete --fix
	cargo shear --fix

.PHONY: check-all
check-all: check-fmt check-features test clippy ## Run all Rust checks

.PHONY: help
help: ## Display available targets
	@awk 'BEGIN {FS = ":.*##"; printf "Usage: make <target>\\n\\nTargets:\\n"} /^[a-zA-Z_-]+:.*?##/ {printf "  %-18s %s\\n", $$1, $$2}' $(MAKEFILE_LIST)
