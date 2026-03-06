pub mod edge;
pub mod edge_props;
pub mod node;
pub mod node_props;
pub mod props;

use serde::{Deserialize, Serialize};

/// The six conceptual layers of the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    Reality,
    Epistemic,
    Intent,
    Action,
    Memory,
    Agent,
}

impl Layer {
    pub fn as_str(&self) -> &str {
        match self {
            Layer::Reality => "reality",
            Layer::Epistemic => "epistemic",
            Layer::Intent => "intent",
            Layer::Action => "action",
            Layer::Memory => "memory",
            Layer::Agent => "agent",
        }
    }
}

/// All node types across the six layers, plus extensible custom types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // Reality (4)
    Source,
    Snippet,
    Entity,
    Observation,

    // Epistemic (24)
    Claim,
    Evidence,
    Warrant,
    Argument,
    Hypothesis,
    Theory,
    Paradigm,
    Anomaly,
    Method,
    Experiment,
    Concept,
    Assumption,
    Question,
    OpenQuestion,
    Analogy,
    Pattern,
    Mechanism,
    Model,
    ModelEvaluation,
    InferenceChain,
    SensitivityAnalysis,
    ReasoningStrategy,
    Theorem,
    Equation,

    // Intent (6)
    Goal,
    Project,
    Decision,
    Option,
    Constraint,
    Milestone,

    // Action (5)
    Affordance,
    Flow,
    FlowStep,
    Control,
    RiskAssessment,

    // Memory (6)
    Session,
    Trace,
    Summary,
    Preference,
    MemoryPolicy,
    Journal,

    // Agent (8)
    Agent,
    Task,
    Plan,
    PlanStep,
    Approval,
    Policy,
    Execution,
    SafetyBudget,

    // Extensible
    Custom(String),
}

impl NodeType {
    pub fn layer(&self) -> Layer {
        match self {
            // Reality
            NodeType::Source | NodeType::Snippet | NodeType::Entity | NodeType::Observation => {
                Layer::Reality
            }

            // Epistemic
            NodeType::Claim
            | NodeType::Evidence
            | NodeType::Warrant
            | NodeType::Argument
            | NodeType::Hypothesis
            | NodeType::Theory
            | NodeType::Paradigm
            | NodeType::Anomaly
            | NodeType::Method
            | NodeType::Experiment
            | NodeType::Concept
            | NodeType::Assumption
            | NodeType::Question
            | NodeType::OpenQuestion
            | NodeType::Analogy
            | NodeType::Pattern
            | NodeType::Mechanism
            | NodeType::Model
            | NodeType::ModelEvaluation
            | NodeType::InferenceChain
            | NodeType::SensitivityAnalysis
            | NodeType::ReasoningStrategy
            | NodeType::Theorem
            | NodeType::Equation => Layer::Epistemic,

            // Intent
            NodeType::Goal
            | NodeType::Project
            | NodeType::Decision
            | NodeType::Option
            | NodeType::Constraint
            | NodeType::Milestone => Layer::Intent,

            // Action
            NodeType::Affordance
            | NodeType::Flow
            | NodeType::FlowStep
            | NodeType::Control
            | NodeType::RiskAssessment => Layer::Action,

            // Memory
            NodeType::Session
            | NodeType::Trace
            | NodeType::Summary
            | NodeType::Preference
            | NodeType::MemoryPolicy
            | NodeType::Journal => Layer::Memory,

            // Agent
            NodeType::Agent
            | NodeType::Task
            | NodeType::Plan
            | NodeType::PlanStep
            | NodeType::Approval
            | NodeType::Policy
            | NodeType::Execution
            | NodeType::SafetyBudget => Layer::Agent,

            // Custom types default to Reality; callers override via NodeProps::Custom { layer, .. }
            NodeType::Custom(_) => Layer::Reality,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            NodeType::Source => "Source",
            NodeType::Snippet => "Snippet",
            NodeType::Entity => "Entity",
            NodeType::Observation => "Observation",
            NodeType::Claim => "Claim",
            NodeType::Evidence => "Evidence",
            NodeType::Warrant => "Warrant",
            NodeType::Argument => "Argument",
            NodeType::Hypothesis => "Hypothesis",
            NodeType::Theory => "Theory",
            NodeType::Paradigm => "Paradigm",
            NodeType::Anomaly => "Anomaly",
            NodeType::Method => "Method",
            NodeType::Experiment => "Experiment",
            NodeType::Concept => "Concept",
            NodeType::Assumption => "Assumption",
            NodeType::Question => "Question",
            NodeType::OpenQuestion => "OpenQuestion",
            NodeType::Analogy => "Analogy",
            NodeType::Pattern => "Pattern",
            NodeType::Mechanism => "Mechanism",
            NodeType::Model => "Model",
            NodeType::ModelEvaluation => "ModelEvaluation",
            NodeType::InferenceChain => "InferenceChain",
            NodeType::SensitivityAnalysis => "SensitivityAnalysis",
            NodeType::ReasoningStrategy => "ReasoningStrategy",
            NodeType::Theorem => "Theorem",
            NodeType::Equation => "Equation",
            NodeType::Goal => "Goal",
            NodeType::Project => "Project",
            NodeType::Decision => "Decision",
            NodeType::Option => "Option",
            NodeType::Constraint => "Constraint",
            NodeType::Milestone => "Milestone",
            NodeType::Affordance => "Affordance",
            NodeType::Flow => "Flow",
            NodeType::FlowStep => "FlowStep",
            NodeType::Control => "Control",
            NodeType::RiskAssessment => "RiskAssessment",
            NodeType::Session => "Session",
            NodeType::Trace => "Trace",
            NodeType::Summary => "Summary",
            NodeType::Preference => "Preference",
            NodeType::MemoryPolicy => "MemoryPolicy",
            NodeType::Journal => "Journal",
            NodeType::Agent => "Agent",
            NodeType::Task => "Task",
            NodeType::Plan => "Plan",
            NodeType::PlanStep => "PlanStep",
            NodeType::Approval => "Approval",
            NodeType::Policy => "Policy",
            NodeType::Execution => "Execution",
            NodeType::SafetyBudget => "SafetyBudget",
            NodeType::Custom(name) => name.as_str(),
        }
    }

    /// Returns true if this is a user-defined custom type.
    pub fn is_custom(&self) -> bool {
        matches!(self, NodeType::Custom(_))
    }

    pub fn requires_provenance(&self) -> bool {
        self.layer() == Layer::Epistemic
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// All edge types across layers, plus extensible custom types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    // Structural (5)
    ExtractedFrom,
    PartOf,
    HasPart,
    InstanceOf,
    Contains,

    // Epistemic (31)
    Supports,
    Refutes,
    Justifies,
    HasPremise,
    HasConclusion,
    HasWarrant,
    Rebuts,
    Assumes,
    Tests,
    Produces,
    UsesMethod,
    Addresses,
    Generates,
    Extends,
    Supersedes,
    Contradicts,
    AnomalousTo,
    AnalogousTo,
    Instantiates,
    TransfersTo,
    Evaluates,
    Outperforms,
    FailsOn,
    HasChainStep,
    PropagatesUncertaintyTo,
    SensitiveTo,
    RobustAcross,
    Describes,
    DerivedFrom,
    ReliesOn,
    ProvenBy,

    // Provenance (5)
    ProposedBy,
    AuthoredBy,
    CitedBy,
    BelievedBy,
    ConsensusIn,

    // Intent (9)
    DecomposesInto,
    MotivatedBy,
    HasOption,
    DecidedOn,
    ConstrainedBy,
    Blocks,
    Informs,
    RelevantTo,
    DependsOn,

    // Action (5)
    AvailableOn,
    ComposedOf,
    StepUses,
    RiskAssessedBy,
    Controls,

    // Memory (5)
    CapturedIn,
    TraceEntry,
    Summarizes,
    Recalls,
    GovernedBy,

    // Agent (10)
    AssignedTo,
    PlannedBy,
    HasStep,
    Targets,
    RequiresApproval,
    ExecutedBy,
    ExecutionOf,
    ProducesNode,
    GovernedByPolicy,
    BudgetFor,

    // Temporal
    Follows,

    // Social
    WorksFor,
    AffiliatedWith,
    About,
    KnownBy,

    // Extensible
    Custom(String),
}

impl EdgeType {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeType::ExtractedFrom => "EXTRACTED_FROM",
            EdgeType::PartOf => "PART_OF",
            EdgeType::HasPart => "HAS_PART",
            EdgeType::InstanceOf => "INSTANCE_OF",
            EdgeType::Contains => "CONTAINS",
            EdgeType::Supports => "SUPPORTS",
            EdgeType::Refutes => "REFUTES",
            EdgeType::Justifies => "JUSTIFIES",
            EdgeType::HasPremise => "HAS_PREMISE",
            EdgeType::HasConclusion => "HAS_CONCLUSION",
            EdgeType::HasWarrant => "HAS_WARRANT",
            EdgeType::Rebuts => "REBUTS",
            EdgeType::Assumes => "ASSUMES",
            EdgeType::Tests => "TESTS",
            EdgeType::Produces => "PRODUCES",
            EdgeType::UsesMethod => "USES_METHOD",
            EdgeType::Addresses => "ADDRESSES",
            EdgeType::Generates => "GENERATES",
            EdgeType::Extends => "EXTENDS",
            EdgeType::Supersedes => "SUPERSEDES",
            EdgeType::Contradicts => "CONTRADICTS",
            EdgeType::AnomalousTo => "ANOMALOUS_TO",
            EdgeType::AnalogousTo => "ANALOGOUS_TO",
            EdgeType::Instantiates => "INSTANTIATES",
            EdgeType::TransfersTo => "TRANSFERS_TO",
            EdgeType::Evaluates => "EVALUATES",
            EdgeType::Outperforms => "OUTPERFORMS",
            EdgeType::FailsOn => "FAILS_ON",
            EdgeType::HasChainStep => "HAS_CHAIN_STEP",
            EdgeType::PropagatesUncertaintyTo => "PROPAGATES_UNCERTAINTY_TO",
            EdgeType::SensitiveTo => "SENSITIVE_TO",
            EdgeType::RobustAcross => "ROBUST_ACROSS",
            EdgeType::Describes => "DESCRIBES",
            EdgeType::DerivedFrom => "DERIVED_FROM",
            EdgeType::ReliesOn => "RELIES_ON",
            EdgeType::ProvenBy => "PROVEN_BY",
            EdgeType::ProposedBy => "PROPOSED_BY",
            EdgeType::AuthoredBy => "AUTHORED_BY",
            EdgeType::CitedBy => "CITED_BY",
            EdgeType::BelievedBy => "BELIEVED_BY",
            EdgeType::ConsensusIn => "CONSENSUS_IN",
            EdgeType::DecomposesInto => "DECOMPOSES_INTO",
            EdgeType::MotivatedBy => "MOTIVATED_BY",
            EdgeType::HasOption => "HAS_OPTION",
            EdgeType::DecidedOn => "DECIDED_ON",
            EdgeType::ConstrainedBy => "CONSTRAINED_BY",
            EdgeType::Blocks => "BLOCKS",
            EdgeType::Informs => "INFORMS",
            EdgeType::RelevantTo => "RELEVANT_TO",
            EdgeType::DependsOn => "DEPENDS_ON",
            EdgeType::AvailableOn => "AVAILABLE_ON",
            EdgeType::ComposedOf => "COMPOSED_OF",
            EdgeType::StepUses => "STEP_USES",
            EdgeType::RiskAssessedBy => "RISK_ASSESSED_BY",
            EdgeType::Controls => "CONTROLS",
            EdgeType::CapturedIn => "CAPTURED_IN",
            EdgeType::TraceEntry => "TRACE_ENTRY",
            EdgeType::Summarizes => "SUMMARIZES",
            EdgeType::Recalls => "RECALLS",
            EdgeType::GovernedBy => "GOVERNED_BY",
            EdgeType::AssignedTo => "ASSIGNED_TO",
            EdgeType::PlannedBy => "PLANNED_BY",
            EdgeType::HasStep => "HAS_STEP",
            EdgeType::Targets => "TARGETS",
            EdgeType::RequiresApproval => "REQUIRES_APPROVAL",
            EdgeType::ExecutedBy => "EXECUTED_BY",
            EdgeType::ExecutionOf => "EXECUTION_OF",
            EdgeType::ProducesNode => "PRODUCES_NODE",
            EdgeType::GovernedByPolicy => "GOVERNED_BY_POLICY",
            EdgeType::BudgetFor => "BUDGET_FOR",
            EdgeType::Follows => "FOLLOWS",
            EdgeType::WorksFor => "WORKS_FOR",
            EdgeType::AffiliatedWith => "AFFILIATED_WITH",
            EdgeType::About => "ABOUT",
            EdgeType::KnownBy => "KNOWN_BY",
            EdgeType::Custom(name) => name.as_str(),
        }
    }

    /// Returns true if this is a user-defined custom type.
    pub fn is_custom(&self) -> bool {
        matches!(self, EdgeType::Custom(_))
    }
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Trait for compile-time registration of custom node types.
///
/// Implement this trait on a struct to use it with [`MindGraph::add_custom_node`](crate::MindGraph::add_custom_node).
///
/// # Example
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use mindgraph::{CustomNodeType, Layer};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct CodeSnippet {
///     language: String,
///     code: String,
/// }
///
/// impl CustomNodeType for CodeSnippet {
///     fn type_name() -> &'static str { "CodeSnippet" }
///     fn layer() -> Layer { Layer::Reality }
/// }
/// ```
pub trait CustomNodeType: serde::Serialize + serde::de::DeserializeOwned + Send + Sync {
    /// The string name stored in the database (e.g. `"CodeSnippet"`).
    fn type_name() -> &'static str;
    /// The layer this custom type belongs to.
    fn layer() -> Layer;
}
