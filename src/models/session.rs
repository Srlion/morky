use chrono::{DateTime, Utc};

use crate::db::{self, conn};

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl db::FromRow for Session {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            ip: row.get("ip")?,
            user_agent: row.get("user_agent")?,
            expires_at: row.get("expires_at")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl Session {
    pub async fn create(
        id: &str,
        user_id: i64,
        ip: &str,
        user_agent: Option<&str>,
        expires_at: i64,
    ) -> db::Result<Self> {
        conn()
            .query_as(
                "INSERT INTO sessions (id, user_id, ip, user_agent, expires_at) VALUES (?, ?, ?, ?, ?) RETURNING *",
            )
            .bind(id)
            .bind(user_id)
            .bind(ip)
            .bind(user_agent)
            .bind(expires_at)
            .fetch_one()
            .await
    }

    pub async fn get_valid(id: &str) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM sessions WHERE id = ? AND expires_at > unixepoch()")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn delete(id: &str) -> db::Result<()> {
        conn()
            .query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn delete_expired() -> db::Result<u64> {
        conn()
            .query("DELETE FROM sessions WHERE expires_at <= unixepoch()")
            .execute()
            .await
            .map(|r| r.rows_affected())
    }
}
