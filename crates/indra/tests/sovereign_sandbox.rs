use indra::sovereign::sandbox::detect_backend;
use std::time::Duration;

#[test]
fn sandbox_runs_trivial_code() {
    let Ok(backend) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let out = backend
        .run("print(2 + 2)", Duration::from_secs(30))
        .unwrap();
    assert_eq!(out.stdout.trim(), "4");
    assert_eq!(out.exit_code, 0);
}

#[test]
fn sandbox_cannot_reach_the_network() {
    let Ok(backend) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let code = r#"
import socket
try:
    socket.create_connection(("1.1.1.1", 53), timeout=5)
    print("REACHED")
except Exception:
    print("BLOCKED")
"#;
    let out = backend.run(code, Duration::from_secs(30)).unwrap();
    assert!(
        out.stdout.contains("BLOCKED"),
        "sandbox reached the network — A5 is violated: {}",
        out.stdout
    );
}

#[test]
fn sandbox_enforces_timeout() {
    let Ok(backend) = detect_backend() else {
        return;
    };
    let out = backend
        .run("import time; time.sleep(60)", Duration::from_secs(2))
        .unwrap();
    assert!(out.timed_out, "long-running code was not terminated");
}
