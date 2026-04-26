use std::collections::HashMap;
use std::os::fd::OwnedFd;
use std::sync::{LazyLock, Mutex};

use maw::CancellationToken;
use tokio::io::{AsyncBufReadExt as _, BufReader};

use crate::common::{LogErr, podman};
use crate::deploy::env::{EnvMode, inject_env};
use crate::deploy::shell::run_logged;
use crate::models::{App, ContainerLog, Deployment};
use crate::{constants, networking};

static TAILERS: LazyLock<Mutex<HashMap<i64, CancellationToken>>> = LazyLock::new(Default::default);

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
    podman()
        .args(["stop", "--time", "60", &container_name])
        .output()
        .await
        .log_err("failed to stop container");
    stop_log_tailer(app_id, Some(deploy_id)).await;
    podman()
        .args(["rm", &container_name])
        .output()
        .await
        .log_err("failed to remove container");

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
        let host_dir = format!("{}/volumes/app-{app_id}", constants::morky_host_data_dir());
        let container_dir = format!("{}/volumes/app-{app_id}", constants::morky_data_dir());
        let _ = tokio::fs::create_dir_all(&container_dir).await;
        args.extend(["-v".into(), format!("{host_dir}:{volume_path}")]);
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

async fn log_marker(deploy_id: i64, text: &str) {
    ContainerLog::append(deploy_id, text, chrono::Utc::now().timestamp())
        .await
        .log_err("failed to append container marker log");
}

pub fn start_log_tailer(app_id: i64, deploy_id: i64) {
    let mut tailers = TAILERS.lock().unwrap();
    if tailers.contains_key(&app_id) {
        return;
    }

    let token = CancellationToken::new();
    tailers.insert(app_id, token.clone());

    tokio::spawn(async move {
        log_marker(deploy_id, "--- container started ---").await;

        tail_logs(app_id, deploy_id, token)
            .await
            .log_err("log tailer failed");

        log_marker(deploy_id, "--- container stopped ---").await;
        TAILERS.lock().unwrap().remove(&app_id);
    });
}

async fn tail_logs(app_id: i64, deploy_id: i64, token: CancellationToken) -> std::io::Result<()> {
    let (reader, writer) = std::io::pipe()?;
    let mut child = podman()
        .args(["logs", "-f", "--timestamps", &name(app_id)])
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .spawn()?;

    let async_file = tokio::fs::File::from_std(std::fs::File::from(OwnedFd::from(reader)));
    let mut lines = BufReader::new(async_file).lines();

    tokio::select! {
        _ = token.cancelled() => {
            let _ = child.kill().await;
        }
        _ = async {
            while let Ok(Some(line)) = lines.next_line().await {
                let (ts, text) = parse_log_line(&line);
                ContainerLog::append(deploy_id, text, ts).await.log_err("failed to append container log");
            }
            child.wait().await.log_err("failed to wait for child");
        } => {}
    }
    Ok(())
}

pub async fn stop_log_tailer(app_id: i64, _deploy_id: Option<i64>) {
    let token = TAILERS.lock().unwrap().remove(&app_id);
    if let Some(t) = token {
        t.cancel();
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
