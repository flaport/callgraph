use indexmap::{IndexMap, IndexSet};
use ruff_python_ast::{Expr, Stmt};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::py::{FileAnalyzer, PythonAnalyzer};
use crate::schema::{CallGraph, FunctionInfo, ModuleInfo};
use crate::yaml::YamlAnalyzer;

pub struct CallGraphBuilder {
    pub functions: Vec<FunctionInfo>, // Keep as Vec since we build incrementally
    pub modules: HashMap<String, ModuleInfo>,
    pub current_file: String,
    pub current_file_path: PathBuf,
    pub current_prefix: String,           // Current prefix being processed
    pub imports: HashMap<String, String>, // alias -> full_module_path
    pub current_function_defaults: HashMap<String, serde_json::Value>, // param_name -> default_value
    pub current_function_component_gets: IndexSet<String>, // component gets for current function
    pub lib_paths: IndexMap<String, PathBuf>, // Ordered mapping of prefix -> library path (preserves insertion order)
}

impl CallGraphBuilder {
    pub fn new(lib_paths: IndexMap<String, PathBuf>) -> Self {
        Self {
            functions: Vec::new(),
            modules: HashMap::new(),
            current_file: String::new(),
            current_file_path: PathBuf::new(),
            current_prefix: String::new(),
            imports: HashMap::new(),
            current_function_defaults: HashMap::new(),
            current_function_component_gets: IndexSet::new(),
            lib_paths,
        }
    }

    pub fn analyze_file(
        &mut self,
        file_path: &Path,
        lib_root: &Path,
        prefix: &str,
    ) -> anyhow::Result<()> {
        self.current_file = file_path.display().to_string();
        self.current_file_path = file_path.to_path_buf();
        self.current_prefix = prefix.to_string();

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let result = if file_name.ends_with(".pic.yml") {
            YamlAnalyzer::analyze_file(self, file_path, lib_root, prefix)
        } else if file_path.extension().map_or(false, |ext| ext == "py") {
            PythonAnalyzer::analyze_file(self, file_path, lib_root, prefix)
        } else {
            Ok(())
        };

        // If there was an error, add it to the module
        if let Err(ref error) = result {
            let module_name = self.derive_module(file_path, lib_root, prefix);
            self.add_error_to_module(&module_name, &error.to_string());
        }

        result
    }

    pub fn add_function_to_module(&mut self, module_name: &str, function_name: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add function if not already present
            module_info.functions.insert(function_name.to_string());
        } else {
            // Create new module
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::from([function_name.to_string()]),
                partials: HashMap::new(),
                imports: IndexSet::new(),
                aliases: std::collections::HashMap::new(),
                constants: std::collections::HashMap::new(),
                errors: IndexSet::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_import_to_module(&mut self, module_name: &str, import: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add import if not already present
            module_info.imports.insert(import.to_string());
        } else {
            // Create new module with this import
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::new(),
                partials: HashMap::new(),
                imports: IndexSet::from([import.to_string()]),
                aliases: std::collections::HashMap::new(),
                constants: std::collections::HashMap::new(),
                errors: IndexSet::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_partial_to_module(
        &mut self,
        module_name: &str,
        partial_name: &str,
        func: &str,
        args: Vec<serde_json::Value>,
        kwargs: HashMap<String, serde_json::Value>,
    ) {
        let partial_info = crate::schema::PartialInfo {
            func: func.to_string(),
            args,
            kwargs,
        };

        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add partial
            module_info
                .partials
                .insert(partial_name.to_string(), partial_info);
        } else {
            // Create new module with this partial
            let mut partials = HashMap::new();
            partials.insert(partial_name.to_string(), partial_info);
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::new(),
                partials,
                imports: IndexSet::new(),
                aliases: std::collections::HashMap::new(),
                constants: std::collections::HashMap::new(),
                errors: IndexSet::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_alias_to_module(&mut self, module_name: &str, alias: &str, full_path: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add alias
            module_info
                .aliases
                .insert(alias.to_string(), full_path.to_string());
        } else {
            // Create new module with this alias
            let mut aliases = std::collections::HashMap::new();
            aliases.insert(alias.to_string(), full_path.to_string());
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::new(),
                partials: HashMap::new(),
                imports: IndexSet::new(),
                aliases,
                constants: std::collections::HashMap::new(),
                errors: IndexSet::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_constant_to_module(
        &mut self,
        module_name: &str,
        constant_name: &str,
        constant_value: &str,
    ) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, add constant
            module_info
                .constants
                .insert(constant_name.to_string(), constant_value.to_string());
        } else {
            // Create new module with this constant
            let mut constants = std::collections::HashMap::new();
            constants.insert(constant_name.to_string(), constant_value.to_string());
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::new(),
                partials: HashMap::new(),
                imports: IndexSet::new(),
                aliases: std::collections::HashMap::new(),
                constants,
                errors: IndexSet::new(),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn add_error_to_module(&mut self, module_name: &str, error: &str) {
        if let Some(module_info) = self.modules.get_mut(module_name) {
            // Module exists, set error
            module_info.errors.insert(error.to_string());
        } else {
            // Create new module with error
            let module_info = ModuleInfo {
                name: module_name.to_string(),
                path: self.current_file.clone(),
                functions: IndexSet::new(),
                partials: HashMap::new(),
                imports: IndexSet::new(),
                aliases: std::collections::HashMap::new(),
                constants: std::collections::HashMap::new(),
                errors: IndexSet::from([error.to_string()]),
            };
            self.modules.insert(module_name.to_string(), module_info);
        }
    }

    pub fn derive_module(&self, file_path: &Path, lib_root: &Path, prefix: &str) -> String {
        // Derive module path relative to the library root
        if let Some(relative_path) = file_path.strip_prefix(lib_root).ok() {
            let mut module_path = relative_path
                .to_str()
                .unwrap_or("")
                .replace(std::path::MAIN_SEPARATOR, ".")
                .replace(".py", "");

            // Remove __init__ suffix for package __init__.py files
            // In Python, you import the package, not the __init__ file
            if module_path.ends_with(".__init__") {
                module_path = module_path
                    .strip_suffix(".__init__")
                    .unwrap_or(&module_path)
                    .to_string();
            } else if module_path == "__init__" {
                // Handle the case where __init__.py is at the root of the library
                module_path = String::new();
            }

            // Combine prefix with the module path
            if module_path.is_empty() {
                prefix.to_string()
            } else {
                format!("{}.{}", prefix, module_path)
            }
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

    pub fn visit_stmt(&mut self, stmt: &Stmt, lib_root: &Path, prefix: &str) {
        let current_module = self.derive_module(&self.current_file_path, lib_root, prefix);

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
                    self.imports.insert(alias_name.clone(), module_name.clone());

                    // Add absolute import to the current module's imports list
                    self.add_import_to_module(&current_module, &module_name);

                    // Add alias to module if it's different from the original name
                    if alias_name != module_name {
                        self.add_alias_to_module(&current_module, &alias_name, &module_name);
                    }
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
                        self.imports.insert(alias_name.clone(), full_path.clone());

                        // Add alias to module if it's different from the imported name
                        if alias_name != imported_name {
                            self.add_alias_to_module(&current_module, &alias_name, &full_path);
                        }

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
                        self.imports
                            .insert(alias_name.clone(), absolute_import.clone());

                        // Add alias to module if it's different from the imported name
                        if alias_name != imported_name {
                            self.add_alias_to_module(
                                &current_module,
                                &alias_name,
                                &absolute_import,
                            );
                        }

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
            Stmt::Assign(assign_stmt) => {
                // Check for functools.partial assignments
                // Examples: my_func = functools.partial(some_function, arg1)
                //           my_func = partial(some_function, arg1)
                self.detect_partial_assignments(assign_stmt);

                // Check for module-level constants
                // Examples: gc = "grating_coupler_elliptical"
                self.detect_constant_assignments(assign_stmt);
            }
            Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();
                let mut calls = IndexSet::new();

                // Extract parameter defaults first
                let parameter_defaults = self.extract_parameter_defaults(&func_def.parameters);

                // Set current function defaults for argument resolution
                self.current_function_defaults = parameter_defaults.clone();

                // Clear component gets for this function
                self.current_function_component_gets.clear();

                for body_stmt in &func_def.body {
                    self.extract_calls_from_stmt(body_stmt, &mut calls);
                    // Also extract component_gets for this function
                    self.extract_component_gets_from_stmt(body_stmt);
                }

                // Extract decorator names
                let decorators = func_def
                    .decorator_list
                    .iter()
                    .filter_map(|decorator| self.get_decorator_name(decorator))
                    .collect();

                // Extract return type annotation
                let return_annotation = self.extract_return_annotation(func_def);

                // Defer resolution until all functions are analyzed
                let resolved_calls = IndexSet::new();

                let module_path =
                    self.derive_module(&self.current_file_path, lib_root, &self.current_prefix);
                let func_info = FunctionInfo {
                    name: func_name.clone(),
                    module: module_path.clone(),
                    line: func_def.range.start().to_usize(),
                    calls,
                    decorators,
                    resolved_calls,
                    resolved_decorators: IndexSet::new(),
                    parameter_defaults,
                    component_gets: self.current_function_component_gets.clone(),
                    resolved_component_gets: IndexSet::new(),
                    is_partial: false,
                    return_annotation,
                    resolved_return_annotation: None,
                };

                // Add function to module's function list
                self.add_function_to_module(&module_path, &func_name);

                self.functions.push(func_info);
            }
            Stmt::ClassDef(class_def) => {
                for class_stmt in &class_def.body {
                    if let Stmt::FunctionDef(method_def) = class_stmt {
                        let full_method_name = format!("{}.{}", class_def.name, method_def.name);
                        let mut calls = IndexSet::new();

                        // Extract parameter defaults for methods first
                        let parameter_defaults =
                            self.extract_parameter_defaults(&method_def.parameters);

                        // Set current function defaults for argument resolution
                        self.current_function_defaults = parameter_defaults.clone();

                        // Clear component gets for this method
                        self.current_function_component_gets.clear();

                        for body_stmt in &method_def.body {
                            self.extract_calls_from_stmt(body_stmt, &mut calls);
                            // Also extract component_gets for this method
                            self.extract_component_gets_from_stmt(body_stmt);
                        }

                        // Extract decorator names for methods
                        let decorators = method_def
                            .decorator_list
                            .iter()
                            .filter_map(|decorator| self.get_decorator_name(decorator))
                            .collect();

                        // Extract return type annotation for methods
                        let return_annotation = self.extract_return_annotation(method_def);

                        // Defer resolution until all functions are analyzed
                        let resolved_calls = IndexSet::new();

                        let module_path = self.derive_module(
                            &self.current_file_path,
                            lib_root,
                            &self.current_prefix,
                        );
                        let func_info = FunctionInfo {
                            name: full_method_name.clone(),
                            module: module_path.clone(),
                            line: method_def.range.start().to_usize(),
                            calls,
                            decorators,
                            resolved_calls,
                            resolved_decorators: IndexSet::new(),
                            parameter_defaults,
                            component_gets: self.current_function_component_gets.clone(),
                            resolved_component_gets: IndexSet::new(),
                            is_partial: false,
                            return_annotation,
                            resolved_return_annotation: None,
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

    fn extract_calls_from_stmt(&self, stmt: &Stmt, calls: &mut IndexSet<String>) {
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

    fn extract_calls_from_expr(&self, expr: &Expr, calls: &mut IndexSet<String>) {
        match expr {
            Expr::Call(call_expr) => {
                if let Some(func_name) = self.get_function_name(&call_expr.func) {
                    calls.insert(func_name);
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

    fn extract_return_annotation(
        &self,
        func_def: &ruff_python_ast::StmtFunctionDef,
    ) -> Option<String> {
        func_def
            .returns
            .as_ref()
            .map(|annotation| self.expr_to_string(annotation))
    }

    fn expr_to_string(&self, expr: &ruff_python_ast::Expr) -> String {
        use ruff_python_ast::Expr;
        match expr {
            Expr::Name(name_expr) => name_expr.id.to_string(),
            Expr::Attribute(attr_expr) => {
                format!(
                    "{}.{}",
                    self.expr_to_string(&attr_expr.value),
                    attr_expr.attr
                )
            }
            Expr::Subscript(subscript_expr) => {
                format!(
                    "{}[{}]",
                    self.expr_to_string(&subscript_expr.value),
                    self.expr_to_string(&subscript_expr.slice)
                )
            }
            Expr::Tuple(tuple_expr) => {
                let elements: Vec<String> = tuple_expr
                    .elts
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect();
                format!("({})", elements.join(", "))
            }
            Expr::List(list_expr) => {
                let elements: Vec<String> = list_expr
                    .elts
                    .iter()
                    .map(|e| self.expr_to_string(e))
                    .collect();
                format!("[{}]", elements.join(", "))
            }
            Expr::StringLiteral(string_expr) => {
                format!("\"{}\"", string_expr.value)
            }
            Expr::NumberLiteral(num_expr) => match &num_expr.value {
                ruff_python_ast::Number::Int(int_val) => int_val.to_string(),
                ruff_python_ast::Number::Float(float_val) => float_val.to_string(),
                ruff_python_ast::Number::Complex { real, imag } => format!("({real}+{imag}j)"),
            },
            Expr::BooleanLiteral(bool_expr) => {
                format!("{}", bool_expr.value)
            }
            Expr::BinOp(binop_expr) => {
                format!(
                    "{} | {}",
                    self.expr_to_string(&binop_expr.left),
                    self.expr_to_string(&binop_expr.right)
                )
            }
            _ => "<unknown>".to_string(),
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

    fn find_base_function<'a>(
        mut current_func: &'a FunctionInfo,
        functions: &'a [FunctionInfo],
    ) -> &'a FunctionInfo {
        // Follow the chain of partials until we find a non-partial function
        while current_func.is_partial && !current_func.resolved_calls.is_empty() {
            if let Some(wrapped_func_name) = current_func.resolved_calls.first() {
                // Find the wrapped function
                let mut wrapped_func = functions
                    .iter()
                    .find(|f| format!("{}.{}", f.module, f.name) == *wrapped_func_name);

                // If not found by exact match, try to find by function name alone
                if wrapped_func.is_none() {
                    if let Some(function_name) = wrapped_func_name.split('.').last() {
                        wrapped_func = functions.iter().find(|f| {
                            f.name == function_name && f.module.starts_with("gdsfactory")
                        });
                    }
                }

                if let Some(found_func) = wrapped_func {
                    current_func = found_func;
                } else {
                    break; // Can't find the wrapped function, stop here
                }
            } else {
                break; // No resolved calls, stop here
            }
        }
        current_func
    }

    fn apply_partial_overrides(
        func: &mut FunctionInfo,
        functions: &[FunctionInfo],
        modules: &std::collections::HashMap<String, crate::schema::ModuleInfo>,
    ) {
        // Collect all partial overrides by following the chain
        let mut overrides = std::collections::HashMap::new();

        // Start with the current partial's overrides
        if let Some(module_info) = modules.get(&func.module) {
            if let Some(partial_info) = module_info.partials.get(&func.name) {
                for (key, value) in &partial_info.kwargs {
                    overrides.insert(key.clone(), value.clone());
                }
            }
        }

        // Follow the chain and collect overrides from each partial
        let mut current_func_name = func.resolved_calls.first().cloned();
        while let Some(wrapped_func_name) = current_func_name {
            // Find the wrapped function
            let mut wrapped_func = functions
                .iter()
                .find(|f| format!("{}.{}", f.module, f.name) == wrapped_func_name);

            if wrapped_func.is_none() {
                if let Some(function_name) = wrapped_func_name.split('.').last() {
                    wrapped_func = functions
                        .iter()
                        .find(|f| f.name == function_name && f.module.starts_with("gdsfactory"));
                }
            }

            if let Some(found_func) = wrapped_func {
                if found_func.is_partial {
                    // Get the partial info for this function
                    if let Some(module_info) = modules.get(&found_func.module) {
                        if let Some(partial_info) = module_info.partials.get(&found_func.name) {
                            for (key, value) in &partial_info.kwargs {
                                // Only add if not already overridden by a more specific partial
                                if !overrides.contains_key(key) {
                                    overrides.insert(key.clone(), value.clone());
                                }
                            }
                        }
                    }
                    current_func_name = found_func.resolved_calls.first().cloned();
                } else {
                    break; // Reached the base function
                }
            } else {
                break; // Can't find the function
            }
        }

        // Apply all collected overrides
        for (key, value) in overrides {
            func.parameter_defaults.insert(key, value);
        }
    }

    fn populate_partial_function_details(&mut self) {
        // Create a clone of functions to avoid borrowing issues
        let functions_clone = self.functions.clone();

        // Find all partial functions and update their details
        for func in self.functions.iter_mut() {
            if func.is_partial && !func.resolved_calls.is_empty() {
                // Get the wrapped function from resolved_calls (should have exactly one)
                if let Some(wrapped_func_name) = func.resolved_calls.first() {
                    // Find the wrapped function by exact match first
                    let mut wrapped_func = functions_clone
                        .iter()
                        .find(|f| format!("{}.{}", f.module, f.name) == *wrapped_func_name);

                    // If not found by exact match, try to find by function name alone
                    // This handles cases where resolution gives us "gdsfactory.routing.route_bundle_electrical"
                    // but the actual function is "gdsfactory.routing.route_bundle.route_bundle_electrical"
                    if wrapped_func.is_none() {
                        if let Some(function_name) = wrapped_func_name.split('.').last() {
                            wrapped_func = functions_clone.iter().find(|f| {
                                f.name == function_name && f.module.starts_with("gdsfactory")
                            });
                        }
                    }

                    if let Some(wrapped_func) = wrapped_func {
                        // Find the original base function by following the chain of partials
                        let base_func = Self::find_base_function(wrapped_func, &functions_clone);

                        // Copy parameter defaults from the base function
                        func.parameter_defaults = base_func.parameter_defaults.clone();

                        // Override with kwargs from all partials in the chain
                        Self::apply_partial_overrides(func, &functions_clone, &self.modules);

                        // Copy decorators from the base function (excluding 'partial' decorators)
                        func.decorators = base_func
                            .decorators
                            .iter()
                            .filter(|d| *d != "partial")
                            .cloned()
                            .collect();

                        // Copy resolved decorators from the base function (excluding 'functools.partial')
                        func.resolved_decorators = base_func
                            .resolved_decorators
                            .iter()
                            .filter(|d| *d != "functools.partial")
                            .cloned()
                            .collect();

                        // Copy component gets from the base function
                        func.component_gets = base_func.component_gets.clone();
                        func.resolved_component_gets = base_func.resolved_component_gets.clone();

                        // Copy return annotations from the base function
                        func.return_annotation = base_func.return_annotation.clone();
                        func.resolved_return_annotation =
                            base_func.resolved_return_annotation.clone();
                    }
                }
            }
        }
    }

    pub fn build_callgraph(mut self) -> CallGraph {
        // Now that all functions are analyzed, resolve the calls
        self.resolve_all_calls();

        // After resolving calls, populate partial function details from their wrapped functions
        self.populate_partial_function_details();

        CallGraph {
            functions: self
                .functions
                .iter()
                .map(|f| (format!("{}.{}", f.module, f.name), f.clone()))
                .collect(),
            modules: self.modules,
        }
    }

    fn resolve_type_annotation(
        annotation: &str,
        current_module: &str,
        modules: &std::collections::HashMap<String, crate::schema::ModuleInfo>,
    ) -> String {
        // Handle common patterns in type annotations
        if annotation.contains('.') {
            // This looks like a qualified type (e.g., "typing.List", "mymodule.MyClass")
            let parts: Vec<&str> = annotation.split('.').collect();
            if parts.len() >= 2 {
                let first_part = parts[0];

                // Check if the first part is an alias in the current module
                if let Some(module_info) = modules.get(current_module) {
                    if let Some(resolved_first_part) = module_info.aliases.get(first_part) {
                        // Replace the first part with the resolved alias
                        let remaining_parts = parts[1..].join(".");
                        return format!("{}.{}", resolved_first_part, remaining_parts);
                    }
                }
            }
        }

        // For now, return the annotation as-is if we can't resolve it
        // This handles built-in types like str, int, bool, etc.
        annotation.to_string()
    }

    fn resolve_all_calls(&mut self) {
        let functions_clone = self.functions.clone();
        let modules_clone = self.modules.clone();
        let lib_paths_clone = self.lib_paths.clone();

        for func_info in self.functions.iter_mut() {
            let mut resolved_calls = IndexSet::new();
            let mut resolved_decorators = IndexSet::new();

            // Resolve calls
            for call in &func_info.calls {
                // Check if this is a YAML function (has "yaml" decorator)
                if func_info.decorators.contains(&"yaml".to_string()) {
                    // For YAML functions, use simple name matching with prefix filtering
                    if let Some(resolved) =
                        Self::resolve_yaml_call_static(call, &functions_clone, &lib_paths_clone)
                    {
                        resolved_calls.insert(resolved);
                    }
                } else {
                    // For Python functions, use import-aware resolution
                    if let Some(resolved) = Self::resolve_call_with_imports(
                        call,
                        &func_info.module,
                        &functions_clone,
                        &modules_clone,
                    ) {
                        resolved_calls.insert(resolved);
                    }
                }
            }

            // Resolve decorators (only for Python functions, not YAML)
            if !func_info.decorators.contains(&"yaml".to_string()) {
                for decorator in &func_info.decorators {
                    // Remove the (...) suffix from decorator calls for resolution
                    let decorator_name = if decorator.ends_with("(...)") {
                        decorator.strip_suffix("(...)").unwrap_or(decorator)
                    } else {
                        decorator
                    };

                    if let Some(resolved) = Self::resolve_call_with_imports(
                        decorator_name,
                        &func_info.module,
                        &functions_clone,
                        &modules_clone,
                    ) {
                        // Restore the (...) suffix if it was there
                        if decorator.ends_with("(...)") {
                            resolved_decorators.insert(format!("{}(...)", resolved));
                        } else {
                            resolved_decorators.insert(resolved);
                        }
                    }
                }
            }

            // Resolve component_gets (similar to YAML resolution)
            let mut resolved_component_gets = IndexSet::new();
            for component_get in &func_info.component_gets {
                let component_name = component_get.trim_matches('"');
                // First try to resolve as a YAML call (simple function name)
                if let Some(resolved) = Self::resolve_yaml_call_static(
                    &component_name,
                    &functions_clone,
                    &lib_paths_clone,
                ) {
                    resolved_component_gets.insert(resolved);
                } else if component_name.contains('.') {
                    // If it's a dotted name and YAML resolution failed, try function call resolution
                    if let Some(resolved) = Self::resolve_call_with_imports(
                        &component_name,
                        &func_info.module,
                        &functions_clone,
                        &modules_clone,
                    ) {
                        resolved_component_gets.insert(resolved);
                    }
                }
            }

            func_info.resolved_calls = resolved_calls;
            func_info.resolved_decorators = resolved_decorators;
            func_info.resolved_component_gets = resolved_component_gets;

            // Resolve return type annotation
            if let Some(ref return_annotation) = func_info.return_annotation {
                func_info.resolved_return_annotation = Some(Self::resolve_type_annotation(
                    return_annotation,
                    &func_info.module,
                    &modules_clone,
                ));
            }
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
            // Handle dotted calls like "cells.mzi" or "gf.cell"
            let imported_name = &call_name[..dot_pos];
            let function_name = &call_name[dot_pos + 1..];

            // First check if this is an alias in the module's aliases
            if let Some(resolved_module) = current_module_info.aliases.get(imported_name) {
                // This is an aliased import, use the resolved module path
                if let Some(resolved) = Self::resolve_function_in_module(
                    function_name,
                    resolved_module,
                    all_functions,
                    all_modules,
                ) {
                    return Some(resolved);
                } else {
                    // If we can't find the exact function, at least provide the resolved module path
                    return Some(format!("{}.{}", resolved_module, function_name));
                }
            }

            // Fall back to the original import resolution logic
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

            // Check if the function_name contains dots and might involve aliases within this module
            if function_name.contains('.') {
                let parts: Vec<&str> = function_name.split('.').collect();
                if parts.len() >= 2 {
                    let first_part = parts[0];
                    let remaining_parts = parts[1..].join(".");

                    // Check if the first part is an alias in this module
                    if let Some(alias_target) = module_info.aliases.get(first_part) {
                        // Recursively resolve in the alias target module
                        return Self::resolve_function_in_module(
                            &remaining_parts,
                            alias_target,
                            all_functions,
                            all_modules,
                        );
                    }
                }
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

    fn resolve_yaml_call_static(
        call_name: &str,
        all_functions: &[FunctionInfo],
        lib_paths: &IndexMap<String, PathBuf>,
    ) -> Option<String> {
        // For YAML calls, iterate through lib_paths in order to find the first match
        // This ensures resolution priority based on the order of lib_paths

        for (prefix, _) in lib_paths {
            let candidates: Vec<&FunctionInfo> = all_functions
                .iter()
                .filter(|func_info| func_info.module.starts_with(prefix))
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
        }

        None
    }

    fn extract_component_gets_from_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                self.extract_component_gets_from_expr(&expr_stmt.value);
            }
            Stmt::Assign(assign_stmt) => {
                self.extract_component_gets_from_expr(&assign_stmt.value);
            }
            Stmt::Return(return_stmt) => {
                if let Some(value) = &return_stmt.value {
                    self.extract_component_gets_from_expr(value);
                }
            }
            Stmt::If(if_stmt) => {
                self.extract_component_gets_from_expr(&if_stmt.test);
                for s in &if_stmt.body {
                    self.extract_component_gets_from_stmt(s);
                }
                for s in &if_stmt.elif_else_clauses {
                    for stmt in &s.body {
                        self.extract_component_gets_from_stmt(stmt);
                    }
                }
            }
            Stmt::For(for_stmt) => {
                self.extract_component_gets_from_expr(&for_stmt.iter);
                for s in &for_stmt.body {
                    self.extract_component_gets_from_stmt(s);
                }
            }
            _ => {}
        }
    }

    fn extract_component_gets_from_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call(call_expr) => {
                // Check if this is a get_component call
                if let Some(func_name) = self.get_function_name(&call_expr.func) {
                    if func_name == "gf.get_component" || func_name == "get_component" {
                        // Extract the first argument (component name)
                        if let Some(first_arg) = call_expr.arguments.args.first() {
                            let current_module = self.derive_module(
                                &self.current_file_path,
                                &self.lib_paths.get(&self.current_prefix).unwrap(),
                                &self.current_prefix,
                            );
                            let resolved_component_name =
                                self.resolve_component_argument(first_arg, &current_module);
                            let component_get_info = resolved_component_name;
                            self.current_function_component_gets
                                .insert(component_get_info);
                        }
                    }
                }

                // Recursively process the function being called
                self.extract_component_gets_from_expr(&call_expr.func);

                // Process arguments
                for arg in &call_expr.arguments.args {
                    self.extract_component_gets_from_expr(arg);
                }
            }
            Expr::Attribute(attr_expr) => {
                // Recursively process the value part of the attribute access
                self.extract_component_gets_from_expr(&attr_expr.value);
            }
            Expr::BinOp(binop_expr) => {
                self.extract_component_gets_from_expr(&binop_expr.left);
                self.extract_component_gets_from_expr(&binop_expr.right);
            }
            Expr::List(list_expr) => {
                for elt in &list_expr.elts {
                    self.extract_component_gets_from_expr(elt);
                }
            }
            Expr::Tuple(tuple_expr) => {
                for elt in &tuple_expr.elts {
                    self.extract_component_gets_from_expr(elt);
                }
            }
            _ => {}
        }
    }

    fn get_string_literal(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::StringLiteral(string_expr) => Some(string_expr.value.to_string()),
            _ => None,
        }
    }

    fn get_variable_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name_expr) => Some(name_expr.id.to_string()),
            Expr::Attribute(_attr_expr) => {
                // Handle dotted names like cells.pad
                self.get_function_name(expr)
            }
            _ => None,
        }
    }

    fn detect_constant_assignments(&mut self, assign_stmt: &ruff_python_ast::StmtAssign) {
        // Check if this is a simple assignment to a string literal
        if let Some(string_value) = self.get_string_literal(&assign_stmt.value) {
            // Extract the variable names being assigned to
            for target in &assign_stmt.targets {
                if let Some(var_names) = self.extract_assignment_targets(target) {
                    let current_module = self.derive_module(
                        &self.current_file_path,
                        &self.lib_paths.get(&self.current_prefix).unwrap(),
                        &self.current_prefix,
                    );
                    for var_name in var_names {
                        self.add_constant_to_module(&current_module, &var_name, &string_value);
                    }
                }
            }
        } else if let Some(var_value) = self.get_variable_name(&assign_stmt.value) {
            // Check if this is a simple assignment to a variable/module (like c = components)
            // Add these as aliases since they're module references, not string constants
            for target in &assign_stmt.targets {
                if let Some(var_names) = self.extract_assignment_targets(target) {
                    let current_module = self.derive_module(
                        &self.current_file_path,
                        &self.lib_paths.get(&self.current_prefix).unwrap(),
                        &self.current_prefix,
                    );
                    for var_name in var_names {
                        // For module assignments like "c = components", we need to resolve the full path
                        let full_path = if var_value.contains('.') {
                            // If it's already a dotted path, use as-is
                            var_value.clone()
                        } else {
                            // If it's a simple name, assume it's relative to current module
                            format!("{}.{}", current_module, var_value)
                        };
                        self.add_alias_to_module(&current_module, &var_name, &full_path);
                    }
                }
            }
        }
    }

    fn expr_to_json_value(&self, expr: &ruff_python_ast::Expr) -> serde_json::Value {
        use ruff_python_ast::Expr;

        match expr {
            // String literals
            Expr::StringLiteral(string_lit) => {
                serde_json::Value::String(string_lit.value.to_string())
            }
            // Number literals
            Expr::NumberLiteral(num_lit) => {
                match &num_lit.value {
                    ruff_python_ast::Number::Int(int_val) => {
                        if let Some(i) = int_val.as_i64() {
                            serde_json::Value::Number(serde_json::Number::from(i))
                        } else {
                            // Fallback to string for very large integers
                            serde_json::Value::String(int_val.to_string())
                        }
                    }
                    ruff_python_ast::Number::Float(float_val) => {
                        if let Some(f) = serde_json::Number::from_f64(*float_val) {
                            serde_json::Value::Number(f)
                        } else {
                            serde_json::Value::String(float_val.to_string())
                        }
                    }
                    ruff_python_ast::Number::Complex { real, imag } => {
                        // Complex numbers as strings since JSON doesn't support them natively
                        serde_json::Value::String(format!("({real}+{imag}j)"))
                    }
                }
            }
            // Boolean literals
            Expr::BooleanLiteral(bool_lit) => serde_json::Value::Bool(bool_lit.value),
            // None literal
            Expr::NoneLiteral(_) => serde_json::Value::Null,
            // Variable names and other expressions as strings
            _ => {
                if let Some(var_name) = self.get_variable_name(expr) {
                    serde_json::Value::String(var_name)
                } else {
                    serde_json::Value::String("<unknown>".to_string())
                }
            }
        }
    }

    fn extract_parameter_defaults(
        &self,
        parameters: &ruff_python_ast::Parameters,
    ) -> std::collections::HashMap<String, serde_json::Value> {
        let mut defaults = std::collections::HashMap::new();

        // Extract defaults from regular args with defaults
        for arg in &parameters.args {
            if let Some(default_expr) = &arg.default {
                let param_name = arg.parameter.name.to_string();
                let default_value = self.expr_to_json_value(default_expr);
                defaults.insert(param_name, default_value);
            }
        }

        // Extract defaults from keyword-only args
        for arg in &parameters.kwonlyargs {
            if let Some(default_expr) = &arg.default {
                let param_name = arg.parameter.name.to_string();
                let default_value = self.expr_to_json_value(default_expr);
                defaults.insert(param_name, default_value);
            }
        }

        defaults
    }

    fn resolve_component_argument(&self, arg_expr: &Expr, current_module: &str) -> String {
        // First try direct resolution
        if let Some(string_literal) = self.get_string_literal(arg_expr) {
            return string_literal;
        }

        if let Some(var_name) = self.get_variable_name(arg_expr) {
            // Try to resolve from current function parameter defaults
            if let Some(default_value) = self.current_function_defaults.get(&var_name) {
                return self.resolve_component_argument_recursively(default_value, current_module);
            }

            // Try to resolve from module constants
            if let Some(module_info) = self.modules.get(current_module) {
                if let Some(constant_value) = module_info.constants.get(&var_name) {
                    return constant_value.clone();
                }
            }

            // Return the variable name if we can't resolve it
            return var_name;
        }

        "unknown".to_string()
    }

    fn resolve_component_argument_recursively(
        &self,
        value: &serde_json::Value,
        current_module: &str,
    ) -> String {
        match value {
            // If it's already a string, return it unquoted
            serde_json::Value::String(s) => s.clone(),

            // For other JSON values, try to resolve as variable names
            _ => {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string().trim_matches('"').to_string(),
                };

                // Try to resolve as a module constant
                if let Some(module_info) = self.modules.get(current_module) {
                    if let Some(constant_value) = module_info.constants.get(&value_str) {
                        return constant_value.clone();
                    }
                }

                // Return as variable name if we can't resolve further
                value_str
            }
        }
    }

    fn detect_partial_assignments(&mut self, assign_stmt: &ruff_python_ast::StmtAssign) {
        // Check if the assignment value is a call to functools.partial or partial
        if let Expr::Call(call_expr) = assign_stmt.value.as_ref() {
            if let Some(func_name) = self.get_function_name(&call_expr.func) {
                // Check if it's a partial call
                if func_name == "functools.partial" || func_name == "partial" {
                    // Extract the variable names being assigned to
                    for target in &assign_stmt.targets {
                        if let Some(var_names) = self.extract_assignment_targets(target) {
                            let current_module = self.derive_module(
                                &self.current_file_path,
                                &self.lib_paths.get(&self.current_prefix).unwrap(),
                                &self.current_prefix,
                            );
                            for var_name in var_names {
                                // Get the first argument of partial (the function being wrapped)
                                if let Some(wrapped_func) = call_expr.arguments.args.first() {
                                    if let Some(wrapped_func_name) =
                                        self.get_function_name(wrapped_func)
                                    {
                                        // Extract positional arguments (skip the first one which is the function)
                                        let mut args = Vec::new();
                                        for arg in call_expr.arguments.args.iter().skip(1) {
                                            args.push(self.expr_to_json_value(arg));
                                        }

                                        // Extract keyword arguments
                                        let mut kwargs = HashMap::new();
                                        for keyword in &call_expr.arguments.keywords {
                                            if let Some(arg_name) = &keyword.arg {
                                                let value = self.expr_to_json_value(&keyword.value);
                                                kwargs.insert(arg_name.to_string(), value);
                                            }
                                        }

                                        self.add_partial_to_module(
                                            &current_module,
                                            &var_name,
                                            &wrapped_func_name,
                                            args,
                                            kwargs,
                                        );
                                        let partial_info = FunctionInfo {
                                            name: var_name.clone(),
                                            module: current_module.clone(),
                                            line: assign_stmt.range.start().to_usize(),
                                            calls: IndexSet::from([wrapped_func_name]),
                                            decorators: IndexSet::new(),
                                            resolved_calls: IndexSet::new(),
                                            resolved_decorators: IndexSet::new(),
                                            parameter_defaults: HashMap::new(),
                                            component_gets: IndexSet::new(),
                                            resolved_component_gets: IndexSet::new(),
                                            is_partial: true,
                                            return_annotation: None, // Partials don't have their own return annotations
                                            resolved_return_annotation: None,
                                        };
                                        self.functions.push(partial_info);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn extract_assignment_targets(&self, target: &Expr) -> Option<Vec<String>> {
        match target {
            Expr::Name(name_expr) => Some(vec![name_expr.id.to_string()]),
            Expr::Tuple(tuple_expr) => {
                let mut names = Vec::new();
                for elt in &tuple_expr.elts {
                    if let Expr::Name(name_expr) = elt {
                        names.push(name_expr.id.to_string());
                    }
                }
                if names.is_empty() { None } else { Some(names) }
            }
            Expr::List(list_expr) => {
                let mut names = Vec::new();
                for elt in &list_expr.elts {
                    if let Expr::Name(name_expr) = elt {
                        names.push(name_expr.id.to_string());
                    }
                }
                if names.is_empty() { None } else { Some(names) }
            }
            _ => None,
        }
    }
}
