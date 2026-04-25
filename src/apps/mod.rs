use maw::prelude::*;
use serde::Deserialize;

use crate::deploy::container;
use crate::deploy::jobs::{BuildJob, RollbackJob, StartJob, StopJob};
use crate::models::{App, AppStatus, DeployStatus, Deployment};
use crate::{common, db, globals};

mod deployments;
mod volume;

pub fn routes() -> Router {
    db::on_update(|event| {
        use db::Action::*;

        if event.table != "apps" {
            return;
        }

        let row_id = event.row_id;
        match event.action {
            SQLITE_INSERT | SQLITE_UPDATE => {
                crate::tokio_handle().spawn(async move {
                    if let Some(app) = App::get_by_id(row_id).await.ok() {
                        globals::set(format!("app_status_{row_id}"), app.status);
                        globals::set(
                            format!("app_current_deployment_{row_id}"),
                            app.current_deployment_id,
                        );
                    }
                });
            }
            SQLITE_DELETE => {
                globals::set(format!("app_status_{row_id}"), ());
            }
            _ => {}
        }
    });

    Router::group("/apps")
        .post("/", create)
        .get("/{app_id}", get_one)
        .delete("/{app_id}", delete)
        .put("/{app_id}/settings", save_settings)
        .put("/{app_id}/general-settings", save_general_settings)
        .post("/{app_id}/deploy", deploy_action)
        .post("/{app_id}/stop", stop_action)
        .post("/{app_id}/start", start_action)
        .post("/{app_id}/rollback/{deploy_id}", rollback_action)
        .put("/{app_id}/env", save_env)
        .get("/{app_id}/container-logs", container_logs)
        .push(
            Router::group("/{app_id}")
                .push(deployments::routes())
                .push(volume::routes()),
        )
}

fn bad(c: &mut Ctx, msg: &str) {
    c.res
        .status(StatusCode::BAD_REQUEST)
        .json(&serde_json::json!({"error": msg}));
}

fn not_found(c: &mut Ctx) {
    c.res
        .status(StatusCode::NOT_FOUND)
        .json(&serde_json::json!({"error": "not found"}));
}

fn fail(c: &mut Ctx, msg: &str) {
    c.res
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .json(&serde_json::json!({"error": msg}));
}

#[derive(Deserialize)]
struct CreateBody {
    project_id: i64,
    name: String,
    git_source_id: i64,
    repo: String,
    branch: String,
}

fn validate_app_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() || name.len() > 63 {
        return Err("app name must be 1-63 characters");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("app name may only contain lowercase letters, digits, and hyphens");
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err("app name must not start or end with a hyphen");
    }
    Ok(())
}

async fn create(c: &mut Ctx) {
    let body: CreateBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => return bad(c, "invalid body"),
    };
    let name = body.name.trim();
    if let Err(e) = validate_app_name(&name) {
        return bad(c, e);
    }
    if body.repo.is_empty() || body.branch.is_empty() {
        return bad(c, "all fields required");
    }
    let port = match App::next_available_port().await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("port: {e}");
            return fail(c, "failed to allocate port");
        }
    };
    match App::create(
        body.project_id,
        name,
        body.git_source_id,
        &body.repo,
        &body.branch,
        port,
    )
    .await
    {
        Ok(app) => c.res.json(&app),
        Err(e) => {
            tracing::error!("create app: {e}");
            fail(c, "failed to create");
        }
    }
}

async fn get_one(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    match App::get_by_id(app_id).await {
        Ok(app) => c.res.json(&app),
        _ => not_found(c),
    }
}

async fn delete(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    match App::delete(app_id).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            tracing::error!("delete app: {e}");
            fail(c, "failed to delete");
        }
    }
}

#[derive(Deserialize)]
struct SettingsBody {
    branch: String,
    build_method: String,
    dockerfile_path: Option<String>,
    port: Option<i64>,
    health_check_path: Option<String>,
    volume_path: Option<String>,
}

async fn save_settings(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let body: SettingsBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => return bad(c, "invalid body"),
    };
    let method = if body.build_method == "dockerfile" {
        "dockerfile"
    } else {
        "railpack"
    };
    let df = body
        .dockerfile_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Dockerfile");
    let port = body.port.unwrap_or(3000).max(1).min(65535);
    let hc = body
        .health_check_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");

    match App::is_port_available(port, Some(app_id)).await {
        Ok(false) => return bad(c, &format!("port {port} already in use")),
        Err(e) => return bad(c, &format!("port check: {e}")),
        _ => {}
    }

    let volume_path = if method == "railpack" {
        body.volume_path
            .as_deref()
            .map(str::trim)
            .unwrap_or("/data")
            .to_string()
    } else {
        String::new() // ignored for dockerfile builds
    };

    match App::update(app_id)
        .branch(body.branch.trim())
        .build_method(method)
        .dockerfile_path(df)
        .port(port)
        .health_check_path(hc)
        .volume_path(&volume_path)
        .apply()
        .await
    {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => fail(c, &format!("save: {e}")),
    }
}

#[derive(Deserialize)]
struct GeneralSettingsBody {
    container_logs_enabled: bool,
    domain: Option<String>, // null or "" to clear
}

async fn save_general_settings(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let body: GeneralSettingsBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => return bad(c, "invalid body"),
    };

    let new_domain = body
        .domain
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());
    if let Some(ref d) = new_domain {
        if !common::is_fqdn(d) {
            return bad(c, "invalid domain format (use example.com)");
        }
    }

    if let Err(e) = App::update(app_id)
        .container_logs_enabled(body.container_logs_enabled)
        .domain(new_domain.as_deref())
        .apply()
        .await
    {
        return fail(c, &format!("save: {e}"));
    }

    c.res.json(&serde_json::json!({"ok": true}));
}

async fn deploy_action(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let app = match App::get_by_id(app_id).await {
        Ok(a) => a,
        _ => {
            return not_found(c);
        }
    };
    let df = if app.build_method == "dockerfile" {
        Some(app.dockerfile_path.as_str())
    } else {
        None
    };
    match Deployment::create(app_id, &app.branch, &app.build_method, df).await {
        Ok(d) => {
            if let Err(e) = BuildJob::queue(app_id, d.id).await {
                return bad(c, &e);
            }
            c.res
                .json(&serde_json::json!({"ok": true, "deployment_id": d.id}));
        }
        Err(e) => {
            tracing::error!("create deployment: {e}");
            fail(c, "failed to start deployment");
        }
    }
}

async fn stop_action(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let app = match App::get_by_id(app_id).await {
        Ok(a) => a,
        _ => return not_found(c),
    };
    if app.status != AppStatus::Running {
        return bad(c, "app is not running");
    }
    match StopJob::queue(app_id).await {
        Ok(()) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => bad(c, &e),
    }
}

async fn start_action(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let app = match App::get_by_id(app_id).await {
        Ok(a) => a,
        _ => return not_found(c),
    };
    if app.status == AppStatus::Running {
        return bad(c, "app is already running");
    }
    if app.current_deployment_id.is_none() {
        return bad(c, "no previous deployment to start from");
    }
    match StartJob::queue(app_id, None).await {
        Ok(()) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => bad(c, &e),
    }
}

async fn rollback_action(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let Ok(deploy_id) = c.req.param::<i64>("deploy_id") else {
        return bad(c, "invalid deploy id");
    };
    let ok = matches!(
        Deployment::get_by_id(deploy_id).await,
        Ok(d) if d.status == DeployStatus::Done && d.app_id == app_id
    );
    if !ok {
        return bad(c, "cannot rollback to this deployment");
    }
    if let Err(e) = RollbackJob::queue(app_id, deploy_id).await {
        return bad(c, &e);
    }
    c.res.json(&serde_json::json!({"ok": true}));
}

async fn container_logs(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return c.res.json(&serde_json::json!({"log": ""}));
    };
    let cname = container::name(app_id);
    let out = common::podman()
        .args(&["logs", "--tail", "500", &cname])
        .output()
        .await;
    let log = match out {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).to_string();
            let e = String::from_utf8_lossy(&o.stderr);
            if !e.is_empty() {
                if !s.is_empty() {
                    s.push('\n');
                }
                s.push_str(&e);
            }
            s
        }
        Err(e) => format!("Error: {e}"),
    };
    c.res.json(&serde_json::json!({"log": log}));
}

async fn save_env(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "invalid id"}));
    };

    let env_vars = match c.req.text().await {
        Ok(b) => b,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": "invalid body"}));
        }
    };

    match App::update(app_id).env_vars(env_vars).apply().await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            tracing::error!("save env: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to save"}));
        }
    }
}
