//! Text embedding backends. Default is local fastembed (BGE-large, 1024d).

use std::sync::Mutex;

use anyhow::Result;
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};

use crate::types::EMBEDDING_DIM;

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

/// Local fastembed-backed embedder (BGE-large-en-v1.5, 1024 dims).
///
/// fastembed `TextEmbedding::embed` takes `&mut self`, so the model is held
/// behind a `Mutex` to satisfy the `&self` trait method and `Send + Sync`.
pub struct LocalEmbedder {
    model: Mutex<TextEmbedding>,
}

impl LocalEmbedder {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            TextInitOptions::new(EmbeddingModel::BGELargeENV15) // 1024 dims
                .with_show_download_progress(true),
        )?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }
}

impl Embedder for LocalEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut model = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("embedder mutex poisoned"))?;
        // fastembed embed(&mut self, texts, batch_size) -> Result<Vec<Vec<f32>>>;
        // None => default batch size.
        let out = model.embed(texts.to_vec(), None)?;
        if let Some(first) = out.first() {
            anyhow::ensure!(
                first.len() == EMBEDDING_DIM,
                "unexpected embedding dim: {} (expected {})",
                first.len(),
                EMBEDDING_DIM
            );
        }
        Ok(out)
    }
}
