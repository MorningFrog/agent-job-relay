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
    let remove_install = matches!(&cli.command, Commands::Uninstall { .. });
    let mut state = cli.state_dir;
    if let Commands::Plugin {
        command: PluginCommand::Bootstrap { root, .. },
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
        Commands::Plugin { command } => {
            let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
                use agent_job_relay::{deployment, plugin};
                let root =
                    |path: Option<PathBuf>| path.map(Ok).unwrap_or_else(deployment::plugin_root);
                match command {
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
                        plugin::bootstrap(&config, root(path)?, marketplace, apply)
                    }
                    PluginCommand::Update {
                        root: path,
                        bundle,
                        cache,
                        apply,
                        ..
                    } => plugin::update(root(path)?, bundle, cache, apply),
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
            })
            .await??;
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
