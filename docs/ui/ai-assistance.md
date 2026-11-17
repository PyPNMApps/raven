# AI Assistance

[Desktop guide](README.md) · [Documentation index](../README.md)

AI Assistance reviews **OFDM Profile Performance** results. MERLIN/Rust supply
all calculations; AI adds optional, read-only commentary. Normal viewing and
simulation work without an AI account.

## Try Demo AI without a key

Start a local demonstration session:

```bash
cd /home/dev01/Projects/raven &&
./scripts/qt-desktop.sh --ai-demo
```

For the already-built development binary:

```bash
cd /home/dev01/Projects/raven &&
./scripts/qt-desktop.sh --ai-demo --skip-build
```

Run **OFDM Profile Performance → FEC Performance**, then select **AI Analysis…**.
Select **Preview request**, then **Run approved demo**. The report and chat log
are marked **DEMO**; exported exchanges carry `demo: true` and `provider: demo`.
No credentials, model or provider tokens are used. The question is recorded but
not interpreted by a language model; the response is built from existing Rust
findings and their exact references. This tests the workflow, not an AI connection
or a genuine AI assessment. To ask this chat for an actual interpretation, export
the conversation and share the JSON here yourself.

`--ai-demo` locks the session to local demo responses. Restart without it for
live AI. Normal sessions also offer a **Demo AI** checkbox in the analysis popup;
switching modes invalidates approval and never sends a request automatically.
Existing session credentials are not erased by that checkbox. Normal release
update checks are independent of AI and may still contact GitHub.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="images/ai/ofdm-ai-demo-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="images/ai/ofdm-ai-demo-light.png">
  <img src="images/ai/ofdm-ai-demo-dark.png" alt="Local Demo AI report from real OFDM evidence, with no model or provider usage">
</picture>

## Configure and test OpenAI

1. Select **AI Assistance · Off** on the top bar, or **Settings → System… → AI…**.
2. Choose **OpenAI**. Enter an exact model ID supporting Chat Completions and
   structured JSON output, plus your [OpenAI API key](https://platform.openai.com/api-keys).
   A ChatGPT subscription does not itself specify your API model access.
3. Paste the key into the masked field, or use **Open key file…** to select a
   `.txt`/`.key` file containing only the key on one line, up to 4096 bytes.
4. Select **Save for this session**, then reopen the popup and select
   **Test connection**. This retrieves metadata for the saved model, with no PNM
   data or inference prompt. Success confirms key/model visibility, not inference
   compatibility, remaining quota or correctness of future answers.

Credentials remain independent per provider, in Rust-owned session memory.
Fields clear on submit, provider change and close. To change a configured model,
leave the key blank to retain its session key. **Disable AI Assistance** retains
settings; **Forget provider** removes that provider's settings without revoking
its key or deleting a selected key file. Closing RAVEN discards credentials.
Gemini, Claude and Grok currently support setup only; Kiro is deferred.

## Ask about an OFDM run

1. Run **OFDM Profile Performance → FEC Performance** with your chosen profiles.
2. In the workspace, select **AI Analysis…**. Enter your question.
3. Select **Preview request**. Review the destination, model and **complete exact
   JSON body** in Request preview. Local findings remain separate from AI output.
4. Select **Analyze approved request** to send that frozen request once. API usage
   may incur charges. No request is sent merely by opening a popup or preview.
5. Read **AI report** for advisory observations, labeled hypotheses, limitations,
   suggested checks and evidence references. Provider-reported token counts are
   shown when supplied; they are not a cost estimate or invoice.

Changing the question, provider/key or source run invalidates approval. A new
preview and explicit approval are required. Follow-up questions use the current
run only; previous answers are **not** sent implicitly. Reopen the popup to read
the current report without making a new request.

The evidence includes original engine counters, exact decimal count strings,
per-profile scopes, effective transmission settings and their origins, source
capture hashes and the pinned MERLIN revision. It excludes MAC addresses, paths,
filenames, raw captures, sample arrays, payloads and source error text. The question
is sent as entered, so review it for private information. Narrow-band RxMER
summaries are not included; whole-run counters cannot localize an impairment.

**Cancel** or closing the analysis popup requests cancellation. A blocked network
operation may take up to the 60-second timeout to acknowledge it; already-sent
requests may still be processed/billed. Late results cannot attach to a replaced
run. Errors preserve your original numerical results and offline findings.

## Session chat log and export

The Chat log retains up to **20 accepted exchanges for the current run**, each
with its question, validated report, exact reference values, provenance, usage and
request digest. The oldest accepted exchange is removed at the limit. Changing
the run clears this session log. Rejected/raw responses are not retained.

Use **Export conversation…** to write a new JSON file explicitly; existing files
are never replaced. This export contains the evidence and accepted commentary,
not API keys. Review it before sharing. Automatic saving, a saved-history browser,
failed/cancelled-turn records, previous-turn context selection, OS-vault credentials
and token pricing/session totals remain planned.

## Screens

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="images/ai/ai-services-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="images/ai/ai-services-light.png">
  <img src="images/ai/ai-services-dark.png" alt="RAVEN AI provider setup and explicit model-access test">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="images/ai/ofdm-ai-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="images/ai/ofdm-ai-light.png">
  <img src="images/ai/ofdm-ai-dark.png" alt="Exact OFDM AI request preview from a native fixture-backed run; no provider call">
</picture>
