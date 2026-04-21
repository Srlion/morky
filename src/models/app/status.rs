use std::fmt;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStatus {
    /// App created but never deployed, or manually stopped.
    Idle,
    /// A deployment is currently in progress (building or starting).
    Deploying,
    /// App is live and serving traffic.
    Running,
    /// Last deployment failed.
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployStatus {
    /// Source cloned, image being built.
    Building,
    /// Image built, container being started.
    Deploying,
    /// Completed successfully, image available for rollback.
    Done,
    /// Build or startup failed.
    Failed,
    /// Cancelled.
    Cancelled,
}

macro_rules! impl_status {
    ($ty:ty, $($variant:ident => $str:literal),+ $(,)?) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self { $(Self::$variant => $str),+ })
            }
        }

        impl From<$ty> for serde_json::Value {
            fn from(s: $ty) -> Self {
                serde_json::Value::String(s.to_string())
            }
        }

        impl $ty {
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($str => Some(Self::$variant),)+ _ => None }
            }
        }

        impl ToSql for $ty {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.to_string()))
            }
        }

        impl FromSql for $ty {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                let s = value.as_str()?;
                Self::parse(s).ok_or_else(|| FromSqlError::Other(
                    format!("unknown {} value: {s}", stringify!($ty)).into(),
                ))
            }
        }
    };
}

impl_status!(AppStatus,
    Idle => "idle",
    Deploying => "deploying",
    Running => "running",
    Failed => "failed",
);

impl_status!(DeployStatus,
    Building => "building",
    Deploying => "deploying",
    Done => "done",
    Failed => "failed",
    Cancelled => "cancelled",
);
