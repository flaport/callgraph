# Call Graph Development Tasks

# Show available commands
default:
    @just --list

# Set up development environment
setup:
    uv venv
    @echo "Virtual environment created. Activate with: source .venv/bin/activate"
    @echo "Then run: just dev-install"

# Install development dependencies
install-deps:
    @echo "Using uv run to execute maturin - no installation needed"

# Build the CLI tool (release mode)
build:
    cargo build --release

# Build the CLI tool (debug mode)
build-debug:
    cargo build

# Build and install Python library for development
dev-install:
    uv run maturin develop

# Build Python library for production
build-python:
    uv run maturin build --release

# Install Python library from wheel
install-python:
    uv pip install target/wheels/*.whl

# Run the CLI tool with example arguments (based on memory)
run-example:
    cargo run -- mycspdk cspdk gdsfactory

# Run the CLI tool with custom arguments
run *args:
    cargo run -- {{args}}

# Run tests
test:
    cargo test

# Format Rust code
fmt:
    cargo fmt

# Check Rust code
check:
    cargo check

# Run clippy linter
lint:
    cargo clippy

# Format Python files (if any)
fmt-python:
    ruff format .

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/wheels/
    rm -rf .venv/

# Full development setup (create venv, build)
dev-setup: setup dev-install

# Build everything (CLI and Python library)
build-all: build build-python

# Show project info
info:
    @echo "Call Graph - Static analysis tool for Python and YAML files"
    @echo "CLI usage: just run-example"
    @echo "Python library: just dev-install"
