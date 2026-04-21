use maw::prelude::rust_embed;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::common::hex;

#[derive(rust_embed::RustEmbed)]
#[folder = "migrations/"]
struct Migrations;

fn checksum(sql: &str) -> String {
    let mut h = Sha256::new();
    h.update(sql.as_bytes());
    hex::encode(h.finalize().as_slice())
}

pub fn migrate(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name TEXT PRIMARY KEY,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let mut names: Vec<String> = Migrations::iter()
        .filter(|n| n.ends_with(".sql"))
        .map(|n| n.into_owned())
        .collect();
    names.sort();

    for name in names {
        let file = Migrations::get(&name)
            .ok_or_else(|| anyhow::anyhow!("migration '{name}' not found"))?;
        let sql = std::str::from_utf8(file.data.as_ref())
            .map_err(|_| anyhow::anyhow!("migration '{name}' is not valid UTF-8"))?;
        let hash = checksum(sql);

        let existing: Option<String> = conn
            .query_row(
                "SELECT checksum FROM _migrations WHERE name = ?1",
                [&name],
                |r| r.get(0),
            )
            .ok();

        match existing {
            Some(h) if h != hash => {
                anyhow::bail!("migration '{name}' was modified after being applied");
            }
            Some(_) => continue,
            None => {
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO _migrations (name, checksum) VALUES (?1, ?2)",
                    (&name, &hash),
                )?;
                tracing::info!("applied migration: {name}");
            }
        }
    }

    Ok(())
}
