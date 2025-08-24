# Call Graph

A static analysis tool for generating call graphs from Python and YAML files.

## Features

- Analyzes Python files and extracts function calls, decorators, and imports
- Analyzes YAML files for component definitions
- Resolves function calls to their fully qualified names
- Tracks `gf.get_component` calls and their arguments
- Supports partial parsing of files with syntax errors
- Available as both a CLI tool and Python library

## CLI Usage

```bash
cargo run -- path/to/library1 path/to/library2 --prefix some.prefix
```

## Python Library Usage

```python
import callgraph

# Basic usage - analyze all functions in given directories
result = callgraph.generate_call_graph(['path/to/library1', 'path/to/library2'])
print(f"Found {len(result['functions'])} functions")

# Filter to a specific function
result = callgraph.generate_call_graph(
    ['path/to/library1'], 
    function_filter='my_function'
)

# Use prefix for YAML function resolution
result = callgraph.generate_call_graph(
    ['path/to/library1'], 
    prefix='my.module.prefix'
)

# Extract specific data using select path (colon-separated)
component_gets = callgraph.generate_call_graph(
    ['path/to/library1'],
    function_filter='my_function',
    select_path='functions:my.module.my_function:component_gets'
)
```

### Function Parameters

- `lib_paths`: List of directory paths to analyze
- `prefix` (optional): Prefix for YAML function resolution
- `function_filter` (optional): Filter results to a specific function name
- `select_path` (optional): Colon-separated path to extract specific data from results

### Return Value

Returns a Python dictionary with the same structure as the CLI JSON output:
- `functions`: Dictionary of function information keyed by fully qualified name
- `modules`: Dictionary of module information

## Quick Start

This project uses `uv` for Python package management and `just` for task automation. 

### Prerequisites

1. Install `uv`: `curl -LsSf https://astral.sh/uv/install.sh | sh` (or see [uv installation docs](https://docs.astral.sh/uv/getting-started/installation/))
2. Install `just`: `brew install just` (or see [just installation docs](https://just.systems/man/en/))

### Development Setup

```bash
# Set up development environment
just dev-setup

# Or manually:
just setup          # Create virtual environment
source .venv/bin/activate
just dev-install    # Build and install Python library (uses uv run maturin)
```

### Common Tasks

```bash
just                 # Show all available commands
just run-example     # Run CLI with example: mycspdk cspdk gdsfactory
just run <args>      # Run CLI with custom arguments
just build           # Build CLI tool (release)
just build-python    # Build Python library
just test            # Run tests
just clean           # Clean build artifacts
```

## Installation

### CLI Tool

Build the CLI tool using Cargo:

```bash
cargo build --release
./target/release/callgraph --help
```

### Python Library

Build the Python library using uv and maturin:

```bash
# Create and activate virtual environment with uv
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Build and install the Python library (using uv run to execute maturin)
uv run maturin develop  # For development
# or
uv run maturin build --release  # For production builds
uv pip install target/wheels/*.whl
```

Note: The Python library functionality is behind a feature flag. When building with `cargo build`, only the CLI functionality is included. The Python library is built using maturin which automatically enables the `python` feature.
