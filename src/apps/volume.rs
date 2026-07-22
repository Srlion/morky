use std::path::{Path, PathBuf};

use maw::prelude::*;
use tokio::io::AsyncWriteExt as _;

use crate::{
    constants,
    models::{App, AppStatus},
};

pub fn routes() -> Router {
    Router::group("/volume")
        .get("/files", list_files)
        .get("/file", download_file)
        .post("/file", upload_file)
        .delete("/file", delete_file)
        .post("/mkdir", mkdir)
        .post("/move", move_entry)
}

async fn volume_root(app_id: i64) -> Option<PathBuf> {
    let data_dir = constants::morky_data_dir();
    let path = PathBuf::from(format!("{data_dir}/volumes/app-{app_id}"));
    if !path.exists() {
        tokio::fs::create_dir_all(&path).await.ok()?;
    }
    Some(path)
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
        .json(serde_json::json!({ "error": msg }));
}

fn forbidden(c: &mut Ctx, msg: &str) {
    c.res
        .status(StatusCode::FORBIDDEN)
        .json(serde_json::json!({ "error": msg }));
}

fn not_found(c: &mut Ctx) {
    c.res
        .status(StatusCode::NOT_FOUND)
        .json(serde_json::json!({ "error": "not found" }));
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

    if !dir.exists()
        && let Err(e) = tokio::fs::create_dir_all(&dir).await
    {
        c.res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(serde_json::json!({ "error": e.to_string() }));
        return;
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

    c.res.json(serde_json::json!({ "entries": entries }));
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

    let fname = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let res = c
        .res
        .header((
            "content-disposition",
            &format!("attachment; filename=\"{fname}\""),
        ))
        .header(("content-type", "application/octet-stream"))
        .send_file(&file_path)
        .await; // same API backup/mod.rs uses
    if let Err(e) = res {
        tracing::error!("send volume file: {e}");
        c.res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(serde_json::json!({"error": "read failed"}));
    }
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
    if !dir_path.exists() && tokio::fs::create_dir_all(&dir_path).await.is_err() {
        return bad(c, "could not create target directory");
    }

    let mut mp = match c.req.multipart() {
        Ok(m) => m,
        Err(_) => return bad(c, "expected multipart body"),
    };

    let mut saved = 0usize;
    while let Ok(Some(mut field)) = mp.next_field().await {
        let filename = field
            .file_name()
            .map(str::to_string)
            .or_else(|| field.name().map(str::to_string))
            .and_then(|f| {
                PathBuf::from(f)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "upload".to_string());

        let dest = dir_path.join(&filename);
        let mut file = match tokio::fs::File::create(&dest).await {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("create failed: {e} | dest: {dest:?}");
                return c
                    .res
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .json(serde_json::json!({"error": "write failed"}));
            }
        };
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    if let Err(e) = file.write_all(&chunk).await {
                        tracing::error!("write failed: {e} | dest: {dest:?}");
                        let _ = tokio::fs::remove_file(&dest).await; // don't leave partial files
                        return c
                            .res
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .json(serde_json::json!({"error": "write failed"}));
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    let _ = tokio::fs::remove_file(&dest).await;
                    return bad(c, "failed to read field");
                }
            }
        }
        saved += 1;
    }
    c.res.json(serde_json::json!({"ok": true, "saved": saved}));
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
        Ok(_) => c.res.json(serde_json::json!({ "ok": true })),
        Err(e) => {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(serde_json::json!({ "error": e.to_string() }));
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
    let target = match safe_join(&root, &rel) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };
    match tokio::fs::create_dir_all(&target).await {
        Ok(_) => c.res.json(serde_json::json!({ "ok": true })),
        Err(e) => {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    }
}

async fn move_entry(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let root = resolve_root!(c, app_id);

    let from = match c.req.query_value::<String>("from") {
        Ok(p) => p,
        Err(_) => return bad(c, "missing 'from' param"),
    };
    let to = match c.req.query_value::<String>("to") {
        Ok(p) => p,
        Err(_) => return bad(c, "missing 'to' param"),
    };

    let src = match safe_join(&root, &from) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };
    if !src.exists() {
        return not_found(c);
    }

    let dst = match safe_join(&root, &to) {
        Ok(p) => p,
        Err(e) => return bad(c, e),
    };

    // If destination is a directory, move into it keeping the original name
    let final_dst = if dst.is_dir() {
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        dst.join(name)
    } else {
        dst
    };

    if final_dst.exists() {
        return bad(c, "destination already exists");
    }

    match tokio::fs::rename(&src, &final_dst).await {
        Ok(_) => c.res.json(serde_json::json!({ "ok": true })),
        Err(e) => {
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(serde_json::json!({ "error": e.to_string() }));
        }
    }
}
