use anyhow::Context;
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
    paths: Vec<PathBuf>,

    /// Show only the specified function (optional)
    #[arg(short, long)]
    function: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Collect all paths to analyze (main path + dependencies)
    let mut paths = vec![];
    for path in &args.paths {
        if !path.exists() {
            eprintln!(
                "Warning: Dependency path does not exist: {}",
                path.display()
            );
            continue;
        }
        if !path.is_dir() {
            eprintln!(
                "Warning: Dependency path is not a directory: {}",
                path.display()
            );
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
            eprintln!(
                "Warning: No Python or YAML files found in {}",
                path.display()
            );
            continue;
        }

        for file_path in files {
            if let Err(e) = builder.analyze_file(&file_path, &path) {
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
