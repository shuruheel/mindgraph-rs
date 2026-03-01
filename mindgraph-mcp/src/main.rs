use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt,
    model::{
        CallToolResult, Content, Implementation, ListToolsResult, ServerCapabilities, ServerInfo,
        Tool,
    },
    service::RequestContext,
    transport::io::stdio,
};
use rmcp::model::{CallToolRequestParams, PaginatedRequestParams};
use serde_json::Value;

// ─────────────────────────────────────────────────────────────────────────────
// Core proxy struct
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct MindgraphMcp {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MindgraphMcp {
    async fn post(&self, path: &str, body: Value) -> CallToolResult {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }
        match req.send().await {
            Err(e) => CallToolResult::error(vec![Content::text(format!("HTTP error: {e}"))]),
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    CallToolResult::success(vec![Content::text(text)])
                } else {
                    CallToolResult::error(vec![Content::text(format!("HTTP {status}: {text}"))])
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ServerHandler implementation
// ─────────────────────────────────────────────────────────────────────────────

impl ServerHandler for MindgraphMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mindgraph".into(),
                version: "0.1.0".into(),
                title: None,
                description: Some(
                    "Semantic memory graph for agentic systems".into(),
                ),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Use track_session open at conversation start, retrieve active_goals + open_questions for context, \
                 distill + track_session close at conversation end."
                    .into(),
            ),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult {
            tools: all_tools(),
            next_cursor: None,
            meta: None,
        }))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        let path = match request.name.as_ref() {
            "ingest" => "/reality/ingest",
            "resolve_entity" => "/reality/entity",
            "argue" => "/epistemic/argument",
            "inquire" => "/epistemic/inquiry",
            "crystallize" => "/epistemic/structure",
            "commit" => "/intent/commitment",
            "deliberate" => "/intent/deliberation",
            "design_procedure" => "/action/procedure",
            "assess_risk" => "/action/risk",
            "track_session" => "/memory/session",
            "distill" => "/memory/distill",
            "configure" => "/memory/config",
            "plan" => "/agent/plan",
            "govern" => "/agent/governance",
            "execute" => "/agent/execution",
            "retrieve" => "/retrieve",
            "traverse" => "/traverse",
            "evolve" => "/evolve",
            other => {
                return Err(McpError::invalid_params(
                    format!("unknown tool: {other}"),
                    None,
                ))
            }
        };
        Ok(self.post(path, args).await)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool helper
// ─────────────────────────────────────────────────────────────────────────────

fn mk_tool(name: &'static str, desc: &'static str, schema: Value) -> Tool {
    let obj = match schema {
        Value::Object(m) => m,
        _ => panic!("schema must be an object"),
    };
    Tool::new(name, desc, Arc::new(obj))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool catalogue
// ─────────────────────────────────────────────────────────────────────────────

fn all_tools() -> Vec<Tool> {
    vec![
        // ── ingest ───────────────────────────────────────────────────────────
        mk_tool(
            "ingest",
            "Call when you encounter information worth preserving: a URL, document, user \
             statement, or observation. Use `action: source` for a web page or document (set \
             `url` and `medium`), `snippet` for an extracted passage (requires `source_uid`), \
             or `observation` for an ephemeral fact. Ingest sources before creating arguments \
             or claims that cite them.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["source", "snippet", "observation"],
                        "description": "What to create: source, snippet, or observation"
                    },
                    "label": { "type": "string", "description": "Short title" },
                    "url": { "type": "string", "description": "URL for sources" },
                    "medium": {
                        "type": "string",
                        "description": "Medium type e.g. webpage, pdf, video"
                    },
                    "content": { "type": "string", "description": "Textual content or description" },
                    "source_uid": {
                        "type": "string",
                        "description": "Parent source UID (required for snippet)"
                    },
                    "confidence": { "type": "number", "description": "Confidence 0.0–1.0" },
                    "salience": { "type": "number", "description": "Salience 0.0–1.0" },
                    "agent_id": { "type": "string", "description": "Agent identity (optional)" }
                }
            }),
        ),

        // ── resolve_entity ───────────────────────────────────────────────────
        mk_tool(
            "resolve_entity",
            "Call when creating, finding, or deduplicating named things (people, organizations, \
             concepts, places). Use `action: create` for new entities, `alias` to add a name \
             variant, `resolve` for exact name lookup, `fuzzy_resolve` when unsure of the \
             canonical name, `merge` to consolidate duplicates. Resolve before creating nodes \
             that reference an entity by name.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create", "alias", "resolve", "fuzzy_resolve", "merge"],
                        "description": "Operation to perform"
                    },
                    "name": { "type": "string", "description": "Entity name (for create)" },
                    "entity_type": {
                        "type": "string",
                        "description": "Entity subtype e.g. Person, Organization, Place"
                    },
                    "text": {
                        "type": "string",
                        "description": "Name/text to look up or add as alias (for alias/resolve/fuzzy_resolve)"
                    },
                    "canonical_uid": {
                        "type": "string",
                        "description": "UID of the canonical entity (for alias)"
                    },
                    "alias_score": {
                        "type": "number",
                        "description": "Alias confidence 0.0–1.0 (for alias)"
                    },
                    "keep_uid": {
                        "type": "string",
                        "description": "UID of entity to keep (for merge)"
                    },
                    "merge_uid": {
                        "type": "string",
                        "description": "UID of entity to merge away (for merge)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results (for fuzzy_resolve, default 5)"
                    },
                    "agent_id": { "type": "string", "description": "Agent identity (optional)" }
                }
            }),
        ),

        // ── argue ─────────────────────────────────────────────────────────────
        mk_tool(
            "argue",
            "Call when forming a structured epistemic argument: a claim supported by evidence \
             with a warrant explaining the inference. Provide `claim` (content + optional \
             confidence), `evidence[]` (description + evidence_type), and optionally `warrant`, \
             `refutes_uid` to counter an existing claim, or `extends_uid` to build on one. \
             Use to assert something with explicit, revisable reasoning rather than an inline \
             assertion.",
            serde_json::json!({
                "type": "object",
                "required": ["claim"],
                "properties": {
                    "claim": {
                        "type": "object",
                        "required": ["label", "content"],
                        "properties": {
                            "label": { "type": "string", "description": "Short title for the claim" },
                            "content": { "type": "string", "description": "Full claim statement" },
                            "confidence": { "type": "number" }
                        }
                    },
                    "evidence": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["label", "description"],
                            "properties": {
                                "label": { "type": "string", "description": "Short title for this piece of evidence" },
                                "description": { "type": "string", "description": "Evidence detail" },
                                "evidence_type": {
                                    "type": "string",
                                    "description": "e.g. empirical, anecdotal, statistical"
                                }
                            }
                        }
                    },
                    "warrant": {
                        "type": "object",
                        "description": "Logical bridge between evidence and claim",
                        "properties": {
                            "label": { "type": "string", "description": "Short title for the warrant" },
                            "principle": { "type": "string", "description": "The inferential principle that links evidence to claim" }
                        }
                    },
                    "argument": {
                        "type": "object",
                        "description": "Optional wrapping argument node grouping claim + evidence",
                        "properties": {
                            "label": { "type": "string" },
                            "summary": { "type": "string" }
                        }
                    },
                    "refutes_uid": {
                        "type": "string",
                        "description": "UID of claim this argument refutes"
                    },
                    "extends_uid": {
                        "type": "string",
                        "description": "UID of claim this argument extends"
                    },
                    "source_uids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs of ingested sources supporting the claim"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── inquire ───────────────────────────────────────────────────────────
        mk_tool(
            "inquire",
            "Call when recording a gap, anomaly, or open question. Use \
             `action: hypothesis` for a testable explanation, `question` or `open_question` \
             for unresolved inquiries, `anomaly` when observations contradict existing \
             knowledge, `assumption` to make an implicit premise explicit. These persist \
             across sessions and are surfaced at conversation start via \
             `retrieve action: open_questions`.",
            serde_json::json!({
                "type": "object",
                "required": ["action", "content"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "hypothesis", "theory", "paradigm", "anomaly",
                            "assumption", "question", "open_question"
                        ],
                        "description": "Type of epistemic inquiry node to create"
                    },
                    "content": { "type": "string", "description": "The inquiry text" },
                    "label": { "type": "string", "description": "Short title" },
                    "confidence": { "type": "number" },
                    "salience": { "type": "number" },
                    "related_uid": {
                        "type": "string",
                        "description": "UID of a related node"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── crystallize ───────────────────────────────────────────────────────
        mk_tool(
            "crystallize",
            "Call to encode a durable structural insight: a concept definition (`concept`), \
             recurring pattern (`pattern`), causal mechanism (`mechanism`), mental model \
             (`model`), analogy between domains (`analogy`), inference chain \
             (`inference_chain`), theorem (`theorem`), or equation (`equation`). Use when \
             you want a reusable epistemic primitive that can be referenced and combined \
             across sessions.",
            serde_json::json!({
                "type": "object",
                "required": ["action", "label", "content"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "concept", "pattern", "mechanism", "model",
                            "analogy", "inference_chain", "theorem", "equation"
                        ],
                        "description": "Type of structural knowledge to crystallize"
                    },
                    "label": { "type": "string", "description": "Name of the concept/pattern" },
                    "content": {
                        "type": "string",
                        "description": "Detailed description or definition (used as the node body)"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Optional shorter summary; falls back to content if omitted"
                    },
                    "confidence": { "type": "number" },
                    "related_uids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs of related nodes to link"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── commit ─────────────────────────────────────────────────────────────
        mk_tool(
            "commit",
            "Call when a goal, project, or milestone is established. Use `action: goal` for \
             objectives, `project` for multi-step efforts, `milestone` for concrete \
             checkpoints. Link to parent commitments via `parent_uid` and motivating beliefs \
             via `motivated_by_uid`. Active goals are surfaced at session start via \
             `retrieve action: active_goals`.",
            serde_json::json!({
                "type": "object",
                "required": ["action", "label", "description"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["goal", "project", "milestone"],
                        "description": "Type of commitment to create"
                    },
                    "label": { "type": "string", "description": "Goal/project/milestone title" },
                    "description": { "type": "string", "description": "Description and acceptance criteria" },
                    "priority": {
                        "type": "string",
                        "description": "Priority level e.g. high, medium, low"
                    },
                    "status": {
                        "type": "string",
                        "description": "Initial status (default: active for goals)"
                    },
                    "parent_uid": {
                        "type": "string",
                        "description": "UID of parent goal or project (creates DecomposesInto edge)"
                    },
                    "motivated_by_uid": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs of motivating beliefs or claims"
                    },
                    "due_date": {
                        "type": "number",
                        "description": "Unix timestamp deadline (for milestone)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── deliberate ────────────────────────────────────────────────────────
        mk_tool(
            "deliberate",
            "Call to work through a decision. First `open_decision` to create the decision \
             node, then `add_option` for each alternative, `add_constraint` for limiting \
             factors, and `resolve` to record the outcome with `resolution_rationale`. Use \
             `get_open` to retrieve pending decisions that need resolution.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["open_decision", "add_option", "add_constraint", "resolve", "get_open"],
                        "description": "Decision lifecycle step"
                    },
                    "label": { "type": "string", "description": "Decision or option title" },
                    "summary": { "type": "string", "description": "Description" },
                    "decision_uid": {
                        "type": "string",
                        "description": "UID of the decision node (for add_option/add_constraint/resolve)"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description of the decision, option, or constraint being added"
                    },
                    "constraint_type": {
                        "type": "string",
                        "description": "Type of constraint e.g. budget, technical, legal (for add_constraint)"
                    },
                    "blocks_uid": {
                        "type": "string",
                        "description": "UID of option this constraint blocks (for add_constraint)"
                    },
                    "informs_uid": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs this option informs (for add_option)"
                    },
                    "chosen_option_uid": {
                        "type": "string",
                        "description": "UID of the chosen option (for resolve)"
                    },
                    "resolution_rationale": {
                        "type": "string",
                        "description": "Explanation of why this option was chosen"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── design_procedure ──────────────────────────────────────────────────
        mk_tool(
            "design_procedure",
            "Call when planning a repeatable process. Use `create_flow` to define the \
             workflow (optionally linked to a goal via `goal_uid`), `add_step` to add \
             ordered actions (set `step_order`), `add_affordance` to name available \
             capabilities, `add_control` for conditional branches. Steps can reference \
             affordances via `uses_affordance_uids`.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create_flow", "add_step", "add_affordance", "add_control"],
                        "description": "Procedure design operation"
                    },
                    "label": { "type": "string", "description": "Flow/step/affordance title" },
                    "summary": { "type": "string" },
                    "description": {
                        "type": "string",
                        "description": "Description of the flow, step, affordance, or control"
                    },
                    "flow_uid": {
                        "type": "string",
                        "description": "UID of the parent flow (for add_step/add_affordance/add_control)"
                    },
                    "goal_uid": {
                        "type": "string",
                        "description": "UID of the related goal (for create_flow)"
                    },
                    "step_order": {
                        "type": "integer",
                        "description": "Ordering index for the step"
                    },
                    "previous_step_uid": {
                        "type": "string",
                        "description": "UID of the preceding step (creates DependsOn edge)"
                    },
                    "uses_affordance_uids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Affordance UIDs this step uses"
                    },
                    "affordance_type": {
                        "type": "string",
                        "description": "Type of affordance e.g. tool, api, capability (for add_affordance)"
                    },
                    "control_type": {
                        "type": "string",
                        "description": "Type of control node e.g. conditional, loop, branch (for add_control)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── assess_risk ───────────────────────────────────────────────────────
        mk_tool(
            "assess_risk",
            "Call when identifying risks to a plan, decision, or proposed action before \
             proceeding. Provide `assessed_uid` (the node being assessed), `likelihood` \
             (0.0–1.0), `severity` (low/medium/high/critical), `mitigations[]`, and \
             `residual_risk` (0.0–1.0 after mitigations). Use `get_assessments` to retrieve \
             existing risk assessments for a given node.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["assess", "get_assessments"],
                        "description": "Assess a risk or retrieve existing assessments"
                    },
                    "assessed_uid": {
                        "type": "string",
                        "description": "UID of the node being assessed"
                    },
                    "label": { "type": "string", "description": "Risk title" },
                    "likelihood": {
                        "type": "number",
                        "description": "Probability 0.0–1.0"
                    },
                    "severity": {
                        "type": "string",
                        "enum": ["low", "medium", "high", "critical"]
                    },
                    "mitigations": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of mitigation actions"
                    },
                    "residual_risk": {
                        "type": "number",
                        "description": "Remaining risk after mitigations 0.0–1.0"
                    },
                    "filter_uid": {
                        "type": "string",
                        "description": "Filter get_assessments to those linked to this node UID"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── track_session ─────────────────────────────────────────────────────
        mk_tool(
            "track_session",
            "Call at the start of every conversation (`open` — provide a `focus` summary \
             describing the session intent), periodically to record key reasoning steps \
             (`trace` — requires `session_uid`, provide `trace_content`), and at \
             conversation end (`close` — requires `session_uid`). The session lifecycle is: \
             open → retrieve active_goals + open_questions → work → trace significant \
             insights → close → distill.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["open", "trace", "close"],
                        "description": "Session lifecycle step"
                    },
                    "label": { "type": "string", "description": "Session title (for open)" },
                    "focus": {
                        "type": "string",
                        "description": "High-level intent of this session (for open)"
                    },
                    "session_uid": {
                        "type": "string",
                        "description": "UID of the open session (for trace/close)"
                    },
                    "trace_content": {
                        "type": "string",
                        "description": "Reasoning step or insight to record (for trace)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── distill ────────────────────────────────────────────────────────────
        mk_tool(
            "distill",
            "Call at conversation end or when compressing a set of nodes into a durable \
             summary. Provide `label` (title), `content` (the distilled insight), \
             `summarizes_uids[]` (the nodes being summarized), and `importance` (0.0–1.0 \
             salience). Link to the current session via `session_uid`. Always call just \
             before `track_session close`.",
            serde_json::json!({
                "type": "object",
                "required": ["label", "content", "summarizes_uids"],
                "properties": {
                    "label": { "type": "string", "description": "Summary title" },
                    "content": { "type": "string", "description": "Distilled insight text" },
                    "summarizes_uids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs of nodes being summarized"
                    },
                    "importance": {
                        "type": "number",
                        "description": "Salience/importance 0.0–1.0"
                    },
                    "session_uid": {
                        "type": "string",
                        "description": "UID of the current session (CapturedIn edge)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── configure ─────────────────────────────────────────────────────────
        mk_tool(
            "configure",
            "Call to set or retrieve persistent preferences or memory policies. Use \
             `set_preference` with `key`/`value` to record user preferences, `set_policy` \
             with `policy_content` to define memory retention rules, `get_preferences` to \
             retrieve current preferences, `get_policies` to list active policies.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["set_preference", "set_policy", "get_preferences", "get_policies"],
                        "description": "Configuration operation"
                    },
                    "key": {
                        "type": "string",
                        "description": "Preference key (for set_preference)"
                    },
                    "value": {
                        "type": "string",
                        "description": "Preference value (for set_preference)"
                    },
                    "policy_content": {
                        "type": "string",
                        "description": "Policy rule text (for set_policy)"
                    },
                    "label": {
                        "type": "string",
                        "description": "Policy label (for set_policy)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── plan ───────────────────────────────────────────────────────────────
        mk_tool(
            "plan",
            "Call when breaking work into an actionable plan. Use `create_task` to define \
             work items, `create_plan` to group them (link to goal via `goal_uid`), \
             `add_step` to sequence actions with `depends_on_uids`, `update_status` to \
             track progress (pending/in_progress/completed/failed), `get_plan` to retrieve \
             the current plan structure including all steps.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "create_task", "create_plan", "add_step",
                            "update_status", "get_plan"
                        ],
                        "description": "Planning operation"
                    },
                    "label": { "type": "string", "description": "Task/plan/step title" },
                    "description": { "type": "string", "description": "Detail about the task, plan, or step" },
                    "goal_uid": {
                        "type": "string",
                        "description": "UID of the related goal (for create_task/create_plan)"
                    },
                    "task_uid": {
                        "type": "string",
                        "description": "UID of the task this plan implements (for create_plan)"
                    },
                    "plan_uid": {
                        "type": "string",
                        "description": "UID of the parent plan (for add_step/get_plan)"
                    },
                    "target_uid": {
                        "type": "string",
                        "description": "UID of the task or plan step to update (for update_status)"
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "failed"],
                        "description": "New status (for update_status)"
                    },
                    "depends_on_uids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "UIDs of prerequisite steps (for add_step)"
                    },
                    "step_order": { "type": "integer" },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── govern ─────────────────────────────────────────────────────────────
        mk_tool(
            "govern",
            "Call to establish safety guardrails or require human approval before sensitive \
             actions. Use `create_policy` for rules, `set_budget` for resource limits, \
             `request_approval` to pause and await authorization (provide `approval_request` \
             description), `resolve_approval` to record a decision (provide `approval_uid`, \
             `approved` bool, `resolution_note`). Use `get_pending` to check for open \
             approvals.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "create_policy", "set_budget",
                            "request_approval", "resolve_approval", "get_pending"
                        ],
                        "description": "Governance operation"
                    },
                    "label": { "type": "string", "description": "Policy/budget title" },
                    "policy_content": {
                        "type": "string",
                        "description": "Policy rule text (for create_policy)"
                    },
                    "budget_type": {
                        "type": "string",
                        "description": "Budget type e.g. tokens, api_calls (for set_budget)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Budget limit value (for set_budget)"
                    },
                    "approval_request": {
                        "type": "string",
                        "description": "Description of what needs approval (for request_approval)"
                    },
                    "approval_uid": {
                        "type": "string",
                        "description": "UID of the approval node (for resolve_approval)"
                    },
                    "approved": {
                        "type": "boolean",
                        "description": "Approval decision (for resolve_approval)"
                    },
                    "resolution_note": {
                        "type": "string",
                        "description": "Explanation of the decision (for resolve_approval)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── execute ────────────────────────────────────────────────────────────
        mk_tool(
            "execute",
            "Call when initiating or tracking a plan execution. Use `start` to begin \
             (provide `plan_uid`, optionally `executor_uid`), `complete` to record success \
             (provide `execution_uid`, `outcome`), `fail` to record failure (provide \
             `execution_uid`, `error_description`), `register_agent` to log a participant \
             agent (provide `agent_name`, `agent_role`). Use `get_executions` to retrieve \
             history.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "complete", "fail", "register_agent", "get_executions"],
                        "description": "Execution lifecycle operation"
                    },
                    "plan_uid": {
                        "type": "string",
                        "description": "UID of the plan to execute (for start)"
                    },
                    "executor_uid": {
                        "type": "string",
                        "description": "UID of the executing agent node (for start)"
                    },
                    "execution_uid": {
                        "type": "string",
                        "description": "UID of the execution node (for complete/fail)"
                    },
                    "outcome": {
                        "type": "string",
                        "description": "Result description (for complete)"
                    },
                    "error_description": {
                        "type": "string",
                        "description": "Error details (for fail)"
                    },
                    "agent_name": {
                        "type": "string",
                        "description": "Agent name to register (for register_agent)"
                    },
                    "agent_role": {
                        "type": "string",
                        "description": "Role of the agent (for register_agent)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── retrieve ───────────────────────────────────────────────────────────
        mk_tool(
            "retrieve",
            "Call BEFORE responding to any factual query or when you need context from the \
             knowledge graph. At session start: call with `action: active_goals` then \
             `action: open_questions`. For content queries: `action: text` with a `query` \
             string. Also: `weak_claims` (uncertain beliefs below a confidence threshold), \
             `pending_approvals`, `unresolved_contradictions`, `layer` (all nodes in a \
             layer with optional `limit`/`offset`), `recent`.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "text", "semantic", "active_goals", "open_questions",
                            "weak_claims", "pending_approvals",
                            "unresolved_contradictions", "layer", "recent"
                        ],
                        "description": "Retrieval mode"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (for text/semantic)"
                    },
                    "layer": {
                        "type": "string",
                        "description": "Layer name (for layer mode): reality, epistemic, intent, action, memory, agent"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default 20)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Pagination offset (default 0)"
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Confidence threshold for weak_claims (default 0.5)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── traverse ───────────────────────────────────────────────────────────
        mk_tool(
            "traverse",
            "Call when exploring how concepts relate. Use `action: chain` to follow \
             epistemic reasoning from a node (max_depth configurable), `neighborhood` to \
             discover connected concepts, `path` to find how two nodes are connected \
             (requires `end_uid`), `subgraph` to extract a self-contained cluster. Use \
             `direction` (incoming/outgoing/both) and `edge_types[]` to filter traversal.",
            serde_json::json!({
                "type": "object",
                "required": ["action", "start_uid"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["chain", "neighborhood", "path", "subgraph"],
                        "description": "Traversal mode"
                    },
                    "start_uid": {
                        "type": "string",
                        "description": "Starting node UID"
                    },
                    "end_uid": {
                        "type": "string",
                        "description": "Target node UID (required for path)"
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["incoming", "outgoing", "both"],
                        "description": "Edge direction to follow (default: outgoing)"
                    },
                    "edge_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter by edge types e.g. Supports, Refutes, DecomposesInto"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum traversal depth (default 3)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),

        // ── evolve ─────────────────────────────────────────────────────────────
        mk_tool(
            "evolve",
            "Call to update, archive, or query history of any node or edge. Use `update` \
             to modify label/summary/confidence/salience/props_patch (provide `uid`), \
             `tombstone` to soft-delete (reversible; `cascade: true` also removes edges), \
             `restore` to undo, `decay` to reduce salience of stale nodes across the graph, \
             `history` to audit all versions of a node, `snapshot` to retrieve a specific \
             past version (provide `version` number), \
             `tombstone_edge`/`restore_edge` for edges.",
            serde_json::json!({
                "type": "object",
                "required": ["action"],
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "update", "tombstone", "restore", "decay",
                            "history", "snapshot",
                            "tombstone_edge", "restore_edge"
                        ],
                        "description": "Evolution operation"
                    },
                    "uid": {
                        "type": "string",
                        "description": "Node UID (for update/tombstone/restore/history/snapshot)"
                    },
                    "label": { "type": "string", "description": "New label (for update)" },
                    "summary": { "type": "string", "description": "New summary (for update)" },
                    "confidence": { "type": "number", "description": "New confidence (for update)" },
                    "salience": { "type": "number", "description": "New salience (for update)" },
                    "props_patch": {
                        "type": "object",
                        "description": "Partial props fields to merge (for update)"
                    },
                    "cascade": {
                        "type": "boolean",
                        "description": "Also tombstone connected edges (for tombstone)"
                    },
                    "version": {
                        "type": "integer",
                        "description": "Version number to snapshot (for snapshot)"
                    },
                    "from_uid": {
                        "type": "string",
                        "description": "Source node UID of edge (for tombstone_edge/restore_edge)"
                    },
                    "to_uid": {
                        "type": "string",
                        "description": "Target node UID of edge (for tombstone_edge/restore_edge)"
                    },
                    "edge_type": {
                        "type": "string",
                        "description": "Edge type (for tombstone_edge/restore_edge)"
                    },
                    "decay_factor": {
                        "type": "number",
                        "description": "Salience reduction factor 0.0–1.0 (for decay)"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for the change (audit trail)"
                    },
                    "agent_id": { "type": "string" }
                }
            }),
        ),
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // CRITICAL: tracing must write to stderr — stdout is MCP JSON-RPC wire
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let base_url = std::env::var("MINDGRAPH_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:18790".into());
    let api_key = std::env::var("MINDGRAPH_API_KEY").unwrap_or_default();

    let svc = MindgraphMcp {
        client: reqwest::Client::new(),
        base_url,
        api_key,
    };

    tracing::info!("mindgraph-mcp starting (stdio)");
    let server = svc.serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
