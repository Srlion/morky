use crate::models::{Session, User};
use maw::prelude::*;

pub async fn auth_loader(c: &mut Ctx) {
    if let Ok(sid) = c.session.get::<String>("sid") {
        if let Ok(session) = Session::get_valid(&sid).await {
            if let Ok(user) = User::get_by_id(session.user_id).await {
                c.res.locals.set("user", user);
            }
        } else {
            c.session.remove("sid");
        }
    }
    c.next().await;
}

pub async fn require_auth(c: &mut Ctx) {
    if !c.res.locals.contains_key("user") {
        c.res.redirect("/auth/signin", None);
    } else {
        c.next().await;
    }
}

pub async fn require_auth_api(c: &mut Ctx) {
    if !c.res.locals.contains_key("user") {
        c.res.status(StatusCode::UNAUTHORIZED);
        c.res.json(&serde_json::json!({"error": "unauthorized"}));
    } else {
        c.next().await;
    }
}
