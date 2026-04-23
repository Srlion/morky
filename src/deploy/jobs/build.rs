use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    deploy::{
        builder::Builder as _,
        cancel_registry,
        jobs::{DeployContainerJob, fail_deploy},
        log_broadcast,
    },
    git_sources::ops as git,
    jobs::{self, Job},
    models::{DeployStatus, Deployment},
};

#[derive(Serialize, Deserialize)]
pub struct BuildJob {
    app_id: i64,
    deploy_id: i64,
}

impl BuildJob {
    pub async fn queue(app_id: i64, deploy_id: i64) -> Result<(), String> {
        jobs::enqueue(&Self { app_id, deploy_id }).await.map(|_| ())
    }
}

impl fmt::Display for BuildJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Building App {}", self.app_id)
    }
}

impl Job for BuildJob {
    const NAME: &'static str = "build_app";
    const MAX_RETRIES: i32 = 0;
    const CPU_BOUND: bool = true;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        let (app_id, deploy_id) = (self.app_id, self.deploy_id);

        Deployment::clear_log(deploy_id).await.ok();
        let _ = tokio::fs::remove_dir_all(Deployment::work_dir_for(deploy_id)).await;

        match cancel_registry::run(deploy_id, || do_build(deploy_id)).await {
            Ok(()) => {
                if let Err(e) = DeployContainerJob::queue(app_id, deploy_id).await {
                    tracing::error!(deploy_id, app_id, "failed to enqueue deploy container: {e}");
                    fail_deploy(app_id, deploy_id, &e).await;
                }
            }
            Err(e) => {
                tracing::error!(deploy_id, app_id, "build failed: {e}");
                fail_deploy(app_id, deploy_id, &e).await;
            }
        }
        let _ = tokio::fs::remove_dir_all(Deployment::work_dir_for(deploy_id)).await;
        Ok(())
    }
}

async fn do_build(deploy_id: i64) -> Result<(), String> {
    let _ = Deployment::update_status(deploy_id, DeployStatus::Building).await;
    let deployment = Deployment::get_by_id(deploy_id)
        .await
        .map_err(|e| e.to_string())?;

    clone_source(&deployment).await?;

    log_broadcast::append_log(
        deploy_id,
        &format!("building with {}...", deployment.build_method),
    )
    .await;
    build_image(&deployment).await?;
    log_broadcast::append_log(deploy_id, "build complete, deploying...").await;
    Ok(())
}

async fn clone_source(deployment: &Deployment) -> Result<(), String> {
    let id = deployment.id;
    let app = deployment
        .app()
        .await
        .map_err(|e| format!("get app: {e}"))?;
    let git_source_id = app.git_source_id.ok_or("no git source")?;
    let repo = app.repo.as_deref().ok_or("app has no repo")?;

    let work_dir = &deployment.work_dir();
    let repo_dir = &deployment.repo_dir();

    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| format!("mkdir: {e}"))?;

    if deployment.commit_sha.is_empty() {
        log_broadcast::append_log(
            id,
            &format!("cloning {repo} (branch: {})...", deployment.branch),
        )
        .await;
        crate::git_sources::clone_repo(git_source_id, repo, &deployment.branch, repo_dir, work_dir)
            .await?;
        let commit = git::head_commit(repo_dir).await?;
        Deployment::update_building(id, &commit.sha, &commit.message)
            .await
            .map_err(|e| format!("update building: {e}"))?;
        log_broadcast::append_log(
            id,
            &format!("commit ({}): {}", commit.short_sha, commit.message),
        )
        .await;
    } else {
        let sha = &deployment.commit_sha;
        log_broadcast::append_log(
            id,
            &format!("cloning {repo} at commit {}...", &sha[..sha.len().min(7)]),
        )
        .await;
        crate::git_sources::clone_repo_at_commit(git_source_id, repo, sha, repo_dir, work_dir)
            .await?;
        log_broadcast::append_log(
            id,
            &format!(
                "commit ({}): {}",
                &sha[..sha.len().min(7)],
                deployment.commit_message.as_deref().unwrap_or("")
            ),
        )
        .await;
    }
    Ok(())
}

async fn build_image(deployment: &Deployment) -> Result<(), String> {
    match deployment.build_method.as_str() {
        "dockerfile" => {
            crate::deploy::dockerfile::DockerfileBuilder
                .build(deployment)
                .await
        }
        "railpack" => {
            crate::deploy::railpack::RailpackBuilder
                .build(deployment)
                .await
        }
        other => Err(format!("unknown build method: {other}")),
    }
}
