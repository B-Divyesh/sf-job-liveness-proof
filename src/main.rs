use run_proof::{apply_retention, connect, router, AppState};
use std::env;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().json().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_|"run_proof=info,tower_http=info".into())).init();
    let port=env::var("PORT").ok().and_then(|v|v.parse().ok()).unwrap_or(8080);
    let database=env::var("DATABASE_URL").unwrap_or_else(|_|"sqlite://run-proof.db?mode=rwc".into());
    let secret=env::var("RUN_PROOF_SECRET").expect("RUN_PROOF_SECRET must be set to a strong random value");
    if secret.len()<32 { panic!("RUN_PROOF_SECRET must contain at least 32 characters"); }
    let key_id=env::var("RUN_PROOF_KEY_ID").unwrap_or_else(|_|"default".into());
    let retention=env::var("RETENTION_DAYS").ok().and_then(|v|v.parse().ok()).unwrap_or(30).clamp(1,3650);
    let skew=env::var("CLOCK_SKEW_SECONDS").ok().and_then(|v|v.parse().ok()).unwrap_or(300).clamp(30,3600);
    let pool=connect(&database).await.expect("database connection"); apply_retention(&pool,retention).await.expect("retention cleanup");
    let state=AppState::new(pool,secret,key_id,retention,skew,env::var("BUILD_SHA").unwrap_or_else(|_|"development".into()));
    let listener=TcpListener::bind(("0.0.0.0",port)).await.expect("bind server"); info!(port,"Run Proof listening");
    axum::serve(listener,router(state,Some("dist")).into_make_service()).with_graceful_shutdown(shutdown()).await.expect("server");
}
async fn shutdown(){ let ctrl_c=async{tokio::signal::ctrl_c().await.expect("ctrl-c handler")}; #[cfg(unix)] let terminate=async{tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler").recv().await;}; #[cfg(not(unix))] let terminate=std::future::pending::<()>(); tokio::select!{_=ctrl_c=>{},_=terminate=>{}} }
