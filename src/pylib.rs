use crate::builder::CallGraphBuilder;
use crate::walk::find_analyzable_files;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (lib_paths, function_filter=None, select_path=None))]
fn generate_call_graph(
    py: Python,
    lib_paths: PyObject,
    function_filter: Option<String>,
    select_path: Option<String>,
) -> PyResult<PyObject> {
    // Parse lib_paths - can be either a dict or a list of "prefix:path" strings
    let mut lib_paths_map = BTreeMap::new();
    
    // Try to extract as a dict first
    if let Ok(dict) = lib_paths.extract::<std::collections::HashMap<String, String>>(py) {
        for (prefix, path) in dict {
            lib_paths_map.insert(prefix, PathBuf::from(path));
        }
    } else if let Ok(list) = lib_paths.extract::<Vec<String>>(py) {
        // Fall back to list of "prefix:path" strings for backwards compatibility
        for lib_path_str in list {
            let parts: Vec<&str> = lib_path_str.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid lib_path format '{}'. Expected 'prefix:path' or a dict", lib_path_str)
                ));
            }
            let prefix = parts[0].to_string();
            let path = PathBuf::from(parts[1]);
            lib_paths_map.insert(prefix, path);
        }
    } else {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "lib_paths must be either a dict or a list of 'prefix:path' strings"
        ));
    }

    // Build the call graph
    let mut builder = CallGraphBuilder::new(lib_paths_map.clone());

    for (prefix, lib_path) in &lib_paths_map {
        match find_analyzable_files(lib_path) {
            Ok(files) => {
                for file_path in files {
                    if let Err(_) = builder.analyze_file(&file_path, lib_path, prefix) {
                        // eprintln!("Error processing {}: {}", file_path.display(), e);
                    }
                }
            }
            Err(_) => {
                // eprintln!("Error finding files in {}: {}", lib_path.display(), e);
            }
        }
    }

    let call_graph = builder.build_callgraph();

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
