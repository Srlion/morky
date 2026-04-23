use argon2::{Argon2, PasswordHash, PasswordVerifier};
use maw::prelude::{minijinja::context, *};
use serde::Deserialize;
use validator::Validate;

use crate::models::{Session, User};

pub mod middlewares;

const BAD: &str = "Invalid email or password.";

pub fn routes() -> Router {
    Router::group("/auth")
        .get("/signin", signin_page)
        .post("/signin", signin_action)
        .post("/logout", logout_action)
}

#[derive(Deserialize, Validate)]
struct SigninForm {
    #[validate(email(message = "Invalid email address."))]
    email: String,

    #[validate(length(min = 1, message = "Password is required."))]
    password: String,
}

async fn signin_page(c: &mut Ctx) {
    if c.res.locals.contains_key("user") {
        return c.res.redirect("/", None);
    }
    c.res.render_str(signin_html());
}

async fn signin_action(c: &mut Ctx) {
    let form: SigninForm = match c.req.form().await {
        Ok(f) => f,
        Err(_) => return c.close(),
    };

    if let Err(errs) = form.validate() {
        return render_signin_err(c, &first_error(&errs), &form);
    }

    let email = form.email.trim().to_lowercase();

    let user = match User::get_by_email(&email).await {
        Ok(u) => u,
        _ => return render_signin_err(c, BAD, &form),
    };

    let Some(ref stored) = user.password_hash else {
        return render_signin_err(c, BAD, &form);
    };

    let Ok(parsed) = PasswordHash::new(stored) else {
        return render_signin_err(c, BAD, &form);
    };

    if Argon2::default()
        .verify_password(form.password.as_bytes(), &parsed)
        .is_err()
    {
        return render_signin_err(c, BAD, &form);
    }

    if let Err(e) = create_session(c, user.id).await {
        tracing::error!("session: {e}");
        return render_signin_err(c, "Server error.", &form);
    }

    c.res.redirect("/", None);
}

async fn logout_action(c: &mut Ctx) {
    if let Ok(sid) = c.session.get::<String>("sid") {
        let _ = Session::delete(&sid).await;
    }
    c.session.clear();
    c.res.redirect("/", None);
}

async fn create_session(c: &mut Ctx, user_id: i64) -> anyhow::Result<()> {
    let sid = uuid::Uuid::new_v4().to_string();
    let expires = chrono::Utc::now().timestamp() + 30 * 24 * 3600;
    let ip = &c.req.ip();
    let ua = c.req.header("user-agent").map(|v| v.to_string());
    Session::create(&sid, user_id, ip, ua.as_deref(), expires).await?;
    c.session.set("sid", &sid);
    Ok(())
}

fn first_error(errs: &validator::ValidationErrors) -> String {
    for (_, field_errs) in errs.field_errors() {
        if let Some(e) = field_errs.first()
            && let Some(msg) = &e.message
        {
            return msg.to_string();
        }
    }
    "Invalid input.".to_string()
}

fn render_signin_err(c: &mut Ctx, msg: &str, form: &SigninForm) {
    c.res.render_str_with(
        signin_html(),
        context! {
            error => msg,
            email => form.email,
        },
    );
}

fn signin_html() -> &'static str {
    r###"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Sign In - Morky</title>
        <style>
            * { box-sizing: border-box; font-family: system-ui, sans-serif; }
            body {
                margin: 0; min-height: 100vh; background: #1a1a1a; color: #e5e5e5;
                display: grid; place-items: center;
            }
            form {
                background: #242424; padding: 24px; border: 1px solid #333;
                border-radius: 6px; width: 100%; max-width: 340px;
                display: flex; flex-direction: column; gap: 12px;
            }
            h1 { margin: 0 0 8px; font-size: 20px; text-align: center; }
            .err { color: #ef4444; border: 1px solid #ef4444; padding: 8px; border-radius: 4px; font-size: 13px; }
            label { font-size: 12px; color: #999; }
            input {
                width: 100%; padding: 8px; margin-top: 4px; border-radius: 4px;
                background: #1a1a1a; border: 1px solid #444; color: #fff;
            }
            input:focus { outline: 2px solid #3b82f6; border-color: transparent; }
            button {
                margin-top: 8px; padding: 10px; background: #3b82f6; color: #fff;
                border: none; border-radius: 4px; font-weight: 600; cursor: pointer;
            }
            button:hover { filter: brightness(1.1); }
        </style>
    </head>
    <body>
        <form method="POST" action="/auth/signin">
            <h1>Sign in to Morky</h1>

            {% if error %}
            <div class="err">{{ error }}</div>
            {% endif %}

            <label>Email
                <input type="email" name="email" value="{{ email|default('') }}" required autofocus>
            </label>

            <label>Password
                <input type="password" name="password" required>
            </label>

            <button type="submit">Sign In</button>
        </form>
    </body>
    </html>
    "###
}
