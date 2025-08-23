use ruff_python_ast::{Expr, Stmt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::py::analyze_python_file;
use crate::schema::{CallGraph, FunctionInfo, ModuleInfo};
use crate::yaml::analyze_yaml_file;

pub struct CallGraphBuilder {
    pub functions: Vec<FunctionInfo>,
    pub modules: HashMap<String, ModuleInfo>,
    pub current_file: String,
    pub current_file_path: PathBuf,
    pub imports: HashMap<String, String>, // alias -> full_module_path
}

impl CallGraphBuilder {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
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
                imports: Vec::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_import_to_module(&mut self, module_name: &str, import: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add import if not already present
            if !module_info.imports.contains(&import.to_string()) {
                module_info.imports.push(import.to_string());
            }
        } else {
            // Create new module with this import
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: Vec::new(),
                imports: vec![import.to_string()],
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn derive_module(&self, file_path: &Path, lib_root: &Path) -> String {
        // Derive module path relative to the library root
        let parent_lib_root = lib_root.parent().unwrap_or(lib_root);
        if let Some(relative_path) = file_path.strip_prefix(parent_lib_root).ok() {
            let mut module_name = relative_path
                .to_str()
                .unwrap_or("")
                .replace(std::path::MAIN_SEPARATOR, ".")
                .replace(".py", "");

            // Remove .__init__ suffix for package __init__.py files
            // In Python, you import the package, not the __init__ file
            if module_name.ends_with(".__init__") {
                module_name = module_name
                    .strip_suffix(".__init__")
                    .unwrap_or(&module_name)
                    .to_string();
            }

            module_name
        } else {
            file_path.to_str().unwrap_or("").to_string()
        }
    }

    pub fn resolve_relative_import_with_level(
        &self,
        module_name: &str,
        current_module: &str,
        level: u32,
    ) -> String {
        // Handle relative imports using the level attribute
        // level 1 = single dot (.), level 2 = double dot (..), etc.

        let current_parts: Vec<&str> = current_module.split('.').collect();

        if level == 1 {
            // Single dot means import from current package
            // For __init__.py files, this means import from within the current package
            // For regular .py files, this means import from sibling module
            format!("{}.{}", current_module, module_name)
        } else if level > 1 {
            // Multiple dots mean go up directories
            let levels_up = (level - 1) as usize; // level 2 = go up 1 level, level 3 = go up 2 levels, etc.

            if levels_up >= current_parts.len() {
                // Can't go up more levels than we have
                return format!("{}{}", ".".repeat(level as usize), module_name);
            }

            let base_parts = &current_parts[..current_parts.len() - levels_up];
            let base_module = base_parts.join(".");

            if base_module.is_empty() {
                module_name.to_string()
            } else {
                format!("{}.{}", base_module, module_name)
            }
        } else {
            // level 0 should not happen here, but handle it as absolute
            module_name.to_string()
        }
    }

    pub fn resolve_relative_import(&self, relative_import: &str, current_module: &str) -> String {
        // Handle relative imports like "from .cells import mzi" or "from ..cband import cells"
        if relative_import.starts_with('.') {
            let dots = relative_import.chars().take_while(|&c| c == '.').count();
            let import_path = &relative_import[dots..];

            // Split current module into parts
            let current_parts: Vec<&str> = current_module.split('.').collect();

            if dots == 1 {
                // Single dot means relative to current package
                if import_path.is_empty() {
                    // "from . import something" - import from current package
                    current_module.to_string()
                } else {
                    // "from .something import ..."
                    // If we're in an __init__.py file, this imports from within the current package
                    // If we're in a regular .py file, this imports from a sibling module
                    format!("{}.{}", current_module, import_path)
                }
            } else {
                // Multiple dots mean go up directories
                if dots > current_parts.len() {
                    // Can't go up more levels than we have
                    return relative_import.to_string();
                }

                // Go up 'dots-1' levels from current module (dots-1 because 1 dot is current level)
                let levels_up = dots - 1;
                if levels_up >= current_parts.len() {
                    return relative_import.to_string();
                }

                let base_parts = &current_parts[..current_parts.len() - levels_up];
                let base_module = base_parts.join(".");

                if import_path.is_empty() {
                    base_module
                } else {
                    format!("{}.{}", base_module, import_path)
                }
            }
        } else {
            // Not a relative import, return as-is
            relative_import.to_string()
        }
    }

    pub fn visit_stmt(&mut self, stmt: &Stmt, lib_root: &Path) {
        let current_module = self.derive_module(&self.current_file_path, lib_root);

        match stmt {
            Stmt::Import(import_stmt) => {
                for alias in &import_stmt.names {
                    let module_name = alias.name.to_string();
                    let alias_name = alias
                        .asname
                        .as_ref()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| module_name.clone());

                    // Store in internal imports map for function resolution
                    self.imports.insert(alias_name, module_name.clone());

                    // Add absolute import to the current module's imports list
                    self.add_import_to_module(&current_module, &module_name);
                }
            }
            Stmt::ImportFrom(import_from_stmt) => {
                if let Some(module) = &import_from_stmt.module {
                    let module_name = module.to_string();

                    // Use the level attribute to determine if this is a relative import
                    let absolute_module = if import_from_stmt.level > 0 {
                        // This is a relative import
                        self.resolve_relative_import_with_level(
                            &module_name,
                            &current_module,
                            import_from_stmt.level,
                        )
                    } else {
                        // This is an absolute import
                        module_name.clone()
                    };

                    for alias in &import_from_stmt.names {
                        let imported_name = alias.name.to_string();
                        let alias_name = alias
                            .asname
                            .as_ref()
                            .map(|name| name.to_string())
                            .unwrap_or_else(|| imported_name.clone());

                        // Store in internal imports map for function resolution
                        let full_path = format!("{}.{}", absolute_module, imported_name);
                        self.imports.insert(alias_name, full_path.clone());

                        // Add absolute import to the current module's imports list
                        if imported_name == "*" {
                            // Handle star imports
                            let star_import = format!("{}.*", absolute_module);
                            self.add_import_to_module(&current_module, &star_import);
                        } else {
                            self.add_import_to_module(&current_module, &full_path);
                        }
                    }
                } else if import_from_stmt.level > 0 {
                    // Handle relative imports without explicit module (e.g., "from . import something")
                    // This means it's a relative import from the current package level
                    for alias in &import_from_stmt.names {
                        let imported_name = alias.name.to_string();
                        let alias_name = alias
                            .asname
                            .as_ref()
                            .map(|name| name.to_string())
                            .unwrap_or_else(|| imported_name.clone());

                        // This is a relative import - use level to determine the base module
                        let absolute_import = if import_from_stmt.level == 1 {
                            // Single dot: import from current package
                            if current_module.is_empty() {
                                imported_name.clone()
                            } else {
                                format!("{}.{}", current_module, imported_name)
                            }
                        } else {
                            // Multiple dots: go up directories
                            let current_parts: Vec<&str> = current_module.split('.').collect();
                            let levels_up = (import_from_stmt.level - 1) as usize;

                            if levels_up >= current_parts.len() {
                                imported_name.clone()
                            } else {
                                let base_parts = &current_parts[..current_parts.len() - levels_up];
                                let base_module = base_parts.join(".");
                                if base_module.is_empty() {
                                    imported_name.clone()
                                } else {
                                    format!("{}.{}", base_module, imported_name)
                                }
                            }
                        };

                        // Store in internal imports map for function resolution
                        self.imports.insert(alias_name, absolute_import.clone());

                        // Add absolute import to the current module's imports list
                        if imported_name == "*" {
                            let star_import = format!(
                                "{}.*",
                                absolute_import
                                    .strip_suffix(".*")
                                    .unwrap_or(&absolute_import)
                            );
                            self.add_import_to_module(&current_module, &star_import);
                        } else {
                            self.add_import_to_module(&current_module, &absolute_import);
                        }
                    }
                } else {
                    // Absolute import with no module (shouldn't happen in normal Python, but handle it)
                    // This case is rare and might indicate malformed import statements
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

                self.functions.push(func_info);
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

                        self.functions.push(func_info);
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

    pub fn build_callgraph(mut self, yaml_prefix: &Option<String>) -> CallGraph {
        // Now that all functions are analyzed, resolve the calls
        self.resolve_all_calls(yaml_prefix);

        CallGraph {
            functions: self
                .functions
                .iter()
                .map(|f| (format!("{}.{}", f.module, f.name), f.clone()))
                .collect(),
            modules: self.modules,
        }
    }

    fn resolve_all_calls(&mut self, yaml_prefix: &Option<String>) {
        let functions_clone = self.functions.clone();
        let modules_clone = self.modules.clone();

        for func_info in self.functions.iter_mut() {
            let mut resolved_calls = Vec::new();

            for call in &func_info.calls {
                // Check if this is a YAML function (has "yaml" decorator)
                if func_info.decorators.contains(&"yaml".to_string()) {
                    // For YAML functions, use simple name matching with prefix filtering
                    if let Some(resolved) =
                        Self::resolve_yaml_call(call, &functions_clone, yaml_prefix)
                    {
                        resolved_calls.push(resolved);
                    }
                } else {
                    // For Python functions, use import-aware resolution
                    if let Some(resolved) = Self::resolve_call_with_imports(
                        call,
                        &func_info.module,
                        &functions_clone,
                        &modules_clone,
                    ) {
                        resolved_calls.push(resolved);
                    }
                }
            }

            func_info.resolved_calls = resolved_calls;
        }
    }

    fn resolve_call_with_imports(
        call_name: &str,
        calling_module: &str,
        all_functions: &[FunctionInfo],
        all_modules: &HashMap<String, ModuleInfo>,
    ) -> Option<String> {
        // Get the module info for the calling module
        let current_module_info = all_modules.get(calling_module)?;

        if let Some(dot_pos) = call_name.find('.') {
            // Handle dotted calls like "cells.mzi"
            let imported_name = &call_name[..dot_pos];
            let function_name = &call_name[dot_pos + 1..];

            // Find the import that matches the imported_name
            if let Some(target_module) =
                Self::find_import_target(imported_name, current_module_info)
            {
                // Now resolve the function in the target module
                return Self::resolve_function_in_module(
                    function_name,
                    &target_module,
                    all_functions,
                    all_modules,
                );
            }
        } else {
            // Handle direct function calls - look in current module first, then imports
            // First check if it's defined in the current module
            if current_module_info
                .functions
                .contains(&call_name.to_string())
            {
                return Some(format!("{}.{}", calling_module, call_name));
            }

            // Then check imports (this would handle cases like "from module import function")
            for import in &current_module_info.imports {
                if import.ends_with(&format!(".{}", call_name)) {
                    // This import brings the function directly into scope
                    return Some(import.clone());
                }
            }
        }

        None
    }

    fn find_import_target(imported_name: &str, module_info: &ModuleInfo) -> Option<String> {
        // Look through the imports to find what the imported_name refers to
        for import in &module_info.imports {
            // Check if this is a direct module import like "cspdk.si220.cband.cells"
            if import.ends_with(&format!(".{}", imported_name)) || import == imported_name {
                // The imported_name refers to this module
                return Some(import.clone());
            }
            // Check if this is an aliased import (we'd need to track aliases separately)
        }
        None
    }

    fn resolve_function_in_module(
        function_name: &str,
        target_module: &str,
        all_functions: &[FunctionInfo],
        all_modules: &HashMap<String, ModuleInfo>,
    ) -> Option<String> {
        // First, check if the function is directly defined in this module
        if let Some(module_info) = all_modules.get(target_module) {
            if module_info.functions.contains(&function_name.to_string()) {
                return Some(format!("{}.{}", target_module, function_name));
            }

            // If not directly defined, check if it's imported
            // Case 1: Explicit import like "cspdk.si220.cband.cells.mzis.mzi"
            for import in &module_info.imports {
                if import.ends_with(&format!(".{}", function_name)) {
                    return Some(import.clone());
                }
            }

            // Case 2: Star imports like "cspdk.si220.cband.cells.mzis.*"
            for import in &module_info.imports {
                if import.ends_with(".*") {
                    let star_module = import.strip_suffix(".*").unwrap();
                    // Recursively check the star-imported module
                    if let Some(resolved) = Self::resolve_function_in_module(
                        function_name,
                        star_module,
                        all_functions,
                        all_modules,
                    ) {
                        return Some(resolved);
                    }
                }
            }
        }

        None
    }

    fn resolve_yaml_call(
        call_name: &str,
        all_functions: &[FunctionInfo],
        yaml_prefix: &Option<String>,
    ) -> Option<String> {
        // For YAML calls, do simple name matching against all available functions
        // If a prefix is provided, only consider functions whose module starts with that prefix

        let candidates: Vec<&FunctionInfo> = all_functions
            .iter()
            .filter(|func_info| {
                // Filter by prefix if provided
                if let Some(prefix) = yaml_prefix {
                    func_info.module.starts_with(prefix)
                } else {
                    true
                }
            })
            .collect();

        // First, try exact function name match
        for func_info in &candidates {
            if func_info.name == call_name {
                return Some(format!("{}.{}", func_info.module, func_info.name));
            }
        }

        // If no exact match found, try matching the last part of compound function names
        // This handles cases where YAML calls "mzi" but the function is named "cells.mzi"
        for func_info in &candidates {
            if func_info.name.contains('.') {
                if let Some(last_part) = func_info.name.split('.').last() {
                    if last_part == call_name {
                        return Some(format!("{}.{}", func_info.module, func_info.name));
                    }
                }
            }
        }

        None
    }
}
