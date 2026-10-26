//! Minimal pre-shutdown IPC peer for upgrade regression tests; never shipped.
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
static STOP: AtomicBool = AtomicBool::new(false);
unsafe extern "C" {
    fn signal(number: i32, handler: extern "C" fn(i32)) -> usize;
}
extern "C" fn interrupt(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let root = PathBuf::from(&args[args.iter().position(|v| v == "--state-dir").unwrap() + 1]);
    let lock = File::create(root.join("daemon.lock")).unwrap();
    lock.try_lock().unwrap();
    unsafe {
        signal(2, interrupt);
    }
    let listener = UnixListener::bind(root.join("relay.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();
    while !STOP.load(Ordering::Relaxed) {
        let mut stream = match listener.accept() {
            Ok((s, _)) => s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(e) => panic!("{e}"),
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut length = [0; 4];
        if stream.read_exact(&mut length).is_err() {
            continue;
        }
        let n = u32::from_le_bytes(length) as usize;
        assert!(n < 131072);
        let mut body = vec![0; n];
        stream.read_exact(&mut body).unwrap();
        let body = String::from_utf8(body).unwrap();
        let response = if body.contains("\"doctor\"") {
            r#"{"protocol_version":1,"ok":true,"result":{"daemon":"reachable","version":"0.1.0","store":"ok","runtime":"linux","active_jobs":0}}"#
        } else if root.join("reject").exists() {
            r#"{"protocol_version":1,"ok":false,"error":"shutdown permission denied"}"#
        } else {
            r#"{"protocol_version":1,"ok":false,"error":"unknown variant `shutdown`, expected one of `start`, `doctor`"}"#
        };
        stream
            .write_all(&(response.len() as u32).to_le_bytes())
            .unwrap();
        stream.write_all(response.as_bytes()).unwrap();
    }
    // Exercise the client's wait for actual exit, not just the signal/ack.
    std::thread::sleep(Duration::from_millis(100));
    std::fs::remove_file(root.join("relay.sock")).unwrap();
}
