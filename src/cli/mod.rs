//! CLI dispatch and profile selection. Runtime behavior stays in the library.

mod args;

use agent_job_relay::{
    config::Config,
    core::{CommandSpec, StartSpec},
    ipc::{self, Request},
};
use anyhow::{Context, Result};
use args::*;
use clap::Parser;
use std::{collections::BTreeMap, path::PathBuf};

pub(super) async fn run() -> Result<()> {
    let cli = Cli::parse();
    let online_remove = matches!(&cli.command, Commands::Uninstall { root: None, .. })
        && std::env::var_os("AJR_NPM_LAUNCHER").is_some();
    let remove_install = matches!(&cli.command, Commands::Uninstall { .. });
    let mut state = cli.state_dir;
    if let Commands::Plugin {
        command: PluginCommand::Bootstrap { root, .. } | PluginCommand::Install { root, .. },
    } = &cli.command
    {
        let root = root
            .clone()
            .map(Ok)
            .unwrap_or_else(agent_job_relay::deployment::plugin_root)?;
        // A lifecycle operation cannot switch an existing profile through an
        // unrelated exported AJR_STATE_DIR. Fresh bootstrap may select a state.
        if let Some(installed) = agent_job_relay::deployment::installation_state(&root)? {
            state = Some(installed);
        }
    }
    if let Commands::Daemon {
        command:
            Some(
                DaemonCommand::Stop {
                    install_root: Some(root),
                }
                | DaemonCommand::Start {
                    install_root: Some(root),
                },
            ),
    } = &cli.command
    {
        state = Some(agent_job_relay::deployment::installation_state(root)?
            .context("no owned installation at --install-root; use an existing installation or an explicit --state-dir")?);
    }
    if state.is_none()
        && !online_remove
        && let Commands::Install { root, .. } | Commands::Uninstall { root, .. } = &cli.command
    {
        let root = root
            .clone()
            .map(Ok)
            .unwrap_or_else(agent_job_relay::deployment::default_root)?;
        state = agent_job_relay::deployment::installation_state(&root)?;
    }
    if let Commands::Ui {
        install_root: Some(root),
        ..
    } = &cli.command
    {
        state = Some(
            agent_job_relay::deployment::installation_state(root)?
                .context("no owned installation at --install-root")?,
        );
    }
    let config = Config::new(state)?;
    if let Commands::Uninstall {
        cli_only, force, ..
    } = &cli.command
    {
        anyhow::ensure!(
            online_remove || (!cli_only && !force),
            "--cli-only/--force require the global npm entry without --root"
        );
    }
    let req = match cli.command {
        Commands::Update {
            dry_run,
            cli_only,
            to,
        } => {
            anyhow::ensure!(
                std::env::var_os("AJR_NPM_LAUNCHER").is_some(),
                "Online updates require the npm CLI. Install it once: npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest. Native installations use plugin install --from <bundle>."
            );
            if let Some(version) = &to {
                agent_job_relay::online_update::validate_version(version)?;
            }
            println!(
                "{}",
                serde_json::json!({"operation":"online_update","dry_run":dry_run,"cli_only":cli_only,"to":to,"state_dir":config.root})
            );
            return Ok(());
        }
        Commands::UpdateGuard {
            installation,
            profile,
            target_version,
            apply,
            remove,
            force,
            allow_active,
            original_version,
        } => {
            return agent_job_relay::online_update::guard(
                installation,
                profile,
                &target_version,
                apply,
                if remove {
                    agent_job_relay::online_update::GuardMode::Remove { force }
                } else {
                    agent_job_relay::online_update::GuardMode::Update { allow_active }
                },
                original_version.as_deref(),
            )
            .await;
        }
        Commands::Uninstall {
            root: None,
            dry_run,
            cli_only,
            force,
            ..
        } if online_remove => {
            println!(
                "{}",
                serde_json::json!({"operation":"online_uninstall","dry_run":dry_run,"cli_only":cli_only,"force":force,"state_dir":config.root})
            );
            return Ok(());
        }
        Commands::Instructions => {
            println!(
                "{}",
                serde_json::to_string(&agent_job_relay::mcp::workflow())?
            );
            return Ok(());
        }
        Commands::Remote { command } => {
            match command {
                RemoteCommand::Init => {
                    println!("{}", agent_job_relay::remote::initialize(&config)?)
                }
                RemoteCommand::Serve => agent_job_relay::remote::serve(config).await?,
            }
            return Ok(());
        }
        Commands::StartSpec {
            file,
            wake,
            wait_goal,
        } => {
            use std::io::Read;
            let reader: Box<dyn Read> = if file == std::path::Path::new("-") {
                Box::new(std::io::stdin())
            } else {
                Box::new(std::fs::File::open(file)?)
            };
            let mut bytes = Vec::new();
            reader.take(65537).read_to_end(&mut bytes)?;
            anyhow::ensure!(bytes.len() <= 65536, "specification exceeds 64 KiB");
            let spec = serde_json::from_slice(&bytes).context("input must be a JSON StartSpec")?;
            let value = if wait_goal {
                agent_job_relay::client::start_waiting(&config, vec![spec], false).await?
            } else {
                agent_job_relay::client::start(&config, spec, wake).await?
            };
            println!("{}", serde_json::to_string(&value)?);
            return Ok(());
        }
        Commands::Ui {
            job_id, no_open, ..
        } => {
            if let Some(id) = &job_id {
                agent_job_relay::core::validate_id(id)?;
            }
            let health = agent_job_relay::client::start_daemon(&config).await?;
            anyhow::ensure!(
                health["runtime_monitoring"] == true,
                "selected daemon {} does not support the Web UI; finish its active jobs, explicitly run daemon stop for this same profile, then retry ui. No daemon was restarted",
                health["version"].as_str().unwrap_or("unknown")
            );
            let info = ipc::call(&config, Request::UiInfo).await?;
            let mut url = info["access_url"]
                .as_str()
                .context(
                    "Web UI unavailable; inspect doctor for its startup error or AJR_WEB_UI=off",
                )?
                .to_owned();
            if let Some(id) = job_id {
                url.push_str(&format!("&job={id}"));
            }
            println!("{}", serde_json::json!({"url":url,"profile":config.root}));
            if !no_open && let Err(error) = agent_job_relay::platform::open_browser(&url) {
                eprintln!("Open the URL above in your browser: {error}");
            }
            return Ok(());
        }
        Commands::Data { command } => {
            use agent_job_relay::{maintenance, storage::Store};
            let value = if let DataCommand::Restore { from, to } = command {
                maintenance::restore(from, to).await?
            } else {
                // Explicit data maintenance never starts a daemon or a workload.
                config.prepare()?;
                let store = Store::new(config);
                store.init().await?;
                match command {
                    DataCommand::Status => store.data_status().await?,
                    DataCommand::Check => store.data_check().await?,
                    DataCommand::Maintain { apply } => store.maintain(apply).await?,
                    DataCommand::Backup => store.backup().await?,
                    DataCommand::Policy {
                        hot_jobs,
                        archive_after_days,
                        hot_log_mib,
                        backup_hours,
                        backup_keep,
                        apply,
                    } => {
                        let mut policy: maintenance::Policy = serde_json::from_value(
                            store.data_policy(None, false).await?["policy"].clone(),
                        )?;
                        if let Some(value) = hot_jobs {
                            policy.hot_jobs = value;
                        }
                        if let Some(value) = archive_after_days {
                            policy.archive_after_days = value;
                        }
                        if let Some(value) = hot_log_mib {
                            policy.hot_log_mib = value;
                        }
                        if let Some(value) = backup_hours {
                            policy.backup_hours = value;
                        }
                        if let Some(value) = backup_keep {
                            policy.backup_keep = value;
                        }
                        store.data_policy(Some(policy), apply).await?
                    }
                    DataCommand::Restore { .. } => unreachable!(),
                }
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(());
        }
        Commands::Connect {
            cwd,
            recover,
            apply,
            after,
            limit,
            full,
            wait_goal,
            request_id,
            request_label,
            job_count,
        } => {
            let value = if request_id.is_some() || request_label.is_some() || job_count.is_some() {
                agent_job_relay::client::connect_request(
                    &config,
                    cwd.unwrap_or(std::env::current_dir()?),
                    agent_job_relay::goal_consent::Options {
                        request_id,
                        label: request_label,
                        job_limit: job_count,
                        user_authorized: wait_goal,
                    },
                )
                .await?
            } else if wait_goal {
                agent_job_relay::client::connect_with_goal(
                    &config,
                    cwd.unwrap_or(std::env::current_dir()?),
                )
                .await?
            } else {
                agent_job_relay::client::connect_context(
                    &config,
                    cwd.unwrap_or(std::env::current_dir()?),
                    recover.then_some((apply, after, limit)),
                )
                .await?
            };
            println!(
                "{}",
                serde_json::to_string(&if full {
                    value
                } else {
                    agent_job_relay::client::brief_binding(value)
                })?
            );
            return Ok(());
        }
        Commands::GoalWait {
            goal_wait_id,
            cancel,
            confirm_paused,
            expected_revision,
        } => Request::GoalWait {
            goal_wait_id,
            cancel,
            confirm_paused,
            expected_revision,
        },
        Commands::Plugin { command } => {
            let result: serde_json::Value = async {
                use agent_job_relay::{deployment, plugin};
                let root =
                    |path: Option<PathBuf>| path.map(Ok).unwrap_or_else(deployment::plugin_root);
                match command {
                    PluginCommand::Install {
                        root: path,
                        bundle,
                        config: marketplace,
                        cache,
                        apply,
                        full,
                        ..
                    } => {
                        let marketplace = marketplace.map(Ok).unwrap_or_else(|| {
                            agent_job_relay::integration::default_path(
                                agent_job_relay::integration::Client::Plugin,
                            )
                        })?;
                        let value = plugin::install(
                            &config,
                            root(path)?,
                            bundle,
                            marketplace,
                            cache,
                            apply,
                        )
                        .await?;
                        Ok(if full {
                            value
                        } else {
                            agent_job_relay::output::installation(value)
                        })
                    }
                    PluginCommand::Repair { path, apply, .. } => {
                        agent_job_relay::lifecycle::repair(path, apply)
                    }
                    PluginCommand::Pack { output, apply, .. } => plugin::pack(output, apply),
                    PluginCommand::Bootstrap {
                        root: path,
                        config: marketplace,
                        apply,
                        ..
                    } => {
                        let marketplace = marketplace.map(Ok).unwrap_or_else(|| {
                            agent_job_relay::integration::default_path(
                                agent_job_relay::integration::Client::Plugin,
                            )
                        })?;
                        plugin::bootstrap(&config, root(path)?, marketplace, apply).await
                    }
                    PluginCommand::Update {
                        root: path,
                        bundle,
                        cache,
                        apply,
                        ..
                    } => plugin::update(root(path)?, bundle, cache, apply).await,
                    PluginCommand::Status { root: path, cache } => {
                        plugin::status(root(path)?, cache)
                    }
                    PluginCommand::Cache {
                        command:
                            CacheCommand::Refresh {
                                root: path,
                                path: cache,
                                apply,
                                ..
                            },
                    } => plugin::refresh_cache(root(path)?, cache, apply),
                }
            }
            .await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
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
            export: Some(path),
            ..
        } => {
            let result =
                agent_job_relay::client::export_logs(&config, job_id, stream, path).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Commands::Logs {
            job_id,
            stream,
            cursor,
            limit,
            follow: true,
            ..
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
                if let Some(from) = v["skipped"]["from"].as_u64() {
                    println!(
                        "\n[AgentJobRelay missing bytes {from}..{}]",
                        v["skipped"]["to"]
                    );
                }
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
            SessionCommand::Recover {
                apply,
                after,
                limit,
                ..
            } => {
                let value = agent_job_relay::client::recover(&config, apply, after, limit).await?;
                println!("{}", serde_json::to_string_pretty(&value)?);
                return Ok(());
            }
            SessionCommand::Connect { brief } => {
                let value = agent_job_relay::client::connect_context(
                    &config,
                    std::env::current_dir()?,
                    None,
                )
                .await?;
                let value = if brief {
                    agent_job_relay::client::brief_binding(value)
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
                renew_on_delivery,
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
                    renew_on_delivery,
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

        Commands::Mcp {
            shared_host,
            worker,
        } => {
            return agent_job_relay::mcp::serve_mode(config, shared_host, !worker).await;
        }
        Commands::Install { root, apply, .. } | Commands::Uninstall { root, apply, .. } => {
            let root = root
                .map(Ok)
                .unwrap_or_else(agent_job_relay::deployment::default_root)?;
            let result =
                agent_job_relay::deployment::change(&config, root, remove_install, apply).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Commands::Request { json } => serde_json::from_str(&json)?,
        #[cfg(windows)]
        Commands::WindowsBridge { command } => {
            let result = match command {
                WindowsBridgeCommand::Prepare => {
                    agent_job_relay::platform::prepare_bridge(&config)?
                }
                WindowsBridgeCommand::Secure {
                    endpoint,
                    server_pid,
                } => agent_job_relay::platform::secure_bridge_pipe(&endpoint, server_pid)?,
            };
            println!("{}", serde_json::to_string(&result)?);
            return Ok(());
        }
        #[cfg(windows)]
        Commands::WindowsBootstrap {
            endpoint,
            parent_identity,
        } => {
            return agent_job_relay::platform::bootstrap(config, endpoint, parent_identity).await;
        }
        #[cfg(windows)]
        Commands::WindowsWorkload { job_id, job_name } => {
            return agent_job_relay::runner::windows_workload(config, job_id, job_name).await;
        }
        Commands::Daemon { command: None } => return agent_job_relay::daemon::serve(config).await,
        Commands::Daemon {
            command: Some(DaemonCommand::Start { .. }),
        } => {
            let result = agent_job_relay::client::start_daemon(&config).await?;
            println!("{}", serde_json::to_string(&result)?);
            return Ok(());
        }
        Commands::Daemon {
            command: Some(DaemonCommand::Stop { .. }),
        } => Request::Shutdown,
        Commands::Runner { job_id, nonce } => {
            return agent_job_relay::runner::run(config, job_id, nonce).await;
        }
        Commands::Start {
            title,
            shell,
            interpreter,
            cwd,
            timeout,
            session,
            wake,
            wait_goal,
            idempotency_key,
            monitor,
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
                title,
                command,
                cwd,
                env: BTreeMap::new(),
                timeout_seconds: timeout,
                session,
                idempotency_key,
                monitor_rules: if monitor.no_output.is_some()
                    || monitor.elapsed.is_some()
                    || monitor.contains.is_some()
                    || monitor.progress.is_some()
                {
                    vec![rule_spec(monitor)?]
                } else {
                    anyhow::ensure!(
                        monitor.action == "alert"
                            && !monitor.notify_agent
                            && monitor.grace == 5
                            && monitor.stream == "both",
                        "monitor options require --no-output, --elapsed, --contains or --progress"
                    );
                    vec![]
                },
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&if wait_goal {
                    agent_job_relay::client::start_waiting(&config, vec![spec], false).await?
                } else {
                    agent_job_relay::client::start(&config, spec, wake).await?
                })?
            );
            return Ok(());
        }
        Commands::Status { job_id } => Request::Status { job_id },
        Commands::StartBatch {
            file,
            wake,
            wait_goal,
        } => {
            use std::io::Read;
            let reader: Box<dyn Read> = if file == std::path::Path::new("-") {
                Box::new(std::io::stdin())
            } else {
                Box::new(std::fs::File::open(file)?)
            };
            let mut bytes = Vec::new();
            reader
                .take(agent_job_relay::batch::MAX_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            anyhow::ensure!(
                bytes.len() <= agent_job_relay::batch::MAX_BYTES,
                "batch input exceeds 128 KiB"
            );
            let specs = serde_json::from_slice(&bytes)
                .context("batch input must be a JSON array of start specifications")?;
            let result = if wait_goal {
                agent_job_relay::client::start_waiting(&config, specs, true).await?
            } else {
                agent_job_relay::client::start_batch(&config, specs, wake).await?
            };
            println!("{}", serde_json::to_string_pretty(&result)?);
            return Ok(());
        }
        Commands::List { after, limit } => Request::List { after, limit },
        Commands::Logs {
            follow: false,
            job_id,
            stream,
            cursor,
            limit,
            ..
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
        Commands::Stop {
            job_id,
            grace,
            wait,
        } => Request::Stop {
            job_id,
            grace_seconds: grace,
            wait_seconds: wait,
        },
        Commands::Monitor { command } => {
            use agent_job_relay::monitor::Mutation;
            let (job_id, mutation) = match command {
                MonitorCommand::List { job_id } => (job_id, None),
                MonitorCommand::Add { job_id, rule } => (
                    job_id,
                    Some(Mutation::Add {
                        rule: rule_spec(rule)?,
                    }),
                ),
                MonitorCommand::Update {
                    job_id,
                    rule_id,
                    revision,
                    rule,
                } => (
                    job_id,
                    Some(Mutation::Update {
                        rule_id,
                        expected_revision: revision,
                        rule: rule_spec(rule)?,
                    }),
                ),
                MonitorCommand::Enable {
                    job_id,
                    rule_id,
                    revision,
                } => (
                    job_id,
                    Some(Mutation::Enable {
                        rule_id,
                        expected_revision: revision,
                        enabled: true,
                    }),
                ),
                MonitorCommand::Disable {
                    job_id,
                    rule_id,
                    revision,
                } => (
                    job_id,
                    Some(Mutation::Enable {
                        rule_id,
                        expected_revision: revision,
                        enabled: false,
                    }),
                ),
                MonitorCommand::Remove {
                    job_id,
                    rule_id,
                    revision,
                } => (
                    job_id,
                    Some(Mutation::Remove {
                        rule_id,
                        expected_revision: revision,
                    }),
                ),
            };
            Request::Monitor { job_id, mutation }
        }
        Commands::Inspect {
            job_id,
            watch,
            json,
            compact,
            before,
            limit,
        } => {
            loop {
                let value = agent_job_relay::monitor::inspect_client(
                    &config,
                    job_id.clone(),
                    before,
                    limit,
                )
                .await?;
                let value = if compact {
                    agent_job_relay::output::compact("inspect", value)
                } else {
                    value
                };
                if json {
                    println!("{}", serde_json::to_string(&value)?);
                } else {
                    use std::io::IsTerminal;
                    if watch && std::io::stdout().is_terminal() {
                        print!("\x1b[2J\x1b[H");
                    }
                    println!("{}", agent_job_relay::monitor::human(&value));
                }
                if !watch {
                    break;
                }
                tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_secs(1))=>{}}
            }
            return Ok(());
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&agent_job_relay::client::call(&config, req).await?)?
    );
    Ok(())
}

fn rule_spec(args: RuleArgs) -> Result<agent_job_relay::monitor::RuleSpec> {
    use agent_job_relay::monitor::*;
    let condition = match (args.no_output, args.elapsed, args.contains, args.progress) {
        (None, None, None, Some(raw)) => {
            let mut value: serde_json::Value =
                serde_json::from_str(&raw).context("--progress requires a JSON object")?;
            anyhow::ensure!(
                value.is_object() && value.get("kind").is_none(),
                "--progress takes source, trigger and cooldown_seconds; omit kind"
            );
            value["kind"] = serde_json::json!("progress");
            serde_json::from_value(value)?
        }
        (Some(seconds), None, None, None) => Condition::NoOutput { seconds },
        (None, Some(seconds), None, None) => Condition::Elapsed { seconds },
        (None, None, Some(text), None) => Condition::Contains {
            text,
            stream: match args.stream.as_str() {
                "stdout" => Stream::Stdout,
                "stderr" => Stream::Stderr,
                "both" => Stream::Both,
                _ => anyhow::bail!("stream must be stdout, stderr or both"),
            },
        },
        _ => {
            anyhow::bail!("provide exactly one of --no-output, --elapsed, --contains or --progress")
        }
    };
    let rule = RuleSpec {
        condition,
        action: match args.action.as_str() {
            "alert" => Action::Alert,
            "stop" => Action::Stop,
            _ => anyhow::bail!("action must be alert or stop"),
        },
        notify_agent: args.notify_agent,
        grace_seconds: args.grace,
    };
    rule.validate()?;
    Ok(rule)
}
