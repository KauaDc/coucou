// Integration pollers — the Rust side of StripePoller / GithubPoller /
// VercelPoller / N8nPoller / ResendPoller / NotionPoller / CalcomPoller.
//
// Same endpoints, same first-run delays and intervals as the Swift pollers. Each
// one emits an `integration` event; the island owns the badge, the sound and the
// 60 s auto-clear, exactly as the Swift handlers do.
//
// Discloud is Windows-only and the one integration that writes: its start / stop
// / restart commands run only from a click in the island.
//
// Nothing is polled until its key exists in the Credential Manager, and no
// request goes anywhere the user has not configured.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::island::WINDOW_LABEL;
use crate::log;
use crate::secrets;

const TIMEOUT: Duration = Duration::from_secs(10);

/// What the island receives. `event` is only set when something actually changed,
/// which is what drives the pill badge and the sound.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationUpdate {
    pub id: &'static str,
    pub data: Value,
    pub error: Option<String>,
    pub event: Option<IntegrationEvent>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationEvent {
    pub success: bool,
    pub label: String,
    pub detail: Option<String>,
}

fn emit(app: &AppHandle, update: IntegrationUpdate) {
    let _ = app.emit_to(WINDOW_LABEL, "integration", update);
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .unwrap_or_default()
}

/// Set from the tray's Pause item. While it is on, nothing reaches the network:
/// pausing Coucou has to mean pausing Coucou, not just hiding the island.
pub static PAUSED: AtomicBool = AtomicBool::new(false);

pub fn set_paused(on: bool) {
    PAUSED.store(on, Ordering::Relaxed);
}

/// Spawns every poller with the macOS delays and intervals.
pub fn start(app: AppHandle) {
    spawn(app.clone(), "integration_n8n", 3, 15, poll_n8n);
    spawn(app.clone(), "integration_vercel", 5, 30, poll_vercel);
    spawn(app.clone(), "integration_stripe", 6, 30, poll_stripe);
    spawn(app.clone(), "integration_resend", 6, 60, poll_resend);
    spawn(app.clone(), "integration_github", 7, 300, poll_github);
    spawn(app.clone(), "integration_calcom", 8, 300, poll_calcom);
    spawn(app.clone(), "integration_notion", 9, 300, poll_notion);
    // The Discloud rate limit is undocumented: two requests a minute stays well clear.
    spawn(app, "integration_discloud", 10, 60, poll_discloud);
}

/// True when the user has this integration switched on in settings.
fn enabled(app: &AppHandle, id: &str) -> bool {
    app.try_state::<crate::Shared>()
        .map(|shared| {
            let settings = shared.settings.lock().unwrap();
            settings.active_integrations.iter().any(|x| x == id)
        })
        .unwrap_or(false)
}

fn spawn<F, Fut>(app: AppHandle, id: &'static str, delay_secs: u64, every_secs: u64, poll: F)
where
    F: Fn(AppHandle) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(every_secs));
        loop {
            ticker.tick().await;
            // The ticker keeps its cadence; we just decline to do the work. An
            // integration the user switched off, or a paused app, must make no
            // network calls at all — CLAUDE.md allows talking only to services
            // the user configured, and a disabled one is not configured.
            if PAUSED.load(Ordering::Relaxed) || !enabled(&app, id) {
                continue;
            }
            poll(app.clone()).await;
        }
    });
}

/// One-shot refresh from the Refresh buttons in the island.
pub async fn poll_once(app: AppHandle, id: &str) {
    match id {
        "integration_stripe" => poll_stripe(app).await,
        "integration_github" => poll_github(app).await,
        "integration_vercel" => poll_vercel(app).await,
        "integration_n8n" => poll_n8n(app).await,
        "integration_resend" => poll_resend(app).await,
        "integration_notion" => poll_notion(app).await,
        "integration_calcom" => poll_calcom(app).await,
        "integration_discloud" => poll_discloud(app).await,
        _ => {}
    }
}

/// Remembers the newest id per integration so an event fires once, not on every poll.
struct Seen(Mutex<std::collections::HashMap<&'static str, String>>);

static SEEN: std::sync::LazyLock<Seen> =
    std::sync::LazyLock::new(|| Seen(Mutex::new(std::collections::HashMap::new())));

/// Returns true the first time a given id is seen (and false on the very first
/// load, which only fills the card).
fn is_new(key: &'static str, id: &str) -> bool {
    let mut map = SEEN.0.lock().unwrap();
    match map.insert(key, id.to_string()) {
        Some(previous) => previous != id,
        None => false, // first poll: populate silently, like the Swift pollers
    }
}

fn status_error(code: u16, unauthorised_hint: &str) -> String {
    match code {
        401 => crate::i18n::t("Invalid API key (401)", "Chave de API inválida (401)").into(),
        403 => unauthorised_hint.into(),
        _ => if crate::i18n::pt() { format!("Erro da API {code}") } else { format!("API error {code}") },
    }
}

// ── Stripe ────────────────────────────────────────────────────────────────────

async fn poll_stripe(app: AppHandle) {
    let Some(key) = secrets::get("stripe-api-key") else { return };
    let auth = format!("Basic {}", crate::chat::base64_for(format!("{key}:").as_bytes()));
    let http = client();

    let balance = http
        .get("https://api.stripe.com/v1/balance")
        .header("Authorization", &auth)
        .send()
        .await;

    let (amount, currency) = match balance {
        Ok(r) if r.status().is_success() => {
            let json: Value = r.json().await.unwrap_or(json!({}));
            let mut buckets: Vec<Value> = Vec::new();
            for k in ["available", "pending"] {
                if let Some(arr) = json.get(k).and_then(Value::as_array) {
                    buckets.extend(arr.iter().cloned());
                }
            }
            let currency = buckets
                .first()
                .and_then(|b| b.get("currency"))
                .and_then(Value::as_str)
                .unwrap_or("eur")
                .to_string();
            let amount: i64 = buckets
                .iter()
                .filter_map(|b| b.get("amount").and_then(Value::as_i64))
                .sum();
            (amount, currency)
        }
        Ok(r) => {
            let code = r.status().as_u16();
            emit(&app, IntegrationUpdate {
                id: "integration_stripe",
                data: json!({}),
                error: Some(status_error(code, crate::i18n::t("Use a secret key (sk_live_… not pk_live_…)", "Use uma chave secreta (sk_live_…, não pk_live_…)"))),
                event: None,
            });
            return;
        }
        Err(e) => {
            emit(&app, IntegrationUpdate {
                id: "integration_stripe",
                data: json!({}),
                error: Some(if crate::i18n::pt() { format!("Sem conexão: {e}") } else { format!("No connection: {e}") }),
                event: None,
            });
            return;
        }
    };

    let charges = http
        .get("https://api.stripe.com/v1/charges?limit=3")
        .header("Authorization", &auth)
        .send()
        .await;
    let Ok(response) = charges else { return };
    if !response.status().is_success() {
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let payments: Vec<Value> = json
        .get("data")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|c| {
                    let description = c
                        .get("description")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            c.get("billing_details")
                                .and_then(|b| b.get("name"))
                                .and_then(Value::as_str)
                        })
                        .map(str::to_string);
                    Some(json!({
                        "id": c.get("id")?.as_str()?,
                        "amount": c.get("amount")?.as_i64()?,
                        "currency": c.get("currency")?.as_str()?,
                        "description": description,
                        "createdAt": c.get("created").and_then(Value::as_i64).unwrap_or(0) * 1000,
                        "status": c.get("status").and_then(Value::as_str).unwrap_or("succeeded"),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    let newest = payments
        .first()
        .and_then(|p| p.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let event = if !newest.is_empty() && is_new("stripe", &newest) {
        let label = payments[0]
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                let cents = payments[0].get("amount").and_then(Value::as_i64).unwrap_or(0);
                format!("{:.2}", cents as f64 / 100.0)
            });
        Some(IntegrationEvent { success: true, label, detail: None })
    } else {
        None
    };

    emit(&app, IntegrationUpdate {
        id: "integration_stripe",
        data: json!({ "balance": amount, "currency": currency, "payments": payments }),
        error: None,
        event,
    });
}

// ── GitHub ────────────────────────────────────────────────────────────────────

async fn poll_github(app: AppHandle) {
    let Some(token) = secrets::get("github-token") else { return };
    let http = client();

    let user = http
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Coucou")
        .send()
        .await;
    let Ok(response) = user else { return };
    if !response.status().is_success() {
        emit(&app, IntegrationUpdate {
            id: "integration_github",
            data: json!({}),
            error: Some(status_error(response.status().as_u16(), crate::i18n::t("Token lacks the needed scope", "O token não tem o escopo necessário"))),
            event: None,
        });
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let public = json.get("public_repos").and_then(Value::as_i64).unwrap_or(0);
    let private = json
        .get("owned_private_repos")
        .or_else(|| json.get("total_private_repos"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let repos = http
        .get("https://api.github.com/user/repos?per_page=100&affiliation=owner&sort=pushed")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Coucou")
        .send()
        .await;
    let stars: i64 = match repos {
        Ok(r) if r.status().is_success() => r
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v.as_array().cloned())
            .map(|list| {
                list.iter()
                    .filter_map(|r| r.get("stargazers_count").and_then(Value::as_i64))
                    .sum()
            })
            .unwrap_or(0),
        _ => 0,
    };

    emit(&app, IntegrationUpdate {
        id: "integration_github",
        data: json!({ "totalRepos": public + private, "totalStars": stars }),
        error: None,
        event: None,
    });
}

// ── Vercel ────────────────────────────────────────────────────────────────────

async fn poll_vercel(app: AppHandle) {
    let Some(token) = secrets::get("vercel-token") else { return };
    let response = client()
        .get("https://api.vercel.com/v6/deployments?limit=5")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .send()
        .await;
    let Ok(response) = response else { return };
    if !response.status().is_success() {
        emit(&app, IntegrationUpdate {
            id: "integration_vercel",
            data: json!({}),
            error: Some(status_error(response.status().as_u16(), crate::i18n::t("Token lacks access", "O token não tem acesso"))),
            event: None,
        });
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let terminal = ["READY", "ERROR", "CANCELED"];
    let deployments: Vec<Value> = json
        .get("deployments")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|d| {
                    let state = d.get("state")?.as_str()?;
                    if !terminal.contains(&state) {
                        return None;
                    }
                    let meta = d.get("meta");
                    let pick = |keys: [&str; 3]| {
                        meta.and_then(|m| keys.iter().find_map(|k| m.get(*k).and_then(Value::as_str)))
                            .map(str::to_string)
                    };
                    Some(json!({
                        "id": d.get("uid")?.as_str()?,
                        "projectName": d.get("name")?.as_str()?,
                        "url": d.get("url").and_then(Value::as_str).unwrap_or(""),
                        "state": state,
                        "createdAt": d.get("createdAt").and_then(Value::as_f64).unwrap_or(0.0),
                        "commitMessage": pick(["githubCommitMessage", "gitlabCommitMessage", "bitbucketCommitMessage"]),
                        "branch": pick(["githubCommitRef", "gitlabCommitRef", "bitbucketBranch"]),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    let event = deployments.first().and_then(|latest| {
        let id = latest.get("id")?.as_str()?;
        if !is_new("vercel", id) {
            return None;
        }
        let success = latest.get("state")?.as_str()? == "READY";
        Some(IntegrationEvent {
            success,
            label: latest.get("projectName")?.as_str()?.to_string(),
            detail: None,
        })
    });

    emit(&app, IntegrationUpdate {
        id: "integration_vercel",
        data: json!({ "deployments": deployments }),
        error: None,
        event,
    });
}

// ── Discloud ──────────────────────────────────────────────────────────────────

const DISCLOUD_API: &str = "https://api.discloud.app/v2";
const DISCLOUD_ID: &str = "integration_discloud";
/// A stop or restart the user clicked makes the app drop: that drop is expected
/// and must not sound like an outage.
const DISCLOUD_QUIET: Duration = Duration::from_secs(180);
const DISCLOUD_LOG_LINES: usize = 200;
const DISCLOUD_LOG_BYTES: usize = 32 * 1024;

/// Last known state per app, so an event fires when an app changes, not on every poll.
#[derive(Default)]
struct DiscloudMemory {
    online: HashMap<String, bool>,
    quiet_until: HashMap<String, Instant>,
    primed: bool,
    skip_next: bool,
}

static DISCLOUD: LazyLock<Mutex<DiscloudMemory>> = LazyLock::new(Default::default);

struct DiscloudFailure {
    code: Option<u16>,
    message: String,
}

fn no_connection(e: impl std::fmt::Display) -> String {
    if crate::i18n::pt() { format!("Sem conexão: {e}") } else { format!("No connection: {e}") }
}

async fn discloud_send(method: reqwest::Method, path: &str) -> Result<Value, DiscloudFailure> {
    let Some(token) = secrets::get("discloud-token") else {
        return Err(DiscloudFailure {
            code: None,
            message: crate::i18n::t("Token not configured", "Token não configurado").into(),
        });
    };
    let response = client()
        .request(method, format!("{DISCLOUD_API}{path}"))
        .header("api-token", token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| DiscloudFailure { code: None, message: no_connection(e) })?;
    let code = response.status().as_u16();
    let json: Value = response.json().await.unwrap_or(json!({}));
    let api_message = json.get("message").and_then(Value::as_str).map(str::to_string);
    let failed_in_body = json.get("status").and_then(Value::as_str) == Some("error");
    if (200..300).contains(&code) && !failed_in_body {
        return Ok(json);
    }
    let message = match code {
        401 => status_error(401, ""),
        429 => crate::i18n::t("API rate limit, trying again shortly", "Limite da API, tentando de novo em breve").into(),
        _ => api_message.unwrap_or_else(|| {
            status_error(code, crate::i18n::t("Token lacks access", "O token não tem acesso"))
        }),
    };
    Err(DiscloudFailure { code: Some(code), message })
}

/// `apps` is a list for `/app/all` and a single object for `/app/{id}`.
fn as_list(v: Option<&Value>) -> Vec<&Value> {
    match v {
        Some(Value::Array(list)) => list.iter().collect(),
        Some(obj @ Value::Object(_)) => vec![obj],
        _ => Vec::new(),
    }
}

/// Joins `/app/all` (names, online) with `/app/all/status` (CPU, RAM), offline first.
fn discloud_apps(info: &Value, status: Option<&Value>) -> Vec<Value> {
    let metrics: HashMap<&str, &Value> = as_list(status.and_then(|s| s.get("apps")))
        .into_iter()
        .filter_map(|m| Some((m.get("id")?.as_str()?, m)))
        .collect();
    let mut apps: Vec<Value> = as_list(info.get("apps"))
        .into_iter()
        .filter_map(|a| {
            let id = a.get("id")?.as_str()?;
            let m = metrics.get(id);
            let field = |k: &str| m.and_then(|m| m.get(k)).and_then(Value::as_str).map(str::to_string);
            let online = a
                .get("online")
                .and_then(Value::as_bool)
                .unwrap_or_else(|| field("container").as_deref() == Some("Online"));
            Some(json!({
                "id": id,
                "name": a.get("name").and_then(Value::as_str).unwrap_or(id),
                "online": online,
                "ramKilled": a.get("ramKilled").and_then(Value::as_bool).unwrap_or(false),
                "exitCode": a.get("exitCode").and_then(Value::as_i64),
                "ramLimit": a.get("ram").and_then(Value::as_i64),
                "cpu": field("cpu"),
                "memory": field("memory"),
                "startedAt": field("startedAt"),
            }))
        })
        .collect();
    let key = |a: &Value| {
        let online = a.get("online").and_then(Value::as_bool).unwrap_or(false);
        let name = a.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase();
        (online, name)
    };
    apps.sort_by_key(key);
    apps
}

fn discloud_down_detail(app: &Value) -> String {
    if app.get("ramKilled").and_then(Value::as_bool).unwrap_or(false) {
        return crate::i18n::t("Out of memory", "Sem memória").into();
    }
    match app.get("exitCode").and_then(Value::as_i64) {
        Some(code) if code != 0 => {
            if crate::i18n::pt() { format!("Saiu com código {code}") } else { format!("Exited with code {code}") }
        }
        _ => crate::i18n::t("Went offline", "Ficou offline").into(),
    }
}

/// Compares this poll with the last one. The first poll only fills the card; apps
/// the user just stopped or restarted stay quiet. Several changes in one poll
/// become one event, an outage winning over a recovery.
fn discloud_event(mem: &mut DiscloudMemory, apps: &[Value], now: Instant) -> Option<IntegrationEvent> {
    let mut downs: Vec<(String, String)> = Vec::new();
    let mut ups: Vec<String> = Vec::new();
    let mut seen = HashMap::new();
    for app in apps {
        let Some(id) = app.get("id").and_then(Value::as_str) else { continue };
        let name = app.get("name").and_then(Value::as_str).unwrap_or(id).to_string();
        let online = app.get("online").and_then(Value::as_bool).unwrap_or(false);
        seen.insert(id.to_string(), online);
        let quiet = mem.quiet_until.get(id).is_some_and(|until| *until > now);
        if !mem.primed || quiet {
            continue;
        }
        match mem.online.get(id) {
            Some(true) if !online => downs.push((name, discloud_down_detail(app))),
            Some(false) if online => ups.push(name),
            _ => {}
        }
    }
    // Apps deleted in the dashboard simply disappear.
    mem.online = seen;
    mem.quiet_until.retain(|_, until| *until > now);
    if !mem.primed {
        mem.primed = true;
        return None;
    }

    let (success, label, detail, count) = if let Some((name, detail)) = downs.first() {
        (false, name.clone(), detail.clone(), downs.len())
    } else {
        let name = ups.first()?;
        (true, name.clone(), crate::i18n::t("Back online", "De volta ao ar").to_string(), ups.len())
    };
    let detail = if count > 1 {
        let others = count - 1;
        if crate::i18n::pt() { format!("{detail} · +{others} outros") } else { format!("{detail} · +{others} more") }
    } else {
        detail
    };
    Some(IntegrationEvent { success, label, detail: Some(detail) })
}

async fn poll_discloud(app: AppHandle) {
    if secrets::get("discloud-token").is_none() {
        return;
    }
    {
        let mut mem = DISCLOUD.lock().unwrap();
        if mem.skip_next {
            mem.skip_next = false;
            return;
        }
    }
    let info = match discloud_send(reqwest::Method::GET, "/app/all").await {
        Ok(json) => json,
        Err(failure) => {
            if failure.code == Some(429) {
                DISCLOUD.lock().unwrap().skip_next = true;
            }
            emit(&app, IntegrationUpdate {
                id: DISCLOUD_ID,
                data: json!({}),
                error: Some(failure.message),
                event: None,
            });
            return;
        }
    };
    // Metrics are a bonus: without them the card still shows which apps are up.
    let status = discloud_send(reqwest::Method::GET, "/app/all/status").await.ok();
    let apps = discloud_apps(&info, status.as_ref());
    let event = discloud_event(&mut DISCLOUD.lock().unwrap(), &apps, Instant::now());
    emit(&app, IntegrationUpdate {
        id: DISCLOUD_ID,
        data: json!({ "apps": apps }),
        error: None,
        event,
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscloudActionResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscloudLogs {
    pub text: String,
    pub url: Option<String>,
}

/// Discloud app ids are short alphanumeric strings. Anything else — and `all`,
/// which would hit every app at once — is refused before it reaches a URL.
fn valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "all"
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn discloud_allowed(app: &AppHandle) -> Result<(), String> {
    if PAUSED.load(Ordering::Relaxed) {
        return Err(crate::i18n::t("Coucou is paused", "O Coucou está pausado").into());
    }
    if !enabled(app, DISCLOUD_ID) {
        return Err(crate::i18n::t("Discloud is off in Settings", "A Discloud está desligada nos Ajustes").into());
    }
    Ok(())
}

/// Start / stop / restart from the island's buttons — never called on its own.
pub async fn discloud_action(app: AppHandle, app_id: String, action: String) -> DiscloudActionResult {
    let refuse = |message: String| DiscloudActionResult { ok: false, message };
    let action: &'static str = match action.as_str() {
        "start" => "start",
        "stop" => "stop",
        "restart" => "restart",
        _ => return refuse(crate::i18n::t("Unknown action", "Ação desconhecida").into()),
    };
    if !valid_app_id(&app_id) {
        return refuse(crate::i18n::t("Invalid app", "App inválido").into());
    }
    if let Err(message) = discloud_allowed(&app) {
        return refuse(message);
    }

    DISCLOUD
        .lock()
        .unwrap()
        .quiet_until
        .insert(app_id.clone(), Instant::now() + DISCLOUD_QUIET);
    let result = match discloud_send(reqwest::Method::PUT, &format!("/app/{app_id}/{action}")).await {
        Ok(json) => DiscloudActionResult {
            ok: true,
            message: json
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| crate::i18n::t("Done", "Feito").into()),
        },
        Err(failure) => {
            // Nothing changed on the server, so a real outage must still be heard.
            DISCLOUD.lock().unwrap().quiet_until.remove(&app_id);
            refuse(failure.message)
        }
    };
    log::line(format!("discloud {action} {app_id} → {}", if result.ok { "ok" } else { "failed" }));

    // Once now, and again when the container has had time to move.
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        poll_discloud(handle.clone()).await;
        tokio::time::sleep(Duration::from_secs(8)).await;
        poll_discloud(handle).await;
    });
    result
}

/// The last lines of an app's terminal. The text may hold the app's own secrets,
/// so it goes to the island and nowhere else — never to the log.
pub async fn discloud_logs(app: AppHandle, app_id: String) -> Result<DiscloudLogs, String> {
    if !valid_app_id(&app_id) {
        return Err(crate::i18n::t("Invalid app", "App inválido").into());
    }
    discloud_allowed(&app)?;
    let json = discloud_send(reqwest::Method::GET, &format!("/app/{app_id}/logs"))
        .await
        .map_err(|failure| failure.message)?;
    let terminal = as_list(json.get("apps")).into_iter().next().and_then(|a| a.get("terminal"));
    let pick = |k: &str| terminal.and_then(|t| t.get(k)).and_then(Value::as_str).filter(|s| !s.is_empty());
    let text = pick("small").or_else(|| pick("big")).unwrap_or("");
    Ok(DiscloudLogs {
        text: tail(text, DISCLOUD_LOG_LINES, DISCLOUD_LOG_BYTES),
        url: pick("url").map(str::to_string),
    })
}

/// The last `max_lines` lines, then the last `max_bytes` bytes on a char boundary.
fn tail(text: &str, max_lines: usize, max_bytes: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let out = lines[lines.len().saturating_sub(max_lines)..].join("\n");
    if out.len() <= max_bytes {
        return out;
    }
    let mut cut = out.len() - max_bytes;
    while !out.is_char_boundary(cut) {
        cut += 1;
    }
    out[cut..].to_string()
}

// ── Resend ────────────────────────────────────────────────────────────────────

async fn poll_resend(app: AppHandle) {
    let Some(key) = secrets::get("resend-api-key") else { return };
    let response = client()
        .get("https://api.resend.com/emails?limit=100")
        .header("Authorization", format!("Bearer {key}"))
        .header("Accept", "application/json")
        .send()
        .await;
    let Ok(response) = response else { return };
    if !response.status().is_success() {
        emit(&app, IntegrationUpdate {
            id: "integration_resend",
            data: json!({}),
            error: Some(status_error(response.status().as_u16(), crate::i18n::t("Key lacks access", "A chave não tem acesso"))),
            event: None,
        });
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let total = json
        .get("total")
        .or_else(|| json.get("count"))
        .and_then(Value::as_i64);
    let emails: Vec<Value> = json
        .get("data")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .take(5)
                .filter_map(|e| {
                    let to = match e.get("to") {
                        Some(Value::Array(a)) => a.clone(),
                        Some(Value::String(s)) => vec![Value::String(s.clone())],
                        _ => vec![],
                    };
                    Some(json!({
                        "id": e.get("id")?.as_str()?,
                        "to": to,
                        "subject": e.get("subject").and_then(Value::as_str).unwrap_or(""),
                        "createdAt": e.get("created_at").and_then(Value::as_str).unwrap_or(""),
                        "lastEvent": e.get("last_event").and_then(Value::as_str).unwrap_or(""),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    emit(&app, IntegrationUpdate {
        id: "integration_resend",
        data: json!({ "emails": emails, "total": total }),
        error: None,
        event: None,
    });
}

// ── Notion ────────────────────────────────────────────────────────────────────

async fn poll_notion(app: AppHandle) {
    let Some(token) = secrets::get("notion-api-key") else { return };
    let response = client()
        .post("https://api.notion.com/v1/search")
        .header("Authorization", format!("Bearer {token}"))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .json(&json!({
            "sort": { "direction": "descending", "timestamp": "last_edited_time" },
            "page_size": 3
        }))
        .send()
        .await;
    let Ok(response) = response else { return };
    if !response.status().is_success() {
        emit(&app, IntegrationUpdate {
            id: "integration_notion",
            data: json!({}),
            error: Some(status_error(response.status().as_u16(), crate::i18n::t("Integration lacks access", "A integração não tem acesso"))),
            event: None,
        });
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let pages: Vec<Value> = json
        .get("results")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(parse_notion_page).collect())
        .unwrap_or_default();

    emit(&app, IntegrationUpdate {
        id: "integration_notion",
        data: json!({ "pages": pages }),
        error: None,
        event: None,
    });
}

fn parse_notion_page(obj: &Value) -> Option<Value> {
    let id = obj.get("id")?.as_str()?;
    let is_database = obj.get("object").and_then(Value::as_str) == Some("database");

    let mut title = "Untitled".to_string();
    if is_database {
        if let Some(text) = obj
            .get("title")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|t| t.get("plain_text"))
            .and_then(Value::as_str)
        {
            if !text.is_empty() {
                title = text.to_string();
            }
        }
    } else if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        for prop in props.values() {
            if prop.get("type").and_then(Value::as_str) != Some("title") {
                continue;
            }
            if let Some(text) = prop
                .get("title")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(|t| t.get("plain_text"))
                .and_then(Value::as_str)
            {
                if !text.is_empty() {
                    title = text.to_string();
                    break;
                }
            }
        }
    }

    let emoji = obj
        .get("icon")
        .filter(|i| i.get("type").and_then(Value::as_str) == Some("emoji"))
        .and_then(|i| i.get("emoji"))
        .and_then(Value::as_str);

    Some(json!({
        "id": id,
        "title": title,
        "emoji": emoji,
        "lastEditedAt": obj.get("last_edited_time").and_then(Value::as_str)?,
        "url": obj.get("url").and_then(Value::as_str).unwrap_or("https://notion.so"),
    }))
}

// ── Cal.com ───────────────────────────────────────────────────────────────────

async fn poll_calcom(app: AppHandle) {
    let Some(key) = secrets::get("calcom-api-key") else { return };
    let response = client()
        .get("https://api.cal.com/v2/bookings?status=upcoming")
        .header("Authorization", format!("Bearer {key}"))
        .header("cal-api-version", "2024-08-13")
        .send()
        .await;
    let Ok(response) = response else { return };
    if !response.status().is_success() {
        emit(&app, IntegrationUpdate {
            id: "integration_calcom",
            data: json!({}),
            error: Some(status_error(response.status().as_u16(), crate::i18n::t("Key lacks access", "A chave não tem acesso"))),
            event: None,
        });
        return;
    }
    let json: Value = response.json().await.unwrap_or(json!({}));
    let bookings: Vec<Value> = json
        .get("data")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|b| {
                    let start = b
                        .get("start")
                        .or_else(|| b.get("startTime"))
                        .and_then(Value::as_str)?;
                    let attendee = b.get("attendees").and_then(Value::as_array).and_then(|a| a.first());
                    let notes = b
                        .get("responses")
                        .and_then(|r| r.get("notes"))
                        .and_then(|n| n.get("value"))
                        .and_then(Value::as_str)
                        .or_else(|| b.get("description").and_then(Value::as_str))
                        .filter(|s| !s.is_empty());
                    Some(json!({
                        "id": b.get("id").map(|v| v.to_string()).unwrap_or_default(),
                        "title": b.get("title").and_then(Value::as_str).unwrap_or("Meeting"),
                        "start": start,
                        "status": b.get("status").and_then(Value::as_str).unwrap_or("accepted"),
                        "attendeeName": attendee.and_then(|a| a.get("name")).and_then(Value::as_str),
                        "attendeeEmail": attendee.and_then(|a| a.get("email")).and_then(Value::as_str),
                        "attendeeNotes": notes,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    emit(&app, IntegrationUpdate {
        id: "integration_calcom",
        data: json!({ "bookings": bookings }),
        error: None,
        event: None,
    });
}

// ── n8n ───────────────────────────────────────────────────────────────────────

async fn poll_n8n(app: AppHandle) {
    let (Some(key), Some(raw_base)) = (secrets::get("n8n-api-key"), secrets::get("n8n-url")) else {
        return;
    };
    let base = raw_base.trim_end_matches('/').to_string();
    let http = client();

    // Same two shapes as the Swift poller: the public API first, then /rest.
    let list_urls = [
        format!("{base}/api/v1/executions?limit=1&includeData=false"),
        format!("{base}/rest/executions?limit=1&includeData=false"),
    ];

    let mut items: Option<Vec<Value>> = None;
    for url in &list_urls {
        let Ok(response) = http.get(url).header("X-N8N-API-KEY", &key).header("Accept", "application/json").send().await
        else {
            continue;
        };
        if !response.status().is_success() {
            // Only the status: a self-hosted base URL can carry credentials.
            log::line(format!("n8n list HTTP {}", response.status()));
            continue;
        }
        let Ok(json) = response.json::<Value>().await else { continue };
        items = match &json {
            Value::Object(o) => o.get("data").and_then(Value::as_array).cloned(),
            Value::Array(a) => Some(a.clone()),
            _ => None,
        };
        if items.is_some() {
            break;
        }
    }

    let Some(first) = items.and_then(|list| list.into_iter().next()) else { return };
    let id = match first.get("id") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return,
    };

    let status = first.get("status").and_then(Value::as_str).unwrap_or("");
    if !["success", "error", "crashed", "canceled", "failed"].contains(&status) {
        return;
    }
    if !is_new("n8n", &id) {
        return;
    }
    let success = status == "success";

    let detail_urls = [
        format!("{base}/api/v1/executions/{id}?includeData=true"),
        format!("{base}/api/v1/executions/{id}"),
        format!("{base}/rest/executions/{id}?includeData=true"),
        format!("{base}/rest/executions/{id}"),
    ];
    let mut name = "Workflow".to_string();
    let mut detail = None;
    for url in &detail_urls {
        let Ok(response) = http.get(url).header("X-N8N-API-KEY", &key).header("Accept", "application/json").send().await
        else {
            continue;
        };
        if !response.status().is_success() {
            continue;
        }
        let Ok(json) = response.json::<Value>().await else { continue };
        name = json
            .get("workflowData")
            .and_then(|w| w.get("name"))
            .and_then(Value::as_str)
            .or_else(|| json.get("name").and_then(Value::as_str))
            .unwrap_or("Workflow")
            .to_string();
        detail = n8n_detail(&json, success);
        break;
    }

    log::line(format!("n8n execution {id} {status} · {name}"));
    emit(&app, IntegrationUpdate {
        id: "integration_n8n",
        data: json!({ "workflow": name, "status": status }),
        error: None,
        event: Some(IntegrationEvent { success, label: name, detail }),
    });
}

fn n8n_detail(json: &Value, success: bool) -> Option<String> {
    let result = json.get("data")?.get("resultData")?;
    if !success {
        if let Some(error) = result.get("error") {
            let message = error.get("message").and_then(Value::as_str).unwrap_or("");
            if let Some(node) = error.get("node").and_then(|n| n.get("name")).and_then(Value::as_str) {
                if !node.is_empty() {
                    return Some(format!("{node}\n{message}"));
                }
            }
            return Some(message.to_string());
        }
        let runs = result.get("runData")?.as_object()?;
        for (node, value) in runs {
            if let Some(message) = value
                .as_array()
                .and_then(|a| a.first())
                .and_then(|r| r.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
            {
                return Some(format!("{node}\n{message}"));
            }
        }
        return None;
    }

    let last_node = result.get("lastNodeExecuted")?.as_str()?;
    let items = result
        .get("runData")?
        .get(last_node)?
        .as_array()?
        .first()?
        .get("data")?
        .get("main")?
        .as_array()?
        .first()?
        .as_array()?;
    let count = items.len();
    let header = format!("→ {last_node} · {count} item{}", if count == 1 { "" } else { "s" });

    let fields = items
        .first()
        .and_then(|i| i.get("json"))
        .and_then(Value::as_object)
        .map(|obj| {
            obj.iter()
                .take(4)
                .map(|(k, v)| format!("{k}: {}", fmt_value(v)))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|s| !s.is_empty());

    Some(match fields {
        Some(f) => format!("{header}\n{f}"),
        None => header,
    })
}

fn fmt_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.chars().take(50).collect(),
        Value::Array(a) => format!("[{}]", a.len()),
        Value::Object(_) => "{…}".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, online: bool) -> Value {
        json!({ "id": id, "name": id, "online": online, "ramKilled": false, "exitCode": 0 })
    }

    #[test]
    fn discloud_apps_accepts_list_and_object_and_sorts_offline_first() {
        let info = json!({ "apps": [
            { "id": "b", "name": "Beta", "online": true },
            { "id": "a", "name": "alpha", "online": false },
        ] });
        let status = json!({ "apps": { "id": "b", "cpu": "12%", "memory": "180/512MB" } });
        let apps = discloud_apps(&info, Some(&status));
        assert_eq!(apps[0]["id"], "a");
        assert_eq!(apps[1]["cpu"], "12%");
        assert!(apps[0]["cpu"].is_null());
    }

    #[test]
    fn discloud_online_falls_back_to_container() {
        let info = json!({ "apps": [{ "id": "a" }] });
        let status = json!({ "apps": [{ "id": "a", "container": "Online" }] });
        assert_eq!(discloud_apps(&info, Some(&status))[0]["online"], true);
    }

    #[test]
    fn first_poll_is_silent_then_changes_fire_once() {
        let mut mem = DiscloudMemory::default();
        let now = Instant::now();
        assert!(discloud_event(&mut mem, &[app("a", true)], now).is_none());
        let down = discloud_event(&mut mem, &[app("a", false)], now).unwrap();
        assert!(!down.success);
        assert_eq!(down.detail.as_deref(), Some("Went offline"));
        assert!(discloud_event(&mut mem, &[app("a", false)], now).is_none());
        assert!(discloud_event(&mut mem, &[app("a", true)], now).unwrap().success);
    }

    #[test]
    fn outage_wins_and_counts_the_others() {
        let mut mem = DiscloudMemory::default();
        let now = Instant::now();
        discloud_event(&mut mem, &[app("a", true), app("b", true), app("c", false)], now);
        let e = discloud_event(&mut mem, &[app("a", false), app("b", false), app("c", true)], now).unwrap();
        assert!(!e.success);
        assert_eq!(e.detail.as_deref(), Some("Went offline · +1 more"));
    }

    #[test]
    fn user_actions_stay_quiet() {
        let mut mem = DiscloudMemory::default();
        let now = Instant::now();
        discloud_event(&mut mem, &[app("a", true)], now);
        mem.quiet_until.insert("a".into(), now + DISCLOUD_QUIET);
        assert!(discloud_event(&mut mem, &[app("a", false)], now).is_none());
        // Still offline after the quiet window: no late alarm either.
        assert!(discloud_event(&mut mem, &[app("a", false)], now + DISCLOUD_QUIET * 2).is_none());
    }

    #[test]
    fn app_ids_are_checked() {
        assert!(valid_app_id("1712345678901"));
        assert!(valid_app_id("my-bot_2"));
        assert!(!valid_app_id("all"));
        assert!(!valid_app_id(""));
        assert!(!valid_app_id("a/../../user"));
        assert!(!valid_app_id("a?x=1"));
    }

    #[test]
    fn tail_keeps_the_end() {
        let text = (1..=300).map(|n| n.to_string()).collect::<Vec<_>>().join("\n");
        let out = tail(&text, 200, 1 << 20);
        assert!(out.starts_with("101\n") && out.ends_with("300"));
        assert_eq!(tail("ééé", 10, 3), "é");
    }
}
