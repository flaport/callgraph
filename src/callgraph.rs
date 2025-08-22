use anyhow::Context;
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::schema::{CallGraph, FunctionInfo, ResolvedCall};
use super::yaml::analyze_yaml_file;

pub struct CallGraphBuilder {
    pub functions: HashMap<String, FunctionInfo>,
    pub current_file: String,
    pub current_file_path: PathBuf,
    pub imports: HashMap<String, String>, // alias -> full_module_path
}

impl CallGraphBuilder {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
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
            self.analyze_python_file(file_path)
        } else {
            Ok(())
        }
    }

    pub fn derive_module_path(&self, file_path: &Path) -> String {
        let path_str = file_path.display().to_string();

        // Remove file extension
        let without_extension = if path_str.ends_with(".py") {
            path_str.strip_suffix(".py").unwrap_or(&path_str)
        } else if path_str.ends_with(".pic.yml") {
            path_str.strip_suffix(".pic.yml").unwrap_or(&path_str)
        } else {
            &path_str
        };

        // Convert path separators to dots for module notation
        without_extension.replace('/', ".").replace('\\', ".")
    }

    fn analyze_python_file(&mut self, file_path: &Path) -> anyhow::Result<()> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Clear imports for each new file
        self.imports.clear();

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
            self.visit_stmt(stmt);
        }

        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
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

                // Resolve calls to modules (only keep those that can be resolved)
                // Note: Path resolution will be done later after all functions are analyzed
                let resolved_calls = calls
                    .iter()
                    .filter_map(|call| {
                        let resolved = self.resolve_call_module_only(call);
                        if resolved.module.is_some() {
                            Some(resolved)
                        } else {
                            None
                        }
                    })
                    .collect();

                let module_path = self.derive_module_path(&self.current_file_path);
                let func_info = FunctionInfo {
                    name: func_name.clone(),
                    module: format!("{}.{}", module_path, func_name),
                    file: self.current_file.clone(),
                    line: func_def.range.start().to_usize(),
                    calls,
                    decorators,
                    resolved_calls,
                };

                self.functions.insert(func_name, func_info);
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

                        // Resolve calls to modules (only keep those that can be resolved)
                        // Note: Path resolution will be done later after all functions are analyzed
                        let resolved_calls = calls
                            .iter()
                            .filter_map(|call| {
                                let resolved = self.resolve_call_module_only(call);
                                if resolved.module.is_some() {
                                    Some(resolved)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let module_path = self.derive_module_path(&self.current_file_path);
                        let func_info = FunctionInfo {
                            name: full_method_name.clone(),
                            module: format!("{}.{}", module_path, full_method_name),
                            file: self.current_file.clone(),
                            line: method_def.range.start().to_usize(),
                            calls,
                            decorators,
                            resolved_calls,
                        };

                        self.functions.insert(full_method_name, func_info);
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

    fn resolve_call_module_only(&self, call_name: &str) -> ResolvedCall {
        // Check if the call starts with a known import
        if let Some(dot_pos) = call_name.find('.') {
            let prefix = &call_name[..dot_pos];
            let function_name = &call_name[dot_pos + 1..];
            if let Some(module_path) = self.imports.get(prefix) {
                let full_module = format!("{}.{}", module_path, function_name);

                return ResolvedCall {
                    name: function_name.to_string(),
                    module: Some(full_module),
                    path: None, // Will be resolved later
                };
            }
        }

        // Check if the entire call name is an imported module/function
        if let Some(module_path) = self.imports.get(call_name) {
            let path = self.find_function_path(call_name, module_path);

            return ResolvedCall {
                name: call_name.to_string(),
                module: Some(module_path.clone()),
                path,
            };
        }

        // No module resolution found
        ResolvedCall {
            name: call_name.to_string(),
            module: None,
            path: None,
        }
    }

    fn find_function_path(&self, function_name: &str, expected_module: &str) -> Option<String> {
        // Look for the function in our analyzed functions
        // We need to check if any function matches the expected module pattern
        for (_, func_info) in &self.functions {
            if func_info.name == function_name && func_info.module.contains(expected_module) {
                return Some(func_info.file.clone());
            }
            // Also check if the function's module ends with the expected module
            if func_info.name == function_name
                && func_info.module.ends_with(&format!(".{}", function_name))
            {
                let module_prefix = func_info
                    .module
                    .strip_suffix(&format!(".{}", function_name))
                    .unwrap_or("");
                if expected_module.contains(module_prefix)
                    || module_prefix.contains(expected_module)
                {
                    return Some(func_info.file.clone());
                }
            }
        }
        None
    }

    pub fn build_callgraph(self) -> CallGraph {
        CallGraph {
            functions: self.functions,
        }
    }
}
