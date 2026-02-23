use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionProps {
    pub started_at: f64,
    pub ended_at: Option<f64>,
    pub session_type: Option<String>,
    pub focus_summary: Option<String>,
    pub active_goal_uids: Vec<String>,
}

impl Default for SessionProps {
    fn default() -> Self {
        Self {
            started_at: 0.0,
            ended_at: None,
            session_type: None,
            focus_summary: None,
            active_goal_uids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TraceProps {
    pub session_uid: Option<String>,
    pub trace_type: Option<String>,
    pub entry_count: u32,
    pub started_at: Option<f64>,
    pub ended_at: Option<f64>,
    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SummaryProps {
    pub content: String,
    pub summary_type: Option<String>,
    pub source_node_uids: Vec<String>,
    pub compression_ratio: Option<f64>,
    pub generated_by: Option<String>,
    pub generated_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PreferenceProps {
    pub key: String,
    pub value: String,
    pub preference_type: Option<String>,
    pub learned: bool,
    pub explicit: bool,
    pub evidence_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryPolicyProps {
    pub policy_type: Option<String>,
    pub target_node_type: Option<String>,
    pub condition: Option<String>,
    pub action: Option<String>,
    pub active: bool,
}

impl Default for MemoryPolicyProps {
    fn default() -> Self {
        Self {
            policy_type: None,
            target_node_type: None,
            condition: None,
            action: None,
            active: true,
        }
    }
}
