use tokio::fs;
use tokio::sync::Mutex;

use crate::{
    constants, hook,
    models::{AppEvent, ProjectEvent, Settings, SettingsEvent},
};

mod haproxy;
mod routes;

use routes::Route;

static LOCK: Mutex<()> = Mutex::const_new(());

const MORKY_CONTAINER: &str = "morky";

pub async fn init() -> std::io::Result<()> {
    let _g = LOCK.lock().await;

    fs::create_dir_all(haproxy::dir()).await?;
    if !fs::try_exists(haproxy::map_file()).await.unwrap_or(false) {
        fs::write(haproxy::map_file(), "").await?;
    }

    haproxy::ensure_network(haproxy::HAPROXY_NET).await?;
    haproxy::ensure_container().await?;

    reconcile().await?;

    hook::add_async(async |_: SettingsEvent| log_err("settings", rebuild().await));
    hook::add_async(async |_: ProjectEvent| log_err("project", rebuild().await));
    hook::add_async(async |e: AppEvent| {
        if matches!(
            e,
            AppEvent::Deleting(_) | AppEvent::Updated(_) | AppEvent::Started(_, _)
        ) {
            log_err("app", rebuild().await);
        }
    });

    Ok(())
}

pub async fn rebuild() -> std::io::Result<()> {
    let _g = LOCK.lock().await;
    reconcile().await
}

pub fn container_name(app_id: i64) -> String {
    format!("app-{app_id}")
}

pub fn project_net(project_id: i64) -> String {
    format!("project-{project_id}-net")
}

fn log_err(tag: &str, r: std::io::Result<()>) {
    if let Err(e) = r {
        tracing::error!("rebuild on {tag} event: {e}");
    }
}

async fn reconcile() -> std::io::Result<()> {
    let rows: Vec<(i64, i64, String, i64)> = crate::db::conn()
        .query_as(
            "SELECT a.id, a.project_id, a.domain, d.port
               FROM apps a
               JOIN deployments d ON d.id = a.current_deployment_id
              WHERE a.domain IS NOT NULL AND a.domain != ''",
        )
        .fetch_all()
        .await
        .unwrap_or_default();

    let all_project_ids: Vec<i64> = crate::db::conn()
        .query_scalar("SELECT id FROM projects")
        .fetch_all()
        .await
        .unwrap_or_default();

    let mut desired: std::collections::BTreeSet<i64> = all_project_ids.into_iter().collect();
    desired.extend(rows.iter().map(|(_, pid, _, _)| *pid).filter(|&p| p > 0));

    let existing = haproxy::list_project_networks().await.unwrap_or_default();

    for pid in &desired {
        let net = project_net(*pid);
        if !existing.contains(&net) {
            haproxy::ensure_network(&net).await?;
        }
        // Connect both haproxy and morky to every project network
        let _ = haproxy::connect(&net).await;
        let _ = haproxy::connect_container(MORKY_CONTAINER, &net).await;
    }

    for net in &existing {
        let pid = parse_project_net(net);
        if pid.is_none_or(|p| !desired.contains(&p)) {
            haproxy::disconnect(net).await;
            haproxy::disconnect_container(MORKY_CONTAINER, net).await;
            haproxy::remove_network(net).await;
        }
    }

    let routes = load_routes().await;
    routes::save(&routes).await?;
    haproxy::write_config(&routes).await?;
    haproxy::write_map(&routes).await?;
    haproxy::reload().await
}

async fn load_routes() -> Vec<Route> {
    let rows: Vec<(i64, i64, String, i64)> = crate::db::conn()
        .query_as(
            "SELECT a.id, a.project_id, a.domain, d.port
               FROM apps a
               JOIN deployments d ON d.id = a.current_deployment_id
              WHERE a.domain IS NOT NULL AND a.domain != ''",
        )
        .fetch_all()
        .await
        .unwrap_or_default();

    let mut routes: Vec<Route> = Vec::new();
    for (app_id, project_id, domain, port) in rows {
        let container = container_name(app_id);
        routes.push(Route {
            project_id,
            app_id,
            domain: domain.to_lowercase(),
            backend_host: container,
            port: port as u16,
        });
    }

    if let Ok(settings) = Settings::get() {
        if let Some(d) = settings
            .panel_domain
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            routes.push(Route {
                project_id: -1,
                app_id: -1,
                domain: d.to_lowercase(),
                backend_host: MORKY_CONTAINER.to_string(),
                port: constants::port(),
            });
        }
    }
    routes
}

fn parse_project_net(name: &str) -> Option<i64> {
    name.strip_prefix("project-")?
        .strip_suffix("-net")?
        .parse()
        .ok()
}
