use std::fmt;

use serde::{Deserialize, Serialize};

use crate::deploy::log_broadcast;
use crate::jobs::{self, Job};
use crate::models::{App, AppStatus, DeployStatus, Deployment};

use super::StartJob;
use super::fail_deploy;

#[derive(Serialize, Deserialize)]
pub struct DeployContainerJob {
    pub app_id: i64,
    pub deploy_id: i64,
}

impl DeployContainerJob {
    pub async fn queue(app_id: i64, deploy_id: i64) -> Result<(), String> {
        jobs::enqueue(&Self { app_id, deploy_id }).await.map(|_| ())
    }
}

impl fmt::Display for DeployContainerJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deploying App {}", self.app_id)
    }
}

impl Job for DeployContainerJob {
    const NAME: &'static str = "deploy_app_container";
    const MAX_RETRIES: i32 = 0;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        let (app_id, deploy_id) = (self.app_id, self.deploy_id);

        match do_deploy_container(app_id, deploy_id).await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!(deploy_id, app_id, "deploy failed: {e}");
                fail_deploy(app_id, deploy_id, &e).await;
            }
        }

        Ok(())
    }
}

async fn do_deploy_container(app_id: i64, deploy_id: i64) -> Result<(), String> {
    Deployment::update_status(deploy_id, DeployStatus::Deploying)
        .await
        .ok();
    App::set_status(app_id, AppStatus::Deploying)
        .await
        .map_err(|e| format!("db error: {e}"))?;

    log_broadcast::append_log(deploy_id, "build complete, queuing start...").await;

    StartJob::queue(app_id, Some(deploy_id))
        .await
        .map_err(|e| format!("queue start: {e}"))?;

    Ok(())
}
