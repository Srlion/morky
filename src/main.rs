use maw::{
    middlewares::cookie::{self, CookieOptions},
    prelude::*,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _};

mod apps;
mod auth;
mod auto_setup;
mod backup;
mod common;
mod constants;
mod db;
mod deploy;
mod git_sources;
mod globals;
mod hook;
mod http_client;
mod jobs;
mod models;
mod monitoring;
mod networking;
mod projects;
mod session_storage;
mod settings;
mod tokio_handle;

use common::hex;
pub use http_client::client as http_client;
pub use tokio_handle::tokio_handle;

use crate::models::Settings;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(rust_embed::RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::registry()
        .with(EnvFilter::new(constants::rust_log()))
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_writer))
        .init();

    tokio_handle::init().await;

    db::init().await?;
    auto_setup::run().await;
    networking::init().await?;
    deploy::init().await; // register job types
    backup::register_jobs();
    jobs::start();
    monitoring::start_sampler();

    App::new()
        .proxy_header_fn(|| {
            Settings::get()
                .ok()
                .filter(|s| s.panel_domain.is_some())
                .map(|s| s.proxy_ip_header)
        })
        .router(routes())
        .listen((constants::host(), constants::port()))
        .await?;

    Ok(())
}

fn routes() -> Router {
    Router::new()
        .get("/healthz", async |_: &mut Ctx| StatusCode::NO_CONTENT)
        .middleware(maw::CatchPanicMiddleware::new())
        .middleware(
            maw::CookieMiddleware::new().key(
                hex::decode(&constants::cookie_secret_key())
                    .expect("COOKIE_SECRET_KEY must be valid hex"),
            ),
        )
        .middleware(
            maw::SessionMiddleware::new()
                .storage(session_storage::SqliteSessionStorage)
                .cookie_name("morky.session")
                .cookie_options(
                    CookieOptions::new()
                        .secure_fn(|| {
                            Settings::get()
                                .map(|s| s.panel_domain.is_some())
                                .unwrap_or(false)
                        })
                        .http_only(true)
                        .same_site(cookie::SameSite::Lax)
                        .path("/"),
                ),
        )
        .push(auth::routes())
        .middleware(auth::middlewares::auth_loader)
        .push(git_sources::browser_routes())
        .push(
            Router::group("/api")
                .middleware(auth::middlewares::require_auth_api)
                .middleware(maw::LoggingMiddleware::new())
                .get("/globals/stream", globals::sse_handler)
                .get("/me", me_handler)
                .push(projects::routes())
                .push(git_sources::api_routes())
                .push(apps::routes())
                .push(settings::routes())
                .push(monitoring::routes())
                .push(backup::routes()),
        )
        .middleware(auth::middlewares::require_auth)
        .static_files(
            "/",
            StaticFiles::new(Assets)
                .max_age(3600)
                .fallback_to("spa.html"),
        )
}

async fn me_handler(c: &mut Ctx) {
    let user: &models::User = c.res.locals.get("user").unwrap();
    c.res.json(serde_json::json!(user));
}
