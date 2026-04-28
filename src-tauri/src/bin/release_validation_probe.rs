use std::collections::BTreeMap;

use mathi_runtime::types::AgentRequest;
use mathi_runtime::Orchestrator;
use serde_json::Value;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    match run_probe().await {
        Ok(()) => {
            println!("Release Validation Probe Results");
            println!("result=PASS");
        }
        Err(error) => {
            eprintln!("probe_error={error}");
            println!("result=FAIL");
            std::process::exit(1);
        }
    }
}

async fn run_probe() -> Result<(), String> {
    let orchestrator = Orchestrator::new(2, 32);
    orchestrator
        .bootstrap()
        .await
        .map_err(|error| format!("bootstrap failed: {error}"))?;

    let mut read_context = BTreeMap::new();
    read_context.insert("action_class".to_string(), Value::String("read".to_string()));
    read_context.insert("agent_id".to_string(), Value::String("release-validation".to_string()));

    let read_request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "release-validation-read".to_string(),
        payload: serde_json::json!({}),
        deadline_ms: Some(2_000),
        context: read_context,
    };

    orchestrator
        .dispatch(read_request)
        .await
        .map_err(|error| format!("read dispatch failed: {error}"))?;

    let mut shell_context = BTreeMap::new();
    shell_context.insert("action_class".to_string(), Value::String("shell".to_string()));
    shell_context.insert("agent_id".to_string(), Value::String("release-validation".to_string()));
    shell_context.insert("command".to_string(), Value::String("cargo check".to_string()));

    let shell_request = AgentRequest {
        id: Uuid::new_v4(),
        task_type: "release-validation-shell".to_string(),
        payload: serde_json::json!({}),
        deadline_ms: Some(2_000),
        context: shell_context,
    };

    let shell_result = orchestrator.dispatch(shell_request).await;
    if shell_result.is_ok() {
        return Err("shell action unexpectedly allowed without approval".to_string());
    }

    Ok(())
}
