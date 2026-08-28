use axum::{body::Body, http::{Request,StatusCode}};
use chrono::{Duration,Utc};
use hmac::{Hmac,Mac};
use http_body_util::BodyExt;
use run_proof::{connect,router,AppState};
use serde_json::{json,Value};
use sha2::Sha256;
use tower::ServiceExt;

const SECRET:&str="test-secret-that-is-at-least-thirty-two-characters";

async fn app()->axum::Router{
    let pool=connect("sqlite::memory:").await.unwrap();
    router(AppState::new(pool,SECRET.into(),"test".into(),30,300,"test-sha".into()),None)
}

fn signed(path:&str,value:Value)->Request<Body>{
    let body=serde_json::to_vec(&value).unwrap();let timestamp=Utc::now().timestamp().to_string();let mut mac=Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();mac.update(timestamp.as_bytes());mac.update(b".");mac.update(&body);
    Request::post(path).header("content-type","application/json").header("x-run-proof-key","test").header("x-run-proof-timestamp",timestamp).header("x-run-proof-signature",format!("v1={}",hex::encode(mac.finalize().into_bytes()))).body(Body::from(body)).unwrap()
}

async fn json(response:axum::response::Response)->Value{serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()}

#[tokio::test]
async fn signed_events_create_a_contradictory_receipt(){
    let app=app().await;
    let response=app.clone().oneshot(signed("/api/v1/jobs",json!({"job_key":"billing","display_name":"Billing sweep","expected_interval_seconds":3600,"grace_seconds":60}))).await.unwrap();
    assert_eq!(response.status(),StatusCode::CREATED);
    let scheduled=(Utc::now()-Duration::hours(2)).to_rfc3339();
    assert_eq!(app.clone().oneshot(signed("/api/v1/runs/start",json!({"job_key":"billing","run_id":"run-1","scheduled_at":scheduled,"started_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    assert_eq!(app.clone().oneshot(signed("/api/v1/runs/finish",json!({"job_key":"billing","run_id":"run-1","status":"success","completion_count":42,"finished_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    assert_eq!(app.clone().oneshot(signed("/api/v1/ci-snapshots",json!({"job_key":"billing","run_id":"run-1","source":"GitHub Actions","observed_status":"failed","source_url":"https://example.test/check/1","observed_at":null}))).await.unwrap().status(),StatusCode::CREATED);
    let ledger=json(app.clone().oneshot(Request::get("/api/v1/ledger").body(Body::empty()).unwrap()).await.unwrap()).await;
    let rows=ledger["rows"].as_array().unwrap();
    let concrete=rows.iter().find(|row|row["run_id"]=="run-1").unwrap();
    assert_eq!(concrete["state"],"contradictory");
    assert_eq!(concrete["completion_count"],42);
    let missing=rows.iter().find(|row|row["state"]=="missed").unwrap();
    let missing_id=missing["run_id"].as_str().unwrap();
    let derived=json(app.clone().oneshot(Request::get(format!("/api/v1/runs/{missing_id}/receipt")).body(Body::empty()).unwrap()).await.unwrap()).await;
    assert_eq!(derived["derived_alert"]["state"],"missed");
    let receipt=json(app.oneshot(Request::get("/api/v1/runs/run-1/receipt").body(Body::empty()).unwrap()).await.unwrap()).await;
    assert_eq!(receipt["format"],"run-proof-receipt/v1");assert!(receipt["receipt_hash"].as_str().unwrap().starts_with("sha256:"));
}

#[tokio::test]
async fn invalid_signature_and_unknown_payload_are_rejected(){
    let app=app().await;
    let request=Request::post("/api/v1/jobs").header("content-type","application/json").body(Body::from("{}")).unwrap();
    assert_eq!(app.clone().oneshot(request).await.unwrap().status(),StatusCode::UNAUTHORIZED);
    let response=app.oneshot(signed("/api/v1/jobs",json!({"job_key":"safe","display_name":"Safe","expected_interval_seconds":60,"grace_seconds":0,"payload":{"secret":"must not persist"}}))).await.unwrap();
    assert_eq!(response.status(),StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn health_exposes_build_sha(){
    let body=json(app().await.oneshot(Request::get("/health").body(Body::empty()).unwrap()).await.unwrap()).await;
    assert_eq!(body["build_sha"],"test-sha");
}
