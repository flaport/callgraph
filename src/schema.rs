use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    pub functions: HashMap<String, FunctionInfo>,
    pub modules: HashMap<String, ModuleInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PartialInfo {
    pub func: String,
    pub args: Vec<serde_json::Value>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub module: String,
    pub line: usize,
    pub calls: IndexSet<String>,
    pub decorators: IndexSet<String>,
    pub resolved_calls: IndexSet<String>,
    pub resolved_decorators: IndexSet<String>,
    pub parameter_defaults: std::collections::HashMap<String, serde_json::Value>,
    pub resolved_parameter_defaults: std::collections::HashMap<String, serde_json::Value>,
    pub component_gets: IndexSet<String>,
    pub resolved_component_gets: IndexSet<String>,
    pub is_partial: bool,
    pub return_annotation: Option<String>,
    pub resolved_return_annotation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub functions: IndexSet<String>,
    pub partials: HashMap<String, PartialInfo>,
    pub imports: IndexSet<String>,
    pub aliases: std::collections::HashMap<String, String>,
    pub constants: std::collections::HashMap<String, String>,
    pub errors: IndexSet<String>,
}
