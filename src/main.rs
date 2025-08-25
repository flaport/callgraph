use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

use callgraph::graph::build_graph;

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
    let args = Args::parse();
    let output_value = build_graph(
        args.paths,
        args.function,
        args.select,
        args.prefix,
        args.simplify,
    )?;
    let json_output = serde_json::to_string_pretty(&output_value)
        .context("Failed to serialize output to JSON")?;

    println!("{}", json_output);

    Ok(())
}
