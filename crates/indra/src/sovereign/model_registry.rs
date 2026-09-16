use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use url::{Host, Url};

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

/// Rejected at configuration time rather than request time: an endpoint that
/// can reach the public internet would turn universal model support into an
/// egress hole.
///
/// A company's model server may run on a separate machine on the plant's own
/// air-gapped network, so private LAN addresses are permitted alongside
/// loopback. Domain names are refused even when they would resolve to a
/// private address: resolution happens later and can change, so a name cannot
/// be validated here in any way that still holds at request time. Configure
/// the address literally.
pub fn validate_backend(backend: &Backend) -> Result<()> {
    let Backend::LocalHttp { endpoint } = backend else {
        return Ok(());
    };

    let url = Url::parse(endpoint)
        .map_err(|e| anyhow::anyhow!("LocalHttp endpoint {endpoint} is not a valid URL: {e}"))?;

    match url.host() {
        Some(Host::Ipv4(ip)) if is_internal_v4(&ip) => Ok(()),
        Some(Host::Ipv6(ip)) if is_internal_v6(&ip) => Ok(()),
        Some(Host::Domain("localhost")) => Ok(()),
        Some(Host::Domain(name)) => bail!(
            "LocalHttp endpoint {endpoint} uses the name '{name}'; configure the private IP \
             literally, since a name resolves at request time and cannot be checked here"
        ),
        _ => bail!(
            "LocalHttp endpoint {endpoint} is not a loopback or private-network address; \
             this would violate A5"
        ),
    }
}

fn is_internal_v4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local()
}

// Hand-rolled rather than using is_unique_local/is_unicast_link_local, which
// are still unstable: fc00::/7 is unique-local, fe80::/10 is link-local.
fn is_internal_v6(ip: &Ipv6Addr) -> bool {
    let unique_local = ip.octets()[0] & 0xfe == 0xfc;
    let link_local = ip.segments()[0] & 0xffc0 == 0xfe80;
    ip.is_loopback() || unique_local || link_local
}
