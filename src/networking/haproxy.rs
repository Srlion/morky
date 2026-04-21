use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::routes::Route;
use crate::common::{atomic_write, podman};

pub const HAPROXY_CONTAINER: &str = "morky-haproxy";
pub const HAPROXY_NET: &str = "morky-haproxy-net";
const IMAGE: &str = "docker.io/library/haproxy:3.3-alpine";
const PROJECT_NET_PREFIX: &str = "project-";
const PROJECT_NET_SUFFIX: &str = "-net";

const CFG_HEADER: &str = r#"
global
    user root
    stats socket /etc/haproxy/admin.sock mode 660 level admin expose-fd listeners
    stats timeout 30s
    log stdout format raw local0 notice

resolvers podman
    parse-resolv-conf
    resolve_retries 30
    timeout resolve 1s
    timeout retry 1s
    hold valid 1s

defaults
    log     global
    mode    http
    option  dontlognull
    timeout connect 5s
    timeout client  50s
    timeout server  50s
    retries 10

frontend http_in
    bind *:80
    http-request set-var(txn.host) req.hdr(host),field(1,:),lower
    use_backend %[var(txn.host),map(/etc/haproxy/hosts.map)] if { var(txn.host),map(/etc/haproxy/hosts.map) -m found }
    default_backend no_match

backend no_match
    http-request return status 503 content-type text/plain string "no route\n"
"#;

fn base_dir() -> &'static Path {
    static BASE: OnceLock<PathBuf> = OnceLock::new();
    BASE.get_or_init(|| {
        if let Ok(d) = std::env::var("MORKY_DATA_DIR") {
            PathBuf::from(d)
        } else if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg).join("morky")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".local/share/morky")
        } else {
            PathBuf::from("./morky-data")
        }
    })
}

pub fn dir() -> PathBuf {
    base_dir().join("haproxy")
}
pub fn map_file() -> PathBuf {
    dir().join("hosts.map")
}
pub fn config_file() -> PathBuf {
    dir().join("haproxy.cfg")
}
pub fn routes_file() -> PathBuf {
    dir().join("routes.json")
}
fn host_dir() -> PathBuf {
    if let Ok(d) = std::env::var("MORKY_HOST_DATA_DIR") {
        PathBuf::from(d).join("haproxy")
    } else {
        dir() // fallback for running outside container
    }
}
fn bind_mount() -> String {
    format!("{}:/etc/haproxy", host_dir().display())
}

async fn podman_run(label: &str, args: &[&str], tolerated: &[&str]) -> std::io::Result<()> {
    let o = podman().args(args).output().await?;
    if o.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&o.stderr);
    if tolerated.iter().any(|t| stderr.contains(t)) {
        return Ok(());
    }
    Err(std::io::Error::other(format!("{label}: {}", stderr.trim())))
}

pub async fn ensure_network(name: &str) -> std::io::Result<()> {
    podman_run(
        &format!("podman network create {name}"),
        &["network", "create", name],
        &["already exists"],
    )
    .await
}

pub async fn remove_network(name: &str) {
    let _ = podman().args(["network", "rm", "-f", name]).status().await;
}

pub async fn connect(name: &str) -> std::io::Result<()> {
    podman_run(
        &format!("podman network connect {name}"),
        &["network", "connect", name, HAPROXY_CONTAINER],
        &["already"],
    )
    .await
}

pub async fn disconnect(name: &str) {
    let _ = podman()
        .args(["network", "disconnect", name, HAPROXY_CONTAINER])
        .output()
        .await;
}

/// List podman networks that look like `project-<id>-net`.
pub async fn list_project_networks() -> std::io::Result<Vec<String>> {
    let o = podman()
        .args(["network", "ls", "--format", "{{.Name}}"])
        .output()
        .await?;
    if !o.status.success() {
        return Err(std::io::Error::other(format!(
            "podman network ls: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with(PROJECT_NET_PREFIX) && l.ends_with(PROJECT_NET_SUFFIX))
        .map(String::from)
        .collect())
}

pub async fn ensure_container() -> std::io::Result<()> {
    let inspect = podman()
        .args([
            "container",
            "inspect",
            "--format",
            "{{.State.Status}}",
            HAPROXY_CONTAINER,
        ])
        .output()
        .await?;

    if inspect.status.success() {
        if String::from_utf8_lossy(&inspect.stdout).trim() == "running" {
            return Ok(());
        }
        let _ = podman()
            .args(["rm", "-f", HAPROXY_CONTAINER])
            .status()
            .await;
    }

    podman_run(
        "podman run haproxy",
        &[
            "run",
            "-d",
            "--name",
            HAPROXY_CONTAINER,
            "--user",
            "root",
            "--restart",
            "always",
            "--network",
            HAPROXY_NET,
            "-p",
            "80:80",
            "-v",
            &bind_mount(),
            IMAGE,
            "haproxy",
            "-W",
            "-f",
            "/etc/haproxy/haproxy.cfg",
        ],
        &[],
    )
    .await
}

pub async fn reload() -> std::io::Result<()> {
    let o = podman()
        .args(["kill", "--signal", "HUP", HAPROXY_CONTAINER])
        .output()
        .await?;

    if o.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&o.stderr);
    if stderr.contains("no such container") || stderr.contains("not found") {
        return ensure_container().await;
    }
    Err(std::io::Error::other(format!(
        "podman kill -s HUP {HAPROXY_CONTAINER}: {}",
        stderr.trim()
    )))
}

pub async fn write_config(routes: &[Route]) -> std::io::Result<()> {
    let mut cfg = String::from(CFG_HEADER);
    let mut seen = std::collections::HashSet::new();
    for r in routes {
        let b = r.backend();
        if !seen.insert(b.clone()) {
            continue;
        }
        let _ = write!(
            cfg,
            "\nbackend {b}\n    server srv1 {}:{} check resolvers podman resolve-prefer ipv4 init-addr none\n",
            r.backend_host, r.port
        );
    }
    atomic_write(&config_file(), &cfg).await
}

pub async fn write_map(routes: &[Route]) -> std::io::Result<()> {
    let mut body = routes
        .iter()
        .map(|r| format!("{} {}", r.domain, r.backend()))
        .collect::<Vec<_>>()
        .join("\n");
    body.push('\n');
    atomic_write(&map_file(), &body).await
}

pub async fn connect_container(container: &str, network: &str) -> std::io::Result<()> {
    podman_run(
        &format!("podman network connect {network} {container}"),
        &["network", "connect", network, container],
        &["already"],
    )
    .await
}

pub async fn disconnect_container(container: &str, network: &str) {
    let _ = podman()
        .args(["network", "disconnect", network, container])
        .output()
        .await;
}
