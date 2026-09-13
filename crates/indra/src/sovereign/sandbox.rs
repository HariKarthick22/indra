use anyhow::{bail, Result};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SandboxOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput>;
}

pub struct LinuxNamespace;
pub struct DockerNoNetwork;

impl SandboxBackend for LinuxNamespace {
    fn name(&self) -> &str {
        "unshare-no-net"
    }

    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput> {
        let secs = timeout.as_secs().max(1).to_string();
        run_capturing(
            Command::new("unshare")
                .args([
                    "-n",
                    "-p",
                    "-f",
                    "--mount-proc",
                    "timeout",
                    &secs,
                    "python3",
                    "-c",
                    code,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
    }
}

impl SandboxBackend for DockerNoNetwork {
    fn name(&self) -> &str {
        "docker-network-none"
    }

    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput> {
        let secs = timeout.as_secs().max(1).to_string();
        run_capturing(
            Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "--network=none",
                    "--memory=512m",
                    "--cpus=1",
                    "python:3.11-slim",
                    "timeout",
                    &secs,
                    "python",
                    "-c",
                    code,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
    }
}

fn run_capturing(cmd: &mut Command) -> Result<SandboxOutput> {
    let out = cmd.output()?;
    let code = out.status.code().unwrap_or(-1);
    Ok(SandboxOutput {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        exit_code: code,
        // GNU timeout reports 124 when it kills the child.
        timed_out: code == 124,
    })
}

fn available(bin: &str, probe_args: &[&str]) -> bool {
    Command::new(bin)
        .args(probe_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn detect_backend() -> Result<Box<dyn SandboxBackend>> {
    if cfg!(target_os = "linux") && available("unshare", &["--version"]) {
        return Ok(Box::new(LinuxNamespace));
    }
    if available("docker", &["info"]) {
        return Ok(Box::new(DockerNoNetwork));
    }
    bail!("no isolated sandbox available; refusing to execute code unisolated")
}
