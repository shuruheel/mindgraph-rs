---
name: mindgraph
description: "Behavioral protocol for the mindgraph MCP knowledge graph.
  Loads when mindgraph MCP tools are available. Teaches Claude the session
  lifecycle, research protocol, and write discipline for maintaining a
  persistent semantic knowledge graph across conversations."
---

# mindgraph Protocol

You have access to a persistent knowledge graph via the mindgraph MCP tools.
This graph survives across all conversations. Treat it as a second mind — a
living epistemic record, not a logging system.

## Session Lifecycle (required)

Every conversation:

1. OPEN — call track_session (action: open) with a focus describing the session intent
2. LOAD CONTEXT — call retrieve (action: active_goals), then retrieve
   (action: open_questions). Read results before responding to anything.
3. WORK — follow the research and write protocols below
4. CLOSE — call distill summarizing what was learned with UIDs of significant
   nodes created, then call track_session (action: close)

Never skip open or close. Never skip loading context — what is already in the
graph may change what you say.

## Research Protocol

When doing web research or working through a topic with the user:

**SOURCES** — for every URL consulted or document referenced, call ingest
(action: source) before citing it. For significant extracted passages, call
ingest (action: snippet) with the source_uid linking it to the parent source.

**ENTITIES** — before creating any node referencing a named person,
organization, concept, or place, call resolve_entity to find or create the
canonical entity. Never create duplicate entity nodes.

**CLAIMS** — when asserting something non-trivial that the user might want to
revisit, verify, or build on later, call argue with a confidence score and at
least one piece of evidence. Not for every statement — for claims that do
genuine epistemic work.

**GAPS** — when something is unknown, contradictory, or worth investigating in
a future session, call inquire (action: open_question or anomaly). These
surface automatically at the start of future sessions.

**INSIGHTS** — when a pattern, mechanism, or structural idea emerges that would
be valuable across future conversations, call crystallize. Reserve this for
genuinely reusable ideas — not passing observations.

## Write Discipline

**Before calling commit (action: goal) or configure (set_preference /
set_policy)**, narrate what you are about to write and give the user a chance
to confirm before the graph is updated.

**The test for whether to write something to the graph:** would the user want
to find this in three months? If no, do not write it. A casual question does
not require the full research protocol — use judgment proportional to the
significance of what is being discussed.

## Retrieval Before Responses

For any question where prior graph knowledge might be relevant, call retrieve
(action: text) with a query before responding. A cache miss is fine — the
habit of checking is what matters.

## Tool Sequence Reference

The canonical session flow:
open → retrieve active_goals → retrieve open_questions → [work: ingest, argue,
inquire, crystallize as appropriate] → distill → close

For research sessions specifically:
resolve_entity (named things) → ingest (source) → ingest (snippet) → argue
(claims from source) → inquire (gaps identified) → crystallize (reusable
patterns)
