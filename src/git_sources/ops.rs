use tokio::process::Command;

pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
}

/// Clone a repo by authenticated URL (shallow, single branch).
pub async fn clone(url: &str, branch: &str, dest: &str, cwd: &str) -> Result<(), String> {
    run_cmd(
        "git",
        &["clone", "--depth", "1", "--branch", branch, url, dest],
        cwd,
    )
    .await
}

pub async fn clone_at_commit(url: &str, commit: &str, dest: &str, cwd: &str) -> Result<(), String> {
    run_cmd("git", &["clone", "--no-checkout", url, dest], cwd).await?;
    run_cmd("git", &["checkout", commit], dest).await
}

/// Read the HEAD commit from a cloned repo.
pub async fn head_commit(repo_dir: &str) -> Result<CommitInfo, String> {
    let sha = cmd_stdout("git", &["log", "-1", "--format=%H"], repo_dir).await?;
    let message = cmd_stdout("git", &["log", "-1", "--format=%s"], repo_dir).await?;
    let short_sha = sha[..sha.len().min(7)].to_string();
    Ok(CommitInfo {
        sha,
        short_sha,
        message,
    })
}

async fn run_cmd(cmd: &str, args: &[&str], cwd: &str) -> Result<(), String> {
    let o = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| format!("{cmd}: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "{cmd} failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(())
}

async fn cmd_stdout(cmd: &str, args: &[&str], cwd: &str) -> Result<String, String> {
    let o = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| format!("{cmd}: {e}"))?;
    if !o.status.success() {
        return Err(format!(
            "{cmd} failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
}
