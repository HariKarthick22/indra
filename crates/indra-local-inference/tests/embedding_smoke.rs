use indra_local_inference::embedding::embed_text;
use std::path::Path;

#[test]
fn embedding_of_real_text_is_a_nonzero_vector_of_fixed_dimension() {
    let model_path = std::env::var("INDRA_TEST_EMBEDDING_MODEL").unwrap_or_default();
    if model_path.is_empty() {
        eprintln!("INDRA_TEST_EMBEDDING_MODEL not set; skipping (no model on this machine)");
        return;
    }

    let a = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();
    let b = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();

    assert!(!a.iter().all(|&x| x == 0.0), "embedding was all zeros");
    assert_eq!(a.len(), b.len(), "dimension must be stable across calls");
    assert_eq!(
        a, b,
        "identical input must produce identical embedding (determinism)"
    );
}

#[test]
fn different_text_produces_different_embeddings() {
    let model_path = std::env::var("INDRA_TEST_EMBEDDING_MODEL").unwrap_or_default();
    if model_path.is_empty() {
        return;
    }

    let a = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();
    let b = embed_text(Path::new(&model_path), "quarterly revenue report").unwrap();
    assert_ne!(a, b, "unrelated text produced identical embeddings");
}
