use std::collections::HashMap;
use std::fmt::Display;
use std::pin::Pin;
use std::sync::{Arc, LazyLock, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::db::{self, FromRow, Row, conn};
use crate::globals;

mod worker;

/// Implement this on any struct to make it a job.
pub trait Job: Serialize + DeserializeOwned + Display + Send + Sync + 'static {
    /// Unique name for this job type (e.g. "deploy", "send_email").
    const NAME: &'static str;

    /// Max retry attempts. 0 = no retries.
    const MAX_RETRIES: i32 = 0;

    /// If true, this job shares a global CPU-bounded slot limit.
    const CPU_BOUND: bool = false;

    /// If true, no other jobs can be running when this starts,
    /// and no other jobs can start while this is running.
    const EXCLUSIVE: bool = false;

    /// If true, only one instance of this job can be pending or running at a time.
    const UNIQUE: bool = false;

    fn run(&self) -> impl Future<Output = Result<(), String>> + Send;
}

type RunFn =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync>;

type DisplayFn = Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

struct JobDef {
    run_fn: RunFn,
    display_fn: DisplayFn,
    max_retries: i32,
    cpu_bound: bool,
    exclusive: bool,
}

static REGISTRY: LazyLock<Mutex<HashMap<&str, Arc<JobDef>>>> = LazyLock::new(Default::default);

/// Register a job type. Call once per type at startup before `start()`.
pub fn register<J: Job>() {
    let def = JobDef {
        run_fn: Arc::new(|payload| {
            Box::pin(async move {
                let job: J = serde_json::from_str(&payload)
                    .map_err(|e| format!("deserialize {}: {e}", J::NAME))?;
                job.run().await
            })
        }),
        display_fn: Arc::new(|payload| {
            let job: J = serde_json::from_str(payload)
                .map_err(|e| format!("deserialize {}: {e}", J::NAME))?;
            Ok(job.to_string())
        }),
        max_retries: J::MAX_RETRIES,
        cpu_bound: J::CPU_BOUND,
        exclusive: J::EXCLUSIVE,
    };
    REGISTRY.lock().unwrap().insert(J::NAME, Arc::new(def));
}

pub(self) fn get_def(name: &str) -> Option<Arc<JobDef>> {
    REGISTRY.lock().unwrap().get(name).cloned()
}

/// Enqueue a job.
pub async fn enqueue<J: Job>(job: &J) -> Result<Option<i64>, String> {
    let payload = serde_json::to_string(job).expect("serialize job");

    if J::UNIQUE {
        let r = conn()
            .query("INSERT INTO jobs (name, payload) VALUES (?, ?) ON CONFLICT DO NOTHING")
            .bind(J::NAME)
            .bind(&payload)
            .execute()
            .await
            .map_err(|e| e.to_string())?;

        if r.rows_affected() == 0 {
            return Err(format!("{} is already running", job));
        }

        worker::notify();
        Ok(Some(r.last_insert_rowid()))
    } else {
        let r = conn()
            .query("INSERT INTO jobs (name, payload) VALUES (?, ?)")
            .bind(J::NAME)
            .bind(&payload)
            .execute()
            .await
            .map_err(|e| e.to_string())?;
        worker::notify();
        Ok(Some(r.last_insert_rowid()))
    }
}

/// Start the worker. Call once after registering all job types.
pub fn start() {
    db::on_update(|event| {
        use db::Action::*;
        if event.table != "jobs" {
            return;
        }
        let row_id = event.row_id;
        match event.action {
            SQLITE_INSERT | SQLITE_UPDATE => {
                crate::tokio_handle().spawn(async move {
                    if let Ok(Some(job)) = crate::jobs::get_job_info(row_id).await {
                        globals::set(format!("job_{row_id}"), job);
                    }
                });
            }
            SQLITE_DELETE => {
                globals::set(format!("job_{row_id}"), ());
            }
            _ => {}
        }
    });

    worker::start();
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("pending"),
            Self::Running => f.write_str("running"),
            Self::Done => f.write_str("done"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

impl rusqlite::types::ToSql for JobStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.to_string()))
    }
}

impl rusqlite::types::FromSql for JobStatus {
    fn column_result(v: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match v.as_str()? {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            s => Err(rusqlite::types::FromSqlError::Other(
                format!("unknown job status: {s}").into(),
            )),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: i64,
    pub name: String,
    pub payload: String,
    pub status: JobStatus,
    pub attempts: i32,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl FromRow for JobRow {
    fn from_row(row: &Row) -> db::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            payload: row.get("payload")?,
            status: row.get("status")?,
            attempts: row.get("attempts")?,
            error: row.get("error")?,
            created_at: row.get("created_at")?,
            started_at: row.get("started_at")?,
            finished_at: row.get("finished_at")?,
        })
    }
}

pub async fn get_job_info(id: i64) -> db::Result<Option<serde_json::Value>> {
    let row: Option<(String, String, String, Option<String>)> = conn()
        .query_as("SELECT name, payload, status, error FROM jobs WHERE id = ?")
        .bind(id)
        .fetch_optional()
        .await?;

    Ok(row.map(|(name, payload, status, error)| {
        let display = get_def(&name)
            .and_then(|def| (def.display_fn)(&payload).ok())
            .unwrap_or(name.clone());

        serde_json::json!({
            "id": id,
            "name": name,
            "display": display,
            "payload": serde_json::from_str::<serde_json::Value>(&payload).unwrap_or_default(),
            "status": status,
            "error": error,
        })
    }))
}
