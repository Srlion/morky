use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::process::Command;
use tokio::sync::Mutex;

pub const ADDR: &str = "tcp://morky-buildkit:1234";
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

static ACTIVE: AtomicU64 = AtomicU64::new(0);
static LAST_USED: AtomicU64 = AtomicU64::new(0);
static LOCK: LazyLock<Mutex<()>> = LazyLock::new(Default::default);

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

/// Returns how many CPUs to allow buildkitd to use.
/// Half of available CPUs, minimum 1:
///   2 CPUs -> 1,  3 -> 1,  4 -> 2,  8 -> 4, etc.
pub fn cpu_limit() -> usize {
    let total = num_cpus::get();
    (total / 2).max(1)
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
    crate::common::podman()
        .args(["stop", "morky-buildkit"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .ok();
}

async fn start() -> anyhow::Result<()> {
    if is_ready().await {
        return Ok(());
    }

    // Remove stale container if any
    crate::common::podman()
        .args(["rm", "-f", "morky-buildkit"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .ok();

    let cpus = cpu_limit().to_string();
    let status = crate::common::podman()
        .args([
            "run",
            "-d",
            "--name",
            "morky-buildkit",
            "--privileged",
            "--cpus",
            &cpus,
            "--network",
            "morky-haproxy-net",
            "--volume",
            "morky-buildkit.volume:/var/lib/buildkit",
            "moby/buildkit:v0.29.0",
            "--addr",
            "tcp://0.0.0.0:1234",
            "--oci-max-parallelism",
            &cpus,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("failed to start buildkit container: {e}"))?;

    if !status.success() {
        anyhow::bail!("podman run morky-buildkit failed");
    }

    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if is_ready().await {
            return Ok(());
        }
    }

    let _ = kill().await;
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
