use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use tokio::sync::broadcast;

use crate::models::{DeployStatus, Deployment};

static CH: LazyLock<Mutex<HashMap<i64, broadcast::Sender<String>>>> =
    LazyLock::new(Default::default);

pub fn subscribe(id: i64) -> broadcast::Receiver<String> {
    CH.lock()
        .unwrap()
        .entry(id)
        .or_insert_with(|| broadcast::channel(512).0)
        .subscribe()
}

fn broadcast(id: i64, line: &str) {
    if let Some(tx) = CH.lock().unwrap().get(&id) {
        let _ = tx.send(line.to_string());
    }
}

pub fn remove(id: i64) {
    CH.lock().unwrap().remove(&id);
}

pub async fn append_log(id: i64, line: &str) {
    let _ = Deployment::append_log(id, line).await;
    broadcast(id, line);
}

pub fn send_status(id: i64, status: DeployStatus) {
    broadcast(id, &format!("\x01STATUS:{status}"));
}
