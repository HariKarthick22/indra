use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    Llamacpp,
    Mlx,
    /// Any format the native backends cannot load, served by a local runtime
    /// (vLLM, Ollama, TGI) over an OpenAI-compatible API. Loopback only.
    LocalHttp {
        endpoint: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCalling {
    Reliable,
    Unreliable,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Domain {
    Code,
    Document,
    Engineering,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub vision: bool,
    pub tool_calling: ToolCalling,
    pub context_tokens: u32,
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub backend: Backend,
    pub path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub capabilities: Capabilities,
    pub min_vram_mb: u64,
}

#[derive(Debug, Clone)]
pub struct Requirements {
    pub vision: bool,
    pub tool_calling: ToolCalling,
    pub domain: Domain,
    pub min_context_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    Comfortable,
    Tight { warning: String },
    Degraded { warning: String },
}

#[derive(Debug, Clone)]
pub struct Selection<'a> {
    pub descriptor: &'a ModelDescriptor,
    pub fit: Fit,
    pub reason: String,
}

/// Capability gaps are disqualifying; hardware shortfalls are not. A model too
/// large for available memory still runs via CPU offload, so it is selected with
/// a warning rather than hidden — otherwise a venue machine shows an empty list.
fn assess(m: &ModelDescriptor, hw: &crate::sovereign::hardware::HardwareProfile) -> Fit {
    // GPU VRAM is the constrained resource callers need to see in the warning;
    // total RAM only stands in for it on a CPU-only machine (no GPU at all).
    let available = if hw.gpu_vram_mb > 0 {
        hw.gpu_vram_mb
    } else {
        hw.total_ram_mb
    };

    if m.min_vram_mb == 0 || m.min_vram_mb * 100 <= available * 80 {
        return Fit::Comfortable;
    }
    if m.min_vram_mb <= available {
        return Fit::Tight {
            warning: format!(
                "{} needs {} MB, {} MB available — little headroom, expect slowdowns",
                m.id, m.min_vram_mb, available
            ),
        };
    }
    Fit::Degraded {
        warning: format!(
            "{} needs {} MB, {} MB available — offloading to CPU, expect substantially \
             slower generation",
            m.id, m.min_vram_mb, available
        ),
    }
}

pub fn select_model<'a>(
    registry: &'a [ModelDescriptor],
    req: &Requirements,
    hw: &crate::sovereign::hardware::HardwareProfile,
) -> Result<Selection<'a>> {
    let capable: Vec<&ModelDescriptor> = registry
        .iter()
        .filter(|m| {
            if req.vision && !m.capabilities.vision {
                return false;
            }
            m.capabilities.context_tokens >= req.min_context_tokens
        })
        .collect();

    if capable.is_empty() {
        bail!(
            "no model satisfies requirements (vision={}, domain={:?}, context>={}); \
             this is a capability gap, not a hardware limit",
            req.vision,
            req.domain,
            req.min_context_tokens
        );
    }

    let mut ranked = capable;
    ranked.sort_by_key(|m| {
        let domain_match = m.capabilities.domains.contains(&req.domain);
        let tools_ok = m.capabilities.tool_calling == req.tool_calling;
        let degraded = matches!(assess(m, hw), Fit::Degraded { .. });
        (
            !domain_match,
            !tools_ok,
            degraded,
            std::cmp::Reverse(m.capabilities.context_tokens),
        )
    });

    let chosen = ranked[0];
    let fit = assess(chosen, hw);

    Ok(Selection {
        descriptor: chosen,
        reason: format!(
            "{:?} requires domain {:?}, tool-calling {:?}; {} declares {:?}",
            req.domain, req.domain, req.tool_calling, chosen.id, chosen.capabilities.domains
        ),
        fit,
    })
}

/// Rejected at configuration time rather than request time: a non-loopback
/// endpoint would turn universal model support into an egress hole.
pub fn validate_backend(backend: &Backend) -> Result<()> {
    let Backend::LocalHttp { endpoint } = backend else {
        return Ok(());
    };

    let host = endpoint
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|hostport| hostport.rsplit(':').next_back())
        .unwrap_or_default();

    let is_loopback = host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);

    if !is_loopback {
        bail!("LocalHttp endpoint {endpoint} is not loopback; this would violate A5");
    }
    Ok(())
}
