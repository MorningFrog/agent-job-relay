use agent_job_relay::{
    config::Config,
    core::{CommandSpec, StartSpec},
    ipc::{self, Request},
};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{collections::BTreeMap, path::PathBuf};
#[derive(Parser)]
#[command(
    version,
    about = "Durable background jobs for local AI agents",
    long_about = "Run commands independently of MCP clients. The daemon starts automatically on task submission. Commands run with your OS permissions, not in a sandbox. JSON output is bounded; host acceptance is not confirmed Agent execution."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "AJR_STATE_DIR",
        help = "Private state directory (default: installed profile or XDG state)"
    )]
    state_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Run in foreground, or stop the daemon; start auto-launches it when needed
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
    /// Serve MCP on stdio; submissions automatically start the daemon
    Mcp {
        /// Ignore inherited chat identity; use a current-terminal session binding
        #[arg(long)]
        shared_host: bool,
    },
    /// Preview/install a stable native executable and local Desktop plugin
    Install {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Preview/remove unchanged owned install files; preserve all task data
    Uninstall {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Register, refresh, probe or invalidate an exact Agent session
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Inspect durable notifications for one explicitly registered session
    Inbox {
        session_key: String,
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Inspect deliveries, attempt history or explicitly retry failed delivery
    Delivery {
        #[command(subcommand)]
        command: DeliveryCommand,
    },
    /// Bind jobs to exact session registrations; no latest-session routing
    Target {
        #[command(subcommand)]
        command: TargetCommand,
    },
    /// Preview MCP or local plugin marketplace configuration; write only with --apply
    Setup {
        client: agent_job_relay::integration::Client,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Remove only an unchanged, owned integration entry; job history is preserved
    Integration {
        #[command(subcommand)]
        command: IntegrationCommand,
    },
    /// Send a JSON IPC request; useful for explicit session and delivery operations
    Request { json: String },
    /// Execute explicit argv, or opt into a shell with --shell
    Start {
        #[arg(long, conflicts_with = "command")]
        shell: Option<String>,
        #[arg(long, default_value = "/bin/sh")]
        interpreter: String,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(long)]
        session: Option<String>,
        #[arg(
            long,
            conflicts_with = "session",
            help = "Opt into experimental Desktop wake; bind the current host session automatically"
        )]
        wake: bool,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[arg(last = true, required_unless_present = "shell")]
        command: Vec<String>,
    },
    /// Show durable job/run state, exit outcome and process identity
    Status { job_id: String },
    /// List jobs with a bounded page and continuation cursor
    List {
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Read bounded stdout/stderr bytes or follow output
    Logs {
        job_id: String,
        #[arg(
            long,
            help = "Follow output until terminal state; Ctrl-C stops only the viewer"
        )]
        follow: bool,
        #[arg(long, default_value = "stdout")]
        stream: String,
        #[arg(long, default_value_t = 0)]
        cursor: u64,
        #[arg(long, default_value_t = 8192)]
        limit: u32,
    },
    /// Read durable events after a monotonic cursor
    Events {
        job_id: String,
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Wait at most 30 seconds for new events or terminal state
    Wait {
        job_id: String,
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 10)]
        seconds: u32,
    },
    /// Request idempotent process-group cancellation
    Stop { job_id: String },
    /// Diagnose connectivity and persistent store health
    Doctor {
        #[arg(long)]
        client: Option<agent_job_relay::integration::Client>,
        #[arg(long, requires = "client")]
        config: Option<PathBuf>,
        /// Exercise stdio initialize/tools discovery without contacting a Desktop host
        #[arg(long)]
        check_mcp: bool,
    },
    #[command(hide = true)]
    Runner { job_id: String, nonce: String },
}
#[derive(Subcommand)]
enum IntegrationCommand {
    Remove {
        client: agent_job_relay::integration::Client,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Subcommand)]
enum SessionCommand {
    /// Register and probe the current Desktop context (experimental opt-in)
    Connect {
        /// Return only the binding key and capability, without private host paths
        #[arg(long)]
        brief: bool,
    },
    Register {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        session_id: String,
        #[arg(long)]
        instance_id: String,
        #[arg(long)]
        host_id: String,
        #[arg(long)]
        endpoint: Option<PathBuf>,
        #[arg(long)]
        caller_thread_id: Option<String>,
        #[arg(long)]
        experimental: bool,
        #[arg(long, default_value_t = 3600)]
        lease_seconds: u32,
    },
    List {
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    Probe {
        session_key: String,
    },
    Invalidate {
        session_key: String,
    },
}
#[derive(Subcommand)]
enum DaemonCommand {
    /// Stop only this daemon; independent runners continue and data is retained
    Stop,
}
#[derive(Subcommand)]
enum DeliveryCommand {
    List {
        #[arg(long)]
        job_id: Option<String>,
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    Attempts {
        delivery_id: String,
        #[arg(long, default_value_t = 0)]
        after: u32,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    Retry {
        delivery_id: String,
    },
}
#[derive(Subcommand)]
enum TargetCommand {
    Add {
        job_id: String,
        session_key: String,
        #[arg(long)]
        replay: bool,
    },
    Rebind {
        target_id: String,
        session_key: String,
    },
}
#[tokio::main(worker_threads = 2)]
async fn main() {
    agent_job_relay::platform::initialize();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    if let Err(e) = run(Cli::parse()).await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
async fn run(cli: Cli) -> Result<()> {
    let remove_install = matches!(&cli.command, Commands::Uninstall { .. });
    let mut state = cli.state_dir;
    if state.is_none()
        && let Commands::Install { root, .. } | Commands::Uninstall { root, .. } = &cli.command
    {
        let root = root
            .clone()
            .map(Ok)
            .unwrap_or_else(agent_job_relay::deployment::default_root)?;
        state = agent_job_relay::deployment::installation_state(&root)?;
    }
    let config = Config::new(state)?;
    let req = match cli.command {
        Commands::Doctor {
            client,
            config: path,
            check_mcp,
        } => {
            let mut report = agent_job_relay::diagnostics::report(&config, true).await?;
            if check_mcp {
                report["stdio_self_check"] =
                    agent_job_relay::diagnostics::check_mcp(&config).await?;
            }
            report["mcp_registration"] = serde_json::json!(
                "not checked; use doctor --client codex --config <absolute-path>"
            );
            if let Some(client) = client {
                let path = path
                    .map(Ok)
                    .unwrap_or_else(|| agent_job_relay::integration::default_path(client))?;
                let preview =
                    agent_job_relay::integration::configure(&config, client, path, false, false)?;
                report["mcp_registration"] = serde_json::json!({"config_path":preview["config_path"],"matches_current_binary":preview["changed"]==false,"next_step":"Use setup --dry-run; restart the client after applying. Registration alone cannot verify a live MCP connection."});
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
            return Ok(());
        }
        Commands::Logs {
            job_id,
            stream,
            cursor,
            limit,
            follow: true,
        } => {
            let mut cursor = cursor;
            loop {
                let v = ipc::call(
                    &config,
                    Request::Logs {
                        job_id: job_id.clone(),
                        stream: stream.clone(),
                        cursor,
                        limit,
                    },
                )
                .await?;
                cursor = v["next_cursor"].as_u64().unwrap();
                let bytes = v["bytes"].as_u64().unwrap();
                if bytes > 0 {
                    print!("{}", v["text"].as_str().unwrap_or(""));
                    use std::io::Write;
                    std::io::stdout().flush()?;
                }
                if bytes == 0 {
                    let status = ipc::call(
                        &config,
                        Request::Status {
                            job_id: job_id.clone(),
                        },
                    )
                    .await?;
                    if agent_job_relay::core::terminal(status["status"].as_str().unwrap_or("")) {
                        break;
                    }
                }
                if bytes > 0 {
                    continue;
                }
                tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_millis(200))=>{}}
            }
            return Ok(());
        }
        Commands::Session { command } => match command {
            SessionCommand::Connect { brief } => {
                let value = agent_job_relay::client::connect(&config).await?;
                let value = if brief {
                    serde_json::json!({"session_key":value["session"]["key"],"probe":"passed","can_resume":true,"can_confirm_resume":false})
                } else {
                    value
                };
                println!("{}", serde_json::to_string_pretty(&value)?);
                return Ok(());
            }
            SessionCommand::Register {
                agent,
                session_id,
                instance_id,
                host_id,
                endpoint,
                caller_thread_id,
                experimental,
                lease_seconds,
            } => Request::RegisterSession {
                registration: agent_job_relay::session::Registration {
                    agent_kind: agent,
                    session_id,
                    instance_id,
                    host_id,
                    adapter: if endpoint.is_some() {
                        agent_job_relay::session::Adapter::DesktopExperimental
                    } else {
                        agent_job_relay::session::Adapter::Inbox
                    },
                    endpoint,
                    caller_thread_id,
                    experimental,
                    lease_seconds,
                },
            },
            SessionCommand::List { after, limit } => Request::Sessions { after, limit },
            SessionCommand::Probe { session_key } => Request::ProbeSession { session_key },
            SessionCommand::Invalidate { session_key } => {
                Request::InvalidateSession { session_key }
            }
        },
        Commands::Inbox {
            session_key,
            after,
            limit,
        } => Request::Inbox {
            session_key,
            after,
            limit,
        },
        Commands::Delivery { command } => match command {
            DeliveryCommand::List {
                job_id,
                after,
                limit,
            } => Request::Deliveries {
                job_id,
                after,
                limit,
            },
            DeliveryCommand::Attempts {
                delivery_id,
                after,
                limit,
            } => Request::DeliveryAttempts {
                delivery_id,
                after,
                limit,
            },
            DeliveryCommand::Retry { delivery_id } => Request::RetryDelivery { delivery_id },
        },
        Commands::Target { command } => match command {
            TargetCommand::Add {
                job_id,
                session_key,
                replay,
            } => Request::AddWakeTarget {
                job_id,
                session_key,
                replay,
            },
            TargetCommand::Rebind {
                target_id,
                session_key,
            } => Request::RebindTarget {
                target_id,
                session_key,
            },
        },
        Commands::Setup {
            client,
            config: path,
            apply,
            ..
        } => {
            let path = path
                .map(Ok)
                .unwrap_or_else(|| agent_job_relay::integration::default_path(client))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&agent_job_relay::integration::configure(
                    &config, client, path, false, apply
                )?)?
            );
            return Ok(());
        }
        Commands::Integration {
            command:
                IntegrationCommand::Remove {
                    client,
                    config: path,
                    apply,
                    ..
                },
        } => {
            let path = path
                .map(Ok)
                .unwrap_or_else(|| agent_job_relay::integration::default_path(client))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&agent_job_relay::integration::configure(
                    &config, client, path, true, apply
                )?)?
            );
            return Ok(());
        }

        Commands::Mcp { shared_host } => {
            return agent_job_relay::mcp::serve(config, shared_host).await;
        }
        Commands::Install { root, apply, .. } | Commands::Uninstall { root, apply, .. } => {
            let root = root
                .map(Ok)
                .unwrap_or_else(agent_job_relay::deployment::default_root)?;
            let result = tokio::task::spawn_blocking(move || {
                agent_job_relay::deployment::change(&config, root, remove_install, apply)
            })
            .await??;
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Commands::Request { json } => serde_json::from_str(&json)?,
        Commands::Daemon { command: None } => return agent_job_relay::daemon::serve(config).await,
        Commands::Daemon {
            command: Some(DaemonCommand::Stop),
        } => Request::Shutdown,
        Commands::Runner { job_id, nonce } => {
            return agent_job_relay::runner::run(config, job_id, nonce).await;
        }
        Commands::Start {
            shell,
            interpreter,
            cwd,
            timeout,
            session,
            wake,
            idempotency_key,
            command,
        } => {
            let command = if let Some(script) = shell {
                CommandSpec::Shell {
                    interpreter,
                    script,
                }
            } else {
                CommandSpec::Exec {
                    program: command[0].clone(),
                    args: command[1..].to_vec(),
                }
            };
            let cwd = cwd.unwrap_or(std::env::current_dir()?);
            let cwd = if cwd.is_absolute() {
                cwd
            } else {
                std::env::current_dir()?.join(cwd)
            };
            let spec = StartSpec {
                command,
                cwd,
                env: BTreeMap::new(),
                timeout_seconds: timeout,
                session,
                idempotency_key,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &agent_job_relay::client::start(&config, spec, wake).await?
                )?
            );
            return Ok(());
        }
        Commands::Status { job_id } => Request::Status { job_id },
        Commands::List { after, limit } => Request::List { after, limit },
        Commands::Logs {
            follow: false,
            job_id,
            stream,
            cursor,
            limit,
        } => Request::Logs {
            job_id,
            stream,
            cursor,
            limit,
        },
        Commands::Events {
            job_id,
            after,
            limit,
        } => Request::Events {
            job_id,
            after,
            limit,
        },
        Commands::Wait {
            job_id,
            after,
            seconds,
        } => Request::Wait {
            job_id,
            after,
            seconds,
        },
        Commands::Stop { job_id } => Request::Stop { job_id },
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&agent_job_relay::client::call(&config, req).await?)?
    );
    Ok(())
}
