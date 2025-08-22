use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;

use callgraph::callgraph::CallGraphBuilder;
use callgraph::walk::find_analyzable_files;

#[derive(Parser, Debug)]
#[command(
    name = "callgraph",
    version = "1.0",
    about = "Generates a call graph for a Python library"
)]
struct Args {
    /// Path to the top-level folder of the Python library
    path: PathBuf,

    /// Show only the specified function (optional)
    #[arg(short, long)]
    function: Option<String>,

    /// Additional dependency module paths to analyze (can be used multiple times)
    #[arg(short, long)]
    dependency: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }

    if !args.path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", args.path.display());
    }

    // Collect all paths to analyze (main path + dependencies)
    let mut all_paths = vec![args.path.clone()];
    for dep_path in &args.dependency {
        if !dep_path.exists() {
            eprintln!(
                "Warning: Dependency path does not exist: {}",
                dep_path.display()
            );
            continue;
        }
        if !dep_path.is_dir() {
            eprintln!(
                "Warning: Dependency path is not a directory: {}",
                dep_path.display()
            );
            continue;
        }
        all_paths.push(dep_path.clone());
    }

    let mut builder = CallGraphBuilder::new();

    // Analyze all paths (main + dependencies)
    for path in &all_paths {
        let files = find_analyzable_files(path)
            .with_context(|| format!("Failed to find analyzable files in {}", path.display()))?;

        if files.is_empty() {
            eprintln!(
                "Warning: No Python or YAML files found in {}",
                path.display()
            );
            continue;
        }

        for file_path in files {
            if let Err(e) = builder.analyze_file(&file_path) {
                eprintln!("Warning: Failed to analyze {}: {}", file_path.display(), e);
                continue;
            }
        }
    }

    let mut callgraph = builder.build_callgraph();

    // Filter to specific function if requested
    if let Some(function_name) = &args.function {
        if let Some(func_info) = callgraph.functions.get(function_name) {
            let mut filtered_functions = HashMap::new();
            filtered_functions.insert(function_name.clone(), func_info.clone());
            callgraph.functions = filtered_functions;
        } else {
            anyhow::bail!("Function '{}' not found in the call graph", function_name);
        }
    }

    let json_output = serde_json::to_string_pretty(&callgraph)
        .context("Failed to serialize call graph to JSON")?;

    println!("{}", json_output);

    Ok(())
}
