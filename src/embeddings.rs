/// Trait for embedding providers that convert text to vectors.
///
/// This trait is intentionally synchronous. The `openai` feature provides a blocking
/// implementation via `ureq`. When used with `AsyncMindGraph`, calls are automatically
/// wrapped in `spawn_blocking`. For high-throughput async workloads, implement a custom
/// provider that uses an async HTTP client internally (e.g., `reqwest`) and block only
/// within the `embed`/`embed_batch` methods.
pub trait EmbeddingProvider: Send + Sync {
    /// The dimensionality of embeddings produced by this provider.
    fn dimension(&self) -> usize;

    /// Embed a single text string into a vector.
    fn embed(&self, text: &str) -> crate::Result<Vec<f32>>;

    /// Embed a batch of text strings. Default implementation calls `embed` sequentially.
    fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}
