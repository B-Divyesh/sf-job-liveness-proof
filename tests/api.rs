use axum::{
    body::Body,
    extract::Query,
    http::{Request, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use run_proof::{connect, router, AppState};
use serde_json::{json, Value};
use sha2::Sha256;
use std::{
    fs,
    net::TcpListener,
    process::{Child, Command},
    time::{Duration as StdDuration, SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

const SECRET: &str = "test-secret-that-is-at-least-thirty-two-characters";

async fn app() -> axum::Router {
    let pool = connect("sqlite::memory:").await.unwrap();
    router(
        AppState::new(
            pool,
            SECRET.into(),
            "test".into(),
            30,
            300,
            "test-sha".into(),
        ),
        None,
    )
}

fn signed(path: &str, value: Value) -> Request<Body> {
    let body = serde_json::to_vec(&value).unwrap();
    let timestamp = Utc::now().timestamp().to_string();
    let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(&body);
    Request::post(path)
        .header("content-type", "application/json")
        .header("x-run-proof-key", "test")
        .header("x-run-proof-timestamp", timestamp)
        .header(
            "x-run-proof-signature",
            format!("v1={}", hex::encode(mac.finalize().into_bytes())),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn signed_events_create_a_contradictory_receipt() {
    let app = app().await;
    let response=app.clone().oneshot(signed("/api/v1/jobs",json!({"job_key":"billing","display_name":"Billing sweep","expected_interval_seconds":3600,"grace_seconds":60}))).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let scheduled = (Utc::now() - Duration::hours(2)).to_rfc3339();
    assert_eq!(app.clone().oneshot(signed("/api/v1/runs/start",json!({"job_key":"billing","run_id":"run-1","scheduled_at":scheduled,"started_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    assert_eq!(app.clone().oneshot(signed("/api/v1/runs/finish",json!({"job_key":"billing","run_id":"run-1","status":"success","completion_count":42,"finished_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    assert_eq!(app.clone().oneshot(signed("/api/v1/ci-snapshots",json!({"job_key":"billing","run_id":"run-1","source":"GitHub Actions","observed_status":"failed","source_url":"https://example.test/check/1","observed_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    let ledger = json(
        app.clone()
            .oneshot(Request::get("/api/v1/ledger").body(Body::empty()).unwrap())
            .await
            .unwrap(),
    )
    .await;
    let rows = ledger["rows"].as_array().unwrap();
    let concrete = rows.iter().find(|row| row["run_id"] == "run-1").unwrap();
    assert_eq!(concrete["state"], "contradictory");
    assert_eq!(concrete["completion_count"], 42);
    let missing = rows.iter().find(|row| row["state"] == "missed").unwrap();
    let missing_id = missing["run_id"].as_str().unwrap();
    let derived = json(
        app.clone()
            .oneshot(
                Request::get(format!("/api/v1/jobs/billing/runs/{missing_id}/receipt"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(derived["derived_alert"]["state"], "missed");
    assert_eq!(derived["derivation_basis"]["job_key"], "billing");
    assert!(derived["derivation_basis"]["signed_body"]
        .as_str()
        .unwrap()
        .contains("run-1"));
    let receipt = json(
        app.oneshot(
            Request::get("/api/v1/jobs/billing/runs/run-1/receipt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(receipt["format"], "run-proof-receipt/v2");
    assert!(receipt["receipt_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(receipt["job_key"], "billing");
    for record in receipt["events"]
        .as_array()
        .unwrap()
        .iter()
        .chain(receipt["ci_snapshots"].as_array().unwrap())
    {
        assert_eq!(record["job_key"], "billing");
        let timestamp = record["signed_timestamp"].as_str().unwrap();
        let body = record["signed_body"].as_str().unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(body.as_bytes());
        assert_eq!(
            record["signature"],
            format!("v1={}", hex::encode(mac.finalize().into_bytes()))
        );
    }
    assert!(receipt["registration"]["signed_body"]
        .as_str()
        .unwrap()
        .contains("billing"));
}

#[tokio::test]
async fn receipts_are_scoped_by_job_when_run_ids_match() {
    let app = app().await;
    for job in ["job-a", "job-b"] {
        assert_eq!(app.clone().oneshot(signed("/api/v1/jobs",json!({"job_key":job,"display_name":job,"expected_interval_seconds":3600,"grace_seconds":60}))).await.unwrap().status(),StatusCode::CREATED);
        assert_eq!(app.clone().oneshot(signed("/api/v1/runs/start",json!({"job_key":job,"run_id":"shared-run","scheduled_at":Utc::now().to_rfc3339(),"started_at":null}))).await.unwrap().status(),StatusCode::CREATED);
        assert_eq!(app.clone().oneshot(signed("/api/v1/runs/finish",json!({"job_key":job,"run_id":"shared-run","status":if job=="job-a"{"success"}else{"failed"},"completion_count":if job=="job-a"{10}else{2},"finished_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    }
    for job in ["job-a", "job-b"] {
        let receipt = json(
            app.clone()
                .oneshot(
                    Request::get(format!("/api/v1/jobs/{job}/runs/shared-run/receipt"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(receipt["job_key"], job);
        assert_eq!(receipt["events"].as_array().unwrap().len(), 2);
        assert!(receipt["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["job_key"] == job));
    }
}

#[tokio::test]
async fn finish_before_start_is_renderable_and_scoped() {
    let app = app().await;
    app.clone().oneshot(signed("/api/v1/jobs",json!({"job_key":"out-of-order","display_name":"Out of order","expected_interval_seconds":3600,"grace_seconds":60}))).await.unwrap();
    assert_eq!(app.clone().oneshot(signed("/api/v1/runs/finish",json!({"job_key":"out-of-order","run_id":"finish-first","status":"success","completion_count":0,"finished_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    let ledger = json(
        app.clone()
            .oneshot(Request::get("/api/v1/ledger").body(Body::empty()).unwrap())
            .await
            .unwrap(),
    )
    .await;
    let row = &ledger["rows"][0];
    assert_eq!(row["run_id"], "finish-first");
    assert!(row["scheduled_at"].is_null());
    assert_eq!(row["state"], "completed");
    let receipt = json(
        app.oneshot(
            Request::get("/api/v1/jobs/out-of-order/runs/finish-first/receipt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(receipt["events"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn every_non_health_route_uses_first_forwarded_ip_and_returns_retry_after() {
    let app = app().await;
    let missing = app
        .clone()
        .oneshot(
            Request::get("/api/v1/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(missing.headers()["content-type"], "application/json");
    assert_eq!(missing.headers()["cache-control"], "no-cache");
    assert!(missing.headers().contains_key("strict-transport-security"));
    assert!(missing.headers().contains_key("permissions-policy"));
    for _ in 0..40 {
        let request = Request::get("/api/v1/config")
            .header("x-forwarded-for", "198.51.100.77, 10.0.0.12")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::OK
        );
    }
    let limited = app
        .clone()
        .oneshot(
            Request::get("/api/v1/config")
                .header("x-forwarded-for", "198.51.100.77, 203.0.113.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()["retry-after"], "1");

    let different_first_hop = Request::post("/api/v1/jobs")
        .header("x-forwarded-for", "198.51.100.78, 198.51.100.77")
        .header("x-run-proof-key", "untrusted-key")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(different_first_hop)
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );

    // Health is explicitly exempt so platform probes cannot be starved.
    for _ in 0..60 {
        let request = Request::get("/health")
            .header("x-forwarded-for", "198.51.100.77")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::OK
        );
    }

    // Rotating an unauthenticated key cannot create a fresh source bucket.
    for index in 0..40 {
        let request = Request::post("/api/v1/jobs")
            .header("x-forwarded-for", "203.0.113.9")
            .header("x-run-proof-key", format!("attacker-{index}"))
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let request = Request::post("/api/v1/jobs")
        .header("x-forwarded-for", "203.0.113.9")
        .header("x-run-proof-key", "another-key")
        .body(Body::from("{}"))
        .unwrap();
    let limited = app.oneshot(request).await.unwrap();
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()["retry-after"], "1");
}

#[tokio::test]
async fn license_verification_is_same_origin_proxied_no_store_and_rate_limited() {
    async fn billing(
        Query(query): Query<std::collections::HashMap<String, String>>,
    ) -> Json<Value> {
        Json(json!({
            "valid": false,
            "reason": if query.get("license").map(String::as_str) == Some("invalid-token") {
                "invalid"
            } else {
                "wrong_product"
            },
            "expires_at": null
        }))
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let billing_server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/api/v1/products/job-liveness-proof/verify", get(billing)),
        )
        .await
        .unwrap();
    });
    let pool = connect("sqlite::memory:").await.unwrap();
    let app = router(
        AppState::new(
            pool,
            SECRET.into(),
            "test".into(),
            30,
            300,
            "test-sha".into(),
        )
        .with_billing_base(format!("http://{address}")),
        None,
    );
    let path = "/api/v1/products/job-liveness-proof/verify";
    let response = app
        .clone()
        .oneshot(
            Request::post(path)
                .header("x-forwarded-for", "192.0.2.44")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"license":"invalid-token"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(json(response).await["reason"], "invalid");

    let mut limited = None;
    for _ in 0..100 {
        let response = app
            .clone()
            .oneshot(
                Request::post(path)
                    .header("x-forwarded-for", "192.0.2.44")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let limited = limited.expect("license verification route must be rate limited");
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.headers()["retry-after"], "1");
    billing_server.abort();
}

#[tokio::test]
async fn invalid_signature_and_unknown_payload_are_rejected() {
    let app = app().await;
    let request = Request::post("/api/v1/jobs")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let response=app.oneshot(signed("/api/v1/jobs",json!({"job_key":"safe","display_name":"Safe","expected_interval_seconds":60,"grace_seconds":0,"payload":{"secret":"must not persist"}}))).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn health_exposes_build_sha() {
    let body = json(
        app()
            .await
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(body["build_sha"], "test-sha");
}

fn start_with_only_port(workdir: &std::path::Path, port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_run-proof-server"))
        .current_dir(workdir)
        .env_clear()
        .env("PORT", port.to_string())
        .spawn()
        .expect("start receiver with only PORT")
}

async fn wait_for_ready(port: u16) -> reqwest::Response {
    let url = format!("http://127.0.0.1:{port}/health");
    for _ in 0..50 {
        if let Ok(response) = reqwest::get(&url).await {
            return response;
        }
        tokio::time::sleep(StdDuration::from_millis(100)).await;
    }
    panic!("receiver did not become ready at {url}");
}

#[tokio::test]
async fn server_starts_and_serves_with_only_port() {
    let workdir = std::env::temp_dir().join(format!(
        "run-proof-port-only-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(workdir.join("dist")).unwrap();
    fs::write(
        workdir.join("dist/index.html"),
        "<!doctype html><title>Run Proof</title><main>Run Proof</main>",
    )
    .unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = start_with_only_port(&workdir, port);
    let health = wait_for_ready(port).await;
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(health.json::<Value>().await.unwrap()["status"], "ok");
    let home = reqwest::get(format!("http://127.0.0.1:{port}/"))
        .await
        .unwrap();
    assert_eq!(home.status(), StatusCode::OK);
    assert!(home.text().await.unwrap().contains("Run Proof"));
    let secret = fs::read_to_string(workdir.join("run-proof.secret")).unwrap();
    assert!(secret.trim().len() >= 32);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(workdir.join("run-proof.secret"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let mut restarted = start_with_only_port(&workdir, port);
    assert_eq!(wait_for_ready(port).await.status(), StatusCode::OK);
    restarted.kill().unwrap();
    restarted.wait().unwrap();
    fs::remove_dir_all(workdir).unwrap();
}

#[tokio::test]
async fn durable_mount_uses_a_network_filesystem_safe_vfs() {
    let directory = std::env::temp_dir().join("data").join(format!(
        "run-proof-vfs-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let database_url = format!("sqlite://{}/run-proof.db?mode=rwc", directory.display());
    let pool = connect(&database_url).await.unwrap();
    sqlx::query("INSERT INTO jobs(job_key,display_name,expected_interval_seconds,grace_seconds,created_at,updated_at) VALUES('vfs','VFS',60,0,'now','now')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert!(directory.join("run-proof.db").metadata().unwrap().len() > 0);
    fs::remove_dir_all(directory).unwrap();
}
