use maw::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::common::{self, single_use_token};
use crate::models::{GitSource, Settings};

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubData {
    pub app_id: i64,
    pub app_slug: String,
    pub client_id: String,
    pub client_secret: String,
    pub pem: String,
    pub webhook_secret: String,
    pub owner_type: String,
    pub owner_login: String,
    pub html_url: String,
    #[serde(default)]
    pub installation_id: Option<i64>,
}

impl TryFrom<Value> for GithubData {
    type Error = serde_json::Error;
    fn try_from(v: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(v)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub private: bool,
    pub html_url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub default_branch: String,
    pub clone_url: String,
}

fn gh(method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
    crate::http_client()
        .request(method, url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "morky")
}

async fn gh_ok(resp: reqwest::Response) -> Result<Value, String> {
    if resp.status().is_success() {
        return resp.json().await.map_err(|e| format!("json: {e}"));
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(format!("GitHub {status}: {body}"))
}

async fn current_base_url() -> Result<String, &'static str> {
    match Settings::get().await.ok().and_then(|s| s.panel_domain) {
        Some(d) => Ok(format!("https://{d}")),
        None => common::public_base_url().await.map_err(
            |_| "could not determine host: no domain configured and failed to get public IP",
        ),
    }
}

impl GithubData {
    fn make_jwt(&self) -> Result<String, String> {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
        let now = chrono::Utc::now().timestamp();
        let claims = serde_json::json!({
            "iat": now - 60,
            "exp": now + (10 * 60),
            "iss": self.app_id.to_string(),
        });
        let key =
            EncodingKey::from_rsa_pem(self.pem.as_bytes()).map_err(|e| format!("bad PEM: {e}"))?;
        encode(&Header::new(Algorithm::RS256), &claims, &key).map_err(|e| format!("jwt: {e}"))
    }

    async fn owns_installation(&self, iid: i64) -> Result<bool, String> {
        let jwt = self.make_jwt()?;
        let resp = gh(
            reqwest::Method::GET,
            &format!("https://api.github.com/app/installations/{iid}"),
        )
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .map_err(|e| format!("request: {e}"))?;

        let ok = resp.status().is_success();
        if !ok {
            let body = resp.text().await.unwrap_or_default();
            tracing::debug!(
                app_id = self.app_id,
                iid,
                "owns_installation failed: {body}"
            );
        }
        Ok(ok)
    }

    pub async fn get_installation_token(&self) -> Result<String, String> {
        let iid = self.installation_id.ok_or("App not installed")?;
        let jwt = self.make_jwt()?;
        let resp = gh(
            reqwest::Method::POST,
            &format!("https://api.github.com/app/installations/{iid}/access_tokens"),
        )
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .map_err(|e| format!("request: {e}"))?;

        let body = gh_ok(resp).await?;
        body["token"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "no token in response".into())
    }

    pub async fn list_repos(&self) -> Result<Vec<GithubRepo>, String> {
        let token = self.get_installation_token().await?;
        let mut repos = Vec::new();
        let mut page = 1u32;
        loop {
            let resp = gh(
                reqwest::Method::GET,
                "https://api.github.com/installation/repositories",
            )
            .query(&[("per_page", "100"), ("page", &page.to_string())])
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| format!("request: {e}"))?;

            let body = gh_ok(resp).await?;
            let batch: Vec<GithubRepo> =
                serde_json::from_value(body["repositories"].clone()).unwrap_or_default();
            let done = batch.len() < 100;
            repos.extend(batch);
            if done {
                break;
            }
            page += 1;
        }
        Ok(repos)
    }

    pub async fn list_branches(&self, repo: &str) -> Result<Vec<String>, String> {
        let token = self.get_installation_token().await?;
        let mut branches = Vec::new();
        let mut page = 1u32;
        loop {
            let resp = gh(
                reqwest::Method::GET,
                &format!("https://api.github.com/repos/{repo}/branches"),
            )
            .query(&[("per_page", "100"), ("page", &page.to_string())])
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| format!("request: {e}"))?;

            let batch: Vec<Value> = gh_ok(resp)
                .await
                .and_then(|v| serde_json::from_value(v).map_err(|e| format!("json: {e}")))?;
            let done = batch.len() < 100;
            branches.extend(
                batch
                    .into_iter()
                    .filter_map(|b| b["name"].as_str().map(String::from)),
            );
            if done {
                break;
            }
            page += 1;
        }
        Ok(branches)
    }

    pub async fn clone_repo(
        &self,
        repo: &str,
        branch: &str,
        dest: &str,
        cwd: &str,
    ) -> Result<(), String> {
        let token = self.get_installation_token().await?;
        let url = format!("https://x-access-token:{token}@github.com/{repo}.git");
        super::ops::clone(&url, branch, dest, cwd).await
    }

    pub async fn clone_repo_at_commit(
        &self,
        repo: &str,
        commit: &str,
        dest: &str,
        cwd: &str,
    ) -> Result<(), String> {
        let token = self.get_installation_token().await?;
        let url = format!("https://x-access-token:{token}@github.com/{repo}.git");
        super::ops::clone_at_commit(&url, commit, dest, cwd).await
    }
}

pub fn browser_routes() -> Router {
    Router::group("/github")
        .get("/callback/{token}", callback)
        .get("/install-callback", install_callback)
        .post("/{id}/install", install_redirect)
}

pub fn api_routes() -> Router {
    Router::group("/github")
        .post("/create-manifest", create_manifest)
        .delete("/{id}", delete)
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
}

async fn callback(c: &mut Ctx) {
    let url_token = c.req.param_str("token");
    if !single_use_token::verify(url_token) {
        return render_err(c, "Invalid or expired callback token.");
    }
    let q = match c.req.query::<CallbackQuery>() {
        Ok(q) => q,
        Err(_) => return render_err(c, "Missing parameters."),
    };

    let resp = gh(
        reqwest::Method::POST,
        &format!(
            "https://api.github.com/app-manifests/{}/conversions",
            q.code
        ),
    )
    .send()
    .await;

    let body: Value = match resp {
        Ok(r) => match gh_ok(r).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("github conversion: {e}");
                return render_err(c, "GitHub conversion failed.");
            }
        },
        Err(e) => {
            tracing::error!("github manifest conversion: {e}");
            return render_err(c, "Failed to contact GitHub.");
        }
    };

    let s = |k: &str| body[k].as_str().unwrap_or("").to_string();
    let data = GithubData {
        app_id: body["id"].as_i64().unwrap_or(0),
        app_slug: s("slug"),
        client_id: s("client_id"),
        client_secret: s("client_secret"),
        pem: s("pem"),
        webhook_secret: s("webhook_secret"),
        owner_type: body["owner"]["type"].as_str().unwrap_or("User").to_string(),
        owner_login: body["owner"]["login"].as_str().unwrap_or("").to_string(),
        html_url: s("html_url"),
        installation_id: None,
    };

    let name = if data.owner_type == "Organization" {
        format!("{} ({})", data.app_slug, data.owner_login)
    } else {
        data.app_slug.clone()
    };

    if let Err(e) = GitSource::create("github", &name, serde_json::to_value(&data).unwrap()).await {
        tracing::error!("save git source: {e}");
        return render_err(c, "Failed to save source.");
    }
    c.res.redirect("/git-sources", None);
}

async fn install_redirect(c: &mut Ctx) {
    let id: i64 = c.req.param("id").unwrap_or(0);
    let source = match GitSource::get_by_id(id).await {
        Ok(s) => s,
        _ => return render_err(c, "Source not found."),
    };
    let data: GithubData = match source.provider_data.try_into() {
        Ok(d) => d,
        Err(_) => return render_err(c, "Corrupt source data."),
    };
    c.res.redirect(
        &format!(
            "https://github.com/apps/{}/installations/new",
            data.app_slug
        ),
        None,
    );
}

#[derive(Deserialize)]
struct InstallCallbackQuery {
    #[serde(default)]
    installation_id: Option<i64>,
}

async fn install_callback(c: &mut Ctx) {
    let q = c
        .req
        .query::<InstallCallbackQuery>()
        .unwrap_or(InstallCallbackQuery {
            installation_id: None,
        });
    let Some(iid) = q.installation_id else {
        return c.res.redirect("/git-sources", None);
    };

    let sources = GitSource::list().await.unwrap_or_default();
    let uninstalled: Vec<_> = sources
        .iter()
        .filter(|s| {
            s.provider == "github"
                && s.provider_data
                    .get("installation_id")
                    .map_or(true, |v| v.is_null())
        })
        .collect();

    // Retry: GitHub API may lag briefly after install
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        for source in &uninstalled {
            let Ok(mut data) = GithubData::try_from(source.provider_data.clone()) else {
                continue;
            };
            match data.owns_installation(iid).await {
                Ok(true) => {
                    data.installation_id = Some(iid);
                    let json = serde_json::to_value(&data).unwrap();
                    if let Err(e) = GitSource::update_provider_data(source.id, json).await {
                        tracing::error!(source_id = source.id, "save installation_id: {e}");
                    }
                    return c.res.redirect("/git-sources", None);
                }
                Err(e) => tracing::warn!(source_id = source.id, attempt, "owns_installation: {e}"),
                _ => {}
            }
        }
    }

    tracing::warn!(iid, "could not match installation after retries");
    c.res.redirect("/git-sources", None);
}

async fn delete(c: &mut Ctx) {
    let Ok(id) = c.req.param::<i64>("id") else {
        return c
            .res
            .status(StatusCode::BAD_REQUEST)
            .json(&serde_json::json!({"error": "invalid id"}));
    };

    let source = match GitSource::get_by_id(id).await {
        Ok(s) => s,
        _ => {
            return c
                .res
                .status(StatusCode::NOT_FOUND)
                .json(&serde_json::json!({"error": "source not found"}));
        }
    };

    let data: GithubData = match source.provider_data.try_into() {
        Ok(d) => d,
        Err(_) => {
            return c
                .res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "corrupt source data"}));
        }
    };

    if let Some(iid) = data.installation_id {
        match data.make_jwt() {
            Ok(jwt) => {
                let resp = gh(
                    reqwest::Method::DELETE,
                    &format!("https://api.github.com/app/installations/{iid}"),
                )
                .header("Authorization", format!("Bearer {jwt}"))
                .send()
                .await;

                match resp {
                    Ok(r) if r.status().is_success() || r.status() == 404 => {}
                    Ok(r) => {
                        let body = r.text().await.unwrap_or_default();
                        tracing::error!(iid, "delete installation: {body}");
                        return c.res.status(StatusCode::BAD_GATEWAY).json(
                            &serde_json::json!({"error": "failed to remove GitHub installation"}),
                        );
                    }
                    Err(e) => {
                        tracing::error!(iid, "delete installation request: {e}");
                        return c
                            .res
                            .status(StatusCode::BAD_GATEWAY)
                            .json(&serde_json::json!({"error": "failed to contact GitHub"}));
                    }
                }
            }
            Err(e) => {
                tracing::error!("make_jwt for delete: {e}");
                return c
                    .res
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .json(&serde_json::json!({"error": "failed to authenticate with GitHub"}));
            }
        }
    }

    match GitSource::delete(id).await {
        Ok(_) => c.res.json(&serde_json::json!({"ok": true})),
        Err(e) => {
            tracing::error!(source_id = id, "delete git source: {e}");
            c.res
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .json(&serde_json::json!({"error": "failed to delete source"}));
        }
    }
}

#[derive(Deserialize)]
struct ManifestBody {
    #[serde(default)]
    org: Option<String>,
}

async fn create_manifest(c: &mut Ctx) {
    let body: ManifestBody = c.req.json().await.unwrap_or(ManifestBody { org: None });
    let token = single_use_token::generate();
    let host = match current_base_url().await {
        Ok(h) => h,
        Err(e) => {
            c.res.status(StatusCode::INTERNAL_SERVER_ERROR);
            return c.res.json(&serde_json::json!({ "message": e }));
        }
    };
    let cb = format!("{host}/git-sources/github/callback/{token}");

    let manifest = serde_json::json!({
        "name": format!("morky-{}", &uuid::Uuid::new_v4().to_string()[..8]),
        "url": &host,
        "hook_attributes": { "url": format!("{host}/git-sources/github/webhook"), "active": false },
        "redirect_url": &cb,
        "callback_urls": [&cb],
        "setup_url": format!("{host}/git-sources/github/install-callback"),
        "setup_on_update": true,
        "public": false,
        "default_permissions": {
            "contents": "read", "metadata": "read",
            "pull_requests": "read", "administration": "read"
        },
        "default_events": ["push", "pull_request"]
    });

    let github_url = match body.org.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(o) => format!("https://github.com/organizations/{o}/settings/apps/new"),
        None => "https://github.com/settings/apps/new".to_string(),
    };

    c.res
        .json(&serde_json::json!({ "github_url": github_url, "manifest": manifest.to_string() }));
}

fn render_err(c: &mut Ctx, msg: &str) {
    let encoded: String = form_urlencoded::byte_serialize(msg.as_bytes()).collect();
    c.res
        .redirect(&format!("/git-sources?error={encoded}"), None);
}
