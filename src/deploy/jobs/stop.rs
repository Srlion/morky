use std::fmt;

use serde::{Deserialize, Serialize};

use crate::common::podman;
use crate::deploy::container;
use crate::jobs::{self, Job};
use crate::models::{App, AppStatus, DeployStatus, Deployment};

#[derive(Serialize, Deserialize)]
pub struct StopJob {
    pub app_id: i64,
}

impl StopJob {
    pub async fn queue(app_id: i64) -> Result<(), String> {
        jobs::enqueue(&Self { app_id }).await.map(|_| ())
    }
}

impl fmt::Display for StopJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stopping App {}", self.app_id)
    }
}

impl Job for StopJob {
    const NAME: &'static str = "stop_app";
    const MAX_RETRIES: i32 = 1;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        let app_id = self.app_id;

        let app = App::get_by_id(app_id)
            .await
            .map_err(|e| format!("load app: {e}"))?;

        let name = container::name(app_id);
        let _ = podman()
            .args(["stop", "--time", "60", &name])
            .status()
            .await;
        container::stop_log_tailer(app_id, app.current_deployment_id).await;
        let _ = podman().args(["rm", &name]).status().await;

        App::set_status(app_id, AppStatus::Idle)
            .await
            .map_err(|e| format!("set status: {e}"))?;

        if let Some(did) = app.current_deployment_id {
            let _ = Deployment::update_status(did, DeployStatus::Done).await;
        }

        Ok(())
    }
}
