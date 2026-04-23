use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::db::{self, conn};

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub username: String,
    // Don't serialize the password hash (for big security reasons)
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl db::FromRow for User {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            email: row.get("email")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            email_verified_at: row.get("email_verified_at")?,
            created_at: row.get("created_at")?,
        })
    }
}

impl User {
    pub async fn get_by_id(id: i64) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn get_by_email(email: &str) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_one()
            .await
    }

    pub async fn create(email: &str, username: &str, password_hash: &str) -> db::Result<Self> {
        conn()
            .query_as(
                "INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?) RETURNING *",
            )
            .bind(email)
            .bind(username)
            .bind(password_hash)
            .fetch_one()
            .await
    }

    pub async fn update_password(id: i64, password_hash: &str) -> db::Result<()> {
        conn()
            .query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(password_hash)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }
}
