//! Offline plugin payloads, profile-preserving updates and explicit cache snapshots.
use crate::{
    config::Config,
    core::id,
    deployment::{self as deploy, Artifact, PROFILE, RECEIPT, RuntimeProfile},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::{Path, PathBuf},
};

const BUNDLE: &str = "bundle.json";
const PAYLOAD: &[&str] = &[
    ".codex-plugin/plugin.json",
    ".mcp.json",
    crate::platform::LAUNCHER_FILE,
    "skills/background-jobs/SKILL.md",
    "assets/icon.svg",
    "LICENSE",
    crate::platform::NATIVE_FILE,
];
#[cfg(not(windows))]
const BOOTSTRAP: &str = "#!/bin/sh\nset -eu\nbundle_root=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nexec \"$bundle_root/payload/bin/agent-job-relay\" \"$@\"\n";

#[cfg(windows)]
const BOOTSTRAP: &str = "@echo off\r\nsetlocal DisableDelayedExpansion\r\n\"%~dp0payload\\bin\\agent-job-relay.exe\" %*\r\nexit /b %errorlevel%\r\n";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    format: u32,
    product: String,
    version: String,
    platform: String,
    files: BTreeMap<String, String>,
}

pub(crate) fn version(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value
        .split('.')
        .map(str::parse::<u64>)
        .collect::<std::result::Result<_, _>>()?;
    parts
        .try_into()
        .map_err(|_| anyhow::anyhow!("bundle version must have three numeric components"))
}
fn payload_version(files: &BTreeMap<String, Artifact>) -> Result<String> {
    let manifest: Value = serde_json::from_slice(&files[".codex-plugin/plugin.json"].bytes)?;
    let value = manifest["version"]
        .as_str()
        .context("plugin version missing")?;
    version(value)?;
    ensure!(
        manifest["name"] == "agent-job-relay",
        "unexpected plugin identity"
    );
    Ok(value.into())
}
fn hashes(files: &BTreeMap<String, Artifact>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(name, artifact)| (name.clone(), deploy::digest(&artifact.bytes)))
        .collect()
}
fn write_tree(root: &Path, files: &BTreeMap<String, Artifact>) -> Result<()> {
    deploy::checked_path(root)?;
    crate::platform::secure_dir(root)?;
    for (name, artifact) in files {
        deploy::write_artifact(&root.join(name), artifact)?;
    }
    crate::platform::sync_directory(root)?;
    Ok(())
}
pub(crate) fn verify_tree(root: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    let mut expected: BTreeSet<String> = files.keys().cloned().collect();
    for name in files.keys() {
        let mut parent = Path::new(name).parent();
        while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
            expected.insert(format!(
                "{}/",
                path.to_str().context("invalid bundle/cache path")?
            ));
            parent = path.parent();
        }
    }
    let mut actual = BTreeSet::new();
    deploy::actual_files(root, root, &mut actual)?;
    ensure!(
        actual == expected,
        "bundle/cache has missing or unmanaged content; nothing was changed"
    );
    for (name, hash) in files {
        ensure!(
            deploy::digest(&deploy::read_file(&root.join(name))?) == *hash,
            "bundle/cache file was modified: {name}; nothing was changed"
        );
    }
    Ok(())
}

/// Export contains templates and native bytes only, never the author's state/profile.
pub fn pack(output: PathBuf, apply: bool) -> Result<Value> {
    crate::platform::require_runtime()?;
    deploy::checked_path(&output)?;
    ensure!(
        !output.exists(),
        "bundle output already exists; choose a new directory"
    );
    let mut files: BTreeMap<String, Artifact> = deploy::payload()?
        .into_iter()
        .map(|(name, file)| (format!("payload/{name}"), file))
        .collect();
    files.insert(
        crate::platform::LAUNCHER_FILE.into(),
        Artifact {
            bytes: BOOTSTRAP.as_bytes().into(),
            executable: true,
        },
    );
    let bundle = Bundle {
        format: 1,
        product: "agent-job-relay".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: crate::platform::runtime_target(),
        files: hashes(&files),
    };
    files.insert(
        BUNDLE.into(),
        Artifact {
            bytes: serde_json::to_vec_pretty(&bundle)?,
            executable: false,
        },
    );
    if apply {
        let parent = output.parent().context("bundle parent missing")?;
        std::fs::create_dir_all(parent)?;
        let lock = path_lock(&output, "bundle")?;
        ensure!(
            !output.exists(),
            "bundle output appeared; retry with a new directory"
        );
        let stage = parent.join(format!(".ajr-bundle-{}", id()));
        let result = (|| -> Result<()> {
            write_tree(&stage, &files)?;
            crate::platform::rename_directory(&stage, &output)?;
            crate::platform::sync_directory(parent)?;
            Ok(())
        })();
        if stage.exists() {
            let _ = std::fs::remove_dir_all(stage);
        }
        result?;
        drop(lock);
    }
    Ok(
        json!({"mode":mode(apply),"output":output,"version":bundle.version,"platform":bundle.platform,
        "contains":"native executable and embedded plugin resources; no user state or credentials",
        "next_step":"Transfer the directory, then run its relay plugin bootstrap --dry-run / --apply. No Rust compiler is needed on a matching supported platform.",
        "publication":"none; local development artifact only"}),
    )
}

fn read_bundle(root: &Path) -> Result<(Bundle, BTreeMap<String, Artifact>)> {
    deploy::checked_path(root)?;
    let raw = deploy::read_file(&root.join(BUNDLE))?;
    let bundle: Bundle = serde_json::from_slice(&raw).context("invalid plugin bundle manifest")?;
    ensure!(
        bundle.format == 1 && bundle.product == "agent-job-relay",
        "unsupported plugin bundle"
    );
    ensure!(
        bundle.platform == crate::platform::runtime_target(),
        "bundle platform {} does not match {}",
        bundle.platform,
        crate::platform::runtime_target()
    );
    version(&bundle.version)?;
    let expected: BTreeSet<String> = PAYLOAD
        .iter()
        .map(|n| format!("payload/{n}"))
        .chain(std::iter::once(crate::platform::LAUNCHER_FILE.into()))
        .collect();
    ensure!(
        bundle.files.keys().cloned().collect::<BTreeSet<_>>() == expected,
        "unsupported or unsafe bundle file layout"
    );
    let mut all = bundle.files.clone();
    all.insert(BUNDLE.into(), deploy::digest(&raw));
    verify_tree(root, &all)?;
    let files = PAYLOAD
        .iter()
        .map(|name| {
            Ok((
                (*name).into(),
                Artifact {
                    bytes: deploy::read_file(&root.join("payload").join(name))?,
                    executable: matches!(
                        *name,
                        crate::platform::LAUNCHER_FILE | crate::platform::NATIVE_FILE
                    ),
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    // Verify the exact bytes retained for installation, including after concurrent edits.
    for (name, artifact) in &files {
        ensure!(
            deploy::digest(&artifact.bytes) == bundle.files[&format!("payload/{name}")],
            "bundle changed while reading; retry"
        );
    }
    ensure!(
        payload_version(&files)? == bundle.version,
        "bundle/plugin versions disagree"
    );
    Ok((bundle, files))
}

fn installed_version(root: &Path) -> Result<String> {
    let name = if root.join(".codex-plugin/plugin.json").is_file() {
        ".codex-plugin/plugin.json"
    } else {
        "plugin.json"
    };
    let identity: Value = serde_json::from_slice(&deploy::read_file(&root.join(name))?)?;
    Ok(identity["version"]
        .as_str()
        .context("installed version unavailable")?
        .into())
}
fn check_update(root: &Path, new_version: &str) -> Result<()> {
    deploy::checked_path(root)?;
    if root.exists() {
        let receipt = deploy::read_receipt(root)?;
        deploy::verify_owned(root, &receipt)?;
        ensure!(
            version(new_version)? >= version(&installed_version(root)?)?,
            "refusing a runtime downgrade; use a compatible newer bundle"
        );
    }
    Ok(())
}
pub(crate) fn guard_downgrade(root: &Path, files: &BTreeMap<String, Artifact>) -> Result<()> {
    ensure!(
        version(&payload_version(files)?)? >= version(&installed_version(root)?)?,
        "refusing a runtime downgrade; use a compatible newer bundle"
    );
    Ok(())
}

/// Run from the package, install its validated bytes, and register the local marketplace.
pub async fn bootstrap(
    config: &Config,
    root: PathBuf,
    marketplace: PathBuf,
    apply: bool,
) -> Result<Value> {
    install(config, root, None, marketplace, Vec::new(), apply).await
}

/// One preview/apply entry for first installation and profile-preserving update.
pub async fn install(
    config: &Config,
    root: PathBuf,
    bundle_path: Option<PathBuf>,
    marketplace: PathBuf,
    caches: Vec<PathBuf>,
    apply: bool,
) -> Result<Value> {
    crate::platform::require_runtime()?;
    let exe = std::env::current_exe()?;
    let own_bundle = bundle_path.is_none();
    let bundle_root = match bundle_path {
        Some(path) => path,
        None => exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .context("run plugin install from an exported bundle, or pass --from <bundle>")?
            .to_path_buf(),
    };
    let (bundle, files) = read_bundle(&bundle_root)
        .context("run plugin install from an intact exported bundle, or pass --from <bundle>")?;
    let config = Config {
        root: deploy::installation_state(&root)?.unwrap_or_else(|| config.root.clone()),
    };
    deploy::checked_path(&marketplace)?;
    for destination in [&root, &config.root, &marketplace] {
        ensure!(
            !destination.starts_with(&bundle_root) && !bundle_root.starts_with(destination),
            "bundle and installation/state/marketplace destinations must be separate, not nested"
        );
    }
    ensure!(
        !marketplace.starts_with(&root),
        "marketplace configuration must remain outside the installation"
    );
    ensure!(
        !own_bundle
            || deploy::digest(&deploy::read_file(&exe)?)
                == bundle.files[&format!("payload/{}", crate::platform::NATIVE_FILE)],
        "bootstrap image differs from the bundle"
    );
    deploy_bundle(
        &config,
        root,
        bundle,
        files,
        caches,
        Some(marketplace),
        apply,
    )
    .await
}

fn mode(apply: bool) -> &'static str {
    if apply { "apply" } else { "dry_run" }
}
fn path_lock(root: &Path, operation: &str) -> Result<File> {
    deploy::checked_path(root)?;
    let parent = root.parent().context("lock parent missing")?;
    let lock = crate::platform::private_file(&parent.join(format!(
        ".{}.{}.lock",
        root.file_name().unwrap().to_string_lossy(),
        operation
    )))?;
    lock.try_lock()
        .context("another plugin lifecycle operation is in progress")?;
    Ok(lock)
}
fn separate_cache(root: &Path, state: &Path, cache: &Path) -> Result<()> {
    deploy::checked_path(cache)?;
    for protected in [root, state] {
        ensure!(
            !cache.starts_with(protected) && !protected.starts_with(cache),
            "cache, install and state directories must be separate, not nested"
        );
    }
    Ok(())
}
fn cache_profile(cache: &Path, root: &Path, state: &Path) -> Result<RuntimeProfile> {
    separate_cache(root, state, cache)?;
    crate::lifecycle::require_clear(cache)?;
    let raw = deploy::read_file(&cache.join(PROFILE)).context(
        "cache lacks a managed runtime profile; refresh this legacy cache through the host first",
    )?;
    let profile: RuntimeProfile = serde_json::from_slice(&raw)?;
    ensure!(
        profile.format == 1
            && profile.install_root == root
            && profile.state_dir == state
            && profile.platform == crate::platform::runtime_target(),
        "cache belongs to a different installation, state or platform"
    );
    ensure!(
        profile
            .files
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            == PAYLOAD.iter().copied().collect(),
        "unsupported or unsafe cache file layout"
    );
    version(&profile.version)?;
    let mut files = profile.files.clone();
    files.insert(PROFILE.into(), deploy::digest(&raw));
    if cache.join(RECEIPT).try_exists()? {
        let receipt = deploy::read_receipt(cache)?;
        ensure!(
            receipt.root == root && receipt.state_dir == state && receipt.files == files,
            "cached ownership receipt differs from its runtime profile"
        );
        files.insert(
            RECEIPT.into(),
            deploy::digest(&deploy::read_file(&cache.join(RECEIPT))?),
        );
    }
    verify_tree(cache, &files)?;
    Ok(profile)
}

/// Only caller-specified cache directories are inspected; no global host-cache scan.
pub fn status(root: PathBuf, caches: Vec<PathBuf>) -> Result<Value> {
    crate::platform::require_runtime()?;
    deploy::checked_path(&root)?;
    crate::lifecycle::require_clear(&root)?;
    let receipt = deploy::read_receipt(&root)?;
    deploy::verify_owned(&root, &receipt)?;
    let installed = installed_version(&root)?;
    let mut reports = Vec::new();
    for cache in caches {
        let profile = cache_profile(&cache, &root, &receipt.state_dir)?;
        let current = receipt.files.get(PROFILE).is_some_and(|hash| {
            deploy::read_file(&cache.join(PROFILE)).is_ok_and(|raw| deploy::digest(&raw) == *hash)
        });
        reports.push(json!({"path":cache,"version":profile.version,"matches_installed":current}));
    }
    Ok(
        json!({"root":root,"version":installed,"state_dir":receipt.state_dir,"caches":reports,
        "cache_discovery":"explicit paths only; host enabled state is not changed","host_connection":"not checked"}),
    )
}

/// Refresh a checked snapshot; no daemon lock/stop is needed because both entry
/// points route to the managed runtime, not the cache's historical native copy.
pub fn refresh_cache(root: PathBuf, cache: PathBuf, apply: bool) -> Result<Value> {
    crate::platform::require_runtime()?;
    deploy::checked_path(&root)?;
    crate::lifecycle::require_clear(&root)?;
    crate::lifecycle::require_clear(&cache)?;
    let initial = deploy::read_receipt(&root)?;
    // Validate before creating any lock file, especially for protected paths.
    cache_profile(&cache, &root, &initial.state_dir)?;
    let install_lock = if apply {
        Some(path_lock(&root, "install")?)
    } else {
        None
    };
    let cache_lock = if apply {
        Some(path_lock(&cache, "cache")?)
    } else {
        None
    };
    let receipt = deploy::read_receipt(&root)?;
    deploy::verify_owned(&root, &receipt)?;
    let old = cache_profile(&cache, &root, &receipt.state_dir)?;
    let mut files = BTreeMap::new();
    for name in receipt.files.keys() {
        files.insert(
            name.clone(),
            Artifact {
                bytes: deploy::read_file(&root.join(name))?,
                executable: matches!(
                    name.as_str(),
                    crate::platform::LAUNCHER_FILE | crate::platform::NATIVE_FILE
                ),
            },
        );
    }
    let new: RuntimeProfile = serde_json::from_slice(
        &files
            .get(PROFILE)
            .context("installed runtime profile missing; update the local installation first")?
            .bytes,
    )?;
    let changed = hashes(&files).get(PROFILE)
        != Some(&deploy::digest(&deploy::read_file(&cache.join(PROFILE))?));
    let mut backup = None;
    if apply && changed {
        backup = crate::lifecycle::replace(
            &cache,
            &root,
            &receipt.state_dir,
            crate::lifecycle::Kind::Cache,
            &files,
        )?;
    }
    drop((install_lock, cache_lock));
    Ok(
        json!({"mode":mode(apply),"path":cache,"changed":changed,"old_version":old.version,"version":new.version,"backup":backup,
        "data":"preserved","next_step":"Reconnect/restart the host's MCP connection to load the updated program and skill. Host installation/enabled state is unchanged."}),
    )
}

pub async fn update(
    root: PathBuf,
    bundle_path: PathBuf,
    caches: Vec<PathBuf>,
    apply: bool,
) -> Result<Value> {
    crate::platform::require_runtime()?;
    let state = deploy::installation_state(&root)?.context(
        "plugin update requires an existing owned installation; bootstrap the bundle first",
    )?;
    let config = Config { root: state };
    let (bundle, files) = read_bundle(&bundle_path)?;
    deploy_bundle(&config, root, bundle, files, caches, None, apply).await
}

async fn deploy_bundle(
    config: &Config,
    root: PathBuf,
    bundle: Bundle,
    files: BTreeMap<String, Artifact>,
    caches: Vec<PathBuf>,
    marketplace: Option<PathBuf>,
    apply: bool,
) -> Result<Value> {
    check_update(&root, &bundle.version)?;
    let files = deploy::render(config, &root, files)?;
    // Preflight every selected destination before replacing any directory.
    let registration = marketplace
        .as_ref()
        .map(|path| crate::integration::configure_plugin_at(config, &root, path.clone(), false))
        .transpose()?;
    for (i, cache) in caches.iter().enumerate() {
        cache_profile(cache, &root, &config.root)?;
        for other in &caches[..i] {
            ensure!(
                !cache.starts_with(other) && !other.starts_with(cache),
                "duplicate or overlapping cache paths"
            );
        }
    }
    let mut preview_caches = Vec::new();
    if !apply {
        let new_profile: RuntimeProfile = serde_json::from_slice(&files[PROFILE].bytes)?;
        for cache in &caches {
            let old = cache_profile(cache, &root, &config.root)?;
            preview_caches.push(json!({"mode":mode(false),"path":cache,"changed":deploy::digest(&deploy::read_file(&cache.join(PROFILE))?) != deploy::digest(&files[PROFILE].bytes),"old_version":old.version,"version":new_profile.version}));
        }
    }
    let installation = deploy::change_artifacts(config, root.clone(), false, apply, files).await?;
    if !apply {
        let mut value = json!({"mode":mode(false),"version":bundle.version,"install":installation,"caches":preview_caches,"data":"preserved","configuration":"unchanged",
            "next_step":"Review the preview, then repeat with --apply. Finish active jobs first; apply restarts an online daemon from the updated installation."});
        if let Some(registration) = registration {
            value["marketplace"] = registration;
            value["configuration"] = json!("previewed");
        }
        return Ok(value);
    }
    let registration = marketplace.map(|path| crate::integration::configure_plugin_at(config,&root,path,true)
        .context("runtime installation completed; marketplace registration failed; inspect ownership and retry plugin install (no automatic rollback)")).transpose()?;
    let mut refreshed = Vec::new();
    for cache in caches {
        refreshed.push(refresh_cache(root.clone(), cache, apply)
            .context("runtime update may have completed; cache refresh failed; inspect plugin status and retry cache refresh (no automatic rollback)")?);
    }
    let mut value = json!({"mode":mode(apply),"version":bundle.version,"management":{"program":root.join(if cfg!(windows){"relay.cmd"}else{"relay"}),"args":["ui"]},"install":installation,"caches":refreshed,
        "data":"preserved","configuration":"unchanged","next_step":"Refresh/enable AgentJobRelay in the host, then reconnect MCP. Changed runtime updates restart an online daemon; an offline profile starts on demand."});
    if let Some(registration) = registration {
        value["marketplace"] = registration;
        value["configuration"] = json!("managed marketplace entry");
    }
    Ok(value)
}
