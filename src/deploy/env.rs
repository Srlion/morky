use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::common::hex;
use crate::models::{App, Project};

const APP_DIR_KEY: &str = "MORKY_DIR";
const APP_DIR_VAL: &str = "/app";

pub enum EnvMode {
    /// `--env KEY=VALUE`
    Runtime,
    /// `--secret id=KEY,env=KEY`
    Build,
}

pub fn inject_env(user_env: &HashMap<String, String>, mode: EnvMode) -> Vec<String> {
    match mode {
        EnvMode::Runtime => user_env
            .iter()
            .flat_map(|(k, v)| ["--env".into(), format!("{k}={v}")])
            .collect(),
        EnvMode::Build => user_env
            .iter()
            .flat_map(|(k, _)| ["--secret".into(), format!("id={k},env={k}")])
            .collect(),
    }
}

pub fn secrets_hash(env_vars: &HashMap<String, String>) -> String {
    let mut h = Sha256::new();
    for (k, v) in env_vars {
        h.update(format!("{k}={v}\n"));
    }
    hex::encode(h.finalize().as_slice())
}

pub fn build_env_vars(app: &App, project: &Project) -> HashMap<String, String> {
    let mut env = project.env_vars();
    env.extend(app.env_vars());
    env.insert("PORT".into(), app.port.to_string());
    env.insert(APP_DIR_KEY.into(), APP_DIR_VAL.into());
    env
}
