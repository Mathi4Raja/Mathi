use std::collections::BTreeMap;
use std::time::Instant;

use mathi_runtime::types::{AgentEvent, AgentRequest, WorkerCommand};
use mathi_runtime::worker::spawn_worker;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    match run_probe(20).await {
        Ok((p95, avg)) => {
            println!("TTFT Probe Results");
            println!("ttft_p95_ms={}", p95);
            println!("ttft_avg_ms={}", avg);

            if p95 < 100 {
                println!("result=PASS");
            } else {
                println!("result=FAIL");
                eprintln!("ttft SLO violated: p95 {}ms >= 100ms", p95);
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("probe_error={error}");
            std::process::exit(1);
        }
    }
}

async fn run_probe(iterations: usize) -> Result<(u128, u128), String> {
    let worker_id = Uuid::new_v4();
    let (command_tx, command_rx) = mpsc::channel(64);
    let (event_tx, mut event_rx) = mpsc::channel(64);
    let handle = spawn_worker(worker_id, command_rx, event_tx);
    wait_for_ready(&mut event_rx, worker_id).await?;

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let mut context = BTreeMap::new();
        context.insert("action_class".to_string(), Value::String("read".to_string()));
        context.insert("agent_id".to_string(), Value::String("ttft-probe".to_string()));

        let request = AgentRequest {
            id: Uuid::new_v4(),
            task_type: "ttft-probe-task".to_string(),
            payload: serde_json::json!({}),
            deadline_ms: Some(2_000),
            context,
        };

        let start = Instant::now();
        command_tx
            .send(WorkerCommand {
                id: Uuid::new_v4(),
                request,
            })
            .await
            .map_err(|error| error.to_string())?;

        let ttft_ms = wait_for_first_stream_chunk(&mut event_rx, worker_id, start).await?;
        samples.push(ttft_ms);
    }

    let _ = handle.cancel();

    samples.sort_unstable();
    let p95 = percentile(&samples, 95).unwrap_or(0);
    let avg = if samples.is_empty() {
        0
    } else {
        samples.iter().sum::<u128>() / samples.len() as u128
    };

    Ok((p95, avg))
}

fn percentile(values: &[u128], percentile: usize) -> Option<u128> {
    if values.is_empty() {
        return None;
    }

    let index = ((percentile * values.len()).saturating_sub(1)) / 100;
    values.get(index).copied()
}

async fn wait_for_ready(
    event_rx: &mut tokio::sync::mpsc::Receiver<AgentEvent>,
    worker_id: Uuid,
) -> Result<(), String> {
    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::Ready { worker_id: id } if id == worker_id => return Ok(()),
            _ => {}
        }
    }
    Err("worker did not emit Ready event".to_string())
}

async fn wait_for_first_stream_chunk(
    event_rx: &mut tokio::sync::mpsc::Receiver<AgentEvent>,
    worker_id: Uuid,
    started: Instant,
) -> Result<u128, String> {
    while let Some(event) = event_rx.recv().await {
        match event {
            AgentEvent::StreamChunk { worker_id: id, sequence, .. } if id == worker_id && sequence == 0 => {
                return Ok(started.elapsed().as_millis())
            }
            _ => {}
        }
    }

    Err("worker channel closed before first stream chunk".to_string())
}
