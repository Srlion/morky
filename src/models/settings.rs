use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    db::{self, FromRow, Row, conn},
    hook::{self, Event},
};

static CACHE: LazyLock<Mutex<Option<Settings>>> = LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub panel_domain: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl FromRow for Settings {
    fn from_row(row: &Row) -> db::Result<Self> {
        Ok(Self {
            panel_domain: row.get("panel_domain")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl Settings {
    pub fn get() -> db::Result<Self> {
        if let Some(cached) = CACHE.lock().unwrap().clone() {
            return Ok(cached);
        }
        crate::tokio_handle().block_on(Self::fetch())
    }

    async fn fetch() -> db::Result<Self> {
        let settings: Settings = conn()
            .query_as("SELECT * FROM settings WHERE id = 1")
            .fetch_one()
            .await?;
        *CACHE.lock().unwrap() = Some(settings.clone());
        Ok(settings)
    }

    pub async fn set_panel_domain(domain: Option<String>) -> db::Result<Self> {
        conn()
            .query("UPDATE settings SET panel_domain = ?, updated_at = unixepoch() WHERE id = 1")
            .bind(domain)
            .execute()
            .await?;
        let settings = Self::fetch().await;
        if let Ok(settings) = &settings {
            SettingsEvent::updated(settings.clone());
        }
        settings
    }
}

#[must_use]
#[derive(Clone)]
pub enum SettingsEvent {
    Updated(Settings),
}

impl SettingsEvent {
    pub fn updated(settings: Settings) {
        SettingsEvent::Updated(settings).fire();
    }
}

impl hook::Event for SettingsEvent {}
