use std::sync::Arc;

use axum::{
    extract::{Path, Query, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use mindgraph::query::TypedSnapshot;
use mindgraph::schema::edge_props::EdgeProps;
use mindgraph::traversal::{Direction, TraversalOptions};
use mindgraph::{
    AsyncMindGraph, Confidence, CreateEdge, CreateNode, EdgeType, Layer, NodeFilter, NodeProps,
    NodeType, Salience, SearchOptions, Timestamp, Uid,
};
use serde::{Deserialize, Serialize};

// ---- App State ----

struct AppState {
    graph: AsyncMindGraph,
    token: Option<String>,
}

// ---- Helpers ----

fn default_agent() -> String {
    std::env::var("MINDGRAPH_DEFAULT_AGENT").unwrap_or_else(|_| "system".into())
}

fn map_err_500(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("internal error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: e.to_string(),
        }),
    )
}

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse { error: msg.into() }),
    )
}

fn not_found(msg: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: msg.into() }),
    )
}

async fn auth_middleware(
    token: Option<String>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    if request.uri().path() == "/health" {
        return next.run(request).await.into_response();
    }
    if let Some(ref expected) = token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        match provided {
            Some(t) if t == expected => {}
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "invalid or missing Bearer token".into(),
                    }),
                )
                    .into_response();
            }
        }
    }
    next.run(request).await.into_response()
}

fn parse_node_type(s: &str) -> NodeType {
    match s {
        "Source" => NodeType::Source,
        "Snippet" => NodeType::Snippet,
        "Entity" => NodeType::Entity,
        "Observation" => NodeType::Observation,
        "Claim" => NodeType::Claim,
        "Evidence" => NodeType::Evidence,
        "Warrant" => NodeType::Warrant,
        "Argument" => NodeType::Argument,
        "Hypothesis" => NodeType::Hypothesis,
        "Theory" => NodeType::Theory,
        "Paradigm" => NodeType::Paradigm,
        "Anomaly" => NodeType::Anomaly,
        "Method" => NodeType::Method,
        "Experiment" => NodeType::Experiment,
        "Concept" => NodeType::Concept,
        "Assumption" => NodeType::Assumption,
        "Question" => NodeType::Question,
        "OpenQuestion" => NodeType::OpenQuestion,
        "Analogy" => NodeType::Analogy,
        "Pattern" => NodeType::Pattern,
        "Mechanism" => NodeType::Mechanism,
        "Model" => NodeType::Model,
        "ModelEvaluation" => NodeType::ModelEvaluation,
        "InferenceChain" => NodeType::InferenceChain,
        "SensitivityAnalysis" => NodeType::SensitivityAnalysis,
        "ReasoningStrategy" => NodeType::ReasoningStrategy,
        "Theorem" => NodeType::Theorem,
        "Equation" => NodeType::Equation,
        "Goal" => NodeType::Goal,
        "Project" => NodeType::Project,
        "Decision" => NodeType::Decision,
        "Option" => NodeType::Option,
        "Constraint" => NodeType::Constraint,
        "Milestone" => NodeType::Milestone,
        "Affordance" => NodeType::Affordance,
        "Flow" => NodeType::Flow,
        "FlowStep" => NodeType::FlowStep,
        "Control" => NodeType::Control,
        "RiskAssessment" => NodeType::RiskAssessment,
        "Session" => NodeType::Session,
        "Trace" => NodeType::Trace,
        "Summary" => NodeType::Summary,
        "Preference" => NodeType::Preference,
        "MemoryPolicy" => NodeType::MemoryPolicy,
        "Agent" => NodeType::Agent,
        "Task" => NodeType::Task,
        "Plan" => NodeType::Plan,
        "PlanStep" => NodeType::PlanStep,
        "Approval" => NodeType::Approval,
        "Policy" => NodeType::Policy,
        "Execution" => NodeType::Execution,
        "SafetyBudget" => NodeType::SafetyBudget,
        other => NodeType::Custom(other.to_string()),
    }
}

/// Convert PascalCase or camelCase to SCREAMING_SNAKE_CASE.
/// Already-screaming input passes through unchanged.
fn to_screaming_snake(s: &str) -> String {
    // If it already contains underscores and is all-uppercase, return as-is
    if s.contains('_') && s.chars().all(|c| !c.is_lowercase()) {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            // Don't double-insert underscore if previous char was already uppercase
            // e.g. "FTSIndex" → "FTS_INDEX" not "F_T_S_INDEX"
            let prev = s.chars().nth(i - 1).unwrap_or('a');
            if prev.is_lowercase()
                || (prev.is_uppercase() && s.chars().nth(i + 1).map_or(false, |c| c.is_lowercase()))
            {
                result.push('_');
            }
        }
        result.push(ch.to_ascii_uppercase());
    }
    result
}

fn parse_edge_type(s: &str) -> EdgeType {
    // Normalize PascalCase/camelCase → SCREAMING_SNAKE_CASE before matching
    let normalized = to_screaming_snake(s);
    match normalized.as_str() {
        "EXTRACTED_FROM" => EdgeType::ExtractedFrom,
        "PART_OF" => EdgeType::PartOf,
        "HAS_PART" => EdgeType::HasPart,
        "INSTANCE_OF" => EdgeType::InstanceOf,
        "CONTAINS" => EdgeType::Contains,
        "SUPPORTS" => EdgeType::Supports,
        "REFUTES" => EdgeType::Refutes,
        "JUSTIFIES" => EdgeType::Justifies,
        "HAS_PREMISE" => EdgeType::HasPremise,
        "HAS_CONCLUSION" => EdgeType::HasConclusion,
        "HAS_WARRANT" => EdgeType::HasWarrant,
        "REBUTS" => EdgeType::Rebuts,
        "ASSUMES" => EdgeType::Assumes,
        "TESTS" => EdgeType::Tests,
        "PRODUCES" => EdgeType::Produces,
        "USES_METHOD" => EdgeType::UsesMethod,
        "ADDRESSES" => EdgeType::Addresses,
        "GENERATES" => EdgeType::Generates,
        "EXTENDS" => EdgeType::Extends,
        "SUPERSEDES" => EdgeType::Supersedes,
        "CONTRADICTS" => EdgeType::Contradicts,
        "ANOMALOUS_TO" => EdgeType::AnomalousTo,
        "ANALOGOUS_TO" => EdgeType::AnalogousTo,
        "INSTANTIATES" => EdgeType::Instantiates,
        "TRANSFERS_TO" => EdgeType::TransfersTo,
        "EVALUATES" => EdgeType::Evaluates,
        "OUTPERFORMS" => EdgeType::Outperforms,
        "FAILS_ON" => EdgeType::FailsOn,
        "HAS_CHAIN_STEP" => EdgeType::HasChainStep,
        "PROPAGATES_UNCERTAINTY_TO" => EdgeType::PropagatesUncertaintyTo,
        "SENSITIVE_TO" => EdgeType::SensitiveTo,
        "ROBUST_ACROSS" => EdgeType::RobustAcross,
        "DESCRIBES" => EdgeType::Describes,
        "DERIVED_FROM" => EdgeType::DerivedFrom,
        "RELIES_ON" => EdgeType::ReliesOn,
        "PROVEN_BY" => EdgeType::ProvenBy,
        "PROPOSED_BY" => EdgeType::ProposedBy,
        "AUTHORED_BY" => EdgeType::AuthoredBy,
        "CITED_BY" => EdgeType::CitedBy,
        "BELIEVED_BY" => EdgeType::BelievedBy,
        "CONSENSUS_IN" => EdgeType::ConsensusIn,
        "DECOMPOSES_INTO" => EdgeType::DecomposesInto,
        "MOTIVATED_BY" => EdgeType::MotivatedBy,
        "HAS_OPTION" => EdgeType::HasOption,
        "DECIDED_ON" => EdgeType::DecidedOn,
        "CONSTRAINED_BY" => EdgeType::ConstrainedBy,
        "BLOCKS" => EdgeType::Blocks,
        "INFORMS" => EdgeType::Informs,
        "RELEVANT_TO" => EdgeType::RelevantTo,
        "DEPENDS_ON" => EdgeType::DependsOn,
        "AVAILABLE_ON" => EdgeType::AvailableOn,
        "COMPOSED_OF" => EdgeType::ComposedOf,
        "STEP_USES" => EdgeType::StepUses,
        "RISK_ASSESSED_BY" => EdgeType::RiskAssessedBy,
        "CONTROLS" => EdgeType::Controls,
        "CAPTURED_IN" => EdgeType::CapturedIn,
        "TRACE_ENTRY" => EdgeType::TraceEntry,
        "SUMMARIZES" => EdgeType::Summarizes,
        "RECALLS" => EdgeType::Recalls,
        "GOVERNED_BY" => EdgeType::GovernedBy,
        "ASSIGNED_TO" => EdgeType::AssignedTo,
        "PLANNED_BY" => EdgeType::PlannedBy,
        "HAS_STEP" => EdgeType::HasStep,
        "TARGETS" => EdgeType::Targets,
        "REQUIRES_APPROVAL" => EdgeType::RequiresApproval,
        "EXECUTED_BY" => EdgeType::ExecutedBy,
        "EXECUTION_OF" => EdgeType::ExecutionOf,
        "PRODUCES_NODE" => EdgeType::ProducesNode,
        "GOVERNED_BY_POLICY" => EdgeType::GovernedByPolicy,
        "BUDGET_FOR" => EdgeType::BudgetFor,
        other => EdgeType::Custom(other.to_string()),
    }
}

fn parse_layer(s: &str) -> Option<Layer> {
    match s {
        "reality" => Some(Layer::Reality),
        "epistemic" => Some(Layer::Epistemic),
        "intent" => Some(Layer::Intent),
        "action" => Some(Layer::Action),
        "memory" => Some(Layer::Memory),
        "agent" => Some(Layer::Agent),
        _ => None,
    }
}

// ---- Request/Response Types ----

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct EntityRequest {
    label: String,
    entity_type: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct ClaimRequest {
    label: String,
    content: String,
    #[serde(default = "default_half")]
    confidence: f64,
    #[serde(default = "default_agent")]
    agent_id: String,
}

fn default_half() -> f64 {
    0.5
}

fn default_one() -> f64 {
    1.0
}

#[derive(Deserialize)]
struct GoalRequest {
    label: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct PreferenceRequest {
    label: String,
    key: String,
    value: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct SessionRequest {
    label: String,
    #[serde(default)]
    focus: Option<String>,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct ObservationRequest {
    label: String,
    content: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct SummaryRequest {
    label: String,
    content: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct NodeRequest {
    label: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    salience: Option<f64>,
    props: NodeProps,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct UpdateNodeRequest {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    salience: Option<f64>,
    #[serde(default)]
    props: Option<NodeProps>,
    #[serde(default = "default_reason")]
    reason: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

fn default_reason() -> String {
    "updated via API".into()
}

#[derive(Deserialize)]
struct LinkRequest {
    from_uid: String,
    to_uid: String,
    edge_type: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct EdgeRequest {
    from_uid: String,
    to_uid: String,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    weight: Option<f64>,
    props: EdgeProps,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct UpdateEdgeRequest {
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    weight: Option<f64>,
    #[serde(default)]
    props: Option<EdgeProps>,
    #[serde(default = "default_reason")]
    reason: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    #[serde(default)]
    node_type: Option<String>,
    #[serde(default)]
    layer: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    min_score: Option<f64>,
}

#[derive(Deserialize)]
struct NodesQuery {
    #[serde(default)]
    layer: Option<String>,
    #[serde(default)]
    node_type: Option<String>,
    #[serde(default)]
    label_contains: Option<String>,
    #[serde(default)]
    agent: Option<String>,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}

fn default_limit() -> u32 {
    100
}

#[derive(Deserialize)]
struct EdgesQuery {
    #[serde(default)]
    from_uid: Option<String>,
    #[serde(default)]
    to_uid: Option<String>,
    #[serde(default)]
    edge_type: Option<String>,
}

#[derive(Deserialize)]
struct ChainQuery {
    #[serde(default = "default_max_depth")]
    max_depth: u32,
}

fn default_max_depth() -> u32 {
    5
}

#[derive(Deserialize)]
struct NeighborhoodQuery {
    #[serde(default = "default_depth")]
    depth: u32,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    edge_types: Option<Vec<String>>,
}

fn default_depth() -> u32 {
    2
}

#[derive(Deserialize)]
struct PathQuery {
    from: String,
    to: String,
    #[serde(default = "default_max_depth")]
    max_depth: u32,
}

#[derive(Deserialize)]
struct DecayRequest {
    #[serde(default = "default_half_life")]
    half_life_secs: f64,
    #[serde(default)]
    auto_tombstone_threshold: Option<f64>,
    #[serde(default)]
    min_age_secs: Option<f64>,
}

fn default_half_life() -> f64 {
    86400.0
}

#[derive(Serialize)]
struct DecayResponse {
    nodes_decayed: usize,
    below_threshold: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_tombstoned: Option<usize>,
}

#[derive(Deserialize)]
struct DeleteQuery {
    #[serde(default = "default_reason")]
    reason: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct MergeRequest {
    keep_uid: String,
    merge_uid: String,
    #[serde(default = "default_reason")]
    reason: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Deserialize)]
struct AliasRequest {
    alias: String,
    canonical_uid: String,
    #[serde(default = "default_one")]
    match_score: f64,
}

#[derive(Deserialize)]
struct FuzzyResolveQuery {
    text: String,
    #[serde(default = "default_fuzzy_limit")]
    limit: u32,
}

fn default_fuzzy_limit() -> u32 {
    5
}

#[derive(Deserialize)]
struct PurgeRequest {
    #[serde(default)]
    older_than: Option<Timestamp>,
}

#[derive(Deserialize)]
struct EdgeDeleteQuery {
    #[serde(default = "default_reason")]
    reason: String,
    #[serde(default = "default_agent")]
    agent_id: String,
}

// ---- Handlers ----

async fn health() -> &'static str {
    "ok"
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let stats = state.graph.stats().await.map_err(map_err_500)?;
    Ok(Json(stats))
}

async fn add_entity(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EntityRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_entity(req.label, req.entity_type)
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_claim(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ClaimRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_claim(req.label, req.content, req.confidence)
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_goal(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GoalRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_goal(req.label, req.priority.unwrap_or_else(|| "medium".into()))
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_preference(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PreferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_preference(req.label, req.key, req.value)
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SessionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_session(req.label, req.focus.unwrap_or_default())
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_observation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ObservationRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_observation(req.label, req.content)
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_summary(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SummaryRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let node = handle
        .add_summary(req.label, req.content)
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn add_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NodeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let mut builder = CreateNode::new(&req.label, req.props);
    if let Some(s) = req.summary {
        builder = builder.summary(s);
    }
    if let Some(c) = req.confidence {
        builder = builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    if let Some(s) = req.salience {
        builder = builder.salience(Salience::new(s).map_err(map_err_500)?);
    }
    let handle = state.graph.agent(&req.agent_id);
    let node = handle.add_node(builder).await.map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn get_node(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let node = state
        .graph
        .get_node(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    match node {
        Some(n) => Ok(Json(n)),
        None => Err(not_found(format!("node {uid} not found"))),
    }
}

async fn update_node(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(req): Json<UpdateNodeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let conf = req
        .confidence
        .map(Confidence::new)
        .transpose()
        .map_err(map_err_500)?;
    let sal = req
        .salience
        .map(Salience::new)
        .transpose()
        .map_err(map_err_500)?;
    let node = state
        .graph
        .update_node(
            Uid::from(uid.as_str()),
            req.label,
            req.summary,
            conf,
            sal,
            req.props,
            req.agent_id,
            req.reason,
        )
        .await
        .map_err(map_err_500)?;
    Ok(Json(node))
}

async fn delete_node(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Query(q): Query<DeleteQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .graph
        .tombstone_cascade(Uid::from(uid.as_str()), q.reason, q.agent_id)
        .await
        .map_err(map_err_500)?;
    Ok(Json(result))
}

async fn add_link(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LinkRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let edge_type = parse_edge_type(&req.edge_type);
    let handle = state.graph.agent(&req.agent_id);
    let edge = handle
        .add_link(
            Uid::from(req.from_uid.as_str()),
            Uid::from(req.to_uid.as_str()),
            edge_type,
        )
        .await
        .map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(edge)))
}

async fn add_edge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EdgeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let mut builder = CreateEdge::new(
        Uid::from(req.from_uid.as_str()),
        Uid::from(req.to_uid.as_str()),
        req.props,
    );
    if let Some(c) = req.confidence {
        builder = builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    if let Some(w) = req.weight {
        builder = builder.weight(w);
    }
    let handle = state.graph.agent(&req.agent_id);
    let edge = handle.add_edge(builder).await.map_err(map_err_500)?;
    Ok((StatusCode::CREATED, Json(edge)))
}

async fn update_edge(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(req): Json<UpdateEdgeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let conf = req
        .confidence
        .map(Confidence::new)
        .transpose()
        .map_err(map_err_500)?;
    let edge = state
        .graph
        .update_edge(
            Uid::from(uid.as_str()),
            conf,
            req.weight,
            req.props,
            req.agent_id,
            req.reason,
        )
        .await
        .map_err(map_err_500)?;
    Ok(Json(edge))
}

async fn get_edges(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EdgesQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let et = q.edge_type.map(|s| parse_edge_type(&s));

    // P1: Support both from_uid and to_uid queries
    match (q.from_uid, q.to_uid) {
        (Some(from), None) => {
            let edges = state
                .graph
                .edges_from(Uid::from(from.as_str()), et)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(edges).unwrap()))
        }
        (None, Some(to)) => {
            let edges = state
                .graph
                .edges_to(Uid::from(to.as_str()), et)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(edges).unwrap()))
        }
        (Some(from), Some(to)) => {
            // Return edges between two specific nodes
            let edges = state
                .graph
                .edges_from(Uid::from(from.as_str()), et.clone())
                .await
                .map_err(map_err_500)?;
            let to_uid = Uid::from(to.as_str());
            let filtered: Vec<_> = edges.into_iter().filter(|e| e.to_uid == to_uid).collect();
            Ok(Json(serde_json::to_value(filtered).unwrap()))
        }
        (None, None) => Err(bad_request("either from_uid or to_uid is required")),
    }
}

async fn search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let mut opts = SearchOptions::new();
    if let Some(nt) = &req.node_type {
        opts.node_type = Some(parse_node_type(nt));
    }
    if let Some(l) = &req.layer {
        opts.layer = parse_layer(l);
    }
    opts.limit = req.limit;
    opts.min_score = req.min_score;
    let results = state
        .graph
        .search(req.query, opts)
        .await
        .map_err(map_err_500)?;
    Ok(Json(results))
}

async fn get_nodes(
    State(state): State<Arc<AppState>>,
    Query(q): Query<NodesQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // If agent filter is set, use my_nodes() via AgentHandle
    if let Some(agent) = &q.agent {
        let handle = state.graph.agent(agent);
        let nodes = handle.my_nodes().await.map_err(map_err_500)?;
        return Ok(Json(serde_json::to_value(nodes).unwrap()));
    }

    let mut filter = NodeFilter::new();
    if let Some(l) = &q.layer {
        if let Some(layer) = parse_layer(l) {
            filter = filter.layer(layer);
        } else {
            return Err(bad_request(format!("unknown layer: {l}")));
        }
    }
    if let Some(nt) = &q.node_type {
        filter = filter.node_type(parse_node_type(nt));
    }
    if let Some(lc) = &q.label_contains {
        filter = filter.label_contains(lc);
    }
    filter.limit = Some(q.limit);
    filter.offset = Some(q.offset);

    let page = state
        .graph
        .find_nodes_paginated(filter)
        .await
        .map_err(map_err_500)?;
    Ok(Json(serde_json::to_value(page).unwrap()))
}

async fn get_chain(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Query(q): Query<ChainQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let steps = state
        .graph
        .reasoning_chain(Uid::from(uid.as_str()), q.max_depth)
        .await
        .map_err(map_err_500)?;
    Ok(Json(steps))
}

async fn get_neighborhood(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Query(q): Query<NeighborhoodQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Parse direction
    let direction = match q.direction.as_deref() {
        Some("outgoing") => Direction::Outgoing,
        Some("incoming") => Direction::Incoming,
        _ => Direction::Both,
    };

    // Parse edge types if provided
    let edge_types = q
        .edge_types
        .map(|types| types.iter().map(|t| parse_edge_type(t)).collect());

    let opts = TraversalOptions {
        direction,
        edge_types,
        max_depth: q.depth,
        weight_threshold: None,
    };

    let steps = state
        .graph
        .reachable(Uid::from(uid.as_str()), opts)
        .await
        .map_err(map_err_500)?;
    Ok(Json(steps))
}

async fn get_path(
    State(state): State<Arc<AppState>>,
    Query(q): Query<PathQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let opts = TraversalOptions {
        max_depth: q.max_depth,
        ..Default::default()
    };
    let path = state
        .graph
        .find_path(Uid::from(q.from.as_str()), Uid::from(q.to.as_str()), opts)
        .await
        .map_err(map_err_500)?;
    match path {
        Some(steps) => Ok(Json(serde_json::to_value(steps).unwrap())),
        None => Ok(Json(serde_json::Value::Null)),
    }
}

async fn get_agent_nodes(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&agent_id);
    let nodes = handle.my_nodes().await.map_err(map_err_500)?;
    Ok(Json(nodes))
}

async fn decay(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DecayRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .graph
        .decay_salience(req.half_life_secs)
        .await
        .map_err(map_err_500)?;
    let auto_tombstoned = if let (Some(threshold), Some(min_age)) =
        (req.auto_tombstone_threshold, req.min_age_secs)
    {
        Some(
            state
                .graph
                .auto_tombstone(threshold, min_age)
                .await
                .map_err(map_err_500)?,
        )
    } else {
        None
    };
    Ok(Json(DecayResponse {
        nodes_decayed: result.nodes_decayed,
        below_threshold: result.below_threshold,
        auto_tombstoned,
    }))
}

// ---- History/Versions ----

async fn get_node_history(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let history = state
        .graph
        .node_history(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    Ok(Json(history))
}

async fn get_node_at_version(
    State(state): State<Arc<AppState>>,
    Path((uid, version)): Path<(String, i64)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = state
        .graph
        .node_at_version(Uid::from(uid.as_str()), version)
        .await
        .map_err(map_err_500)?;
    match snapshot {
        Some(v) => Ok(Json(v)),
        None => Err(not_found(format!("node {uid} version {version} not found"))),
    }
}

async fn get_edge_history(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let history = state
        .graph
        .edge_history(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    Ok(Json(history))
}

// ---- Edge DELETE ----

async fn delete_edge(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Query(q): Query<EdgeDeleteQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .graph
        .tombstone_edge(Uid::from(uid.as_str()), q.reason, q.agent_id)
        .await
        .map_err(map_err_500)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Entity Resolution ----

async fn merge_entities(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MergeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .graph
        .merge_entities(
            Uid::from(req.keep_uid.as_str()),
            Uid::from(req.merge_uid.as_str()),
            req.reason,
            req.agent_id,
        )
        .await
        .map_err(map_err_500)?;
    Ok(Json(result))
}

async fn add_alias(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AliasRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .graph
        .add_alias(
            req.alias,
            Uid::from(req.canonical_uid.as_str()),
            req.match_score,
        )
        .await
        .map_err(map_err_500)?;
    Ok(StatusCode::CREATED)
}

async fn get_aliases(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let aliases = state
        .graph
        .aliases_for(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    Ok(Json(aliases))
}

async fn resolve_alias(
    State(state): State<Arc<AppState>>,
    Query(q): Query<FuzzyResolveQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let exact = state
        .graph
        .resolve_alias(q.text.clone())
        .await
        .map_err(map_err_500)?;
    if let Some(uid) = exact {
        return Ok(Json(serde_json::json!({ "exact": uid.to_string() })));
    }
    let fuzzy = state
        .graph
        .fuzzy_resolve(q.text, q.limit)
        .await
        .map_err(map_err_500)?;
    let results: Vec<_> = fuzzy
        .into_iter()
        .map(|(uid, score)| serde_json::json!({ "uid": uid.to_string(), "score": score }))
        .collect();
    Ok(Json(serde_json::json!({ "fuzzy": results })))
}

// ---- Export/Import ----

async fn export_typed(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let snapshot = state.graph.export_typed().await.map_err(map_err_500)?;
    Ok(Json(snapshot))
}

async fn import_typed(
    State(state): State<Arc<AppState>>,
    Json(snapshot): Json<TypedSnapshot>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .graph
        .import_typed(snapshot)
        .await
        .map_err(map_err_500)?;
    Ok(Json(result))
}

// ---- Purge ----

async fn purge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PurgeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .graph
        .purge_tombstoned(req.older_than)
        .await
        .map_err(map_err_500)?;
    Ok(Json(result))
}

// ---- P2: Embedding Endpoints ----

#[derive(Deserialize)]
struct ConfigureEmbeddingsRequest {
    dimension: usize,
    /// If true, drop existing embedding schema and recreate with new dimension.
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
struct SetEmbeddingRequest {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingSearchRequest {
    query: Vec<f32>,
    #[serde(default = "default_embedding_k")]
    k: u32,
}

#[derive(Deserialize)]
struct EmbeddingSearchTextRequest {
    text: String,
    #[serde(default = "default_embedding_k")]
    k: u32,
}

fn default_embedding_k() -> u32 {
    10
}

async fn configure_embeddings(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ConfigureEmbeddingsRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    if req.force {
        // Force reconfigure: clear existing embeddings and reset dimension
        if let Some(existing_dim) = state.graph.embedding_dimension() {
            if existing_dim != req.dimension {
                // Clear all embeddings first, then reconfigure
                state.graph.clear_embeddings().await.map_err(map_err_500)?;
            }
        }
    }
    state
        .graph
        .configure_embeddings(req.dimension)
        .await
        .map_err(map_err_500)?;
    Ok(Json(serde_json::json!({ "dimension": req.dimension })))
}

async fn set_embedding(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
    Json(req): Json<SetEmbeddingRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .graph
        .set_embedding(Uid::from(uid.as_str()), req.embedding)
        .await
        .map_err(map_err_500)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_embedding(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let embedding = state
        .graph
        .get_embedding(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    match embedding {
        Some(vec) => Ok(Json(serde_json::json!({ "embedding": vec }))),
        None => Err(not_found(format!("no embedding for node {uid}"))),
    }
}

async fn delete_embedding(
    State(state): State<Arc<AppState>>,
    Path(uid): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    state
        .graph
        .delete_embedding(Uid::from(uid.as_str()))
        .await
        .map_err(map_err_500)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn embedding_search(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmbeddingSearchRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let results = state
        .graph
        .semantic_search(req.query, req.k as usize)
        .await
        .map_err(map_err_500)?;
    // Convert Vec<(GraphNode, f64)> to a serializable format
    let items: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(node, score)| {
            serde_json::json!({
                "node": node,
                "score": score,
            })
        })
        .collect();
    Ok(Json(items))
}

async fn embedding_search_text(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmbeddingSearchTextRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let results = state
        .graph
        .semantic_search_text(&req.text, req.k as usize)
        .await
        .map_err(map_err_500)?;
    // Convert Vec<(GraphNode, f64)> to a serializable format
    let items: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(node, score)| {
            serde_json::json!({
                "node": node,
                "score": score,
            })
        })
        .collect();
    Ok(Json(items))
}

// ---- P3: Batch Operations ----

#[derive(Deserialize)]
struct BatchNodeItem {
    label: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    salience: Option<f64>,
    props: NodeProps,
}

#[derive(Deserialize)]
struct BatchEdgeItem {
    from_uid: String,
    to_uid: String,
    edge_type: String,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    weight: Option<f64>,
}

#[derive(Deserialize)]
struct BatchRequest {
    #[serde(default)]
    nodes: Vec<BatchNodeItem>,
    #[serde(default)]
    edges: Vec<BatchEdgeItem>,
    #[serde(default = "default_agent")]
    agent_id: String,
}

#[derive(Serialize)]
struct BatchResponse {
    nodes_added: usize,
    edges_added: usize,
    node_uids: Vec<String>,
    errors: Vec<String>,
}

async fn batch_ops(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BatchRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let mut node_uids = Vec::new();
    let mut errors = Vec::new();
    let mut nodes_added = 0usize;
    let mut edges_added = 0usize;

    // Create nodes
    for item in &req.nodes {
        let mut builder = CreateNode::new(&item.label, item.props.clone());
        if let Some(ref s) = item.summary {
            builder = builder.summary(s);
        }
        if let Some(c) = item.confidence {
            match Confidence::new(c) {
                Ok(conf) => builder = builder.confidence(conf),
                Err(e) => {
                    errors.push(format!("node '{}': {}", item.label, e));
                    node_uids.push(String::new());
                    continue;
                }
            }
        }
        if let Some(s) = item.salience {
            match Salience::new(s) {
                Ok(sal) => builder = builder.salience(sal),
                Err(e) => {
                    errors.push(format!("node '{}': {}", item.label, e));
                    node_uids.push(String::new());
                    continue;
                }
            }
        }
        match handle.add_node(builder).await {
            Ok(node) => {
                node_uids.push(node.uid.to_string());
                nodes_added += 1;
            }
            Err(e) => {
                errors.push(format!("node '{}': {}", item.label, e));
                node_uids.push(String::new());
            }
        }
    }

    // Create edges
    for item in &req.edges {
        let edge_type = parse_edge_type(&item.edge_type);
        match handle
            .add_link(
                Uid::from(item.from_uid.as_str()),
                Uid::from(item.to_uid.as_str()),
                edge_type,
            )
            .await
        {
            Ok(_) => edges_added += 1,
            Err(e) => errors.push(format!("edge {} → {}: {}", item.from_uid, item.to_uid, e)),
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(BatchResponse {
            nodes_added,
            edges_added,
            node_uids,
            errors,
        }),
    ))
}

// ---- P4: Epistemic Query Endpoints ----

#[derive(Deserialize)]
struct WeakClaimsQuery {
    #[serde(default = "default_half")]
    max_confidence: f64,
}

async fn get_goals(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let goals = state.graph.active_goals().await.map_err(map_err_500)?;
    Ok(Json(goals))
}

async fn get_open_decisions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let decisions = state.graph.open_decisions().await.map_err(map_err_500)?;
    Ok(Json(decisions))
}

async fn get_open_questions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let questions = state.graph.open_questions().await.map_err(map_err_500)?;
    Ok(Json(questions))
}

async fn get_weak_claims(
    State(state): State<Arc<AppState>>,
    Query(q): Query<WeakClaimsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let claims = state
        .graph
        .weak_claims(q.max_confidence)
        .await
        .map_err(map_err_500)?;
    Ok(Json(claims))
}

async fn get_contradictions(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let contradictions = state
        .graph
        .unresolved_contradictions()
        .await
        .map_err(map_err_500)?;
    Ok(Json(contradictions))
}

async fn get_pending_approvals(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let approvals = state.graph.pending_approvals().await.map_err(map_err_500)?;
    Ok(Json(approvals))
}

// ---- P5: Subgraph Extraction ----

#[derive(Deserialize)]
struct SubgraphRequest {
    start_uids: Vec<String>,
    #[serde(default = "default_depth")]
    max_depth: u32,
    #[serde(default)]
    edge_types: Option<Vec<String>>,
    #[serde(default)]
    node_types: Option<Vec<String>>,
}

async fn get_subgraph(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubgraphRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    if req.start_uids.is_empty() {
        return Err(bad_request("start_uids must not be empty"));
    }
    let edge_types = req
        .edge_types
        .map(|types| types.iter().map(|s| parse_edge_type(s)).collect::<Vec<_>>());
    let opts = TraversalOptions {
        max_depth: req.max_depth,
        edge_types,
        ..Default::default()
    };

    // subgraph() takes a single start UID — collect results for all start UIDs
    let mut all_nodes = Vec::new();
    let mut all_edges = Vec::new();
    let mut seen_node_uids = std::collections::HashSet::new();
    let mut seen_edge_uids = std::collections::HashSet::new();

    for uid_str in &req.start_uids {
        let (nodes, edges) = state
            .graph
            .subgraph(Uid::from(uid_str.as_str()), opts.clone())
            .await
            .map_err(map_err_500)?;
        for node in nodes {
            if seen_node_uids.insert(node.uid.to_string()) {
                all_nodes.push(node);
            }
        }
        for edge in edges {
            if seen_edge_uids.insert(edge.uid.to_string()) {
                all_edges.push(edge);
            }
        }
    }

    Ok(Json(serde_json::json!({
        "nodes": all_nodes,
        "edges": all_edges,
    })))
}

// ---- P6: Edge-Between Query ----

#[derive(Deserialize)]
struct EdgeBetweenQuery {
    from_uid: String,
    to_uid: String,
    #[serde(default)]
    edge_type: Option<String>,
}

async fn get_edge_between(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EdgeBetweenQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let et = q.edge_type.map(|s| parse_edge_type(&s));
    let edges = state
        .graph
        .get_edge_between(
            Uid::from(q.from_uid.as_str()),
            Uid::from(q.to_uid.as_str()),
            et,
        )
        .await
        .map_err(map_err_500)?;
    Ok(Json(edges))
}

// ---- Router & Main ----

fn app(state: Arc<AppState>) -> Router {
    let token = state.token.clone();
    Router::new()
        .route("/health", get(health))
        .route("/stats", get(get_stats))
        // Convenience constructors
        .route("/entity", post(add_entity))
        .route("/claim", post(add_claim))
        .route("/goal", post(add_goal))
        .route("/preference", post(add_preference))
        .route("/session", post(add_session))
        .route("/observation", post(add_observation))
        .route("/summary", post(add_summary))
        // Generic node CRUD
        .route("/node", post(add_node))
        .route(
            "/node/{uid}",
            get(get_node).patch(update_node).delete(delete_node),
        )
        .route("/node/{uid}/history", get(get_node_history))
        .route("/node/{uid}/history/{version}", get(get_node_at_version))
        // Edges
        .route("/link", post(add_link))
        .route("/edge", post(add_edge))
        .route("/edge/{uid}", patch(update_edge).delete(delete_edge))
        .route("/edge/{uid}/history", get(get_edge_history))
        .route("/edges", get(get_edges))
        // Search & filter
        .route("/search", post(search))
        .route("/nodes", get(get_nodes))
        // Traversal
        .route("/chain/{uid}", get(get_chain))
        .route("/neighborhood/{uid}", get(get_neighborhood))
        .route("/path", get(get_path))
        // Agent
        .route("/agent/{agent_id}/nodes", get(get_agent_nodes))
        // Entity resolution
        .route("/entities/merge", post(merge_entities))
        .route("/alias", post(add_alias))
        .route("/aliases/{uid}", get(get_aliases))
        .route("/resolve", get(resolve_alias))
        // P2: Embeddings
        .route("/embeddings/configure", post(configure_embeddings))
        .route("/embeddings/search", post(embedding_search))
        .route("/embeddings/search-text", post(embedding_search_text))
        .route(
            "/node/{uid}/embedding",
            axum::routing::put(set_embedding)
                .get(get_embedding)
                .delete(delete_embedding),
        )
        // P3: Batch operations
        .route("/batch", post(batch_ops))
        // P4: Epistemic queries
        .route("/goals", get(get_goals))
        .route("/decisions", get(get_open_decisions))
        .route("/questions", get(get_open_questions))
        .route("/claims/weak", get(get_weak_claims))
        .route("/contradictions", get(get_contradictions))
        .route("/approvals/pending", get(get_pending_approvals))
        // P5: Subgraph extraction
        .route("/subgraph", post(get_subgraph))
        // P6: Edge-between query
        .route("/edge/between", get(get_edge_between))
        // Export/import
        .route("/export", get(export_typed))
        .route("/import", post(import_typed))
        // Lifecycle
        .route("/decay", post(decay))
        .route("/purge", post(purge))
        .layer(middleware::from_fn(
            move |headers: HeaderMap, req: Request, next: Next| {
                let token = token.clone();
                auth_middleware(token, headers, req, next)
            },
        ))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,mindgraph_server=debug".parse().unwrap()),
        )
        .init();

    let db_path = std::env::var("MINDGRAPH_DB_PATH").unwrap_or_else(|_| "mindgraph.db".into());
    let token = std::env::var("MINDGRAPH_TOKEN").ok();
    let port: u16 = std::env::var("MINDGRAPH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(18790);

    let graph = if db_path == ":memory:" {
        AsyncMindGraph::open_in_memory()
            .await
            .expect("failed to open in-memory graph")
    } else {
        AsyncMindGraph::open(&db_path)
            .await
            .expect("failed to open graph database")
    };

    if let Ok(agent) = std::env::var("MINDGRAPH_DEFAULT_AGENT") {
        graph.set_default_agent(agent).await;
    }

    let state = Arc::new(AppState { graph, token });

    let bind_addr = std::env::var("MINDGRAPH_BIND").unwrap_or_else(|_| "127.0.0.1".into());
    let listener = tokio::net::TcpListener::bind((bind_addr.as_str(), port))
        .await
        .expect("failed to bind");
    tracing::info!("mindgraph-server listening on {bind_addr}:{port}");

    let router = app(state);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}
