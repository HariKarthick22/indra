use super::operation::IndraOperation;
use std::collections::HashSet;

pub trait Specialist: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn owned_operations(&self) -> &[IndraOperation];
    fn is_stub(&self) -> bool;
    fn stub_message(&self) -> Option<&str> {
        None
    }
    fn system_prompt_framing(&self) -> Option<&str> {
        None
    }
    fn allowed_tools(&self) -> Option<&HashSet<String>> {
        None
    }
}

pub struct GeneralChatSpecialist;

impl Specialist for GeneralChatSpecialist {
    fn name(&self) -> &str {
        "General Chat Specialist"
    }

    fn description(&self) -> &str {
        "Handles general queries, reasoning, and software development tasks."
    }

    fn owned_operations(&self) -> &[IndraOperation] {
        &[IndraOperation::GeneralChat, IndraOperation::CodeGeneration]
    }

    fn is_stub(&self) -> bool {
        false
    }
}

pub struct InspectionAnalysisSpecialist;

impl Specialist for InspectionAnalysisSpecialist {
    fn name(&self) -> &str {
        "Inspection Analysis Specialist"
    }

    fn description(&self) -> &str {
        "Domain engineering specialist for mechanical inspections, NDT reports, and equipment integrity."
    }

    fn owned_operations(&self) -> &[IndraOperation] {
        &[IndraOperation::InspectionAnalysis]
    }

    fn is_stub(&self) -> bool {
        false
    }

    fn system_prompt_framing(&self) -> Option<&str> {
        Some(
            "You are INDRA's Inspection Analysis Specialist, an expert AI engineer for refinery and plant equipment. \
            When analyzing inspections, non-destructive testing (NDT), or vibration reports:\n\
            1. Cite specific inspection findings, test dates, and equipment tag numbers (e.g., P-101).\n\
            2. Compare current values against historical baselines, design limits, and previous inspection reports.\n\
            3. Clearly categorize anomalies by severity (Normal / Monitor / Critical Action Required).\n\
            4. Provide concrete, actionable engineering recommendations.",
        )
    }
}

pub struct PidAnalysisSpecialist;

impl Specialist for PidAnalysisSpecialist {
    fn name(&self) -> &str {
        "P&ID Analysis Specialist"
    }

    fn description(&self) -> &str {
        "Domain specialist for Piping & Instrumentation Diagrams and process schematics."
    }

    fn owned_operations(&self) -> &[IndraOperation] {
        &[IndraOperation::PidAnalysis]
    }

    fn is_stub(&self) -> bool {
        true
    }

    fn stub_message(&self) -> Option<&str> {
        Some(
            "⚠️ **P&ID Analysis Specialist is not yet available in this version.**\n\n\
            Automated parsing of Piping & Instrumentation Diagrams requires the vision/OCR pipeline, \
            which is scheduled for a future release. Please consult engineering drawings manually.",
        )
    }
}

pub struct EngineeringCalcSpecialist;

impl Specialist for EngineeringCalcSpecialist {
    fn name(&self) -> &str {
        "Engineering Calculation Specialist"
    }

    fn description(&self) -> &str {
        "Domain specialist for deterministic engineering calculations and simulations."
    }

    fn owned_operations(&self) -> &[IndraOperation] {
        &[IndraOperation::EngineeringCalculation]
    }

    fn is_stub(&self) -> bool {
        true
    }

    fn stub_message(&self) -> Option<&str> {
        Some(
            "⚠️ **Engineering Calculation Specialist is not yet available in this version.**\n\n\
            Deterministic engineering solvers (for pressure vessel design, piping stress, and fluid dynamics) \
            are currently under development.",
        )
    }
}
