use crate::models::{Settings, User};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use maw::prelude::*;
use serde::Deserialize;

pub fn routes() -> Router {
    Router::group("/settings")
        .get("/", get)
        .put("/", update)
        .post("/change-password", change_password)
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
                .json(&serde_json::json!({"error": "invalid"}));
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
                .json(&serde_json::json!({"error": "invalid domain"}));
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

#[derive(Deserialize)]
struct ChangePasswordBody {
    current_password: String,
    new_password: String,
}

async fn change_password(c: &mut Ctx) {
    let body: ChangePasswordBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": "invalid"}));
        }
    };

    if body.new_password.len() < 8 {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "password must be at least 8 characters"}));
    }

    let user: &crate::models::User = c.res.locals.get("user").unwrap();
    let user = match User::get_by_id(user.id).await {
        Ok(u) => u,
        Err(_) => {
            return c
                .res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to load user"}));
        }
    };

    let Some(ref stored) = user.password_hash else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "no password set"}));
    };
    let Ok(parsed) = PasswordHash::new(stored) else {
        return c
            .res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({"error": "internal error"}));
    };
    if Argon2::default()
        .verify_password(body.current_password.as_bytes(), &parsed)
        .is_err()
    {
        return c
            .res
            .status(StatusCode::UNAUTHORIZED)
            .json(&serde_json::json!({"error": "current password is incorrect"}));
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = match Argon2::default().hash_password(body.new_password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(_) => {
            return c
                .res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to hash password"}));
        }
    };

    match User::update_password(user.id, &hash).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => c
            .res
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .json(&serde_json::json!({"error": e.to_string()})),
    }
}
