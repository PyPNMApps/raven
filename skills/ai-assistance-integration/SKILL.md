---
name: ai-assistance-integration
description: Build or debug optional AI analysis in applications, including browser authentication, exact-payload approval, bounded streaming, safe diagnostics and live-provider verification. Use when changing AI connection or analysis behavior, not for unrelated application work.
---

# Optional AI assistance integration

Preserve the target application's scope and existing calculation owners. Apply
this skill when implementing or investigating provider authentication, model
catalogs, request approval, streamed reports, transcripts or their diagnostics.
Read [integration lessons](references/integration-lessons.md) for observed RAVEN
failures, transferable checks and the local source map. These are implementation
lessons, not universal provider limits or permission to change another repository.

## Establish the current contract

Read the target checkout's architecture and reusable owners before adding code.
Verify current authentication, endpoint, model-catalog and streaming contracts
against official provider documentation. Subscription authorization and API keys
are distinct paths; do not assume one provider supports another's login protocol,
reuse a coding-agent token, or silently fall back to separately billed access.

Keep optional analysis separate from offline functionality. For read-only
scientific analysis, existing numerical owners remain authoritative; AI explains
allowlisted results and cannot change simulation inputs or recalculate metrics.
Other application scopes require their own action and authorization contracts.

## Separate the response budgets

Use independent, named budgets for token replies, catalog metadata, individual
stream events, total transferred bytes, event count and final accepted reports.
Set bounds from the endpoint's purpose and observed evidence; do not treat one
small HTTP-body limit as suitable for every endpoint. Keep deadlines, idle read
timeouts, cancellation and allocation bounds explicit.

Consume SSE incrementally across arbitrary network chunks and supported newline
forms. Discard partial/delta events after validation, retaining only the bounded
completed output required by the application. Require terminal success and retain
failure, refusal, interruption, tool-policy and late-result rejection. Do not
accept partial text or weaken final report limits to accommodate transport overhead.

## Protect approval and credentials

Use the provider's supported browser flow with validated state, PKCE and identity
where applicable. Keep credentials in their existing authorized owner; changing
storage policy requires a concrete product decision. Preserve fixed destinations,
TLS verification and redirect policy. Never log tokens, authorization codes,
callback URLs, headers, questions or provider response bodies.

Freeze the exact outbound payload, destination and model before approval. Bind
single-use approval and asynchronous results to the current configuration and
source generation. Context, question, account/model changes and cancellation must
invalidate stale approval. Validate any cited evidence IDs, values, units and scope
against the approved immutable evidence; attach provenance locally.

## Diagnose and verify

Distinguish browser callback, token exchange, identity/consent, catalog loading,
inference transport and report validation. Use sanitized stage/status codes and
byte/event counts. Initialize the application's existing logger; establish where
output is actually captured before claiming a log was inspected.

Test with synthetic credentials and local transports first. Cover both observed
and documented envelopes, rich catalogs, split UTF-8/CRLF chunks, long streams
with small reports, boundary overflow, missing/duplicate completion, late failures,
secret reflection, invalid references, cancellation and stale queued results.
Use real UI adapters and native builds when affected; label mocked/offscreen and
real-account evidence separately. Do not trigger paid or plan-consuming retries
automatically. A live success requires the user's completed authorized request.

Update the target project's AI architecture, requirements-to-test mapping and user
instructions as implementation changes. Record confirmed causes separately from
suspicions; promote new lessons only when evidence supports them.
