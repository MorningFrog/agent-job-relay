pub fn root() -> std::path::PathBuf {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".tmp");
    std::fs::create_dir_all(&path).expect("isolated test root");
    path
}
