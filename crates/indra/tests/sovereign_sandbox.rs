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

// unshare -n isolates the network only; without a memory ceiling a bomb like
// this would run unbounded until the timeout, risking host OOM. This proves
// the cap is real, not just documented: an allocation past the ceiling must
// be killed or error out, never quietly succeed.
//
// A zero-filled bytearray is not a valid probe here: the kernel can satisfy
// zero-filled anonymous memory via a shared zero page without committing any
// real physical pages, so cgroup/ulimit accounting never sees the requested
// size and the "allocation" silently costs nothing. Writing one distinct byte
// per page forces genuine commitment of every page touched.
#[test]
fn sandbox_rejects_a_memory_bomb() {
    let Ok(backend) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let code = r#"
x = bytearray(2 * 1024 * 1024 * 1024)
for i in range(0, len(x), 4096):
    x[i] = 1
print(len(x))
"#;
    let out = backend
        .run(code, Duration::from_secs(20))
        .expect("sandbox invocation itself should not error");

    assert_ne!(
        out.exit_code, 0,
        "a 2GB allocation with every page touched succeeded — no memory ceiling is enforced: {}",
        out.stdout
    );
}
