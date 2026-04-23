use crate::{
    deploy::{builder::Builder, buildkit, shell::run_logged},
    models::Deployment,
};

pub struct DockerfileBuilder;

impl Builder for DockerfileBuilder {
    async fn build(&self, deployment: &Deployment) -> Result<(), String> {
        let _g = buildkit::ensure_running().await;
        let deploy_id = deployment.id;
        let work_dir = deployment.work_dir();
        let repo_dir = deployment.repo_dir();
        let dockerfile = deployment
            .dockerfile_path
            .as_deref()
            .unwrap_or("Dockerfile");
        let oci_tar = format!("{work_dir}/image.tar");

        run_logged(
            deploy_id,
            "buildctl",
            &[
                "--addr",
                buildkit::ADDR,
                "build",
                "--frontend",
                "dockerfile.v0",
                "--local",
                &format!("context={repo_dir}"),
                "--local",
                &format!("dockerfile={repo_dir}"),
                "--opt",
                &format!("filename={dockerfile}"),
                "--output",
                &format!("type=oci,dest={oci_tar}"),
            ],
            &repo_dir,
        )
        .await?;

        deployment.load_image().await
    }
}
