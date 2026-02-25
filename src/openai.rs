//! OpenAI embedding provider (behind the `openai` feature flag).

use crate::embeddings::EmbeddingProvider;
use crate::error::Error;

/// OpenAI-compatible embedding provider using `ureq` for sync HTTP.
pub struct OpenAIEmbeddings {
    api_key: String,
    model: String,
    dim: usize,
    base_url: String,
}

impl OpenAIEmbeddings {
    /// Create a new provider with the given API key.
    /// Defaults to `text-embedding-3-small` with 1536 dimensions.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "text-embedding-3-small".into(),
            dim: 1536,
            base_url: "https://api.openai.com".into(),
        }
    }

    /// Set the model name.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the embedding dimension.
    pub fn dimension(mut self, dim: usize) -> Self {
        self.dim = dim;
        self
    }

    /// Set the base URL (for Azure OpenAI or compatible APIs).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl EmbeddingProvider for OpenAIEmbeddings {
    fn dimension(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> crate::Result<Vec<f32>> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "input": text,
            "model": self.model,
            "dimensions": self.dim,
        });

        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| Error::Http(e.to_string()))?;

        let json: serde_json::Value = resp.into_json().map_err(|e| Error::Http(e.to_string()))?;

        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| Error::Http("Missing embedding in response".into()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "input": texts,
            "model": self.model,
            "dimensions": self.dim,
        });

        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| Error::Http(e.to_string()))?;

        let json: serde_json::Value = resp.into_json().map_err(|e| Error::Http(e.to_string()))?;

        let data = json["data"]
            .as_array()
            .ok_or_else(|| Error::Http("Missing data in response".into()))?;

        let mut results = Vec::with_capacity(data.len());
        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| Error::Http("Missing embedding in response".into()))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            results.push(embedding);
        }

        Ok(results)
    }
}
