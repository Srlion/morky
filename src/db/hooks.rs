use std::sync::{Arc, Mutex, OnceLock, mpsc};

pub use rusqlite::hooks::Action;

#[derive(Clone, Debug)]
pub struct UpdateEvent {
    pub action: Action,
    pub db_name: String,
    pub table: String,
    pub row_id: i64,
}

pub type Listener = Arc<dyn Fn(&UpdateEvent) + Send + Sync>;

static LISTENERS: OnceLock<Mutex<Vec<Listener>>> = OnceLock::new();
fn listeners() -> &'static Mutex<Vec<Listener>> {
    LISTENERS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn on_update<F>(f: F)
where
    F: Fn(&UpdateEvent) + Send + Sync + 'static,
{
    let l: Listener = Arc::new(f);

    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::spawn_blocking(move || {
            listeners().lock().unwrap().push(l);
        });
    } else {
        listeners().lock().unwrap().push(l);
    }
}

pub(super) fn install(conn: &rusqlite::Connection) -> crate::db::Result<()> {
    let (tx, rx) = mpsc::channel::<UpdateEvent>();

    std::thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            let cbs = listeners().lock().unwrap();
            for cb in cbs.iter() {
                cb(&ev);
            }
        }
    });

    conn.update_hook(Some(move |action, db_name: &str, tbl: &str, row_id| {
        let _ = tx.send(UpdateEvent {
            action,
            db_name: db_name.to_owned(),
            table: tbl.to_owned(),
            row_id,
        });
    }))
}
