use super::haproxy;
use crate::common::atomic_write;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Route {
    pub project_id: i64,
    pub app_id: i64,
    pub domain: String,
    pub backend_host: String,
    pub port: u16,
}

impl Route {
    pub fn backend(&self) -> String {
        format!("app_{}_{}", self.project_id, self.app_id)
    }
}

pub async fn save(routes: &[Route]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(routes)
        .map_err(|e| std::io::Error::other(format!("encode routes.json: {e}")))?;
    atomic_write(&haproxy::routes_file(), &json).await
}
