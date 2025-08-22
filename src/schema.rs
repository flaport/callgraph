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
    pub resolved_calls: Vec<ResolvedCall>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResolvedCall {
    pub name: String,
    pub module: Option<String>,
    pub path: Option<String>,
}
