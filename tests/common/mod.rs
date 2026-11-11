pub fn root() -> std::path::PathBuf {
    let path = std::path::Path::new(
        &std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo test workspace"),
    )
    .join(".tmp");
    std::fs::create_dir_all(&path).expect("isolated test root");
    path
}

#[cfg(target_os = "linux")]
pub mod runtime;
