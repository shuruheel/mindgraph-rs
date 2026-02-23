use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AffordanceProps {
    pub action_name: String,
    pub description: String,
    pub affordance_type: Option<String>,
    pub source_uid: Option<String>,
    pub selector: Option<String>,
    pub parameters: Vec<String>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub risk_level: Option<String>,
    pub reversible: bool,
}

impl Default for AffordanceProps {
    fn default() -> Self {
        Self {
            action_name: String::new(),
            description: String::new(),
            affordance_type: None,
            source_uid: None,
            selector: None,
            parameters: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            risk_level: None,
            reversible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FlowProps {
    pub name: String,
    pub description: String,
    pub flow_type: Option<String>,
    pub step_count: Option<u32>,
    pub estimated_duration_seconds: Option<f64>,
    pub overall_risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FlowStepProps {
    pub order: u32,
    pub description: String,
    pub affordance_uid: Option<String>,
    pub is_optional: bool,
    pub is_checkpoint: bool,
    pub fallback_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlProps {
    pub control_type: String,
    pub label: Option<String>,
    pub selector: Option<String>,
    pub source_uid: Option<String>,
    pub state: Option<String>,
    pub value: Option<String>,
}

impl Default for ControlProps {
    fn default() -> Self {
        Self {
            control_type: "button".into(),
            label: None,
            selector: None,
            source_uid: None,
            state: None,
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RiskAssessmentProps {
    pub target_uid: Option<String>,
    pub risk_type: Option<String>,
    pub severity: Option<String>,
    pub likelihood: Option<f64>,
    pub mitigation: Option<String>,
    pub requires_approval: bool,
}

impl Default for RiskAssessmentProps {
    fn default() -> Self {
        Self {
            target_uid: None,
            risk_type: None,
            severity: None,
            likelihood: None,
            mitigation: None,
            requires_approval: true,
        }
    }
}
