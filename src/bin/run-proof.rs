use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::Sha256;
use std::process::ExitCode;

type HmacSha256 = Hmac<Sha256>;

#[derive(Parser)]
#[command(name="run-proof", version, about="Send signed execution evidence to a Run Proof ledger")]
struct Cli {
    #[arg(long, env="RUN_PROOF_URL", default_value="http://localhost:8080")]
    url: String,
    #[arg(long, env="RUN_PROOF_SECRET", hide_env_values=true)]
    secret: String,
    #[arg(long, env="RUN_PROOF_KEY_ID", default_value="default")]
    key_id: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Register or update a recurring job schedule.
    Register { job_key:String, #[arg(long)] name:String, #[arg(long)] every:i64, #[arg(long,default_value_t=300)] grace:i64 },
    /// Record that a scheduled run started.
    Start(RunArgs),
    /// Record a terminal run result and optional completed item count.
    Finish { job_key:String, run_id:String, #[arg(long,default_value="success")] status:String, #[arg(long)] count:Option<i64>, #[arg(long)] at:Option<String> },
    /// Attach a CI/check-provider observation to a run.
    Snapshot { job_key:String, run_id:String, #[arg(long)] source:String, #[arg(long)] status:String, #[arg(long)] source_url:Option<String>, #[arg(long)] at:Option<String> },
}

#[derive(Args)]
struct RunArgs { job_key:String, run_id:String, #[arg(long)] scheduled:String, #[arg(long)] at:Option<String> }

#[tokio::main]
async fn main() -> ExitCode {
    let cli=Cli::parse();
    if cli.secret.len()<32 { eprintln!("error: RUN_PROOF_SECRET must contain at least 32 characters"); return ExitCode::FAILURE; }
    let url=cli.url.clone(); let secret=cli.secret.clone(); let key_id=cli.key_id.clone();
    let (path,body)=match cli.command {
        Command::Register{job_key,name,every,grace}=>("/api/v1/jobs",json!({"job_key":job_key,"display_name":name,"expected_interval_seconds":every,"grace_seconds":grace})),
        Command::Start(a)=>("/api/v1/runs/start",json!({"job_key":a.job_key,"run_id":a.run_id,"scheduled_at":a.scheduled,"started_at":a.at})),
        Command::Finish{job_key,run_id,status,count,at}=>("/api/v1/runs/finish",json!({"job_key":job_key,"run_id":run_id,"status":status,"completion_count":count,"finished_at":at})),
        Command::Snapshot{job_key,run_id,source,status,source_url,at}=>("/api/v1/ci-snapshots",json!({"job_key":job_key,"run_id":run_id,"source":source,"observed_status":status,"source_url":source_url,"observed_at":at})),
    };
    match send(&url,&secret,&key_id,path,body).await { Ok(value)=>{println!("{}",serde_json::to_string_pretty(&value).unwrap());ExitCode::SUCCESS},Err(error)=>{eprintln!("error: {error}");ExitCode::FAILURE} }
}

async fn send(url:&str,secret:&str,key_id:&str,path:&str,body:Value)->Result<Value,String>{
    let raw=serde_json::to_vec(&body).map_err(|e|e.to_string())?; let timestamp=Utc::now().timestamp().to_string();
    let mut mac=HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e|e.to_string())?; mac.update(timestamp.as_bytes());mac.update(b".");mac.update(&raw);let signature=format!("v1={}",hex::encode(mac.finalize().into_bytes()));
    let response=Client::new().post(format!("{}{}",url.trim_end_matches('/'),path)).header("content-type","application/json").header("x-run-proof-key",key_id).header("x-run-proof-timestamp",timestamp).header("x-run-proof-signature",signature).body(raw).send().await.map_err(|e|format!("receiver unavailable: {e}"))?;
    let status=response.status(); let value:Value=response.json().await.map_err(|e|format!("receiver returned unreadable data: {e}"))?; if !status.is_success(){return Err(value.get("error").and_then(Value::as_str).unwrap_or("request rejected").to_string())} Ok(value)
}
