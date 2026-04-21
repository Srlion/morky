mod build;
mod deploy_container;
mod rollback;
mod start;
mod stop;

pub use build::BuildJob;
pub use deploy_container::DeployContainerJob;
pub use rollback::RollbackJob;
pub use start::StartJob;
pub use stop::StopJob;

use crate::models::{App, AppStatus, DeployStatus, Deployment};

use super::log_broadcast;

pub fn register() {
    crate::jobs::register::<BuildJob>();
    crate::jobs::register::<DeployContainerJob>();
    crate::jobs::register::<RollbackJob>();
    crate::jobs::register::<StopJob>();
    crate::jobs::register::<StartJob>();
}

pub(super) async fn fail_deploy(app_id: i64, deploy_id: i64, error: &str) {
    log_broadcast::append_log(deploy_id, &format!("ERROR: {error}")).await;
    let _ = Deployment::finish(deploy_id, DeployStatus::Failed, Some(error)).await;

    if let Ok(AppStatus::Deploying) = App::get_status(app_id).await {
        let _ = App::set_status(app_id, AppStatus::Failed).await;
    }

    log_broadcast::send_status(deploy_id, DeployStatus::Failed);
}
