use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use mathi_runtime::Orchestrator;

const DEV_SERVER_ADDR: &str = "127.0.0.1:1420";
const APP_HTML: &str = include_str!("../dist/index.html");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    start_shell_server();

    let orchestrator = Arc::new(Orchestrator::new(4, 100));

    tauri::Builder::default()
        .manage(orchestrator.clone())
        .setup(move |_app| {
            let orchestrator = orchestrator.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = orchestrator.bootstrap().await {
                    eprintln!("runtime bootstrap failed: {error}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Mathi application");
}

fn start_shell_server() {
    let listener = TcpListener::bind(DEV_SERVER_ADDR).expect("bind local shell server");

    thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(mut stream) = incoming else {
                continue;
            };

            let mut request_buffer = [0_u8; 1024];
            let _ = stream.read(&mut request_buffer);

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                APP_HTML.len(),
                APP_HTML
            );

            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
}
