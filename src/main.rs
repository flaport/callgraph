use anyhow::Context;
use clap::Parser;
use std::collections::BTreeMap;
use std::path::PathBuf;

use callgraph::graph::build_graph;

#[derive(Parser, Debug)]
#[command(
    name = "callgraph",
    version = "1.0",
    about = "Generates a call graph for a Python library"
)]
struct Args {
    /// Library paths in the format "prefix:path" (e.g., "mycspdk:/path/to/mycspdk")
    /// Multiple paths can be specified and will be processed in order for resolution
    lib_paths: Vec<String>,

    /// Show only the specified function (optional)
    #[arg(short, long)]
    function: Option<String>,

    /// Select a specific nested key from the output using colon notation (e.g., "functions:mzi3:resolved_calls")
    #[arg(short, long)]
    select: Option<String>,

    /// Simplify
    #[arg(long)]
    simplify: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Parse lib_paths into a BTreeMap (sorted map)
    let mut lib_paths = BTreeMap::new();
    for lib_path_str in &args.lib_paths {
        let parts: Vec<&str> = lib_path_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid lib_path format '{}'. Expected 'prefix:path'",
                lib_path_str
            );
        }
        let prefix = parts[0].to_string();
        let path = PathBuf::from(parts[1]);
        lib_paths.insert(prefix, path);
    }

    let output_value = build_graph(lib_paths, args.function, args.select, args.simplify)?;
    let json_output = serde_json::to_string_pretty(&output_value)
        .context("Failed to serialize output to JSON")?;

    println!("{}", json_output);

    Ok(())
}
