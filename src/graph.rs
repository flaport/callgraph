use anyhow::Context;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::builder::CallGraphBuilder;
use crate::walk::find_analyzable_files;

pub fn build_graph(
    paths: Vec<PathBuf>,
    function: Option<String>,
    select: Option<String>,
    prefix: Option<String>,
    simplify: bool,
) -> anyhow::Result<serde_json::Value> {
    // Initialize logger with INFO level by default, but respect RUST_LOG env var
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Collect all paths to analyze (main path + dependencies)
    let _paths = paths;
    let mut paths = vec![];
    for path in &_paths {
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

    let mut callgraph = builder.build_callgraph(&prefix);

    // Filter to specific function if requested
    if let Some(function_name) = &function {
        let mut filtered = HashMap::new();
        for (name, func_info) in &callgraph.functions {
            if name == function_name || func_info.name == *function_name {
                filtered.insert(name.clone(), func_info.clone());
            }
        }
        callgraph.functions = filtered;

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

    let json_value = if simplify {
        let mut simple = HashMap::new();
        for (name, func_info) in &callgraph.functions {
            let mut calls = HashSet::new();
            calls.extend(func_info.resolved_calls.clone());
            calls.extend(func_info.resolved_component_gets.clone());
            simple.insert(name.clone(), calls);
        }
        serde_json::to_value(&simple).context("Failed to serialize call graph to JSON value")?
    } else {
        serde_json::to_value(&callgraph).context("Failed to serialize call graph to JSON value")?
    };

    // Apply selection filter if specified
    let output_value = if let Some(select_path) = &select {
        extract_json_path(&json_value, select_path).unwrap_or_else(|| {
            debug!("Path '{}' not found in output", select_path);
            serde_json::Value::Null
        })
    } else {
        json_value
    };

    Ok(output_value)
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
