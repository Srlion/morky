use maw::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    common::podman,
    deploy::buildkit,
    models::{CleanupSettings, CleanupSettingsUpdate},
};

#[derive(Debug, Deserialize)]
pub struct CleanupRequest {
    #[serde(default)]
    pub containers: bool,
    #[serde(default)]
    pub images: bool,
    #[serde(default)]
    pub volumes: bool,
    #[serde(default)]
    pub buildkit: bool,
    pub buildkit_keep_storage_gb: Option<f64>,
}

impl CleanupRequest {
    pub fn from_settings(s: &CleanupSettings) -> Self {
        Self {
            containers: s.clean_containers,
            images: s.clean_images,
            volumes: s.clean_volumes,
            buildkit: s.clean_buildkit,
            buildkit_keep_storage_gb: Some(s.buildkit_keep_storage_gb),
        }
    }

    fn target_names(&self) -> Vec<&'static str> {
        let mut t = Vec::new();
        if self.containers {
            t.push("containers");
        }
        if self.images {
            t.push("images");
        }
        if self.volumes {
            t.push("volumes");
        }
        if self.buildkit {
            t.push("buildkit");
        }
        t
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub targets: Vec<String>,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn run_cleanup(req: CleanupRequest) -> CleanupResult {
    let targets = req.target_names();

    if targets.is_empty() {
        return CleanupResult {
            targets: vec![],
            success: true,
            error: Some("Nothing selected to clean".into()),
        };
    }

    let mut errors = Vec::new();

    if req.containers {
        match podman().args(["container", "prune", "-f"]).output().await {
            Ok(o) if !o.status.success() => {
                errors.push(format!(
                    "containers: {}",
                    String::from_utf8_lossy(&o.stderr)
                ));
            }
            Err(e) => errors.push(format!("containers: {e}")),
            _ => {}
        }
    }

    if req.images {
        match podman().args(["image", "prune", "-af"]).output().await {
            Ok(o) if !o.status.success() => {
                errors.push(format!("images: {}", String::from_utf8_lossy(&o.stderr)));
            }
            Err(e) => errors.push(format!("images: {e}")),
            _ => {}
        }
    }

    if req.volumes {
        match podman().args(["volume", "prune", "-f"]).output().await {
            Ok(o) if !o.status.success() => {
                errors.push(format!("volumes: {}", String::from_utf8_lossy(&o.stderr)));
            }
            Err(e) => errors.push(format!("volumes: {e}")),
            _ => {}
        }
    }

    if req.buildkit {
        let _g = buildkit::ensure_running().await;
        let keep_gb = req.buildkit_keep_storage_gb.unwrap_or(2.0).max(0.0);
        let keep_bytes = format!("{}", (keep_gb * 1_073_741_824.0) as u64);

        match tokio::process::Command::new("buildctl")
            .args([
                "--addr",
                buildkit::ADDR,
                "prune",
                "--keep-storage",
                &keep_bytes,
            ])
            .output()
            .await
        {
            Ok(o) if !o.status.success() => {
                errors.push(format!("buildkit: {}", String::from_utf8_lossy(&o.stderr)));
            }
            Err(e) => errors.push(format!("buildkit: {e}")),
            _ => {}
        }
    }

    let _ = CleanupSettings::mark_cleaned().await;

    let target_names = targets.iter().map(|s| s.to_string()).collect();

    if errors.is_empty() {
        CleanupResult {
            targets: target_names,
            success: true,
            error: None,
        }
    } else {
        CleanupResult {
            targets: target_names,
            success: false,
            error: Some(errors.join("; ")),
        }
    }
}

pub fn start_auto_cleanup() {
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let settings = match CleanupSettings::get().await {
                Ok(s) => s,
                Err(_) => continue,
            };

            if !settings.auto_cleanup_enabled {
                continue;
            }

            let now = chrono::Utc::now().timestamp();
            let interval_secs = settings.cleanup_interval_hours * 3600;
            let last = settings.last_cleanup_at.unwrap_or(0);

            if now - last >= interval_secs {
                tracing::info!("auto-cleanup triggered");
                let req = CleanupRequest::from_settings(&settings);
                let result = run_cleanup(req).await;
                if result.success {
                    tracing::info!("auto-cleanup done: {:?}", result.targets);
                } else {
                    tracing::warn!("auto-cleanup errors: {:?}", result.error);
                }
            }
        }
    });
}

pub fn routes() -> Router {
    Router::group("/cleanup")
        .get("/settings", settings_handler)
        .put("/settings", update_settings_handler)
        .post("/run", run_handler)
}

async fn settings_handler(c: &mut Ctx) {
    match CleanupSettings::get().await {
        Ok(s) => c.res.json(&s),
        Err(e) => {
            c.res.status(StatusCode::INTERNAL_SERVER_ERROR);
            c.res.json(&serde_json::json!({ "error": e.to_string() }));
        }
    }
}

async fn update_settings_handler(c: &mut Ctx) {
    let body: CleanupSettingsUpdate = match c.req.json().await {
        Ok(b) => b,
        Err(e) => {
            c.res.status(StatusCode::BAD_REQUEST);
            c.res.json(&serde_json::json!({ "error": e.to_string() }));
            return;
        }
    };
    match CleanupSettings::update(body).await {
        Ok(s) => c.res.json(&s),
        Err(e) => {
            c.res.status(StatusCode::INTERNAL_SERVER_ERROR);
            c.res.json(&serde_json::json!({ "error": e.to_string() }));
        }
    }
}

async fn run_handler(c: &mut Ctx) {
    let req: CleanupRequest = match c.req.json().await {
        Ok(r) => r,
        Err(e) => {
            c.res.status(StatusCode::INTERNAL_SERVER_ERROR);
            c.res.json(&serde_json::json!({ "error": e.to_string() }));
            return;
        }
    };
    let result = run_cleanup(req).await;
    c.res.json(&result);
}
