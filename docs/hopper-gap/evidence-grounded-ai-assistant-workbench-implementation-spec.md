# Evidence-Grounded AI Assistant Workbench Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim and it does not enable AI, MCP, external model calls, or agentic mutation
in the current product.

Purpose: define how Damsel should build a Hopper-competitive AI-assisted
analysis surface without turning reverse engineering into an ungrounded chat
pane. Hopper now publicly markets AI capabilities through an integrated MCP
server. Damsel's answer should be an evidence-grounded assistant that works
through the same project API, routes, facts, source maps, confidence records,
capability grants, activity logs, and privacy rules as the rest of the
Workbench.

Semantic-pack companion:
[Apple SDK, Header, And Semantic Packs Implementation Spec](apple-sdk-semantic-packs-implementation-spec.md)
defines how Apple SDK/XNU/objc4/dyld/Swift ABI/private header declarations
become versioned, privacy-aware evidence. The assistant may cite those
declarations, but it must distinguish declaration hints from accepted binary
matches and must request explicit capability before reading private packs.

## Why This Spec Exists

Hopper's homepage says it is enhanced with AI capabilities through an integrated
MCP server and can connect to preferred language models for analysis. Hopper's
FAQ says MCP configuration is available through Hopper Help. The official Model
Context Protocol specification defines a standard way for LLM applications to
connect to resources, prompts, and tools, with explicit security and trust
principles around user consent, data privacy, tool safety, sampling controls,
logging, cancellation, progress, and error reporting.

The competitive implication is larger than "add a chat sidebar."

An AI assistant in a reverse-engineering tool can:

- summarize functions
- explain ObjC selectors and message sends
- triage kernel/panic frames
- compare artifacts
- propose names and comments
- identify suspicious control flow
- collect evidence packages
- drive batch queries
- propose patch plans
- explain debugger stops

But it can also:

- hallucinate source code
- overstate weak evidence
- leak private binaries
- hide tool calls
- mutate annotations without consent
- confuse original and patched revisions
- mislead analysts about decompiler output
- expose local paths, dSYMs, KDKs, panic logs, or memory bytes

Therefore Damsel needs an AI Workbench architecture that treats AI as a user
interface over evidence, not as an oracle.

## Companion Documents

Read this spec with:

- [Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Patch Studio, Assembler, And Safe Binary Revision Workbench Implementation Spec](patch-studio-assembler-workbench-implementation-spec.md)
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
- [Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec](investigation-casebook-session-timeline-implementation-spec.md)
- [Quality And Validation Plan](quality-validation-plan.md)

The casebook companion defines where assistant outputs go after generation:
assistant responses can draft hypotheses, findings, notebook snippets, and
report sections, but accepted claims must still link to case evidence pins and
review records.

## Product Thesis

The assistant should answer with evidence objects, not just text.

Every assistant response should be able to carry:

- project id
- source id
- source revision id
- selected routes
- tool calls
- facts used
- source maps used
- confidence levels
- unresolved assumptions
- generated suggestions
- proposed mutations
- evidence package links
- privacy classification
- model/provider metadata
- prompt/context summary

The user should be able to click any claim and see:

- the address/function/object behind it
- the instructions or bytes behind it
- the analysis fact behind it
- the provenance of the fact
- whether it came from Damsel analysis, user annotation, debug info, runtime
  observation, plugin output, or model inference
- whether the assistant is proposing or asserting

If a claim has no route, fact, or evidence, it should be labeled as inference.

## Non-Goals

Do not implement first:

- autonomous background analysis agents
- automatic mutation of project facts
- external model calls by default
- uploading raw binaries without explicit consent
- model-driven patch application
- model-driven debugger attach/launch/control
- model-driven type/signature edits without review
- hidden prompts that include private data
- raw source/byte/memory dumps into prompts by default
- replacing deterministic analyzers with model guesses
- treating assistant text as completion evidence

The first assistant should be local-to-project, explicit, read-mostly,
evidence-citing, and review-driven.

## Workbench UX

### Assistant Surface

The assistant should be a Workbench pane with:

- conversation thread
- active context summary
- selected routes
- tool-call timeline
- evidence tray
- suggestions tray
- capability status
- privacy indicator
- model/provider indicator
- activity link

It should not be a full-screen chat app. It should feel like a reverse
engineering assistant embedded in the document.

### Context Bar

The context bar shows:

- current project
- current source/revision
- selected function/block/instruction/object
- active representation
- selected runtime/debug/pseudocode context
- included artifacts
- excluded artifacts
- privacy mode
- model mode

The user can remove context before sending.

### Evidence Tray

The evidence tray lists:

- instructions
- byte ranges
- pseudocode lines
- graph nodes
- ObjC objects
- C++ objects
- kernel objects
- debug/source lines
- problems
- runtime observations
- exports

Each evidence item has:

- route
- confidence
- provenance
- privacy class
- included/excluded state

### Tool Timeline

Tool timeline shows:

- tool name
- input summary
- capability used
- status
- duration
- output count
- errors
- cancellation
- retry

Tool calls should be visible before, during, and after execution.

### Suggestion Tray

Suggestions are not applied automatically.

Suggestion types:

- add comment
- rename function
- rename local variable
- add tag
- set type hint
- add analysis hint
- create patch plan
- create evidence export
- run recipe
- open route

Each suggestion shows:

- target route
- proposed value
- evidence used
- risk
- required capability
- Apply button
- Dismiss button
- Explain button

### Ask Modes

Modes:

- Explain Selection
- Summarize Function
- Trace Callers
- Explain ObjC Dispatch
- Explain C++ Virtual Call
- Explain Kernel/Panic Frame
- Compare Artifacts
- Triage Problems
- Generate Report Fragment
- Propose Names
- Review Patch Plan
- Explain Debugger Stop

Modes are prompt templates plus tool allowlists, not hard-coded prose.

## Data Model

### AssistantSession

Fields:

- `id`
- `project_id`
- `source_id`
- `source_revision_id`
- `created_at`
- `updated_at`
- `model_profile_id`
- `privacy_policy_id`
- `default_tool_allowlist`
- `messages`
- `tool_calls`
- `evidence_items`
- `suggestions`
- `activity_ids`

### AssistantMessage

Fields:

- `id`
- `session_id`
- `role`
- `content`
- `content_blocks`
- `route_citations`
- `fact_citations`
- `evidence_ids`
- `assumption_ids`
- `suggestion_ids`
- `privacy_classification`
- `model_metadata`
- `created_at`

Roles:

- user
- assistant
- tool
- system_notice
- review

### AssistantEvidenceItem

Fields:

- `id`
- `session_id`
- `kind`
- `route`
- `object_id`
- `summary`
- `payload_ref`
- `confidence`
- `provenance`
- `privacy_classification`
- `included_in_prompt`
- `included_in_response`

Kinds:

- instruction
- byte_range
- function
- graph_node
- pseudocode_line
- objc_object
- cpp_object
- kernel_object
- debug_source
- runtime_observation
- problem
- export_fragment

### AssistantToolCall

Fields:

- `id`
- `session_id`
- `tool_name`
- `arguments`
- `capabilities`
- `status`
- `started_at`
- `ended_at`
- `result_summary`
- `result_ref`
- `errors`
- `activity_id`

### AssistantSuggestion

Fields:

- `id`
- `session_id`
- `kind`
- `target_route`
- `proposed_value`
- `evidence_ids`
- `required_capabilities`
- `risk_level`
- `status`
- `transaction_id`

Statuses:

- proposed
- accepted
- rejected
- superseded
- applied
- failed

### ModelProfile

Fields:

- `id`
- `name`
- `provider`
- `mode`
- `endpoint_kind`
- `allowed_data_classes`
- `max_context_tokens`
- `supports_tools`
- `supports_structured_output`
- `supports_local_only`
- `retention_policy`
- `cost_policy`

Provider modes:

- local_only
- user_configured_remote
- enterprise_gateway
- disabled

## Grounding Contract

Assistant output should use structured blocks:

- claim
- evidence citation
- inference
- uncertainty
- suggested action
- warning
- next query

Example claim block:

```json
{
  "kind": "claim",
  "text": "This function appears to dispatch through an Objective-C selector.",
  "citations": [
    {
      "route": "instruction:0x100003f20",
      "fact_id": "objc-ref-12",
      "confidence": "strong_inference"
    }
  ],
  "uncertainty": "Receiver class is not proven."
}
```

The renderer can show this as readable prose, but the stored output should keep
the structured citation.

## Context Selection

Context sources:

- current selection
- visible function
- caller/callee neighborhood
- current graph region
- current pseudocode source maps
- runtime browser object
- problem list selection
- debug stop state
- patch plan
- diff selection
- search results

Context rules:

- include the smallest useful context
- prefer structured facts to raw bytes
- prefer route ids to copied text
- summarize large lists before expansion
- require explicit consent before raw bytes/source/debug memory enter prompt
- label every included item in the context bar

## Tool Design

Assistant tools should be wrappers over Automation Studio/MCP tools.

Read tools:

- project summary
- route resolve
- function detail
- assembly window
- graph detail
- xrefs
- ObjC object detail
- C++ object detail
- kernel object detail
- pseudocode view
- problem detail
- patch plan detail
- debugger stop summary
- evidence export fragment

Proposal tools:

- propose comment
- propose label
- propose type hint
- propose analysis hint
- propose patch plan
- propose report fragment

Mutation tools:

- disabled by default
- require review
- use transactions
- appear in suggestion tray first

## Prompt Templates

Prompt templates are versioned records.

Fields:

- id
- name
- purpose
- input schema
- tool allowlist
- output schema
- privacy policy
- response contract
- evaluation fixtures

Examples:

- `explain_function_v1`
- `objc_dispatch_triage_v1`
- `kernel_panic_frame_v1`
- `patch_plan_review_v1`
- `debugger_stop_explain_v1`
- `artifact_diff_summary_v1`

Prompt templates should be testable against fixtures.

## Privacy Policy

Privacy modes:

- `no_model`
- `local_model_only`
- `metadata_only_remote`
- `evidence_summary_remote`
- `explicit_bytes_remote`
- `full_private_session`

Default:

- no raw bytes
- no raw memory
- no local absolute paths
- no dSYM private source paths
- no launch arguments/environment
- no panic identifiers
- no exported private artifacts

The user can elevate privacy mode for a single request or session, but the UI
must explain what data classes are included.

## Consent And Capability Model

Capabilities:

- `ai.read_project_summary`
- `ai.read_function`
- `ai.read_bytes_summary`
- `ai.read_raw_bytes`
- `ai.read_pseudocode`
- `ai.read_debug_state`
- `ai.read_patch_plan`
- `ai.propose_annotation`
- `ai.propose_type_hint`
- `ai.propose_patch_plan`
- `ai.apply_mutation`
- `ai.export_evidence`
- `ai.remote_model`

Rules:

- read-only explanation uses low-risk capabilities
- raw bytes require explicit consent
- remote model use requires explicit consent
- mutation requires suggestion review and transaction
- debugger control and patch apply are out of scope for first wave

## Workbench Integration

### Assembly

Assistant can:

- explain selected instruction
- summarize basic block
- trace branch target
- explain unresolved control flow
- propose comments

It should show citations to instruction ids and byte ranges.

### Graph

Assistant can:

- summarize graph region
- explain branch decisions
- identify suspicious unreachable blocks
- propose block comments
- compare graph before/after patch

It should cite graph node ids and source instructions.

### Pseudocode

Assistant can:

- explain pseudocode line
- map line to instructions
- identify weak type/control confidence
- propose variable names
- explain unresolved regions

It must cite pseudocode source maps, not only rendered text.

### Runtime Browsers

Assistant can:

- explain ObjC class/method relations
- summarize selector callers/implementers
- explain C++ vtable candidates
- explain IOKit external method evidence
- explain KDK profile mismatch

It must distinguish static facts, runtime observations, debug info, and
inference.

### Patch Studio

Assistant can:

- review patch plan
- summarize affected facts
- identify code-signing/export warnings
- propose safer alternatives
- prepare evidence report

It cannot apply patches in the first wave.

### Debugger

Assistant can:

- summarize stop event
- explain registers using value interpretation
- map frame to static route
- identify unresolved module/source mapping
- suggest next static queries

It cannot continue, step, attach, or launch in the first wave.

## API Contracts

### Create Assistant Session

```text
POST /api/projects/{project_id}/assistant/sessions
```

Request:

```json
{
  "model_profile_id": "local-summary",
  "privacy_policy": "local_model_only",
  "source_id": "source-main",
  "source_revision_id": "rev-original"
}
```

### Add Context

```text
POST /api/projects/{project_id}/assistant/sessions/{session_id}/context
```

Request:

```json
{
  "items": [
    {
      "kind": "route",
      "route": "function:0x100003f00"
    }
  ],
  "include_policy": "summary_only"
}
```

### Ask Assistant

```text
POST /api/projects/{project_id}/assistant/sessions/{session_id}/messages
```

Request:

```json
{
  "mode": "explain_function",
  "text": "What does this function appear to do?",
  "tool_allowlist": ["get_function", "get_xrefs", "get_objc_refs"],
  "privacy_policy": "metadata_only_remote"
}
```

### Apply Suggestion

```text
POST /api/projects/{project_id}/assistant/suggestions/{suggestion_id}/apply
```

This creates a normal Workbench transaction. It is not a special AI write path.

### Export Assistant Evidence

```text
POST /api/projects/{project_id}/assistant/sessions/{session_id}/export
```

Exports:

- conversation
- prompt/context summary
- tool calls
- evidence citations
- accepted suggestions
- redaction policy

## Native Store Architecture

Add stores:

- `AssistantSessionStore`
- `AssistantContextStore`
- `AssistantToolCallStore`
- `AssistantEvidenceStore`
- `AssistantSuggestionStore`
- `AssistantModelProfileStore`
- `AssistantPrivacyStore`
- `AssistantPromptTemplateStore`
- `AssistantDiagnosticsStore`

Responsibilities:

`AssistantSessionStore`:

- conversations
- active assistant pane
- selected session

`AssistantContextStore`:

- selected routes
- included/excluded evidence
- privacy classification

`AssistantToolCallStore`:

- tool call lifecycle
- progress/cancellation
- errors

`AssistantSuggestionStore`:

- proposed mutations
- review state
- apply/reject

`AssistantPrivacyStore`:

- data-class inclusion
- remote/local model warnings
- redaction preview

## Provider Architecture

Assistant provider boundary:

```text
Workbench
  -> Assistant service
  -> Context builder
  -> Tool router
  -> Model provider adapter
  -> Response validator
  -> Evidence renderer
```

Provider adapters:

- local model adapter
- user-configured remote adapter
- enterprise gateway adapter
- disabled/no-model adapter for fixture tests

The assistant service should validate outputs against response schemas before
rendering as authoritative UI.

## Response Validation

Validate:

- every route citation resolves
- every fact citation exists
- every proposed mutation targets a valid route
- every suggested command has required capability
- privacy classification matches included data
- response schema is valid
- unsupported claims are labeled inference

If validation fails:

- show response as untrusted draft
- keep raw model output hidden behind diagnostic disclosure
- do not create suggestions automatically

## Evaluation And Tests

Fixture classes:

- explain simple function
- explain ObjC dispatch
- explain unresolved selector
- explain C++ virtual call ambiguity
- explain kernel panic frame
- summarize patch plan
- explain debugger stop
- compare two artifacts
- reject hallucinated route
- reject unsupported mutation
- privacy redaction
- tool call denied

Metrics:

- citation coverage
- invalid citation count
- unsupported claim count
- suggestion precision
- privacy violation count
- tool-call count
- latency
- cancellation response

## Implementation Phases

### Phase 1: Assistant Shell Without Model Calls

Deliver:

- assistant pane
- context bar
- evidence tray
- model disabled state
- prompt template registry
- fixture response renderer

Acceptance:

- user can see selected context and evidence without calling a model

### Phase 2: Tool-Grounded Local Summaries

Deliver:

- context builder
- read-only tool calls
- deterministic summary templates
- evidence citation renderer
- assistant session records

Acceptance:

- function explanation can be generated from deterministic tool output

### Phase 3: Local/Configured Model Adapter

Deliver:

- model profile records
- privacy policy gate
- prompt preview
- response schema validation
- tool-call timeline

Acceptance:

- remote use requires explicit consent
- invalid citations are rejected

### Phase 4: Suggestion Review

Deliver:

- propose comments/names/tags/type hints
- suggestion tray
- apply/reject
- transaction integration
- undo/redo

Acceptance:

- AI suggestions are reviewable and undoable

### Phase 5: Apple-System Assistant Modes

Deliver:

- ObjC dispatch triage
- C++ vtable explanation
- kernel/panic frame explanation
- patch plan review
- debugger stop explanation
- artifact diff summary

Acceptance:

- each mode has fixture tests and evidence citations

### Phase 6: Evidence Export

Deliver:

- redacted assistant transcript export
- evidence package export
- accepted suggestion log
- prompt/context summary

Acceptance:

- exports prove what evidence and model/provider were used

## Validation Matrix

Required tests:

- assistant context route snapshot
- privacy mode redaction snapshot
- tool-call timeline snapshot
- evidence citation validation
- hallucinated route rejection
- unsupported claim labeling
- suggestion review/apply/undo
- remote model consent denial
- raw bytes consent denial
- ObjC dispatch explanation fixture
- C++ virtual call explanation fixture
- panic frame explanation fixture
- patch plan review fixture
- debugger stop explanation fixture
- export redaction fixture

## Acceptance Criteria

Damsel should not claim Hopper-competitive AI assistance until:

- assistant output is route/fact/evidence-cited
- every tool call is visible and cancellable
- prompt context is reviewable before sending
- privacy modes prevent accidental raw byte/path/source leaks
- remote model calls require explicit consent
- suggestions are reviewable and transaction-backed
- unsupported claims are labeled as inference
- invalid citations are rejected or downgraded
- Apple-system modes have fixture-backed evaluations
- activity logs connect assistant actions to project changes
- evidence exports can reproduce assistant claims

## Summary

Hopper's AI/MCP surface raises the competitive bar, but it also makes the trust
problem sharper. Damsel's advantage should be evidence grounding: the assistant
does not merely chat about binaries; it cites routes, facts, source maps,
confidence, and provenance inside the project model.

The right implementation order is assistant shell, deterministic tool-grounded
summaries, model adapters behind privacy gates, reviewed suggestions, Apple
system modes, and evidence exports. That gives analysts useful AI help without
letting a language model become an unreviewed analyzer, patcher, debugger, or
source of truth.
