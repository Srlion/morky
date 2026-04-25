use std::path::{Path, PathBuf};

use maw::prelude::*;
use tokio::io::AsyncReadExt;

use crate::models::{App, AppStatus};

pub fn routes() -> Router {
    Router::group("/volume")
        .get("/files", list_files)
        .get("/file", download_file)
        .post("/file", upload_file)
        .delete("/file", delete_file)
        .post("/mkdir", mkdir)
}

async fn volume_root(app_id: i64) -> Option<PathBuf> {
    let vol_name = format!("app-{app_id}-data");
    let out = crate::common::podman()
        .args([
            "volume",
            "inspect",
            "--format",
            "{{.Mountpoint}}",
            &vol_name,
        ])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let p = PathBuf::from(path);
    Some(p.canonicalize().unwrap_or(p))
}

fn safe_join(root: &Path, rel: &str) -> Result<PathBuf, &'static str> {
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() {
        return Ok(root.to_path_buf());
    }
    let joined = root.join(rel);
    let (canonical, check) = if joined.exists() {
        let c = joined.canonicalize().unwrap_or_else(|_| joined.clone());
        (c.clone(), c)
    } else {
        let check = joined
            .ancestors()
            .skip(1)
            .find_map(|p| p.canonicalize().ok())
            .unwrap_or_else(|| joined.clone());
        (joined, check)
    };
    if !check.starts_with(root) {
        return Err("path outside volume");
    }
    Ok(canonical)
}

fn guard_stopped(status: AppStatus) -> bool {
    !matches!(status, AppStatus::Running | AppStatus::Deploying)
}

fn bad(c: &mut Ctx, msg: &str) {
    c.res
        .status(StatusCode::BAD_REQUEST)
        .json(&serde_json::json!({ "error": msg }));
}

fn forbidden(c: &mut Ctx, msg: &str) {
    c.res
        .status(StatusCode::FORBIDDEN)
        .json(&serde_json::json!({ "error": msg }));
}

fn not_found(c: &mut Ctx) {
    c.res
        .status(StatusCode::NOT_FOUND)
        .json(&serde_json::json!({ "error": "not found" }));
}

macro_rules! resolve_root {
    ($c:expr, $app_id:expr) => {{
        let app = match App::get_by_id($app_id).await {
            Ok(a) => a,
            Err(_) => return not_found($c),
        };
        if app.volume_path.is_empty() {
            return bad($c, "this app has no volume configured");
        }
        if !guard_stopped(app.status) {
            return forbidden($c, "app must be stopped to access volume");
        }
        match volume_root($app_id).await {
            Some(p) => p,
            None => return bad($c, "could not resolve volume path"),
        }
    }};
}

async fn list_files(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let rel = c
        .req
        .query_value::<String>("path")
        .unwrap_or_else(|_| "/".to_string());
    let dir = match safe_join(&root, &rel) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };

    if !dir.exists() {
        if let Err(e) = tokio::fs::create_dir_all(&dir).await {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({ "error": e.to_string() }));
            return;
        }
    }

    let mut entries = Vec::new();
    let mut rd = match tokio::fs::read_dir(&dir).await {
        Ok(r) => r,
        Err(_) => return not_found(c),
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        entries.push(serde_json::json!({
            "name": name,
            "is_dir": is_dir,
            "size": size,
        }));
    }
    entries.sort_by(|a, b| {
        let a_dir = a["is_dir"].as_bool().unwrap_or(false);
        let b_dir = b["is_dir"].as_bool().unwrap_or(false);
        b_dir.cmp(&a_dir).then(
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or("")),
        )
    });

    c.res.json(&serde_json::json!({ "entries": entries }));
}

async fn download_file(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let rel = match c.req.query_value::<String>("path") {
        Ok(p) => p,
        Err(_) => return bad(c, "missing path"),
    };
    let file_path = match safe_join(&root, &rel) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };
    if !file_path.is_file() {
        return not_found(c);
    }

    let mut f = match tokio::fs::File::open(&file_path).await {
        Ok(f) => f,
        Err(_) => return not_found(c),
    };
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).await.is_err() {
        c.res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({ "error": "read failed" }));
        return;
    }

    let fname = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    c.res
        .header((
            "content-disposition",
            &format!("attachment; filename=\"{fname}\""),
        ))
        .header(("content-type", "application/octet-stream"))
        .send(buf);
}

async fn upload_file(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let dir_rel = c
        .req
        .query_value::<String>("path")
        .unwrap_or_else(|_| "/".to_string());
    let dir_path = match safe_join(&root, &dir_rel) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };

    let mut mp = match c.req.multipart() {
        Ok(m) => m,
        Err(_) => return bad(c, "expected multipart body"),
    };

    let mut saved = 0usize;
    while let Ok(Some(field)) = mp.next_field().await {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .or_else(|| field.name().map(|s| s.to_string()))
            .unwrap_or_else(|| "upload".to_string());

        let filename = PathBuf::from(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "upload".to_string());

        let dest = dir_path.join(&filename);
        if let Some(parent) = dest.parent() {
            let parent_canon = parent.canonicalize().unwrap_or(parent.to_path_buf());
            if !parent_canon.starts_with(&root) {
                return bad(c, "invalid path");
            }
        }

        let data = match field.bytes().await {
            Ok(b) => b,
            Err(_) => return bad(c, "failed to read field"),
        };
        if let Err(e) = tokio::fs::write(&dest, &data).await {
            tracing::error!("write failed: {e} | dest: {:?}", dest);
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "write failed"}));
            return;
        }
        saved += 1;
    }

    c.res
        .json(&serde_json::json!({ "ok": true, "saved": saved }));
}

async fn delete_file(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let rel = match c.req.query_value::<String>("path") {
        Ok(p) => p,
        Err(_) => return bad(c, "missing path"),
    };
    let target = match safe_join(&root, &rel) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };
    if !target.exists() {
        return not_found(c);
    }

    let result = if target.is_dir() {
        tokio::fs::remove_dir_all(&target).await
    } else {
        tokio::fs::remove_file(&target).await
    };

    match result {
        Ok(_) => c.res.json(&serde_json::json!({ "ok": true })),
        Err(e) => {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({ "error": e.to_string() }));
        }
    }
}

async fn mkdir(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let rel = match c.req.query_value::<String>("path") {
        Ok(p) => p,
        Err(_) => return bad(c, "missing path"),
    };
    let target = root.join(rel.trim_start_matches('/'));
    let parent = target
        .parent()
        .and_then(|p| p.canonicalize().ok())
        .unwrap_or_default();
    if !parent.starts_with(&root) {
        return bad(c, "path outside volume");
    }

    match tokio::fs::create_dir_all(&target).await {
        Ok(_) => c.res.json(&serde_json::json!({ "ok": true })),
        Err(e) => {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({ "error": e.to_string() }));
        }
    }
}
