/// Trait for embedding providers that convert text to vectors.
///
/// This trait is intentionally synchronous. The `openai` feature provides a blocking
/// implementation via `ureq`. When used with `AsyncMindGraph`, calls are automatically
/// wrapped in `spawn_blocking`. For high-throughput async workloads, use
/// [`AsyncEmbeddingProvider`] (behind the `async` feature) with a native async HTTP client.
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

/// Async trait for embedding providers. Requires the `async` feature.
///
/// Unlike [`EmbeddingProvider`], this trait uses native async methods, allowing
/// truly non-blocking HTTP calls (e.g., via `reqwest`).
///
/// Use [`SyncProviderAdapter`] to wrap an existing sync provider.
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncEmbeddingProvider: Send + Sync {
    /// The dimensionality of embeddings produced by this provider.
    fn dimension(&self) -> usize;

    /// Embed a single text string into a vector.
    async fn embed(&self, text: &str) -> crate::Result<Vec<f32>>;

    /// Embed a batch of text strings. Default implementation calls `embed` sequentially.
    async fn embed_batch(&self, texts: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for t in texts {
            results.push(self.embed(t).await?);
        }
        Ok(results)
    }
}

/// Adapter that wraps a sync [`EmbeddingProvider`] as an [`AsyncEmbeddingProvider`]
/// by running calls via `tokio::task::spawn_blocking`.
#[cfg(feature = "async")]
pub struct SyncProviderAdapter {
    inner: std::sync::Arc<dyn EmbeddingProvider>,
}

#[cfg(feature = "async")]
impl SyncProviderAdapter {
    /// Create a new adapter wrapping the given sync provider.
    pub fn new(provider: std::sync::Arc<dyn EmbeddingProvider>) -> Self {
        Self { inner: provider }
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl AsyncEmbeddingProvider for SyncProviderAdapter {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    async fn embed(&self, text: &str) -> crate::Result<Vec<f32>> {
        let provider = self.inner.clone();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || provider.embed(&text))
            .await
            .map_err(|e| crate::Error::TaskJoin(e.to_string()))?
    }

    async fn embed_batch(&self, texts: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        let provider = self.inner.clone();
        let texts: Vec<String> = texts.to_vec();
        tokio::task::spawn_blocking(move || {
            let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            provider.embed_batch(&refs)
        })
        .await
        .map_err(|e| crate::Error::TaskJoin(e.to_string()))?
    }
}
