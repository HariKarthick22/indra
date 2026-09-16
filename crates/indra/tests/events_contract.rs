use indra::events::{emit, IndraEvent};
use indra_provider_types::conversation::message::MessageMetadata;
use serde_json::json;

#[test]
fn model_selected_event_roundtrips_through_metadata() {
    let mut meta = MessageMetadata::default();
    emit(
        &mut meta,
        IndraEvent::ModelSelected {
            model_id: "qwen-coder".to_string(),
            reason: "Code Generation requires domain Code".to_string(),
            fit: json!({"kind": "comfortable"}),
        },
    );

    let stored = meta
        .get_operation_note("indra", "indra_event")
        .expect("event was not recorded");
    assert_eq!(stored["t"], "model.selected");
    assert_eq!(stored["model_id"], "qwen-coder");
}

#[test]
fn egress_attempt_event_carries_blocked_true() {
    let mut meta = MessageMetadata::default();
    emit(
        &mut meta,
        IndraEvent::EgressAttempt {
            url: "https://example.com".to_string(),
            blocked: true,
            at: "2026-09-13T14:02:31Z".to_string(),
        },
    );

    let stored = meta.get_operation_note("indra", "indra_event").unwrap();
    assert_eq!(stored["t"], "egress.attempt");
    assert_eq!(stored["blocked"], true);
}
