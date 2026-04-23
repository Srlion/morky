use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use maw::CancellationToken;

static TOKENS: LazyLock<Mutex<HashMap<i64, CancellationToken>>> = LazyLock::new(Default::default);

fn create(deploy_id: i64) -> CancellationToken {
    let token = CancellationToken::new();
    TOKENS.lock().unwrap().insert(deploy_id, token.clone());
    token
}

pub fn cancel(deploy_id: i64) -> bool {
    TOKENS
        .lock()
        .unwrap()
        .get(&deploy_id)
        .map(|t| {
            t.cancel();
            true
        })
        .unwrap_or(false)
}

pub fn remove(deploy_id: i64) {
    TOKENS.lock().unwrap().remove(&deploy_id);
}

pub async fn run<Fut, T, E>(deploy_id: i64, f: impl FnOnce() -> Fut) -> Result<T, E>
where
    Fut: Future<Output = Result<T, E>>,
    E: From<&'static str>,
{
    let token = create(deploy_id);
    let result = token
        .run_until_cancelled(f())
        .await
        .ok_or_else(|| "cancelled".into());
    remove(deploy_id);
    result?
}
