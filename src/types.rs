use serde::{Deserialize, Serialize};
use serde::de;
use std::fmt;

/// Unique identifier for nodes and edges.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Uid(String);

impl Uid {
    pub fn new() -> Self {
        Uid(uuid::Uuid::new_v4().to_string())
    }

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
    pub fn new(value: f64) -> crate::Result<Self> {
        if (0.0..=1.0).contains(&value) {
            Ok(Confidence(value))
        } else {
            Err(crate::Error::InvalidConfidence(value))
        }
    }

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
    pub fn new(value: f64) -> crate::Result<Self> {
        if (0.0..=1.0).contains(&value) {
            Ok(Salience(value))
        } else {
            Err(crate::Error::InvalidSalience(value))
        }
    }

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
