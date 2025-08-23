use anyhow::Context;
use serde_yaml::Value;
use std::fs;
use std::path::Path;

use crate::builder::CallGraphBuilder;
use crate::schema::FunctionInfo;

pub fn analyze_yaml_file(
    builder: &mut CallGraphBuilder,
    file_path: &Path,
    lib_root: &Path,
) -> anyhow::Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read YAML file: {}", file_path.display()))?;

    let yaml: Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse YAML file: {}", file_path.display()))?;

    // Extract function name from file name (remove .pic.yml extension)
    let file_name = file_path.file_stem().and_then(|n| n.to_str()).unwrap_or("");
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

    // YAML files don't have decorators, imports, or partials
    let module_name = derive_yaml_module(file_path, lib_root);
    let func_info = FunctionInfo {
        name: func_name.to_string(),
        module: module_name.clone(),
        line: 1,
        calls,
        decorators: vec!["yaml".to_string()],
        resolved_calls: Vec::new(),
        resolved_decorators: Vec::new(),
    };

    // Add function to module's function list
    builder.add_function_to_module(&module_name, func_name);

    builder.functions.push(func_info);
    Ok(())
}

fn derive_yaml_module(file_path: &Path, lib_root: &Path) -> String {
    // Derive module path relative to the library root
    let parent_lib_root = lib_root.parent().unwrap_or(lib_root);
    if let Some(relative_path) = file_path.strip_prefix(parent_lib_root).ok() {
        relative_path
            .to_str()
            .unwrap_or("")
            .replace(std::path::MAIN_SEPARATOR, ".")
            .replace(".pic.yml", "_picyml")
    } else {
        file_path.to_str().unwrap_or("").to_string()
    }
}
