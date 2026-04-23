use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub const ADDR: &str = "unix:///tmp/buildkitd.sock";
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

static ACTIVE: AtomicU64 = AtomicU64::new(0);
static LAST_USED: AtomicU64 = AtomicU64::new(0);
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(Default::default);
static CHILD: LazyLock<Mutex<Option<Child>>> = LazyLock::new(Default::default);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Relaxed);
        LAST_USED.store(now_secs(), Relaxed);
    }
}

pub async fn ensure_running() -> anyhow::Result<Guard> {
    let _lock = LOCK.lock().await;
    ACTIVE.fetch_add(1, Relaxed);
    LAST_USED.store(now_secs(), Relaxed);

    if !is_ready().await {
        start().await.map_err(|e| {
            ACTIVE.fetch_sub(1, Relaxed);
            e
        })?;
    }

    Ok(Guard)
}

async fn is_ready() -> bool {
    Command::new("buildctl")
        .args(["--addr", ADDR, "debug", "workers"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn kill() {
    let mut child = CHILD.lock().await;
    if let Some(ref mut c) = *child {
        let _ = c.kill().await;
        let _ = c.wait().await;
    }
    *child = None;
}

async fn start() -> anyhow::Result<()> {
    if is_ready().await {
        return Ok(());
    }

    let child = Command::new("buildkitd")
        .args(["--addr", ADDR, "--oci-worker-binary", "/usr/local/bin/runc"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn buildkitd: {e}"))?;

    *CHILD.lock().await = Some(child);

    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if is_ready().await {
            return Ok(());
        }
    }

    kill().await;
    anyhow::bail!("buildkitd started but never became ready within 10s")
}

/// Spawn once at app startup.
pub fn start_idle_reaper() {
    tokio::spawn(async {
        loop {
            tokio::time::sleep(IDLE_TIMEOUT).await;
            let _lock = LOCK.lock().await;
            if ACTIVE.load(Relaxed) == 0
                && LAST_USED.load(Relaxed) > 0
                && now_secs() - LAST_USED.load(Relaxed) >= IDLE_TIMEOUT.as_secs()
            {
                kill().await;
            }
        }
    });
}
