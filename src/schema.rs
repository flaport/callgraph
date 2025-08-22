use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct CallGraph {
    pub functions: HashMap<String, FunctionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub module: String,
    pub file: String,
    pub line: usize,
    pub calls: Vec<String>,
    pub decorators: Vec<String>,
    pub resolved_calls: Vec<ResolvedCall>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResolvedCall {
    pub name: String,
    pub module: Option<String>,
    pub path: Option<String>,
}

pub struct CallGraphBuilder {
    pub functions: HashMap<String, FunctionInfo>,
    pub current_file: String,
    pub current_file_path: PathBuf,
    pub imports: HashMap<String, String>, // alias -> full_module_path
}
