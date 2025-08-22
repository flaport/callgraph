use ruff_python_ast::{Expr, Stmt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::py::analyze_python_file;
use crate::schema::{CallGraph, FunctionInfo, ModuleInfo};
use crate::yaml::analyze_yaml_file;

pub struct CallGraphBuilder {
    pub functions: HashMap<String, FunctionInfo>,
    pub modules: HashMap<String, ModuleInfo>,
    pub current_file: String,
    pub current_file_path: PathBuf,
    pub imports: HashMap<String, String>, // alias -> full_module_path
}

impl CallGraphBuilder {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            modules: HashMap::new(),
            current_file: String::new(),
            current_file_path: PathBuf::new(),
            imports: HashMap::new(),
        }
    }

    pub fn analyze_file(&mut self, file_path: &Path, lib_root: &Path) -> anyhow::Result<()> {
        self.current_file = file_path.display().to_string();
        self.current_file_path = file_path.to_path_buf();

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name.ends_with(".pic.yml") {
            analyze_yaml_file(self, file_path, lib_root)
        } else if file_path.extension().map_or(false, |ext| ext == "py") {
            analyze_python_file(self, file_path, lib_root)
        } else {
            Ok(())
        }
    }

    pub fn add_function_to_module(&mut self, module_name: &str, function_name: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add function if not already present
            if !module_info.functions.contains(&function_name.to_string()) {
                module_info.functions.push(function_name.to_string());
            }
        } else {
            // Create new module
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: vec![function_name.to_string()],
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn derive_module(&self, file_path: &Path, lib_root: &Path) -> String {
        // Derive module path relative to the library root
        let parent_lib_root = lib_root.parent().unwrap_or(lib_root);
        if let Some(relative_path) = file_path.strip_prefix(parent_lib_root).ok() {
            relative_path
                .to_str()
                .unwrap_or("")
                .replace(std::path::MAIN_SEPARATOR, ".")
                .replace(".py", "")
        } else {
            file_path.to_str().unwrap_or("").to_string()
        }
    }

    pub fn visit_stmt(&mut self, stmt: &Stmt, lib_root: &Path) {
        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    let module_name = alias.name.to_string();
                    let alias_name = alias
                        .asname
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| module_name.clone());
                    self.imports.insert(alias_name, module_name);
                }
            }
            Stmt::ImportFrom(import_from_stmt) => {
                if let Some(module) = &import_from_stmt.module {
                    let module_name = module.to_string();
                    for alias in &import_from_stmt.names {
                        let imported_name = alias.name.to_string();
                        let alias_name = alias
                            .asname
                            .as_ref()
                            .map(|name| name.to_string())
                            .unwrap_or_else(|| imported_name.clone());
                        let full_path = format!("{}.{}", module_name, imported_name);
                        self.imports.insert(alias_name, full_path);
                    }
                }
            }
            Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();
                let mut calls = Vec::new();

                for body_stmt in &func_def.body {
                    self.extract_calls_from_stmt(body_stmt, &mut calls);
                }

                // Extract decorator names
                let decorators = func_def
                    .decorator_list
                    .iter()
                    .filter_map(|decorator| self.get_decorator_name(decorator))
                    .collect();

                // Defer resolution until all functions are analyzed
                let resolved_calls = Vec::new();

                let module_path = self.derive_module(&self.current_file_path, lib_root);
                let func_info = FunctionInfo {
                    name: func_name.clone(),
                    module: module_path.clone(),
                    line: func_def.range.start().to_usize(),
                    calls,
                    decorators,
                    resolved_calls,
                };

                // Add function to module's function list
                self.add_function_to_module(&module_path, &func_name);

                self.functions.insert(func_info.name.clone(), func_info);
            }
            Stmt::ClassDef(class_def) => {
                for class_stmt in &class_def.body {
                    if let Stmt::FunctionDef(method_def) = class_stmt {
                        let full_method_name = format!("{}.{}", class_def.name, method_def.name);
                        let mut calls = Vec::new();

                        for body_stmt in &method_def.body {
                            self.extract_calls_from_stmt(body_stmt, &mut calls);
                        }

                        // Extract decorator names for methods
                        let decorators = method_def
                            .decorator_list
                            .iter()
                            .filter_map(|decorator| self.get_decorator_name(decorator))
                            .collect();

                        // Defer resolution until all functions are analyzed
                        let resolved_calls = Vec::new();

                        let module_path = self.derive_module(&self.current_file_path, lib_root);
                        let func_info = FunctionInfo {
                            name: full_method_name.clone(),
                            module: module_path.clone(),
                            line: method_def.range.start().to_usize(),
                            calls,
                            decorators,
                            resolved_calls,
                        };

                        // Add function to module's function list
                        self.add_function_to_module(&module_path, &full_method_name);

                        self.functions.insert(func_info.name.clone(), func_info);
                    }
                }
            }
            _ => {}
        }
    }

    fn extract_calls_from_stmt(&self, stmt: &Stmt, calls: &mut Vec<String>) {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                self.extract_calls_from_expr(&expr_stmt.value, calls);
            }
            Stmt::Assign(assign_stmt) => {
                self.extract_calls_from_expr(&assign_stmt.value, calls);
            }
            Stmt::Return(return_stmt) => {
                if let Some(value) = &return_stmt.value {
                    self.extract_calls_from_expr(value, calls);
                }
            }
            Stmt::If(if_stmt) => {
                self.extract_calls_from_expr(&if_stmt.test, calls);
                for s in &if_stmt.body {
                    self.extract_calls_from_stmt(s, calls);
                }
                for s in &if_stmt.elif_else_clauses {
                    for stmt in &s.body {
                        self.extract_calls_from_stmt(stmt, calls);
                    }
                }
            }
            Stmt::For(for_stmt) => {
                self.extract_calls_from_expr(&for_stmt.iter, calls);
                for s in &for_stmt.body {
                    self.extract_calls_from_stmt(s, calls);
                }
            }
            _ => {}
        }
    }

    fn extract_calls_from_expr(&self, expr: &Expr, calls: &mut Vec<String>) {
        match expr {
            Expr::Call(call_expr) => {
                if let Some(func_name) = self.get_function_name(&call_expr.func) {
                    calls.push(func_name);
                }

                // Recursively process the function being called (for method chains)
                self.extract_calls_from_expr(&call_expr.func, calls);

                // Process arguments
                for arg in &call_expr.arguments.args {
                    self.extract_calls_from_expr(arg, calls);
                }
            }
            Expr::Attribute(attr_expr) => {
                // Recursively process the value part of the attribute access
                self.extract_calls_from_expr(&attr_expr.value, calls);
            }
            Expr::BinOp(binop_expr) => {
                self.extract_calls_from_expr(&binop_expr.left, calls);
                self.extract_calls_from_expr(&binop_expr.right, calls);
            }
            Expr::List(list_expr) => {
                for elt in &list_expr.elts {
                    self.extract_calls_from_expr(elt, calls);
                }
            }
            Expr::Tuple(tuple_expr) => {
                for elt in &tuple_expr.elts {
                    self.extract_calls_from_expr(elt, calls);
                }
            }
            _ => {}
        }
    }

    fn get_function_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name_expr) => Some(name_expr.id.to_string()),
            Expr::Attribute(attr_expr) => {
                if let Some(base) = self.get_function_name(&attr_expr.value) {
                    Some(format!("{}.{}", base, attr_expr.attr))
                } else {
                    Some(attr_expr.attr.to_string())
                }
            }
            _ => None,
        }
    }

    fn get_decorator_name(&self, decorator: &ruff_python_ast::Decorator) -> Option<String> {
        match &decorator.expression {
            // Handle simple decorators like @my_decorator
            Expr::Name(_) | Expr::Attribute(_) => self.get_function_name(&decorator.expression),
            // Handle decorator calls like @functools.lru_cache(maxsize=128)
            Expr::Call(call_expr) => {
                if let Some(func_name) = self.get_function_name(&call_expr.func) {
                    Some(format!("{}(...)", func_name))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn resolve_call_to_definition(&self, call_name: &str) -> Option<String> {
        // Check if the call starts with a known import
        if let Some(dot_pos) = call_name.find('.') {
            let prefix = &call_name[..dot_pos];
            let function_name = &call_name[dot_pos + 1..];
            if let Some(module_path) = self.imports.get(prefix) {
                let expected_full_module = format!("{}.{}", module_path, function_name);

                // Find the actual function definition in our analyzed functions
                return self.find_function_definition(function_name, &expected_full_module);
            }
        }

        // Check if the entire call name is an imported module/function
        if let Some(module_path) = self.imports.get(call_name) {
            return self.find_function_definition(call_name, module_path);
        }

        // No module resolution found
        None
    }

    fn find_function_definition(
        &self,
        function_name: &str,
        expected_module: &str,
    ) -> Option<String> {
        // Look for the function in our analyzed functions
        // Return format: {module}.{func} where module is where the function is defined
        for (_, func_info) in &self.functions {
            if func_info.name == function_name {
                // Check various matching patterns for the expected module
                if func_info.module.contains(expected_module)
                    || expected_module.contains(&func_info.module)
                    || self.modules_match(expected_module, &func_info.module)
                {
                    return Some(format!("{}.{}", func_info.module, function_name));
                }
            }
        }
        None
    }

    fn modules_match(&self, expected: &str, actual: &str) -> bool {
        // More sophisticated module matching logic
        // Handle cases where the expected module might be a partial path

        // Split modules into parts
        let expected_parts: Vec<&str> = expected.split('.').collect();
        let actual_parts: Vec<&str> = actual.split('.').collect();

        // Check if the expected module is a suffix of the actual module
        if expected_parts.len() <= actual_parts.len() {
            let actual_suffix = &actual_parts[actual_parts.len() - expected_parts.len()..];
            if expected_parts == actual_suffix {
                return true;
            }
        }

        // Check if they share common significant parts (e.g., package name and function)
        if let (Some(expected_last), Some(actual_last)) =
            (expected_parts.last(), actual_parts.last())
        {
            if expected_last == actual_last {
                // Check if they share a common package structure
                let expected_package = expected_parts.get(expected_parts.len().saturating_sub(2));
                let actual_package = actual_parts.get(actual_parts.len().saturating_sub(2));
                if expected_package.is_some() && expected_package == actual_package {
                    return true;
                }
            }
        }

        false
    }

    pub fn build_callgraph(mut self) -> CallGraph {
        // Now that all functions are analyzed, resolve the calls
        self.resolve_all_calls();

        CallGraph {
            functions: self.functions,
            modules: self.modules,
        }
    }

    fn resolve_all_calls(&mut self) {
        let functions_clone = self.functions.clone();

        for (_, func_info) in self.functions.iter_mut() {
            let mut resolved_calls = Vec::new();

            for call in &func_info.calls {
                if let Some(resolved) =
                    Self::resolve_call_against_all_functions(call, &functions_clone)
                {
                    resolved_calls.push(resolved);
                }
            }

            func_info.resolved_calls = resolved_calls;
        }
    }

    fn resolve_call_against_all_functions(
        call_name: &str,
        all_functions: &HashMap<String, FunctionInfo>,
    ) -> Option<String> {
        // Simple resolution: look for functions with matching names
        // This is a simplified approach - in a full implementation, we'd need to track imports per file

        // Direct match - look for exact function name
        for (_, func_info) in all_functions {
            if func_info.name == call_name {
                return Some(format!("{}.{}", func_info.module, func_info.name));
            }
        }

        // Handle dotted calls like "cells.mzi" - look for function "mzi"
        if let Some(dot_pos) = call_name.find('.') {
            let function_name = &call_name[dot_pos + 1..];

            for (_, func_info) in all_functions {
                if func_info.name == function_name {
                    return Some(format!("{}.{}", func_info.module, func_info.name));
                }
            }
        }

        None
    }
}
