use crate::{config::Config, core::id};
use anyhow::{Context, Result, bail, ensure};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Client {
    Codex,
    Json,
    Plugin,
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    version: u32,
    client: Client,
    config_path: PathBuf,
    entry: Value,
}
fn receipt(path: &Path) -> Result<PathBuf> {
    Ok(path.with_file_name(format!(
        ".{}.agent-job-relay.json",
        path.file_name()
            .context("config filename required")?
            .to_string_lossy()
    )))
}
fn checked_read(path: &Path) -> Result<Option<String>> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => {
            ensure!(
                m.is_file() && !m.file_type().is_symlink(),
                "refusing non-regular configuration/receipt: {}",
                path.display()
            );
            ensure!(m.len() <= 2 * 1024 * 1024, "configuration exceeds 2 MiB");
            Ok(Some(std::fs::read_to_string(path)?))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn desired(
    config: &Config,
    client: Client,
    path: &Path,
    plugin_root: Option<&Path>,
) -> Result<Value> {
    if client == Client::Plugin {
        let root = plugin_root
            .map(Path::to_path_buf)
            .map(Ok)
            .unwrap_or_else(crate::deployment::installed_root)?;
        let marketplace = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context(
                "plugin --config must be <marketplace-root>/.agents/plugins/marketplace.json",
            )?;
        ensure!(
            path.ends_with(".agents/plugins/marketplace.json"),
            "plugin --config must be <marketplace-root>/.agents/plugins/marketplace.json"
        );
        let relative = root.strip_prefix(marketplace).context("installation must be inside the marketplace root; use an installation under HOME for a personal marketplace, or a repo-local installation for a repo marketplace")?;
        let source = format!(
            "./{}",
            relative.to_str().context("plugin path is not UTF-8")?
        );
        return Ok(
            json!({"name":"agent-job-relay","source":{"source":"local","path":source},"policy":{"installation":"AVAILABLE","authentication":"ON_INSTALL"},"category":"Productivity"}),
        );
    }
    Ok(
        json!({"command":std::env::current_exe()?.canonicalize()?,"args":["--state-dir",config.root,"mcp","--shared-host"]}),
    )
}
fn edit(client: Client, raw: &str, replacement: Option<&Value>) -> Result<(String, Option<Value>)> {
    match client {
        Client::Plugin => {
            let mut doc: Value = if raw.trim().is_empty() {
                json!({"name":"agent-job-relay-local","interface":{"displayName":"AgentJobRelay Local"},"plugins":[]})
            } else {
                serde_json::from_str(raw).context("invalid marketplace JSON; nothing changed")?
            };
            ensure!(
                doc.is_object() && doc["name"].as_str().is_some_and(|s| !s.trim().is_empty()),
                "marketplace must have a name"
            );
            let plugins = doc["plugins"]
                .as_array_mut()
                .context("marketplace plugins must be an array")?;
            ensure!(
                plugins.iter().all(Value::is_object),
                "marketplace entries must be objects"
            );
            let matching: Vec<_> = plugins
                .iter()
                .enumerate()
                .filter(|(_, v)| v["name"] == "agent-job-relay")
                .map(|(i, _)| i)
                .collect();
            ensure!(
                matching.len() <= 1,
                "duplicate agent-job-relay marketplace entries; review manually"
            );
            let old = matching.first().map(|i| plugins[*i].clone());
            match (matching.first(), replacement) {
                (Some(i), Some(v)) => plugins[*i] = v.clone(),
                (Some(i), None) => {
                    plugins.remove(*i);
                }
                (None, Some(v)) => plugins.push(v.clone()),
                (None, None) => {}
            }
            Ok((format!("{}\n", serde_json::to_string_pretty(&doc)?), old))
        }
        Client::Json => {
            let mut doc: Value = if raw.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(raw).context("invalid MCP JSON; nothing changed")?
            };
            ensure!(doc.is_object(), "MCP JSON root must be an object");
            let old = doc
                .get("mcpServers")
                .and_then(|m| m.get("agent-job-relay"))
                .cloned();
            if let Some(replacement) = replacement {
                if doc.get("mcpServers").is_none() {
                    doc["mcpServers"] = json!({});
                }
                ensure!(
                    doc["mcpServers"].is_object(),
                    "mcpServers must be an object"
                );
                doc["mcpServers"]["agent-job-relay"] = replacement.clone();
            } else if let Some(m) = doc.get_mut("mcpServers") {
                let m = m.as_object_mut().context("mcpServers must be an object")?;
                m.remove("agent-job-relay");
            }
            Ok((format!("{}\n", serde_json::to_string_pretty(&doc)?), old))
        }
        Client::Codex => {
            let mut doc = raw
                .parse::<toml_edit::DocumentMut>()
                .context("invalid Codex TOML; nothing changed")?;
            let old = if let Some(item) = doc
                .get("mcp_servers")
                .and_then(|m| m.get("agent-job-relay"))
            {
                // Convert only this entry, preserving arbitrary user keys for ownership comparison.
                let mut wrapper = toml_edit::DocumentMut::new();
                wrapper["entry"] = item.clone();
                Some(toml_edit::de::from_str::<Value>(&wrapper.to_string())?["entry"].clone())
            } else {
                None
            };
            if let Some(v) = replacement {
                if !doc.contains_key("mcp_servers") {
                    doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());
                }
                let servers = doc["mcp_servers"]
                    .as_table_like_mut()
                    .context("mcp_servers must be a TOML table")?;
                let mut table = toml_edit::Table::new();
                table["command"] =
                    toml_edit::value(v["command"].as_str().context("command must be string")?);
                let mut args = toml_edit::Array::new();
                for a in v["args"].as_array().context("args must be array")? {
                    args.push(a.as_str().context("arg must be string")?);
                }
                table["args"] = toml_edit::value(args);
                servers.insert("agent-job-relay", toml_edit::Item::Table(table));
            } else if let Some(item) = doc.get_mut("mcp_servers") {
                item.as_table_like_mut()
                    .context("mcp_servers must be a table")?
                    .remove("agent-job-relay");
            }
            Ok((doc.to_string(), old))
        }
    }
}
fn atomic_write(path: &Path, raw: &[u8]) -> Result<()> {
    let tmp = path.with_file_name(format!(".ajr-{}.tmp", id()));
    let result = (|| {
        let mut f = crate::platform::private_file(&tmp)?;
        f.write_all(raw)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)?;
        #[cfg(unix)]
        std::fs::File::open(path.parent().context("config parent missing")?)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}
/// Preview by default. --apply is explicit user consent at CLI invocation.
pub fn configure(
    config: &Config,
    client: Client,
    path: PathBuf,
    remove: bool,
    apply: bool,
) -> Result<Value> {
    configure_inner(config, client, path, remove, apply, None)
}
/// Bootstrap can validate marketplace ownership before creating its installation.
pub fn configure_plugin_at(
    config: &Config,
    root: &Path,
    path: PathBuf,
    apply: bool,
) -> Result<Value> {
    configure_inner(config, Client::Plugin, path, false, apply, Some(root))
}
fn configure_inner(
    config: &Config,
    client: Client,
    path: PathBuf,
    remove: bool,
    apply: bool,
    plugin_root: Option<&Path>,
) -> Result<Value> {
    ensure!(
        path.is_absolute() && path.file_name().is_some(),
        "--config must be an absolute file path"
    );
    if !apply {
        return plan(config, client, &path, remove, false, plugin_root);
    }
    std::fs::create_dir_all(path.parent().context("config parent missing")?)?;
    let lock_path = path.with_file_name(format!(
        ".{}.agent-job-relay.lock",
        path.file_name().unwrap().to_string_lossy()
    ));
    let lock = crate::platform::private_file(&lock_path)?;
    lock.try_lock()
        .context("another setup/removal is editing this configuration")?;
    plan(config, client, &path, remove, true, plugin_root)
}
fn plan(
    config: &Config,
    client: Client,
    path: &Path,
    remove: bool,
    apply: bool,
    plugin_root: Option<&Path>,
) -> Result<Value> {
    let receipt_path = receipt(path)?;
    let existing = checked_read(path)?;
    let raw = existing.as_deref().unwrap_or("");
    let receipt = checked_read(&receipt_path)?
        .map(|s| serde_json::from_str::<Receipt>(&s))
        .transpose()
        .context("invalid ownership receipt; inspect it before changing configuration")?;
    if let Some(r) = &receipt {
        ensure!(
            r.version == 1 && r.client == client && r.config_path == path,
            "ownership receipt does not match this client/config path"
        );
    }
    // Removal does not depend on the binary or original plugin still existing.
    let desired = if remove {
        Value::Null
    } else {
        desired(config, client, path, plugin_root)?
    };
    let replacement = if remove { None } else { Some(&desired) };
    let (output, current) = edit(client, raw, replacement)?;
    if let Some(current) = &current {
        let Some(r) = &receipt else {
            bail!(
                "agent-job-relay entry exists without our ownership receipt; refusing to overwrite/remove user configuration"
            );
        };
        ensure!(
            current == &r.entry,
            "managed entry has user modifications; refusing to overwrite/remove it (other MCP entries are untouched)"
        );
    }
    let changed = if remove {
        current.is_some()
    } else {
        current.as_ref() != Some(&desired)
    };
    let mut backup = None;
    if apply && changed {
        if let Some(raw) = &existing {
            let b = path.with_file_name(format!(
                "{}.agent-job-relay.{}.bak",
                path.file_name().unwrap().to_string_lossy(),
                id()
            ));
            atomic_write(&b, raw.as_bytes())?;
            backup = Some(b);
        }
        if !remove {
            let r = Receipt {
                version: 1,
                client,
                config_path: path.into(),
                entry: desired.clone(),
            };
            atomic_write(&receipt_path, &serde_json::to_vec_pretty(&r)?)?;
        }
        atomic_write(path, output.as_bytes())?;
        if remove && receipt.is_some() {
            std::fs::remove_file(receipt_path)?;
        }
    }
    Ok(
        json!({"client":client,"config_path":path,"mode":if apply{"apply"}else{"dry_run"},"changed":changed,"action":if remove{"remove owned integration entry"}else{"set owned integration entry"},"managed_entry":if remove{Value::Null}else{desired},"backup":backup,"data":"preserved; no service/PATH changes","next_step":if apply{if client==Client::Plugin{"Restart Desktop; find AgentJobRelay in the local marketplace, install it, then open a new local chat. Removing this entry does not remove host-managed cache copies; uninstall the plugin in the host as well."}else{"Restart the MCP client. For wake in shared-host mode use the current terminal binding described by server instructions; the plugin automates this workflow."}}else{"Review this preview; repeat with --apply to write. Backups are made before modifying an existing file."}}),
    )
}
pub fn default_path(client: Client) -> Result<PathBuf> {
    match client {
        Client::Codex => {
            let root = std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex")))
                .context("HOME/CODEX_HOME unavailable; pass --config")?;
            Ok(root.join("config.toml"))
        }
        Client::Json => {
            bail!("JSON clients use different paths; provide --config /absolute/path/to/mcp.json")
        }
        Client::Plugin => Ok(PathBuf::from(
            std::env::var_os("HOME").context("HOME unavailable; pass --config")?,
        )
        .join(".agents/plugins/marketplace.json")),
    }
}
