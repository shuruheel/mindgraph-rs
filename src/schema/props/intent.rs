use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GoalProps {
    pub description: String,
    pub goal_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<f64>,
    pub success_criteria: Vec<String>,
    pub progress: f64,
}

impl Default for GoalProps {
    fn default() -> Self {
        Self {
            description: String::new(),
            goal_type: None,
            status: None,
            priority: None,
            deadline: None,
            success_criteria: Vec::new(),
            progress: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectProps {
    pub name: String,
    pub description: String,
    pub status: Option<String>,
    pub started_at: Option<f64>,
    pub target_completion: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DecisionProps {
    pub question: String,
    pub status: Option<String>,
    pub decided_option_uid: Option<String>,
    pub decided_at: Option<f64>,
    pub decision_rationale: Option<String>,
    pub reversibility: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OptionProps {
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub estimated_effort: Option<String>,
    pub estimated_risk: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConstraintProps {
    pub description: String,
    pub constraint_type: Option<String>,
    pub hard: bool,
    pub value: Option<String>,
    pub unit: Option<String>,
}

impl Default for ConstraintProps {
    fn default() -> Self {
        Self {
            description: String::new(),
            constraint_type: None,
            hard: true,
            value: None,
            unit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MilestoneProps {
    pub description: String,
    pub status: Option<String>,
    pub target_date: Option<f64>,
    pub reached_at: Option<f64>,
    pub criteria: Vec<String>,
}
