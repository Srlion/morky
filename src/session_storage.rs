use maw::middlewares::session::{SessionStorage, SessionStore};
use maw::{postcard, prelude::*};

use crate::db;

pub struct SqliteSessionStorage;

impl SessionStorage for SqliteSessionStorage {
    async fn load(&self, _: &mut Ctx, id: &str) -> Option<SessionStore> {
        let data: Vec<u8> = db::conn()
            .query_scalar("SELECT data FROM session_store WHERE id = ?")
            .bind(id)
            .fetch_optional()
            .await
            .ok()??;
        postcard::from_bytes(&data).ok()
    }

    async fn save(&self, _: &mut Ctx, id: &str, session: &SessionStore) {
        let data = match postcard::to_stdvec(session) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to serialize session: {e}");
                return;
            }
        };
        let _ = db::conn()
            .query(
                "INSERT INTO session_store (id, data) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            )
            .bind(id)
            .bind(data)
            .execute()
            .await;
    }
}
