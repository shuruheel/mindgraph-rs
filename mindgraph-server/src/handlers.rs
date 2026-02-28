// Cognitive layer handlers for mindgraph-server.
// These 18 higher-level endpoints map semantic operations to graph primitives.

use axum::{extract::State, http::StatusCode, Json};
use mindgraph::{
    AffordanceProps, AgentProps, AnalogyProps, AnomalyProps, ApprovalProps, ArgumentProps,
    AssumptionProps, ClaimProps, Confidence, ConceptProps, ConstraintProps, ControlProps,
    CreateNode, DecisionProps, Direction, EdgeType, EvidenceProps, ExecutionProps, FlowProps,
    FlowStepProps, GoalProps, HypothesisProps, InferenceChainProps, MechanismProps,
    MemoryPolicyProps, MilestoneProps, ModelEvaluationProps, ModelProps, NodeFilter, NodeProps,
    NodeType, ObservationProps, OpenQuestionProps, OptionProps, Pagination, ParadigmProps,
    PatternProps, PlanProps, PlanStepProps, PolicyProps, ProjectProps, QuestionProps,
    ReasoningStrategyProps, RiskAssessmentProps, SafetyBudgetProps, Salience, SearchOptions,
    SensitivityAnalysisProps, SnippetProps, SourceProps, SummaryProps, TaskProps, TheoryProps,
    TheoremProps, TraceProps, TraversalOptions, Uid, WarrantProps, now,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::{bad_request, default_agent, map_err_500, not_found, parse_edge_type, parse_layer,
    parse_node_type, AppState, ErrorResponse};

// ---- Shared helper ----

fn parse_direction(s: Option<&str>) -> Direction {
    match s {
        Some("incoming") => Direction::Incoming,
        Some("outgoing") => Direction::Outgoing,
        _ => Direction::Both,
    }
}

async fn create_link(
    state: &AppState,
    from: &str,
    to: &str,
    edge_type: EdgeType,
    agent: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(agent);
    handle
        .add_link(Uid::from(from), Uid::from(to), edge_type)
        .await
        .map_err(map_err_500)?;
    Ok(())
}

// ============================================================
// Endpoint 1 — POST /reality/ingest
// ============================================================

#[derive(Deserialize)]
pub(crate) struct IngestRequest {
    #[serde(rename = "type")]
    pub(crate) ingest_type: String,
    pub(crate) label: String,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) source_uid: Option<String>,
    #[serde(default)]
    pub(crate) medium: Option<String>,
    #[serde(default)]
    pub(crate) url: Option<String>,
    #[serde(default)]
    pub(crate) timestamp: Option<f64>,
    #[serde(default)]
    pub(crate) confidence: Option<f64>,
    #[serde(default)]
    pub(crate) salience: Option<f64>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn ingest_reality(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    let props = match req.ingest_type.as_str() {
        "source" => NodeProps::Source(SourceProps {
            source_type: req.medium.clone().unwrap_or_else(|| "web".into()),
            uri: req.url.clone().unwrap_or_default(),
            title: req.label.clone(),
            ..Default::default()
        }),
        "snippet" => {
            if req.source_uid.is_none() {
                return Err(bad_request("source_uid is required for snippet type"));
            }
            NodeProps::Snippet(SnippetProps {
                content: req.content.clone(),
                ..Default::default()
            })
        }
        "observation" => NodeProps::Observation(ObservationProps {
            content: req.content.clone(),
            timestamp: req.timestamp,
            ..Default::default()
        }),
        other => return Err(bad_request(format!("unknown ingest type: {other}"))),
    };

    let mut builder = CreateNode::new(&req.label, props).summary(&req.content);
    if let Some(c) = req.confidence {
        builder = builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    if let Some(s) = req.salience {
        builder = builder.salience(Salience::new(s).map_err(map_err_500)?);
    }

    let node = handle.add_node(builder).await.map_err(map_err_500)?;
    let uid = node.uid.to_string();

    // Auto-create ExtractedFrom edge for snippet
    let mut edges_created = Vec::new();
    if req.ingest_type == "snippet" {
        if let Some(src_uid) = &req.source_uid {
            create_link(&state, &uid, src_uid, EdgeType::ExtractedFrom, &req.agent_id).await?;
            edges_created.push(src_uid.clone());
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "uid": uid,
            "type": req.ingest_type,
            "label": req.label,
            "edges_created": edges_created,
            "version": node.version,
        })),
    ))
}

// ============================================================
// Endpoint 2 — POST /reality/entity
// ============================================================

#[derive(Deserialize)]
pub(crate) struct ManageEntityRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) entity_type: Option<String>,
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) canonical_uid: Option<String>,
    #[serde(default)]
    pub(crate) alias_score: Option<f64>,
    #[serde(default)]
    pub(crate) keep_uid: Option<String>,
    #[serde(default)]
    pub(crate) merge_uid: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn manage_entity(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ManageEntityRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    match req.action.as_str() {
        "create" => {
            let label = req.label.ok_or_else(|| bad_request("label required for create"))?;
            let entity_type = req.entity_type.unwrap_or_else(|| "other".into());
            let handle = state.graph.agent(&req.agent_id);
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Entity(mindgraph::EntityProps {
                        entity_type,
                        canonical_name: label.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(node).unwrap()))
        }
        "alias" => {
            let text = req.text.ok_or_else(|| bad_request("text required for alias"))?;
            let canon = req
                .canonical_uid
                .ok_or_else(|| bad_request("canonical_uid required for alias"))?;
            state
                .graph
                .add_alias(text, Uid::from(canon.as_str()), req.alias_score.unwrap_or(1.0))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "status": "ok" })))
        }
        "resolve" => {
            let text = req.text.ok_or_else(|| bad_request("text required for resolve"))?;
            let result = state
                .graph
                .resolve_alias(text)
                .await
                .map_err(map_err_500)?;
            match result {
                Some(uid) => Ok(Json(serde_json::json!({ "uid": uid.to_string() }))),
                None => Ok(Json(serde_json::json!({ "uid": null }))),
            }
        }
        "fuzzy_resolve" => {
            let text = req
                .text
                .ok_or_else(|| bad_request("text required for fuzzy_resolve"))?;
            let limit = req.limit.unwrap_or(5);
            let matches = state
                .graph
                .fuzzy_resolve(text, limit)
                .await
                .map_err(map_err_500)?;
            // Enrich with labels by looking up nodes
            let mut results = Vec::new();
            for (uid, score) in matches {
                let label = state
                    .graph
                    .get_node(uid.clone())
                    .await
                    .map_err(map_err_500)?
                    .map(|n| n.label)
                    .unwrap_or_default();
                results.push(serde_json::json!({ "uid": uid.to_string(), "label": label, "score": score }));
            }
            Ok(Json(serde_json::json!({ "matches": results })))
        }
        "merge" => {
            let keep = req
                .keep_uid
                .ok_or_else(|| bad_request("keep_uid required for merge"))?;
            let merge = req
                .merge_uid
                .ok_or_else(|| bad_request("merge_uid required for merge"))?;
            let result = state
                .graph
                .merge_entities(
                    Uid::from(keep.as_str()),
                    Uid::from(merge.as_str()),
                    "merged via API".into(),
                    req.agent_id,
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(result).unwrap()))
        }
        other => Err(bad_request(format!("unknown action: {other}"))),
    }
}

// ============================================================
// Endpoint 3 — POST /epistemic/argument
// ============================================================

#[derive(Deserialize)]
pub(crate) struct ClaimItem {
    pub(crate) label: String,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) confidence: Option<f64>,
}

#[derive(Deserialize)]
pub(crate) struct EvidenceItem {
    pub(crate) label: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) evidence_type: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct WarrantItem {
    pub(crate) label: String,
    pub(crate) principle: String,
}

#[derive(Deserialize)]
pub(crate) struct ArgumentItem {
    pub(crate) label: String,
    pub(crate) summary: String,
}

#[derive(Deserialize)]
pub(crate) struct ArgumentRequest {
    pub(crate) claim: ClaimItem,
    #[serde(default)]
    pub(crate) evidence: Option<Vec<EvidenceItem>>,
    #[serde(default)]
    pub(crate) warrant: Option<WarrantItem>,
    #[serde(default)]
    pub(crate) argument: Option<ArgumentItem>,
    #[serde(default)]
    pub(crate) refutes_uid: Option<String>,
    #[serde(default)]
    pub(crate) extends_uid: Option<String>,
    #[serde(default)]
    pub(crate) source_uids: Option<Vec<String>>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn argument(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ArgumentRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    // 1. Create Claim node
    let mut claim_builder = CreateNode::new(
        &req.claim.label,
        NodeProps::Claim(ClaimProps {
            content: req.claim.content.clone(),
            ..Default::default()
        }),
    )
    .summary(&req.claim.content);
    if let Some(c) = req.claim.confidence {
        claim_builder = claim_builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    let claim_node = handle.add_node(claim_builder).await.map_err(map_err_500)?;
    let claim_uid = claim_node.uid.to_string();

    // 2. Create Evidence nodes + Supports edges
    let mut evidence_uids = Vec::new();
    for ev in req.evidence.iter().flatten() {
        let ev_node = handle
            .add_node(CreateNode::new(
                &ev.label,
                NodeProps::Evidence(EvidenceProps {
                    description: ev.description.clone(),
                    evidence_type: ev.evidence_type.clone(),
                    ..Default::default()
                }),
            ))
            .await
            .map_err(map_err_500)?;
        let ev_uid = ev_node.uid.to_string();
        // evidence → claim
        create_link(&state, &ev_uid, &claim_uid, EdgeType::Supports, &req.agent_id).await?;
        evidence_uids.push(ev_uid);
    }

    // 3. Create Warrant node + HasWarrant edge
    let warrant_uid = if let Some(w) = &req.warrant {
        let w_node = handle
            .add_node(CreateNode::new(
                &w.label,
                NodeProps::Warrant(WarrantProps {
                    principle: w.principle.clone(),
                    ..Default::default()
                }),
            ))
            .await
            .map_err(map_err_500)?;
        let w_uid = w_node.uid.to_string();
        // claim → warrant
        create_link(&state, &claim_uid, &w_uid, EdgeType::HasWarrant, &req.agent_id).await?;
        Some(w_uid)
    } else {
        None
    };

    // 4. Create Argument node + HasConclusion + HasPremise edges
    let argument_uid = if let Some(arg) = &req.argument {
        let arg_node = handle
            .add_node(CreateNode::new(
                &arg.label,
                NodeProps::Argument(ArgumentProps {
                    summary: arg.summary.clone(),
                    ..Default::default()
                }),
            ))
            .await
            .map_err(map_err_500)?;
        let arg_uid = arg_node.uid.to_string();
        // argument → claim (conclusion)
        create_link(&state, &arg_uid, &claim_uid, EdgeType::HasConclusion, &req.agent_id).await?;
        // argument → each evidence (premise)
        for ev_uid in &evidence_uids {
            create_link(&state, &arg_uid, ev_uid, EdgeType::HasPremise, &req.agent_id).await?;
        }
        Some(arg_uid)
    } else {
        None
    };

    // 5. Refutes edge
    if let Some(ref_uid) = &req.refutes_uid {
        create_link(&state, &claim_uid, ref_uid, EdgeType::Refutes, &req.agent_id).await?;
    }

    // 6. Extends edge
    if let Some(ext_uid) = &req.extends_uid {
        create_link(&state, &claim_uid, ext_uid, EdgeType::Extends, &req.agent_id).await?;
    }

    // 7. Source edges
    for src_uid in req.source_uids.iter().flatten() {
        create_link(&state, &claim_uid, src_uid, EdgeType::ExtractedFrom, &req.agent_id).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "claim_uid": claim_uid,
            "evidence_uids": evidence_uids,
            "warrant_uid": warrant_uid,
            "argument_uid": argument_uid,
        })),
    ))
}

// ============================================================
// Endpoint 4 — POST /epistemic/inquiry
// ============================================================

#[derive(Deserialize)]
pub(crate) struct InquiryRequest {
    #[serde(rename = "type")]
    pub(crate) inquiry_type: String,
    pub(crate) label: String,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) anomalous_to_uid: Option<String>,
    #[serde(default)]
    pub(crate) assumes_uid: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) tests_uid: Option<String>,
    #[serde(default)]
    pub(crate) addresses_uid: Option<String>,
    #[serde(default)]
    pub(crate) confidence: Option<f64>,
    #[serde(default)]
    pub(crate) salience: Option<f64>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn inquiry(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InquiryRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    let props = match req.inquiry_type.as_str() {
        "hypothesis" => NodeProps::Hypothesis(HypothesisProps {
            statement: req.content.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        "theory" => NodeProps::Theory(TheoryProps {
            name: req.label.clone(),
            description: req.content.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        "paradigm" => NodeProps::Paradigm(ParadigmProps {
            name: req.label.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        "anomaly" => NodeProps::Anomaly(AnomalyProps {
            description: req.content.clone(),
            ..Default::default()
        }),
        "assumption" => NodeProps::Assumption(AssumptionProps {
            content: req.content.clone(),
            ..Default::default()
        }),
        "question" => NodeProps::Question(QuestionProps {
            text: req.content.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        "open_question" => NodeProps::OpenQuestion(OpenQuestionProps {
            text: req.content.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        other => return Err(bad_request(format!("unknown inquiry type: {other}"))),
    };

    let mut builder = CreateNode::new(&req.label, props).summary(&req.content);
    if let Some(c) = req.confidence {
        builder = builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    if let Some(s) = req.salience {
        builder = builder.salience(Salience::new(s).map_err(map_err_500)?);
    }
    let node = handle.add_node(builder).await.map_err(map_err_500)?;
    let uid = node.uid.to_string();

    // Auto-edges based on type
    if req.inquiry_type == "anomaly" {
        if let Some(ref anom_uid) = req.anomalous_to_uid {
            create_link(&state, &uid, anom_uid, EdgeType::AnomalousTo, &req.agent_id).await?;
        }
    }
    if req.inquiry_type == "hypothesis" {
        if let Some(ref test_uid) = req.tests_uid {
            create_link(&state, &uid, test_uid, EdgeType::Tests, &req.agent_id).await?;
        }
    }
    if req.inquiry_type == "assumption" {
        for assume_uid in req.assumes_uid.iter().flatten() {
            create_link(&state, &uid, assume_uid, EdgeType::Assumes, &req.agent_id).await?;
        }
    }
    if req.inquiry_type == "question" || req.inquiry_type == "open_question" {
        if let Some(ref addr_uid) = req.addresses_uid {
            create_link(&state, &uid, addr_uid, EdgeType::Addresses, &req.agent_id).await?;
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "uid": uid, "type": req.inquiry_type, "label": req.label })),
    ))
}

// ============================================================
// Endpoint 5 — POST /epistemic/structure
// ============================================================

#[derive(Deserialize)]
pub(crate) struct StructureRequest {
    #[serde(rename = "type")]
    pub(crate) structure_type: String,
    pub(crate) label: String,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) summary: Option<String>,
    #[serde(default)]
    pub(crate) analogous_to_uid: Option<String>,
    #[serde(default)]
    pub(crate) transfers_to_uid: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) evaluates_uid: Option<String>,
    #[serde(default)]
    pub(crate) outperforms_uid: Option<String>,
    #[serde(default)]
    pub(crate) chain_steps: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) derived_from_uid: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) proven_by_uid: Option<String>,
    #[serde(default)]
    pub(crate) confidence: Option<f64>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn structure(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StructureRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    let summary_text = req.summary.clone().unwrap_or_else(|| req.content.clone());

    let props = match req.structure_type.as_str() {
        "concept" => NodeProps::Concept(ConceptProps {
            name: req.label.clone(),
            definition: Some(req.content.clone()),
            ..Default::default()
        }),
        "pattern" => NodeProps::Pattern(PatternProps {
            name: req.label.clone(),
            description: req.content.clone(),
            ..Default::default()
        }),
        "mechanism" => NodeProps::Mechanism(MechanismProps {
            name: req.label.clone(),
            description: req.content.clone(),
            ..Default::default()
        }),
        "model" => NodeProps::Model(ModelProps {
            name: req.label.clone(),
            description: req.content.clone(),
            ..Default::default()
        }),
        "model_evaluation" => NodeProps::ModelEvaluation(ModelEvaluationProps {
            ..Default::default()
        }),
        "analogy" => NodeProps::Analogy(AnalogyProps {
            description: req.content.clone(),
            ..Default::default()
        }),
        "inference_chain" => NodeProps::InferenceChain(InferenceChainProps {
            description: req.content.clone(),
            ..Default::default()
        }),
        "reasoning_strategy" => NodeProps::ReasoningStrategy(ReasoningStrategyProps {
            name: req.label.clone(),
            description: req.content.clone(),
            ..Default::default()
        }),
        "sensitivity_analysis" => NodeProps::SensitivityAnalysis(SensitivityAnalysisProps {
            ..Default::default()
        }),
        "theorem" => NodeProps::Theorem(TheoremProps {
            statement: req.content.clone(),
            ..Default::default()
        }),
        "equation" => NodeProps::Equation(mindgraph::EquationProps {
            expression: req.content.clone(),
            ..Default::default()
        }),
        other => return Err(bad_request(format!("unknown structure type: {other}"))),
    };

    let mut builder = CreateNode::new(&req.label, props).summary(&summary_text);
    if let Some(c) = req.confidence {
        builder = builder.confidence(Confidence::new(c).map_err(map_err_500)?);
    }
    let node = handle.add_node(builder).await.map_err(map_err_500)?;
    let uid = node.uid.to_string();

    // Auto-edges
    if let Some(ref alog_uid) = req.analogous_to_uid {
        create_link(&state, &uid, alog_uid, EdgeType::AnalogousTo, &req.agent_id).await?;
    }
    if req.structure_type == "analogy" {
        for t_uid in req.transfers_to_uid.iter().flatten() {
            create_link(&state, &uid, t_uid, EdgeType::TransfersTo, &req.agent_id).await?;
        }
    }
    if req.structure_type == "model_evaluation" {
        if let Some(ref eval_uid) = req.evaluates_uid {
            create_link(&state, &uid, eval_uid, EdgeType::Evaluates, &req.agent_id).await?;
        }
        if let Some(ref out_uid) = req.outperforms_uid {
            create_link(&state, &uid, out_uid, EdgeType::Outperforms, &req.agent_id).await?;
        }
    }
    if req.structure_type == "inference_chain" {
        for step_uid in req.chain_steps.iter().flatten() {
            create_link(&state, &uid, step_uid, EdgeType::HasChainStep, &req.agent_id).await?;
        }
    }
    if req.structure_type == "theorem" || req.structure_type == "equation" {
        for src_uid in req.derived_from_uid.iter().flatten() {
            create_link(&state, &uid, src_uid, EdgeType::DerivedFrom, &req.agent_id).await?;
        }
        if let Some(ref pb_uid) = req.proven_by_uid {
            create_link(&state, &uid, pb_uid, EdgeType::ProvenBy, &req.agent_id).await?;
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "uid": uid, "type": req.structure_type, "label": req.label })),
    ))
}

// ============================================================
// Endpoint 6 — POST /intent/commitment
// ============================================================

#[derive(Deserialize)]
pub(crate) struct CommitmentRequest {
    #[serde(rename = "type")]
    pub(crate) commitment_type: String,
    pub(crate) label: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) priority: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) parent_uid: Option<String>,
    #[serde(default)]
    pub(crate) due_date: Option<f64>,
    #[serde(default)]
    pub(crate) motivated_by_uid: Option<Vec<String>>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn commitment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CommitmentRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    let props = match req.commitment_type.as_str() {
        "goal" => NodeProps::Goal(GoalProps {
            description: req.description.clone(),
            status: req.status.clone().or_else(|| Some("active".into())),
            priority: req.priority.clone(),
            ..Default::default()
        }),
        "project" => NodeProps::Project(ProjectProps {
            name: req.label.clone(),
            description: req.description.clone(),
            status: req.status.clone(),
            ..Default::default()
        }),
        "milestone" => NodeProps::Milestone(MilestoneProps {
            description: req.description.clone(),
            status: req.status.clone(),
            target_date: req.due_date,
            ..Default::default()
        }),
        other => return Err(bad_request(format!("unknown commitment type: {other}"))),
    };

    let node = handle
        .add_node(CreateNode::new(&req.label, props).summary(&req.description))
        .await
        .map_err(map_err_500)?;
    let uid = node.uid.to_string();

    // Auto-edges
    if let Some(ref par_uid) = req.parent_uid {
        // parent → child (DecomposesInto)
        create_link(&state, par_uid, &uid, EdgeType::DecomposesInto, &req.agent_id).await?;
    }
    for mot_uid in req.motivated_by_uid.iter().flatten() {
        create_link(&state, &uid, mot_uid, EdgeType::MotivatedBy, &req.agent_id).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "uid": uid, "type": req.commitment_type, "label": req.label })),
    ))
}

// ============================================================
// Endpoint 7 — POST /intent/deliberation
// ============================================================

#[derive(Deserialize)]
pub(crate) struct DeliberationRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) decision_uid: Option<String>,
    #[serde(default)]
    pub(crate) chosen_option_uid: Option<String>,
    #[serde(default)]
    pub(crate) resolution_rationale: Option<String>,
    #[serde(default)]
    pub(crate) constraint_type: Option<String>,
    #[serde(default)]
    pub(crate) blocks_uid: Option<String>,
    #[serde(default)]
    pub(crate) informs_uid: Option<Vec<String>>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn deliberation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeliberationRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "open_decision" => {
            let label = req
                .label
                .ok_or_else(|| bad_request("label required for open_decision"))?;
            let desc = req.description.clone().unwrap_or_default();
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Decision(DecisionProps {
                        question: desc.clone(),
                        status: Some("open".into()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": node.uid.to_string(),
                "action": "open_decision",
                "label": label,
            })))
        }
        "add_option" => {
            let dec_uid = req
                .decision_uid
                .as_deref()
                .ok_or_else(|| bad_request("decision_uid required for add_option"))?;
            let label = req
                .label
                .ok_or_else(|| bad_request("label required for add_option"))?;
            let desc = req.description.clone().unwrap_or_default();
            let opt_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Option(OptionProps {
                        description: desc,
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let opt_uid = opt_node.uid.to_string();
            // decision → option
            create_link(&state, dec_uid, &opt_uid, EdgeType::HasOption, &req.agent_id).await?;
            // option informs additional nodes
            for inf_uid in req.informs_uid.iter().flatten() {
                create_link(&state, &opt_uid, inf_uid, EdgeType::Informs, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({
                "uid": opt_uid,
                "action": "add_option",
                "label": label,
            })))
        }
        "add_constraint" => {
            let dec_uid = req
                .decision_uid
                .as_deref()
                .ok_or_else(|| bad_request("decision_uid required for add_constraint"))?;
            let label = req
                .label
                .ok_or_else(|| bad_request("label required for add_constraint"))?;
            let desc = req.description.clone().unwrap_or_default();
            let con_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Constraint(ConstraintProps {
                        description: desc,
                        constraint_type: req.constraint_type.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let con_uid = con_node.uid.to_string();
            // decision → constraint
            create_link(&state, dec_uid, &con_uid, EdgeType::ConstrainedBy, &req.agent_id).await?;
            // constraint blocks option
            if let Some(ref blk_uid) = req.blocks_uid {
                create_link(&state, &con_uid, blk_uid, EdgeType::Blocks, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({
                "uid": con_uid,
                "action": "add_constraint",
                "label": label,
            })))
        }
        "resolve" => {
            let dec_uid = req
                .decision_uid
                .clone()
                .ok_or_else(|| bad_request("decision_uid required for resolve"))?;
            let chosen = req
                .chosen_option_uid
                .as_deref()
                .ok_or_else(|| bad_request("chosen_option_uid required for resolve"))?;
            // Fetch current node to get existing props
            let current = state
                .graph
                .get_node(Uid::from(dec_uid.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("decision node {dec_uid} not found")))?;
            // Update props
            let updated_props = if let NodeProps::Decision(mut dp) = current.props {
                dp.status = Some("resolved".into());
                dp.decided_option_uid = Some(chosen.to_string());
                dp.decision_rationale = req.resolution_rationale.clone();
                dp.decided_at = Some(now());
                Some(NodeProps::Decision(dp))
            } else {
                None
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(dec_uid.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id.clone(),
                    "resolved via API".into(),
                )
                .await
                .map_err(map_err_500)?;
            // Create DecidedOn edge
            create_link(&state, &dec_uid, chosen, EdgeType::DecidedOn, &req.agent_id).await?;
            Ok(Json(serde_json::json!({
                "uid": dec_uid,
                "action": "resolve",
                "version": updated.version,
            })))
        }
        "get_open" => {
            let decisions = state.graph.open_decisions().await.map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(decisions).unwrap()))
        }
        other => Err(bad_request(format!("unknown deliberation action: {other}"))),
    }
}

// ============================================================
// Endpoint 8 — POST /action/procedure
// ============================================================

#[derive(Deserialize)]
pub(crate) struct ProcedureRequest {
    pub(crate) action: String,
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) flow_uid: Option<String>,
    #[serde(default)]
    pub(crate) step_order: Option<u32>,
    #[serde(default)]
    pub(crate) previous_step_uid: Option<String>,
    #[serde(default)]
    pub(crate) affordance_type: Option<String>,
    #[serde(default)]
    pub(crate) control_type: Option<String>,
    #[serde(default)]
    pub(crate) uses_affordance_uids: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) goal_uid: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn procedure(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProcedureRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);
    let desc = req.description.clone().unwrap_or_default();

    match req.action.as_str() {
        "create_flow" => {
            let node = handle
                .add_node(CreateNode::new(
                    &req.label,
                    NodeProps::Flow(FlowProps {
                        name: req.label.clone(),
                        description: desc.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let uid = node.uid.to_string();
            if let Some(ref g_uid) = req.goal_uid {
                create_link(&state, &uid, g_uid, EdgeType::RelevantTo, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({
                "uid": uid, "action": "create_flow", "label": req.label,
            })))
        }
        "add_step" => {
            let flow_uid = req
                .flow_uid
                .as_deref()
                .ok_or_else(|| bad_request("flow_uid required for add_step"))?;
            let order = req.step_order.unwrap_or(0);
            let step_node = handle
                .add_node(CreateNode::new(
                    &req.label,
                    NodeProps::FlowStep(FlowStepProps {
                        order,
                        description: desc,
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let step_uid = step_node.uid.to_string();
            // flow → step
            create_link(&state, flow_uid, &step_uid, EdgeType::ComposedOf, &req.agent_id).await?;
            // previous_step → new_step ordering
            if let Some(ref prev_uid) = req.previous_step_uid {
                create_link(&state, prev_uid, &step_uid, EdgeType::DependsOn, &req.agent_id)
                    .await?;
            }
            // step uses affordances
            for aff_uid in req.uses_affordance_uids.iter().flatten() {
                create_link(&state, &step_uid, aff_uid, EdgeType::StepUses, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({
                "uid": step_uid, "action": "add_step", "order": order,
            })))
        }
        "add_affordance" => {
            let node = handle
                .add_node(CreateNode::new(
                    &req.label,
                    NodeProps::Affordance(AffordanceProps {
                        action_name: req.label.clone(),
                        description: desc,
                        affordance_type: req.affordance_type.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": node.uid.to_string(), "action": "add_affordance", "label": req.label,
            })))
        }
        "add_control" => {
            let ctype = req.control_type.clone().unwrap_or_else(|| "conditional".into());
            let ctrl_node = handle
                .add_node(CreateNode::new(
                    &req.label,
                    NodeProps::Control(ControlProps {
                        control_type: ctype,
                        label: Some(req.label.clone()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let ctrl_uid = ctrl_node.uid.to_string();
            if let Some(ref f_uid) = req.flow_uid {
                create_link(&state, f_uid, &ctrl_uid, EdgeType::Controls, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({
                "uid": ctrl_uid, "action": "add_control", "label": req.label,
            })))
        }
        other => Err(bad_request(format!("unknown procedure action: {other}"))),
    }
}

// ============================================================
// Endpoint 9 — POST /action/risk
// ============================================================

#[derive(Deserialize)]
pub(crate) struct RiskRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) assessed_uid: Option<String>,
    #[serde(default)]
    pub(crate) severity: Option<String>,
    #[serde(default)]
    pub(crate) likelihood: Option<f64>,
    #[serde(default)]
    pub(crate) mitigations: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) residual_risk: Option<f64>,
    #[serde(default)]
    pub(crate) filter_uid: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn risk(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RiskRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "assess" => {
            let label = req
                .label
                .clone()
                .unwrap_or_else(|| "Risk Assessment".into());
            let summary = req.description.clone().unwrap_or_else(|| label.clone());
            let residual = req.residual_risk.unwrap_or(0.0);
            let conf = Confidence::new((1.0 - residual).clamp(0.0, 1.0)).map_err(map_err_500)?;
            let mitigation_str = req
                .mitigations
                .as_ref()
                .map(|v| v.join("; "));
            let ra_node = handle
                .add_node(
                    CreateNode::new(
                        &label,
                        NodeProps::RiskAssessment(RiskAssessmentProps {
                            target_uid: req.assessed_uid.clone(),
                            severity: req.severity.clone(),
                            likelihood: req.likelihood,
                            mitigation: mitigation_str,
                            ..Default::default()
                        }),
                    )
                    .confidence(conf)
                    .summary(&summary),
                )
                .await
                .map_err(map_err_500)?;
            let ra_uid = ra_node.uid.to_string();
            if let Some(ref a_uid) = req.assessed_uid {
                create_link(&state, a_uid, &ra_uid, EdgeType::RiskAssessedBy, &req.agent_id)
                    .await?;
            }
            Ok(Json(serde_json::json!({
                "uid": ra_uid, "action": "assess",
            })))
        }
        "get_assessments" => {
            let filter = NodeFilter::new().node_type(NodeType::RiskAssessment);
            let nodes = state.graph.find_nodes(filter).await.map_err(map_err_500)?;
            let results = if let Some(ref f_uid) = req.filter_uid {
                // Filter by edges pointing to the target
                let edges = state
                    .graph
                    .edges_to(Uid::from(f_uid.as_str()), Some(EdgeType::RiskAssessedBy))
                    .await
                    .map_err(map_err_500)?;
                let from_uids: std::collections::HashSet<_> =
                    edges.iter().map(|e| e.from_uid.to_string()).collect();
                nodes
                    .into_iter()
                    .filter(|n| from_uids.contains(&n.uid.to_string()))
                    .collect::<Vec<_>>()
            } else {
                nodes
            };
            Ok(Json(serde_json::to_value(results).unwrap()))
        }
        other => Err(bad_request(format!("unknown risk action: {other}"))),
    }
}

// ============================================================
// Endpoint 10 — POST /memory/session
// ============================================================

#[derive(Deserialize)]
pub(crate) struct SessionOpRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) focus: Option<String>,
    #[serde(default)]
    pub(crate) session_uid: Option<String>,
    #[serde(default)]
    pub(crate) trace_content: Option<String>,
    #[serde(default)]
    pub(crate) trace_type: Option<String>,
    #[serde(default)]
    pub(crate) relevant_node_uids: Option<Vec<String>>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn session_op(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SessionOpRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "open" => {
            let label = req.label.clone().unwrap_or_else(|| "Session".into());
            let focus = req.focus.clone().unwrap_or_default();
            let node = handle
                .add_session(label.clone(), focus.clone())
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": node.uid.to_string(),
                "action": "open",
                "label": label,
            })))
        }
        "trace" => {
            let sess_uid = req
                .session_uid
                .as_deref()
                .ok_or_else(|| bad_request("session_uid required for trace"))?;
            let content = req.trace_content.clone().unwrap_or_default();
            let trace_node = handle
                .add_node(CreateNode::new(
                    &content,
                    NodeProps::Trace(TraceProps {
                        session_uid: Some(sess_uid.to_string()),
                        trace_type: req.trace_type.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let trace_uid = trace_node.uid.to_string();
            // trace → session
            create_link(&state, &trace_uid, sess_uid, EdgeType::CapturedIn, &req.agent_id).await?;
            // trace → relevant nodes
            for rel_uid in req.relevant_node_uids.iter().flatten() {
                create_link(&state, &trace_uid, rel_uid, EdgeType::TraceEntry, &req.agent_id)
                    .await?;
            }
            Ok(Json(serde_json::json!({
                "uid": trace_uid,
                "action": "trace",
            })))
        }
        "close" => {
            let sess_uid = req
                .session_uid
                .clone()
                .ok_or_else(|| bad_request("session_uid required for close"))?;
            let current = state
                .graph
                .get_node(Uid::from(sess_uid.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("session {sess_uid} not found")))?;
            let updated_props = if let NodeProps::Session(mut sp) = current.props {
                sp.ended_at = Some(now());
                Some(NodeProps::Session(sp))
            } else {
                None
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(sess_uid.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id,
                    "session closed".into(),
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": sess_uid,
                "action": "close",
                "version": updated.version,
            })))
        }
        other => Err(bad_request(format!("unknown session action: {other}"))),
    }
}

// ============================================================
// Endpoint 11 — POST /memory/distill
// ============================================================

#[derive(Deserialize)]
pub(crate) struct DistillRequest {
    pub(crate) label: String,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) session_uid: Option<String>,
    #[serde(default)]
    pub(crate) summarizes_uids: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) importance: Option<f64>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn distill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DistillRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    let source_uids = req
        .summarizes_uids
        .clone()
        .unwrap_or_default();

    let mut builder = CreateNode::new(
        &req.label,
        NodeProps::Summary(SummaryProps {
            content: req.content.clone(),
            source_node_uids: source_uids,
            ..Default::default()
        }),
    )
    .summary(&req.content);

    if let Some(imp) = req.importance {
        builder = builder.salience(Salience::new(imp.clamp(0.0, 1.0)).map_err(map_err_500)?);
    }

    let node = handle.add_node(builder).await.map_err(map_err_500)?;
    let uid = node.uid.to_string();

    if let Some(ref sess_uid) = req.session_uid {
        create_link(&state, &uid, sess_uid, EdgeType::CapturedIn, &req.agent_id).await?;
    }
    for sum_uid in req.summarizes_uids.iter().flatten() {
        create_link(&state, &uid, sum_uid, EdgeType::Summarizes, &req.agent_id).await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "uid": uid, "label": req.label })),
    ))
}

// ============================================================
// Endpoint 12 — POST /memory/config
// ============================================================

#[derive(Deserialize)]
pub(crate) struct MemoryConfigRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) key: Option<String>,
    #[serde(default)]
    pub(crate) value: Option<String>,
    #[serde(default)]
    pub(crate) policy_content: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn memory_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<MemoryConfigRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "set_preference" => {
            let label = req
                .label
                .clone()
                .ok_or_else(|| bad_request("label required for set_preference"))?;
            let key = req
                .key
                .clone()
                .ok_or_else(|| bad_request("key required for set_preference"))?;
            let value = req
                .value
                .clone()
                .ok_or_else(|| bad_request("value required for set_preference"))?;
            let node = handle
                .add_preference(label.clone(), key, value)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": node.uid.to_string(), "label": label })))
        }
        "set_policy" => {
            let label = req
                .label
                .clone()
                .ok_or_else(|| bad_request("label required for set_policy"))?;
            let content = req.policy_content.clone().unwrap_or_default();
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::MemoryPolicy(MemoryPolicyProps {
                        condition: Some(content),
                        active: true,
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": node.uid.to_string(), "label": label })))
        }
        "get_preferences" => {
            let nodes = state
                .graph
                .find_nodes(NodeFilter::new().node_type(NodeType::Preference))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(nodes).unwrap()))
        }
        "get_policies" => {
            let nodes = state
                .graph
                .find_nodes(NodeFilter::new().node_type(NodeType::MemoryPolicy))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(nodes).unwrap()))
        }
        other => Err(bad_request(format!("unknown memory config action: {other}"))),
    }
}

// ============================================================
// Endpoint 13 — POST /agent/plan
// ============================================================

#[derive(Deserialize)]
pub(crate) struct AgentPlanRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) goal_uid: Option<String>,
    #[serde(default)]
    pub(crate) task_uid: Option<String>,
    #[serde(default)]
    pub(crate) plan_uid: Option<String>,
    #[serde(default)]
    pub(crate) step_order: Option<u32>,
    #[serde(default)]
    pub(crate) depends_on_uids: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) target_uid: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn agent_plan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentPlanRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "create_task" => {
            let label = req.label.clone().unwrap_or_else(|| "Task".into());
            let desc = req.description.clone().unwrap_or_default();
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Task(TaskProps {
                        description: desc,
                        status: Some("pending".into()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let uid = node.uid.to_string();
            if let Some(ref g_uid) = req.goal_uid {
                create_link(&state, &uid, g_uid, EdgeType::Targets, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({ "uid": uid, "action": "create_task", "label": label })))
        }
        "create_plan" => {
            let label = req.label.clone().unwrap_or_else(|| "Plan".into());
            let desc = req.description.clone().unwrap_or_default();
            let plan_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Plan(PlanProps {
                        description: desc,
                        task_uid: req.task_uid.clone(),
                        status: Some("pending".into()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let uid = plan_node.uid.to_string();
            if let Some(ref t_uid) = req.task_uid {
                create_link(&state, &uid, t_uid, EdgeType::PlannedBy, &req.agent_id).await?;
            }
            if let Some(ref g_uid) = req.goal_uid {
                create_link(&state, &uid, g_uid, EdgeType::Targets, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({ "uid": uid, "action": "create_plan", "label": label })))
        }
        "add_step" => {
            let plan_uid = req
                .plan_uid
                .as_deref()
                .ok_or_else(|| bad_request("plan_uid required for add_step"))?;
            let label = req.label.clone().unwrap_or_else(|| "Step".into());
            let desc = req.description.clone().unwrap_or_default();
            let order = req.step_order.unwrap_or(0);
            let step_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::PlanStep(PlanStepProps {
                        order,
                        description: desc,
                        plan_uid: Some(plan_uid.to_string()),
                        status: Some("pending".into()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let step_uid = step_node.uid.to_string();
            create_link(&state, plan_uid, &step_uid, EdgeType::HasStep, &req.agent_id).await?;
            for dep_uid in req.depends_on_uids.iter().flatten() {
                create_link(&state, &step_uid, dep_uid, EdgeType::DependsOn, &req.agent_id)
                    .await?;
            }
            Ok(Json(serde_json::json!({
                "uid": step_uid, "action": "add_step", "order": order,
            })))
        }
        "update_status" => {
            let target = req
                .target_uid
                .clone()
                .ok_or_else(|| bad_request("target_uid required for update_status"))?;
            let new_status = req
                .status
                .clone()
                .ok_or_else(|| bad_request("status required for update_status"))?;
            // Fetch current node, update status in props
            let current = state
                .graph
                .get_node(Uid::from(target.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("node {target} not found")))?;
            let updated_props = match current.props {
                NodeProps::Task(mut p) => {
                    p.status = Some(new_status.clone());
                    Some(NodeProps::Task(p))
                }
                NodeProps::Plan(mut p) => {
                    p.status = Some(new_status.clone());
                    Some(NodeProps::Plan(p))
                }
                NodeProps::PlanStep(mut p) => {
                    p.status = Some(new_status.clone());
                    Some(NodeProps::PlanStep(p))
                }
                _ => None,
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(target.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id,
                    format!("status updated to {new_status}"),
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": target, "status": new_status, "version": updated.version,
            })))
        }
        "get_plan" => {
            let plan_uid = req
                .plan_uid
                .as_deref()
                .ok_or_else(|| bad_request("plan_uid required for get_plan"))?;
            let plan_node = state
                .graph
                .get_node(Uid::from(plan_uid))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("plan {plan_uid} not found")))?;
            let steps = state
                .graph
                .edges_from(Uid::from(plan_uid), Some(EdgeType::HasStep))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "plan": plan_node,
                "steps": steps,
            })))
        }
        other => Err(bad_request(format!("unknown plan action: {other}"))),
    }
}

// ============================================================
// Endpoint 14 — POST /agent/governance
// ============================================================

#[derive(Deserialize)]
pub(crate) struct GovernanceRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) policy_content: Option<String>,
    #[serde(default)]
    pub(crate) budget_type: Option<String>,
    #[serde(default)]
    pub(crate) budget_limit: Option<f64>,
    #[serde(default)]
    pub(crate) governed_uid: Option<String>,
    #[serde(default)]
    pub(crate) approval_uid: Option<String>,
    #[serde(default)]
    pub(crate) approved: Option<bool>,
    #[serde(default)]
    pub(crate) resolution_note: Option<String>,
    #[serde(default)]
    pub(crate) requires_plan_uid: Option<String>,
    #[serde(default)]
    pub(crate) approval_request: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn governance(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GovernanceRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "create_policy" => {
            let label = req
                .label
                .clone()
                .ok_or_else(|| bad_request("label required for create_policy"))?;
            let content = req.policy_content.clone().unwrap_or_default();
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Policy(PolicyProps {
                        name: label.clone(),
                        description: content,
                        active: true,
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": node.uid.to_string(), "label": label })))
        }
        "set_budget" => {
            let label = req
                .label
                .clone()
                .unwrap_or_else(|| "Safety Budget".into());
            let limit = req.budget_limit.unwrap_or(100.0);
            let node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::SafetyBudget(SafetyBudgetProps {
                        budget_type: req.budget_type.clone(),
                        limit,
                        remaining: limit,
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let uid = node.uid.to_string();
            if let Some(ref gov_uid) = req.governed_uid {
                create_link(&state, &uid, gov_uid, EdgeType::BudgetFor, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({ "uid": uid, "label": label })))
        }
        "request_approval" => {
            let label = req
                .approval_request
                .clone()
                .or_else(|| req.label.clone())
                .unwrap_or_else(|| "Approval Request".into());
            let appr_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Approval(ApprovalProps {
                        target_uid: req.governed_uid.clone(),
                        status: Some("pending".into()),
                        requested_at: Some(now()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let appr_uid = appr_node.uid.to_string();
            if let Some(ref plan_uid) = req.requires_plan_uid {
                create_link(
                    &state,
                    plan_uid,
                    &appr_uid,
                    EdgeType::RequiresApproval,
                    &req.agent_id,
                )
                .await?;
            }
            Ok(Json(serde_json::json!({ "uid": appr_uid, "action": "request_approval" })))
        }
        "resolve_approval" => {
            let appr_uid = req
                .approval_uid
                .clone()
                .ok_or_else(|| bad_request("approval_uid required for resolve_approval"))?;
            let approved = req.approved.unwrap_or(false);
            let current = state
                .graph
                .get_node(Uid::from(appr_uid.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("approval {appr_uid} not found")))?;
            let updated_props = if let NodeProps::Approval(mut ap) = current.props {
                ap.status = Some(if approved { "approved" } else { "denied" }.into());
                ap.decided_at = Some(now());
                ap.reason = req.resolution_note.clone();
                Some(NodeProps::Approval(ap))
            } else {
                None
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(appr_uid.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id,
                    "approval resolved".into(),
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": appr_uid,
                "approved": approved,
                "version": updated.version,
            })))
        }
        "get_pending" => {
            let approvals = state.graph.pending_approvals().await.map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(approvals).unwrap()))
        }
        other => Err(bad_request(format!("unknown governance action: {other}"))),
    }
}

// ============================================================
// Endpoint 15 — POST /agent/execution
// ============================================================

#[derive(Deserialize)]
pub(crate) struct ExecutionRequest {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) plan_uid: Option<String>,
    #[serde(default)]
    pub(crate) executor_uid: Option<String>,
    #[serde(default)]
    pub(crate) execution_uid: Option<String>,
    #[serde(default)]
    pub(crate) outcome: Option<String>,
    #[serde(default)]
    pub(crate) produces_node_uid: Option<String>,
    #[serde(default)]
    pub(crate) error_description: Option<String>,
    #[serde(default)]
    pub(crate) agent_name: Option<String>,
    #[serde(default)]
    pub(crate) agent_role: Option<String>,
    #[serde(default)]
    pub(crate) filter_plan_uid: Option<String>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn execution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecutionRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let handle = state.graph.agent(&req.agent_id);

    match req.action.as_str() {
        "start" => {
            let label = req.label.clone().unwrap_or_else(|| "Execution".into());
            let exec_node = handle
                .add_node(CreateNode::new(
                    &label,
                    NodeProps::Execution(ExecutionProps {
                        description: label.clone(),
                        status: Some("running".into()),
                        started_at: Some(now()),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            let uid = exec_node.uid.to_string();
            if let Some(ref p_uid) = req.plan_uid {
                create_link(&state, &uid, p_uid, EdgeType::ExecutionOf, &req.agent_id).await?;
            }
            if let Some(ref ex_uid) = req.executor_uid {
                create_link(&state, &uid, ex_uid, EdgeType::ExecutedBy, &req.agent_id).await?;
            }
            Ok(Json(serde_json::json!({ "uid": uid, "action": "start" })))
        }
        "complete" => {
            let exec_uid = req
                .execution_uid
                .clone()
                .ok_or_else(|| bad_request("execution_uid required for complete"))?;
            let current = state
                .graph
                .get_node(Uid::from(exec_uid.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("execution {exec_uid} not found")))?;
            let updated_props = if let NodeProps::Execution(mut ep) = current.props {
                ep.status = Some("completed".into());
                ep.completed_at = Some(now());
                if let Some(ref outcome) = req.outcome {
                    ep.description = outcome.clone();
                }
                Some(NodeProps::Execution(ep))
            } else {
                None
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(exec_uid.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id.clone(),
                    "execution completed".into(),
                )
                .await
                .map_err(map_err_500)?;
            if let Some(ref pn_uid) = req.produces_node_uid {
                create_link(&state, &exec_uid, pn_uid, EdgeType::ProducesNode, &req.agent_id)
                    .await?;
            }
            Ok(Json(serde_json::json!({
                "uid": exec_uid, "action": "complete", "version": updated.version,
            })))
        }
        "fail" => {
            let exec_uid = req
                .execution_uid
                .clone()
                .ok_or_else(|| bad_request("execution_uid required for fail"))?;
            let current = state
                .graph
                .get_node(Uid::from(exec_uid.as_str()))
                .await
                .map_err(map_err_500)?
                .ok_or_else(|| not_found(format!("execution {exec_uid} not found")))?;
            let updated_props = if let NodeProps::Execution(mut ep) = current.props {
                ep.status = Some("failed".into());
                ep.error = req.error_description.clone();
                Some(NodeProps::Execution(ep))
            } else {
                None
            };
            let updated = state
                .graph
                .update_node(
                    Uid::from(exec_uid.as_str()),
                    None,
                    None,
                    None,
                    None,
                    updated_props,
                    req.agent_id,
                    "execution failed".into(),
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "uid": exec_uid, "action": "fail", "version": updated.version,
            })))
        }
        "register_agent" => {
            let name = req
                .agent_name
                .clone()
                .ok_or_else(|| bad_request("agent_name required for register_agent"))?;
            let node = handle
                .add_node(CreateNode::new(
                    &name,
                    NodeProps::Agent(AgentProps {
                        name: name.clone(),
                        agent_type: req.agent_role.clone(),
                        ..Default::default()
                    }),
                ))
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": node.uid.to_string(), "name": name })))
        }
        "get_executions" => {
            let mut filter = NodeFilter::new().node_type(NodeType::Execution);
            filter.limit = Some(100);
            let nodes = state.graph.find_nodes(filter).await.map_err(map_err_500)?;
            let results = if let Some(ref fp_uid) = req.filter_plan_uid {
                let edges = state
                    .graph
                    .edges_to(Uid::from(fp_uid.as_str()), Some(EdgeType::ExecutionOf))
                    .await
                    .map_err(map_err_500)?;
                let from_uids: std::collections::HashSet<_> =
                    edges.iter().map(|e| e.from_uid.to_string()).collect();
                nodes
                    .into_iter()
                    .filter(|n| from_uids.contains(&n.uid.to_string()))
                    .collect::<Vec<_>>()
            } else {
                nodes
            };
            Ok(Json(serde_json::to_value(results).unwrap()))
        }
        other => Err(bad_request(format!("unknown execution action: {other}"))),
    }
}

// ============================================================
// Endpoint 16 — POST /retrieve
// ============================================================

#[derive(Deserialize)]
pub(crate) struct RetrieveRequest {
    pub(crate) mode: String,
    #[serde(default)]
    pub(crate) query: Option<String>,
    #[serde(default)]
    pub(crate) k: Option<usize>,
    #[serde(default)]
    pub(crate) threshold: Option<f64>,
    #[serde(default)]
    pub(crate) layer: Option<String>,
    #[serde(default)]
    pub(crate) node_types: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) confidence_min: Option<f64>,
    #[serde(default)]
    pub(crate) salience_min: Option<f64>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
    #[serde(default)]
    pub(crate) offset: Option<u32>,
}

pub(crate) async fn retrieve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetrieveRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let limit = req.limit.unwrap_or(20);
    let offset = req.offset.unwrap_or(0);

    match req.mode.as_str() {
        "text" => {
            let query = req
                .query
                .clone()
                .ok_or_else(|| bad_request("query required for text mode"))?;
            let mut opts = SearchOptions::new();
            if let Some(ref nts) = req.node_types {
                if let Some(first) = nts.first() {
                    opts.node_type = Some(parse_node_type(first));
                }
            }
            if let Some(ref l) = req.layer {
                opts.layer = parse_layer(l);
            }
            opts.limit = Some(limit);
            let results = state
                .graph
                .search(query, opts)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(results).unwrap()))
        }
        "semantic" => {
            let k = req.k.unwrap_or(10);
            Err::<_, (StatusCode, Json<ErrorResponse>)>((
                StatusCode::NOT_IMPLEMENTED,
                Json(ErrorResponse {
                    error: format!(
                        "semantic text retrieval (k={k}) requires a configured embedding provider; \
                         use POST /embeddings/search with a pre-computed vector"
                    ),
                }),
            ))
        }
        "active_goals" => {
            let goals = state.graph.active_goals().await.map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(goals).unwrap()))
        }
        "open_questions" => {
            let questions = state.graph.open_questions().await.map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(questions).unwrap()))
        }
        "weak_claims" => {
            let threshold = req.threshold.unwrap_or(0.6);
            let claims = state
                .graph
                .weak_claims(threshold)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(claims).unwrap()))
        }
        "pending_approvals" => {
            let approvals = state.graph.pending_approvals().await.map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(approvals).unwrap()))
        }
        "unresolved_contradictions" => {
            let contrs = state
                .graph
                .unresolved_contradictions()
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(contrs).unwrap()))
        }
        "layer" => {
            let layer_str = req
                .layer
                .as_deref()
                .ok_or_else(|| bad_request("layer required for layer mode"))?;
            let layer = parse_layer(layer_str)
                .ok_or_else(|| bad_request(format!("unknown layer: {layer_str}")))?;
            let page = state
                .graph
                .nodes_in_layer_paginated(layer, Pagination { limit, offset })
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(page).unwrap()))
        }
        "recent" => {
            let mut filter = NodeFilter::new();
            filter.limit = Some(limit);
            filter.offset = Some(offset);
            if let Some(c_min) = req.confidence_min {
                filter.confidence_min = Some(c_min);
            }
            if let Some(s_min) = req.salience_min {
                filter.salience_min = Some(s_min);
            }
            let page = state
                .graph
                .find_nodes_paginated(filter)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(page).unwrap()))
        }
        other => Err(bad_request(format!("unknown retrieve mode: {other}"))),
    }
}

// ============================================================
// Endpoint 17 — POST /traverse
// ============================================================

#[derive(Deserialize)]
pub(crate) struct TraverseRequest {
    pub(crate) mode: String,
    pub(crate) start_uid: String,
    #[serde(default)]
    pub(crate) end_uid: Option<String>,
    #[serde(default)]
    pub(crate) max_depth: Option<u32>,
    #[serde(default)]
    pub(crate) direction: Option<String>,
    #[serde(default)]
    pub(crate) edge_types: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) weight_threshold: Option<f64>,
}

pub(crate) async fn traverse(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TraverseRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let start = Uid::from(req.start_uid.as_str());
    let max_depth = req.max_depth.unwrap_or(5);

    let opts = TraversalOptions {
        max_depth,
        direction: parse_direction(req.direction.as_deref()),
        edge_types: req
            .edge_types
            .as_ref()
            .map(|v| v.iter().map(|s| parse_edge_type(s)).collect()),
        weight_threshold: req.weight_threshold,
    };

    match req.mode.as_str() {
        "chain" => {
            let steps = state
                .graph
                .reasoning_chain(start, max_depth)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "mode": "chain",
                "start_uid": req.start_uid,
                "steps": steps,
            })))
        }
        "neighborhood" => {
            let steps = state
                .graph
                .reachable(start, opts)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "mode": "neighborhood",
                "start_uid": req.start_uid,
                "steps": steps,
            })))
        }
        "path" => {
            let end_uid = req
                .end_uid
                .as_deref()
                .ok_or_else(|| bad_request("end_uid required for path mode"))?;
            let path = state
                .graph
                .find_path(start, Uid::from(end_uid), opts)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "mode": "path",
                "start_uid": req.start_uid,
                "end_uid": end_uid,
                "steps": path,
            })))
        }
        "subgraph" => {
            let (nodes, edges) = state
                .graph
                .subgraph(start, opts)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({
                "mode": "subgraph",
                "start_uid": req.start_uid,
                "nodes": nodes,
                "edges": edges,
            })))
        }
        other => Err(bad_request(format!("unknown traverse mode: {other}"))),
    }
}

// ============================================================
// Endpoint 18 — POST /evolve
// ============================================================

#[derive(Deserialize)]
pub(crate) struct EvolveRequest {
    pub(crate) action: String,
    pub(crate) uid: String,
    #[serde(default)]
    pub(crate) label: Option<String>,
    #[serde(default)]
    pub(crate) summary: Option<String>,
    #[serde(default)]
    pub(crate) confidence: Option<f64>,
    #[serde(default)]
    pub(crate) salience: Option<f64>,
    #[serde(default)]
    pub(crate) props_patch: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) reason: Option<String>,
    #[serde(default)]
    pub(crate) cascade: Option<bool>,
    #[serde(default)]
    pub(crate) half_life_secs: Option<f64>,
    #[serde(default)]
    pub(crate) min_salience: Option<f64>,
    #[serde(default)]
    pub(crate) min_age_secs: Option<f64>,
    #[serde(default)]
    pub(crate) version: Option<i64>,
    #[serde(default = "default_agent")]
    pub(crate) agent_id: String,
}

pub(crate) async fn evolve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EvolveRequest>,
) -> Result<impl axum::response::IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let uid = Uid::from(req.uid.as_str());
    let reason = req.reason.clone().unwrap_or_else(|| "updated via /evolve".into());

    match req.action.as_str() {
        "update" => {
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

            // Merge props_patch if provided
            let merged_props = if let Some(patch) = &req.props_patch {
                let current = state
                    .graph
                    .get_node(uid.clone())
                    .await
                    .map_err(map_err_500)?
                    .ok_or_else(|| not_found(format!("node {} not found", req.uid)))?;
                let node_type = current.node_type.clone();
                let mut base = current.props.to_json();
                // Merge patch into base
                if let (Some(base_map), Some(patch_obj)) = (base.as_object_mut(), patch.as_object()) {
                    for (k, v) in patch_obj {
                        base_map.insert(k.clone(), v.clone());
                    }
                }
                let rebuilt = mindgraph::NodeProps::from_json(&node_type, &base)
                    .map_err(map_err_500)?;
                Some(rebuilt)
            } else {
                None
            };

            let updated = state
                .graph
                .update_node(
                    uid,
                    req.label.clone(),
                    req.summary.clone(),
                    conf,
                    sal,
                    merged_props,
                    req.agent_id,
                    reason,
                )
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(updated).unwrap()))
        }
        "tombstone" => {
            if req.cascade.unwrap_or(false) {
                let result = state
                    .graph
                    .tombstone_cascade(uid, reason, req.agent_id)
                    .await
                    .map_err(map_err_500)?;
                Ok(Json(serde_json::to_value(result).unwrap()))
            } else {
                state
                    .graph
                    .tombstone(uid, reason, req.agent_id)
                    .await
                    .map_err(map_err_500)?;
                Ok(Json(serde_json::json!({ "uid": req.uid, "action": "tombstone" })))
            }
        }
        "restore" => {
            state
                .graph
                .restore(uid)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": req.uid, "action": "restore" })))
        }
        "decay" => {
            let half_life = req.half_life_secs.unwrap_or(86400.0);
            let result = state
                .graph
                .decay_salience(half_life)
                .await
                .map_err(map_err_500)?;
            let auto_tombstoned = if let (Some(min_sal), Some(min_age)) =
                (req.min_salience, req.min_age_secs)
            {
                Some(
                    state
                        .graph
                        .auto_tombstone(min_sal, min_age)
                        .await
                        .map_err(map_err_500)?,
                )
            } else {
                None
            };
            Ok(Json(serde_json::json!({
                "nodes_decayed": result.nodes_decayed,
                "below_threshold": result.below_threshold,
                "auto_tombstoned": auto_tombstoned,
            })))
        }
        "history" => {
            let history = state
                .graph
                .node_history(uid)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::to_value(history).unwrap()))
        }
        "snapshot" => {
            let version = req
                .version
                .ok_or_else(|| bad_request("version required for snapshot action"))?;
            let record = state
                .graph
                .node_at_version(uid, version)
                .await
                .map_err(map_err_500)?;
            match record {
                Some(r) => Ok(Json(serde_json::to_value(r).unwrap())),
                None => Err(not_found(format!(
                    "node {} version {} not found",
                    req.uid, version
                ))),
            }
        }
        "tombstone_edge" => {
            state
                .graph
                .tombstone_edge(uid, reason, req.agent_id)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": req.uid, "action": "tombstone_edge" })))
        }
        "restore_edge" => {
            state
                .graph
                .restore_edge(uid)
                .await
                .map_err(map_err_500)?;
            Ok(Json(serde_json::json!({ "uid": req.uid, "action": "restore_edge" })))
        }
        other => Err(bad_request(format!("unknown evolve action: {other}"))),
    }
}
