# Investigation Casebook, Session Timeline, And Report Workspace Implementation Spec

Status: implementation-grade architecture spec.

Date of public-source pass: 2026-05-30.

Purpose: define the analyst-facing memory layer that turns Damsel from a set of
excellent binary views into a complete Apple-system investigation workspace.

This is a companion to:

- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)
- [Apple System Diffing, Export, And Evidence Implementation Spec](apple-system-diffing-export-evidence-implementation-spec.md)
- [Evidence-Grounded AI Assistant Workbench Implementation Spec](evidence-grounded-ai-assistant-workbench-implementation-spec.md)
- [Automation Studio, Scripting, MCP, And Plugin Workbench Implementation Spec](automation-studio-scripting-plugin-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec](live-debugger-dynamic-workbench-implementation-spec.md)
- [Remote Device Debugging And Target Environment Implementation Spec](remote-device-debugging-target-environment-implementation-spec.md)
- [Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md)
- [Apple SDK, Header, And Semantic Packs Implementation Spec](apple-sdk-semantic-packs-implementation-spec.md)

## Why This Exists

Hopper's public surface already includes many pieces of an analyst workflow:
comments, labels, tags, colors, manual references, graph annotation, Python
scripting, CFG PDF export, pseudocode/assembly export, MCP, debugger
integration, and save/export behavior gated by licensing. Those are not merely
small UI affordances. They reveal the shape of real reverse-engineering work:
users navigate, test hypotheses, annotate, extract evidence, automate a
question, return later, and eventually explain what they found.

Damsel's docs already specify many of the technical parts:

- persistent project database
- route model
- command registry
- semantic edits
- annotation transactions
- automation runs
- AI evidence citations
- diff snapshots
- report/evidence exports
- debugger stop reports
- Apple runtime browsers

The missing product layer is the connective tissue between them. Damsel needs
a first-class Investigation Casebook: a native Workbench surface where an
analyst can keep the story of the investigation, not just the current cursor.

This is especially important for Damsel's Apple niche. A macOS/iOS/XNU
investigation can involve:

- one or more dyld shared caches
- projected framework images
- standalone app binaries
- dSYM/KDK sidecars
- Swift and Objective-C runtime metadata
- C++/IOKit evidence
- panic logs
- remote debug sessions
- automation recipes
- AI-assisted summaries
- patch previews
- cross-version diffs

Without a casebook, those objects become scattered panes and exported files.
With a casebook, they become a reproducible, navigable research record.

## Product Thesis

Damsel can compete with Hopper by matching the essential interactive
disassembler workflow, then exceeding it for evidence-driven Apple-system
research.

The casebook should answer:

- what question was the analyst trying to answer?
- which sources, sidecars, and build versions were involved?
- which routes mattered?
- what evidence supports each finding?
- which facts were weak, ambiguous, inferred, imported, or user-authored?
- what commands, scripts, AI prompts, debugger actions, and exports happened?
- what changed between sessions?
- what can be safely shared without leaking paths, strings, comments, or bytes?
- how can another analyst reopen the project and reproduce the path?

This is not "notes in a sidebar." It is a project subsystem with data models,
routes, transactions, privacy classifications, APIs, Workbench views,
performance budgets, and validation gates.

## Public Source Set

Primary Hopper sources:

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper download page: <https://www.hopperapp.com/download>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper Debugger Server page: <https://www.hopperapp.com/HopperDebuggerServer/>

Public scripting and workflow sources:

- Hopper public Python scripts:
  <https://github.com/droe/hopper-scripts>
- Hopper procedure-name export workflow:
  <https://calderonpale.com/2025/03/27/exporting-procedure-names-from-hopper-disassembler-when-the-ui-wont-help/>
- Hopper Python type stubs:
  <https://pypi.org/project/types-hopper-disassembler/>
- Hopper macOS script examples:
  <https://github.com/melomac/Hopper>

Apple and platform-adjacent sources already used by companion specs:

- Apple debugger entitlement documentation
- Apple LLDB documentation
- Apple KDK and kernel debugging documentation
- DWARF documentation
- Swift ABI documentation

This spec does not need new binary-format facts. It uses those sources to
define the analyst workflow that should sit above existing Damsel facts.

## Public Evidence Summary

### Hopper Tutorial Evidence

The tutorial shows that Hopper analysis is interactive and stateful:

- automatic analysis starts when a file is opened
- users manually change byte types
- undo/redo is part of the exploration loop
- assembly, graph, pseudocode, and hex views are synchronized
- comments can be attached to addresses
- labels can be edited
- graph nodes can be grouped, colored, and edited
- tags can be attached to addresses, basic blocks, and procedures
- tag scope filters the symbol list
- references show forward and reverse relationships
- users can manually add references
- navigation stack records jumps
- breakpoints and bookmarks are visible in the assembly margin
- switch/case hints let users record analysis knowledge

Casebook implication:

- Damsel should treat every meaningful analyst action as a potential timeline
  event.
- User-created semantic edits should be linkable from notes and findings.
- Navigation history should be more than a back button; it is raw material for
  replayable investigation paths.

### Hopper Homepage Evidence

The homepage emphasizes:

- native Mac UI
- keyboard shortcuts
- interactive CFG with PDF export
- direct annotation from graph view
- assembly, CFG, and pseudocode at the same time
- Objective-C and Swift support
- local debugger support
- Python scripting
- MCP/AI integration
- SDK extensibility
- contextual inspector

Casebook implication:

- Exported reports should not be isolated from Workbench state.
- AI and scripting outputs need a visible place in the project timeline.
- Graph and pseudocode annotations should become evidence snippets when useful.
- A Mac-native product should make the investigation recoverable after reopen,
  crash, or context switch.

### Hopper FAQ And Download Evidence

The FAQ says:

- MCP configuration is available through Hopper Help.
- Hopper can modify binaries through hex editor, assembler, and Python scripts.
- Hopper does not fully recover lost source code.
- iOS binaries protected by FairPlay are out of scope until decrypted by the
  user.

The download page says the demo has no save/export of disassembly or modified
binaries, implying save/export is a product capability unlocked by license.

Casebook implication:

- Damsel should separate facts, findings, and claims.
- Unsupported or user-supplied preprocessing steps must be visible in case
  metadata.
- Export is a core workflow, so the casebook must produce shareable artifacts
  with redaction, not just a local-only notebook.

### Public Script Evidence

Public Hopper scripts annotate shellcode, annotate YARA matches, copy
selections as Python or YARA, save bytes, add file offset comments, and fix
import names. A public 2025 workflow used a Python script to export procedure
names and addresses because the UI did not expose the exact export needed.

Casebook implication:

- Automation outputs should be first-class project artifacts.
- A recipe run should be attachable to a finding.
- Script-created annotations should appear in the same timeline as manual
  annotations.
- Damsel should make common evidence exports native so users do not have to
  write one-off scripts for every report.

## Current Damsel Baseline

Current implemented strengths:

- standalone Mach-O parsing
- universal slice selection
- section, symbol, import, relocation, dyld metadata, and ObjC metadata views
- dyld shared-cache inventory, projection, symbolication, dependencies,
  providers/importers, and reexports
- disassembly analysis with blocks, edges, calls, branches, imports,
  ObjC references, recovered values, jump tables, and cache links
- native Workbench shell over local UI server
- docs for future projects, routes, semantic edits, automation, AI, debugger,
  patching, diff/export, and evidence packages

Current gaps:

- no project casebook model
- no session timeline model
- no durable finding records
- no hypothesis or question objects
- no pinned route/evidence set separate from comments/bookmarks
- no notebook pages tied to route/object ids
- no report builder that assembles findings from canonical facts
- no "what changed since last session" view
- no activity log unified across manual edits, automation, AI, debugger, and
  export
- no case-level redaction policy
- no case handoff package
- no validation fixtures for analyst narrative workflows

Damsel has the pieces, but not the project memory layer that binds them.

## Design Goals

### Goal 1: Preserve Investigation Intent

The app should let an analyst declare and refine:

- the research question
- target artifacts
- OS/build versions
- source trust assumptions
- known constraints
- expected outputs
- current hypotheses
- conclusion status

This should live in project data, not in a loose Markdown file.

### Goal 2: Make Evidence Routeable

Every casebook note, finding, timeline event, and report section should be able
to link to project routes:

- source route
- address route
- byte range route
- function route
- basic block route
- instruction route
- graph node route
- pseudocode line route
- ObjC selector/class/method route
- Swift type/conformance route
- C++ vtable/RTTI route
- IOKit external method route
- kernel member/kext/syscall/MIG route
- panic frame route
- debug stop route
- automation run route
- AI response route
- patch preview route
- diff finding route
- problem route

If a note cannot point at evidence, it should be labeled as narrative or
inference.

### Goal 3: Keep Manual And Automated Work Together

Manual comments, tags, labels, semantic edits, script outputs, AI suggestions,
debugger stops, and exports should share one activity model. Users should not
have to inspect five logs to reconstruct what happened.

### Goal 4: Support Handoff Without Leaking Secrets

Apple-system reverse engineering often involves private local paths, local KDKs,
customer app names, strings, comments, memory bytes, device identifiers, and
panic logs. Casebook export must have a redaction preview and a policy that is
testable.

### Goal 5: Stay Fast

Opening the casebook should be cheap even when the project has millions of
facts. Timeline pages, notes, findings, and reports must be paged and indexed.
Generated previews should use summaries first and expand on demand.

## Non-Goals

Do not implement first:

- cloud collaboration
- remote multi-user editing
- real-time shared cursors
- legal hold or enterprise e-discovery systems
- automatic vulnerability scoring
- model-generated final reports without evidence review
- hidden AI summarization of private binaries
- generic Markdown notebook disconnected from project facts
- raw binary byte export by default
- external issue tracker integration
- cryptographic attestation beyond local hashes and manifests

The first version should be local, project-scoped, evidence-driven, and
redaction-aware.

## Core Concepts

### Case

A case is the durable research wrapper for one investigation.

Examples:

- "Foundation selector behavior across macOS 26 builds"
- "IOKit user-client external method audit"
- "Crash panic route triage for kernel extension"
- "Swift/ObjC bridge map for app framework"
- "Patch impact review for standalone Mach-O"

Case fields:

```rust
pub struct InvestigationCase {
    pub id: CaseId,
    pub project_id: ProjectId,
    pub title: String,
    pub status: CaseStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub primary_question: String,
    pub scope: CaseScope,
    pub source_set: Vec<CaseSourceRef>,
    pub privacy_policy_id: PrivacyPolicyId,
    pub default_report_profile_id: ReportProfileId,
    pub tags: Vec<String>,
    pub owner_note: Option<String>,
}
```

`CaseStatus`:

- `draft`
- `active`
- `waiting_on_source`
- `waiting_on_analysis`
- `ready_for_review`
- `archived`

### Session

A session represents a period of work in the Workbench.

```rust
pub struct InvestigationSession {
    pub id: SessionId,
    pub case_id: CaseId,
    pub project_id: ProjectId,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
    pub app_version: String,
    pub helper_version: Option<String>,
    pub project_schema_version: u32,
    pub host_fingerprint: RedactedHostFingerprint,
    pub summary: Option<String>,
    pub event_count: u64,
}
```

Sessions should be auto-created when a case opens and closed on app quit or
project close. Crash recovery should mark a session as interrupted.

### Timeline Event

Timeline events are normalized records of what happened.

```rust
pub struct CaseTimelineEvent {
    pub id: TimelineEventId,
    pub case_id: CaseId,
    pub session_id: Option<SessionId>,
    pub project_id: ProjectId,
    pub timestamp: Timestamp,
    pub actor: CaseActor,
    pub kind: TimelineEventKind,
    pub title: String,
    pub summary: Option<String>,
    pub routes: Vec<RouteId>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub privacy_class: PrivacyClass,
    pub transaction_id: Option<TransactionId>,
    pub job_id: Option<JobId>,
    pub correlation_id: Option<String>,
}
```

`CaseActor`:

- `user`
- `analysis_job`
- `automation_recipe`
- `script`
- `plugin`
- `assistant`
- `debugger`
- `importer`
- `exporter`
- `system`

`TimelineEventKind`:

- `case_created`
- `source_added`
- `sidecar_attached`
- `analysis_started`
- `analysis_completed`
- `route_opened`
- `search_performed`
- `bookmark_created`
- `note_created`
- `semantic_edit_applied`
- `hypothesis_created`
- `hypothesis_status_changed`
- `finding_created`
- `finding_evidence_added`
- `automation_run_started`
- `automation_run_completed`
- `assistant_response_recorded`
- `debugger_session_started`
- `debugger_stop_recorded`
- `patch_preview_created`
- `diff_snapshot_created`
- `report_exported`
- `redaction_policy_changed`
- `problem_recorded`
- `project_reopened`
- `session_interrupted`

The event model should be append-only except for explicit redaction or
compaction maintenance. User-visible edits to event summaries should create
new events.

### Evidence Pin

An evidence pin is a curated link to a project fact or route.

```rust
pub struct EvidencePin {
    pub id: EvidencePinId,
    pub case_id: CaseId,
    pub title: String,
    pub route: RouteId,
    pub object_id: Option<ProjectObjectId>,
    pub source_revision_id: SourceRevisionId,
    pub excerpt: EvidenceExcerpt,
    pub evidence_kind: EvidenceKind,
    pub confidence: Confidence,
    pub provenance: Vec<ProvenanceId>,
    pub privacy_class: PrivacyClass,
    pub created_from: EvidencePinOrigin,
    pub created_at: Timestamp,
}
```

Evidence pins differ from bookmarks:

- a bookmark helps the user navigate back
- an evidence pin is intended to support a hypothesis, finding, or report

### Hypothesis

Hypotheses are working claims with evidence status.

```rust
pub struct Hypothesis {
    pub id: HypothesisId,
    pub case_id: CaseId,
    pub title: String,
    pub status: HypothesisStatus,
    pub statement: String,
    pub supporting_evidence: Vec<EvidencePinId>,
    pub contradicting_evidence: Vec<EvidencePinId>,
    pub open_questions: Vec<OpenQuestionId>,
    pub confidence: Confidence,
    pub last_reviewed_at: Option<Timestamp>,
}
```

`HypothesisStatus`:

- `open`
- `supported`
- `contradicted`
- `inconclusive`
- `superseded`
- `accepted_as_finding`

### Finding

A finding is a reviewed claim that may appear in a report.

```rust
pub struct Finding {
    pub id: FindingId,
    pub case_id: CaseId,
    pub title: String,
    pub severity: FindingSeverity,
    pub status: FindingStatus,
    pub summary: String,
    pub technical_detail: String,
    pub evidence_pins: Vec<EvidencePinId>,
    pub affected_objects: Vec<ProjectObjectId>,
    pub affected_sources: Vec<SourceId>,
    pub confidence: Confidence,
    pub limitations: Vec<String>,
    pub redaction_notes: Vec<String>,
    pub report_section_ids: Vec<ReportSectionId>,
}
```

`FindingSeverity` should be user-defined or project-profile-defined. Damsel
should not ship with exploitability claims by default.

### Notebook Page

Notebook pages are structured notes that can embed routes and pins.

```rust
pub struct NotebookPage {
    pub id: NotebookPageId,
    pub case_id: CaseId,
    pub title: String,
    pub body: NotebookBody,
    pub linked_routes: Vec<RouteId>,
    pub linked_pins: Vec<EvidencePinId>,
    pub linked_findings: Vec<FindingId>,
    pub privacy_class: PrivacyClass,
    pub updated_at: Timestamp,
}
```

`NotebookBody` should use a constrained rich-text or Markdown subset:

- headings
- paragraphs
- bullet lists
- code blocks
- tables
- callout blocks
- route links
- evidence pin embeds
- image/screenshot references
- redaction markers

Do not allow arbitrary HTML or remote image embeds in the first version.

### Report

A report is a generated or assembled artifact from reviewed case data.

```rust
pub struct CaseReport {
    pub id: ReportId,
    pub case_id: CaseId,
    pub title: String,
    pub profile_id: ReportProfileId,
    pub status: ReportStatus,
    pub sections: Vec<ReportSectionId>,
    pub export_policy_id: ExportPolicyId,
    pub generated_at: Option<Timestamp>,
    pub export_jobs: Vec<JobId>,
}
```

Reports should be generated from canonical facts, findings, and notes. The
rendered report should not become the source of truth.

## Architecture Overview

```text
Project database
  |
  +-- routes, facts, annotations, jobs, problems
  +-- automation runs
  +-- assistant responses
  +-- debugger sessions and stop reports
  +-- diff snapshots and findings
  +-- patch previews and source revisions
  |
  v
Casebook subsystem
  |
  +-- cases
  +-- sessions
  +-- timeline events
  +-- evidence pins
  +-- hypotheses
  +-- findings
  +-- notebook pages
  +-- reports
  +-- redaction policy
  |
  v
Workbench
  |
  +-- Casebook sidebar
  +-- Timeline
  +-- Evidence tray
  +-- Hypothesis board
  +-- Findings list
  +-- Notebook editor
  +-- Report builder
  +-- Handoff/export review
```

The casebook sits above project facts. It references facts; it does not copy
large fact payloads unless an excerpt is explicitly pinned.

## Ownership Boundaries

### Rust Project Core Owns

- case ids
- timeline event persistence
- evidence pin identity
- route resolution and stale-route repair
- finding/hypothesis records
- report generation inputs
- export manifests
- redaction policy enforcement
- timeline indexes
- privacy classifications

### Swift Workbench Owns

- current case selection
- visible timeline pages
- notebook editing UI state
- report builder layout
- drag/drop of pins into notes
- inspector-to-casebook commands
- redaction preview presentation
- keyboard focus and split-view state

### Automation Owns

- recipe output records
- run provenance
- capability checks
- optional generated notebook snippets or finding drafts

### Assistant Owns

- assistant response records
- evidence citations
- suggestion drafts
- generated summaries that require review before becoming findings

The assistant should not directly write accepted findings without review.

## Casebook Commands

Command ids:

- `casebook.new_case`
- `casebook.open_case`
- `casebook.close_case`
- `casebook.rename_case`
- `casebook.set_primary_question`
- `casebook.pin_current_route`
- `casebook.pin_selection`
- `casebook.create_note`
- `casebook.create_note_from_selection`
- `casebook.create_hypothesis`
- `casebook.add_evidence_to_hypothesis`
- `casebook.accept_hypothesis_as_finding`
- `casebook.create_finding`
- `casebook.add_evidence_to_finding`
- `casebook.open_timeline`
- `casebook.filter_timeline`
- `casebook.export_report`
- `casebook.preview_redaction`
- `casebook.create_handoff_package`
- `casebook.copy_evidence_link`

Every command should declare:

- route requirements
- project requirements
- privacy effects
- undo behavior
- activity log behavior
- keyboard shortcut eligibility

## Workbench UX

### Casebook Sidebar

The sidebar should show:

- active case title
- primary question
- source set summary
- analysis status
- privacy mode
- pinned evidence count
- open hypotheses
- reviewed findings
- latest timeline events

Primary actions:

- New Note
- Pin Current Route
- New Hypothesis
- New Finding
- Build Report
- Export Handoff

### Timeline View

Timeline filters:

- all events
- navigation
- annotations
- analysis jobs
- automation
- assistant
- debugger
- imports/sidecars
- problems
- exports
- findings

Timeline rows should include:

- timestamp
- actor
- event kind
- title
- route badges
- evidence count
- privacy badge
- problem/severity badge

Interactions:

- click route badge to navigate
- expand event details
- add event to note
- pin event evidence
- hide low-value navigation events
- mark event as important
- export filtered timeline

Navigation events should be sampled or compacted. The app should not produce a
timeline row for every cursor movement by default.

### Evidence Tray

The evidence tray is a dockable panel:

- pinned route cards
- selected route quick pin
- evidence excerpts
- source identity
- confidence
- privacy class
- related notes/findings

Evidence card types:

- instruction
- byte range
- function
- pseudocode line
- graph node
- xref group
- ObjC object
- Swift object
- C++ object
- IOKit object
- kernel object
- panic frame
- debug stop
- automation output
- assistant response
- diff finding
- patch preview
- problem

### Hypothesis Board

Columns:

- Open
- Supported
- Contradicted
- Inconclusive
- Accepted

Cards:

- hypothesis title
- confidence
- evidence count
- contradiction count
- open questions
- last reviewed

The board should not imply scientific proof. It is a thinking workspace.

### Findings List

Findings should be reviewable:

- draft findings
- accepted findings
- rejected findings
- findings needing more evidence
- findings blocked by missing source or sidecar

Each finding should show:

- status
- confidence
- affected source count
- evidence count
- limitations
- report inclusion state
- redaction warnings

### Notebook Editor

Notebook editor capabilities:

- route links by drag/drop
- evidence pin embeds
- finding embeds
- code blocks for scripts or pseudocode excerpts
- tables for object lists
- callout blocks for limitations
- redaction markers
- preview as report section

No hidden conversion should copy raw bytes or private strings into a note.
Embedding an evidence pin should preserve a reference plus a small excerpt.

### Report Builder

Report builder tabs:

- Outline
- Findings
- Evidence
- Redaction
- Preview
- Exports

Report section types:

- executive summary
- source manifest
- methods
- timeline summary
- finding
- evidence appendix
- diff summary
- panic analysis
- debugger session summary
- automation run summary
- patch review summary
- limitations

Exports:

- Markdown
- HTML
- JSON evidence package
- local PDF later, if renderer exists

PDF can wait. Markdown/HTML/JSON should come first because they are testable
and diffable.

## API Contracts

### Cases

```text
GET /api/projects/{project_id}/cases
POST /api/projects/{project_id}/cases
GET /api/projects/{project_id}/cases/{case_id}
PATCH /api/projects/{project_id}/cases/{case_id}
POST /api/projects/{project_id}/cases/{case_id}/archive
```

### Sessions

```text
GET /api/projects/{project_id}/cases/{case_id}/sessions
POST /api/projects/{project_id}/cases/{case_id}/sessions
PATCH /api/projects/{project_id}/cases/{case_id}/sessions/{session_id}
POST /api/projects/{project_id}/cases/{case_id}/sessions/{session_id}/close
```

### Timeline

```text
GET /api/projects/{project_id}/cases/{case_id}/timeline
GET /api/projects/{project_id}/cases/{case_id}/timeline/{event_id}
POST /api/projects/{project_id}/cases/{case_id}/timeline/events
POST /api/projects/{project_id}/cases/{case_id}/timeline/compact-preview
```

Timeline query params:

- `kind`
- `actor`
- `route_kind`
- `since`
- `until`
- `privacy_class`
- `important_only`
- `cursor`
- `limit`

### Evidence Pins

```text
GET /api/projects/{project_id}/cases/{case_id}/evidence-pins
POST /api/projects/{project_id}/cases/{case_id}/evidence-pins
GET /api/projects/{project_id}/cases/{case_id}/evidence-pins/{pin_id}
PATCH /api/projects/{project_id}/cases/{case_id}/evidence-pins/{pin_id}
DELETE /api/projects/{project_id}/cases/{case_id}/evidence-pins/{pin_id}
POST /api/projects/{project_id}/cases/{case_id}/evidence-pins/{pin_id}/repair-route
```

### Hypotheses

```text
GET /api/projects/{project_id}/cases/{case_id}/hypotheses
POST /api/projects/{project_id}/cases/{case_id}/hypotheses
GET /api/projects/{project_id}/cases/{case_id}/hypotheses/{hypothesis_id}
PATCH /api/projects/{project_id}/cases/{case_id}/hypotheses/{hypothesis_id}
POST /api/projects/{project_id}/cases/{case_id}/hypotheses/{hypothesis_id}/evidence
POST /api/projects/{project_id}/cases/{case_id}/hypotheses/{hypothesis_id}/accept-as-finding
```

### Findings

```text
GET /api/projects/{project_id}/cases/{case_id}/findings
POST /api/projects/{project_id}/cases/{case_id}/findings
GET /api/projects/{project_id}/cases/{case_id}/findings/{finding_id}
PATCH /api/projects/{project_id}/cases/{case_id}/findings/{finding_id}
POST /api/projects/{project_id}/cases/{case_id}/findings/{finding_id}/evidence
POST /api/projects/{project_id}/cases/{case_id}/findings/{finding_id}/review
```

### Notebook

```text
GET /api/projects/{project_id}/cases/{case_id}/notebook/pages
POST /api/projects/{project_id}/cases/{case_id}/notebook/pages
GET /api/projects/{project_id}/cases/{case_id}/notebook/pages/{page_id}
PATCH /api/projects/{project_id}/cases/{case_id}/notebook/pages/{page_id}
POST /api/projects/{project_id}/cases/{case_id}/notebook/pages/{page_id}/preview
```

### Reports

```text
GET /api/projects/{project_id}/cases/{case_id}/reports
POST /api/projects/{project_id}/cases/{case_id}/reports
GET /api/projects/{project_id}/cases/{case_id}/reports/{report_id}
PATCH /api/projects/{project_id}/cases/{case_id}/reports/{report_id}
POST /api/projects/{project_id}/cases/{case_id}/reports/{report_id}/preview
POST /api/projects/{project_id}/cases/{case_id}/reports/{report_id}/export
```

### Redaction

```text
POST /api/projects/{project_id}/cases/{case_id}/redaction/preview
GET /api/projects/{project_id}/cases/{case_id}/redaction/policies
POST /api/projects/{project_id}/cases/{case_id}/handoff-package/preview
POST /api/projects/{project_id}/cases/{case_id}/handoff-package/export
```

## CLI Contracts

### Case Commands

```text
damsel case list <project>
damsel case create <project> --title "IOKit audit" --question "..."
damsel case show <project> <case-id> --json
damsel case archive <project> <case-id>
```

### Timeline Commands

```text
damsel case timeline <project> <case-id> --actor automation --json
damsel case timeline <project> <case-id> --since 2026-05-30 --markdown
```

### Evidence Commands

```text
damsel case pin <project> <case-id> --route function:...
damsel case evidence <project> <case-id> --json
damsel case evidence repair-routes <project> <case-id> --dry-run
```

### Finding Commands

```text
damsel case finding create <project> <case-id> --title "Selector added"
damsel case finding add-evidence <project> <case-id> <finding-id> --pin <pin-id>
damsel case finding review <project> <case-id> <finding-id> --status accepted
```

### Report Commands

```text
damsel case report preview <project> <case-id> <report-id> --format markdown
damsel case report export <project> <case-id> <report-id> --format html --out report.html
damsel case handoff preview <project> <case-id> --profile redacted
damsel case handoff export <project> <case-id> --profile redacted --out casebook.zip
```

CLI JSON must use the same ids as Workbench APIs.

## Database Tables

Suggested tables:

- `cases`
- `case_sources`
- `case_sessions`
- `case_timeline_events`
- `case_timeline_event_routes`
- `case_evidence_pins`
- `case_hypotheses`
- `case_hypothesis_evidence`
- `case_open_questions`
- `case_findings`
- `case_finding_evidence`
- `case_notebook_pages`
- `case_notebook_embeds`
- `case_reports`
- `case_report_sections`
- `case_report_exports`
- `case_redaction_policies`
- `case_handoff_packages`

Indexes:

- timeline by case and timestamp
- timeline by kind and actor
- events by route id
- evidence pins by route id
- findings by status and severity
- notebook full-text search
- report sections by report order
- privacy class by case

Storage rules:

- do not duplicate large fact payloads
- store excerpts with size limits
- keep route id plus route repair metadata
- store source revision id on every evidence pin
- preserve transaction ids for semantic edits
- preserve job ids for automation/import/export events

## Timeline Capture Policy

Capture by default:

- case created/opened/closed
- source added or removed
- sidecar attached or detached
- analysis job start/complete/fail
- semantic edit applied
- evidence pin created
- hypothesis/finding status changes
- automation run start/complete/fail
- assistant response accepted or rejected
- debugger session start/stop/error
- patch preview/export
- diff snapshot/export
- report/handoff export
- redaction policy changes

Sample or compact:

- route opened
- search performed
- selection changed
- scroll/hover events
- repeated failed route repairs

Do not capture:

- raw keyboard input not submitted
- raw note drafts before save
- full prompt text by default if privacy policy excludes it
- raw memory bytes from debugger unless explicitly pinned
- raw binary bytes unless explicitly exported

## Privacy Model

Privacy classes:

- `public_safe`
- `project_internal`
- `private_paths`
- `private_strings`
- `private_comments`
- `private_bytes`
- `debug_memory`
- `device_identity`
- `credential_risk`

Every casebook object gets a computed privacy class from:

- linked routes
- excerpts
- source identities
- note text
- findings
- assistant prompt/response metadata
- automation output
- debugger output
- export destinations

Redaction policies:

- minimal handoff
- technical no-bytes
- technical with selected strings
- full local archive
- custom

Export preview must show:

- included sources
- included routes
- included raw bytes count
- included strings count
- included user comments count
- local path count
- device identifier count
- debug memory count
- assistant text count
- automation output count

## Report Builder Details

### Report Profiles

Profiles:

- `triage_summary`
- `technical_deep_dive`
- `apple_runtime_audit`
- `kernel_panic_analysis`
- `diff_report`
- `patch_review`
- `debug_session_summary`
- `handoff_packet`

Each profile defines:

- default sections
- required finding status
- allowed privacy classes
- evidence appendix style
- route link style
- code/byte inclusion rules
- export formats

### Section Generation

Generated sections should cite ids:

- source ids
- route ids
- finding ids
- evidence pin ids
- job ids
- assistant response ids
- automation run ids

Every generated paragraph should be traceable to facts or findings. If the
paragraph is narrative, the report preview should label it as narrative.

### Evidence Appendix

Evidence appendix entries:

- title
- route
- source identity
- excerpt
- confidence
- provenance
- privacy class
- reproduction command, if available

Appendix entries should be stable across exports unless source revisions change.

## Integration With Existing Specs

### Semantic Editing

Comments, labels, tags, colors, manual xrefs, byte overrides, function hints,
type edits, and runtime bindings should generate timeline events when they are
case-relevant. The semantic edit transaction remains the source of truth.

### Automation Studio

Automation runs can:

- create timeline events
- emit evidence pins
- draft notebook snippets
- draft findings
- export outputs

Run outputs are not accepted findings until reviewed.

### AI Assistant

Assistant responses can:

- cite evidence pins
- draft a hypothesis
- draft a finding
- summarize a timeline interval
- propose report text
- flag missing evidence

Assistant output should remain a suggestion until reviewed.

### Diff And Evidence Export

Diff findings can become case findings. Evidence packages can be generated from
a case report or a filtered evidence pin set.

### Debugger

Debugger stops can become timeline events and evidence pins:

- breakpoint hit
- register snapshot
- memory watch
- module slide mapping
- thread frame
- stop reason

Debugger memory bytes require explicit privacy review before export.

### Patch Studio

Patch plans and previews can become evidence pins. A patch report can be a
case report profile. Patch application remains capability-gated and separate.

### Preferences

Casebook preferences:

- auto-create case for new project
- record navigation events
- compact navigation events
- default privacy policy
- default report profile
- timeline page size
- evidence excerpt size
- report export folder

## Workbench Store Plan

Swift stores:

```swift
@MainActor
final class CasebookStore: ObservableObject {
  @Published var activeCase: CaseSummary?
  @Published var sidebarSummary: CaseSidebarSummary?
  @Published var privacyWarnings: [CasePrivacyWarning] = []
}
```

```swift
@MainActor
final class CaseTimelineStore: ObservableObject {
  @Published var filters: TimelineFilters = .default
  @Published var page: PagedResult<TimelineRow>?
  @Published var selectedEvent: TimelineEventDetail?
}
```

```swift
@MainActor
final class EvidenceTrayStore: ObservableObject {
  @Published var pins: PagedResult<EvidencePinRow>?
  @Published var currentRoutePreview: EvidencePreview?
}
```

Other stores:

- `HypothesisBoardStore`
- `FindingStore`
- `NotebookStore`
- `ReportBuilderStore`
- `RedactionPreviewStore`

Rules:

- no store loads all timeline events
- no store loads all evidence excerpts
- note editor autosaves structured deltas
- report preview is job-backed for large reports
- route navigation uses shared `SelectionStore`

## Performance Budgets

Casebook sidebar:

- open under 150 ms from cached summary
- refresh under 250 ms after timeline event append

Timeline:

- first page under 250 ms for normal projects
- filter change under 300 ms
- event append visible under 500 ms
- compaction preview under 1 s for 10,000 navigation events

Evidence tray:

- pin current route under 150 ms
- first page under 250 ms
- route repair preview under 500 ms for 1,000 pins

Notebook:

- open small page under 150 ms
- autosave acknowledgement under 300 ms
- route embed preview under 250 ms

Report builder:

- outline open under 250 ms
- Markdown preview under 1 s for normal report
- large report export streamed with progress events every 250 ms

## Validation Matrix

### Unit Tests

- case schema serialization
- timeline event append order
- timeline event privacy class computation
- route link validation
- evidence pin excerpt size cap
- stale route repair
- hypothesis status transitions
- finding review transitions
- notebook embed parser
- report section ordering
- redaction policy inclusion/exclusion

### Integration Tests

- create case, pin route, create finding, export report
- semantic edit produces timeline event
- automation run produces timeline event and output link
- assistant response draft does not become finding until reviewed
- debugger stop report can be pinned with privacy warning
- diff finding can be imported as case finding
- patch preview can be referenced without applying patch
- source hash change marks pins stale

### Workbench UI Tests

- casebook sidebar opens with active case
- pin current route from assembly
- drag evidence pin into note
- create hypothesis from selected route
- accept hypothesis as finding
- timeline filters by actor
- report preview shows redaction warnings
- handoff export preview blocks private bytes by default

### Snapshot Tests

- case summary JSON
- timeline event JSON
- evidence pin JSON
- hypothesis JSON
- finding JSON
- notebook page JSON
- report preview Markdown
- redaction preview JSON
- handoff manifest JSON

### Performance Tests

- 100,000 timeline events paged
- 10,000 evidence pins paged
- 1,000 findings filtered
- large notebook page open
- large report preview streamed
- route repair batch

## Migration Slices

### S1: Case And Timeline Foundation

Deliverables:

- case table
- session table
- timeline event table
- append API
- Workbench case sidebar summary

Validation:

- create/open/close case test
- timeline append snapshot
- session interrupted fixture

### S2: Evidence Pins

Deliverables:

- evidence pin model
- pin current route command
- evidence tray
- route repair preview

Validation:

- pin function/instruction/ObjC route
- stale route repair test
- privacy class test

### S3: Notes And Notebook Pages

Deliverables:

- notebook page model
- structured Markdown subset
- route/evidence embeds
- autosave

Validation:

- note save/reopen
- embed parse snapshot
- redaction marker test

### S4: Hypotheses And Findings

Deliverables:

- hypothesis board
- finding records
- evidence-to-finding links
- review transitions

Validation:

- hypothesis status test
- accept hypothesis as finding
- finding evidence completeness warning

### S5: Activity Unification

Deliverables:

- semantic edit timeline integration
- automation run timeline integration
- assistant response timeline integration
- debugger stop timeline integration
- export timeline integration

Validation:

- one fixture for each actor
- timeline filter snapshots
- actor privacy tests

### S6: Report Builder

Deliverables:

- report model
- report sections
- Markdown preview
- HTML export
- JSON evidence package export

Validation:

- report preview snapshot
- export manifest snapshot
- evidence appendix snapshot

### S7: Redaction And Handoff

Deliverables:

- redaction preview
- handoff package manifest
- privacy profile presets
- export blocker rules

Validation:

- no raw bytes by default
- private path redaction
- debug memory blocker
- full-local archive profile

### S8: Apple-System Case Profiles

Deliverables:

- ObjC selector audit case template
- dyld cache diff case template
- IOKit external method case template
- kernel panic case template
- Swift bridge case template
- patch review case template

Validation:

- template creates expected sections
- required sources are listed
- missing sidecar warnings appear

## Risk Register

### Risk: Casebook Becomes A Loose Notes App

Mitigation:

- every evidence block uses route/object ids
- notes distinguish narrative from evidence
- report builder cites pins and findings
- route repair is a first-class feature

### Risk: Timeline Becomes Too Noisy

Mitigation:

- compact navigation events
- filters by actor/kind
- important-event markers
- sampling policy
- per-case preferences

### Risk: Reports Leak Private Data

Mitigation:

- privacy class on every casebook object
- redaction preview
- default no-raw-bytes profile
- export blockers for debug memory and local paths
- snapshot tests for redaction

### Risk: Assistant Text Becomes Unreviewed Truth

Mitigation:

- assistant output is suggestion until reviewed
- findings require explicit review status
- report generator labels narrative and inference
- evidence citations are validated

### Risk: Timeline And Notes Slow Down Large Projects

Mitigation:

- timeline paging
- evidence excerpt caps
- indexes by case/time/kind/actor
- report generation as job
- Workbench stores load visible pages only

### Risk: Handoff Package Cannot Reopen Elsewhere

Mitigation:

- include manifest with schema versions
- include source identity and missing-source explanations
- do not require raw binaries unless profile includes them
- route repair on import
- problem records for missing private artifacts

## Acceptance Criteria

Documentation acceptance:

- this spec is linked from the Hopper-gap README
- feature inventory names the investigation casebook gap
- implementation epics include a casebook/timeline/report workspace epic
- API/data contracts mention casebook routes
- evidence matrix lists casebook validation gates
- research sources cite Hopper comments/tags/scripts/export evidence
- diff/export, AI, automation, debugger, and semantic-edit specs link back

Implementation acceptance for first casebook milestone:

- user can create a case
- Workbench starts a session
- timeline events persist
- user can pin current route
- evidence pins reopen to routes
- user can create a note with a route embed
- redaction preview exists for pinned evidence
- no unrelated project facts are copied into notes

Implementation acceptance for Hopper-competitive casebook lane:

- manual edits, automation runs, assistant responses, debugger stops, diffs,
  patch previews, and exports all appear in one timeline
- hypotheses and findings link to evidence pins
- report builder exports Markdown/HTML/JSON from canonical facts
- handoff package preview explains included and omitted data
- stale route repair handles source revision changes
- privacy/redaction rules are enforced by tests
- large timelines and evidence sets remain responsive

## First Implementation Recommendation

Start with `S1` and `S2`:

1. Add `cases`, `case_sessions`, `case_timeline_events`, and
   `case_evidence_pins`.
2. Add `casebook.new_case`, `casebook.pin_current_route`, and
   `casebook.open_timeline` commands.
3. Record timeline events for source open, analysis job completion, semantic
   edit application, and evidence pin creation.
4. Add a small Workbench casebook sidebar and evidence tray.
5. Add CLI JSON snapshots for creating a case and pinning a route.

This is the smallest meaningful slice because it creates the durable project
memory layer without waiting for report generation, AI, debugger, or diffing.
Once evidence pins exist, every later subsystem can attach its outputs to a
shared investigation record instead of inventing its own local log.
