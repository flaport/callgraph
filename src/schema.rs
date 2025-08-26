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
    pub partials: HashMap<String, PartialInfo>,
    pub imports: Vec<String>,
    pub aliases: std::collections::HashMap<String, String>,
    pub constants: std::collections::HashMap<String, String>,
    pub errors: Vec<String>,
}
