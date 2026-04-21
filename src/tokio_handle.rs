use std::sync::OnceLock;

use tokio::runtime::Handle;

static TOKIO_HANDLE: OnceLock<Handle> = OnceLock::new();

pub fn tokio_handle() -> &'static Handle {
    TOKIO_HANDLE.get().expect("TOKIO_HANDLE not set")
}

pub(super) async fn init() {
    TOKIO_HANDLE
        .set(Handle::current())
        .expect("failed to set TOKIO_HANDLE");
}
