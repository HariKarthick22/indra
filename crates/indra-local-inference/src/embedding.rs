use std::path::Path;
use std::sync::OnceLock;

use anyhow::{bail, Result};
use llama_cpp_2::context::params::{LlamaContextParams, LlamaPoolingType};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};

/// Advisory dimension for the embedding models this crate is expected to run
/// (e.g. nomic-embed-text-class models). `embed_text` returns whatever
/// dimension the loaded GGUF model actually reports; callers that need the
/// real dimension for a specific model should use the returned vector's
/// length rather than this constant.
pub const EMBEDDING_DIM: usize = 768;

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

// `LlamaBackend::init()` succeeds exactly once per process (it calls into
// llama.cpp's global, non-reentrant `llama_backend_init()`); every subsequent
// call returns `Err(BackendAlreadyInitialized)`. `embed_text` is called
// repeatedly (once per document/query), so it must reuse one process-wide
// backend rather than initializing a fresh one on every call.
fn backend() -> Result<&'static LlamaBackend> {
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    let b = LlamaBackend::init()?;
    Ok(BACKEND.get_or_init(|| b))
}

/// Generate a text embedding for `text` using the GGUF model at `model_path`,
/// via llama.cpp's embedding pooling mode (mean pooling over the sequence).
pub fn embed_text(model_path: &Path, text: &str) -> Result<Vec<f32>> {
    let backend = backend()?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;

    let tokens = model.str_to_token(text, AddBos::Always)?;
    if tokens.is_empty() {
        bail!("text tokenized to zero tokens");
    }

    let n_ctx = (tokens.len() as u32).max(512);
    let ctx_params = LlamaContextParams::default()
        .with_embeddings(true)
        .with_pooling_type(LlamaPoolingType::Mean)
        .with_n_ctx(std::num::NonZeroU32::new(n_ctx));
    let mut ctx = model.new_context(&backend, ctx_params)?;

    let mut batch = LlamaBatch::new(tokens.len(), 1);
    batch.add_sequence(&tokens, 0, false)?;
    ctx.decode(&mut batch)?;

    let embedding = ctx.embeddings_seq_ith(0)?;
    Ok(embedding.to_vec())
}
