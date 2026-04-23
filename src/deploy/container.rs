use std::collections::HashMap;
use std::os::fd::OwnedFd;
use std::sync::{LazyLock, Mutex};

use tokio::io::{AsyncBufReadExt as _, BufReader};

use crate::common::podman;
use crate::deploy::env::{EnvMode, inject_env};
use crate::deploy::shell::run_logged;
use crate::models::{App, Deployment};
use crate::networking;

static TAILERS: LazyLock<Mutex<HashMap<i64, tokio::task::AbortHandle>>> =
    LazyLock::new(Default::default);

pub fn name(app_id: i64) -> String {
    format!("app-{app_id}")
}

pub async fn start(deployment: &Deployment, log_to: Option<i64>) -> Result<(), String> {
    let deploy_id = deployment.id;
    let app_id = deployment.app_id;
    let app_name = deployment.app_name().await.map_err(|e| e.to_string())?;
    let container_name = deployment.container_name();
    let project_id = deployment
        .project_id()
        .await
        .map_err(|e| format!("project not found: {e}"))?;

    let net = networking::project_net(project_id);
    let _ = podman()
        .args(["stop", "--time", "60", &container_name])
        .output()
        .await;
    stop_log_tailer(app_id, Some(deploy_id));
    let _ = podman().args(["rm", &container_name]).output().await;

    let mut args: Vec<String> = [
        "run",
        "-d",
        "--init",
        "--name",
        &container_name,
        "--restart",
        "unless-stopped",
        "--network",
        &net,
        "--network-alias",
        &format!("{app_name}.internal"),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let volume_path = &deployment.volume_path;
    if !volume_path.is_empty() {
        let vol_name = format!("app-{app_id}-data");
        args.extend(["-v".into(), format!("{vol_name}:{volume_path}")]);
    }

    args.extend(inject_env(&deployment.env_vars, EnvMode::Runtime));
    args.extend([
        "--workdir".into(),
        "/app".into(),
        deployment.image_tag().into(),
    ]);

    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match log_to {
        Some(id) => run_logged(id, "podman", &refs, ".").await?,
        None => {
            let o = podman()
                .args(&refs)
                .output()
                .await
                .map_err(|e| format!("podman run: {e}"))?;
            if !o.status.success() {
                return Err(format!(
                    "podman run: {}",
                    String::from_utf8_lossy(&o.stderr)
                ));
            }
        }
    }

    if App::containers_enabled(app_id).await {
        start_log_tailer(app_id, deploy_id);
    }

    Ok(())
}

pub fn start_log_tailer(app_id: i64, deploy_id: i64) {
    let mut tailers = TAILERS.lock().unwrap();
    if tailers.contains_key(&app_id) {
        return;
    }

    let handle = tokio::spawn(async move {
        let _ = crate::models::ContainerLog::append(
            deploy_id,
            "--- container started ---",
            chrono::Utc::now().timestamp(),
        )
        .await;

        let cname = name(app_id);
        let (reader, writer) = match std::io::pipe() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(app_id, "pipe creation failed: {e}");
                TAILERS.lock().unwrap().remove(&app_id);
                return;
            }
        };

        let mut child = match podman()
            .args(["logs", "-f", "--timestamps", &cname])
            .stdout(writer.try_clone().expect("pipe stdout"))
            .stderr(writer)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(app_id, "log tailer spawn failed: {e}");
                TAILERS.lock().unwrap().remove(&app_id);
                return;
            }
        };

        let owned_fd: OwnedFd = reader.into();
        let std_file = std::fs::File::from(owned_fd);
        let async_file = tokio::fs::File::from_std(std_file);
        let mut lines = BufReader::new(async_file).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let (ts, text) = parse_log_line(&line);
            let _ = crate::models::ContainerLog::append(deploy_id, text, ts).await;
        }

        let _ = child.wait().await;
        TAILERS.lock().unwrap().remove(&app_id);
    });

    tailers.insert(app_id, handle.abort_handle());
}

pub fn stop_log_tailer(app_id: i64, deploy_id: Option<i64>) {
    if let Some(handle) = TAILERS.lock().unwrap().remove(&app_id) {
        handle.abort();
        if let Some(did) = deploy_id {
            tokio::spawn(async move {
                let _ = crate::models::ContainerLog::append(
                    did,
                    "--- container stopped ---",
                    chrono::Utc::now().timestamp(),
                )
                .await;
            });
        }
    }
}

fn parse_log_line(line: &str) -> (i64, &str) {
    if let Some((ts_str, rest)) = line.split_once(' ') {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts_str.trim()) {
            return (dt.timestamp(), rest);
        }
    }
    (chrono::Utc::now().timestamp(), line)
}
