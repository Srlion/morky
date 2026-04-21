pub mod github;
pub mod ops;

use crate::models::GitSource;
use github::GithubData;
use maw::prelude::*;
use serde::Deserialize;

pub fn api_routes() -> Router {
    Router::group("/git-sources")
        .get("/", list)
        .put("/{id}/name", rename)
        .get("/{id}/repos", list_repos)
        .get("/{id}/branches", list_branches)
        .push(github::api_routes())
}

pub fn browser_routes() -> Router {
    Router::group("/git-sources").push(github::browser_routes())
}

async fn list(c: &mut Ctx) {
    c.res.json(GitSource::list().await.unwrap_or_default());
}

#[derive(Deserialize)]
struct RenameBody {
    name: String,
}

async fn rename(c: &mut Ctx) {
    let Ok(id) = c.req.param::<i64>("id") else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "invalid id"}));
    };
    let body: RenameBody = match c.req.json().await {
        Ok(b) => b,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": "invalid body"}));
        }
    };
    let name = body.name.trim();
    if name.is_empty() {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "name required"}));
    }
    match GitSource::update_name(id, name).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                "name already exists"
            } else {
                "failed to rename"
            };
            c.res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": msg}));
        }
    }
}

async fn list_repos(c: &mut Ctx) {
    let Some(data) = get_github_data(c).await else {
        return;
    };
    match data.list_repos().await {
        Ok(repos) => {
            let list: Vec<serde_json::Value> = repos
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "full_name": r.full_name, "name": r.name,
                        "private": r.private, "default_branch": r.default_branch,
                    })
                })
                .collect();
            c.res.json(&list);
        }
        Err(e) => c
            .res
            .status(StatusCode::BAD_GATEWAY)
            .json(&serde_json::json!({"error": e})),
    }
}

#[derive(Deserialize)]
struct BranchQuery {
    repo: String,
}

async fn list_branches(c: &mut Ctx) {
    let repo = match c.req.query::<BranchQuery>() {
        Ok(q) => q.repo,
        Err(_) => {
            return c
                .res
                .status(StatusCode::BAD_REQUEST)
                .json(&serde_json::json!({"error": "missing ?repo="}));
        }
    };
    let Some(data) = get_github_data(c).await else {
        return;
    };
    match data.list_branches(&repo).await {
        Ok(b) => c.res.json(&b),
        Err(e) => c
            .res
            .status(StatusCode::BAD_GATEWAY)
            .json(&serde_json::json!({"error": e})),
    }
}

async fn get_github_data(c: &mut Ctx) -> Option<GithubData> {
    let id: i64 = c.req.param("id").ok()?;
    let source = GitSource::get_by_id(id).await.ok()?;
    let data: GithubData = source.provider_data.try_into().ok()?;
    if data.installation_id.is_none() {
        c.res
            .status(StatusCode::BAD_GATEWAY)
            .json(&serde_json::json!({"error": "not installed"}));
        return None;
    }
    Some(data)
}

pub async fn clone_repo(
    git_source_id: i64,
    repo: &str,
    branch: &str,
    dest: &str,
    cwd: &str,
) -> Result<(), String> {
    let gs = GitSource::get_by_id(git_source_id)
        .await
        .map_err(|e| format!("load git source: {e}"))?;
    match gs.provider.as_str() {
        "github" => {
            let gh: github::GithubData = gs
                .provider_data
                .try_into()
                .map_err(|e| format!("parse: {e}"))?;
            gh.clone_repo(repo, branch, dest, cwd).await
        }
        other => Err(format!("unsupported provider: {other}")),
    }
}

pub async fn clone_repo_at_commit(
    git_source_id: i64,
    repo: &str,
    commit: &str,
    dest: &str,
    cwd: &str,
) -> Result<(), String> {
    let gs = GitSource::get_by_id(git_source_id)
        .await
        .map_err(|e| format!("load git source: {e}"))?;
    match gs.provider.as_str() {
        "github" => {
            let gh: github::GithubData = gs
                .provider_data
                .try_into()
                .map_err(|e| format!("parse: {e}"))?;
            gh.clone_repo_at_commit(repo, commit, dest, cwd).await
        }
        other => Err(format!("unsupported provider: {other}")),
    }
}
