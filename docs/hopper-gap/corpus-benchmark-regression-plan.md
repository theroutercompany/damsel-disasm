# Corpus, Benchmark, And Regression Plan

Status: Validation and measurement plan. This document defines how Damsel
should prove correctness, performance, and Hopper-competitive behavior across
fixtures, synthetic corpora, opt-in real Apple artifacts, and native app
workflows.

Purpose: the Hopper-gap program cannot be validated by a single smoke test or
by successful parsing of one binary. It needs a layered corpus that proves
model correctness, Apple-systems depth, performance, responsiveness, regression
safety, and truthfulness about unsupported cases.

## Thesis

Competitive reverse-engineering tools are trusted because they keep working
across weird binaries, huge system artifacts, malformed metadata, OS-version
drift, stripped symbols, and long-running user sessions. Damsel needs a corpus
and benchmark strategy that is as intentional as its parser strategy.

The validation program should answer:

- did the feature produce correct structured facts?
- did it preserve existing CLI and Workbench behavior?
- did it stay fast enough for interactive use?
- did it avoid overstating ambiguous evidence?
- did it scale from tiny fixtures to dyld caches and kernel collections?
- did it fail clearly when evidence was missing or malformed?

## Current Corpus Baseline

Current checked-in fixture binaries include:

- `arm64-symbolized`
- `arm64-stripped`
- `arm64e-sample`
- `universal-hello`
- `x86_64-only-hello`
- `import-rich`
- `import-lazy`
- `export-kinds`
- `duplicate-symbol-ordinal`
- `indirect-dispatch`
- `relative-dispatch`
- `semantic-switch`
- `objc-sample`
- `swift-sample`
- malformed dyld/stub/ObjC fixtures

Current synthetic shared-cache corpus includes:

- single cache
- split cache
- missing subcache
- absent/present local symbols
- ambiguous basename
- internal linkage
- reexport linkage
- malformed headers/tables
- unsupported architecture

Current verification commands include:

- `./fixtures/build-fixtures.sh --check`
- `./fixtures/tests/build-fixtures-parity.sh`
- `cargo test --workspace`
- `cargo test -p damsel-cli --test cli_snapshots`
- `cargo test -p damsel-cli --test ui_api`
- `cargo bench -p damsel-macho --bench decode_bench --no-run`
- opt-in real-cache validation with `DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT`

This is a solid foundation for parser and command correctness. It is not yet
enough for Hopper-gap project workflows, app responsiveness, decompiler-lite
analysis, kernel collections, KDK/dSYM enrichment, or large-project UI claims.

## Validation Tiers

### Tier 0: Unit Fixtures

Purpose:

- small deterministic tests for individual model/parser/analyzer behavior

Examples:

- address-space conversion
- object id stability
- byte classification range overlap
- annotation transaction replay
- xref direction
- ObjC type encoding decode
- C++ demangle parse
- PAC pointer interpretation
- patchability decision for byte ranges
- patch plan source hash precondition
- runtime module to source matching
- runtime address to project route mapping
- command descriptor validation
- command availability disabled reasons
- navigation stack coalescing
- transaction-backed undo/redo
- ObjC browser class/selector/detail rows
- ObjC dispatch confidence buckets
- C++ vtable and virtual call browser rows
- IOKit candidate evidence labels
- kernel member and panic frame browser rows

Properties:

- no external Apple artifacts
- checked in
- fast enough for every commit
- precise expected output

### Tier 1: Checked-In Binary Fixtures

Purpose:

- prove behavior against real Mach-O binaries produced from source fixtures

Existing examples:

- imports
- exports
- ObjC metadata
- Swift symbols
- indirect dispatch
- switch setup
- malformed metadata

Additions needed:

- path-based source registration fixture
- source availability/missing-source fixture
- byte-window fixture
- job scheduler lifecycle fixture
- job cancellation fixture
- index status fixture
- stale cursor fixture
- response-size/metric fixture
- local helper token fixture
- helper identity/protocol fixture
- source access grant fixture
- stale security bookmark fixture
- diagnostic redaction fixture
- helper crash summary fixture
- helper hang summary fixture
- artifact intake session fixture
- source registration plan fixture
- source graph fixture
- sidecar candidate fixture
- projection recipe fixture
- protected input negative fixture
- procedure seed fixture
- procedure overlap/conflict fixture
- function overlap fixture
- tail-call fixture
- noreturn/error-path fixture
- nested switch fixture
- stack-frame fixture
- register alias fixture
- local label fixture
- ABI/calling-convention fixture
- ObjC block fixture
- ObjC superclass/category edge fixture
- C++ vtable fixture
- C++ RTTI fixture
- virtual call fixture
- ObjC++ mixed method fixture
- PAC/chained-fixup pointer fixture if feasible
- byte patch preview fixture
- AArch64 same-size assembly preview fixture
- AArch64 NOP-region preview fixture
- local debug executable fixture
- automation query fixture
- annotation mutation audit fixture
- MCP tool schema fixture
- plugin manifest fixture
- helper-process failure fixture
- analysis snapshot fixture
- object fingerprint fixture
- function diff fixture
- ObjC surface diff fixture
- C++ vtable diff fixture
- dyld cache dependency diff fixture
- kernel member diff fixture
- evidence package redaction fixture
- small fileset/kernel-collection fixture if feasible

### Tier 2: Synthetic Container Corpora

Purpose:

- deterministic container-level tests where real Apple artifacts are too large,
  private, or OS-dependent

Existing:

- synthetic dyld shared-cache corpus
- export-trie corpus

Additions needed:

- synthetic project database snapshots
- synthetic projected cache image project
- synthetic cache cross-image xref graph
- synthetic function CFG graph corpus
- synthetic graph layout corpus
- synthetic feature-map lane corpus
- synthetic hex-window byte-state corpus
- synthetic representation selection-sync corpus
- synthetic procedure database snapshots
- synthetic architecture-semantics snapshots
- synthetic IR/SSA snapshots
- synthetic patch preview snapshots
- synthetic debug session/module/breakpoint snapshots
- synthetic automation activity snapshots
- synthetic MCP batch partial-failure snapshots
- synthetic plugin-produced fact snapshots
- synthetic plugin capability-denial snapshots
- synthetic analysis snapshot corpus
- synthetic function diff corpus
- synthetic ObjC/C++ runtime diff corpus
- synthetic dyld cache image/dependency diff corpus
- synthetic evidence package corpus
- synthetic trust status corpus
- synthetic source grant corpus
- synthetic diagnostic bundle corpus
- synthetic helper crash/hang corpus
- synthetic update manifest corpus
- synthetic plugin trust policy corpus
- synthetic debugger trust policy corpus
- synthetic artifact intake corpus
- synthetic app-bundle intake corpus
- synthetic extracted-asset-root corpus
- synthetic sidecar matching corpus
- synthetic panic attachment corpus
- synthetic kernel collection/fileset corpus
- synthetic KDK/dSYM mismatch corpus
- synthetic ObjC type encoding corpus
- synthetic C++ demangle corpus
- synthetic Swift demangle corpus
- synthetic dSYM line-table/type corpus
- synthetic dSYM UUID exact/mismatch corpus
- synthetic compile-unit and source-file corpus
- synthetic inline-callsite corpus
- synthetic variable location expression corpus
- synthetic source-root privacy corpus
- synthetic type-conflict corpus
- synthetic type import/export corpus
- synthetic panic log corpus
- synthetic symbolication profile corpus

### Tier 3: Opt-In Local Real Artifacts

Purpose:

- prove Damsel works on actual macOS/iOS system artifacts without making CI
  depend on machine-local files

Artifacts:

- local dyld shared cache
- local split dyld cache
- local shared cache symbols sidecar where available
- extracted iOS app/framework, if user supplies one
- local kernel collection
- KDK bundle, if user installs one
- dSYM bundle, if user supplies one
- panic logs, if user supplies sanitized samples

Rules:

- opt-in only
- never required for ordinary CI
- paths are not stored in public artifacts by default
- evidence packages are redacted
- claims must state exact artifact type and date/run environment

### Tier 4: Private Research Corpus

Purpose:

- allow maintainers to test against private or proprietary Apple-system samples
  without leaking data

Rules:

- never checked in
- diagnostics redacted by default
- summary metrics can be archived
- raw strings/comments/bytes are opt-in
- test harness supports local corpus roots

## Corpus Categories Needed For Hopper Gap

### Project Database Corpus

Cases:

- standalone project
- projected cache-image project
- multi-source project
- path-registered source
- missing source
- source hash changed
- schema version mismatch
- analyzer version mismatch
- project reopen
- corrupt project log

Expected evidence:

- manifest snapshot
- source registry snapshot
- object id snapshot
- route resolution snapshot
- stale fact behavior
- recovery errors

### Scheduler, Index, And Paging Corpus

Cases:

- source registration starts inventory job
- foreground job preempts background queue at checkpoint
- job cancellation acknowledged
- job cancellation checkpoint stops pass
- failed job creates problem record
- index building state
- partial index query
- ready index query
- stale index query
- stale cursor error
- paged symbols query
- paged strings query
- paged procedures query
- paged problems query
- response-size metric
- slow-query metric
- event stream reconnect with project version refresh

Expected evidence:

- job lifecycle snapshots
- job progress snapshots
- cancellation snapshots
- problem record snapshot
- index status snapshots
- paged response snapshots
- stale cursor snapshot
- metrics snapshots
- event stream snapshots

### Byte Classification Corpus

Cases:

- headers/load commands
- code section
- string section
- data section
- ObjC metadata range
- C++ metadata range
- fixup/binding range
- unknown range
- user override
- overlapping inferred ranges

Expected evidence:

- byte map snapshot
- compact feature-map runs
- override transaction
- targeted invalidation
- restored inferred classification

### Hex, Patch, And Debug Corpus

Cases:

- read-only hex window for text bytes
- read-only hex window for data bytes
- virtual zero-fill byte window
- projected cache byte window
- unsupported patch target
- same-size AArch64 replacement preview
- invalid assembly diagnostic
- NOP-region preview
- patchability query for selected instruction
- patchability query for readonly projection
- Patch Studio byte replacement dry run
- Patch Studio assembler dry run
- Patch Studio NOP dry run
- copy-on-write patch revision
- patch history snapshot
- code-signing warning snapshot
- dyld shared-cache projection patch rejection
- kernel collection patch rejection
- patch evidence export
- stale source hash rejection
- source revision snapshot
- debug module exact UUID match
- debug module mismatch
- runtime address maps to project route
- project route maps to runtime breakpoint address
- unresolved breakpoint problem
- launch local signed fixture
- attach denied fixture or simulated denial
- stop at entry event
- breakpoint hit event
- PC outside known source
- register snapshot with pointer interpretation
- stack memory with route pointer
- helper crash event
- redacted debugger export

Expected evidence:

- byte-window snapshots
- patch preview snapshots
- patchability snapshots
- patch impact snapshots
- patch revision snapshots
- patch history snapshots
- assembler/NOP diagnostic snapshots
- invalidation plan snapshots
- source revision snapshots
- patch evidence export snapshots
- debug mapping snapshots
- debug target/session/module snapshots
- breakpoint and stop-event snapshots
- register/frame/memory snapshots
- helper-isolation test output
- platform-denial problem snapshots where environment supports them

### Function And Xref Corpus

Cases:

- symbol-seeded function
- stripped function
- function-start function
- ObjC IMP function
- direct call
- tail call
- conditional branch
- back edge
- jump table
- data reference
- string reference
- import/stub call
- cross-image cache link
- unresolved indirect branch

Expected evidence:

- function list snapshot
- function detail snapshot
- CFG snapshot
- xrefs-from/xrefs-to snapshots
- caller/callee index
- unresolved problem record

### ObjC Corpus

Cases:

- class
- metaclass
- protocol
- category
- method
- property
- ivar
- selector ref
- class ref
- `objc_msgSend`
- `objc_msgSendSuper`
- category method collision
- ambiguous receiver
- malformed method list

Expected evidence:

- class browser snapshot
- selector implementer snapshot
- selector caller snapshot
- method-to-IMP/function link
- type-encoding decode
- ambiguous receiver problem
- manual binding reanalysis

### C++ And ObjC++ Corpus

Cases:

- demangled function
- namespace/class method
- constructor/destructor
- RTTI typeinfo
- vtable
- virtual call
- multiple inheritance
- ObjC++ method calling C++ virtual method
- stripped vtable-like table
- false-positive vtable

Expected evidence:

- demangle snapshot
- vtable snapshot
- virtual call candidate snapshot
- inheritance graph snapshot
- false-positive rejection

### Kernel And KDK Corpus

Cases:

- synthetic fileset
- kernel collection member list
- kext identity
- KDK exact match
- KDK mismatch
- dSYM match
- syscall/trap/MIG table
- IOKit class pattern
- panic log with slide
- panic log missing slide
- panic frame outside known member

Expected evidence:

- kernel overview snapshot
- member/kext snapshot
- KDK status
- symbolication output
- panic frame routes
- mismatch problems

### Decompiler And IR Corpus

Cases:

- straight-line arithmetic
- simple if
- loop
- switch
- tail call
- stack local
- argument use
- return value
- ObjC message
- ObjC ivar candidate
- C++ virtual call
- ObjC++ mixed call
- arm64e authenticated indirect call
- chained-fixup call target
- KDK-backed kernel call
- unresolved indirect
- dead/unreachable block
- huge optimized function

Expected evidence:

- IR snapshot
- SSA summary
- stack frame snapshot
- function summary
- pseudocode view snapshot
- pseudocode source-map snapshot
- confidence and issue snapshot
- variable rename/type edit transaction snapshot
- large-function partial-result metric
- unresolved fact visibility

### Workbench UX Corpus

Cases:

- tiny fixture open
- long instruction listing
- branch-heavy graph
- ObjC browser
- shared-cache image browser
- large symbol search
- problems panel
- missing source project
- canceled analysis job
- KDK mismatch

Expected evidence:

- UI smoke
- selection preservation
- keyboard navigation
- latency metrics
- screenshot or structured UI state where useful
- no terminal-output dependency

### Automation, MCP, And Plugin Corpus

Cases:

- project summary tool
- search tool
- route resolution tool
- function detail tool
- bounded byte-window tool
- xref tool
- ObjC selector tool
- kernel member tool
- batch request with one failing item
- comment mutation with audit record
- label mutation with undo
- permission denial for byte read
- privacy redaction for diagnostics
- plugin manifest accepted
- plugin manifest rejected for incompatible schema
- plugin fact emission
- plugin problem emission
- plugin cancellation
- helper-process crash isolation
- Automation Studio recipe catalog
- Automation Studio run cancellation
- MCP session binding and call log
- capability grant and revoke
- script package timeout
- emitted fact provenance
- ObjC selector audit recipe
- dyld cache provider graph recipe
- IOKit external method audit recipe
- panic frame pack recipe
- C++ vtable sweep recipe
- patch plan review recipe
- assistant context route snapshot
- assistant tool-call timeline snapshot
- assistant citation validation
- assistant hallucinated-route rejection
- assistant privacy redaction
- assistant reviewed suggestion undo
- assistant ObjC dispatch explanation
- assistant C++ virtual call explanation
- assistant panic frame explanation
- assistant patch plan review
- assistant debugger stop explanation

Expected evidence:

- tool schema snapshots
- tool response snapshots with routes/provenance
- batch partial-failure snapshot
- automation activity snapshot
- annotation transaction snapshot
- capability-denial snapshot
- redaction snapshot
- plugin manifest validation snapshot
- plugin-produced fact provenance snapshot
- helper isolation problem record
- recipe output snapshots
- automation run snapshots
- capability prompt snapshots
- activity log snapshots
- diagnostics redaction snapshots
- assistant message/output snapshots
- assistant evidence citation snapshots
- assistant privacy-mode snapshots
- assistant suggestion transaction snapshots

### Diff, Export, And Evidence Corpus

Cases:

- project snapshot manifest
- selected function export
- no-raw-bytes default export
- redacted local path export
- function added
- function removed
- function changed
- symbol rename
- ambiguous fuzzy match
- manual match pin
- manual match rejection
- ObjC class added/removed
- selector implementation changed
- C++ vtable slot changed
- dyld image added/removed
- provider/importer/reexport changed
- kernel member changed
- KDK mismatch changes symbolication
- panic frame route remapped
- Markdown report export
- JSON evidence package export

Expected evidence:

- snapshot manifest snapshots
- fingerprint snapshots
- diff finding snapshots
- ambiguous match snapshots
- match transaction snapshots
- ObjC/C++/dyld/kernel diff snapshots
- evidence package manifest
- redaction log
- report output snapshot
- reproducibility command list

## Performance Benchmark Classes

### Parser/Loader Benchmarks

Metrics:

- load time
- parse time by section/metadata family
- peak resident memory where available
- bytes read
- allocations if available

Targets:

- standalone small fixture: effectively instant
- import/ObjC fixtures: low hundreds of milliseconds at most
- shared-cache inventory: progress visible quickly
- kernel collection inventory: member list before deep analysis

### Disassembly Benchmarks

Metrics:

- decode throughput
- instructions per second
- semantic annotation cost
- analysis cost by pass
- stop reason
- memory usage

Cases:

- stripped/symbolized arm64
- arm64e
- semantic-switch
- indirect-dispatch
- relative-dispatch
- import-rich
- projected cache image

### Project Database Benchmarks

Metrics:

- project create time
- source registration time
- fact insertion rate
- index build time
- query latency
- project reopen time
- migration time
- database size

Queries:

- list functions
- function detail
- xrefs to/from
- search
- byte map range
- feature map
- problem list
- annotation write
- undo/redo

### Workbench Responsiveness Benchmarks

Metrics:

- app launch to ready
- file/project open to first useful UI
- selection-to-inspector update
- route navigation latency
- search keystroke to results
- graph open time
- feature-map update time
- cancel button response
- main-thread stall count

Targets:

- toolbar/menu interaction under 50 ms
- selection/inspector update under 100 ms for warm data
- search first page under 150 ms after warm index
- project manifest visible under 300 ms for common cases
- cancellation acknowledged immediately, even if worker stops later

### Memory Benchmarks

Metrics:

- Swift process RSS
- Rust/server process RSS
- duplicated byte storage
- database cache size
- graph layout cache size
- string index size
- KDK/DWARF import size

Rules:

- local project open must avoid copying whole large files into Swift `Data`
- Rust should memory-map file-backed sources where practical
- Workbench lists must be paged
- diagnostics should capture high-level memory stats without raw bytes

## Regression Gates

### Every Commit Gate

Run:

```sh
cargo test --workspace
./fixtures/build-fixtures.sh --check
./fixtures/tests/build-fixtures-parity.sh
swift test --package-path damsel-app
```

If Swift or macOS-only tooling is unavailable, record the skip explicitly.

### Docs Gate

Run:

```sh
find docs/hopper-gap -maxdepth 1 -type f -print
wc -l docs/hopper-gap/*.md
wc -w docs/hopper-gap/*.md
rg -n "work marker pattern" docs/hopper-gap README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
rg -n "[^[:ascii:]]|[ \t]+$" docs/hopper-gap README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
git diff --check -- README.md docs/roadmaps/apple-runtime-analysis-roadmap.md
```

### Feature Branch Gate

Add:

- focused unit tests for touched models
- CLI snapshot for new command output
- UI API test for new endpoint
- Workbench model decode test for new payloads
- fixture snapshot for new analyzer facts

### Release Gate

Add:

- decode bench compile
- selected benchmark run
- Workbench fixture launch smoke
- optional real shared-cache smoke if claims mention real caches
- optional KDK/kernel smoke if claims mention KDK/kernel readiness
- diagnostics export redaction check

## Benchmark Output Format

Every benchmark should emit structured output:

```json
{
  "benchmark": "project_create",
  "artifact": "semantic-switch",
  "artifact_kind": "fixture_macho",
  "host": {
    "os": "macos",
    "arch": "arm64"
  },
  "metrics": {
    "duration_ms": 42,
    "peak_rss_mb": 64,
    "fact_count": 1200
  },
  "damsel": {
    "git_commit": "unknown",
    "schema_version": 1,
    "analyzer_version": "project-v1"
  }
}
```

Rules:

- include artifact kind
- include host OS/arch
- include Damsel commit when available
- include schema/analyzer versions for project benchmarks
- redact paths by default
- separate fixture and real-artifact results

## Performance Regression Policy

Define thresholds per benchmark:

- warning threshold: investigate
- failure threshold: block merge
- known-regression threshold: requires explicit note

Example:

- project manifest open regresses by more than 20 percent: warning
- search first page regresses by more than 30 percent: failure
- memory doubles for same fixture: failure unless justified

Rules:

- do not compare different artifact classes
- do not compare cold and warm runs without labeling
- record host information
- keep noisy UI timing thresholds wider than deterministic model benchmarks

## Real-Artifact Evidence Policy

When reporting real shared-cache or kernel evidence, include:

- artifact kind
- OS build if available
- architecture
- cache/kernel UUID if safe to report
- Damsel commit
- command run
- pass/fail summary
- redaction status
- whether raw artifacts were included

Do not claim:

- "real-cache support" from one projected image command
- "kernel support" from standalone Mach-O parsing
- "KDK support" from accepting a path without UUID/type validation
- "Hopper-level performance" without latency and memory measurements

## Workbench UX Measurement

Use automated and manual probes:

- launch app
- open fixture
- open project
- search symbol
- jump to address
- select instruction
- open graph
- open inspector module
- cancel analysis
- recover from server failure

Record:

- time to ready
- time to first instruction
- time to first search result
- main-thread stall symptoms
- selection correctness
- visible error state
- screenshot only when it proves layout

UI tests should assert state, not pixel perfection, except for targeted layout
smoke checks.

## Corpus Expansion Roadmap

### Phase C0: Document Current Corpus

- update fixture inventory
- map each fixture to feature coverage
- identify missing coverage
- link tests to fixtures

### Phase C1: Project Foundations

- project manifest fixture
- byte map fixture
- source mismatch fixture
- annotation transaction fixture
- problem/hint fixture

### Phase C2: Function/Xref/Graph

- function overlap
- tail call
- switch/jump table
- reverse xrefs
- graph export

### Phase C3: Runtime Objects

- ObjC callsite
- ObjC ambiguity
- C++ vtable
- C++ virtual call
- ObjC++ mixed fixture

### Phase C4: Apple System Containers

- cache cross-image xrefs
- driver-cache inventory if supported
- fileset/kernel collection synthetic fixture
- KDK mismatch fixture
- panic log fixture

### Phase C5: Hex, Patch, And Debug Foundations

- byte-window fixture
- patch preview fixture
- assembler diagnostic fixture
- source revision fixture
- local debug executable fixture
- runtime mapping fixture

### Phase C6: Large And Real Artifacts

- opt-in real-cache benchmark
- opt-in KDK benchmark
- opt-in large-project Workbench smoke
- redacted evidence package

## Acceptance Criteria

The Hopper-gap validation program is credible when:

- every major feature has a fixture or explicit opt-in evidence plan
- every performance claim has a metric name and benchmark class
- every Workbench responsiveness claim has a UI probe
- every real-artifact claim identifies whether evidence is opt-in
- malformed/ambiguous inputs produce typed problems instead of silent success
- baseline commands remain covered
- project persistence and migrations have reopen tests
- privacy/redaction is tested
- release notes can cite evidence without exposing private artifacts
