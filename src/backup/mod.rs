use maw::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

use crate::common::podman;
use crate::db::conn;
use crate::jobs::{self, Job};

const BACKUP_DIR: &str = "/tmp/morky-backup";

pub fn register_jobs() {
    jobs::register::<BackupJob>();
}

pub fn routes() -> Router {
    Router::group("/backup")
        .post("/create", create_backup)
        .get("/download", download_backup)
        .get("/status", backup_status)
}

async fn create_backup(c: &mut Ctx) {
    match jobs::enqueue(&BackupJob {}).await {
        Ok(Some(id)) => c.res.json(&serde_json::json!({"ok": true, "job_id": id})),
        Ok(None) => c
            .res
            .status(StatusCode::CONFLICT)
            .json(&serde_json::json!({"error": "a backup is already in progress"})),
        Err(e) => {
            tracing::error!("enqueue backup: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to start backup"}));
        }
    }
}

async fn backup_status(c: &mut Ctx) {
    let has_zip = Path::new(&format!("{BACKUP_DIR}/backup.tar.gz")).exists();

    // find latest backup job
    let job: Option<(String, Option<String>)> = conn()
        .query_as(
            "SELECT status, error FROM jobs WHERE name = 'backup' ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional()
        .await
        .unwrap_or(None);

    let (status, error) = job.unwrap_or(("none".into(), None));

    c.res.json(&serde_json::json!({
        "has_backup": has_zip,
        "job_status": status,
        "error": error,
    }));
}

async fn download_backup(c: &mut Ctx) {
    let zip_path = format!("{BACKUP_DIR}/backup.tar.gz");
    if !Path::new(&zip_path).exists() {
        return c
            .res
            .status(StatusCode::NOT_FOUND)
            .json(&serde_json::json!({"error": "no backup available, create one first"}));
    }

    let res = c
        .res
        .header(("Content-Type", "application/gzip"))
        .header((
            "Content-Disposition",
            &format!(
                "attachment; filename=\"morky-backup-{}.tar.gz\"",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ),
        ))
        .send_file(&zip_path)
        .await;

    if let Err(e) = res {
        tracing::error!("send backup file: {e}");
        c.res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({"error": "failed to send backup file"}));
    }
}

#[derive(Serialize, Deserialize)]
pub struct BackupJob {}

impl fmt::Display for BackupJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Backing up")
    }
}

impl Job for BackupJob {
    const NAME: &'static str = "backup";
    const MAX_RETRIES: i32 = 0;
    const CPU_BOUND: bool = true;
    const EXCLUSIVE: bool = true;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        do_backup().await
    }
}

async fn do_backup() -> Result<(), String> {
    let _ = fs::remove_dir_all(BACKUP_DIR).await;
    fs::create_dir_all(BACKUP_DIR)
        .await
        .map_err(|e| format!("mkdir: {e}"))?;

    // 1. backup sqlite via VACUUM INTO
    let db_backup_path = format!("{BACKUP_DIR}/database.db");
    conn()
        .query(&format!("VACUUM INTO '{db_backup_path}'"))
        .execute()
        .await
        .map_err(|e| format!("vacuum into: {e}"))?;

    // 2. gather all apps
    let apps: Vec<i64> = conn()
        .query_as("SELECT id FROM apps")
        .fetch_all()
        .await
        .unwrap_or_default();

    // pause running containers
    let mut paused: Vec<String> = Vec::new();
    for app_id in &apps {
        let name = format!("app-{app_id}");
        if is_container_running(&name).await {
            if let Ok(o) = podman().args(["pause", &name]).output().await {
                if o.status.success() {
                    paused.push(name);
                }
            }
        }
    }

    // backup volumes, then always unpause
    let vol_result = backup_volumes(&apps).await;

    for name in &paused {
        if let Ok(o) = podman().args(["unpause", name]).output().await {
            if !o.status.success() {
                tracing::error!(
                    name,
                    "unpause failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                );
            }
        } else {
            tracing::error!(name, "unpause command failed");
        }
    }

    vol_result?;

    // 3. tar
    let tar_path = format!("{BACKUP_DIR}/backup.tar.gz");

    let mut tar_args = vec!["-czf", &tar_path, "database.db"];
    let volumes_dir = format!("{BACKUP_DIR}/volumes");
    if Path::new(&volumes_dir).exists() {
        tar_args.push("volumes");
    }

    let o = Command::new("tar")
        .args(&tar_args)
        .current_dir(BACKUP_DIR)
        .output()
        .await
        .map_err(|e| format!("tar: {e}"))?;

    if !o.status.success() {
        return Err(format!(
            "tar failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ));
    }

    Ok(())
}

async fn backup_volumes(apps: &[i64]) -> Result<(), String> {
    let volumes_dir = format!("{BACKUP_DIR}/volumes");
    fs::create_dir_all(&volumes_dir)
        .await
        .map_err(|e| format!("mkdir volumes: {e}"))?;

    for app_id in apps {
        let container_name = format!("app-{app_id}");
        let volumes = get_container_volumes(&container_name).await;
        if volumes.is_empty() {
            continue;
        }

        let app_vol_dir = format!("{volumes_dir}/{container_name}");
        fs::create_dir_all(&app_vol_dir)
            .await
            .map_err(|e| format!("mkdir {container_name}: {e}"))?;

        for vol in &volumes {
            let tar_name = format!("{}/{}.tar.gz", app_vol_dir, sanitize_vol_name(vol));
            let o = podman()
                .args(["volume", "export", vol, "--output", &tar_name])
                .output()
                .await
                .map_err(|e| format!("export volume {vol}: {e}"))?;

            if !o.status.success() {
                tracing::warn!(
                    vol,
                    "volume export failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                );
            }
        }
    }

    Ok(())
}

async fn is_container_running(name: &str) -> bool {
    podman()
        .args(["inspect", "--format", "{{.State.Running}}", name])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

async fn get_container_volumes(container_name: &str) -> Vec<String> {
    let o = match podman()
        .args([
            "inspect",
            "--format",
            "{{range .Mounts}}{{if eq .Type \"volume\"}}{{.Name}}\n{{end}}{{end}}",
            container_name,
        ])
        .output()
        .await
    {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn sanitize_vol_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
