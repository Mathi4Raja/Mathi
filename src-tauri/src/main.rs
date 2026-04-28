#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use mathi_runtime::Orchestrator;
use serde::Serialize;
use tauri::{Emitter, Window};

#[cfg(debug_assertions)]
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

const DEV_SERVER_ADDR: &str = "127.0.0.1:1420";
const APP_HTML_EMBEDDED: &str = include_str!("../dist/index.html");
const APP_HTML_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/dist/index.html");

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    start_shell_server();

    let orchestrator = Arc::new(Orchestrator::new(4, 100));

    tauri::Builder::default()
        .manage(orchestrator.clone())
        .invoke_handler(tauri::generate_handler![
            window_control,
            window_is_maximized,
            workspace_list_files,
            workspace_read_file,
            workspace_write_file,
            workspace_git_status,
            workspace_run_command,
        ])
        .setup(move |app| {
            let orchestrator = orchestrator.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = orchestrator.bootstrap().await {
                    eprintln!("runtime bootstrap failed: {error}");
                }
            });

            #[cfg(debug_assertions)]
            start_dev_hot_reload_watcher(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Mathi application");
}

#[tauri::command]
fn window_control(window: Window, action: String) -> Result<(), String> {
    match action.as_str() {
        "minimize" => window.minimize().map_err(|error| error.to_string()),
        "maximize" => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map_err(|error| error.to_string())
            } else {
                window.maximize().map_err(|error| error.to_string())
            }
        }
        "restore" => window.unmaximize().map_err(|error| error.to_string()),
        "close" => window.close().map_err(|error| error.to_string()),
        other => Err(format!("unknown window action: {other}")),
    }
}

#[allow(dead_code)]
#[tauri::command]
fn window_is_maximized(window: Window) -> Result<bool, String> {
    window.is_maximized().map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize)]
struct WorkspaceFile {
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct GitStatusSnapshot {
    branch: String,
    dirty_count: usize,
    summary: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CommandOutput {
    command: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

#[tauri::command]
fn workspace_list_files() -> Result<Vec<WorkspaceFile>, String> {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files).map_err(|error| error.to_string())?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

#[tauri::command]
fn workspace_read_file(path: String) -> Result<String, String> {
    let resolved = resolve_workspace_path(&path)?;
    fs::read_to_string(&resolved).map_err(|error| format!("{}: {}", resolved.display(), error))
}

#[tauri::command]
fn workspace_write_file(path: String, content: String) -> Result<(), String> {
    let resolved = resolve_workspace_path(&path)?;
    if let Some(parent) = resolved.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {}", parent.display(), error))?;
    }
    fs::write(&resolved, content).map_err(|error| format!("{}: {}", resolved.display(), error))
}

#[tauri::command]
fn workspace_git_status() -> Result<GitStatusSnapshot, String> {
    let root = workspace_root();
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--short", "--branch"])
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let stdout = String::from_utf8(output.stdout).map_err(|error| error.to_string())?;
    let mut lines = stdout.lines();
    let branch_line = lines.next().unwrap_or("## unknown");
    let branch = branch_line
        .trim_start_matches("## ")
        .split("...")
        .next()
        .unwrap_or("unknown")
        .to_string();
    let summary: Vec<String> = lines.map(ToString::to_string).collect();

    Ok(GitStatusSnapshot {
        branch,
        dirty_count: summary.len(),
        summary,
    })
}

#[tauri::command]
fn workspace_run_command(command_text: String, cwd: Option<String>) -> Result<CommandOutput, String> {
    let root = workspace_root();
    let working_dir = match cwd {
        Some(path) if !path.trim().is_empty() => resolve_workspace_path(&path)?,
        _ => root,
    };

    let mut shell_command = if cfg!(windows) {
        let mut shell = Command::new("cmd");
        shell.args(["/C", &command_text]);
        shell
    } else {
        let mut shell = Command::new("sh");
        shell.args(["-lc", &command_text]);
        shell
    };

    let output = shell_command
        .current_dir(working_dir)
        .output()
        .map_err(|error| error.to_string())?;

    let exit_code = output.status.code().unwrap_or(-1);
    Ok(CommandOutput {
        command: command_text,
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn resolve_workspace_path(relative_path: &str) -> Result<PathBuf, String> {
    let root = workspace_root();
    let mut resolved = root.clone();

    for component in Path::new(relative_path).components() {
        match component {
            Component::Normal(segment) => resolved.push(segment),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err("path escapes workspace root".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not allowed".to_string());
            }
        }
    }

    Ok(resolved)
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<WorkspaceFile>) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;

        if should_skip_entry(relative) {
            continue;
        }

        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }

        if file_type.is_file() {
            files.push(WorkspaceFile {
                path: relative.to_string_lossy().replace('\\', "/"),
            });
        }
    }

    Ok(())
}

fn should_skip_entry(relative: &Path) -> bool {
    relative.components().any(|component| {
        matches!(component, Component::Normal(name) if name == std::ffi::OsStr::new("target") || name == std::ffi::OsStr::new(".git") || name == std::ffi::OsStr::new("node_modules"))
    })
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

            let app_html = load_app_html();

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                app_html.len(),
                app_html
            );

            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
}

fn load_app_html() -> String {
    #[cfg(debug_assertions)]
    {
        if let Ok(html) = std::fs::read_to_string(APP_HTML_PATH) {
            return html;
        }
    }

    APP_HTML_EMBEDDED.to_string()
}

#[cfg(debug_assertions)]
fn start_dev_hot_reload_watcher(app: tauri::AppHandle) {
    thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(move |result| {
            let _ = tx.send(result);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("hot-reload watcher init failed: {error}");
                return;
            }
        };

        if let Err(error) = watcher.watch(Path::new(APP_HTML_PATH), RecursiveMode::NonRecursive) {
            eprintln!("hot-reload watch failed: {error}");
            return;
        }

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        let _ = app.emit("dev-reload", "dist/index.html changed");
                    }
                }
                Ok(Err(error)) => {
                    eprintln!("hot-reload watch error: {error}");
                }
                Err(_) => break,
            }
        }
    });
}
