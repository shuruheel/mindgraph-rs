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
}
