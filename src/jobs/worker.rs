use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst};
use std::time::Duration;

use tokio::sync::{Notify, Semaphore};

use crate::db::conn;
use crate::deploy::buildkit::cpu_limit;

use super::{JobRow, get_def};

static WAKE: LazyLock<Notify> = LazyLock::new(Notify::new);
static CPU_SEM: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(cpu_limit()));

static RUNNING: AtomicUsize = AtomicUsize::new(0);
static EXCLUSIVE: AtomicBool = AtomicBool::new(false);

pub fn notify() {
    WAKE.notify_one();
}

pub fn start() {
    // Mark anything that was running as pending (server crashed mid-job)
    tokio::spawn(async {
        let r = conn()
            .query("UPDATE jobs SET status = 'pending' WHERE status = 'running'")
            .execute()
            .await;
        if let Ok(r) = r {
            if r.rows_affected() > 0 {
                tracing::info!("reset {} interrupted jobs to pending", r.rows_affected());
            }
        }
        notify();
    });

    tokio::spawn(poll_loop());
}

async fn poll_loop() {
    loop {
        let jobs: Vec<JobRow> = conn()
            .query_as(
                "SELECT * FROM jobs WHERE status = 'pending' \
                 ORDER BY created_at ASC LIMIT 20",
            )
            .fetch_all()
            .await
            .unwrap_or_default();

        if jobs.is_empty() {
            tokio::select! {
                _ = WAKE.notified() => {}
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            continue;
        }

        for job in jobs {
            let Some(def) = get_def(&job.name) else {
                tracing::warn!(
                    id = job.id,
                    name = job.name,
                    "no handler registered, skipping"
                );
                let _ = conn()
                    .query("UPDATE jobs SET status = 'failed', error = 'no handler registered', finished_at = unixepoch() WHERE id = ?")
                    .bind(job.id)
                    .execute()
                    .await;
                continue;
            };

            // Strict ordering: if we can't start this job, stop and wait.
            // Don't skip to the next one.

            // An exclusive job is running - nothing else can start
            if EXCLUSIVE.load(SeqCst) {
                break;
            }

            // This job is exclusive - wait for everything to finish
            if def.exclusive && RUNNING.load(SeqCst) > 0 {
                break;
            }

            // CPU-bound - need a permit
            let permit = if def.cpu_bound {
                match CPU_SEM.try_acquire() {
                    Ok(p) => Some(p),
                    Err(_) => break,
                }
            } else {
                None
            };

            let claimed = conn()
                .query(
                    "UPDATE jobs SET status = 'running', started_at = unixepoch(), \
                     attempts = attempts + 1 \
                     WHERE id = ? AND status = 'pending'",
                )
                .bind(job.id)
                .execute()
                .await;

            match claimed {
                Ok(r) if r.rows_affected() == 0 => continue, // someone else got it
                Err(e) => {
                    tracing::error!(id = job.id, "claim job: {e}");
                    continue;
                }
                _ => {}
            }

            RUNNING.fetch_add(1, SeqCst);
            if def.exclusive {
                EXCLUSIVE.store(true, SeqCst);
            }

            let id = job.id;
            let attempts = job.attempts + 1;
            let payload = job.payload.clone();
            let run_fn = def.run_fn.clone();
            tokio::spawn(async move {
                let _permit = permit;

                let result = run_fn(payload).await;

                match result {
                    Ok(()) => {
                        let _ = conn()
                            .query(
                                "UPDATE jobs SET status = 'done', finished_at = unixepoch() \
                                 WHERE id = ?",
                            )
                            .bind(id)
                            .execute()
                            .await;
                    }
                    Err(e) => {
                        tracing::error!(id, "job failed: {e}");
                        if attempts <= def.max_retries {
                            tracing::info!(id, attempts, def.max_retries, "retrying");
                            let _ = conn()
                                .query("UPDATE jobs SET status = 'pending', error = ? WHERE id = ?")
                                .bind(&e)
                                .bind(id)
                                .execute()
                                .await;
                        } else {
                            let _ = conn()
                                .query(
                                    "UPDATE jobs SET status = 'failed', error = ?, \
                                     finished_at = unixepoch() WHERE id = ?",
                                )
                                .bind(&e)
                                .bind(id)
                                .execute()
                                .await;
                        }
                    }
                }

                if def.exclusive {
                    EXCLUSIVE.store(false, SeqCst);
                }
                RUNNING.fetch_sub(1, SeqCst);
                notify();
            });
        }

        // Small yield to not spin if there are many jobs
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
