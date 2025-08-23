use anyhow::Context;
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;

use callgraph::builder::CallGraphBuilder;
use callgraph::walk::find_analyzable_files;

#[derive(Parser, Debug)]
#[command(
    name = "callgraph",
    version = "1.0",
    about = "Generates a call graph for a Python library"
)]
struct Args {
    /// Path to the top-level folder of the Python library
    paths: Vec<PathBuf>,

    /// Show only the specified function (optional)
    #[arg(short, long)]
    function: Option<String>,

    /// Select a specific nested key from the output using colon notation (e.g., "functions:mzi3:resolved_calls")
    #[arg(short, long)]
    select: Option<String>,

    /// Prefix for resolving YAML function calls (e.g., "cspdk.si220.cband")
    #[arg(long)]
    prefix: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Collect all paths to analyze (main path + dependencies)
    let mut paths = vec![];
    for path in &args.paths {
        if !path.exists() {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        paths.push(path.clone());
    }

    if paths.len() == 0 {
        anyhow::bail!("No valid paths given.");
    }

    let mut builder = CallGraphBuilder::new();

    // Analyze all paths (main + dependencies)
    for path in &paths {
        let path = path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;
        let files = find_analyzable_files(&path)
            .with_context(|| format!("Failed to find analyzable files in {}", path.display()))?;

        if files.is_empty() {
            continue;
        }

        for file_path in files {
            if let Err(_e) = builder.analyze_file(&file_path, &path) {
                // Silently skip files that can't be analyzed
                continue;
            }
        }
    }

    let mut callgraph = builder.build_callgraph(&args.prefix);

    // Filter to specific function if requested
    if let Some(function_name) = &args.function {
        // Find function by name (could be just function name or full resolved name)
        let matching_function = callgraph
            .functions
            .iter()
            .find(|(resolved_name, func_info)| {
                // Match either the full resolved name or just the function name
                *resolved_name == function_name || func_info.name == *function_name
            });

        if let Some((resolved_name, func_info)) = matching_function {
            let mut filtered_functions = HashMap::new();
            filtered_functions.insert(resolved_name.clone(), func_info.clone());
            let filtered_modules = filtered_functions
                .iter()
                .filter_map(|(_, ff)| callgraph.modules.get(&ff.module))
                .collect::<Vec<_>>();
            callgraph.functions = filtered_functions;
            callgraph.modules = filtered_modules
                .into_iter()
                .map(|m| (m.name.clone(), m.clone()))
                .collect();
        } else {
            anyhow::bail!("Function '{}' not found in the call graph", function_name);
        }
    }

    // Serialize to JSON value for potential filtering
    let json_value =
        serde_json::to_value(&callgraph).context("Failed to serialize call graph to JSON value")?;

    // Apply selection filter if specified
    let output_value = if let Some(select_path) = &args.select {
        extract_json_path(&json_value, select_path).unwrap_or_else(|| {
            // Silently return null if path not found
            serde_json::Value::Null
        })
    } else {
        json_value
    };

    let json_output = serde_json::to_string_pretty(&output_value)
        .context("Failed to serialize output to JSON")?;

    println!("{}", json_output);

    Ok(())
}

/// Extract a value from a JSON object using colon-separated path notation
/// Examples: "functions", "functions:mzi3", "functions:mzi3:resolved_calls"
fn extract_json_path(json: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split(':').collect();
    let mut current = json;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(part)?;
            }
            serde_json::Value::Array(arr) => {
                // Try to parse part as array index
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index)?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(current.clone())
}
