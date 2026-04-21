use std::sync::{Arc, LazyLock};

use maw::prelude::*;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{common::podman, deploy::buildkit};

mod cleanup;

#[derive(Clone, Serialize, Default)]
pub struct SystemStats {
    pub cpu_percent: f64,
    pub mem_total_mb: u64,
    pub mem_used_mb: u64,
    pub mem_percent: f64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub disk_percent: f64,
    pub timestamp: i64,
}

const MAX_HISTORY: usize = 100;

static HISTORY: LazyLock<Mutex<Vec<SystemStats>>> = LazyLock::new(|| Mutex::new(Vec::new()));

static CONTAINER_STATS: LazyLock<Mutex<Arc<[ContainerStats]>>> =
    LazyLock::new(|| Mutex::new(Arc::new([])));

pub fn routes() -> Router {
    Router::group("/monitoring")
        .get("/stats", stats_handler)
        .get("/processes", processes_handler)
        .get("/podman-disk", podman_disk_handler)
        .push(cleanup::routes())
}

pub fn start_sampler() {
    tokio::spawn(system_sampler());
    tokio::spawn(container_sampler());
    cleanup::start_auto_cleanup();
}

async fn system_sampler() {
    use sysinfo::{Disks, System};

    let mut sys = System::new_all();
    let mut disks = Disks::new_with_refreshed_list();

    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    sys.refresh_cpu_usage();
    tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
    sys.refresh_cpu_usage();

    loop {
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        disks.refresh(true);

        let cpu = sys.global_cpu_usage() as f64;
        let mem_total = sys.total_memory();
        let mem_used = sys.used_memory();
        let mem_pct = if mem_total > 0 {
            mem_used as f64 / mem_total as f64 * 100.0
        } else {
            0.0
        };

        let (dt, du) = disks
            .iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"))
            .map(|d| (d.total_space(), d.total_space() - d.available_space()))
            .unwrap_or((0, 0));
        let gib = 1_073_741_824.0; // 1024³

        let s = SystemStats {
            cpu_percent: r1(cpu),
            mem_total_mb: (mem_total / 1_048_576) as u64, // MiB
            mem_used_mb: (mem_used / 1_048_576) as u64,   // MiB
            mem_percent: r1(mem_pct),
            disk_total_gb: r2(dt as f64 / gib),
            disk_used_gb: r2(du as f64 / gib),
            disk_percent: if dt > 0 {
                r1(du as f64 / dt as f64 * 100.0)
            } else {
                0.0
            },
            timestamp: chrono::Utc::now().timestamp(),
        };

        let mut h = HISTORY.lock().await;
        h.push(s);
        if h.len() > MAX_HISTORY {
            h.remove(0);
        }
        drop(h);

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[derive(Serialize, Clone)]
struct ContainerStats {
    name: String,
    app_id: Option<i64>,
    app_name: Option<String>,
    project_id: Option<i64>,
    project_name: Option<String>,
    cpu: String,
    mem_used: String,
    mem_percent: String,
}

async fn container_sampler() {
    use tokio::io::{AsyncBufReadExt, BufReader};

    loop {
        let mut child = match podman()
            .args(["stats", "--no-reset", "--no-trunc", "--format", "json"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("failed to spawn podman stats: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();
        let mut buf = String::new();

        // Stream continuously - podman stats keeps emitting JSON arrays
        while let Ok(Some(line)) = lines.next_line().await {
            buf.push_str(&line);
            buf.push('\n');
            if line.trim() == "]" {
                if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&buf) {
                    let parsed = parse_container_stats(arr).await;
                    *CONTAINER_STATS.lock().await = parsed.into();
                }
                buf.clear();
            }
        }

        // Only get here if podman exits or stdout closes
        let _ = child.kill().await;
        let _ = child.wait().await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn read_container_stats() -> Arc<[ContainerStats]> {
    CONTAINER_STATS.lock().await.clone()
}

async fn parse_container_stats(arr: Vec<serde_json::Value>) -> Vec<ContainerStats> {
    let app_names = crate::models::App::name_map().await.unwrap_or_default();
    let num_cores = num_cpus::get() as f64;

    arr.into_iter()
        .map(|v| {
            let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
            let name = s("name");
            let mem_usage = s("mem_usage");
            let mem_used = mem_usage.split('/').next().unwrap_or("").trim().to_string();

            // Normalize: podman reports per-system CPU, divide by cores
            let raw_cpu: f64 = s("cpu_percent")
                .trim_end_matches('%')
                .parse()
                .unwrap_or(0.0);
            let cpu = format!("{:.1}%", raw_cpu / num_cores);

            let (app_id, app_name, project_id, project_name) = name
                .strip_prefix("app-")
                .and_then(|id_str| id_str.parse::<i64>().ok())
                .and_then(|id| {
                    app_names
                        .get(&id)
                        .map(|(a, pid, p)| (Some(id), Some(a.clone()), Some(*pid), Some(p.clone())))
                })
                .unwrap_or((None, None, None, None));

            ContainerStats {
                name,
                app_id,
                app_name,
                project_id,
                project_name,
                cpu,
                mem_used,
                mem_percent: s("mem_percent"),
            }
        })
        .collect()
}

async fn stats_handler(c: &mut Ctx) {
    let h = HISTORY.lock().await.clone();
    c.res.json(&h);
}

async fn processes_handler(c: &mut Ctx) {
    let containers = read_container_stats().await;
    let latest = HISTORY.lock().await.last().cloned().unwrap_or_default();
    let ctr_cpu: f64 = containers
        .iter()
        .map(|c| c.cpu.trim_end_matches('%').parse::<f64>().unwrap_or(0.0))
        .sum();

    #[derive(Serialize)]
    struct Res<'a> {
        containers: &'a [ContainerStats],
        system_cpu: f64,
        latest: SystemStats,
    }

    c.res.json(&Res {
        containers: containers.as_ref(),
        system_cpu: r1((latest.cpu_percent - ctr_cpu).max(0.0)),
        latest,
    });
}

async fn podman_disk_handler(c: &mut Ctx) {
    let podman = podman()
        .args(["system", "df", "--format", "json"])
        .output()
        .await;

    let podman_data = match podman {
        Ok(o) if o.status.success() => serde_json::from_slice::<serde_json::Value>(&o.stdout)
            .unwrap_or(serde_json::json!(null)),
        _ => serde_json::json!(null),
    };

    let _g = buildkit::ensure_running().await;
    let buildkit_bytes = match tokio::process::Command::new("buildctl")
        .args(["--addr", buildkit::ADDR, "du", "--format", "json"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => serde_json::from_slice::<Vec<serde_json::Value>>(&o.stdout)
            .unwrap_or_default()
            .iter()
            .filter_map(|r| r["size"].as_u64())
            .sum(),
        _ => 0,
    };

    c.res.json(&serde_json::json!({
        "podman": podman_data,
        "buildkit_bytes": buildkit_bytes,
    }));
}
