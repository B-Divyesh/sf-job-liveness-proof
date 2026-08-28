use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    FromRow, SqlitePool,
};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration as StdDuration, Instant},
};
use subtle::ConstantTimeEq;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

type HmacSha256 = Hmac<Sha256>;
type JobSchedule = (String, i64, i64, Option<DateTime<Utc>>);

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub secret: Arc<Vec<u8>>,
    pub key_id: Arc<String>,
    pub retention_days: i64,
    pub clock_skew_seconds: i64,
    limiter: Arc<Mutex<HashMap<IpAddr, (Instant, u32)>>>,
    pub build_sha: Arc<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("signature is missing or invalid")]
    Unauthorized,
    #[error("this event was already recorded")]
    Conflict,
    #[error("receipt not found")]
    NotFound,
    #[error("API route not found")]
    RouteNotFound,
    #[error("too many ingest requests; retry shortly")]
    RateLimited,
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict => StatusCode::CONFLICT,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RouteNotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let public = if matches!(self, Self::Database(_)) {
            "could not read the proof ledger".to_string()
        } else {
            self.to_string()
        };
        (status, Json(serde_json::json!({"error": public}))).into_response()
    }
}

pub async fn connect(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let durable_network_mount = database_url.contains("/data/");
    let connections = if database_url.contains(":memory:") || durable_network_mount {
        1
    } else {
        8
    };
    // SQLite WAL relies on shared-memory primitives that network file systems do
    // not provide. The container's durable /data mount therefore uses the
    // rollback journal; local disks retain WAL's better write concurrency.
    let mut options = SqliteConnectOptions::from_str(database_url)?
        .foreign_keys(true)
        .busy_timeout(StdDuration::from_secs(10));
    if durable_network_mount {
        // Azure Files does not expose POSIX byte-range locks to Container Apps.
        // SQLite's dot-file VFS provides a network-filesystem-compatible lock.
        options = options.vfs("unix-dotfile");
    } else {
        options = options.journal_mode(SqliteJournalMode::Wal);
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(connections)
        .connect_with(options)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

pub fn router(state: AppState, static_dir: Option<&str>) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/api/v1/config", get(config))
        .route("/api/v1/jobs", post(register_job))
        .route("/api/v1/runs/start", post(start_run))
        .route("/api/v1/runs/finish", post(finish_run))
        .route("/api/v1/ci-snapshots", post(ci_snapshot))
        .route("/api/v1/ledger", get(ledger))
        .route("/api/v1/exports/ledger.csv", get(export_csv))
        .route("/api/v1/jobs/:job_key/runs/:run_id/receipt", get(receipt))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ingest_rate_limit,
        ));

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::HeaderName::from_static("x-run-proof-key"),
            header::HeaderName::from_static("x-run-proof-timestamp"),
            header::HeaderName::from_static("x-run-proof-signature"),
        ]);
    let app = Router::new()
        .merge(api)
        .route("/api", any(api_not_found))
        .route("/api/*path", any(api_not_found));
    let app = if let Some(dir) = static_dir {
        app.fallback_service(
            ServeDir::new(dir).fallback(ServeFile::new(format!("{dir}/index.html"))),
        )
    } else {
        app
    };
    app.with_state(state).layer(cors).layer(TraceLayer::new_for_http()).layer(CompressionLayer::new()).layer(middleware::from_fn(cache_headers))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::if_not_present(header::STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000")))
        .layer(SetResponseHeaderLayer::if_not_present(header::HeaderName::from_static("permissions-policy"), HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()")))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self' https://api.sociobot.in; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")))
}

async fn cache_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

async fn api_not_found() -> ApiError {
    ApiError::RouteNotFound
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"ok", "build_sha": state.build_sha.as_str()}))
}

async fn config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(
        serde_json::json!({"retention_days": state.retention_days, "clock_skew_seconds": state.clock_skew_seconds, "payload_storage": false}),
    )
}

async fn ingest_rate_limit(
    State(state): State<AppState>,
    connect: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    if request.method() == Method::POST {
        const WINDOW: StdDuration = StdDuration::from_secs(1);
        const MAX_SOURCES: usize = 4096;
        let source = connect
            .map(|ConnectInfo(address)| address.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        let mut map = state.limiter.lock().expect("rate limiter lock");
        map.retain(|_, (started, _)| started.elapsed() <= WINDOW);
        if !map.contains_key(&source) && map.len() >= MAX_SOURCES {
            return Err(ApiError::RateLimited);
        }
        let entry = map.entry(source).or_insert((Instant::now(), 0));
        if entry.0.elapsed() > WINDOW {
            *entry = (Instant::now(), 0);
        }
        entry.1 += 1;
        if entry.1 > 100 {
            return Err(ApiError::RateLimited);
        }
    }
    Ok(next.run(request).await)
}

fn validate_id(value: &str, label: &str) -> Result<(), ApiError> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.:/".contains(c))
    {
        return Err(ApiError::BadRequest(format!(
            "{label} must be 1–100 URL-safe characters"
        )));
    }
    Ok(())
}

fn parse_time(value: &str, label: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| ApiError::BadRequest(format!("{label} must be an RFC 3339 timestamp")))
}

struct SignedRequest {
    key_id: String,
    timestamp: String,
    body: String,
    signature: String,
}

fn verify(headers: &HeaderMap, body: &[u8], state: &AppState) -> Result<SignedRequest, ApiError> {
    let key = headers
        .get("x-run-proof-key")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    if key != state.key_id.as_str() {
        return Err(ApiError::Unauthorized);
    }
    let timestamp = headers
        .get("x-run-proof-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let unix: i64 = timestamp.parse().map_err(|_| ApiError::Unauthorized)?;
    if (Utc::now().timestamp() - unix).abs() > state.clock_skew_seconds {
        return Err(ApiError::BadRequest(format!(
            "request clock differs by more than {} seconds",
            state.clock_skew_seconds
        )));
    }
    let supplied = headers
        .get("x-run-proof-signature")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("v1="))
        .and_then(|v| hex::decode(v).ok())
        .ok_or(ApiError::Unauthorized)?;
    let mut mac = HmacSha256::new_from_slice(&state.secret).map_err(|_| ApiError::Unauthorized)?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    if supplied.len() != expected.len() || supplied.ct_eq(expected.as_slice()).unwrap_u8() != 1 {
        return Err(ApiError::Unauthorized);
    }
    let body = std::str::from_utf8(body)
        .map_err(|_| ApiError::BadRequest("request body must be UTF-8 JSON".into()))?;
    Ok(SignedRequest {
        key_id: key.to_owned(),
        timestamp: timestamp.to_owned(),
        body: body.to_owned(),
        signature: format!("v1={}", hex::encode(supplied)),
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobInput {
    job_key: String,
    display_name: String,
    expected_interval_seconds: i64,
    #[serde(default = "default_grace")]
    grace_seconds: i64,
}
fn default_grace() -> i64 {
    300
}

async fn register_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let signed = verify(&headers, &body, &state)?;
    let input: JobInput = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid job: {e}")))?;
    validate_id(&input.job_key, "job_key")?;
    if input.display_name.trim().is_empty() || input.display_name.len() > 120 {
        return Err(ApiError::BadRequest(
            "display_name must be 1–120 characters".into(),
        ));
    }
    if !(60..=31_536_000).contains(&input.expected_interval_seconds)
        || !(0..=86_400).contains(&input.grace_seconds)
    {
        return Err(ApiError::BadRequest(
            "interval or grace is outside its allowed range".into(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO jobs(job_key,display_name,expected_interval_seconds,grace_seconds,created_at,updated_at,signed_key_id,signed_timestamp,signed_body,signature) VALUES(?,?,?,?,?,?,?,?,?,?) ON CONFLICT(job_key) DO UPDATE SET display_name=excluded.display_name,expected_interval_seconds=excluded.expected_interval_seconds,grace_seconds=excluded.grace_seconds,updated_at=excluded.updated_at,signed_key_id=excluded.signed_key_id,signed_timestamp=excluded.signed_timestamp,signed_body=excluded.signed_body,signature=excluded.signature")
        .bind(&input.job_key).bind(input.display_name.trim()).bind(input.expected_interval_seconds).bind(input.grace_seconds).bind(&now).bind(&now).bind(&signed.key_id).bind(&signed.timestamp).bind(&signed.body).bind(&signed.signature).execute(&state.pool).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"recorded": true, "job_key": input.job_key})),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartInput {
    job_key: String,
    run_id: String,
    scheduled_at: String,
    #[serde(default)]
    started_at: Option<String>,
}

async fn start_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let signed = verify(&headers, &body, &state)?;
    let input: StartInput = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid start event: {e}")))?;
    validate_id(&input.job_key, "job_key")?;
    validate_id(&input.run_id, "run_id")?;
    let scheduled = parse_time(&input.scheduled_at, "scheduled_at")?;
    let occurred = input
        .started_at
        .as_deref()
        .map(|v| parse_time(v, "started_at"))
        .transpose()?
        .unwrap_or_else(Utc::now);
    ensure_job(&state.pool, &input.job_key).await?;
    let result = sqlx::query("INSERT INTO events(job_key,run_id,event_type,scheduled_at,occurred_at,received_at,signature,signed_key_id,signed_timestamp,signed_body) VALUES(?,?,'start',?,?,?,?,?,?,?)")
        .bind(&input.job_key).bind(&input.run_id).bind(scheduled.to_rfc3339()).bind(occurred.to_rfc3339()).bind(Utc::now().to_rfc3339()).bind(&signed.signature).bind(&signed.key_id).bind(&signed.timestamp).bind(&signed.body).execute(&state.pool).await;
    map_insert(result)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"recorded":true,"run_id":input.run_id,"event":"start"})),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinishInput {
    job_key: String,
    run_id: String,
    status: String,
    #[serde(default)]
    completion_count: Option<i64>,
    #[serde(default)]
    finished_at: Option<String>,
}

async fn finish_run(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let signed = verify(&headers, &body, &state)?;
    let input: FinishInput = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid finish event: {e}")))?;
    validate_id(&input.job_key, "job_key")?;
    validate_id(&input.run_id, "run_id")?;
    if !["success", "failed", "cancelled"].contains(&input.status.as_str()) {
        return Err(ApiError::BadRequest(
            "status must be success, failed, or cancelled".into(),
        ));
    }
    if input.completion_count.is_some_and(|v| v < 0) {
        return Err(ApiError::BadRequest(
            "completion_count cannot be negative".into(),
        ));
    }
    ensure_job(&state.pool, &input.job_key).await?;
    let occurred = input
        .finished_at
        .as_deref()
        .map(|v| parse_time(v, "finished_at"))
        .transpose()?
        .unwrap_or_else(Utc::now);
    let result = sqlx::query("INSERT INTO events(job_key,run_id,event_type,occurred_at,received_at,status,completion_count,signature,signed_key_id,signed_timestamp,signed_body) VALUES(?,?,'finish',?,?,?,?,?,?,?,?)")
        .bind(&input.job_key).bind(&input.run_id).bind(occurred.to_rfc3339()).bind(Utc::now().to_rfc3339()).bind(&input.status).bind(input.completion_count).bind(&signed.signature).bind(&signed.key_id).bind(&signed.timestamp).bind(&signed.body).execute(&state.pool).await;
    map_insert(result)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"recorded":true,"run_id":input.run_id,"event":"finish"})),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotInput {
    job_key: String,
    run_id: String,
    source: String,
    observed_status: String,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
}

async fn ci_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let signed = verify(&headers, &body, &state)?;
    let input: SnapshotInput = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid snapshot: {e}")))?;
    validate_id(&input.job_key, "job_key")?;
    validate_id(&input.run_id, "run_id")?;
    if input.source.trim().is_empty()
        || input.source.len() > 80
        || !["passed", "failed", "pending", "missing"].contains(&input.observed_status.as_str())
    {
        return Err(ApiError::BadRequest(
            "source or observed_status is invalid".into(),
        ));
    }
    if let Some(url) = &input.source_url {
        let parsed = url::Url::parse(url)
            .map_err(|_| ApiError::BadRequest("source_url must be an absolute URL".into()))?;
        if !["http", "https"].contains(&parsed.scheme()) {
            return Err(ApiError::BadRequest(
                "source_url must use http or https".into(),
            ));
        }
    }
    ensure_job(&state.pool, &input.job_key).await?;
    let observed = input
        .observed_at
        .as_deref()
        .map(|v| parse_time(v, "observed_at"))
        .transpose()?
        .unwrap_or_else(Utc::now);
    let result = sqlx::query("INSERT INTO ci_snapshots(job_key,run_id,source,observed_status,source_url,observed_at,received_at,signature,signed_key_id,signed_timestamp,signed_body) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&input.job_key).bind(&input.run_id).bind(input.source.trim()).bind(&input.observed_status).bind(&input.source_url).bind(observed.to_rfc3339()).bind(Utc::now().to_rfc3339()).bind(&signed.signature).bind(&signed.key_id).bind(&signed.timestamp).bind(&signed.body).execute(&state.pool).await;
    map_insert(result)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"recorded":true,"run_id":input.run_id,"event":"ci_snapshot"})),
    ))
}

async fn ensure_job(pool: &SqlitePool, key: &str) -> Result<(), ApiError> {
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM jobs WHERE job_key=?")
        .bind(key)
        .fetch_one(pool)
        .await?
        == 0
    {
        return Err(ApiError::BadRequest(
            "register this job before sending run events".into(),
        ));
    }
    Ok(())
}

fn map_insert(
    result: Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>,
) -> Result<(), ApiError> {
    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(ApiError::Conflict),
        Err(e) => Err(ApiError::Database(e)),
    }
}

#[derive(FromRow)]
struct RunDb {
    job_key: String,
    display_name: String,
    interval: i64,
    grace: i64,
    run_id: Option<String>,
    scheduled_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    status: Option<String>,
    completion_count: Option<i64>,
    source: Option<String>,
    observed_status: Option<String>,
    source_url: Option<String>,
    observed_at: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct LedgerRow {
    job_key: String,
    display_name: String,
    run_id: String,
    scheduled_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    completion_count: Option<i64>,
    state: String,
    source: Option<String>,
    observed_status: Option<String>,
    source_url: Option<String>,
    observed_at: Option<String>,
    receipt_hash: Option<String>,
    is_virtual: bool,
}

#[derive(Serialize)]
struct LedgerResponse {
    generated_at: String,
    rows: Vec<LedgerRow>,
    summary: HashMap<String, usize>,
}

#[derive(Deserialize, Default)]
struct LedgerQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    job: Option<String>,
}

async fn build_ledger(pool: &SqlitePool) -> Result<Vec<LedgerRow>, ApiError> {
    let records = sqlx::query_as::<_, RunDb>(r#"
      SELECT j.job_key,j.display_name,j.expected_interval_seconds interval,j.grace_seconds grace,
       r.run_id,r.scheduled_at,r.started_at,r.finished_at,r.status,r.completion_count,
       s.source,s.observed_status,s.source_url,s.observed_at
      FROM jobs j
      LEFT JOIN (
        SELECT job_key,run_id,MAX(CASE WHEN event_type='start' THEN scheduled_at END) scheduled_at,
         MAX(CASE WHEN event_type='start' THEN occurred_at END) started_at,
         MAX(CASE WHEN event_type='finish' THEN occurred_at END) finished_at,
         MAX(CASE WHEN event_type='finish' THEN status END) status,
         MAX(CASE WHEN event_type='finish' THEN completion_count END) completion_count
        FROM events GROUP BY job_key,run_id
      ) r ON r.job_key=j.job_key
      LEFT JOIN ci_snapshots s ON s.id=(SELECT id FROM ci_snapshots cs WHERE cs.job_key=j.job_key AND cs.run_id=r.run_id ORDER BY observed_at DESC LIMIT 1)
      ORDER BY COALESCE(r.scheduled_at,j.created_at) DESC
    "#).fetch_all(pool).await?;
    let now = Utc::now();
    let mut rows = Vec::new();
    let mut jobs: HashMap<String, JobSchedule> = HashMap::new();
    for r in records {
        let latest = jobs.entry(r.job_key.clone()).or_insert((
            r.display_name.clone(),
            r.interval,
            r.grace,
            None,
        ));
        if let Some(scheduled) = r.scheduled_at.as_deref().and_then(parse_db_time) {
            if latest.3.is_none_or(|v| scheduled > v) {
                latest.3 = Some(scheduled);
            }
        }
        let Some(run_id) = r.run_id else { continue };
        let scheduled = r.scheduled_at.clone();
        let deadline = scheduled
            .as_deref()
            .and_then(parse_db_time)
            .map(|v| v + Duration::seconds(r.grace));
        let mut state_name = match r.status.as_deref() {
            Some("success") => "completed",
            Some("failed") | Some("cancelled") => "failed",
            _ if deadline.is_some_and(|d| now > d) => "late",
            _ => "running",
        }
        .to_string();
        let contradictory = matches!(
            (state_name.as_str(), r.observed_status.as_deref()),
            ("completed", Some("failed" | "missing")) | ("failed" | "late", Some("passed"))
        );
        if contradictory {
            state_name = "contradictory".into();
        }
        let hash = Some(row_hash(
            &r.job_key,
            &run_id,
            scheduled.as_deref(),
            r.started_at.as_deref(),
            r.finished_at.as_deref(),
            r.status.as_deref(),
            r.completion_count,
        ));
        rows.push(LedgerRow {
            job_key: r.job_key,
            display_name: r.display_name,
            run_id,
            scheduled_at: scheduled,
            started_at: r.started_at,
            finished_at: r.finished_at,
            completion_count: r.completion_count,
            state: state_name,
            source: r.source,
            observed_status: r.observed_status,
            source_url: r.source_url,
            observed_at: r.observed_at,
            receipt_hash: hash,
            is_virtual: false,
        });
    }
    for (key, (name, interval, grace, latest)) in jobs {
        if let Some(last) = latest {
            let due = last + Duration::seconds(interval);
            if now > due + Duration::seconds(grace) {
                rows.push(LedgerRow {
                    job_key: key.clone(),
                    display_name: name,
                    run_id: format!("missing:{}", due.timestamp()),
                    scheduled_at: Some(due.to_rfc3339()),
                    started_at: None,
                    finished_at: None,
                    completion_count: None,
                    state: "missed".into(),
                    source: None,
                    observed_status: None,
                    source_url: None,
                    observed_at: None,
                    receipt_hash: Some(row_hash(
                        &key,
                        "missing",
                        Some(&due.to_rfc3339()),
                        None,
                        None,
                        None,
                        None,
                    )),
                    is_virtual: true,
                });
            }
        }
    }
    rows.sort_by(|a, b| {
        let a_time = a
            .scheduled_at
            .as_ref()
            .or(a.started_at.as_ref())
            .or(a.finished_at.as_ref());
        let b_time = b
            .scheduled_at
            .as_ref()
            .or(b.started_at.as_ref())
            .or(b.finished_at.as_ref());
        b_time.cmp(&a_time)
    });
    Ok(rows)
}

fn parse_db_time(v: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
fn row_hash(
    job: &str,
    run: &str,
    scheduled: Option<&str>,
    started: Option<&str>,
    finished: Option<&str>,
    status: Option<&str>,
    count: Option<i64>,
) -> String {
    let canonical = format!(
        "{job}|{run}|{}|{}|{}|{}|{}",
        scheduled.unwrap_or(""),
        started.unwrap_or(""),
        finished.unwrap_or(""),
        status.unwrap_or(""),
        count.map(|v| v.to_string()).unwrap_or_default()
    );
    format!(
        "sha256:{}",
        hex::encode(Sha256::digest(canonical.as_bytes()))
    )
}

async fn ledger(
    State(state): State<AppState>,
    Query(query): Query<LedgerQuery>,
) -> Result<Json<LedgerResponse>, ApiError> {
    let mut rows = build_ledger(&state.pool).await?;
    if let Some(status) = query.status {
        if status != "all" {
            rows.retain(|r| r.state == status);
        }
    }
    if let Some(job) = query.job {
        let q = job.to_lowercase();
        rows.retain(|r| {
            r.job_key.to_lowercase().contains(&q) || r.display_name.to_lowercase().contains(&q)
        });
    }
    let mut summary = HashMap::new();
    for r in &rows {
        *summary.entry(r.state.clone()).or_insert(0) += 1;
    }
    Ok(Json(LedgerResponse {
        generated_at: Utc::now().to_rfc3339(),
        rows,
        summary,
    }))
}

#[derive(Serialize, FromRow)]
struct EventReceipt {
    job_key: String,
    event_type: String,
    scheduled_at: Option<String>,
    occurred_at: String,
    received_at: String,
    status: Option<String>,
    completion_count: Option<i64>,
    signed_key_id: Option<String>,
    signed_timestamp: Option<String>,
    signed_body: Option<String>,
    signature: String,
}
#[derive(Serialize, FromRow)]
struct SnapshotReceipt {
    job_key: String,
    source: String,
    observed_status: String,
    source_url: Option<String>,
    observed_at: String,
    received_at: String,
    signed_key_id: Option<String>,
    signed_timestamp: Option<String>,
    signed_body: Option<String>,
    signature: String,
}
#[derive(Serialize, FromRow)]
struct JobReceipt {
    job_key: String,
    display_name: String,
    expected_interval_seconds: i64,
    grace_seconds: i64,
    updated_at: String,
    signed_key_id: Option<String>,
    signed_timestamp: Option<String>,
    signed_body: Option<String>,
    signature: Option<String>,
}

async fn receipt(
    State(state): State<AppState>,
    Path((job_key, run_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validate_id(&job_key, "job_key")?;
    validate_id(&run_id, "run_id")?;
    let registration=sqlx::query_as::<_,JobReceipt>("SELECT job_key,display_name,expected_interval_seconds,grace_seconds,updated_at,signed_key_id,signed_timestamp,signed_body,signature FROM jobs WHERE job_key=?").bind(&job_key).fetch_optional(&state.pool).await?;
    let events=sqlx::query_as::<_,EventReceipt>("SELECT job_key,event_type,scheduled_at,occurred_at,received_at,status,completion_count,signed_key_id,signed_timestamp,signed_body,signature FROM events WHERE job_key=? AND run_id=? ORDER BY id").bind(&job_key).bind(&run_id).fetch_all(&state.pool).await?;
    let snapshots=sqlx::query_as::<_,SnapshotReceipt>("SELECT job_key,source,observed_status,source_url,observed_at,received_at,signed_key_id,signed_timestamp,signed_body,signature FROM ci_snapshots WHERE job_key=? AND run_id=? ORDER BY id").bind(&job_key).bind(&run_id).fetch_all(&state.pool).await?;
    if events.is_empty() && snapshots.is_empty() {
        if let Some(row) = build_ledger(&state.pool)
            .await?
            .into_iter()
            .find(|row| row.is_virtual && row.job_key == job_key && row.run_id == run_id)
        {
            let derivation_basis = sqlx::query_as::<_, EventReceipt>(
                "SELECT job_key,event_type,scheduled_at,occurred_at,received_at,status,completion_count,signed_key_id,signed_timestamp,signed_body,signature FROM events WHERE job_key=? AND event_type='start' ORDER BY scheduled_at DESC LIMIT 1",
            )
            .bind(&job_key)
            .fetch_optional(&state.pool)
            .await?;
            return Ok(Json(serde_json::json!({
                "format":"run-proof-receipt/v2", "job_key":job_key, "run_id":run_id, "exported_at":Utc::now().to_rfc3339(),
                "derived_alert": {"job_key":row.job_key,"display_name":row.display_name,"scheduled_at":row.scheduled_at,"state":"missed","reason":"No signed start was received by the configured interval and grace deadline."},
                "registration":registration, "derivation_basis":derivation_basis, "events":[], "ci_snapshots":[], "receipt_hash":row.receipt_hash,
                "receipt_hash_role":"Integrity checksum for the derived row; authenticity comes from the signed registration and derivation basis.",
                "verification":"This alert is derived from the signed registration and the job's last signed start schedule. Verify each signed_body byte-for-byte as HMAC-SHA256(signed_timestamp + '.' + signed_body)."
            })));
        }
        return Err(ApiError::NotFound);
    }
    let canonical = serde_json::to_vec(&(
        job_key.clone(),
        run_id.clone(),
        &registration,
        &events,
        &snapshots,
    ))
    .map_err(|_| ApiError::NotFound)?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&canonical)));
    Ok(Json(
        serde_json::json!({"format":"run-proof-receipt/v2","job_key":job_key,"run_id":run_id,"exported_at":Utc::now().to_rfc3339(),"registration":registration,"events":events,"ci_snapshots":snapshots,"receipt_hash":digest,"receipt_hash_role":"Integrity checksum for this export; authenticity comes from the per-record HMAC signatures.","verification":"For every record, compute HMAC-SHA256 over the UTF-8 bytes `signed_timestamp + '.' + signed_body` using the deployment secret, hex-encode it, prefix `v1=`, and compare it with signature."}),
    ))
}

async fn export_csv(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let rows = build_ledger(&state.pool).await?;
    let mut csv=String::from("job_key,display_name,run_id,scheduled_at,started_at,finished_at,state,completion_count,source,observed_status,receipt_hash\n");
    for r in rows {
        let values = [
            r.job_key,
            r.display_name,
            r.run_id,
            r.scheduled_at.unwrap_or_default(),
            r.started_at.unwrap_or_default(),
            r.finished_at.unwrap_or_default(),
            r.state,
            r.completion_count
                .map(|v| v.to_string())
                .unwrap_or_default(),
            r.source.unwrap_or_default(),
            r.observed_status.unwrap_or_default(),
            r.receipt_hash.unwrap_or_default(),
        ];
        csv.push_str(
            &values
                .iter()
                .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=run-proof-ledger.csv",
            ),
        ],
        csv,
    ))
}

pub async fn apply_retention(pool: &SqlitePool, days: i64) -> Result<(), sqlx::Error> {
    let cutoff = (Utc::now() - Duration::days(days)).to_rfc3339();
    sqlx::query("DELETE FROM ci_snapshots WHERE received_at < ?")
        .bind(&cutoff)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM events WHERE received_at < ?")
        .bind(&cutoff)
        .execute(pool)
        .await?;
    Ok(())
}

impl AppState {
    pub fn new(
        pool: SqlitePool,
        secret: String,
        key_id: String,
        retention_days: i64,
        clock_skew_seconds: i64,
        build_sha: String,
    ) -> Self {
        Self {
            pool,
            secret: Arc::new(secret.into_bytes()),
            key_id: Arc::new(key_id),
            retention_days,
            clock_skew_seconds,
            limiter: Arc::new(Mutex::new(HashMap::new())),
            build_sha: Arc::new(build_sha),
        }
    }
}
