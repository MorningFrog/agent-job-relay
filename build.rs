use std::{
    env, fs,
    path::{Path, PathBuf},
};
fn main() {
    // Clap's complete debug command tree exceeds Windows' 1-MiB main stack.
    // Reserve stack address space on demand, without changing OS/user settings.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg-bin=agent-job-relay=/STACK:8388608");
    }
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
            .map(|(name, source)| {
                // Local CI checkouts move between runs. Keep rustc's generated
                // asset dependencies in the stable target directory, so cached
                // code never refers to a checkout that the Worker has removed.
                let target = PathBuf::from(env::var("OUT_DIR").unwrap())
                    .join("web-assets")
                    .join(name.trim_start_matches('/'));
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                write_changed(&target, &fs::read(source).unwrap());
                let target = target.to_string_lossy();
                format!("({name:?},include_bytes!({target:?}))")
            })
            .collect::<Vec<_>>()
            .join(",")
    );
    write_changed(
        &PathBuf::from(env::var("OUT_DIR").unwrap()).join("web_assets.rs"),
        output.as_bytes(),
    );
}
fn write_changed(path: &Path, bytes: &[u8]) {
    if fs::read(path).ok().as_deref() != Some(bytes) {
        fs::write(path, bytes).unwrap();
    }
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
