pub mod builder;
pub mod py;
pub mod schema;
pub mod walk;
pub mod yaml;

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};
#[cfg(feature = "python")]
use std::path::Path;

#[cfg(feature = "python")]
use crate::builder::CallGraphBuilder;
#[cfg(feature = "python")]
use crate::walk::find_analyzable_files;

#[cfg(feature = "python")]
/// Generate a call graph from the given library paths
///
/// Args:
///     lib_paths: List of directory paths to analyze
///     prefix: Optional prefix for YAML function resolution
///     function_filter: Optional function name to filter results
///     select_path: Optional colon-separated path to select from results
///
/// Returns:
///     Dictionary containing the call graph data
#[pyfunction]
#[pyo3(signature = (lib_paths, prefix=None, function_filter=None, select_path=None))]
fn generate_call_graph(
    py: Python,
    lib_paths: Vec<String>,
    prefix: Option<String>,
    function_filter: Option<String>,
    select_path: Option<String>,
) -> PyResult<PyObject> {
    // Convert string paths to Path objects
    let paths: Vec<&Path> = lib_paths.iter().map(|p| Path::new(p)).collect();

    // Build the call graph
    let mut builder = CallGraphBuilder::new();

    for lib_path in &paths {
        match find_analyzable_files(lib_path) {
            Ok(files) => {
                for file_path in files {
                    if let Err(e) = builder.analyze_file(&file_path, lib_path) {
                        // Log error but continue processing other files
                        eprintln!("Error processing {}: {}", file_path.display(), e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error finding files in {}: {}", lib_path.display(), e);
            }
        }
    }

    let call_graph = builder.build_callgraph(&prefix);

    // Convert to serde_json::Value first
    let mut json_value = serde_json::to_value(&call_graph).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization error: {}", e))
    })?;

    // Apply function filter if specified
    if let Some(func_name) = function_filter {
        if let Some(functions_obj) = json_value.get_mut("functions") {
            if let Some(functions_map) = functions_obj.as_object_mut() {
                // Try to find function by resolved name or simple name
                let matching_key = functions_map
                    .keys()
                    .find(|key| {
                        key == &&func_name || // Exact match with resolved name
                        key.ends_with(&format!(".{}", func_name)) // Match simple name
                    })
                    .cloned();

                if let Some(key) = matching_key {
                    let func_info = functions_map.get(&key).unwrap().clone();
                    functions_map.clear();
                    functions_map.insert(key, func_info);
                } else {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Function '{}' not found in the call graph",
                        func_name
                    )));
                }
            }
        }
    }

    // Apply select path if specified
    if let Some(path) = select_path {
        json_value = extract_json_path(&json_value, &path);
    }

    // Convert serde_json::Value to Python object
    json_value_to_python(py, &json_value)
}

#[cfg(feature = "python")]
/// Extract a nested value from JSON using a colon-separated path
fn extract_json_path(value: &serde_json::Value, path: &str) -> serde_json::Value {
    let parts: Vec<&str> = path.split(':').collect();
    let mut current = value;

    for part in parts {
        match current {
            serde_json::Value::Object(map) => {
                if let Some(next_value) = map.get(part) {
                    current = next_value;
                } else {
                    return serde_json::Value::Null;
                }
            }
            _ => return serde_json::Value::Null,
        }
    }

    current.clone()
}

#[cfg(feature = "python")]
/// Convert serde_json::Value to Python object
fn json_value_to_python(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        }
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Array(arr) => {
            let py_list = PyList::empty_bound(py);
            for item in arr {
                py_list.append(json_value_to_python(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        }
        serde_json::Value::Object(obj) => {
            let py_dict = PyDict::new_bound(py);
            for (key, val) in obj {
                py_dict.set_item(key, json_value_to_python(py, val)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

#[cfg(feature = "python")]
/// Python module definition
#[pymodule]
fn callgraph(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(generate_call_graph, m)?)?;
    Ok(())
}
