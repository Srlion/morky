use maw::prelude::*;
use serde::Deserialize;

use crate::common::podman;
use crate::db::conn;
use crate::models::{ContainerLog, DeployStatus, Deployment};
use crate::{db, deploy, globals};

pub fn routes() -> Router {
    db::on_update(|event| {
        use db::Action::*;

        if event.table != "deployments" {
            return;
        }

        let row_id = event.row_id;
        match event.action {
            SQLITE_INSERT | SQLITE_UPDATE => {
                crate::tokio_handle().spawn(async move {
                    if let Some(deploy) = Deployment::get_by_id(row_id).await.ok() {
                        globals::set(format!("deploy_status_{}", deploy.id), deploy.status);

                        globals::set(
                            format!("app_deploy_status_{}", deploy.app_id),
                            deploy.status,
                        );
                    }
                });
            }
            SQLITE_DELETE => {
                globals::set(format!("deploy_status_{row_id}"), ());
            }
            _ => {}
        }
    });

    Router::group("/deployments")
        .get("/", list_deployments)
        .get("/{deploy_id}", get_deployment)
        .get("/{deploy_id}/log", get_log)
        .get("/{deploy_id}/ws", deploy_log_ws)
        .post("/{deploy_id}/cancel", cancel_deploy)
        .get("/{deploy_id}/container-log", get_container_log)
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

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_per_page")]
    per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

async fn list_deployments(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let q = c.req.query::<ListQuery>().unwrap_or(ListQuery {
        page: 1,
        per_page: 20,
    });
    let per_page = q.per_page.clamp(1, 100);
    let offset = (q.page.max(1) - 1) * per_page;

    let total: i64 = conn()
        .query_as("SELECT COUNT(*) FROM deployments WHERE app_id = ?")
        .bind(app_id)
        .fetch_one()
        .await
        .unwrap_or(0);

    let deployments: Vec<Deployment> = conn()
        .query_as(
            "SELECT * FROM deployments WHERE app_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(app_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all()
        .await
        .unwrap_or_default();

    let checks = deployments.iter().map(|d| async move {
        podman()
            .args(["image", "exists", &d.image_tag()])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    });
    let image_exists: Vec<bool> = futures_util::future::join_all(checks).await;

    let items: Vec<serde_json::Value> = deployments
        .iter()
        .zip(image_exists)
        .map(|(d, exists)| {
            let mut v = serde_json::to_value(d).unwrap();
            v["image_exists"] = serde_json::json!(exists);
            v
        })
        .collect();

    c.res.json(&serde_json::json!({
        "items": items,
        "total": total,
        "page": q.page,
        "per_page": per_page,
    }));
}

async fn get_deployment(c: &mut Ctx) {
    let Ok(deploy_id) = c.req.param::<i64>("deploy_id") else {
        return bad(c, "invalid id");
    };
    match Deployment::get_by_id(deploy_id).await {
        Ok(d) => c.res.json(&d),
        _ => not_found(c),
    }
}

async fn cancel_deploy(c: &mut Ctx) {
    let Ok(app_id) = c.req.param::<i64>("app_id") else {
        return bad(c, "invalid id");
    };
    let Ok(deploy_id) = c.req.param::<i64>("deploy_id") else {
        return bad(c, "invalid deploy id");
    };
    let d = match Deployment::get_by_id(deploy_id).await {
        Ok(d) if d.app_id == app_id => d,
        _ => return not_found(c),
    };
    if !matches!(d.status, DeployStatus::Building) {
        return bad(c, "deployment is not building");
    }
    deploy::cancel_deploy(deploy_id);
    c.res.json(&serde_json::json!({"ok": true}));
}

async fn get_log(c: &mut Ctx) {
    let Ok(deploy_id) = c.req.param::<i64>("deploy_id") else {
        return c.res.json(&serde_json::json!({"lines":[],"status":""}));
    };
    let status = Deployment::get_by_id(deploy_id)
        .await
        .map(|d| d.status)
        .ok();
    let lines = Deployment::get_log_lines(deploy_id)
        .await
        .unwrap_or_default();

    c.res.json(&serde_json::json!({
        "lines": lines,
        "status": status,
    }));
}

async fn deploy_log_ws(c: &mut Ctx) -> Result<(), StatusError> {
    let deploy_id: i64 = c.req.param("deploy_id").unwrap_or(0);
    let existing = Deployment::get_by_id(deploy_id).await.ok();
    let mut rx = deploy::log_broadcast::subscribe(deploy_id);

    Ok(c.upgrade_websocket(async move |mut ws| {
        if let Some(ref d) = existing {
            let lines = Deployment::get_log_lines(d.id).await.unwrap_or_default();
            if !lines.is_empty() {
                let _ = ws.send(serde_json::json!({"t":"bulk","d":lines}).to_string()).await;
            }
            if matches!(d.status, DeployStatus::Done | DeployStatus::Failed) {
                let _ = ws.send(serde_json::json!({"t":"status","d":d.status}).to_string()).await;
                return;
            }
        }
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(line) => {
                            if let Some(status) = line.strip_prefix("\x01STATUS:") {
                                let _ = ws.send(serde_json::json!({"t":"status","d":status}).to_string()).await;
                                break;
                            }
                            let _ = ws.send(serde_json::json!({"t":"line","d":line}).to_string()).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            let _ = ws.send(serde_json::json!({"t":"line","d":format!("(skipped {n} lines)")}).to_string()).await;
                        }
                    }
                }
                msg = ws.recv() => {
                    match msg {
                        Some(Ok(WsMessage::Close(_))) | None => break,
                        _ => {}
                    }
                }
            }
        }
    })?)
}

#[derive(Deserialize)]
struct ContainerLogQuery {
    #[serde(default)]
    offset: i64,
    #[serde(default = "default_log_limit")]
    limit: i64,
}

fn default_log_limit() -> i64 {
    500
}

async fn get_container_log(c: &mut Ctx) {
    let Ok(deploy_id) = c.req.param::<i64>("deploy_id") else {
        return bad(c, "invalid id");
    };
    let q = c
        .req
        .query::<ContainerLogQuery>()
        .unwrap_or(ContainerLogQuery {
            offset: 0,
            limit: 500,
        });
    let limit = q.limit.max(1);
    let lines = ContainerLog::get_lines(deploy_id, q.offset, limit)
        .await
        .unwrap_or_default();
    let total = ContainerLog::count(deploy_id).await.unwrap_or(0);

    c.res.json(&serde_json::json!({
        "lines": lines,
        "offset": q.offset,
        "total": total,
    }));
}
