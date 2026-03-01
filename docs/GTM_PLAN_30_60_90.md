# MindGraph GTM Plan (30-60-90 Days)

Date: 2026-03-01
Focus: `mindgraph-server` as production memory backend for agentic systems, with MCP compatibility for Claude Desktop workflows.

## Strategic Positioning
MindGraph should be positioned as:
- Typed memory infrastructure for multi-agent applications.
- Operationally practical graph memory (embedded, fast, local-first, auditable).
- A bridge between agent runtime behavior and explainable memory state.

Core differentiation:
- Strong schema semantics (typed node/edge ontology).
- Versioning + provenance + tombstone lifecycle.
- Unified graph + retrieval + traversal + eventing.
- MCP-friendly integration path for desktop/agent tooling.

## ICP (Ideal Customer Profile)
Primary:
- AI product teams building multi-step assistants with long-lived memory.
- Enterprise internal tools teams needing auditable agent memory.
- Applied AI consultancies delivering custom copilots.

Secondary:
- OSS agent-framework maintainers.
- Research/ops teams building reasoning traceability layers.

## Packaging and Offers
1. OSS Core: Rust crate + server.
2. Integration Offer: Openclaw + MCP implementation blueprint.
3. Consulting Offer:
- Memory architecture assessment (2 weeks)
- Pilot implementation (4-6 weeks)
- Production hardening + observability package (ongoing)

## Messaging
Primary message:
- "Make agent memory typed, inspectable, and production-safe."

Supporting points:
- Stop shipping brittle JSON blobs as memory.
- Gain traceability with provenance and version history.
- Keep deployment simple (embedded/local-first, no heavyweight graph ops team needed).

Proof themes to publish:
- Latency and throughput benchmarks for common memory operations.
- Real examples of memory correctness improvements.
- Openclaw + MCP end-to-end demo outcomes.

## Channel Strategy
1. Developer credibility:
- Technical write-ups on architecture decisions and failure modes.
- Demo repos and reproducible examples.

2. Demand capture:
- Landing page with "Book architecture review" CTA.
- Short discovery form: stack, use case, timeline.

3. Distribution:
- LinkedIn/X founder-led technical posts.
- Hacker News/Reddit only when accompanied by strong artifact (benchmark, demo, incident postmortem).
- Direct outreach to teams already shipping Claude/OpenAI-based internal copilots.

## 30-60-90 Execution Plan

## Day 0-30: Foundation and Proof
Objectives:
- Create market-ready narrative and proof assets.
- Tighten product trust baseline before broad outreach.

Deliverables:
1. Product narrative page (problem -> architecture -> outcomes).
2. Two technical case studies:
- "Openclaw memory architecture with MindGraph"
- "MCP integration with typed graph memory"
3. Reliability hardening plan published and tracked (see `FIX_HARDENING_PLAN.md`).
4. One benchmark report with reproducible scripts.
5. Outreach list of 50 target contacts (founders, AI leads, solution architects).

KPIs:
- 2 publishable artifacts live.
- 10 inbound conversations or intros.
- 3 technical discovery calls booked.

## Day 31-60: Pipeline and Pilot Conversions
Objectives:
- Convert technical interest into paid pilots.
- Build referenceable design-partner relationships.

Deliverables:
1. Pilot package one-pager (scope, timeline, deliverables, pricing band).
2. Demo environment with scripted scenario walkthrough.
3. Client integration kit:
- API patterns
- failure-handling playbook
- observability dashboard template
4. Weekly founder-led outreach cadence.

KPIs:
- 5-8 qualified opportunities.
- 2 design partners engaged.
- 1 paid pilot closed.

## Day 61-90: Credibility Flywheel
Objectives:
- Turn pilots into references and repeatable sales motion.
- Position for consulting + role opportunities.

Deliverables:
1. Public (or anonymized) pilot outcome report.
2. "Production checklist" content for agent-memory systems.
3. Partner conversations with ecosystems (Anthropic, AWS SI channels, graph/vector vendors).
4. Interview-ready portfolio pack:
- architecture deck
- benchmark summary
- incident/risk management story

KPIs:
- 2+ strong references.
- 2 paid engagements in motion.
- 1-2 strategic partner/channel conversations progressing.

## Outreach Targets (Priority Order)
1. Teams already building Claude Desktop or MCP-enabled workflows.
2. Applied AI consultancies delivering enterprise copilots.
3. AI platform teams with memory/reasoning audit requirements.
4. Graph/vector DB adjacent teams for partnerships.

## Consulting Funnel Design
Top of funnel:
- Technical content + demos + OSS credibility.

Middle:
- 45-minute architecture assessment call.
- Rapid memory maturity scorecard.

Bottom:
- Paid pilot SOW with explicit acceptance criteria:
  - memory correctness
  - latency budget
  - incident response path

## Risks and Mitigations
1. Risk: "Impressive tech, weak distribution"
- Mitigation: weekly content + direct outreach cadence with CTA.

2. Risk: integration friction for prospects
- Mitigation: reference Node client + MCP starter kits.

3. Risk: reliability concerns in production memory layer
- Mitigation: execute hardening plan and publish reliability posture.

## Weekly Operating Cadence
- Monday: pipeline review + outreach targets.
- Tuesday-Wednesday: technical artifact creation.
- Thursday: demos/discovery calls.
- Friday: KPI review and plan adjustment.

## Immediate Next Actions (Next 14 Days)
1. Finalize and publish two technical artifacts.
2. Build a 10-minute narrated demo (Openclaw + mindgraph-server + MCP).
3. Start outbound to first 20 high-fit contacts.
4. Create pilot SOW template and pricing framework.
5. Instrument metrics dashboard (inbound, calls, opportunities, pilots).
