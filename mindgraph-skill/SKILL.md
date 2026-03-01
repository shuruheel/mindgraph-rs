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

## Before Responding to Any Substantive Question

If a session is not already open, open one before composing your response —
not after. This means calling track_session (action: open) as your first
action, before writing anything to the user.

A response written outside an open session will not be captured. If a question
is substantive enough to answer thoughtfully, it is substantive enough to
capture. The only exceptions are brief clarifying exchanges, greetings, and
simple factual lookups that require no reasoning.

## Session Lifecycle (required)

Every substantive conversation:

1. OPEN — call track_session (action: open) with a focus describing the session
   intent. Do this before composing your response.
2. LOAD CONTEXT — call retrieve (action: active_goals), then retrieve
   (action: open_questions). Read results before responding — what is already
   in the graph may change what you say.
3. WORK — follow the research and knowledge protocols below.
4. CLOSE — call distill summarizing what was learned with UIDs of significant
   nodes created, then call track_session (action: close).

Never skip open or close. Never compose a substantive response before step 1.

## Web Research Protocol

When doing web research or working through a topic using live sources:

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
genuinely reusable ideas, not passing observations.

## Internal Knowledge Protocol

When answering from training knowledge rather than live sources, the same
obligations apply — the source is just different:

**CLAIMS** — argue the central claims with evidence_type "domain_knowledge" and
an explicit confidence score reflecting your certainty. Low confidence should
be surfaced as an open question, not stated as fact.

**INSIGHTS** — crystallize structural insights, patterns, models, or mechanisms
that would be valuable to retrieve in future sessions. If you are synthesizing
a substantive answer, at least one crystallize call is usually warranted.

**GAPS** — when you notice genuine uncertainty, a domain boundary, or something
that warrants external verification, call inquire (action: open_question).
These are often more valuable than the claims themselves.

**ENTITIES** — resolve named entities before referencing them in nodes, even
when drawing on internal knowledge.

The difference from web research is that you do not call ingest (there is no
external source to record). Everything else is the same.

## Argument as Connective Tissue

argue is the only tool that automatically creates graph edges as a side effect
of creation. crystallize, inquire, commit, and plan produce nodes that are
orphaned unless connected.

After calling crystallize or inquire, immediately create an argue node that
references the new node via extends_uid or in the evidence. This is what makes
concepts, patterns, and open questions reachable via traversal rather than
isolated islands discoverable only by text search.

The canonical pattern:
  argue (central claim) → crystallize (structural insight) → argue (extends
  central claim, references crystallized concept in evidence)

## Write Discipline

**Before calling commit (action: goal) or configure (set_preference /
set_policy)**, narrate what you are about to write and give the user a chance
to confirm before the graph is updated.

**The test for whether to write something:** would the user want to find this
in three months? If no, do not write it. A casual exchange does not require the
full protocol — use judgment proportional to the significance of what is being
discussed. The pre-response gate applies regardless; judgment applies to how
much you write, not whether you open a session.

## Retrieval Before Responses

After opening a session and loading active goals and open questions, call
retrieve (action: text) with a query relevant to the user's question before
composing your response. Prior graph knowledge may contain relevant claims,
open questions, or crystallized insights that should inform what you say.

## Tool Sequence Reference

Canonical session flow:
open → retrieve active_goals → retrieve open_questions → retrieve text →
[work] → distill → close

Web research session:
resolve_entity → ingest source → ingest snippet → argue → crystallize →
argue (extends, referencing crystallized node) → inquire → distill → close

Internal knowledge session:
retrieve text → argue (domain_knowledge evidence) → crystallize →
argue (extends, referencing crystallized node) → inquire (gaps) →
distill → close
