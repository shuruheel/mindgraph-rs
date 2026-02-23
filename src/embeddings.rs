/// Trait for embedding providers that convert text to vectors.
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
