use std::path::Path;

use anyhow::Context;

/// Docker bind mounts require an absolute host path; relative paths like `./data/...` fail.
pub fn absolute_bind_path(path: impl AsRef<Path>) -> Result<String, anyhow::Error> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path).with_context(|| {
            format!(
                "failed to create path for docker bind mount: {}",
                path.display()
            )
        })?;
    }
    let abs = path
        .canonicalize()
        .with_context(|| format!("failed to resolve absolute bind path: {}", path.display()))?;
    Ok(abs.to_string_lossy().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_missing_directory_before_canonicalize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/pages");
        let abs = absolute_bind_path(&path).unwrap();
        assert!(Path::new(&abs).is_dir());
    }

    #[test]
    fn existing_path_canonicalizes() {
        let dir = tempfile::tempdir().unwrap();
        let abs = absolute_bind_path(dir.path()).unwrap();
        assert!(abs.starts_with('/'));
    }
}
