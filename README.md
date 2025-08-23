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

Build from source using Rust and maturin:

```bash
maturin build --release
pip install target/wheels/*.whl
```
