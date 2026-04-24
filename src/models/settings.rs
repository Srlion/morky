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
    pub proxy_ip_header: String,
    pub updated_at: DateTime<Utc>,
}

impl FromRow for Settings {
    fn from_row(row: &Row) -> db::Result<Self> {
        Ok(Self {
            panel_domain: row.get("panel_domain")?,
            proxy_ip_header: row.get("proxy_ip_header")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

impl Settings {
    pub fn get() -> db::Result<Self> {
        if let Some(cached) = CACHE.lock().unwrap().clone() {
            return Ok(cached);
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            tokio::task::block_in_place(|| handle.block_on(Self::fetch()))
        } else {
            crate::tokio_handle().block_on(Self::fetch())
        }
    }

    async fn fetch() -> db::Result<Self> {
        let settings: Settings = conn()
            .query_as("SELECT * FROM settings WHERE id = 1")
            .fetch_one()
            .await?;
        *CACHE.lock().unwrap() = Some(settings.clone());
        Ok(settings)
    }

    pub fn update() -> SettingsUpdate {
        SettingsUpdate::default()
    }
}

#[must_use = "SettingsUpdate does nothing unless you call .apply()"]
#[derive(Default)]
pub struct SettingsUpdate {
    panel_domain: Option<Option<String>>, // Option<Option> to distinguish "not set" from "set to null"
    proxy_ip_header: Option<String>,
}

impl SettingsUpdate {
    pub fn panel_domain(mut self, v: Option<String>) -> Self {
        self.panel_domain = Some(v);
        self
    }
    pub fn proxy_ip_header(mut self, v: impl Into<String>) -> Self {
        self.proxy_ip_header = Some(v.into());
        self
    }

    pub async fn apply(self) -> db::Result<Settings> {
        let mut sets = Vec::new();
        if self.panel_domain.is_some() {
            sets.push("panel_domain = ?");
        }
        if self.proxy_ip_header.is_some() {
            sets.push("proxy_ip_header = ?");
        }

        if sets.is_empty() {
            return Settings::fetch().await;
        }

        sets.push("updated_at = unixepoch()");
        let sql = format!("UPDATE settings SET {} WHERE id = 1", sets.join(", "));

        let mut q = conn().query(&sql);
        if let Some(v) = self.panel_domain {
            q = q.bind(v);
        }
        if let Some(v) = self.proxy_ip_header {
            q = q.bind(v);
        }
        q.execute().await?;

        let settings = Settings::fetch().await;
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
