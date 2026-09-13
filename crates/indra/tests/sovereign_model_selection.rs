use indra::agents::specialists::operation::IndraOperation;
use indra::sovereign::hardware::HardwareProfile;
use indra::sovereign::model_registry::{
    select_model, Backend, Capabilities, Domain, Fit, ModelDescriptor, ToolCalling,
};
use std::path::PathBuf;

fn hw(ram_mb: u64, vram_mb: u64) -> HardwareProfile {
    HardwareProfile {
        total_ram_mb: ram_mb,
        physical_cores: 8,
        gpu_vram_mb: vram_mb,
    }
}

fn model(id: &str, vision: bool, domains: Vec<Domain>, min_vram_mb: u64) -> ModelDescriptor {
    ModelDescriptor {
        id: id.to_string(),
        backend: Backend::Llamacpp,
        path: PathBuf::from(format!("/models/{id}.gguf")),
        mmproj_path: if vision {
            Some(PathBuf::from("/models/mmproj.gguf"))
        } else {
            None
        },
        capabilities: Capabilities {
            vision,
            tool_calling: ToolCalling::Reliable,
            context_tokens: 32_768,
            domains,
        },
        min_vram_mb,
    }
}

#[test]
fn code_task_and_document_task_select_different_models() {
    let registry = vec![
        model("qwen-coder", false, vec![Domain::Code], 0),
        model(
            "llama-doc",
            false,
            vec![Domain::Document, Domain::General],
            0,
        ),
    ];
    let hw = hw(32_000, 0);

    let code = select_model(
        &registry,
        &IndraOperation::CodeGeneration.requirements(),
        &hw,
    )
    .unwrap();
    let doc = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(code.descriptor.id, "qwen-coder");
    assert_eq!(doc.descriptor.id, "llama-doc");
    assert_ne!(
        code.descriptor.id, doc.descriptor.id,
        "A1 requires two task types to select differently"
    );
}

#[test]
fn a_model_that_fits_is_preferred_over_one_that_does_not() {
    let registry = vec![
        model("huge", false, vec![Domain::General], 80_000),
        model("small", false, vec![Domain::General], 0),
    ];
    let hw = hw(16_000, 8_000);

    let sel = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(sel.descriptor.id, "small");
    assert_eq!(sel.fit, Fit::Comfortable);
}

#[test]
fn oversized_model_is_still_selected_but_warns() {
    let registry = vec![model("huge", false, vec![Domain::General], 80_000)];
    let hw = hw(16_000, 8_000);

    let sel = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(
        sel.descriptor.id, "huge",
        "hardware shortfall must warn, not exclude"
    );
    match sel.fit {
        Fit::Degraded { ref warning } => {
            assert!(
                warning.contains("8000"),
                "warning must state what is available: {warning}"
            );
        }
        other => panic!("expected Degraded, got {other:?}"),
    }
}

#[test]
fn missing_vision_is_unsupported_not_degraded() {
    let registry = vec![model("blind", false, vec![Domain::General], 0)];
    let hw = hw(64_000, 48_000);

    let result = select_model(&registry, &IndraOperation::PidAnalysis.requirements(), &hw);

    assert!(
        result.is_err(),
        "no amount of hardware makes a blind model see"
    );
}

#[test]
fn empty_registry_is_an_error_not_a_panic() {
    let hw = hw(16_000, 0);
    let result = select_model(&[], &IndraOperation::GeneralChat.requirements(), &hw);
    assert!(result.is_err());
}

#[test]
fn remote_http_backend_is_rejected_at_configuration_time() {
    use indra::sovereign::model_registry::validate_backend;

    let ok = Backend::LocalHttp {
        endpoint: "http://127.0.0.1:11434/v1".to_string(),
    };
    let also_ok = Backend::LocalHttp {
        endpoint: "http://localhost:8000/v1".to_string(),
    };
    let bad = Backend::LocalHttp {
        endpoint: "https://api.openai.com/v1".to_string(),
    };

    assert!(validate_backend(&ok).is_ok());
    assert!(validate_backend(&also_ok).is_ok());
    assert!(
        validate_backend(&bad).is_err(),
        "a non-loopback endpoint would silently void A5"
    );
}

#[test]
fn ranking_is_stable_so_fallthrough_is_deterministic() {
    let registry = vec![
        model("first", false, vec![Domain::General], 0),
        model("second", false, vec![Domain::General], 0),
    ];
    let hw = hw(16_000, 8_000);
    let req = IndraOperation::GeneralChat.requirements();

    let a = select_model(&registry, &req, &hw).unwrap();
    let b = select_model(&registry, &req, &hw).unwrap();

    assert_eq!(
        a.descriptor.id, b.descriptor.id,
        "fallthrough order must not vary between calls"
    );
}
