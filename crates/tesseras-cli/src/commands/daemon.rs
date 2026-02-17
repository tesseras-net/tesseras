use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::init::expand_tilde;

fn pid_path(data_dir: &Path) -> PathBuf {
    data_dir.join("tesd.pid")
}

fn read_pid(data_dir: &Path) -> Option<u32> {
    std::fs::read_to_string(pid_path(data_dir))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn is_process_alive(pid: u32) -> bool {
    // Check if process exists via kill(pid, 0)
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Check if the daemon is running.
pub fn is_daemon_running(data_dir: &Path) -> bool {
    if let Some(pid) = read_pid(data_dir) {
        if is_process_alive(pid) {
            return true;
        }
        // Stale PID file — clean up
        let _ = std::fs::remove_file(pid_path(data_dir));
    }
    false
}

pub async fn run_start(data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);

    if is_daemon_running(&base) {
        let pid = read_pid(&base).unwrap();
        println!("Daemon already running (PID {pid})");
        return Ok(());
    }

    start_daemon(&base)?;
    println!("Daemon started");
    Ok(())
}

pub fn start_daemon(data_dir: &Path) -> Result<u32> {
    // Find tesd binary — check same directory as current binary first
    let tesd_path = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            let dir = exe.parent()?;
            let candidate = dir.join("tesd");
            candidate.exists().then_some(candidate)
        })
        .unwrap_or_else(|| PathBuf::from("tesd"));

    let child = Command::new(&tesd_path)
        .arg("--data-dir")
        .arg(data_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "failed to start daemon ({}). Is tesd installed?",
                tesd_path.display()
            )
        })?;

    let pid = child.id();
    std::fs::write(pid_path(data_dir), pid.to_string())
        .context("failed to write PID file")?;

    // Wait for daemon to become responsive (up to 15s)
    let socket_path = tesseras_rpc::default_socket_path().ok();
    if let Some(ref sock) = socket_path {
        for i in 0..30 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if tesseras_rpc::DaemonClient::connect(sock).is_ok() {
                return Ok(pid);
            }
            if i == 5 {
                eprintln!("Waiting for daemon to start...");
            }
        }
        eprintln!("Warning: daemon started (PID {pid}) but RPC socket not yet available");
    }

    Ok(pid)
}

pub async fn run_stop(data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);

    if let Some(pid) = read_pid(&base) {
        if is_process_alive(pid) {
            // Send SIGTERM
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
            // Wait up to 5s for exit
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if !is_process_alive(pid) {
                    break;
                }
            }
            if is_process_alive(pid) {
                eprintln!("Daemon did not stop gracefully, sending SIGKILL");
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
            let _ = std::fs::remove_file(pid_path(&base));
            println!("Daemon stopped (PID {pid})");
        } else {
            let _ = std::fs::remove_file(pid_path(&base));
            println!("Daemon not running (stale PID file cleaned)");
        }
    } else {
        println!("Daemon not running");
    }
    Ok(())
}

pub async fn run_status(data_dir: &str) -> Result<()> {
    let base = expand_tilde(data_dir);

    if let Some(pid) = read_pid(&base) {
        if is_process_alive(pid) {
            println!("Daemon running (PID {pid})");

            // Try to get peer count
            if let Ok(socket_path) = tesseras_rpc::default_socket_path() {
                if let Ok(mut client) = tesseras_rpc::DaemonClient::connect(&socket_path) {
                    if let Ok(resp) = client.call(&tesseras_rpc::Request::Peers) {
                        if let tesseras_rpc::Response::Peers { peers } = resp {
                            println!("  Peers: {}", peers.len());
                        }
                    }
                }
            }
        } else {
            let _ = std::fs::remove_file(pid_path(&base));
            println!("Daemon not running (stale PID file cleaned)");
        }
    } else {
        println!("Daemon not running");
    }
    Ok(())
}
