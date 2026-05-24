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
