//! Dependency-free native workload for Windows lifecycle tests; never user data.
use std::{io::{self, Write}, path::PathBuf, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("") {
        "echo" => { for arg in &args[2..] { println!("{arg}"); } eprintln!("stderr-marker"); }
        "exit" => std::process::exit(args[2].parse().unwrap()),
        "gate" | "tree" => {
            let dir = PathBuf::from(&args[2]);
            if args[1] == "tree" {
                Command::new(std::env::current_exe().unwrap()).arg("branch").arg(&dir).stdin(Stdio::null()).spawn().unwrap();
            }
            std::fs::write(dir.join("ready"), std::process::id().to_string()).unwrap();
            for _ in 0..6000 {
                if dir.join("release").exists() { println!("released"); return; }
                thread::sleep(Duration::from_millis(20));
            }
            std::process::exit(3);
        }
        "branch" => {
            let dir = PathBuf::from(&args[2]);
            std::fs::write(dir.join("branch.pid"), std::process::id().to_string()).unwrap();
            Command::new(std::env::current_exe().unwrap()).arg("leaf").arg(&dir).stdin(Stdio::null()).spawn().unwrap();
            thread::sleep(Duration::from_secs(120));
        }
        "leaf" => {
            std::fs::write(PathBuf::from(&args[2]).join("leaf.pid"), std::process::id().to_string()).unwrap();
            thread::sleep(Duration::from_secs(120));
        }
        "bytes" => { io::stdout().write_all(&vec![b'x'; 16 * 1024 * 1024 + 8192]).unwrap(); io::stderr().write_all(&[0xff, b'\n']).unwrap(); }
        "cwd" => println!("{}", std::env::current_dir().unwrap().display()),
        _ => std::process::exit(2),
    }
}
