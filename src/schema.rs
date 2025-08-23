use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    pub functions: HashMap<String, FunctionInfo>,
    pub modules: HashMap<String, ModuleInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub module: String,
    pub line: usize,
    pub calls: Vec<String>,
    pub decorators: Vec<String>,
    pub resolved_calls: Vec<String>,
    pub resolved_decorators: Vec<String>,
    pub parameter_defaults: std::collections::HashMap<String, serde_json::Value>,
    pub component_gets: Vec<String>,
    pub resolved_component_gets: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
    pub functions: Vec<String>,
    pub partials: Vec<String>,
    pub imports: Vec<String>,
    pub aliases: std::collections::HashMap<String, String>,
    pub constants: std::collections::HashMap<String, String>,
    pub error: String,
}
