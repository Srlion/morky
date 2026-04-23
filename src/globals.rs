use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use maw::prelude::*;
use serde_json::{Value, json};
use tokio::sync::broadcast;

static GLOBALS: std::sync::LazyLock<Globals> = std::sync::LazyLock::new(|| Globals::new());

#[inline]
pub fn set(key: impl Into<String>, val: impl Into<Value>) {
    GLOBALS.set(key, val);
}

#[derive(Clone)]
pub struct Globals {
    state: Arc<Mutex<HashMap<String, Value>>>,
    tx: broadcast::Sender<(String, Value)>,
}

impl Globals {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            tx,
        }
    }

    #[inline]
    fn set(&self, key: impl Into<String>, val: impl Into<Value>) {
        let key = key.into();
        let val = val.into();
        self.state.lock().unwrap().insert(key.clone(), val.clone());
        let _ = self.tx.send((key, val));
    }

    fn snapshot(&self) -> HashMap<String, Value> {
        self.state.lock().unwrap().clone()
    }

    fn subscribe(&self) -> broadcast::Receiver<(String, Value)> {
        self.tx.subscribe()
    }
}

pub async fn sse_handler(c: &mut Ctx) {
    let snapshot = GLOBALS.snapshot();
    let mut rx = GLOBALS.subscribe();

    let init = format!("data: {}\n\n", json!({ "snapshot": snapshot }));

    c.res.sse(async_stream::stream! {
        yield Ok::<Bytes, Box<dyn std::error::Error + Send + Sync>>(Bytes::from(init));

        loop {
            match rx.recv().await {
                Ok((k, v)) => {
                    let msg = format!("data: {}\n\n", json!({ "k": k, "v": v }));
                    yield Ok(Bytes::from(msg));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });
}
