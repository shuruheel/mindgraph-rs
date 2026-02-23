use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentProps {
    pub name: String,
    pub agent_type: Option<String>,
    pub model_id: Option<String>,
    pub capabilities: Vec<String>,
    pub status: Option<String>,
    pub current_task_uid: Option<String>,
    pub autonomy_level: Option<String>,
    pub domain_restrictions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TaskProps {
    pub description: String,
    pub task_type: Option<String>,
    pub status: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
    pub priority: Option<String>,
    pub deadline: Option<f64>,
    pub parent_goal_uid: Option<String>,
    pub depends_on_task_uids: Vec<String>,
    pub result_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlanProps {
    pub description: String,
    pub task_uid: Option<String>,
    pub status: Option<String>,
    pub step_count: Option<u32>,
    pub estimated_risk: Option<String>,
    pub proposed_at: Option<f64>,
    pub approved_at: Option<f64>,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlanStepProps {
    pub order: u32,
    pub description: String,
    pub plan_uid: Option<String>,
    pub status: Option<String>,
    pub target_affordance_uid: Option<String>,
    pub expected_outcome: Option<String>,
    pub actual_outcome: Option<String>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ApprovalProps {
    pub target_uid: Option<String>,
    pub target_type: Option<String>,
    pub status: Option<String>,
    pub requested_at: Option<f64>,
    pub decided_at: Option<f64>,
    pub decided_by: Option<String>,
    pub reason: Option<String>,
    pub conditions: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicyProps {
    pub name: String,
    pub description: String,
    pub policy_type: Option<String>,
    pub rules: Vec<String>,
    pub applies_to: Option<String>,
    pub active: bool,
    pub priority: u32,
}

impl Default for PolicyProps {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            policy_type: None,
            rules: Vec::new(),
            applies_to: None,
            active: true,
            priority: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecutionProps {
    pub description: String,
    pub agent_uid: Option<String>,
    pub plan_step_uid: Option<String>,
    pub affordance_uid: Option<String>,
    pub status: Option<String>,
    pub started_at: Option<f64>,
    pub completed_at: Option<f64>,
    pub input_snapshot: serde_json::Value,
    pub output_snapshot: serde_json::Value,
    pub error: Option<String>,
    pub side_effects: Vec<String>,
    pub reversible: bool,
    pub rollback_execution_uid: Option<String>,
}

impl Default for ExecutionProps {
    fn default() -> Self {
        Self {
            description: String::new(),
            agent_uid: None,
            plan_step_uid: None,
            affordance_uid: None,
            status: None,
            started_at: None,
            completed_at: None,
            input_snapshot: serde_json::json!({}),
            output_snapshot: serde_json::json!({}),
            error: None,
            side_effects: Vec::new(),
            reversible: true,
            rollback_execution_uid: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyBudgetProps {
    pub scope: Option<String>,
    pub scope_uid: Option<String>,
    pub budget_type: Option<String>,
    pub limit: f64,
    pub consumed: f64,
    pub remaining: f64,
    pub on_exhaustion: Option<String>,
}

impl Default for SafetyBudgetProps {
    fn default() -> Self {
        Self {
            scope: None,
            scope_uid: None,
            budget_type: None,
            limit: 100.0,
            consumed: 0.0,
            remaining: 100.0,
            on_exhaustion: None,
        }
    }
}
