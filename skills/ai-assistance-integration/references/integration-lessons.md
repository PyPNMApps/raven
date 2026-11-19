# AI integration lessons from RAVEN

Recorded 2026-11-17. Copy this reference with its parent skill when reusing it.
Provider details can change; verify official documentation before implementation.
No credentials, account IDs, private payloads or actual provider bodies belong here.

## Authentication success does not mean the connection is ready

RAVEN wrote only nonsecret registration metadata after verifying identity. Sign-in
then failed at the model catalog. The initial message incorrectly mentioned an
OFDM evidence contract because generic parser diagnostics escaped the auth owner.

Transferable lesson: identify the failing phase before changing credentials or
report schemas. Record a completed stage and sanitize the next failure. A saved
registration establishes that some identity verification succeeded; it does not
prove successful inference, billing entitlement or the latest request's completion.

## Catalog metadata needs its own budget

A user screenshot confirmed that model-catalog retrieval exceeded the shared
256 KiB authentication-response limit before catalog JSON parsing. RAVEN now uses
an independent 8 MiB catalog budget. Token replies remain small; scientific
report limits did not change. A synthetic 350 KB capability-rich catalog reproduces
the coupling and verifies the fix, exact boundary and overflow rejection.

Both the subscription guide's `models`/`slug` catalog and the standard API's
`object=list`, `data`/`id` form have explicit parser coverage. The failed raw
catalog was not retained, so its exact envelope was not confirmed. Do not describe
the alternate envelope as the established cause of the size-limit failure.

The specific byte and entry counts are RAVEN application choices, not provider
requirements. Preserve server model IDs, validate unsafe/duplicate entries, and
avoid inventing entitlement or selecting a model based on an API-key string.

## Streaming overhead is not report size

After successful sign-in, the analysis screen showed the Responses endpoint and
selected model but failed at the 256 KiB HTTP response budget. The previous code
buffered the entire SSE stream before processing it. This establishes an analysis
transport-limit failure, not an oversized or invalid final scientific report.

SSE can repeat text in delta, done and terminal events. Consume the stream event
by event, keeping only the completed response needed by the report validator.
RAVEN's current budgets are 1 MiB/event, 16 MiB total transfer and 32,768 events;
the final JSON report remains at 32 KiB. These values are independent and bounded.

Regression tests must reproduce a stream larger than the old HTTP budget while
its completed report is small, including arbitrary chunk boundaries and CRLF.
Also test failed/incomplete events, truncated streams, duplicate completion,
credential reflection even in discarded events, cancellation and each limit.
Do not fix the issue by accepting a partial report or disabling all size checks.

## Structured output is not the complete application contract

After the transfer-limit fix, a user screenshot showed a generic contract
rejection. No response body or saved runtime log was available; the exact failed
rule is unconfirmed. Source review found that the prompt omitted local rules:
one profile per claim, bounded item counts and UTF-8 byte limits. Prompt v2 states
those rules while the immutable-evidence validator remains authoritative.

Provider-enforced JSON Schema does not enforce every semantic application rule.
A syntactically valid report can still cite a missing metric, change its copied
value/unit/scope, mix profiles within one claim or exceed a local byte limit.
Mirror those rules in the prompt and test multi-profile success with separate
claims. Never weaken validation or silently repair returned claims to get success.

Give stream framing, completed output, report JSON/schema/limits and reference
checks separate static diagnostics. Log only developer-owned rule/category text;
serde error strings may quote untrusted field names or values. Synthetic tests
must assert that diagnostic messages exclude attacker-controlled content. A
clearer diagnostic is not evidence that a live request now succeeds. Changes to
prompt instructions require a new preview and prompt provenance version.

## Logs must exist before they can help

RAVEN had a shared `logging` crate but the Qt entry point had not initialized it.
New AI errors appeared in the popup and terminal; no saved runtime log existed.
The fix initializes the existing logger and emits safe phase/status/count events.

A process writing stderr to a terminal does not imply a logfile exists. In a
checkout, an explicit console capture can be used (paths are project-specific):

```bash
RUST_LOG=warn,viewer::ai=info ./scripts/qt-desktop.sh --skip-build 2>&1 | tee local/raven-ai.log
```

Never enable broad HTTP trace logging to debug authentication. Other components
may log unrelated paths; review captured logs before sharing. No logging change
retroactively recovers a failed session's tokens or response body.

## Native browser and runtime boundaries matter

The system browser performs sign-in; the native backend performs approved HTTPS
analysis. A loopback callback belongs to the machine running the application.
For a VM, use a browser inside that VM unless a separately designed forwarding
flow is available. Do not replace loopback with a public listener to work around it.

OAuth tokens and existing chat history are not interchangeable. RAVEN stores
tokens in memory, only nonsecret host/client registration metadata on disk, and
requires explicit reauthorization on expiry. Another product must choose and
verify its own credential persistence, refresh and account-switching design.

## Preserve the scientific and approval contracts

RAVEN reuses Rust/MERLIN calculations, immutable evidence and exact numeric
references. Model commentary is advisory. The same exact-body preview, single-use
approval, provenance, plain-text rendering and transcript export rules apply to
API-key, subscription and local demo routes.

Demo output is deterministic local commentary, not a live model or a bridge to
the developer's existing chat subscription. A launch-enforced demo lock must be
checked in the backend, independently of editable UI properties.

## Reuse map in RAVEN

- `apps/viewer/src/ai/chatgpt/login.rs`: browser callback, verified identity/catalog.
- `apps/viewer/src/ai/chatgpt/mod.rs`: private credentials and catalog/auth budgets.
- `apps/viewer/src/ai/chatgpt/responses.rs`: bounded SSE and completed envelope.
- `apps/viewer/src/ai/protocol.rs`: exact approval and immutable evidence validation.
- `apps/viewer/src/ai/transport.rs`: fixed HTTPS dispatch, no fallback/retry.
- `apps/qt/src/ai/session.rs`: worker lifecycle and stale-result rejection.
- `docs/development/ai-assistance.md`: current design and validation boundaries.
- `docs/ui/ai-assistance.md`: user sign-in, demo, analysis and troubleshooting.

These are source pointers, not dependencies for another repository's skill use.
Inspect the target project's own owners and licenses before copying implementation.

## Official reference starting points

- [OpenAI subscription sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in)
- [Subscription models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)
- [Models API](https://developers.openai.com/api/reference/python/resources/models/methods/list)
- [Streaming responses](https://developers.openai.com/api/docs/guides/streaming-responses)

## Approval must also be understandable

A raw escaped request body was difficult to read and the Qt report views lacked
reliable bounded-width scrolling. Keep a human-readable review derived from the
same frozen evidence, with unchanged values, units and scopes; retain exact bytes
in an explicit technical view. Test real scroll position and long wrapped text
in both themes. Each enabled analysis needs a versioned prepared question and a
reviewed evidence allowlist. Editing/selecting a question invalidates approval;
it never sends automatically. Document future analysis prompts without implying
their adapters or upload paths exist.

## Rich reports must not relax the existing AI contract

RAVEN's native engineering report schema is separate from its provider narrative
and evidence schemas. Adapt only accepted narratives, repeat exact-reference
validation and retain the verified plain-text fallback. A rich renderer is not
permission to accept arbitrary provider JSON or enable new upload paths.

Resolve data/evidence IDs against one immutable local owner/generation; source
classification and numbers come from Rust. Use a finite component registry, not
model-authored QML paths. Reject HTML/remote images/URLs before Qt Markdown renders
anything, and make headings, tables and buttons explicitly plain text. Check both
per-block limits and aggregate GUI work, including nested tables/plots.

Qt-specific lesson: CXX bridge Rust state type names can collide across modules
when exported without distinct C++ namespaces. Use an unambiguous state name for
each new bridge. Validate native resources and decoder plugins, not just QML mocks.
Adding a Markdown parser also requires checking current advisories before pinning.

When using a custom ScrollBar in a native ScrollView, verify its actual geometry,
orientation, thumb position and drag behavior. A passing contentY test alone can
leave a tiny nonfunctional indicator. Bind the custom bar to the scroll viewport
and test dragging against a long document. Native visual review must use the newly
built binary; --skip-build deliberately reuses the previous executable.


Keep connection UI separate from key/model editing. A common Connect action can
route to different supported methods, while unsupported providers stay clearly
setup-only. Saving configuration must not silently connect, test inference or
claim verification. Clear draft secrets and native-picker state when either the
child form or its owning popup closes, provider changes or demo mode activates;
ignore late picker callbacks. Keep stored keys in the existing credential owner.
