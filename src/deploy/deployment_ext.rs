use crate::{
    common::podman,
    deploy::{container, log_broadcast},
    models::Deployment,
};

pub const BUILD_ROOT: &str = "/tmp/morky-builds";

impl Deployment {
    pub fn container_name(&self) -> String {
        container::name(self.app_id)
    }

    pub fn image_tag(&self) -> String {
        format!("app-{}:deploy-{}", self.app_id, self.id)
    }

    pub fn work_dir(&self) -> String {
        Self::work_dir_for(self.id)
    }

    pub fn work_dir_for(id: i64) -> String {
        format!("{BUILD_ROOT}/{id}")
    }

    pub fn repo_dir(&self) -> String {
        format!("{BUILD_ROOT}/{}/repo", self.id)
    }

    pub fn oci_tar(&self) -> String {
        format!("{}/image.tar", self.work_dir())
    }

    pub async fn load_image(&self) -> Result<(), String> {
        let oci_tar = &self.oci_tar();
        let output = podman()
            .args(["load", "-i", oci_tar])
            .output()
            .await
            .map_err(|e| format!("podman load: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "podman load: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let loaded = String::from_utf8_lossy(&output.stdout);
        let image_ref = loaded
            .lines()
            .find_map(|l| l.strip_prefix("Loaded image:"))
            .map(|s| s.trim())
            .ok_or("could not parse loaded image id")?;
        log_broadcast::append_log(self.id, &format!("loaded image: {image_ref}")).await;

        let o = podman()
            .args(["tag", image_ref, &self.image_tag()])
            .output()
            .await
            .map_err(|e| format!("podman tag: {e}"))?;
        if !o.status.success() {
            return Err(format!(
                "podman tag: {}",
                String::from_utf8_lossy(&o.stderr)
            ));
        }
        let _ = tokio::fs::remove_file(oci_tar).await;
        Ok(())
    }
}
