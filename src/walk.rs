use anyhow::Context;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_analyzable_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir) {
        let entry = entry
            .with_context(|| format!("Failed to read directory entry in {}", dir.display()))?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip paths that contain dot-folders (e.g. .git, .venv)
            if path.components().any(|comp| {
                comp.as_os_str()
                    .to_str()
                    .map_or(false, |s| s.starts_with(".") && s != ".venv")
            }) {
                continue;
            }

            if path.extension().map_or(false, |ext| ext == "py") || file_name.ends_with(".pic.yml")
            {
                files.push(path.to_path_buf());
            }
        }
    }

    Ok(files)
}
