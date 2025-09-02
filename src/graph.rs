use anyhow::Context;
use log::debug;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use crate::builder::CallGraphBuilder;
use crate::walk::find_analyzable_files;

pub fn build_graph(
    lib_paths: BTreeMap<String, PathBuf>,
    function: Option<String>,
    select: Option<String>,
    simplify: bool,
) -> anyhow::Result<serde_json::Value> {
    // Initialize logger with INFO level by default, but respect RUST_LOG env var
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Validate all paths exist
    let mut valid_lib_paths = BTreeMap::new();
    for (prefix, path) in &lib_paths {
        if !path.exists() {
            debug!(
                "Library path does not exist: {} -> {}",
                prefix,
                path.display()
            );
            continue;
        }
        if !path.is_dir() {
            debug!(
                "Library path is not a directory: {} -> {}",
                prefix,
                path.display()
            );
            continue;
        }
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;
        valid_lib_paths.insert(prefix.clone(), canonical_path);
    }

    if valid_lib_paths.is_empty() {
        anyhow::bail!("No valid library paths given.");
    }

    let mut builder = CallGraphBuilder::new(valid_lib_paths.clone());

    // Analyze all library paths
    for (prefix, path) in &valid_lib_paths {
        let files = find_analyzable_files(&path)
            .with_context(|| format!("Failed to find analyzable files in {}", path.display()))?;

        if files.is_empty() {
            debug!(
                "No Python or YAML files found in {} ({})",
                path.display(),
                prefix
            );
            continue;
        }

        for file_path in files {
            if let Err(e) = builder.analyze_file(&file_path, &path, prefix) {
                debug!("Failed to analyze {}: {}", file_path.display(), e);
                // Continue processing - error is now captured in module
            }
        }
    }

    let mut callgraph = builder.build_callgraph();

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
