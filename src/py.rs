use anyhow::Context;
use log::debug;
use ruff_python_parser::parse_module;
use std::fs;
use std::path::Path;

use crate::builder::CallGraphBuilder;

/// Trait for analyzing different file types and building call graph information
pub trait FileAnalyzer {
    /// Analyze a file and add its information to the call graph builder
    fn analyze_file(
        builder: &mut CallGraphBuilder,
        file_path: &Path,
        lib_root: &Path,
    ) -> anyhow::Result<()>;
}

/// Python file analyzer
pub struct PythonAnalyzer;

impl FileAnalyzer for PythonAnalyzer {
    fn analyze_file(
        builder: &mut CallGraphBuilder,
        file_path: &Path,
        lib_root: &Path,
    ) -> anyhow::Result<()> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        // Clear imports for each new file
        builder.imports.clear();

        // First try to parse the entire module
        match parse_module(&content) {
            Ok(parsed) => {
                // Success - parse normally
                let module = parsed.into_syntax();
                for stmt in &module.body {
                    builder.visit_stmt(stmt, lib_root);
                }
                Ok(())
            }
            Err(parse_error) => {
                // Module parsing failed - try partial parsing by splitting into logical blocks
                debug!(
                    "Full module parsing failed, attempting partial parsing for {}: {:?}",
                    file_path.display(),
                    parse_error
                );

                let (statements, errors) = parse_python_partially(&content, file_path);
                let num_statements = statements.len();
                let has_errors = !errors.is_empty();

                // Process successfully parsed statements
                for stmt in &statements {
                    builder.visit_stmt(stmt, lib_root);
                }

                // Add errors to module
                let module_name = builder.derive_module(file_path, lib_root);
                for error in &errors {
                    builder.add_error_to_module(&module_name, error);
                }

                if has_errors {
                    // Return error but with partial parsing info
                    Err(anyhow::anyhow!(
                        "Failed to parse Python file {}: {:?} (partial parsing recovered {} statements)",
                        file_path.display(),
                        parse_error,
                        num_statements
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn parse_python_partially(
    content: &str,
    file_path: &Path,
) -> (Vec<ruff_python_ast::Stmt>, Vec<String>) {
    let mut statements = Vec::new();
    let mut errors = Vec::new();

    // Split content into logical blocks (functions, classes, imports, etc.)
    let blocks = split_into_blocks(content);

    for (block_content, line_start) in blocks {
        match try_parse_block(&block_content) {
            Ok(stmts) => {
                debug!(
                    "Successfully parsed block starting at line {} in {}",
                    line_start,
                    file_path.display()
                );
                statements.extend(stmts);
            }
            Err(e) => {
                let error_msg = format!("Parse error at line {}: {:?}", line_start, e);
                debug!(
                    "Failed to parse block starting at line {} in {}: {:?}",
                    line_start,
                    file_path.display(),
                    e
                );
                errors.push(error_msg);
            }
        }
    }

    (statements, errors)
}

fn split_into_blocks(content: &str) -> Vec<(String, usize)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut current_block = String::new();
    let mut block_start_line = 1;
    let mut in_block = false;
    let mut block_indent = 0;
    let mut in_decorator_sequence = false;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // Check for decorator
        if trimmed.starts_with("@") {
            if in_block {
                // Finish previous block
                if !current_block.trim().is_empty() {
                    blocks.push((current_block.clone(), block_start_line));
                }
                current_block.clear();
            }
            // Start new decorator sequence
            in_decorator_sequence = true;
            in_block = true;
            block_start_line = line_num;
            block_indent = indent;
            current_block.push_str(line);
            current_block.push('\n');
            continue;
        }

        // Check for start of new top-level block
        let is_new_block = trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("from ");

        if is_new_block {
            if in_decorator_sequence {
                // This function/class belongs to the decorator sequence - continue current block
                in_decorator_sequence = false;
                current_block.push_str(line);
                current_block.push('\n');
                continue;
            } else if in_block {
                // Finish previous block
                if !current_block.trim().is_empty() {
                    blocks.push((current_block.clone(), block_start_line));
                }
                current_block.clear();
                block_start_line = line_num;
            }

            in_block = true;
            block_indent = indent;
            current_block.push_str(line);
            current_block.push('\n');
        } else if in_block {
            // Continue current block if indented or empty line
            if indent > block_indent || line.trim().is_empty() {
                current_block.push_str(line);
                current_block.push('\n');
            } else {
                // End of block
                if !current_block.trim().is_empty() {
                    blocks.push((current_block.clone(), block_start_line));
                }
                current_block.clear();
                in_block = false;
                in_decorator_sequence = false;

                // Start new block if this line starts one
                if trimmed.starts_with("def ")
                    || trimmed.starts_with("class ")
                    || trimmed.starts_with("import ")
                    || trimmed.starts_with("from ")
                {
                    in_block = true;
                    block_start_line = line_num;
                    block_indent = indent;
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }
        } else if !line.trim().is_empty() {
            // Standalone statement
            blocks.push((line.to_string(), line_num));
        }
    }

    // Add final block
    if !current_block.trim().is_empty() {
        blocks.push((current_block, block_start_line));
    }

    blocks
}

fn try_parse_block(code: &str) -> anyhow::Result<Vec<ruff_python_ast::Stmt>> {
    // Try parsing as a small module
    match parse_module(code) {
        Ok(parsed) => {
            let module = parsed.into_syntax();
            Ok(module.body)
        }
        Err(e) => Err(anyhow::anyhow!("Parse error: {:?}", e)),
    }
}
