//! Text embedding backends. Default is local fastembed (BGE-large, 1024d).

use anyhow::Result;

/// Produces `EMBEDDING_DIM`-length vectors for text.
///
/// `&self` (not `&mut self`) so it can live behind `Arc<dyn Embedder>` and be
/// shared across async tasks. Implementations that wrap a `&mut`-only model
/// (e.g. fastembed) must use interior mutability (`Mutex`).
pub trait Embedder: Send + Sync {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut out = self.embed(std::slice::from_ref(&text.to_string()))?;
        Ok(out.pop().unwrap_or_default())
    }
}

/// Local fastembed-backed embedder.
///
/// WORKER B — implement:
///   - hold a `std::sync::Mutex<fastembed::TextEmbedding>` (fastembed `embed`
///     takes `&mut self`, so a Mutex gives us `&self` trait access).
///   - `new()`: `TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::BGELargeENV15)
///     .with_show_download_progress(true))`. 1024 dims. Respect `HF_HOME` /
///     `EPISODE_MODEL_CACHE` env for the model cache dir if set.
///   - `embed()`: lock the model, call `model.embed(texts, None)` -> `Vec<Vec<f32>>`.
///     Assert/verify each vector length == `crate::types::EMBEDDING_DIM`.
pub struct LocalEmbedder {
    // WORKER B: model: std::sync::Mutex<fastembed::TextEmbedding>,
}

impl LocalEmbedder {
    pub fn new() -> Result<Self> {
        todo!("WORKER B: init fastembed BGELargeENV15 (1024d)")
    }
}

impl Embedder for LocalEmbedder {
    fn embed(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
        todo!("WORKER B: lock model + fastembed embed")
    }
}
