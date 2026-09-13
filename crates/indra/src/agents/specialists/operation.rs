use crate::sovereign::hardware::HardwareProfile;
use crate::sovereign::model_registry::{
    select_model, Backend, Capabilities, Domain, Fit, ModelDescriptor, Requirements, ToolCalling,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IndraOperation {
    GeneralChat,
    CodeGeneration,
    InspectionAnalysis,
    PidAnalysis,
    EngineeringCalculation,
}

impl IndraOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndraOperation::GeneralChat => "GENERAL_CHAT",
            IndraOperation::CodeGeneration => "CODE_GENERATION",
            IndraOperation::InspectionAnalysis => "INSPECTION_ANALYSIS",
            IndraOperation::PidAnalysis => "PID_ANALYSIS",
            IndraOperation::EngineeringCalculation => "ENGINEERING_CALCULATION",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            IndraOperation::GeneralChat => "General Chat",
            IndraOperation::CodeGeneration => "Code Generation",
            IndraOperation::InspectionAnalysis => "Inspection Analysis",
            IndraOperation::PidAnalysis => "P&ID Analysis",
            IndraOperation::EngineeringCalculation => "Engineering Calculation",
        }
    }

    pub fn requirements(&self) -> Requirements {
        match self {
            IndraOperation::CodeGeneration => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Code,
                min_context_tokens: 16_384,
            },
            IndraOperation::PidAnalysis => Requirements {
                vision: true,
                tool_calling: ToolCalling::Unreliable,
                domain: Domain::Engineering,
                min_context_tokens: 8_192,
            },
            IndraOperation::InspectionAnalysis => Requirements {
                vision: true,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Document,
                min_context_tokens: 16_384,
            },
            IndraOperation::EngineeringCalculation => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Engineering,
                min_context_tokens: 8_192,
            },
            IndraOperation::GeneralChat => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::General,
                min_context_tokens: 8_192,
            },
        }
    }

    /// Trace text describing which model this operation would pick and why,
    /// plus a hardware-fit warning when present. Built only from capabilities
    /// the currently configured model actually declares (context limit, vision
    /// support) — nothing about tool-calling reliability or domain is known for
    /// an arbitrary configured provider, so those are left at neutral defaults
    /// that never overstate what the model can do. Returns `None` when the
    /// configured model has a genuine capability gap (e.g. no vision for a
    /// vision task): that is a fact worth logging elsewhere, not a selection.
    pub fn selection_trace(
        &self,
        model_name: &str,
        context_limit: Option<usize>,
        supports_vision: Option<bool>,
        hw: &HardwareProfile,
    ) -> Option<(String, Option<String>)> {
        let vision = supports_vision.unwrap_or(false);
        let descriptor = ModelDescriptor {
            id: model_name.to_string(),
            backend: Backend::Llamacpp,
            path: PathBuf::new(),
            mmproj_path: vision.then(|| PathBuf::from("configured")),
            capabilities: Capabilities {
                vision,
                tool_calling: ToolCalling::Reliable,
                context_tokens: context_limit.unwrap_or(0) as u32,
                domains: vec![Domain::General],
            },
            min_vram_mb: 0,
        };

        let selection =
            select_model(std::slice::from_ref(&descriptor), &self.requirements(), hw).ok()?;

        let reason = format!(
            "{} \u{2192} `{}` ({})",
            self.display_name(),
            selection.descriptor.id,
            selection.reason
        );
        let warning = match selection.fit {
            Fit::Tight { warning } | Fit::Degraded { warning } => Some(warning),
            Fit::Comfortable => None,
        };

        Some((reason, warning))
    }
}
