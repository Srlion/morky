use std::collections::HashMap;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::log_broadcast;

pub async fn run_logged(deploy_id: i64, cmd: &str, args: &[&str], cwd: &str) -> Result<(), String> {
    run_logged_with_env(deploy_id, cmd, args, cwd, &HashMap::new()).await
}

pub async fn run_logged_with_env(
    deploy_id: i64,
    cmd: &str,
    args: &[&str],
    cwd: &str,
    env: &HashMap<String, String>,
) -> Result<(), String> {
    log_broadcast::append_log(deploy_id, &format!("$ {cmd} {}", args.join(" "))).await;

    let mut command = Command::new(cmd);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(env);

    let mut child = command.spawn().map_err(|e| format!("spawn `{cmd}`: {e}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let id = deploy_id;
    let h1 = tokio::spawn(async move {
        if let Some(r) = stdout {
            let mut lines = BufReader::new(r).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                log_broadcast::append_log(id, &l).await;
            }
        }
    });

    let id2 = deploy_id;
    let h2 = tokio::spawn(async move {
        if let Some(r) = stderr {
            let mut lines = BufReader::new(r).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                log_broadcast::append_log(id2, &l).await;
            }
        }
    });

    let status = child.wait().await.map_err(|e| format!("wait: {e}"))?;

    let _ = (h1.await, h2.await);

    if !status.success() {
        return Err(format!(
            "`{cmd}` exited with code {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}
