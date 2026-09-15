use crate::sovereign::sandbox::SandboxBackend;
use anyhow::{bail, Result};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Agrees,
    Disagrees { computed: f64 },
}

pub fn verify_calculation(
    backend: &dyn SandboxBackend,
    python: &str,
    claimed: f64,
    tolerance: f64,
) -> Result<Verdict> {
    let out = backend.run(python, Duration::from_secs(20))?;
    if out.exit_code != 0 {
        bail!("calculation failed to execute: {}", out.stderr);
    }

    let last = out
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();

    let computed: f64 = last.trim().parse()?;

    if (computed - claimed).abs() <= tolerance {
        Ok(Verdict::Agrees)
    } else {
        Ok(Verdict::Disagrees { computed })
    }
}
