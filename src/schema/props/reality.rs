use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceProps {
    pub source_type: String,
    pub uri: String,
    pub title: String,
    pub content_hash: Option<String>,
    pub fetched_at: Option<f64>,
    pub mime_type: Option<String>,
    pub content_length: Option<u64>,
    pub domain: Option<String>,
}

impl Default for SourceProps {
    fn default() -> Self {
        Self {
            source_type: "web_page".into(),
            uri: String::new(),
            title: String::new(),
            content_hash: None,
            fetched_at: None,
            mime_type: None,
            content_length: None,
            domain: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnippetProps {
    pub content: String,
    pub snippet_type: String,
    pub source_location: Option<String>,
    pub char_offset_start: Option<u64>,
    pub char_offset_end: Option<u64>,
}

impl Default for SnippetProps {
    fn default() -> Self {
        Self {
            content: String::new(),
            snippet_type: "text".into(),
            source_location: None,
            char_offset_start: None,
            char_offset_end: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EntityProps {
    pub entity_type: String,
    pub canonical_name: String,
    pub description: Option<String>,
    pub identifiers: serde_json::Value,
    pub attributes: serde_json::Value,
}

impl Default for EntityProps {
    fn default() -> Self {
        Self {
            entity_type: "other".into(),
            canonical_name: String::new(),
            description: None,
            identifiers: serde_json::json!({}),
            attributes: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservationProps {
    pub observation_type: String,
    pub content: String,
    pub session_uid: Option<String>,
    pub timestamp: Option<f64>,
}

impl Default for ObservationProps {
    fn default() -> Self {
        Self {
            observation_type: "system_event".into(),
            content: String::new(),
            session_uid: None,
            timestamp: None,
        }
    }
}
