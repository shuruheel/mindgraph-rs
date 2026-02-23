use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClaimProps {
    pub content: String,
    pub claim_type: Option<String>,
    pub certainty_degree: Option<f64>,
    pub truth_status: Option<String>,
    pub scope: Option<String>,
    pub quantitative_value: Option<f64>,
    pub unit: Option<String>,
    pub uncertainty_range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EvidenceProps {
    pub description: String,
    pub evidence_type: Option<String>,
    pub quantitative_value: Option<f64>,
    pub unit: Option<String>,
    pub sample_size: Option<u64>,
    pub statistical_significance: Option<f64>,
    pub is_negative: bool,
    pub original_expectation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WarrantProps {
    pub principle: String,
    pub warrant_type: Option<String>,
    pub explicit_in_text: bool,
    pub strength: Option<f64>,
    pub domain_scope: Option<String>,
}

impl Default for WarrantProps {
    fn default() -> Self {
        Self {
            principle: String::new(),
            warrant_type: None,
            explicit_in_text: true,
            strength: None,
            domain_scope: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ArgumentProps {
    pub summary: String,
    pub argument_type: Option<String>,
    pub strength: Option<f64>,
    pub is_valid: Option<bool>,
    pub is_sound: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HypothesisProps {
    pub statement: String,
    pub hypothesis_type: Option<String>,
    pub status: Option<String>,
    pub testability_score: Option<f64>,
    pub novelty: Option<f64>,
    pub predicted_observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TheoryProps {
    pub name: String,
    pub description: String,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub explanatory_scope: Option<String>,
    pub core_commitments: Vec<String>,
    pub predictive_successes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ParadigmProps {
    pub name: String,
    pub field: Option<String>,
    pub status: Option<String>,
    pub core_assumptions: Vec<String>,
    pub exemplar_problems: Vec<String>,
    pub accepted_methods: Vec<String>,
    pub tension_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AnomalyProps {
    pub description: String,
    pub anomaly_type: Option<String>,
    pub severity: Option<String>,
    pub persistence: Option<String>,
    pub resolution_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MethodProps {
    pub name: String,
    pub description: String,
    pub method_type: Option<String>,
    pub domain: Option<String>,
    pub limitations: Vec<String>,
    pub validity_conditions: Vec<String>,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ExperimentProps {
    pub name: Option<String>,
    pub description: String,
    pub design_type: Option<String>,
    pub variables_manipulated: Vec<String>,
    pub variables_measured: Vec<String>,
    pub controls: Vec<String>,
    pub sample_description: Option<String>,
    pub date_conducted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConceptProps {
    pub name: String,
    pub domain: Option<String>,
    pub definition: Option<String>,
    pub definition_type: Option<String>,
    pub abstraction_level: Option<String>,
    pub alternative_definitions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AssumptionProps {
    pub content: String,
    pub assumption_type: Option<String>,
    pub explicit_in_text: bool,
    pub justification: Option<String>,
    pub vulnerability: Option<String>,
}

impl Default for AssumptionProps {
    fn default() -> Self {
        Self {
            content: String::new(),
            assumption_type: None,
            explicit_in_text: true,
            justification: None,
            vulnerability: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct QuestionProps {
    pub text: String,
    pub question_type: Option<String>,
    pub scope: Option<String>,
    pub status: Option<String>,
    pub importance: Option<f64>,
    pub tractability: Option<f64>,
    pub blocking_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OpenQuestionProps {
    pub text: String,
    pub question_type: Option<String>,
    pub scope: Option<String>,
    pub status: Option<String>,
    pub importance: Option<f64>,
    pub tractability: Option<f64>,
    pub blocking_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AnalogyProps {
    pub description: String,
    pub source_domain: Option<String>,
    pub target_domain: Option<String>,
    pub analogy_type: Option<String>,
    pub mapping_elements: Vec<String>,
    pub strength: Option<f64>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PatternProps {
    pub name: String,
    pub description: String,
    pub pattern_type: Option<String>,
    pub domains_observed: Vec<String>,
    pub instance_count: Option<u64>,
    pub generality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MechanismProps {
    pub name: String,
    pub description: String,
    pub components: Vec<String>,
    pub interactions: Vec<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelProps {
    pub name: String,
    pub description: String,
    pub model_type: Option<String>,
    pub target_system: Option<String>,
    pub key_parameters: Vec<String>,
    pub simplifications: Vec<String>,
    pub validation_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelEvaluationProps {
    pub evaluation_type: Option<String>,
    pub metrics: Vec<String>,
    pub failure_domains: Vec<String>,
    pub comparison_to: Vec<String>,
    pub evaluation_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InferenceChainProps {
    pub description: String,
    pub chain_length: Option<u32>,
    pub compound_confidence: Option<f64>,
    pub propagation_method: Option<String>,
    pub weakest_link: Option<String>,
    pub critical_assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SensitivityAnalysisProps {
    pub analysis_type: Option<String>,
    pub target_claim_uid: Option<String>,
    pub sensitivity_map: Vec<String>,
    pub robustness_score: Option<f64>,
    pub critical_inputs: Vec<String>,
    pub break_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ReasoningStrategyProps {
    pub name: String,
    pub description: String,
    pub strategy_type: Option<String>,
    pub applicable_contexts: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TheoremProps {
    pub name: Option<String>,
    pub statement: String,
    pub proof_status: Option<String>,
    pub theorem_type: Option<String>,
    pub proof_technique: Option<String>,
    pub significance: Option<String>,
    pub applications: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EquationProps {
    pub name: Option<String>,
    pub expression: String,
    pub description: Option<String>,
    pub equation_type: Option<String>,
    pub variables: Vec<String>,
    pub parameters: Vec<String>,
    pub domain: Option<String>,
    pub assumptions: Vec<String>,
}
