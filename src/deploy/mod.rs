mod builder;
pub mod buildkit;
mod cancel_registry;
pub mod container;
pub mod deployment_ext;
mod dockerfile;
pub mod env;
pub mod jobs;
pub mod log_broadcast;
mod railpack;
mod recovery;
pub mod shell;

pub async fn init() {
    buildkit::start_idle_reaper();
    jobs::register();
    recovery::check_containers().await;
}

pub fn cancel_deploy(deploy_id: i64) {
    cancel_registry::cancel(deploy_id);
}
