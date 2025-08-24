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

## Installation

### CLI Tool

Build the CLI tool using Cargo:

```bash
cargo build --release
./target/release/callgraph --help
```

### Python Library

Build the Python library using maturin:

```bash
# Install maturin in a virtual environment
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install maturin

# Build and install the Python library
maturin develop  # For development
# or
maturin build --release  # For production builds
pip install target/wheels/*.whl
```

Note: The Python library functionality is behind a feature flag. When building with `cargo build`, only the CLI functionality is included. The Python library is built using maturin which automatically enables the `python` feature.
