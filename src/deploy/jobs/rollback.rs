use std::fmt;

use serde::{Deserialize, Serialize};

use crate::deploy::jobs::StartJob;
use crate::jobs::{self, Job};
use crate::models::{App, AppStatus, Deployment};

#[derive(Serialize, Deserialize)]
pub struct RollbackJob {
    pub app_id: i64,
    pub deploy_id: i64,
}

impl RollbackJob {
    pub async fn queue(app_id: i64, deploy_id: i64) -> Result<(), String> {
        jobs::enqueue(&Self { app_id, deploy_id }).await.map(|_| ())
    }
}

impl fmt::Display for RollbackJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rolling back App {}", self.app_id)
    }
}

impl Job for RollbackJob {
    const NAME: &'static str = "rollback_app";
    const MAX_RETRIES: i32 = 1;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        let (app_id, deploy_id) = (self.app_id, self.deploy_id);

        if let Err(e) = do_rollback(app_id, deploy_id).await {
            tracing::error!(deploy_id, app_id, "rollback failed: {e}");
            let _ = App::set_status(app_id, AppStatus::Failed).await;
        }
        Ok(())
    }
}

async fn do_rollback(app_id: i64, deploy_id: i64) -> Result<(), String> {
    let _d = Deployment::get_by_id(deploy_id)
        .await
        .map_err(|e| e.to_string())?;

    App::set_current_deployment(app_id, deploy_id)
        .await
        .map_err(|e| format!("set current: {e}"))?;

    StartJob::queue(app_id, Some(deploy_id))
        .await
        .map_err(|e| format!("queue start: {e}"))?;

    Ok(())
}
