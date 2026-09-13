use serde::{Deserialize, Serialize};

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
}
