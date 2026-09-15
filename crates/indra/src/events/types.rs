use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBox {
    pub doc_id: String,
    pub version: String,
    pub page: u32,
    pub bbox: [u32; 4],
    pub conf: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum IndraEvent {
    #[serde(rename = "plan.proposed")]
    PlanProposed { steps: Vec<PlanStep> },
    #[serde(rename = "plan.revised")]
    PlanRevised {
        after_step: String,
        steps: Vec<PlanStep>,
        reason: String,
    },
    #[serde(rename = "step.state")]
    StepState {
        step_id: String,
        state: String,
        ms: Option<u64>,
    },
    #[serde(rename = "model.selected")]
    ModelSelected {
        model_id: String,
        reason: String,
        fit: serde_json::Value,
    },
    #[serde(rename = "tool.call")]
    ToolCall {
        call_id: String,
        step_id: String,
        name: String,
        args: serde_json::Value,
        sandbox_backend: String,
    },
    #[serde(rename = "tool.result")]
    ToolResult {
        call_id: String,
        ok: bool,
        ms: u64,
        summary: String,
        bytes: usize,
        citations: Vec<SourceBox>,
    },
    #[serde(rename = "citation")]
    Citation { span: [usize; 2], source: SourceBox },
    #[serde(rename = "context.delta")]
    ContextDelta {
        session_bytes: usize,
        budget_bytes: usize,
    },
    #[serde(rename = "context.compacted")]
    ContextCompacted {
        from_turns: usize,
        to_bytes: usize,
        dropped_count: usize,
    },
    #[serde(rename = "memory.touch")]
    MemoryTouch { node_ids: Vec<String>, op: String },
    #[serde(rename = "egress.attempt")]
    EgressAttempt {
        url: String,
        blocked: bool,
        at: String,
    },
    #[serde(rename = "verify.recompute")]
    VerifyRecompute {
        claim: f64,
        computed: f64,
        verdict: String,
    },
    #[serde(rename = "guard.blocked")]
    GuardBlocked { resource: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub label: String,
    pub tool: Option<String>,
}
