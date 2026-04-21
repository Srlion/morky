use maw::prelude::*;
use serde::Deserialize;

use crate::models::Settings;

pub fn routes() -> Router {
    Router::group("/settings").get("/", get).put("/", update)
}

async fn get(c: &mut Ctx) {
    match Settings::get().await {
        Ok(settings) => c.res.json(&settings),
        Err(e) => c
            .res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
struct UpdateBody {
    panel_domain: Option<String>,
}

async fn update(c: &mut Ctx) {
    let body: UpdateBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error":"invalid"}));
        }
    };

    let pd = body
        .panel_domain
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    if let Some(ref d) = pd {
        if !crate::common::is_fqdn(d) {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error":"invalid domain"}));
        }
    }

    match Settings::set_panel_domain(pd).await {
        Ok(settings) => c
            .res
            .json(&serde_json::json!({"ok": true, "settings": settings})),
        Err(e) => c
            .res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({"error": e.to_string()})),
    }
}
