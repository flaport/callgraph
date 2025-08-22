use anyhow::Context;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

use super::callgraph::CallGraphBuilder;
use super::schema::FunctionInfo;

pub fn analyze_yaml_file(builder: &mut CallGraphBuilder, file_path: &Path) -> anyhow::Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read YAML file: {}", file_path.display()))?;

    let yaml: Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML file: {}", file_path.display()))?;

    // Extract function name from file name (remove .pic.yml extension)
    let file_name = file_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let func_name = file_name.strip_suffix(".pic").unwrap_or(file_name);

    let mut calls = Vec::new();

    // Extract component calls from instances
    if let Some(instances) = yaml.get("instances") {
        if let Some(instances_map) = instances.as_mapping() {
            for (_, instance) in instances_map {
                if let Some(component) = instance.get("component") {
                    if let Some(component_name) = component.as_str() {
                        calls.push(component_name.to_string());
                    }
                }
            }
        }
    }

    // YAML files don't have decorators or resolvable calls (no imports)
    let module_path = builder.derive_module_path(file_path);
    let func_info = FunctionInfo {
        name: func_name.to_string(),
        module: module_path,
        file: builder.current_file.clone(),
        line: 1, // YAML files start at line 1
        calls,
        decorators: Vec::new(),
        resolved_calls: Vec::new(),
    };

    builder.functions.insert(func_name.to_string(), func_info);
    Ok(())
}
