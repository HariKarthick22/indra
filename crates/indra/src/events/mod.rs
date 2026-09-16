mod types;
pub use types::{IndraEvent, PlanStep, SourceBox};

use indra_provider_types::conversation::message::MessageMetadata;

pub fn emit(metadata: &mut MessageMetadata, event: IndraEvent) {
    let value = serde_json::to_value(&event).expect("IndraEvent always serializes");
    metadata.set_operation_note("indra", "indra_event", value);
}
