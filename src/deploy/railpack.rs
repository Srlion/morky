use super::builder::Builder;
use super::buildkit;
use super::env::{EnvMode, inject_env, secrets_hash};
use super::shell::{run_logged, run_logged_with_env};
use crate::models::Deployment;

pub struct RailpackBuilder;

impl Builder for RailpackBuilder {
    async fn build(&self, deployment: &Deployment) -> Result<(), String> {
        let _g = buildkit::ensure_running().await;
        let deploy_id = deployment.id;
        let work_dir = deployment.work_dir();
        let repo_dir = deployment.repo_dir();
        let oci_tar = deployment.oci_tar();
        let env_vars = &deployment.env_vars;

        let plan_path = format!("{work_dir}/railpack-plan.json");
        let mut prep: Vec<String> = vec![
            "prepare".into(),
            ".".into(),
            "--plan-out".into(),
            plan_path.clone(),
        ];
        prep.extend(inject_env(env_vars, EnvMode::Runtime));
        let refs: Vec<&str> = prep.iter().map(String::as_str).collect();
        run_logged(deploy_id, "railpack", &refs, &repo_dir).await?;

        let hash = secrets_hash(env_vars);
        let mut args: Vec<String> = vec![
            "--addr".into(),
            buildkit::ADDR.into(),
            "build".into(),
            "--frontend".into(),
            "gateway.v0".into(),
            "--opt".into(),
            "source=ghcr.io/railwayapp/railpack-frontend".into(),
            "--local".into(),
            format!("dockerfile={work_dir}"),
            "--local".into(),
            format!("context={repo_dir}"),
            "--opt".into(),
            "filename=railpack-plan.json".into(),
            "--opt".into(),
            format!("build-arg:secrets-hash={hash}"),
        ];
        args.extend(inject_env(env_vars, EnvMode::Build));
        args.extend(["--output".into(), format!("type=oci,dest={oci_tar}")]);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_logged_with_env(deploy_id, "buildctl", &refs, &work_dir, env_vars).await?;

        deployment.load_image().await
    }
}
