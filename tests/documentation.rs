use std::path::Path;
#[test]
fn readmes_have_bilingual_introductions_and_valid_local_links() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["README.md", "README.zh-CN.md"] {
        let text = std::fs::read_to_string(root.join(name)).unwrap();
        let intro = text.lines().take(9).collect::<Vec<_>>().join("\n");
        assert!(intro.contains("Run durable background commands"));
        assert!(intro.contains("持久执行后台命令"));
        assert!(intro.contains("[English](README.md)"));
        assert!(intro.contains("[简体中文](README.zh-CN.md)"));
        for part in text.split("](").skip(1) {
            let target = part.split(')').next().unwrap();
            if target.starts_with("https://") || target.starts_with('#') {
                continue;
            }
            let target = target.split('#').next().unwrap();
            assert!(
                root.join(target).is_file(),
                "broken link in {name}: {target}"
            );
        }
        assert!(text.contains("--dry-run"));
        assert!(text.contains("integration remove"));
        assert!(text.contains("accepted_by_host"));
        assert!(text.contains("docs/TODO.md"));
    }
}
