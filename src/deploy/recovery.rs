use crate::common::podman;
use crate::db::conn;
use crate::deploy::container;
use crate::deploy::jobs::StartJob;
use crate::models::{App, AppStatus};

/// Run on startup. Restarts containers that died while the server was down.
/// The job queue handles interrupted deploys automatically (resets running -> pending).
pub async fn check_containers() {
    let running: Vec<i64> = conn()
        .query_as("SELECT id FROM apps WHERE status = 'running'")
        .fetch_all()
        .await
        .unwrap_or_default();

    for app_id in running {
        let name = container::name(app_id);
        let alive = podman()
            .args(["inspect", "--format", "{{.State.Running}}", &name])
            .output()
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false);

        if alive {
            continue;
        }

        let _ = App::set_status(app_id, AppStatus::Idle).await;
        if let Err(e) = StartJob::queue(app_id, None).await {
            tracing::error!(app_id, "failed to queue start: {e}");
            let _ = App::set_status(app_id, AppStatus::Failed).await;
        }
    }
}
