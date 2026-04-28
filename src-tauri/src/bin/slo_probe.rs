use std::collections::BTreeMap;
use std::time::Instant;

use mathi_runtime::AgentRequest;
use mathi_runtime::Orchestrator;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
struct SloReport {
    startup_ms: u128,
    handoff_p95_ms: u128,
    handoff_avg_ms: u128,
}

#[tokio::main]
async fn main() {
    match run_probe().await {
        Ok(report) => {
            println!("SLO Probe Results");
            println!("startup_ms={}", report.startup_ms);
            println!("handoff_p95_ms={}", report.handoff_p95_ms);
            println!("handoff_avg_ms={}", report.handoff_avg_ms);

            let startup_ok = report.startup_ms < 1_000;
            let handoff_ok = report.handoff_p95_ms < 50;
            if startup_ok && handoff_ok {
                println!("result=PASS");
                return;
            }

            println!("result=FAIL");
            if !startup_ok {
                eprintln!(
                    "startup SLO violated: {}ms >= 1000ms",
                    report.startup_ms
                );
            }
            if !handoff_ok {
                eprintln!(
                    "handoff SLO violated: p95 {}ms >= 50ms",
                    report.handoff_p95_ms
                );
            }
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("probe_error={error}");
            std::process::exit(1);
        }
    }
}

async fn run_probe() -> Result<SloReport, String> {
    let orchestrator = Orchestrator::new(4, 64);

    let startup_begin = Instant::now();
    orchestrator
        .bootstrap()
        .await
        .map_err(|error| error.to_string())?;
    let startup_ms = startup_begin.elapsed().as_millis();

    let mut handoff_samples: Vec<u128> = Vec::with_capacity(20);
    for _ in 0..20 {
        let mut context = BTreeMap::new();
        context.insert("action_class".to_string(), Value::String("read".to_string()));
        context.insert("agent_id".to_string(), Value::String("slo-probe".to_string()));

        let request = AgentRequest {
            id: Uuid::new_v4(),
            task_type: "slo-probe-task".to_string(),
            payload: serde_json::json!({}),
            deadline_ms: Some(2_000),
            context,
        };

        let dispatch_begin = Instant::now();
        orchestrator
            .dispatch(request)
            .await
            .map_err(|error| error.to_string())?;
        handoff_samples.push(dispatch_begin.elapsed().as_millis());
    }

    handoff_samples.sort_unstable();
    let handoff_p95_ms = percentile_u128(&handoff_samples, 95).unwrap_or(0);
    let handoff_avg_ms = if handoff_samples.is_empty() {
        0
    } else {
        handoff_samples.iter().sum::<u128>() / handoff_samples.len() as u128
    };

    Ok(SloReport {
        startup_ms,
        handoff_p95_ms,
        handoff_avg_ms,
    })
}

fn percentile_u128(values: &[u128], percentile: usize) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    let index = ((percentile * values.len()).saturating_sub(1)) / 100;
    values.get(index).copied()
}
