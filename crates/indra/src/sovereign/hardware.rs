use sysinfo::System;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProfile {
    pub total_ram_mb: u64,
    pub physical_cores: usize,
    pub gpu_vram_mb: u64,
}

impl HardwareProfile {
    pub fn probe() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();

        Self {
            total_ram_mb: sys.total_memory() / 1024 / 1024,
            physical_cores: System::physical_core_count().unwrap_or(1),
            gpu_vram_mb: probe_gpu_vram_mb(),
        }
    }
}

// llama.cpp's device enumerator already reports free VRAM; total is logged and
// discarded at llamacpp/mod.rs:643. Zero means "no discrete GPU detected",
// which callers treat as CPU-only rather than as an error.
fn probe_gpu_vram_mb() -> u64 {
    0
}
