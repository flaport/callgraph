use anyhow::Context;
use clap::Parser;
use log::debug;
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

    /// Simplify
    #[arg(long)]
    simplify: bool,
}

fn main() -> anyhow::Result<()> {
    // Initialize logger with INFO level by default, but respect RUST_LOG env var
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    // Collect all paths to analyze (main path + dependencies)
    let mut paths = vec![];
    for path in &args.paths {
        if !path.exists() {
            debug!("Dependency path does not exist: {}", path.display());
            continue;
        }
        if !path.is_dir() {
            debug!("Dependency path is not a directory: {}", path.display());
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
            debug!("No Python or YAML files found in {}", path.display());
            continue;
        }

        for file_path in files {
            if let Err(e) = builder.analyze_file(&file_path, &path) {
                debug!("Failed to analyze {}: {}", file_path.display(), e);
                // Continue processing - error is now captured in module
            }
        }
    }

    let mut callgraph = builder.build_callgraph(&args.prefix);

    // Filter to specific function if requested
    if let Some(function_name) = &args.function {
        let mut filtered = HashMap::new();
        for (name, func_info) in &callgraph.functions {
            if name == function_name || func_info.name == *function_name {
                filtered.insert(name.clone(), func_info.clone());
            }
        }
        callgraph.functions = filtered;
        // // Find function by name (could be just function name or full resolved name)
        // let matching_function = callgraph
        //     .functions
        //     .iter()
        //     .find(|(resolved_name, func_info)| {
        //         // Match either the full resolved name or just the function name
        //         *resolved_name == function_name || func_info.name == *function_name
        //     });
        // println!("{:?}", matching_function);

        let modules = callgraph
            .functions
            .values()
            .map(|f| f.module.clone())
            .collect::<Vec<_>>();
        let mut filtered = HashMap::new();
        for (name, mod_info) in callgraph.modules.iter() {
            if modules.contains(name) {
                filtered.insert(name.clone(), mod_info.clone());
            }
        }
        callgraph.modules = filtered;
    }

    // Serialize to JSON value for potential filtering
    let json_value =
        serde_json::to_value(&callgraph).context("Failed to serialize call graph to JSON value")?;

    // Apply selection filter if specified
    let output_value = if let Some(select_path) = &args.select {
        extract_json_path(&json_value, select_path).unwrap_or_else(|| {
            debug!("Path '{}' not found in output", select_path);
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
