use anyhow::{Context, Result};
use call_graph::callgraph::CallGraphBuilder;
use clap::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "call_graph",
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

    let mut call_graph = builder.build_call_graph();

    // Filter to specific function if requested
    if let Some(function_name) = &args.function {
        if let Some(func_info) = call_graph.functions.get(function_name) {
            let mut filtered_functions = HashMap::new();
            filtered_functions.insert(function_name.clone(), func_info.clone());
            call_graph.functions = filtered_functions;
        } else {
            anyhow::bail!("Function '{}' not found in the call graph", function_name);
        }
    }

    let json_output = serde_json::to_string_pretty(&call_graph)
        .context("Failed to serialize call graph to JSON")?;

    println!("{}", json_output);

    Ok(())
}

fn find_analyzable_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir) {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in {}", dir.display()))?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.extension().map_or(false, |ext| ext == "py") || file_name.ends_with(".pic.yml")
            {
                files.push(path.to_path_buf());
            }
        }
    }

    Ok(files)
}
