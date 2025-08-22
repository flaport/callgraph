use anyhow::Context;
use ruff_python_parser::parse_module;
use std::fs;
use std::path::Path;

use crate::builder::CallGraphBuilder;

pub fn analyze_python_file(builder: &mut CallGraphBuilder, file_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Clear imports for each new file
    builder.imports.clear();

    let parsed = parse_module(&content).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse Python file {}: {:?}",
            file_path.display(),
            e
        )
    })?;

    // Extract the module from the parsed result
    let module = parsed.into_syntax();

    for stmt in &module.body {
        builder.visit_stmt(stmt);
    }

    Ok(())
}
