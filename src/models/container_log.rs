use crate::db::{self, conn};

pub struct ContainerLog;

impl ContainerLog {
    pub async fn append(deployment_id: i64, line: &str, timestamp: i64) -> db::Result<()> {
        conn()
            .query("INSERT INTO container_logs (deployment_id, line, created_at) VALUES (?, ?, ?)")
            .bind(deployment_id)
            .bind(line)
            .bind(timestamp)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn get_lines(
        deployment_id: i64,
        offset: i64,
        limit: i64,
    ) -> db::Result<Vec<(String, i64)>> {
        conn()
        .query_as(
            "SELECT line, created_at FROM container_logs WHERE deployment_id = ? ORDER BY id ASC LIMIT ? OFFSET ?",
        )
        .bind(deployment_id)
        .bind(limit)
        .bind(offset)
        .fetch_all()
        .await
    }

    pub async fn count(deployment_id: i64) -> db::Result<i64> {
        conn()
            .query_scalar("SELECT COUNT(*) FROM container_logs WHERE deployment_id = ?")
            .bind(deployment_id)
            .fetch_one()
            .await
    }
}
