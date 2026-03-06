use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::schema::props::*;
use crate::schema::{Layer, NodeType};

/// Type-safe discriminated union of all node properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum NodeProps {
    // Reality
    Source(SourceProps),
    Snippet(SnippetProps),
    Entity(EntityProps),
    Observation(ObservationProps),

    // Epistemic
    Claim(ClaimProps),
    Evidence(EvidenceProps),
    Warrant(WarrantProps),
    Argument(ArgumentProps),
    Hypothesis(HypothesisProps),
    Theory(TheoryProps),
    Paradigm(ParadigmProps),
    Anomaly(AnomalyProps),
    Method(MethodProps),
    Experiment(ExperimentProps),
    Concept(ConceptProps),
    Assumption(AssumptionProps),
    Question(QuestionProps),
    OpenQuestion(OpenQuestionProps),
    Analogy(AnalogyProps),
    Pattern(PatternProps),
    Mechanism(MechanismProps),
    Model(ModelProps),
    ModelEvaluation(ModelEvaluationProps),
    InferenceChain(InferenceChainProps),
    SensitivityAnalysis(SensitivityAnalysisProps),
    ReasoningStrategy(ReasoningStrategyProps),
    Theorem(TheoremProps),
    Equation(EquationProps),

    // Intent
    Goal(GoalProps),
    Project(ProjectProps),
    Decision(DecisionProps),
    Option(OptionProps),
    Constraint(ConstraintProps),
    Milestone(MilestoneProps),

    // Action
    Affordance(AffordanceProps),
    Flow(FlowProps),
    FlowStep(FlowStepProps),
    Control(ControlProps),
    RiskAssessment(RiskAssessmentProps),

    // Memory
    Session(SessionProps),
    Trace(TraceProps),
    Summary(SummaryProps),
    Preference(PreferenceProps),
    MemoryPolicy(MemoryPolicyProps),
    Journal(JournalProps),

    // Agent
    Agent(AgentProps),
    Task(TaskProps),
    Plan(PlanProps),
    PlanStep(PlanStepProps),
    Approval(ApprovalProps),
    Policy(PolicyProps),
    Execution(ExecutionProps),
    SafetyBudget(SafetyBudgetProps),

    // Extensible
    Custom {
        type_name: String,
        layer: Layer,
        data: serde_json::Value,
    },
}

impl NodeProps {
    /// Returns the NodeType corresponding to this props variant.
    pub fn node_type(&self) -> NodeType {
        match self {
            NodeProps::Source(_) => NodeType::Source,
            NodeProps::Snippet(_) => NodeType::Snippet,
            NodeProps::Entity(_) => NodeType::Entity,
            NodeProps::Observation(_) => NodeType::Observation,
            NodeProps::Claim(_) => NodeType::Claim,
            NodeProps::Evidence(_) => NodeType::Evidence,
            NodeProps::Warrant(_) => NodeType::Warrant,
            NodeProps::Argument(_) => NodeType::Argument,
            NodeProps::Hypothesis(_) => NodeType::Hypothesis,
            NodeProps::Theory(_) => NodeType::Theory,
            NodeProps::Paradigm(_) => NodeType::Paradigm,
            NodeProps::Anomaly(_) => NodeType::Anomaly,
            NodeProps::Method(_) => NodeType::Method,
            NodeProps::Experiment(_) => NodeType::Experiment,
            NodeProps::Concept(_) => NodeType::Concept,
            NodeProps::Assumption(_) => NodeType::Assumption,
            NodeProps::Question(_) => NodeType::Question,
            NodeProps::OpenQuestion(_) => NodeType::OpenQuestion,
            NodeProps::Analogy(_) => NodeType::Analogy,
            NodeProps::Pattern(_) => NodeType::Pattern,
            NodeProps::Mechanism(_) => NodeType::Mechanism,
            NodeProps::Model(_) => NodeType::Model,
            NodeProps::ModelEvaluation(_) => NodeType::ModelEvaluation,
            NodeProps::InferenceChain(_) => NodeType::InferenceChain,
            NodeProps::SensitivityAnalysis(_) => NodeType::SensitivityAnalysis,
            NodeProps::ReasoningStrategy(_) => NodeType::ReasoningStrategy,
            NodeProps::Theorem(_) => NodeType::Theorem,
            NodeProps::Equation(_) => NodeType::Equation,
            NodeProps::Goal(_) => NodeType::Goal,
            NodeProps::Project(_) => NodeType::Project,
            NodeProps::Decision(_) => NodeType::Decision,
            NodeProps::Option(_) => NodeType::Option,
            NodeProps::Constraint(_) => NodeType::Constraint,
            NodeProps::Milestone(_) => NodeType::Milestone,
            NodeProps::Affordance(_) => NodeType::Affordance,
            NodeProps::Flow(_) => NodeType::Flow,
            NodeProps::FlowStep(_) => NodeType::FlowStep,
            NodeProps::Control(_) => NodeType::Control,
            NodeProps::RiskAssessment(_) => NodeType::RiskAssessment,
            NodeProps::Session(_) => NodeType::Session,
            NodeProps::Trace(_) => NodeType::Trace,
            NodeProps::Summary(_) => NodeType::Summary,
            NodeProps::Preference(_) => NodeType::Preference,
            NodeProps::MemoryPolicy(_) => NodeType::MemoryPolicy,
            NodeProps::Journal(_) => NodeType::Journal,
            NodeProps::Agent(_) => NodeType::Agent,
            NodeProps::Task(_) => NodeType::Task,
            NodeProps::Plan(_) => NodeType::Plan,
            NodeProps::PlanStep(_) => NodeType::PlanStep,
            NodeProps::Approval(_) => NodeType::Approval,
            NodeProps::Policy(_) => NodeType::Policy,
            NodeProps::Execution(_) => NodeType::Execution,
            NodeProps::SafetyBudget(_) => NodeType::SafetyBudget,
            NodeProps::Custom { type_name, .. } => NodeType::Custom(type_name.clone()),
        }
    }

    /// Serialize the inner props to a JSON value (without the tag).
    pub fn to_json(&self) -> serde_json::Value {
        self.try_to_json_untagged().unwrap_or_default()
    }

    /// Try to serialize the inner props to a JSON value (without the tag).
    pub fn try_to_json_untagged(&self) -> crate::Result<serde_json::Value> {
        if let NodeProps::Custom { data, .. } = self {
            return Ok(data.clone());
        }
        let mut v = serde_json::to_value(self)?;
        if let serde_json::Value::Object(ref mut map) = v {
            map.remove("_type");
        }
        Ok(v)
    }

    /// Try to serialize the inner props to a JSON value (with the tag).
    pub fn try_to_json(&self) -> crate::Result<serde_json::Value> {
        serde_json::to_value(self).map_err(crate::Error::from)
    }

    /// Deserialize props from JSON using the node type as discriminator.
    pub fn from_json(node_type: &NodeType, value: &serde_json::Value) -> crate::Result<Self> {
        fn de<T: serde::de::DeserializeOwned>(v: &serde_json::Value) -> crate::Result<T> {
            serde_json::from_value(v.clone()).map_err(crate::Error::from)
        }
        match node_type {
            NodeType::Source => Ok(NodeProps::Source(de(value)?)),
            NodeType::Snippet => Ok(NodeProps::Snippet(de(value)?)),
            NodeType::Entity => Ok(NodeProps::Entity(de(value)?)),
            NodeType::Observation => Ok(NodeProps::Observation(de(value)?)),
            NodeType::Claim => Ok(NodeProps::Claim(de(value)?)),
            NodeType::Evidence => Ok(NodeProps::Evidence(de(value)?)),
            NodeType::Warrant => Ok(NodeProps::Warrant(de(value)?)),
            NodeType::Argument => Ok(NodeProps::Argument(de(value)?)),
            NodeType::Hypothesis => Ok(NodeProps::Hypothesis(de(value)?)),
            NodeType::Theory => Ok(NodeProps::Theory(de(value)?)),
            NodeType::Paradigm => Ok(NodeProps::Paradigm(de(value)?)),
            NodeType::Anomaly => Ok(NodeProps::Anomaly(de(value)?)),
            NodeType::Method => Ok(NodeProps::Method(de(value)?)),
            NodeType::Experiment => Ok(NodeProps::Experiment(de(value)?)),
            NodeType::Concept => Ok(NodeProps::Concept(de(value)?)),
            NodeType::Assumption => Ok(NodeProps::Assumption(de(value)?)),
            NodeType::Question => Ok(NodeProps::Question(de(value)?)),
            NodeType::OpenQuestion => Ok(NodeProps::OpenQuestion(de(value)?)),
            NodeType::Analogy => Ok(NodeProps::Analogy(de(value)?)),
            NodeType::Pattern => Ok(NodeProps::Pattern(de(value)?)),
            NodeType::Mechanism => Ok(NodeProps::Mechanism(de(value)?)),
            NodeType::Model => Ok(NodeProps::Model(de(value)?)),
            NodeType::ModelEvaluation => Ok(NodeProps::ModelEvaluation(de(value)?)),
            NodeType::InferenceChain => Ok(NodeProps::InferenceChain(de(value)?)),
            NodeType::SensitivityAnalysis => Ok(NodeProps::SensitivityAnalysis(de(value)?)),
            NodeType::ReasoningStrategy => Ok(NodeProps::ReasoningStrategy(de(value)?)),
            NodeType::Theorem => Ok(NodeProps::Theorem(de(value)?)),
            NodeType::Equation => Ok(NodeProps::Equation(de(value)?)),
            NodeType::Goal => Ok(NodeProps::Goal(de(value)?)),
            NodeType::Project => Ok(NodeProps::Project(de(value)?)),
            NodeType::Decision => Ok(NodeProps::Decision(de(value)?)),
            NodeType::Option => Ok(NodeProps::Option(de(value)?)),
            NodeType::Constraint => Ok(NodeProps::Constraint(de(value)?)),
            NodeType::Milestone => Ok(NodeProps::Milestone(de(value)?)),
            NodeType::Affordance => Ok(NodeProps::Affordance(de(value)?)),
            NodeType::Flow => Ok(NodeProps::Flow(de(value)?)),
            NodeType::FlowStep => Ok(NodeProps::FlowStep(de(value)?)),
            NodeType::Control => Ok(NodeProps::Control(de(value)?)),
            NodeType::RiskAssessment => Ok(NodeProps::RiskAssessment(de(value)?)),
            NodeType::Session => Ok(NodeProps::Session(de(value)?)),
            NodeType::Trace => Ok(NodeProps::Trace(de(value)?)),
            NodeType::Summary => Ok(NodeProps::Summary(de(value)?)),
            NodeType::Preference => Ok(NodeProps::Preference(de(value)?)),
            NodeType::MemoryPolicy => Ok(NodeProps::MemoryPolicy(de(value)?)),
            NodeType::Journal => Ok(NodeProps::Journal(de(value)?)),
            NodeType::Agent => Ok(NodeProps::Agent(de(value)?)),
            NodeType::Task => Ok(NodeProps::Task(de(value)?)),
            NodeType::Plan => Ok(NodeProps::Plan(de(value)?)),
            NodeType::PlanStep => Ok(NodeProps::PlanStep(de(value)?)),
            NodeType::Approval => Ok(NodeProps::Approval(de(value)?)),
            NodeType::Policy => Ok(NodeProps::Policy(de(value)?)),
            NodeType::Execution => Ok(NodeProps::Execution(de(value)?)),
            NodeType::SafetyBudget => Ok(NodeProps::SafetyBudget(de(value)?)),
            NodeType::Custom(name) => {
                let layer = value
                    .get("_layer")
                    .and_then(|v| serde_json::from_value::<Layer>(v.clone()).ok())
                    .unwrap_or(Layer::Reality);
                Ok(NodeProps::Custom {
                    type_name: name.clone(),
                    layer,
                    data: value.clone(),
                })
            }
        }
    }

    /// Get the layer for this props variant. For custom types, returns the stored layer.
    pub fn layer(&self) -> Layer {
        match self {
            NodeProps::Custom { layer, .. } => *layer,
            other => other.node_type().layer(),
        }
    }

    /// Extract searchable text content from these props.
    ///
    /// Returns a concatenation of all user-authored text fields (content,
    /// description, name, etc.) suitable for full-text search indexing.
    /// Metadata fields (status, type, timestamps) are excluded.
    pub fn search_text(&self) -> String {
        let json = self.to_json();
        let obj = match json.as_object() {
            Some(o) => o,
            None => return String::new(),
        };

        // Collect text from known searchable field names
        let text_fields = [
            "content",
            "description",
            "canonical_name",
            "title",
            "uri",
            "name",
            "principle",
            "statement",
            "definition",
            "text",
            "question",
            "expression",
            "justification",
            "vulnerability",
            "weakest_link",
            "original_expectation",
            "decision_rationale",
            "action_name",
            "fallback_description",
            "focus_summary",
            "key",
            "value",
            "condition",
            "action",
            "result_summary",
            "expected_outcome",
            "actual_outcome",
            "reason",
            "conditions",
            "applies_to",
            "error",
            "input",
            "output",
            "mitigation",
            "sample_description",
        ];

        let vec_fields = [
            "aliases",
            "predicted_observations",
            "core_commitments",
            "predictive_successes",
            "core_assumptions",
            "exemplar_problems",
            "accepted_methods",
            "limitations",
            "validity_conditions",
            "parameters",
            "variables_manipulated",
            "variables_measured",
            "controls",
            "alternative_definitions",
            "blocking_factors",
            "mapping_elements",
            "domains_observed",
            "components",
            "interactions",
            "key_parameters",
            "simplifications",
            "metrics",
            "failure_domains",
            "comparison_to",
            "critical_assumptions",
            "sensitivity_map",
            "critical_inputs",
            "break_points",
            "applicable_contexts",
            "applications",
            "variables",
            "assumptions",
            "success_criteria",
            "pros",
            "cons",
            "criteria",
            "preconditions",
            "postconditions",
            "capabilities",
            "domain_restrictions",
            "rules",
            "side_effects",
        ];

        let mut parts = Vec::new();

        for field in &text_fields {
            if let Some(serde_json::Value::String(s)) = obj.get(*field) {
                if !s.is_empty() {
                    parts.push(s.as_str());
                }
            }
        }

        for field in &vec_fields {
            if let Some(serde_json::Value::Array(arr)) = obj.get(*field) {
                for item in arr {
                    if let serde_json::Value::String(s) = item {
                        if !s.is_empty() {
                            parts.push(s.as_str());
                        }
                    }
                }
            }
        }

        parts.join(" ")
    }

    /// Returns the set of known field names for the given node type.
    ///
    /// Serializes a default instance of the props struct to JSON and extracts the keys.
    /// For `Custom` types, returns an empty set (all fields are allowed).
    pub fn known_fields_for_type(node_type: &NodeType) -> HashSet<String> {
        fn fields_of<T: Default + Serialize>() -> HashSet<String> {
            serde_json::to_value(T::default())
                .ok()
                .and_then(|v| v.as_object().map(|m| m.keys().cloned().collect()))
                .unwrap_or_default()
        }
        match node_type {
            NodeType::Source => fields_of::<SourceProps>(),
            NodeType::Snippet => fields_of::<SnippetProps>(),
            NodeType::Entity => fields_of::<EntityProps>(),
            NodeType::Observation => fields_of::<ObservationProps>(),
            NodeType::Claim => fields_of::<ClaimProps>(),
            NodeType::Evidence => fields_of::<EvidenceProps>(),
            NodeType::Warrant => fields_of::<WarrantProps>(),
            NodeType::Argument => fields_of::<ArgumentProps>(),
            NodeType::Hypothesis => fields_of::<HypothesisProps>(),
            NodeType::Theory => fields_of::<TheoryProps>(),
            NodeType::Paradigm => fields_of::<ParadigmProps>(),
            NodeType::Anomaly => fields_of::<AnomalyProps>(),
            NodeType::Method => fields_of::<MethodProps>(),
            NodeType::Experiment => fields_of::<ExperimentProps>(),
            NodeType::Concept => fields_of::<ConceptProps>(),
            NodeType::Assumption => fields_of::<AssumptionProps>(),
            NodeType::Question => fields_of::<QuestionProps>(),
            NodeType::OpenQuestion => fields_of::<OpenQuestionProps>(),
            NodeType::Analogy => fields_of::<AnalogyProps>(),
            NodeType::Pattern => fields_of::<PatternProps>(),
            NodeType::Mechanism => fields_of::<MechanismProps>(),
            NodeType::Model => fields_of::<ModelProps>(),
            NodeType::ModelEvaluation => fields_of::<ModelEvaluationProps>(),
            NodeType::InferenceChain => fields_of::<InferenceChainProps>(),
            NodeType::SensitivityAnalysis => fields_of::<SensitivityAnalysisProps>(),
            NodeType::ReasoningStrategy => fields_of::<ReasoningStrategyProps>(),
            NodeType::Theorem => fields_of::<TheoremProps>(),
            NodeType::Equation => fields_of::<EquationProps>(),
            NodeType::Goal => fields_of::<GoalProps>(),
            NodeType::Project => fields_of::<ProjectProps>(),
            NodeType::Decision => fields_of::<DecisionProps>(),
            NodeType::Option => fields_of::<OptionProps>(),
            NodeType::Constraint => fields_of::<ConstraintProps>(),
            NodeType::Milestone => fields_of::<MilestoneProps>(),
            NodeType::Affordance => fields_of::<AffordanceProps>(),
            NodeType::Flow => fields_of::<FlowProps>(),
            NodeType::FlowStep => fields_of::<FlowStepProps>(),
            NodeType::Control => fields_of::<ControlProps>(),
            NodeType::RiskAssessment => fields_of::<RiskAssessmentProps>(),
            NodeType::Session => fields_of::<SessionProps>(),
            NodeType::Trace => fields_of::<TraceProps>(),
            NodeType::Summary => fields_of::<SummaryProps>(),
            NodeType::Preference => fields_of::<PreferenceProps>(),
            NodeType::MemoryPolicy => fields_of::<MemoryPolicyProps>(),
            NodeType::Journal => fields_of::<JournalProps>(),
            NodeType::Agent => fields_of::<AgentProps>(),
            NodeType::Task => fields_of::<TaskProps>(),
            NodeType::Plan => fields_of::<PlanProps>(),
            NodeType::PlanStep => fields_of::<PlanStepProps>(),
            NodeType::Approval => fields_of::<ApprovalProps>(),
            NodeType::Policy => fields_of::<PolicyProps>(),
            NodeType::Execution => fields_of::<ExecutionProps>(),
            NodeType::SafetyBudget => fields_of::<SafetyBudgetProps>(),
            NodeType::Custom(_) => HashSet::new(),
        }
    }

    /// Validate a JSON patch object against the known fields for this node type.
    ///
    /// Returns `Ok(())` if all patch keys are valid fields. Returns `Err` with the
    /// list of unknown field names if any keys don't match.
    pub fn validate_patch(
        node_type: &NodeType,
        patch: &serde_json::Value,
    ) -> std::result::Result<(), Vec<String>> {
        // Custom types allow any fields
        if matches!(node_type, NodeType::Custom(_)) {
            return Ok(());
        }
        let known = Self::known_fields_for_type(node_type);
        if known.is_empty() {
            return Ok(());
        }
        if let Some(obj) = patch.as_object() {
            let unknown: Vec<String> = obj
                .keys()
                .filter(|k| !known.contains(*k))
                .cloned()
                .collect();
            if unknown.is_empty() {
                Ok(())
            } else {
                Err(unknown)
            }
        } else {
            Ok(())
        }
    }
}
