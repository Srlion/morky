use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::db::{self, conn};

#[derive(Debug, Clone, Serialize)]
pub struct GitSource {
    pub id: i64,
    pub provider: String,
    pub name: String,
    pub provider_data: Value,
    pub created_at: DateTime<Utc>,
}

impl db::FromRow for GitSource {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            provider: row.get("provider")?,
            name: row.get("name")?,
            provider_data: row.get("provider_data")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl GitSource {
    pub async fn list() -> db::Result<Vec<Self>> {
        conn()
            .query_as("SELECT * FROM git_sources ORDER BY created_at DESC")
            .fetch_all()
            .await
    }

    pub async fn get_by_id(id: i64) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM git_sources WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn create(provider: &str, name: &str, provider_data: Value) -> db::Result<Self> {
        conn()
            .query_as(
                "INSERT INTO git_sources (provider, name, provider_data) \
                 VALUES (?, ?, ?) RETURNING *",
            )
            .bind(provider)
            .bind(name)
            .bind(provider_data)
            .fetch_one()
            .await
    }

    pub async fn delete(id: i64) -> db::Result<()> {
        conn()
            .query("DELETE FROM git_sources WHERE id = ?")
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_provider_data(id: i64, provider_data: Value) -> db::Result<()> {
        conn()
            .query("UPDATE git_sources SET provider_data = ? WHERE id = ?")
            .bind(provider_data)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_name(id: i64, name: &str) -> db::Result<()> {
        conn()
            .query("UPDATE git_sources SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }
}
