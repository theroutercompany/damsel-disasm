# UX Latency And Polish Measurement Plan

Status: App-quality measurement plan. This document makes native polish and
responsiveness measurable.

Purpose: Damsel's goal is not only to implement Hopper-like features. It must
feel competitive in everyday use. This means measuring route latency, selection
stability, keyboard navigation, progress/cancellation, errors, and large-list
behavior in the native Workbench.

Implementation companion:
[Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
defines the concrete Workbench surfaces, stores, commands, jobs, problems, and
representations these measurements should cover.

Command/navigation companion:
[Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
defines the command dispatch, menu validation, keyboard, command palette,
navigation, selection, transaction, and activity-log behaviors that need
specific latency and stability budgets.

## Thesis

Polish is observable. A reverse-engineering app feels professional when:

- commands respond immediately
- navigation preserves context
- long work is cancellable
- errors are recoverable
- keyboard workflows are complete
- huge lists remain stable
- panes stay synchronized
- the user never has to infer state from a terminal

Those properties need tests, probes, and budgets.

## Measurement Categories

### Perceptual Latency

Measure:

- menu command dispatch
- toolbar action to visible state
- row selection to inspector update
- route jump to visible target
- search keystroke to first result
- navigator filter to first page
- graph node selection to inspector update
- hex selection to assembly sync

Targets:

- direct UI feedback under 50 ms
- warm local route change under 100 ms
- warm search first page under 150 ms
- cold project summary under 300 ms for common fixtures
- long-running jobs show progress within 500 ms

### Interaction Stability

Measure:

- selected row remains selected after refresh
- selection does not jump when filters change
- back/forward route stack returns to exact object where possible
- graph/assembly/hex/inspector agree on selected route
- collapsed inspector modules stay collapsed
- table column widths persist
- app restores project window state

Failures:

- losing selection after background analysis
- re-sorting lists unexpectedly
- resetting user filters after analysis updates
- graph opening on a different function than selected

### Keyboard Completeness

Measure whether the user can complete workflows without mouse:

- open project
- search symbol/string/selector
- jump to address
- move assembly selection
- follow xref
- go back/forward
- add comment/label/bookmark
- switch assembly/graph/hex
- open problems panel
- apply problem hint
- cancel analysis

Evidence:

- keyboard shortcut list
- UI tests for command dispatch
- manual smoke checklist for complex workflows
- accessibility labels for command targets

### Progress And Cancellation

Measure:

- time from job start to visible progress
- time from cancel click to UI acknowledgement
- time from cancel request to worker checkpoint
- partial-result visibility
- retry behavior
- failed-job problem creation

Rules:

- cancel acknowledgement must be immediate
- worker completion can be delayed but must show state
- canceled jobs must not look successful
- partial facts must be marked partial/stale where relevant

### Error Recovery

Measure common failures:

- server unavailable
- source missing
- malformed Mach-O
- unsupported architecture
- missing split cache
- KDK mismatch
- stale schema
- route no longer exists
- query result version stale

Each error state must show:

- concise message
- technical detail location
- recovery action where possible
- no silent terminal-only failure

### Large-List Behavior

Surfaces:

- symbols
- functions
- strings
- xrefs
- ObjC selectors
- C++ types/vtables
- cache images
- kernel members/kexts
- problems

Measure:

- first page latency
- incremental page latency
- filter latency
- scroll smoothness
- memory growth
- selection stability

Rules:

- no full large-list materialization in Swift
- no main-actor filtering over huge arrays
- APIs return result versions/cursors
- stale cursors produce typed errors

## Workbench Probe Scenarios

### Probe W0: App Launch

Steps:

1. Launch Workbench.
2. Start local server.
3. Observe ready state.

Metrics:

- launch to first window
- server start to ready URL
- error state if CLI missing

Acceptance:

- ready state visible
- CLI missing error is actionable
- no terminal required

### Probe W1: Fixture Open

Steps:

1. Open `fixtures/bin/semantic-switch`.
2. Show project or compatibility image summary.
3. Show first disassembly.

Metrics:

- open to summary
- open to first instruction
- memory delta

Acceptance:

- default target selected
- first useful UI visible quickly
- status text is accurate

### Probe W2: Selection And Inspector

Steps:

1. Select instruction.
2. Move selection with keyboard.
3. Filter instructions.
4. Clear filter.

Metrics:

- selection-to-inspector latency
- filter update latency
- selection preservation

Acceptance:

- inspector tracks selected row
- selection does not jump unnecessarily
- empty filter state is clear

### Probe W3: Search And Navigate

Steps:

1. Search symbol.
2. Open result.
3. Jump to xref target.
4. Back/forward.

Metrics:

- search first result
- route resolution latency
- back/forward latency

Acceptance:

- route stack preserves source/function/object
- result target is visible

### Probe W4: Graph

Steps:

1. Open branch-heavy fixture.
2. Switch to graph.
3. Select block.
4. Return to assembly.

Metrics:

- graph data fetch
- layout time
- selection sync

Acceptance:

- graph has accessible list fallback
- assembly and graph select same function/block

### Probe W5: Problems And Hints

Steps:

1. Open fixture with unresolved indirect branch or malformed metadata.
2. Open problems panel.
3. Select problem.
4. Apply a safe hint or suppress.

Metrics:

- problem list first page
- route jump latency
- hint transaction latency

Acceptance:

- problem has route
- action is undoable
- reanalysis status is visible

### Probe W6: Shared Cache Browser

Steps:

1. Open synthetic shared cache.
2. List images.
3. Search image/symbol.
4. Follow provider/importer relation.

Metrics:

- image list first page
- provider query latency
- route jump latency

Acceptance:

- cache/image address context visible
- relation direction is clear

### Probe W7: Cancellation

Steps:

1. Start deep analysis.
2. Click cancel.
3. Observe status.
4. Retry analysis.

Metrics:

- cancel acknowledgement latency
- worker stop latency
- retry correctness

Acceptance:

- canceled state distinct from success/failure
- partial facts marked as partial or stale

## Instrumentation Requirements

### Rust/UI Server Metrics

Record:

- request id
- endpoint
- project id
- route id
- query duration
- analysis pass duration
- bytes read
- facts inserted
- result count
- cursor/page stats
- cancellation checkpoints
- error code

Expose:

- `/api/projects/{id}/metrics`
- diagnostics export
- benchmark JSON output

### Swift Workbench Metrics

Record:

- command start/end
- API request start/end
- decode/render model duration
- main actor update duration
- selected route changes
- visible row count
- error states

Local-only by default:

- no telemetry upload
- diagnostics export redacted
- raw paths and strings opt-in

### Signposts

Use platform signposts where practical for:

- app launch
- project open
- route navigate
- table reload
- graph layout
- search query
- inspector update
- job progress

## UI Regression Harness

Suggested layers:

- Swift unit tests for store transitions
- Swift model decoding tests
- UI API tests in Rust
- Workbench launch smoke
- optional Browser/Playwright style smoke for web UI
- manual native app checklist for release candidates

Do not require pixel-perfect screenshots for every change. Use screenshots for:

- layout regressions
- graph visibility
- feature map visibility
- accessibility review
- release evidence

## Manual Review Checklist

For every major Workbench milestone:

- Can a user open the intended artifact without terminal help?
- Does the first viewport show the artifact identity?
- Are long jobs visible and cancellable?
- Are errors recoverable?
- Are common commands in menus?
- Are common commands in command palette?
- Can the workflow be completed by keyboard?
- Is selection preserved across pane changes?
- Are private paths/strings avoided in diagnostics by default?
- Does the UI remain usable while analysis runs?

## Anti-Patterns

Avoid:

- success-shaped status while background work failed
- filtering huge project lists on the Swift main actor
- using raw addresses as route identity
- clearing selection after every refresh
- modal progress for long analysis
- requiring terminal output to diagnose app state
- hiding cache/kernel/KDK status in advanced details
- showing unsupported features as empty successful panes
- exposing AI/MCP results without provenance

## Acceptance Criteria

Workbench polish is measurable when:

- every primary command has a latency budget
- every long-running operation has progress/cancel behavior
- every large list is paged
- every pane synchronizes through routes/object ids
- every error state has a recovery path or typed reason
- keyboard-only workflows are tested
- accessibility fallback exists for graph-heavy views
- diagnostics can prove responsiveness without leaking private binaries
