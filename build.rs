use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn main() {
    println!("cargo:rerun-if-changed=web/dist");
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("web/dist");
    assert!(
        root.join("index.html").is_file(),
        "Build the bundled React UI first: npm ci --prefix web && npm run build --prefix web"
    );
    let mut files = vec![];
    collect(&root, &root, &mut files);
    files.sort();
    let output = format!(
        "pub const ASSETS: &[(&str,&[u8])] = &[{}];",
        files
            .iter()
            .map(|(name, path)| format!("({name:?},include_bytes!({path:?}))"))
            .collect::<Vec<_>>()
            .join(",")
    );
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("web_assets.rs"),
        output,
    )
    .unwrap();
}
fn collect(root: &Path, path: &Path, files: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        assert!(!kind.is_symlink(), "Web assets must not be symlinks");
        if kind.is_dir() {
            collect(root, &path, files);
        } else if kind.is_file() {
            files.push((
                format!(
                    "/{}",
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                ),
                path.to_string_lossy().into_owned(),
            ));
        }
    }
}
