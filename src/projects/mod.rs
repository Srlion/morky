use crate::models::{App, Project};
use maw::prelude::*;
use serde::Deserialize;
use validator::Validate;

pub fn routes() -> Router {
    Router::group("/projects")
        .get("/", list)
        .post("/", create)
        .get("/{id}", get_one)
        .delete("/{id}", delete)
        .put("/{id}/env", save_env)
}

async fn list(c: &mut Ctx) {
    let projects = Project::list().await.unwrap_or_default();
    c.res.json(&projects);
}

async fn get_one(c: &mut Ctx) {
    let Ok(id) = c.req.param::<i64>("id") else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "invalid id"}));
    };
    match Project::get_by_id(id).await {
        Ok(p) => {
            let apps = App::list_by_project(id).await.unwrap_or_default();
            c.res
                .json(&serde_json::json!({ "project": p, "apps": apps }));
        }
        _ => c
            .res
            .status(StatusCode::NOT_FOUND)
            .json(&serde_json::json!({"error": "not found"})),
    }
}

#[derive(Deserialize, Validate)]
struct CreateBody {
    #[validate(length(min = 1, max = 100))]
    name: String,
    description: Option<String>,
}

async fn create(c: &mut Ctx) {
    let body: CreateBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": "invalid body"}));
        }
    };
    if body.validate().is_err() {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "name is required"}));
    }
    let desc = body.description.as_deref().filter(|s| !s.trim().is_empty());
    match Project::create(body.name.trim(), desc).await {
        Ok(p) => c.res.json(&p),
        Err(e) => {
            tracing::error!("create project: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to create"}));
        }
    }
}

async fn delete(c: &mut Ctx) {
    let Ok(id) = c.req.param::<i64>("id") else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "invalid id"}));
    };
    match Project::delete(id).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            tracing::error!("delete project: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to delete"}));
        }
    }
}

async fn save_env(c: &mut Ctx) {
    let Ok(id) = c.req.param::<i64>("id") else {
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

    match Project::update_env_vars(id, env_vars).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            tracing::error!("save env: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to save"}));
        }
    }
}
