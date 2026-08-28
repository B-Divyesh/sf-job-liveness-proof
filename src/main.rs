use run_proof::{apply_retention, connect, router, AppState};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

fn default_database_url() -> String {
    if Path::new("/data").is_dir() {
        "sqlite:///data/run-proof.db?mode=rwc".into()
    } else {
        "sqlite://run-proof.db?mode=rwc".into()
    }
}

#[derive(Debug, Clone, Copy)]
enum SettingSource {
    Supplied,
    Defaulted,
    Generated,
    Persisted,
}

impl SettingSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Supplied => "supplied",
            Self::Defaulted => "defaulted",
            Self::Generated => "generated",
            Self::Persisted => "persisted",
        }
    }
}

fn secret_file_for(database_url: &str) -> PathBuf {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url)
        .split('?')
        .next()
        .unwrap_or("run-proof.db");
    let database_path = Path::new(path);
    database_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("run-proof.secret")
}

fn load_or_create_secret(path: &Path) -> io::Result<(String, SettingSource)> {
    match fs::read_to_string(path) {
        Ok(secret) if secret.trim().len() >= 32 => {
            Ok((secret.trim().to_owned(), SettingSource::Persisted))
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "persisted RUN_PROOF_SECRET is shorter than 32 characters",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut bytes = [0_u8; 32];
            getrandom::fill(&mut bytes).map_err(|error| {
                io::Error::other(format!("could not generate RUN_PROOF_SECRET: {error}"))
            })?;
            let secret = hex::encode(bytes);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
                {
                    Ok(file) => file,
                    // A rolling deployment can start two revisions against the
                    // same durable mount. The process that loses the atomic
                    // create race must reuse the winner's identity.
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                        let existing = fs::read_to_string(path)?;
                        if existing.trim().len() < 32 {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "persisted RUN_PROOF_SECRET is shorter than 32 characters",
                            ));
                        }
                        return Ok((existing.trim().to_owned(), SettingSource::Persisted));
                    }
                    Err(error) => return Err(error),
                };
                use std::io::Write;
                file.write_all(secret.as_bytes())?;
                file.write_all(b"\n")?;
            }
            #[cfg(not(unix))]
            fs::write(path, format!("{secret}\n"))?;
            Ok((secret, SettingSource::Generated))
        }
        Err(error) => Err(error),
    }
}

fn configured_secret(database_url: &str) -> Result<(String, SettingSource), String> {
    match env::var("RUN_PROOF_SECRET") {
        Ok(secret) if secret.len() >= 32 => Ok((secret, SettingSource::Supplied)),
        Ok(_) => Err("RUN_PROOF_SECRET must contain at least 32 characters".into()),
        Err(env::VarError::NotPresent) => load_or_create_secret(&secret_file_for(database_url))
            .map_err(|error| format!("could not load or generate RUN_PROOF_SECRET: {error}")),
        Err(error) => Err(format!("could not read RUN_PROOF_SECRET: {error}")),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "run_proof=info,tower_http=info".into()),
        )
        .init();
    let port = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let database_source = if env::var_os("DATABASE_URL").is_some() {
        SettingSource::Supplied
    } else {
        SettingSource::Defaulted
    };
    let database = env::var("DATABASE_URL").unwrap_or_else(|_| default_database_url());
    let (secret, secret_source) = match configured_secret(&database) {
        Ok(configured) => configured,
        Err(error) => {
            warn!(%error, "Run Proof configuration error");
            return;
        }
    };
    let key_id = env::var("RUN_PROOF_KEY_ID").unwrap_or_else(|_| "default".into());
    let retention = env::var("RETENTION_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
        .clamp(1, 3650);
    let skew = env::var("CLOCK_SKEW_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
        .clamp(30, 3600);
    info!(
        database = database_source.as_str(),
        secret = secret_source.as_str(),
        key_id = if env::var_os("RUN_PROOF_KEY_ID").is_some() {
            "supplied"
        } else {
            "defaulted"
        },
        "Run Proof runtime configuration"
    );
    let pool = match connect(&database).await {
        Ok(pool) => pool,
        Err(error) => {
            warn!(%error, "Run Proof database connection failed");
            return;
        }
    };
    if let Err(error) = apply_retention(&pool, retention).await {
        warn!(%error, "Run Proof retention cleanup failed");
        return;
    }
    let build_sha = option_env!("BUILD_SHA").unwrap_or("development").to_owned();
    let state = AppState::new(pool, secret, key_id, retention, skew, build_sha);
    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind server");
    info!(port, "Run Proof listening");
    axum::serve(
        listener,
        router(state, Some("dist")).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await
    .expect("server");
}
async fn shutdown() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ctrl-c handler") };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {_=ctrl_c=>{},_=terminate=>{}}
}
