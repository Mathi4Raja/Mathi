use std::thread;
use std::time::Duration;

use mathi_runtime::Orchestrator;
use sysinfo::System;

#[tokio::main]
async fn main() {
    let idle_seconds = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(10);

    match run_probe(idle_seconds).await {
        Ok((rss_mb, startup_ms, limit_mb)) => {
            println!("Idle Memory Probe Results");
            println!("startup_ms={}", startup_ms);
            println!("idle_seconds={}", idle_seconds);
            println!("rss_mb={:.2}", rss_mb);
            println!("limit_mb={:.2}", limit_mb);

            if rss_mb < limit_mb {
                println!("result=PASS");
            } else {
                println!("result=FAIL");
                eprintln!("idle memory SLO violated: {:.2}MB >= {:.2}MB", rss_mb, limit_mb);
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("probe_error={error}");
            std::process::exit(1);
        }
    }
}

async fn run_probe(idle_seconds: u64) -> Result<(f64, u128, f64), String> {
    let start = std::time::Instant::now();
    let orchestrator = Orchestrator::new(4, 64);
    orchestrator
        .bootstrap()
        .await
        .map_err(|error| error.to_string())?;
    let startup_ms = start.elapsed().as_millis();

    thread::sleep(Duration::from_secs(idle_seconds));

    let current_pid = sysinfo::get_current_pid().map_err(|error| error.to_string())?;
    let mut system = System::new_all();
    system.refresh_all();

    let process = system
        .process(current_pid)
        .ok_or_else(|| "unable to resolve current process memory usage".to_string())?;

    let rss_mb = process.memory() as f64 / (1024.0 * 1024.0);
    let limit_mb = 150.0;

    Ok((rss_mb, startup_ms, limit_mb))
}
