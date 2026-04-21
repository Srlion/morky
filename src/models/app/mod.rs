use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::db::{self, conn};

mod deployment;
mod events;
mod status;
pub use deployment::Deployment;
pub use events::*;
pub use status::{AppStatus, DeployStatus};

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct App {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    env_vars: Option<String>,
    pub git_source_id: Option<i64>,
    pub repo: Option<String>,
    pub branch: String,
    pub status: AppStatus,
    pub build_method: String,
    pub dockerfile_path: String,
    pub domain: Option<String>,
    pub port: i64,
    pub current_deployment_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub health_check_path: String,
    pub container_logs_enabled: bool,
    pub volume_path: String,
}

impl db::FromRow for App {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            env_vars: row.get("env_vars")?,
            git_source_id: row.get("git_source_id")?,
            repo: row.get("repo")?,
            branch: row.get("branch")?,
            status: row.get("status")?,
            build_method: row.get("build_method")?,
            dockerfile_path: row.get("dockerfile_path")?,
            domain: row.get("domain")?,
            port: row.get("port")?,
            current_deployment_id: row.get("current_deployment_id")?,
            created_at: row.get("created_at")?,
            health_check_path: row.get("health_check_path")?,
            container_logs_enabled: row.get("container_logs_enabled").unwrap_or(true),
            volume_path: row.get("volume_path").unwrap_or_default(),
        })
    }
}

impl App {
    pub fn env_vars(&self) -> HashMap<String, String> {
        crate::common::env_vars::parse(self.env_vars.as_deref().unwrap_or(""))
    }

    pub async fn list_by_project(project_id: i64) -> db::Result<Vec<Self>> {
        conn()
            .query_as("SELECT * FROM apps WHERE project_id = ? ORDER BY created_at DESC")
            .bind(project_id)
            .fetch_all()
            .await
    }

    pub async fn create(
        project_id: i64,
        name: &str,
        git_source_id: i64,
        repo: &str,
        branch: &str,
        port: i64,
    ) -> db::Result<Self> {
        let app: App = conn()
            .query_as(
                "INSERT INTO apps (project_id, name, git_source_id, repo, branch, port) \
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
            )
            .bind(project_id)
            .bind(name)
            .bind(git_source_id)
            .bind(repo)
            .bind(branch)
            .bind(port)
            .fetch_one()
            .await?;
        AppEvent::created(app.clone());
        Ok(app)
    }

    pub fn update(id: i64) -> AppUpdate {
        AppUpdate::new(id)
    }

    pub async fn get_by_id(id: i64) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM apps WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn project_id(id: i64) -> db::Result<i64> {
        conn()
            .query_scalar("SELECT project_id FROM apps WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn delete(id: i64) -> db::Result<()> {
        let app = Self::get_by_id(id).await?;
        AppEvent::deleting(app);
        conn()
            .query("DELETE FROM apps WHERE id = ?")
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn set_current_deployment(id: i64, deployment_id: i64) -> db::Result<()> {
        conn()
            .txn(move |t| {
                t.query(
                    "UPDATE deployments SET status = 'superseded', finished_at = unixepoch() \
             WHERE app_id = ? AND status = 'running' AND id != ?",
                )
                .bind(id)
                .bind(deployment_id)
                .execute()?;

                t.query(
                    "UPDATE apps SET current_deployment_id = ?, status = 'running' WHERE id = ?",
                )
                .bind(deployment_id)
                .bind(id)
                .execute()?;

                Ok(())
            })
            .await
    }

    pub async fn set_status(id: i64, status: AppStatus) -> db::Result<()> {
        conn()
            .query("UPDATE apps SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    /// Find the next available port starting from `base` (default 4000).
    pub async fn next_available_port() -> db::Result<i64> {
        // Get all ports currently in use
        let used: Vec<i64> = conn()
            .query_scalar("SELECT port FROM apps ORDER BY port")
            .fetch_all()
            .await?;

        let mut port = 4000;
        for p in &used {
            if *p == port {
                port += 1;
            }
        }
        Ok(port)
    }

    /// Check if a port is available (optionally excluding a specific app).
    pub async fn is_port_available(port: i64, exclude_app_id: Option<i64>) -> db::Result<bool> {
        let count: i64 = match exclude_app_id {
            Some(eid) => {
                conn()
                    .query_scalar("SELECT COUNT(*) FROM apps WHERE port = ? AND id != ?")
                    .bind(port)
                    .bind(eid)
                    .fetch_one()
                    .await?
            }
            None => {
                conn()
                    .query_scalar("SELECT COUNT(*) FROM apps WHERE port = ?")
                    .bind(port)
                    .fetch_one()
                    .await?
            }
        };
        Ok(count == 0)
    }

    /// Returns a map of app id -> (app name, project id, project name) for resolving container names.
    pub async fn name_map() -> db::Result<std::collections::HashMap<i64, (String, i64, String)>> {
        let rows: Vec<(i64, String, i64, String)> = conn()
            .query_as(
                "SELECT a.id, a.name, p.id, p.name FROM apps a \
             JOIN projects p ON p.id = a.project_id",
            )
            .fetch_all()
            .await?;
        Ok(rows
            .into_iter()
            .map(|(id, app, pid, proj)| (id, (app, pid, proj)))
            .collect())
    }

    pub async fn name(id: i64) -> db::Result<String> {
        conn()
            .query_scalar("SELECT name FROM apps WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn get_status(id: i64) -> db::Result<AppStatus> {
        conn()
            .query_scalar("SELECT status FROM apps WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn containers_enabled(id: i64) -> bool {
        conn()
            .query_scalar("SELECT container_logs_enabled FROM apps WHERE id = ?")
            .bind(id)
            .fetch_optional()
            .await
            .unwrap_or(Some(true))
            .unwrap_or(true)
    }
}

#[must_use = "AppUpdate does nothing unless you call .apply()"]
#[derive(Default)]
pub struct AppUpdate {
    id: i64,
    branch: Option<String>,
    build_method: Option<String>,
    dockerfile_path: Option<String>,
    port: Option<i64>,
    health_check_path: Option<String>,
    volume_path: Option<String>,
    container_logs_enabled: Option<bool>,
    domain: Option<String>,
    env_vars: Option<String>,
}

impl AppUpdate {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn branch(mut self, v: impl Into<String>) -> Self {
        self.branch = Some(v.into());
        self
    }
    pub fn build_method(mut self, v: impl Into<String>) -> Self {
        self.build_method = Some(v.into());
        self
    }
    pub fn dockerfile_path(mut self, v: impl Into<String>) -> Self {
        self.dockerfile_path = Some(v.into());
        self
    }
    pub fn port(mut self, v: i64) -> Self {
        self.port = Some(v);
        self
    }
    pub fn health_check_path(mut self, v: impl Into<String>) -> Self {
        self.health_check_path = Some(v.into());
        self
    }
    pub fn volume_path(mut self, v: impl Into<String>) -> Self {
        self.volume_path = Some(v.into());
        self
    }
    pub fn container_logs_enabled(mut self, v: bool) -> Self {
        self.container_logs_enabled = Some(v);
        self
    }
    pub fn domain(mut self, v: Option<impl Into<String>>) -> Self {
        self.domain = v.map(|v| v.into());
        self
    }
    pub fn env_vars(mut self, v: impl Into<String>) -> Self {
        self.env_vars = Some(v.into());
        self
    }

    pub async fn apply(self) -> db::Result<()> {
        let mut sets = Vec::new();
        if self.branch.is_some() {
            sets.push("branch = ?")
        }
        if self.build_method.is_some() {
            sets.push("build_method = ?")
        }
        if self.dockerfile_path.is_some() {
            sets.push("dockerfile_path = ?")
        }
        if self.port.is_some() {
            sets.push("port = ?")
        }
        if self.health_check_path.is_some() {
            sets.push("health_check_path = ?")
        }
        if self.volume_path.is_some() {
            sets.push("volume_path = ?")
        }
        if self.container_logs_enabled.is_some() {
            sets.push("container_logs_enabled = ?")
        }
        if self.domain.is_some() {
            sets.push("domain = ?")
        }
        if self.env_vars.is_some() {
            sets.push("env_vars = ?")
        }

        if sets.is_empty() {
            return Ok(());
        }

        let sql = format!("UPDATE apps SET {} WHERE id = ?", sets.join(", "));

        let mut q = conn().query(&sql);
        if let Some(v) = self.branch {
            q = q.bind(v);
        }
        if let Some(v) = self.build_method {
            q = q.bind(v);
        }
        if let Some(v) = self.dockerfile_path {
            q = q.bind(v);
        }
        if let Some(v) = self.port {
            q = q.bind(v);
        }
        if let Some(v) = self.health_check_path {
            q = q.bind(v);
        }
        if let Some(v) = self.volume_path {
            q = q.bind(v);
        }
        if let Some(v) = self.container_logs_enabled {
            q = q.bind(v);
        }
        if let Some(v) = self.domain {
            q = q.bind(v);
        }
        if let Some(v) = self.env_vars {
            q = q.bind(v);
        }
        q.bind(self.id).execute().await?;

        if let Ok(app) = App::get_by_id(self.id).await {
            AppEvent::updated(app);
        }

        Ok(())
    }
}
