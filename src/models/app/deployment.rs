use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    db::{self, conn},
    models::{App, DeployStatus, Project},
};

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct Deployment {
    pub id: i64,
    pub app_id: i64,
    pub commit_sha: String,
    pub commit_message: Option<String>,
    pub branch: String,
    pub build_method: String,
    pub dockerfile_path: Option<String>,
    pub status: DeployStatus,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub port: i64,
    pub health_check_path: String,
    pub env_vars: HashMap<String, String>,
    pub volume_path: String,
}

impl db::FromRow for Deployment {
    fn from_row(row: &db::Row) -> db::Result<Self> {
        let env_vars_str: Option<String> = row.get("env_vars").unwrap_or_default();
        let env_vars = env_vars_str
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Ok(Self {
            id: row.get("id")?,
            app_id: row.get("app_id")?,
            commit_sha: row.get("commit_sha")?,
            commit_message: row.get("commit_message")?,
            branch: row.get("branch")?,
            build_method: row.get("build_method")?,
            dockerfile_path: row.get("dockerfile_path")?,
            status: row.get("status")?,
            error: row.get("error")?,
            created_at: row.get("created_at")?,
            finished_at: row.get("finished_at")?,
            port: row.get("port").unwrap_or(0),
            health_check_path: row.get("health_check_path").unwrap_or_default(),
            env_vars,
            volume_path: row.get("volume_path").unwrap_or_default(),
        })
    }
}

impl Deployment {
    pub async fn create(
        app_id: i64,
        branch: &str,
        build_method: &str,
        dockerfile_path: Option<&str>,
    ) -> anyhow::Result<Self> {
        let app = App::get_by_id(app_id).await?;

        let project = Project::get_by_id(app.project_id).await?;

        let env_vars = crate::deploy::env::build_env_vars(&app, &project);
        let env_json = serde_json::to_string(&env_vars).unwrap_or_default();

        conn()
        .query_as("INSERT INTO deployments (app_id, branch, build_method, dockerfile_path, commit_sha, port, health_check_path, env_vars, volume_path) \
                   VALUES (?, ?, ?, ?, '', ?, ?, ?, ?) RETURNING *")
        .bind(app_id)
        .bind(branch)
        .bind(build_method)
        .bind(dockerfile_path)
        .bind(app.port)
        .bind(app.health_check_path)
        .bind(env_json)
        .bind(app.volume_path)
        .fetch_one()
        .await
        .map_err(|e| anyhow::anyhow!(e))
    }

    pub async fn get_by_id(id: i64) -> db::Result<Self> {
        conn()
            .query_as("SELECT * FROM deployments WHERE id = ?")
            .bind(id)
            .fetch_one()
            .await
    }

    pub async fn update_status(id: i64, status: DeployStatus) -> db::Result<()> {
        conn()
            .query("UPDATE deployments SET status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn update_building(
        id: i64,
        commit_sha: &str,
        commit_message: &str,
    ) -> db::Result<()> {
        conn().query("UPDATE deployments SET status = 'building', commit_sha = ?, commit_message = ? WHERE id = ?",).bind(commit_sha).bind(commit_message).bind(id).execute().await?;
        Ok(())
    }

    pub async fn finish(id: i64, status: DeployStatus, error: Option<&str>) -> db::Result<()> {
        conn().query("UPDATE deployments SET status = ?, error = ?, finished_at = unixepoch() WHERE id = ?",).bind(status).bind(error).bind(id).execute().await?;
        Ok(())
    }

    pub async fn append_log(id: i64, line: &str) -> db::Result<()> {
        conn()
            .query("INSERT INTO build_log_lines (deployment_id, line) VALUES (?, ?)")
            .bind(id)
            .bind(line)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn clear_log(id: i64) -> db::Result<()> {
        conn()
            .query("DELETE FROM build_log_lines WHERE deployment_id = ?")
            .bind(id)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn get_log_lines(id: i64) -> db::Result<Vec<String>> {
        conn()
            .query_as("SELECT line FROM build_log_lines WHERE deployment_id = ? ORDER BY id ASC")
            .bind(id)
            .fetch_all()
            .await
    }

    pub async fn app(&self) -> db::Result<App> {
        App::get_by_id(self.app_id).await
    }

    pub async fn app_name(&self) -> db::Result<String> {
        App::name(self.app_id).await
    }

    pub async fn project_id(&self) -> db::Result<i64> {
        App::project_id(self.app_id).await
    }
}
