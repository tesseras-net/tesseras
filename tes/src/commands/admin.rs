use anyhow::Result;

use tesseras::config::DataDir;
use tesseras::crypto::Identity;
use tesseras::node::Node;
use tesseras::rpc::{self, RpcRequest, RpcResponse, RpcServer};

#[derive(clap::Subcommand)]
pub enum AdminCommand {
    /// Manage bootstrap nodes
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    /// Manage the daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    /// Show node identity info
    Id,
}

#[derive(clap::Subcommand)]
pub enum BootstrapCommand {
    /// Add a bootstrap node
    Add { addr: String },
    /// Remove a bootstrap node
    Rm { addr: String },
    /// List bootstrap nodes
    Ls,
}

#[derive(clap::Subcommand)]
pub enum DaemonCommand {
    /// Start the daemon (backgrounds by default)
    Start {
        /// Run in foreground (for systemd/launchd)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
    /// Tail daemon logs
    Logs {
        /// Number of lines to show
        #[arg(short, long, default_value = "50")]
        lines: usize,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
}

pub fn run(data_dir: &DataDir, command: AdminCommand) -> Result<()> {
    match command {
        AdminCommand::Bootstrap { command } => run_bootstrap(data_dir, command),
        AdminCommand::Daemon { command } => run_daemon(data_dir, command),
        AdminCommand::Id => run_id(data_dir),
    }
}

fn run_bootstrap(data_dir: &DataDir, command: BootstrapCommand) -> Result<()> {
    match command {
        BootstrapCommand::Add { addr } => {
            let mut config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
            if !config.bootstrap.contains(&addr) {
                config.bootstrap.push(addr.clone());
                data_dir
                    .save_config(&config)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            eprintln!("Added bootstrap: {addr}");
            Ok(())
        }
        BootstrapCommand::Rm { addr } => {
            let mut config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
            config.bootstrap.retain(|a| a != &addr);
            data_dir
                .save_config(&config)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            eprintln!("Removed bootstrap: {addr}");
            Ok(())
        }
        BootstrapCommand::Ls => {
            let config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
            if config.bootstrap.is_empty() {
                eprintln!("No bootstrap nodes configured.");
            } else {
                for addr in &config.bootstrap {
                    println!("{addr}");
                }
            }
            Ok(())
        }
    }
}

fn run_daemon(data_dir: &DataDir, command: DaemonCommand) -> Result<()> {
    match command {
        DaemonCommand::Start { foreground } => {
            let pid_path = data_dir.root().join("daemon.pid");

            // Check if already running
            if pid_path.exists() {
                let pid_str = std::fs::read_to_string(&pid_path)?;
                if is_process_alive(pid_str.trim()) {
                    anyhow::bail!(
                        "Daemon already running (PID: {}). Use 'tes admin daemon stop' first.",
                        pid_str.trim()
                    );
                }
                // Stale PID file, clean up
                let _ = std::fs::remove_file(&pid_path);
                let sock_path = rpc::socket_path(data_dir.root());
                let _ = std::fs::remove_file(&sock_path);
            }

            if foreground {
                run_daemon_foreground(data_dir)
            } else {
                run_daemon_background(data_dir)
            }
        }
        DaemonCommand::Stop => {
            let pid_path = data_dir.root().join("daemon.pid");
            if pid_path.exists() {
                let pid_str = std::fs::read_to_string(&pid_path)?;
                let pid = pid_str.trim();
                eprintln!("Stopping daemon (PID: {pid})...");
                #[cfg(unix)]
                {
                    let _ = std::process::Command::new("kill").arg(pid).status();
                }
                // Wait briefly for graceful shutdown
                std::thread::sleep(std::time::Duration::from_millis(500));
                let _ = std::fs::remove_file(&pid_path);
                let sock_path = rpc::socket_path(data_dir.root());
                let _ = std::fs::remove_file(&sock_path);
                eprintln!("Daemon stopped.");
            } else {
                eprintln!("No daemon PID file found.");
            }
            Ok(())
        }
        DaemonCommand::Status => {
            let pid_path = data_dir.root().join("daemon.pid");
            if pid_path.exists() {
                let pid_str = std::fs::read_to_string(&pid_path)?;
                let pid = pid_str.trim();
                if is_process_alive(pid) {
                    eprintln!("Daemon running (PID: {pid})");

                    // Try RPC ping for more info
                    let rt = tokio::runtime::Runtime::new()?;
                    match rt.block_on(rpc::send_request(data_dir.root(), &RpcRequest::Ping)) {
                        Ok(RpcResponse::Pong {
                            node_id,
                            peer_count,
                            listen_addr,
                        }) => {
                            eprintln!("Node ID:     {node_id}");
                            eprintln!("Listen:      {listen_addr}");
                            eprintln!("Peers:       {peer_count}");
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("RPC: {e}");
                        }
                    }
                } else {
                    eprintln!("Daemon not running (stale PID file: {pid})");
                    let _ = std::fs::remove_file(&pid_path);
                }
            } else {
                eprintln!("Daemon is not running.");
            }
            Ok(())
        }
        DaemonCommand::Logs { lines, follow } => {
            let log_path = data_dir.root().join("daemon.log");
            if !log_path.exists() {
                eprintln!("No log file found at {}", log_path.display());
                return Ok(());
            }
            let mut cmd = std::process::Command::new("tail");
            cmd.arg("-n").arg(lines.to_string());
            if follow {
                cmd.arg("-f");
            }
            cmd.arg(&log_path);
            let status = cmd.status()?;
            if !status.success() {
                anyhow::bail!("tail exited with {status}");
            }
            Ok(())
        }
    }
}

/// Run the daemon in the foreground (blocking). Used by --foreground and by the
/// backgrounded child process.
fn run_daemon_foreground(data_dir: &DataDir) -> Result<()> {
    let pid_path = data_dir.root().join("daemon.pid");
    let pid = std::process::id();
    std::fs::write(&pid_path, pid.to_string())?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let result = run_daemon_async(data_dir).await;
        // Clean up PID file on exit
        let _ = std::fs::remove_file(&pid_path);
        result
    })
}

/// Spawn the daemon as a background process by re-execing with --foreground.
fn run_daemon_background(data_dir: &DataDir) -> Result<()> {
    let log_path = data_dir.root().join("daemon.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let log_stderr = log_file.try_clone()?;

    let exe = std::env::current_exe()?;
    let mut args: Vec<String> = Vec::new();

    // Reconstruct the identity flag if the data dir is non-default
    let default_path = DataDir::default_path();
    if data_dir.root() != default_path {
        args.push("--identity".into());
        args.push(data_dir.root().to_string_lossy().into());
    }

    args.extend_from_slice(&[
        "admin".into(),
        "daemon".into(),
        "start".into(),
        "--foreground".into(),
    ]);

    let child = std::process::Command::new(exe)
        .args(&args)
        .stdout(log_file)
        .stderr(log_stderr)
        .stdin(std::process::Stdio::null())
        .spawn()?;

    eprintln!("Daemon started (PID: {})", child.id());
    eprintln!("Logs: {}", log_path.display());
    Ok(())
}

/// The async daemon main loop.
async fn run_daemon_async(data_dir: &DataDir) -> Result<()> {
    let key_path = data_dir.identity_key_path();
    let identity = if key_path.exists() {
        Identity::load(&key_path)?
    } else {
        let id = Identity::generate();
        id.save(&key_path)?;
        id
    };

    let config = data_dir.load_config().map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut node = Node::new(data_dir.clone(), identity, config)?;

    let addr = node.start().await?;
    eprintln!("Node listening on {addr}");
    eprintln!("Node ID: {}", node.node_id());

    // Bootstrap
    let discovered = node.bootstrap().await?;
    eprintln!("Bootstrap: discovered {discovered} peers");

    // Start background maintenance tasks
    node.start_refresh_loop();
    node.start_repair_loop();

    // Start RPC server
    let rpc_server = RpcServer::bind(data_dir.root()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let shutdown_rx = node
        .shutdown_rx()
        .ok_or_else(|| anyhow::anyhow!("node not started"))?;

    let listen_addr = addr.to_string();
    let node = std::sync::Arc::new(node);

    let rpc_node = node.clone();
    let rpc_handle = tokio::spawn(async move {
        let listen_addr = listen_addr;
        rpc_server
            .serve(
                move |req| {
                    let node = rpc_node.clone();
                    let listen_addr = listen_addr.clone();
                    async move { handle_rpc_request(req, &node, &listen_addr).await }
                },
                shutdown_rx,
            )
            .await;
    });

    eprintln!("Daemon ready.");

    // Wait for SIGTERM/SIGINT
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;
        tokio::select! {
            _ = sigterm.recv() => {
                eprintln!("Received SIGTERM, shutting down...");
            }
            _ = sigint.recv() => {
                eprintln!("Received SIGINT, shutting down...");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
        eprintln!("Received Ctrl+C, shutting down...");
    }

    node.shutdown();
    let _ = rpc_handle.await;
    eprintln!("Daemon stopped.");
    Ok(())
}

/// Handle an RPC request using the node.
async fn handle_rpc_request(request: RpcRequest, node: &Node, listen_addr: &str) -> RpcResponse {
    let node_id = node.node_id();

    match request {
        RpcRequest::Ping => RpcResponse::Pong {
            node_id: node_id.to_string(),
            peer_count: node.dht.lock().unwrap().routing_table.len(),
            listen_addr: listen_addr.to_string(),
        },
        RpcRequest::ListTesseras => {
            let storage = node.storage.lock().unwrap();
            match storage.list_tesseras() {
                Ok(list) => RpcResponse::TesseraList(list),
                Err(e) => RpcResponse::Error(e.to_string()),
            }
        }
        RpcRequest::GetTessera { hash } => {
            let storage = node.storage.lock().unwrap();
            match storage.find_tessera(&hash) {
                Ok(Some(t)) => RpcResponse::Tessera(t),
                Ok(None) => RpcResponse::Error(format!("tessera not found: {hash}")),
                Err(e) => RpcResponse::Error(e.to_string()),
            }
        }
        RpcRequest::AddTessera { .. } => {
            // TODO: add_tessera via RPC requires passing file data — not yet supported
            RpcResponse::Error("add via RPC not yet supported — use tes add directly".into())
        }
        RpcRequest::RemoveTessera { hash } => {
            let storage = node.storage.lock().unwrap();
            match storage.delete_tessera(&hash) {
                Ok(()) => RpcResponse::Ok,
                Err(e) => RpcResponse::Error(e.to_string()),
            }
        }
        RpcRequest::NodeStatus => {
            let storage = node.storage.lock().unwrap();
            let tessera_count = storage.list_tesseras().map(|t| t.len()).unwrap_or(0);
            RpcResponse::Status {
                node_id: node_id.to_string(),
                peer_count: node.dht.lock().unwrap().routing_table.len(),
                tessera_count,
                listen_addr: listen_addr.to_string(),
            }
        }
        RpcRequest::CheckFragments => match node.check_fragments() {
            Ok(missing_list) => {
                let total_ok = {
                    let storage = node.storage.lock().unwrap();
                    let tesseras = match storage.list_tesseras() {
                        Ok(t) => t,
                        Err(e) => return RpcResponse::Error(e.to_string()),
                    };
                    let mut ok = 0usize;
                    for tessera in &tesseras {
                        for memory in &tessera.memories {
                            let fragments = match storage.find_fragments(&memory.blob_hash) {
                                Ok(f) => f,
                                Err(_) => continue,
                            };
                            for meta in &fragments {
                                if storage.has_blob(&meta.fragment_hash) {
                                    ok += 1;
                                }
                            }
                        }
                    }
                    ok
                };
                let missing = missing_list
                    .into_iter()
                    .map(|(bh, idx, fh)| (bh.to_string(), idx, fh.to_string()))
                    .collect();
                RpcResponse::FragmentHealth { total_ok, missing }
            }
            Err(e) => RpcResponse::Error(e.to_string()),
        },
        RpcRequest::FetchTesseraFromNetwork { hash } => {
            match node.fetch_tessera_from_network(&hash).await {
                Ok(Some(t)) => RpcResponse::Tessera(t),
                Ok(None) => RpcResponse::Error(format!("tessera not found on network: {hash}")),
                Err(e) => RpcResponse::Error(e.to_string()),
            }
        }
        RpcRequest::AnnounceTessera { hash } => match node.announce_tessera(&hash).await {
            Ok(stored) => RpcResponse::Pong {
                node_id: node_id.to_string(),
                peer_count: stored,
                listen_addr: listen_addr.to_string(),
            },
            Err(e) => RpcResponse::Error(e.to_string()),
        },
        RpcRequest::DistributeFragments { hash } => {
            let tessera = {
                let storage = node.storage.lock().unwrap();
                match storage.find_tessera(&hash) {
                    Ok(Some(t)) => t,
                    Ok(None) => return RpcResponse::Error(format!("tessera not found: {hash}")),
                    Err(e) => return RpcResponse::Error(e.to_string()),
                }
            };
            match node.distribute_fragments(&tessera).await {
                Ok(count) => RpcResponse::Pong {
                    node_id: node_id.to_string(),
                    peer_count: count,
                    listen_addr: listen_addr.to_string(),
                },
                Err(e) => RpcResponse::Error(e.to_string()),
            }
        }
    }
}

fn run_id(data_dir: &DataDir) -> Result<()> {
    let key_path = data_dir.identity_key_path();
    if key_path.exists() {
        let identity = tesseras::crypto::Identity::load(&key_path)?;
        println!("Node ID:     {}", identity.node_id());
        println!("Public Key:  {}", hex::encode(identity.public_key_bytes()));
        println!("Key File:    {}", key_path.display());
    } else {
        eprintln!("No identity found. Run any command to auto-create one.");
    }
    Ok(())
}

#[cfg(unix)]
fn is_process_alive(pid_str: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid_str])
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(not(unix))]
fn is_process_alive(_pid_str: &str) -> bool {
    false
}
