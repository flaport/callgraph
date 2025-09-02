use crate::builder::CallGraphBuilder;
use crate::walk::find_analyzable_files;
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::PathBuf;

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (lib_paths, function_filter=None, select_path=None))]
fn generate_call_graph(
    py: Python,
    lib_paths: Bound<'_, PyDict>,
    function_filter: Option<String>,
    select_path: Option<String>,
) -> PyResult<PyObject> {
    // Iterate over dictionary items explicitly to preserve insertion order
    let mut lib_paths_map = IndexMap::new();

    // Iterate through the dictionary items to preserve Python's insertion order
    for item in lib_paths.items() {
        let (key, value) = item.extract::<(String, String)>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "lib_paths dictionary must contain string keys and string values (paths)",
            )
        })?;
        lib_paths_map.insert(key, PathBuf::from(value));
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
