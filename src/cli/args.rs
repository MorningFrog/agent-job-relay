//! Command-line syntax; execution and side effects live in the parent module.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Durable background jobs for local AI agents",
    long_about = "Run commands independently of MCP clients. The daemon starts automatically on task submission. Commands run with your OS permissions, not in a sandbox. JSON output is bounded; host acceptance is not confirmed Agent execution."
)]
pub(super) struct Cli {
    #[arg(
        long,
        global = true,
        env = "AJR_STATE_DIR",
        help = "Private state directory (default: installed profile or XDG state)"
    )]
    pub(super) state_dir: Option<PathBuf>,
    #[command(subcommand)]
    pub(super) command: Commands,
}
#[derive(Subcommand)]
pub(super) enum Commands {
    /// Export, bootstrap or update a plugin runtime and explicitly refresh caches
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Run in foreground, explicitly start, or stop the selected daemon
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
    #[cfg(windows)]
    #[command(hide = true)]
    WindowsBridge {
        #[command(subcommand)]
        command: WindowsBridgeCommand,
    },
    #[cfg(windows)]
    #[command(hide = true)]
    WindowsBootstrap {
        endpoint: PathBuf,
        parent_identity: String,
    },
    #[cfg(windows)]
    #[command(hide = true)]
    WindowsWorkload { job_id: String, job_name: String },
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
        #[arg(long, default_value = agent_job_relay::platform::DEFAULT_SHELL)]
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
pub(super) enum PluginCommand {
    /// Inspect/finish one interrupted installation or cache replacement
    Repair {
        /// Exact installation or cache directory with a pending operation
        #[arg(long)]
        path: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Export an offline native-platform bundle with native runtime; no recipient Rust toolchain
    Pack {
        #[arg(long)]
        output: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// From a bundle's relay: install runtime and register the local marketplace
    Bootstrap {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Import a checked bundle; keep the installed profile and refuse downgrades
    Update {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long = "from")]
        bundle: PathBuf,
        /// Explicit host cache directory; repeat for multiple caches
        #[arg(long)]
        cache: Vec<PathBuf>,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Inspect the managed installation and caller-specified cache snapshots
    Status {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        cache: Vec<PathBuf>,
    },
    /// Manage only explicitly selected cache directories, never scan host storage
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
}
#[cfg(windows)]
#[derive(Subcommand)]
pub(super) enum WindowsBridgeCommand {
    Prepare,
    Secure { endpoint: PathBuf, server_pid: u32 },
}
#[derive(Subcommand)]
pub(super) enum CacheCommand {
    /// Replace an unchanged cache snapshot from the installed plugin; retain a backup
    Refresh {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        path: PathBuf,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
}
#[derive(Subcommand)]
pub(super) enum IntegrationCommand {
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
pub(super) enum SessionCommand {
    /// Preview recovery for this exact chat; --apply rebinds unexpired notifications
    Recover {
        #[arg(long)]
        apply: bool,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long, default_value_t = 0)]
        after: i64,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
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
        /// Renew this owned Desktop binding after a same-endpoint probe at delivery
        #[arg(long)]
        renew_on_delivery: bool,
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
pub(super) enum DaemonCommand {
    /// Start independently without submitting a job; use outside restrictive host jobs
    Start {
        /// Use this install's recorded state, overriding --state-dir/AJR_STATE_DIR
        #[arg(long)]
        install_root: Option<PathBuf>,
    },
    /// Stop only this daemon; independent runners continue and data is retained
    Stop {
        /// Use this install's recorded state, overriding --state-dir/AJR_STATE_DIR
        #[arg(long)]
        install_root: Option<PathBuf>,
    },
}
#[derive(Subcommand)]
pub(super) enum DeliveryCommand {
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
pub(super) enum TargetCommand {
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
