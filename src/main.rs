use anyhow::{Context, Result};
use clap::Parser;
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(
    name = "call_graph",
    version = "1.0",
    about = "Generates a call graph for a Python library"
)]
struct Args {
    /// Path to the top-level folder of the Python library
    path: PathBuf,

    /// Show only the specified function (optional)
    #[arg(short, long)]
    function: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CallGraph {
    functions: HashMap<String, FunctionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct FunctionInfo {
    name: String,
    file: String,
    line: usize,
    calls: Vec<String>,
    decorators: Vec<String>,
}

struct CallGraphBuilder {
    functions: HashMap<String, FunctionInfo>,
    current_file: String,
}

impl CallGraphBuilder {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
            current_file: String::new(),
        }
    }

    fn analyze_file(&mut self, file_path: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        self.current_file = file_path.display().to_string();

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

                let func_info = FunctionInfo {
                    name: func_name.clone(),
                    file: self.current_file.clone(),
                    line: func_def.range.start().to_usize(),
                    calls,
                    decorators,
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

                        let func_info = FunctionInfo {
                            name: full_method_name.clone(),
                            file: self.current_file.clone(),
                            line: method_def.range.start().to_usize(),
                            calls,
                            decorators,
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

                for arg in &call_expr.arguments.args {
                    self.extract_calls_from_expr(arg, calls);
                }
            }
            Expr::Attribute(attr_expr) => {
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

    fn build_call_graph(self) -> CallGraph {
        CallGraph {
            functions: self.functions,
        }
    }
}

fn find_python_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut python_files = Vec::new();

    for entry in WalkDir::new(dir) {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in {}", dir.display()))?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "py") {
            python_files.push(path.to_path_buf());
        }
    }

    Ok(python_files)
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }

    if !args.path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", args.path.display());
    }

    let python_files = find_python_files(&args.path)
        .with_context(|| format!("Failed to find Python files in {}", args.path.display()))?;

    if python_files.is_empty() {
        anyhow::bail!("No Python files found in {}", args.path.display());
    }

    let mut builder = CallGraphBuilder::new();

    for file_path in python_files {
        if let Err(e) = builder.analyze_file(&file_path) {
            eprintln!("Warning: Failed to analyze {}: {}", file_path.display(), e);
            continue;
        }
    }

    let mut call_graph = builder.build_call_graph();

    // Filter to specific function if requested
    if let Some(function_name) = &args.function {
        if let Some(func_info) = call_graph.functions.get(function_name) {
            let mut filtered_functions = HashMap::new();
            filtered_functions.insert(function_name.clone(), func_info.clone());
            call_graph.functions = filtered_functions;
        } else {
            anyhow::bail!("Function '{}' not found in the call graph", function_name);
        }
    }

    let json_output = serde_json::to_string_pretty(&call_graph)
        .context("Failed to serialize call graph to JSON")?;

    println!("{}", json_output);

    Ok(())
}
