use serde::de;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for nodes and edges.
///
/// Wraps a UUID v4 string. The inner field is private to prevent construction
/// of invalid UIDs. Use [`Uid::new()`] for random UIDs, [`Uid::from()`] to
/// wrap an existing string, or [`Uid::as_str()`] to read the value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Uid(String);

impl Uid {
    /// Generate a new random UUID v4 identifier.
    pub fn new() -> Self {
        Uid(uuid::Uuid::new_v4().to_string())
    }

    /// Get the UID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for Uid {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Uid(s.to_string()))
    }
}

impl Default for Uid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Uid {
    fn from(s: String) -> Self {
        Uid(s)
    }
}

impl From<&str> for Uid {
    fn from(s: &str) -> Self {
        Uid(s.to_string())
    }
}

/// Epistemic certainty score (0.0–1.0).
///
/// Used on both nodes and edges to represent how confident the system is
/// in a piece of knowledge. Validated on construction to ensure the value
/// stays within bounds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Confidence(f64);

impl<'de> de::Deserialize<'de> for Confidence {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        if (0.0..=1.0).contains(&value) {
            Ok(Confidence(value))
        } else {
            Err(de::Error::custom(format!(
                "Confidence must be between 0.0 and 1.0, got {}",
                value
            )))
        }
    }
}

impl Confidence {
    /// Create a new confidence value. Returns an error if `value` is outside 0.0–1.0.
    pub fn new(value: f64) -> crate::Result<Self> {
        if (0.0..=1.0).contains(&value) {
            Ok(Confidence(value))
        } else {
            Err(crate::Error::InvalidConfidence(value))
        }
    }

    /// Get the inner f64 value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Confidence(1.0)
    }
}

impl From<Confidence> for f64 {
    fn from(c: Confidence) -> f64 {
        c.0
    }
}

/// Contextual relevance score (0.0–1.0). Decays over time.
///
/// Salience represents how relevant a node is in the current context.
/// It starts at a configured value (default 0.5) and decays exponentially
/// via [`MindGraph::decay_salience`](crate::MindGraph::decay_salience).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Salience(f64);

impl<'de> de::Deserialize<'de> for Salience {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        if (0.0..=1.0).contains(&value) {
            Ok(Salience(value))
        } else {
            Err(de::Error::custom(format!(
                "Salience must be between 0.0 and 1.0, got {}",
                value
            )))
        }
    }
}

impl Salience {
    /// Create a new salience value. Returns an error if `value` is outside 0.0–1.0.
    pub fn new(value: f64) -> crate::Result<Self> {
        if (0.0..=1.0).contains(&value) {
            Ok(Salience(value))
        } else {
            Err(crate::Error::InvalidSalience(value))
        }
    }

    /// Get the inner f64 value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for Salience {
    fn default() -> Self {
        Salience(0.5)
    }
}

impl From<Salience> for f64 {
    fn from(s: Salience) -> f64 {
        s.0
    }
}

/// Privacy level for nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel {
    #[default]
    Private,
    Shared,
    Public,
}

impl PrivacyLevel {
    /// Get the privacy level as a lowercase string.
    pub fn as_str(&self) -> &str {
        match self {
            PrivacyLevel::Private => "private",
            PrivacyLevel::Shared => "shared",
            PrivacyLevel::Public => "public",
        }
    }
}

/// Unix timestamp as f64.
pub type Timestamp = f64;

/// Returns the current Unix timestamp.
pub fn now() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}
