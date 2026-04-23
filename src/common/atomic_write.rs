use std::path::Path;

use tokio::fs;

pub async fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = path.with_extension(
        path.extension()
            .map(|e| format!("{}.tmp", e.to_string_lossy()))
            .unwrap_or_else(|| "tmp".into()),
    );
    fs::write(&tmp, contents).await?;
    fs::rename(&tmp, path).await
}
