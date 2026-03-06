use serde::{Deserialize, Serialize};

use crate::schema::EdgeType;

/// Type-safe discriminated union of all edge properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum EdgeProps {
    // Structural
    ExtractedFrom {
        location: Option<String>,
        method: Option<String>,
        confidence: Option<f64>,
    },
    PartOf {
        role: Option<String>,
    },
    HasPart {
        role: Option<String>,
    },
    InstanceOf {},
    Contains {},

    // Epistemic
    Supports {
        strength: Option<f64>,
        support_type: Option<String>,
    },
    Refutes {
        strength: Option<f64>,
        refutation_type: Option<String>,
    },
    Justifies {
        necessity: Option<String>,
    },
    HasPremise {
        order: Option<i64>,
        role: Option<String>,
    },
    HasConclusion {},
    HasWarrant {},
    Rebuts {
        defeat_type: Option<String>,
        rebuttal_type: Option<String>,
        content: Option<String>,
        strength: Option<f64>,
    },
    Assumes {
        necessity: Option<String>,
    },
    Tests {
        outcome: Option<String>,
        test_type: Option<String>,
    },
    Produces {},
    UsesMethod {
        variant: Option<String>,
    },
    Addresses {
        completeness: Option<String>,
    },
    Generates {},
    Extends {
        extension_type: Option<String>,
    },
    Supersedes {
        supersession_type: Option<String>,
    },
    Contradicts {
        description: Option<String>,
        contradiction_type: Option<String>,
        resolution_status: Option<String>,
        proposed_resolution: Option<String>,
    },
    AnomalousTo {
        severity: Option<String>,
    },
    AnalogousTo {
        mapping_type: Option<String>,
        strength: Option<f64>,
    },
    Instantiates {},
    TransfersTo {
        success: Option<bool>,
        adaptations: Option<String>,
    },
    Evaluates {
        evaluation_context: Option<String>,
    },
    Outperforms {
        metric: Option<String>,
        margin: Option<f64>,
        conditions: Option<String>,
    },
    FailsOn {
        failure_mode: Option<String>,
        severity: Option<String>,
    },
    HasChainStep {
        order: Option<i64>,
        step_confidence: Option<f64>,
        cumulative_confidence: Option<f64>,
    },
    PropagatesUncertaintyTo {
        propagation_factor: Option<f64>,
        propagation_type: Option<String>,
    },
    SensitiveTo {
        elasticity: Option<f64>,
        direction: Option<String>,
        threshold: Option<f64>,
    },
    RobustAcross {
        variation_type: Option<String>,
        range_tested: Option<String>,
    },
    Describes {},
    DerivedFrom {},
    ReliesOn {},
    ProvenBy {},

    // Provenance
    ProposedBy {
        date: Option<String>,
        context: Option<String>,
    },
    AuthoredBy {
        position: Option<String>,
        contribution: Option<String>,
    },
    CitedBy {
        citation_type: Option<String>,
        context: Option<String>,
    },
    BelievedBy {
        confidence: Option<f64>,
        basis: Option<String>,
        as_of_date: Option<String>,
    },
    ConsensusIn {
        consensus_level: Option<String>,
        as_of_date: Option<String>,
    },

    // Intent
    DecomposesInto {
        order: Option<i64>,
    },
    MotivatedBy {},
    HasOption {},
    DecidedOn {
        rationale: Option<String>,
    },
    ConstrainedBy {},
    Blocks {},
    Informs {
        relevance: Option<f64>,
    },
    RelevantTo {
        relevance_score: Option<f64>,
    },
    DependsOn {
        dependency_type: Option<String>,
    },

    // Action
    AvailableOn {},
    ComposedOf {
        order: Option<i64>,
    },
    StepUses {},
    RiskAssessedBy {},
    Controls {
        interaction_type: Option<String>,
    },

    // Memory
    CapturedIn {
        position: Option<i64>,
    },
    TraceEntry {
        order: Option<i64>,
    },
    Summarizes {},
    Recalls {},
    GovernedBy {},

    // Agent
    AssignedTo {
        assigned_at: Option<f64>,
    },
    PlannedBy {},
    HasStep {
        order: Option<i64>,
    },
    Targets {},
    RequiresApproval {},
    ExecutedBy {},
    ExecutionOf {},
    ProducesNode {},
    GovernedByPolicy {},
    BudgetFor {},

    // Temporal
    Follows {},

    // Entity relations
    WorksFor {},
    AffiliatedWith {},
    About {},
    KnownBy {},

    // Extensible
    Custom {
        type_name: String,
        data: serde_json::Value,
    },
}

impl EdgeProps {
    /// Returns the EdgeType corresponding to this props variant.
    pub fn edge_type(&self) -> EdgeType {
        match self {
            EdgeProps::ExtractedFrom { .. } => EdgeType::ExtractedFrom,
            EdgeProps::PartOf { .. } => EdgeType::PartOf,
            EdgeProps::HasPart { .. } => EdgeType::HasPart,
            EdgeProps::InstanceOf { .. } => EdgeType::InstanceOf,
            EdgeProps::Contains { .. } => EdgeType::Contains,
            EdgeProps::Supports { .. } => EdgeType::Supports,
            EdgeProps::Refutes { .. } => EdgeType::Refutes,
            EdgeProps::Justifies { .. } => EdgeType::Justifies,
            EdgeProps::HasPremise { .. } => EdgeType::HasPremise,
            EdgeProps::HasConclusion { .. } => EdgeType::HasConclusion,
            EdgeProps::HasWarrant { .. } => EdgeType::HasWarrant,
            EdgeProps::Rebuts { .. } => EdgeType::Rebuts,
            EdgeProps::Assumes { .. } => EdgeType::Assumes,
            EdgeProps::Tests { .. } => EdgeType::Tests,
            EdgeProps::Produces { .. } => EdgeType::Produces,
            EdgeProps::UsesMethod { .. } => EdgeType::UsesMethod,
            EdgeProps::Addresses { .. } => EdgeType::Addresses,
            EdgeProps::Generates { .. } => EdgeType::Generates,
            EdgeProps::Extends { .. } => EdgeType::Extends,
            EdgeProps::Supersedes { .. } => EdgeType::Supersedes,
            EdgeProps::Contradicts { .. } => EdgeType::Contradicts,
            EdgeProps::AnomalousTo { .. } => EdgeType::AnomalousTo,
            EdgeProps::AnalogousTo { .. } => EdgeType::AnalogousTo,
            EdgeProps::Instantiates { .. } => EdgeType::Instantiates,
            EdgeProps::TransfersTo { .. } => EdgeType::TransfersTo,
            EdgeProps::Evaluates { .. } => EdgeType::Evaluates,
            EdgeProps::Outperforms { .. } => EdgeType::Outperforms,
            EdgeProps::FailsOn { .. } => EdgeType::FailsOn,
            EdgeProps::HasChainStep { .. } => EdgeType::HasChainStep,
            EdgeProps::PropagatesUncertaintyTo { .. } => EdgeType::PropagatesUncertaintyTo,
            EdgeProps::SensitiveTo { .. } => EdgeType::SensitiveTo,
            EdgeProps::RobustAcross { .. } => EdgeType::RobustAcross,
            EdgeProps::Describes { .. } => EdgeType::Describes,
            EdgeProps::DerivedFrom { .. } => EdgeType::DerivedFrom,
            EdgeProps::ReliesOn { .. } => EdgeType::ReliesOn,
            EdgeProps::ProvenBy { .. } => EdgeType::ProvenBy,
            EdgeProps::ProposedBy { .. } => EdgeType::ProposedBy,
            EdgeProps::AuthoredBy { .. } => EdgeType::AuthoredBy,
            EdgeProps::CitedBy { .. } => EdgeType::CitedBy,
            EdgeProps::BelievedBy { .. } => EdgeType::BelievedBy,
            EdgeProps::ConsensusIn { .. } => EdgeType::ConsensusIn,
            EdgeProps::DecomposesInto { .. } => EdgeType::DecomposesInto,
            EdgeProps::MotivatedBy { .. } => EdgeType::MotivatedBy,
            EdgeProps::HasOption { .. } => EdgeType::HasOption,
            EdgeProps::DecidedOn { .. } => EdgeType::DecidedOn,
            EdgeProps::ConstrainedBy { .. } => EdgeType::ConstrainedBy,
            EdgeProps::Blocks { .. } => EdgeType::Blocks,
            EdgeProps::Informs { .. } => EdgeType::Informs,
            EdgeProps::RelevantTo { .. } => EdgeType::RelevantTo,
            EdgeProps::DependsOn { .. } => EdgeType::DependsOn,
            EdgeProps::AvailableOn { .. } => EdgeType::AvailableOn,
            EdgeProps::ComposedOf { .. } => EdgeType::ComposedOf,
            EdgeProps::StepUses { .. } => EdgeType::StepUses,
            EdgeProps::RiskAssessedBy { .. } => EdgeType::RiskAssessedBy,
            EdgeProps::Controls { .. } => EdgeType::Controls,
            EdgeProps::CapturedIn { .. } => EdgeType::CapturedIn,
            EdgeProps::TraceEntry { .. } => EdgeType::TraceEntry,
            EdgeProps::Summarizes { .. } => EdgeType::Summarizes,
            EdgeProps::Recalls { .. } => EdgeType::Recalls,
            EdgeProps::GovernedBy { .. } => EdgeType::GovernedBy,
            EdgeProps::AssignedTo { .. } => EdgeType::AssignedTo,
            EdgeProps::PlannedBy { .. } => EdgeType::PlannedBy,
            EdgeProps::HasStep { .. } => EdgeType::HasStep,
            EdgeProps::Targets { .. } => EdgeType::Targets,
            EdgeProps::RequiresApproval { .. } => EdgeType::RequiresApproval,
            EdgeProps::ExecutedBy { .. } => EdgeType::ExecutedBy,
            EdgeProps::ExecutionOf { .. } => EdgeType::ExecutionOf,
            EdgeProps::ProducesNode { .. } => EdgeType::ProducesNode,
            EdgeProps::GovernedByPolicy { .. } => EdgeType::GovernedByPolicy,
            EdgeProps::BudgetFor { .. } => EdgeType::BudgetFor,
            EdgeProps::Follows { .. } => EdgeType::Follows,
            EdgeProps::WorksFor { .. } => EdgeType::WorksFor,
            EdgeProps::AffiliatedWith { .. } => EdgeType::AffiliatedWith,
            EdgeProps::About { .. } => EdgeType::About,
            EdgeProps::KnownBy { .. } => EdgeType::KnownBy,
            EdgeProps::Custom { type_name, .. } => EdgeType::Custom(type_name.clone()),
        }
    }

    /// Create a default EdgeProps for the given EdgeType (all fields None/default).
    pub fn default_for(edge_type: EdgeType) -> Self {
        match edge_type {
            EdgeType::ExtractedFrom => EdgeProps::ExtractedFrom {
                location: None,
                method: None,
                confidence: None,
            },
            EdgeType::PartOf => EdgeProps::PartOf { role: None },
            EdgeType::HasPart => EdgeProps::HasPart { role: None },
            EdgeType::InstanceOf => EdgeProps::InstanceOf {},
            EdgeType::Contains => EdgeProps::Contains {},
            EdgeType::Supports => EdgeProps::Supports {
                strength: None,
                support_type: None,
            },
            EdgeType::Refutes => EdgeProps::Refutes {
                strength: None,
                refutation_type: None,
            },
            EdgeType::Justifies => EdgeProps::Justifies { necessity: None },
            EdgeType::HasPremise => EdgeProps::HasPremise {
                order: None,
                role: None,
            },
            EdgeType::HasConclusion => EdgeProps::HasConclusion {},
            EdgeType::HasWarrant => EdgeProps::HasWarrant {},
            EdgeType::Rebuts => EdgeProps::Rebuts {
                defeat_type: None,
                rebuttal_type: None,
                content: None,
                strength: None,
            },
            EdgeType::Assumes => EdgeProps::Assumes { necessity: None },
            EdgeType::Tests => EdgeProps::Tests {
                outcome: None,
                test_type: None,
            },
            EdgeType::Produces => EdgeProps::Produces {},
            EdgeType::UsesMethod => EdgeProps::UsesMethod { variant: None },
            EdgeType::Addresses => EdgeProps::Addresses { completeness: None },
            EdgeType::Generates => EdgeProps::Generates {},
            EdgeType::Extends => EdgeProps::Extends {
                extension_type: None,
            },
            EdgeType::Supersedes => EdgeProps::Supersedes {
                supersession_type: None,
            },
            EdgeType::Contradicts => EdgeProps::Contradicts {
                description: None,
                contradiction_type: None,
                resolution_status: None,
                proposed_resolution: None,
            },
            EdgeType::AnomalousTo => EdgeProps::AnomalousTo { severity: None },
            EdgeType::AnalogousTo => EdgeProps::AnalogousTo {
                mapping_type: None,
                strength: None,
            },
            EdgeType::Instantiates => EdgeProps::Instantiates {},
            EdgeType::TransfersTo => EdgeProps::TransfersTo {
                success: None,
                adaptations: None,
            },
            EdgeType::Evaluates => EdgeProps::Evaluates {
                evaluation_context: None,
            },
            EdgeType::Outperforms => EdgeProps::Outperforms {
                metric: None,
                margin: None,
                conditions: None,
            },
            EdgeType::FailsOn => EdgeProps::FailsOn {
                failure_mode: None,
                severity: None,
            },
            EdgeType::HasChainStep => EdgeProps::HasChainStep {
                order: None,
                step_confidence: None,
                cumulative_confidence: None,
            },
            EdgeType::PropagatesUncertaintyTo => EdgeProps::PropagatesUncertaintyTo {
                propagation_factor: None,
                propagation_type: None,
            },
            EdgeType::SensitiveTo => EdgeProps::SensitiveTo {
                elasticity: None,
                direction: None,
                threshold: None,
            },
            EdgeType::RobustAcross => EdgeProps::RobustAcross {
                variation_type: None,
                range_tested: None,
            },
            EdgeType::Describes => EdgeProps::Describes {},
            EdgeType::DerivedFrom => EdgeProps::DerivedFrom {},
            EdgeType::ReliesOn => EdgeProps::ReliesOn {},
            EdgeType::ProvenBy => EdgeProps::ProvenBy {},
            EdgeType::ProposedBy => EdgeProps::ProposedBy {
                date: None,
                context: None,
            },
            EdgeType::AuthoredBy => EdgeProps::AuthoredBy {
                position: None,
                contribution: None,
            },
            EdgeType::CitedBy => EdgeProps::CitedBy {
                citation_type: None,
                context: None,
            },
            EdgeType::BelievedBy => EdgeProps::BelievedBy {
                confidence: None,
                basis: None,
                as_of_date: None,
            },
            EdgeType::ConsensusIn => EdgeProps::ConsensusIn {
                consensus_level: None,
                as_of_date: None,
            },
            EdgeType::DecomposesInto => EdgeProps::DecomposesInto { order: None },
            EdgeType::MotivatedBy => EdgeProps::MotivatedBy {},
            EdgeType::HasOption => EdgeProps::HasOption {},
            EdgeType::DecidedOn => EdgeProps::DecidedOn { rationale: None },
            EdgeType::ConstrainedBy => EdgeProps::ConstrainedBy {},
            EdgeType::Blocks => EdgeProps::Blocks {},
            EdgeType::Informs => EdgeProps::Informs { relevance: None },
            EdgeType::RelevantTo => EdgeProps::RelevantTo {
                relevance_score: None,
            },
            EdgeType::DependsOn => EdgeProps::DependsOn {
                dependency_type: None,
            },
            EdgeType::AvailableOn => EdgeProps::AvailableOn {},
            EdgeType::ComposedOf => EdgeProps::ComposedOf { order: None },
            EdgeType::StepUses => EdgeProps::StepUses {},
            EdgeType::RiskAssessedBy => EdgeProps::RiskAssessedBy {},
            EdgeType::Controls => EdgeProps::Controls {
                interaction_type: None,
            },
            EdgeType::CapturedIn => EdgeProps::CapturedIn { position: None },
            EdgeType::TraceEntry => EdgeProps::TraceEntry { order: None },
            EdgeType::Summarizes => EdgeProps::Summarizes {},
            EdgeType::Recalls => EdgeProps::Recalls {},
            EdgeType::GovernedBy => EdgeProps::GovernedBy {},
            EdgeType::AssignedTo => EdgeProps::AssignedTo { assigned_at: None },
            EdgeType::PlannedBy => EdgeProps::PlannedBy {},
            EdgeType::HasStep => EdgeProps::HasStep { order: None },
            EdgeType::Targets => EdgeProps::Targets {},
            EdgeType::RequiresApproval => EdgeProps::RequiresApproval {},
            EdgeType::ExecutedBy => EdgeProps::ExecutedBy {},
            EdgeType::ExecutionOf => EdgeProps::ExecutionOf {},
            EdgeType::ProducesNode => EdgeProps::ProducesNode {},
            EdgeType::GovernedByPolicy => EdgeProps::GovernedByPolicy {},
            EdgeType::BudgetFor => EdgeProps::BudgetFor {},
            EdgeType::Follows => EdgeProps::Follows {},
            EdgeType::WorksFor => EdgeProps::WorksFor {},
            EdgeType::AffiliatedWith => EdgeProps::AffiliatedWith {},
            EdgeType::About => EdgeProps::About {},
            EdgeType::KnownBy => EdgeProps::KnownBy {},
            EdgeType::Custom(name) => EdgeProps::Custom {
                type_name: name,
                data: serde_json::Value::Object(Default::default()),
            },
        }
    }

    /// Serialize edge props to JSON (without the tag).
    pub fn to_json(&self) -> serde_json::Value {
        self.try_to_json_untagged().unwrap_or_default()
    }

    /// Try to serialize edge props to JSON (without the tag).
    pub fn try_to_json_untagged(&self) -> crate::Result<serde_json::Value> {
        if let EdgeProps::Custom { data, .. } = self {
            return Ok(data.clone());
        }
        let mut v = serde_json::to_value(self)?;
        if let serde_json::Value::Object(ref mut map) = v {
            map.remove("_type");
        }
        Ok(v)
    }

    /// Deserialize edge props from JSON using edge type as discriminator.
    pub fn from_json(edge_type: &EdgeType, value: &serde_json::Value) -> crate::Result<Self> {
        let de = |v: &serde_json::Value| -> crate::Result<serde_json::Value> { Ok(v.clone()) };
        let v = de(value)?;

        // For empty-props variants, return directly
        match edge_type {
            EdgeType::InstanceOf => return Ok(EdgeProps::InstanceOf {}),
            EdgeType::Contains => return Ok(EdgeProps::Contains {}),
            EdgeType::HasConclusion => return Ok(EdgeProps::HasConclusion {}),
            EdgeType::HasWarrant => return Ok(EdgeProps::HasWarrant {}),
            EdgeType::Produces => return Ok(EdgeProps::Produces {}),
            EdgeType::Generates => return Ok(EdgeProps::Generates {}),
            EdgeType::Instantiates => return Ok(EdgeProps::Instantiates {}),
            EdgeType::Describes => return Ok(EdgeProps::Describes {}),
            EdgeType::DerivedFrom => return Ok(EdgeProps::DerivedFrom {}),
            EdgeType::ReliesOn => return Ok(EdgeProps::ReliesOn {}),
            EdgeType::ProvenBy => return Ok(EdgeProps::ProvenBy {}),
            EdgeType::MotivatedBy => return Ok(EdgeProps::MotivatedBy {}),
            EdgeType::HasOption => return Ok(EdgeProps::HasOption {}),
            EdgeType::ConstrainedBy => return Ok(EdgeProps::ConstrainedBy {}),
            EdgeType::Blocks => return Ok(EdgeProps::Blocks {}),
            EdgeType::AvailableOn => return Ok(EdgeProps::AvailableOn {}),
            EdgeType::StepUses => return Ok(EdgeProps::StepUses {}),
            EdgeType::RiskAssessedBy => return Ok(EdgeProps::RiskAssessedBy {}),
            EdgeType::Summarizes => return Ok(EdgeProps::Summarizes {}),
            EdgeType::Recalls => return Ok(EdgeProps::Recalls {}),
            EdgeType::GovernedBy => return Ok(EdgeProps::GovernedBy {}),
            EdgeType::PlannedBy => return Ok(EdgeProps::PlannedBy {}),
            EdgeType::Targets => return Ok(EdgeProps::Targets {}),
            EdgeType::RequiresApproval => return Ok(EdgeProps::RequiresApproval {}),
            EdgeType::ExecutedBy => return Ok(EdgeProps::ExecutedBy {}),
            EdgeType::ExecutionOf => return Ok(EdgeProps::ExecutionOf {}),
            EdgeType::ProducesNode => return Ok(EdgeProps::ProducesNode {}),
            EdgeType::GovernedByPolicy => return Ok(EdgeProps::GovernedByPolicy {}),
            EdgeType::BudgetFor => return Ok(EdgeProps::BudgetFor {}),
            EdgeType::Follows => return Ok(EdgeProps::Follows {}),
            EdgeType::WorksFor => return Ok(EdgeProps::WorksFor {}),
            EdgeType::AffiliatedWith => return Ok(EdgeProps::AffiliatedWith {}),
            EdgeType::About => return Ok(EdgeProps::About {}),
            EdgeType::KnownBy => return Ok(EdgeProps::KnownBy {}),
            _ => {}
        }

        // For edges with fields, extract from JSON
        let get_str = |key: &str| v.get(key).and_then(|x| x.as_str()).map(String::from);
        let get_f64 = |key: &str| v.get(key).and_then(|x| x.as_f64());
        let get_i64 = |key: &str| v.get(key).and_then(|x| x.as_i64());
        let get_bool = |key: &str| v.get(key).and_then(|x| x.as_bool());

        Ok(match edge_type {
            EdgeType::ExtractedFrom => EdgeProps::ExtractedFrom {
                location: get_str("location"),
                method: get_str("method"),
                confidence: get_f64("confidence"),
            },
            EdgeType::PartOf => EdgeProps::PartOf {
                role: get_str("role"),
            },
            EdgeType::HasPart => EdgeProps::HasPart {
                role: get_str("role"),
            },
            EdgeType::Supports => EdgeProps::Supports {
                strength: get_f64("strength"),
                support_type: get_str("support_type"),
            },
            EdgeType::Refutes => EdgeProps::Refutes {
                strength: get_f64("strength"),
                refutation_type: get_str("refutation_type"),
            },
            EdgeType::Justifies => EdgeProps::Justifies {
                necessity: get_str("necessity"),
            },
            EdgeType::HasPremise => EdgeProps::HasPremise {
                order: get_i64("order"),
                role: get_str("role"),
            },
            EdgeType::Rebuts => EdgeProps::Rebuts {
                defeat_type: get_str("defeat_type"),
                rebuttal_type: get_str("rebuttal_type"),
                content: get_str("content"),
                strength: get_f64("strength"),
            },
            EdgeType::Assumes => EdgeProps::Assumes {
                necessity: get_str("necessity"),
            },
            EdgeType::Tests => EdgeProps::Tests {
                outcome: get_str("outcome"),
                test_type: get_str("test_type"),
            },
            EdgeType::UsesMethod => EdgeProps::UsesMethod {
                variant: get_str("variant"),
            },
            EdgeType::Addresses => EdgeProps::Addresses {
                completeness: get_str("completeness"),
            },
            EdgeType::Extends => EdgeProps::Extends {
                extension_type: get_str("extension_type"),
            },
            EdgeType::Supersedes => EdgeProps::Supersedes {
                supersession_type: get_str("supersession_type"),
            },
            EdgeType::Contradicts => EdgeProps::Contradicts {
                description: get_str("description"),
                contradiction_type: get_str("contradiction_type"),
                resolution_status: get_str("resolution_status"),
                proposed_resolution: get_str("proposed_resolution"),
            },
            EdgeType::AnomalousTo => EdgeProps::AnomalousTo {
                severity: get_str("severity"),
            },
            EdgeType::AnalogousTo => EdgeProps::AnalogousTo {
                mapping_type: get_str("mapping_type"),
                strength: get_f64("strength"),
            },
            EdgeType::TransfersTo => EdgeProps::TransfersTo {
                success: get_bool("success"),
                adaptations: get_str("adaptations"),
            },
            EdgeType::Evaluates => EdgeProps::Evaluates {
                evaluation_context: get_str("evaluation_context"),
            },
            EdgeType::Outperforms => EdgeProps::Outperforms {
                metric: get_str("metric"),
                margin: get_f64("margin"),
                conditions: get_str("conditions"),
            },
            EdgeType::FailsOn => EdgeProps::FailsOn {
                failure_mode: get_str("failure_mode"),
                severity: get_str("severity"),
            },
            EdgeType::HasChainStep => EdgeProps::HasChainStep {
                order: get_i64("order"),
                step_confidence: get_f64("step_confidence"),
                cumulative_confidence: get_f64("cumulative_confidence"),
            },
            EdgeType::PropagatesUncertaintyTo => EdgeProps::PropagatesUncertaintyTo {
                propagation_factor: get_f64("propagation_factor"),
                propagation_type: get_str("propagation_type"),
            },
            EdgeType::SensitiveTo => EdgeProps::SensitiveTo {
                elasticity: get_f64("elasticity"),
                direction: get_str("direction"),
                threshold: get_f64("threshold"),
            },
            EdgeType::RobustAcross => EdgeProps::RobustAcross {
                variation_type: get_str("variation_type"),
                range_tested: get_str("range_tested"),
            },
            EdgeType::ProposedBy => EdgeProps::ProposedBy {
                date: get_str("date"),
                context: get_str("context"),
            },
            EdgeType::AuthoredBy => EdgeProps::AuthoredBy {
                position: get_str("position"),
                contribution: get_str("contribution"),
            },
            EdgeType::CitedBy => EdgeProps::CitedBy {
                citation_type: get_str("citation_type"),
                context: get_str("context"),
            },
            EdgeType::BelievedBy => EdgeProps::BelievedBy {
                confidence: get_f64("confidence"),
                basis: get_str("basis"),
                as_of_date: get_str("as_of_date"),
            },
            EdgeType::ConsensusIn => EdgeProps::ConsensusIn {
                consensus_level: get_str("consensus_level"),
                as_of_date: get_str("as_of_date"),
            },
            EdgeType::DecomposesInto => EdgeProps::DecomposesInto {
                order: get_i64("order"),
            },
            EdgeType::DecidedOn => EdgeProps::DecidedOn {
                rationale: get_str("rationale"),
            },
            EdgeType::Informs => EdgeProps::Informs {
                relevance: get_f64("relevance"),
            },
            EdgeType::RelevantTo => EdgeProps::RelevantTo {
                relevance_score: get_f64("relevance_score"),
            },
            EdgeType::DependsOn => EdgeProps::DependsOn {
                dependency_type: get_str("dependency_type"),
            },
            EdgeType::ComposedOf => EdgeProps::ComposedOf {
                order: get_i64("order"),
            },
            EdgeType::Controls => EdgeProps::Controls {
                interaction_type: get_str("interaction_type"),
            },
            EdgeType::CapturedIn => EdgeProps::CapturedIn {
                position: get_i64("position"),
            },
            EdgeType::TraceEntry => EdgeProps::TraceEntry {
                order: get_i64("order"),
            },
            EdgeType::AssignedTo => EdgeProps::AssignedTo {
                assigned_at: get_f64("assigned_at"),
            },
            EdgeType::HasStep => EdgeProps::HasStep {
                order: get_i64("order"),
            },
            EdgeType::Custom(name) => EdgeProps::Custom {
                type_name: name.clone(),
                data: v,
            },
            // Already handled above
            _ => unreachable!(),
        })
    }
}
