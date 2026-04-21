use serde::{Deserialize, Serialize};

use crate::db::{self, FromRow, Row, conn};

#[derive(Debug, Clone, Serialize)]
pub struct CleanupSettings {
    pub auto_cleanup_enabled: bool,
    pub clean_containers: bool,
    pub clean_images: bool,
    pub clean_volumes: bool,
    pub clean_buildkit: bool,
    pub buildkit_keep_storage_gb: f64,
    pub cleanup_interval_hours: i64,
    pub last_cleanup_at: Option<i64>,
}

impl FromRow for CleanupSettings {
    fn from_row(row: &Row) -> db::Result<Self> {
        Ok(Self {
            auto_cleanup_enabled: row.get("auto_cleanup_enabled")?,
            clean_containers: row.get("clean_containers")?,
            clean_images: row.get("clean_images")?,
            clean_volumes: row.get("clean_volumes")?,
            clean_buildkit: row.get("clean_buildkit")?,
            buildkit_keep_storage_gb: row.get("buildkit_keep_storage_gb")?,
            cleanup_interval_hours: row.get("cleanup_interval_hours")?,
            last_cleanup_at: row.get("last_cleanup_at")?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CleanupSettingsUpdate {
    pub auto_cleanup_enabled: bool,
    pub clean_containers: bool,
    pub clean_images: bool,
    pub clean_volumes: bool,
    pub clean_buildkit: bool,
    pub buildkit_keep_storage_gb: f64,
    pub cleanup_interval_hours: i64,
}

impl CleanupSettings {
    pub async fn get() -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM cleanup_settings WHERE id = 1")
            .fetch_one()
            .await
    }

    pub async fn update(u: CleanupSettingsUpdate) -> db::Result<Self> {
        conn()
            .query(
                "UPDATE cleanup_settings SET \
                 auto_cleanup_enabled = ?, \
                 clean_containers = ?, clean_images = ?, \
                 clean_volumes = ?, clean_buildkit = ?, \
                 buildkit_keep_storage_gb = ?, \
                 cleanup_interval_hours = ?, \
                 updated_at = unixepoch() \
                 WHERE id = 1",
            )
            .bind(u.auto_cleanup_enabled)
            .bind(u.clean_containers)
            .bind(u.clean_images)
            .bind(u.clean_volumes)
            .bind(u.clean_buildkit)
            .bind(u.buildkit_keep_storage_gb.max(0.0))
            .bind(u.cleanup_interval_hours.max(1))
            .execute()
            .await?;

        Self::get().await
    }

    pub async fn mark_cleaned() -> db::Result<()> {
        conn()
            .query("UPDATE cleanup_settings SET last_cleanup_at = unixepoch() WHERE id = 1")
            .execute()
            .await?;
        Ok(())
    }
}
