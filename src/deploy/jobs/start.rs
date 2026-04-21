use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::common::podman;
use crate::deploy::container;
use crate::deploy::jobs::{BuildJob, fail_deploy};
use crate::deploy::log_broadcast;
use crate::jobs::{self, Job};
use crate::models::{App, AppEvent, AppStatus, DeployStatus, Deployment};

#[derive(Serialize, Deserialize)]
pub struct StartJob {
    app_id: i64,
    deploy_id: Option<i64>,
}

impl StartJob {
    pub async fn queue(app_id: i64, deploy_id: Option<i64>) -> Result<(), String> {
        jobs::enqueue(&Self {
            app_id,
            deploy_id: deploy_id,
        })
        .await
        .map(|_| ())
    }
}

impl fmt::Display for StartJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Starting App {}", self.app_id)
    }
}

impl Job for StartJob {
    const NAME: &'static str = "start_app";
    const MAX_RETRIES: i32 = 1;
    const UNIQUE: bool = true;

    async fn run(&self) -> Result<(), String> {
        let (app_id, deploy_id) = (self.app_id, self.deploy_id);

        let deploy_id = match deploy_id {
            Some(id) => id,
            None => {
                let app = App::get_by_id(app_id)
                    .await
                    .map_err(|e| format!("load app: {e}"))?;
                app.current_deployment_id.ok_or("no current deployment")?
            }
        };

        match do_start(app_id, deploy_id).await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!(deploy_id, app_id, "start failed: {e}");
                fail_deploy(app_id, deploy_id, &e).await;
            }
        }

        Ok(())
    }
}

async fn do_start(app_id: i64, deploy_id: i64) -> Result<(), String> {
    let d = Deployment::get_by_id(deploy_id)
        .await
        .map_err(|e| format!("load deployment: {e}"))?;

    let tag = d.image_tag();
    let exists = podman()
        .args(["image", "exists", &tag])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if !exists {
        App::set_status(app_id, AppStatus::Idle).await.ok();
        return BuildJob::queue(app_id, deploy_id)
            .await
            .map_err(|e| format!("queue rebuild: {e}"));
    }

    App::set_status(app_id, AppStatus::Deploying).await.ok();
    log_broadcast::append_log(deploy_id, "starting container...").await;
    container::start(&d, Some(deploy_id)).await?;

    health_check(&d).await?;

    Deployment::finish(deploy_id, DeployStatus::Done, None)
        .await
        .map_err(|e| format!("finish: {e}"))?;
    App::set_current_deployment(app_id, deploy_id)
        .await
        .map_err(|e| format!("set current: {e}"))?;

    log_broadcast::append_log(deploy_id, "deploy complete ✓").await;
    log_broadcast::send_status(deploy_id, DeployStatus::Done);

    if let Ok(app) = App::get_by_id(app_id).await {
        AppEvent::started(app, d);
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    log_broadcast::remove(deploy_id);

    Ok(())
}

async fn health_check(deployment: &Deployment) -> Result<(), String> {
    let path = &deployment.health_check_path;
    if path.is_empty() {
        return Ok(());
    }

    let container_name = deployment.container_name();
    let output = podman()
        .args([
            "inspect",
            "--format",
            "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}",
            &container_name,
        ])
        .output()
        .await
        .map_err(|e| format!("inspect container: {e}"))?;

    let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ip.is_empty() {
        return Err("could not determine container IP".into());
    }

    let url = format!("http://{}:{}{}", ip, deployment.port, path);
    log_broadcast::append_log(deployment.id, &format!("health check: {url}")).await;

    let client = crate::http_client();
    const MAX_ATTEMPTS: u32 = 30;

    for attempt in 1..=MAX_ATTEMPTS {
        tokio::time::sleep(Duration::from_secs(2)).await;
        match client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(r) if r.status() == 200 || r.status() == 201 => {
                log_broadcast::append_log(deployment.id, "health check passed ✓").await;
                return Ok(());
            }
            Ok(r) if attempt % 5 == 0 => {
                log_broadcast::append_log(
                    deployment.id,
                    &format!(
                        "health check attempt {attempt}/{MAX_ATTEMPTS}: status {}",
                        r.status()
                    ),
                )
                .await;
            }
            Err(e) if attempt % 5 == 0 => {
                log_broadcast::append_log(
                    deployment.id,
                    &format!("health check attempt {attempt}/{MAX_ATTEMPTS}: {e}"),
                )
                .await;
            }
            _ => {}
        }
    }

    Err(format!("health check failed after {MAX_ATTEMPTS} attempts"))
}
