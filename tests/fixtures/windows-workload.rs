//! Dependency-free native workload for Windows lifecycle tests; never user data.
use std::{io::{self, Write}, path::PathBuf, process::{Command, Stdio}, thread, time::Duration};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("") {
        "host-start" | "host-nested-start" => restricted_host(&args),
        "echo" => { for arg in &args[2..] { println!("{arg}"); } eprintln!("stderr-marker"); }
        "env" => { for key in &args[2..] { println!("{key}={}", std::env::var(key).unwrap_or_default()); } }
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

#[cfg(windows)]
fn restricted_host(args: &[String]) {
    use std::{ffi::c_void, ptr::null_mut};
    #[repr(C)]
    #[derive(Default)]
    struct Basic { process_time: i64, job_time: i64, flags: u32, min_ws: usize, max_ws: usize, active: u32, affinity: usize, priority: u32, scheduling: u32 }
    #[repr(C)]
    #[derive(Default)]
    struct Limits { basic: Basic, io: [u64; 6], process_mem: usize, job_mem: usize, peak_process: usize, peak_job: usize }
    #[link(name="kernel32")]
    unsafe extern "system" {
        fn CreateJobObjectW(attributes: *const c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(job: *mut c_void, class: i32, data: *const c_void, length: u32) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
        fn GetCurrentProcess() -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    let job = unsafe { CreateJobObjectW(null_mut(), null_mut()) };assert!(!job.is_null());
    let mut limits = Limits::default();limits.basic.flags=0x2000; // KILL_ON_JOB_CLOSE, no breakaway.
    unsafe {
        assert_ne!(SetInformationJobObject(job, 9, (&limits as *const Limits).cast(), std::mem::size_of_val(&limits) as u32),0);
        assert_ne!(AssignProcessToJobObject(job, GetCurrentProcess()),0);
    }
    // BREAKAWAY may leave this inner job while retaining the restrictive outer
    // job. CreateProcess success alone does not establish process independence.
    let inner = if args[1] == "host-nested-start" {
        let inner = unsafe { CreateJobObjectW(null_mut(), null_mut()) };assert!(!inner.is_null());
        limits.basic.flags=0x2800; // KILL_ON_JOB_CLOSE | BREAKAWAY_OK.
        unsafe {
            assert_ne!(SetInformationJobObject(inner, 9, (&limits as *const Limits).cast(), std::mem::size_of_val(&limits) as u32),0);
            assert_ne!(AssignProcessToJobObject(inner, GetCurrentProcess()),0);
        }
        Some(inner)
    } else { None };
    let dir = PathBuf::from(&args[4]);
    let output = Command::new(&args[2]).arg("--state-dir").arg(&args[3]).args(["start", "--cwd"]).arg(&dir)
        .arg("--").arg(std::env::current_exe().unwrap()).arg("gate").arg(&dir)
        .stdin(Stdio::null()).output().unwrap();
    std::fs::write(dir.join("host.stderr"), output.stderr).unwrap();
    std::fs::write(dir.join("host.stdout.tmp"), output.stdout).unwrap();
    std::fs::write(dir.join("host.exit"), output.status.code().unwrap_or(-1).to_string()).unwrap();
    std::fs::rename(dir.join("host.stdout.tmp"), dir.join("host.stdout.json")).unwrap();
    for _ in 0..1500 {
        if dir.join("close-host").exists() { break; }
        thread::sleep(Duration::from_millis(20));
    }
    unsafe {
        if let Some(inner) = inner { CloseHandle(inner); }
        CloseHandle(job);
    } // Terminates the host and every child still in either cleanup job.
    unreachable!();
}
