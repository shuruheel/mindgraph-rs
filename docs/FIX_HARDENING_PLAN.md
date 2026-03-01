# MindGraph Hardening Plan (No Changes Applied Yet)

Date: 2026-03-01
Status: Planning only

## Scope
This document outlines a staged plan to address known reliability/security issues identified during review. It does not apply code changes.

## Goals
- Eliminate deadlock risk in event callback dispatch.
- Prevent silent data loss from props serialization fallback behavior.
- Remove script-injection risk from dynamic filter field interpolation.
- Preserve compatibility for `mindgraph-server` clients (Openclaw and upcoming MCP server).

## Non-Goals
- No schema redesign.
- No endpoint removals.
- No large API shape changes in one release.

## Risk Inventory
1. Event callback deadlock risk when callbacks mutate subscription state.
2. Silent props serialization fallback to `{}` on serialization failures.
3. Dynamic Cozo script field interpolation without strict field validation.
4. CI quality drift (`clippy -D warnings` currently fails in tests).

## Phased Plan

### Phase 0: Guardrails First
Deliverables:
- Add regression tests before implementation:
  - callback re-entrancy/unsubscribe during event delivery
  - invalid filter field rejection behavior
  - invalid props payload behavior (including non-finite numeric values)
- Add compatibility tests for Openclaw-critical endpoints.

Acceptance criteria:
- Tests fail on current behavior where expected and define target behavior explicitly.

### Phase 1: Event Dispatch Locking Fix
Implementation outline:
- Refactor dispatch so callbacks are executed without holding the `subscribers` lock.
- Preserve callback order and `SubscriptionId` semantics.
- Keep event payload format unchanged.

Acceptance criteria:
- No deadlocks in concurrent subscribe/unsubscribe + mutation stress tests.
- Existing event tests remain green.

Potential compatibility impact:
- None expected for Node/Openclaw client.

### Phase 2: Fail-Fast Props Serialization
Implementation outline:
- Replace lossy `unwrap_or_default()` paths in props serialization with fallible serialization.
- Return typed errors to callers instead of silently storing empty props.
- In `mindgraph-server`, return explicit 4xx code for invalid props payloads.

Acceptance criteria:
- Invalid payloads fail with deterministic error code.
- No successful write path can drop props silently.

Potential compatibility impact:
- Medium: clients that currently send malformed payloads may start receiving errors.

Client updates required (Openclaw Node client):
- Validate write payloads before sending.
- Surface structured server errors to logs/telemetry.
- Treat input-validation failures as non-retryable.

### Phase 3: Filter Field Validation Hardening
Implementation outline:
- Validate dynamic property field names against strict allowlist (e.g. `^[A-Za-z0-9_]+$`).
- Reject invalid field names with stable error code (e.g. `invalid_filter_field`).
- Avoid direct interpolation of unvalidated field names into script text.

Acceptance criteria:
- Invalid names are rejected consistently.
- Fuzz tests with malicious field strings cannot alter query structure.

Potential compatibility impact:
- Low to medium if clients send arbitrary field names.

Client updates required (Openclaw Node client / MCP):
- Pre-validate field names client-side.
- Sanitize or block unsupported key paths.

### Phase 4: Compatibility Rollout
Rollout strategy:
- Ship behind a compatibility flag where feasible (especially fail-fast serialization).
- Stage deployment:
  1. local + integration tests
  2. canary environment
  3. production rollout
- Monitor error-rate deltas and rejected-request metrics.

Acceptance criteria:
- No increase in 5xx.
- 4xx increases are explainable and attributable to payload validation.
- Openclaw memory workflows remain functional.

### Phase 5: CI and Maintenance Hygiene
Deliverables:
- Resolve existing clippy warnings in tests.
- Add `cargo clippy --all-targets --all-features -- -D warnings` to CI gating.

Acceptance criteria:
- CI passes with strict linting and test suite.

## Work Breakdown (Suggested)
- Week 1: Phase 0 + Phase 1
- Week 2: Phase 2 + client compatibility patch
- Week 3: Phase 3 + staged rollout
- Week 4: Phase 5 + retrospective

## Success Metrics
- Deadlocks observed: 0
- Silent props drops: 0
- Injection-style filter test pass rate: 100%
- Openclaw production regressions: 0
- CI green on strict lint + tests

## Open Questions
- Should fail-fast serialization be released behind a temporary env toggle?
- Do we want to support dotted/json-path keys or only flat keys?
- Which MCP operations, if any, expose free-form filter fields?
