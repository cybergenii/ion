# Makefile for Ion development
.PHONY: help build test lint fmt check install clean release run-example

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build the project in debug mode
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run all tests
	cargo test

test-verbose: ## Run tests with verbose output
	cargo test -- --nocapture

lint: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code with rustfmt
	cargo fmt --all

fmt-check: ## Check code formatting without modifying files
	cargo fmt --all -- --check

check: fmt-check lint test ## Run all checks (format, lint, test)

install: ## Install Ion locally
	cargo install --path .

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

release: check ## Build release version
	cargo build --release
	@echo ""
	@echo "Release binary: target/release/ion"
	@echo "Version: $$(./target/release/ion --version)"

run-example: ## Run example: create test project
	cargo run -- new example-project --std 20
	@echo ""
	@echo "Created example project in ./example-project"

doc: ## Generate documentation
	cargo doc --no-deps --open

doc-private: ## Generate documentation including private items
	cargo doc --no-deps --document-private-items --open

bench: ## Run benchmarks
	cargo bench

update: ## Update dependencies
	cargo update

audit: ## Run security audit
	cargo audit

bloat: ## Analyze binary size
	cargo bloat --release

watch: ## Watch for changes and rebuild
	cargo watch -x build -x test

watch-run: ## Watch for changes and run
	cargo watch -x 'run -- new test-watch-project'

coverage: ## Generate code coverage report
	cargo tarpaulin --out Html --output-dir coverage

pre-commit: fmt lint test ## Run all pre-commit checks

ci: fmt-check lint test ## Run CI checks

.DEFAULT_GOAL := help

