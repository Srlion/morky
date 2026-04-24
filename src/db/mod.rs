#![allow(unused)]

use std::sync::{OnceLock, mpsc};

use rusqlite::types::{ToSqlOutput, Value};

mod hooks;
mod migrate;

pub use hooks::{Action, UpdateEvent, on_update};
pub use rusqlite::{Connection, Row, types::ToSql};

pub type Result<T> = rusqlite::Result<T>;

type Work = Box<dyn FnOnce(&mut Connection) + Send>;

static DB: OnceLock<mpsc::Sender<Work>> = OnceLock::new();

pub async fn init() -> anyhow::Result<()> {
    let path = crate::constants::db_path();
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap())?;

    let mut conn = Connection::open(path)?;
    conn.set_prepared_statement_cache_capacity(120);
    conn.execute_batch(
        "PRAGMA journal_mode = DELETE;
        PRAGMA busy_timeout = 30000;
        PRAGMA synchronous = FULL;
        PRAGMA cache_size = -8000;
        PRAGMA foreign_keys = ON;
        PRAGMA temp_store = FILE;",
    )?;

    migrate::migrate(&conn)?;

    hooks::install(&conn)?;

    let (tx, rx) = mpsc::channel::<Work>();

    std::thread::spawn(move || {
        while let Ok(work) = rx.recv() {
            work(&mut conn);
        }
    });

    DB.set(tx)
        .map_err(|_| anyhow::anyhow!("DB already initialized"))?;

    Ok(())
}

fn send<T: Send + 'static>(
    f: impl FnOnce(&mut Connection) -> T + Send + 'static,
) -> impl std::future::Future<Output = T> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    DB.get()
        .expect("DB not initialized")
        .send(Box::new(move |c| {
            let _ = tx.send(f(c));
        }))
        .expect("DB thread died");
    async { rx.await.expect("DB thread died") }
}

pub fn conn() -> Db {
    Db
}

pub trait FromRow: Sized {
    fn from_row(row: &Row) -> rusqlite::Result<Self>;
}

fn resolve(v: &dyn ToSql) -> rusqlite::Result<Value> {
    match v.to_sql()? {
        ToSqlOutput::Owned(v) => Ok(v),
        ToSqlOutput::Borrowed(vr) => Ok(Value::try_from(vr)?),
        #[allow(unreachable_patterns)]
        _ => Err(rusqlite::Error::InvalidParameterName(
            "unsupported ToSqlOutput variant for async bind".into(),
        )),
    }
}

fn p(params: &[Value]) -> Vec<&dyn ToSql> {
    params.iter().map(|v| v as &dyn ToSql).collect()
}

fn p_boxed<'a>(params: &'a [Box<dyn ToSql + Send + 'a>]) -> Vec<&'a dyn ToSql> {
    params.iter().map(|b| b.as_ref() as &dyn ToSql).collect()
}

pub struct Db;

impl Db {
    pub fn query(&self, sql: &str) -> Q {
        Q {
            sql: sql.into(),
            params: Ok(vec![]),
        }
    }

    pub fn query_as<T: FromRow>(&self, sql: &str) -> QAs<T> {
        QAs {
            sql: sql.into(),
            params: Ok(vec![]),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn query_scalar<T: rusqlite::types::FromSql>(&self, sql: &str) -> QScalar<T> {
        QScalar {
            sql: sql.into(),
            params: Ok(vec![]),
            _marker: std::marker::PhantomData,
        }
    }

    /// The closure runs synchronously on the dedicated db thread.
    /// Tx methods are **not** async - just call them directly.
    pub async fn txn<T, F>(&self, f: F) -> rusqlite::Result<T>
    where
        T: Send + 'static,
        F: FnOnce(Tx) -> Result<T> + Send + 'static,
    {
        send(move |c| {
            let tx = c.transaction()?;
            let r = f(Tx(&tx))?;
            tx.commit()?;
            Ok(r)
        })
        .await
    }
}

pub struct Q {
    sql: String,
    params: rusqlite::Result<Vec<Value>>,
}

#[allow(dead_code)]
pub struct ExecResult {
    rows_affected: u64,
    last_insert_rowid: i64,
}

impl ExecResult {
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.last_insert_rowid
    }
}

impl Q {
    pub fn bind(mut self, v: impl ToSql) -> Self {
        if let Ok(ref mut params) = self.params {
            match resolve(&v) {
                Ok(val) => params.push(val),
                Err(e) => self.params = Err(e),
            }
        }
        self
    }

    pub async fn execute(self) -> rusqlite::Result<ExecResult> {
        let params = self.params?;
        send(move |c| {
            c.prepare_cached(&self.sql)?
                .execute(p(&params).as_slice())?;
            Ok(ExecResult {
                rows_affected: c.changes(),
                last_insert_rowid: c.last_insert_rowid(),
            })
        })
        .await
    }
}

pub struct QAs<T> {
    sql: String,
    params: rusqlite::Result<Vec<Value>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: FromRow + Send + 'static> QAs<T> {
    pub fn bind(mut self, v: impl ToSql) -> Self {
        if let Ok(ref mut params) = self.params {
            match resolve(&v) {
                Ok(val) => params.push(val),
                Err(e) => self.params = Err(e),
            }
        }
        self
    }

    pub async fn fetch_one(self) -> rusqlite::Result<T> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            s.query_row(p(&params).as_slice(), T::from_row)
        })
        .await
    }

    pub async fn fetch_optional(self) -> rusqlite::Result<Option<T>> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            let mut rows = s.query(p(&params).as_slice())?;
            Ok(rows.next()?.map(T::from_row).transpose()?)
        })
        .await
    }

    pub async fn fetch_all(self) -> rusqlite::Result<Vec<T>> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            let rows = s.query_map(p(&params).as_slice(), T::from_row)?;
            rows.collect()
        })
        .await
    }
}

pub struct QScalar<T> {
    sql: String,
    params: rusqlite::Result<Vec<Value>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: rusqlite::types::FromSql + Send + 'static> QScalar<T> {
    pub fn bind(mut self, v: impl ToSql) -> Self {
        if let Ok(ref mut params) = self.params {
            match resolve(&v) {
                Ok(val) => params.push(val),
                Err(e) => self.params = Err(e),
            }
        }
        self
    }

    pub async fn fetch_one(self) -> rusqlite::Result<T> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            s.query_row(p(&params).as_slice(), |r| r.get(0))
        })
        .await
    }

    pub async fn fetch_optional(self) -> rusqlite::Result<Option<T>> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            let mut rows = s.query(p(&params).as_slice())?;
            rows.next()?.map(|r| r.get(0)).transpose()
        })
        .await
    }

    pub async fn fetch_all(self) -> rusqlite::Result<Vec<T>> {
        let params = self.params?;
        send(move |c| {
            let mut s = c.prepare_cached(&self.sql)?;
            let rows = s.query_map(p(&params).as_slice(), |r| r.get(0))?;
            rows.collect()
        })
        .await
    }
}

pub struct Tx<'a>(&'a rusqlite::Transaction<'a>);

impl<'a> Tx<'a> {
    pub fn query(&self, sql: &str) -> TxQ<'a, '_> {
        TxQ {
            tx: self,
            sql: sql.into(),
            params: vec![],
        }
    }

    pub fn query_as<T: FromRow>(&self, sql: &str) -> TxQAs<'a, '_, T> {
        TxQAs {
            tx: self,
            sql: sql.into(),
            params: vec![],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn query_scalar<T: rusqlite::types::FromSql>(&self, sql: &str) -> TxQScalar<'a, '_, T> {
        TxQScalar {
            tx: self,
            sql: sql.into(),
            params: vec![],
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct TxQ<'a, 'b> {
    tx: &'b Tx<'a>,
    sql: String,
    params: Vec<Box<dyn ToSql + Send + 'b>>,
}

impl<'a, 'b> TxQ<'a, 'b> {
    pub fn bind(mut self, v: impl ToSql + Send + 'b) -> Self {
        self.params.push(Box::new(v));
        self
    }

    pub fn execute(self) -> rusqlite::Result<ExecResult> {
        self.tx
            .0
            .prepare_cached(&self.sql)?
            .execute(p_boxed(&self.params).as_slice())?;
        Ok(ExecResult {
            rows_affected: self.tx.0.changes(),
            last_insert_rowid: self.tx.0.last_insert_rowid(),
        })
    }
}

pub struct TxQAs<'a, 'b, T> {
    tx: &'b Tx<'a>,
    sql: String,
    params: Vec<Box<dyn ToSql + Send + 'b>>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, 'b, T: FromRow> TxQAs<'a, 'b, T> {
    pub fn bind(mut self, v: impl ToSql + Send + 'b) -> Self {
        self.params.push(Box::new(v));
        self
    }

    pub fn fetch_one(self) -> rusqlite::Result<T> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        s.query_row(p_boxed(&self.params).as_slice(), T::from_row)
    }

    pub fn fetch_optional(self) -> rusqlite::Result<Option<T>> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        let mut rows = s.query(p_boxed(&self.params).as_slice())?;
        Ok(rows.next()?.map(T::from_row).transpose()?)
    }

    pub fn fetch_all(self) -> rusqlite::Result<Vec<T>> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        let rows = s.query_map(p_boxed(&self.params).as_slice(), T::from_row)?;
        rows.collect()
    }
}

pub struct TxQScalar<'a, 'b, T> {
    tx: &'b Tx<'a>,
    sql: String,
    params: Vec<Box<dyn ToSql + Send + 'b>>,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, 'b, T: rusqlite::types::FromSql> TxQScalar<'a, 'b, T> {
    pub fn bind(mut self, v: impl ToSql + Send + 'b) -> Self {
        self.params.push(Box::new(v));
        self
    }

    pub fn fetch_one(self) -> rusqlite::Result<T> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        s.query_row(p_boxed(&self.params).as_slice(), |r| r.get(0))
    }

    pub fn fetch_optional(self) -> rusqlite::Result<Option<T>> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        let mut rows = s.query(p_boxed(&self.params).as_slice())?;
        rows.next()?.map(|r| r.get(0)).transpose()
    }

    pub fn fetch_all(self) -> rusqlite::Result<Vec<T>> {
        let mut s = self.tx.0.prepare_cached(&self.sql)?;
        let rows = s.query_map(p_boxed(&self.params).as_slice(), |r| r.get(0))?;
        rows.collect()
    }
}

impl<T: rusqlite::types::FromSql> FromRow for (T,) {
    fn from_row(r: &Row) -> rusqlite::Result<Self> {
        Ok((r.get(0)?,))
    }
}

macro_rules! impl_from_row_single {
    ($($T:ty),+) => {
        $(impl FromRow for $T {
            fn from_row(r: &Row) -> rusqlite::Result<Self> {
                r.get(0)
            }
        })+
    };
}

impl_from_row_single!(bool, i8, i16, i32, i64, u8, u16, u32, f64, String, Vec<u8>);

macro_rules! impl_from_row_tuple {
    ($($idx:tt $T:ident),+) => {
        impl<$($T: rusqlite::types::FromSql),+> FromRow for ($($T,)+) {
            fn from_row(r: &Row) -> rusqlite::Result<Self> {
                Ok(($(r.get($idx)?,)+))
            }
        }
    };
}

impl_from_row_tuple!(0 A, 1 B);
impl_from_row_tuple!(0 A, 1 B, 2 C);
impl_from_row_tuple!(0 A, 1 B, 2 C, 3 D);
impl_from_row_tuple!(0 A, 1 B, 2 C, 3 D, 4 E);
impl_from_row_tuple!(0 A, 1 B, 2 C, 3 D, 4 E, 5 F);
