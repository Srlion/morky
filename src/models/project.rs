use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    db::{self, conn},
    hook::{self, Event},
};

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    env_vars: Option<String>,
    pub created_at: DateTime<Utc>,
    pub app_count: Option<i64>,
}

impl db::FromRow for Project {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            env_vars: row.get("env_vars")?,
            created_at: row.get("created_at")?,
            app_count: row.get("app_count").unwrap_or(None),
        })
    }
}

impl Project {
    pub fn env_vars(&self) -> HashMap<String, String> {
        crate::common::env_vars::parse(self.env_vars.as_deref().unwrap_or(""))
    }

    pub async fn list() -> db::Result<Vec<Self>> {
        conn()
            .query_as(
                "SELECT p.*, (SELECT COUNT(*) FROM apps WHERE project_id = p.id) as app_count \
                 FROM projects p ORDER BY p.created_at DESC",
            )
            .fetch_all()
            .await
    }

    pub async fn create(name: &str, description: Option<&str>) -> db::Result<Self> {
        let project: Self = conn()
            .query_as("INSERT INTO projects (name, description) VALUES (?, ?) RETURNING *")
            .bind(name)
            .bind(description)
            .fetch_one()
            .await?;
        ProjectEvent::created(project.clone());
        Ok(project)
    }

    pub async fn get_by_id(id: i64) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM projects WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn delete(id: i64) -> db::Result<()> {
        let project = Self::get_by_id(id).await?;
        ProjectEvent::deleted(project);
        conn()
            .query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_env_vars(id: i64, env_vars: &str) -> db::Result<()> {
        conn()
            .query("UPDATE projects SET env_vars = ? WHERE id = ?")
            .bind(env_vars)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }
}

#[must_use]
#[derive(Clone)]
pub enum ProjectEvent {
    Created(Project),
    Deleted(Project),
}

impl ProjectEvent {
    pub fn created(project: Project) {
        ProjectEvent::Created(project).fire();
    }

    pub fn deleted(project: Project) {
        ProjectEvent::Deleted(project).fire();
    }
}

impl hook::Event for ProjectEvent {}
