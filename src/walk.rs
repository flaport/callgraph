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

            if path.extension().map_or(false, |ext| ext == "py") || file_name.ends_with(".pic.yml")
            {
                files.push(path.to_path_buf());
            }
        }
    }

    Ok(files)
}
