use serde::{Deserialize, Serialize};

use crate::types::{Timestamp, Uid};

/// A provenance record linking a node to its source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub node_uid: Uid,
    pub source_uid: Uid,
    pub extraction_method: ExtractionMethod,
    pub extraction_confidence: f64,
    pub source_location: String,
    pub text_span: String,
    pub extracted_by: String,
    pub extracted_at: Timestamp,
}

/// How a piece of knowledge was extracted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    Human,
    Llm,
    Rule,
    Computed,
    Unknown,
}

impl ExtractionMethod {
    pub fn as_str(&self) -> &str {
        match self {
            ExtractionMethod::Human => "human",
            ExtractionMethod::Llm => "llm",
            ExtractionMethod::Rule => "rule",
            ExtractionMethod::Computed => "computed",
            ExtractionMethod::Unknown => "unknown",
        }
    }
}

/// An entry in a provenance chain (returned from chain queries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub source_uid: Uid,
    pub source_label: String,
    pub source_type: String,
    pub depth: i64,
}
