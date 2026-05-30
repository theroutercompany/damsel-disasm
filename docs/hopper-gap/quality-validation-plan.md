# Quality And Validation Plan

Status: Validation strategy for the Hopper-competitive documentation and future
implementation program.

Purpose: define how Damsel proves the Hopper-gap work is real, complete,
performant, and trustworthy. This document also gives a completion-audit shape
for this documentation pack and later implementation milestones.

## Validation Thesis

The goal is broad enough that "docs exist" or "tests pass" is not sufficient.
Damsel needs traceable proof across:

- documentation completeness
- model/API contract coverage
- fixture-backed parser correctness
- fixture-backed analysis correctness
- real-system opt-in evidence
- Workbench UX behavior
- performance responsiveness
- source/provenance trust
- regression safety for existing commands

Validation should be requirement-driven. Each requirement needs evidence that
matches its scope.

## Documentation Validation

### Required Docs In This Pack

The docs pack should include, at minimum:

- entrypoint and pack map
- feature gap inventory
- target architecture
- feature specifications
- implementation guide
- integration roadmap
- research sources
- competitive UX/app feature plan
- performance/responsiveness architecture
- corpus/benchmark/regression plan
- UX latency and polish measurement plan
- analysis database blueprint
- refactor/subsystem plan
- product requirements
- Workbench information architecture
- end-to-end workflow playbooks
- API/data contracts
- search/navigation/index plan
- native app polish plan
- Apple systems specialization plan
- type and symbolication system plan
- decompiler and IR strategy
- debugger/automation/extension/MCP plan
- current code integration map
- wave 1 project foundation implementation spec
- project kernel and typed byte map implementation spec
- semantic Workbench surfaces implementation spec
- graph/feature-map/hex representation sync implementation spec
- command/navigation/undo Workbench interaction implementation spec
- Hopper public-web feature architecture audit
- Hopper feature architecture public evidence
- large-project performance/scheduler/indexing implementation spec
- Apple system diffing/export/evidence implementation spec
- function recovery and procedure database plan
- decompiler/function/IR implementation spec
- automation/MCP/scripting implementation spec
- plugin/SDK extension boundary implementation spec
- patching/hex/assembly/transaction implementation spec
- debugger static-dynamic integration implementation spec
- native Workbench Hopper-grade implementation spec
- Apple runtime Workbench browser implementation spec
- native app trust/distribution/diagnostics implementation spec
- Apple artifact intake/source-registration implementation spec
- Apple runtime and kernel object model implementation spec
- DWARF/dSYM/KDK source-level debug-info implementation spec
- architecture decision and risk register
- implementation epic breakdown
- evidence and acceptance matrix
- quality/validation plan

### Documentation Quality Checks

Run:

- file inventory
- word/line counts
- unresolved work-marker scan
- ASCII/trailing whitespace scan
- markdown link sanity where possible
- `git diff --check`

Review:

- each doc has status/purpose
- each doc is linked from pack README
- each major feature has acceptance criteria somewhere
- external research sources are recorded
- current Damsel baseline is not overstated
- real-cache/kernel validation is not claimed unless run

## Requirement Traceability

Use a trace table for future implementation:

| Requirement | Doc source | Model/API | Implementation | Test evidence | Real evidence |
| --- | --- | --- | --- | --- | --- |
| Project database | product, architecture | project models | project runtime | fixture snapshot | reopened project |
| Function xrefs | specs | xref API | xref index | fixture xrefs | framework sample |
| ObjC browser | product/specs | ObjC APIs | runtime store | objc fixture | Foundation/libobjc |
| Kernel collection | specs | kernel APIs | fileset loader | synthetic fileset | KDK kernel |
| Responsiveness | performance | metrics API | scheduler/paging | UI smoke | large cache |
| Performance scheduler | performance spec | source/job/index APIs | runtime scheduler | job/index fixtures | large artifact smoke |
| Native app polish | polish plan | route/jobs APIs | Workbench shell | UI tests | large project smoke |
| Command/navigation/undo | command interaction spec | command, route, transaction APIs | Workbench command spine | menu/palette/undo tests | keyboard-only app smoke |
| Apple runtime browsers | runtime browser spec | ObjC/C++/cache/kernel/panic browser APIs | Workbench browser suite | browser route/list/detail tests | ObjC/cache/KDK/panic smoke |
| Apple specialization | specialization plan | cache/kernel/runtime APIs | source/runtime stores | synthetic fixtures | cache/KDK samples |
| Database persistence | database blueprint | project schema | event log/store | migration tests | reopened project |
| Decompiler/IR | IR strategy | IR/summary schemas | semantic passes | IR snapshots | selected real functions |
| Decompiler Workbench | pseudocode Workbench spec | pseudocode/job/source-map APIs | native pseudocode stores | route/source-map/edit/export UI tests | selected ObjC/C++/kernel functions |
| Procedure database | procedure plan | procedure/block/call APIs | procedure recovery | function snapshots | framework sample |
| Hopper feature architecture | feature evidence | architecture coverage | doc/roadmap mapping | source/cross-link checks | current public sources |
| Hopper public audit | public audit | source-to-requirement map | roadmap/API cross-links | release/source checks | current public release API |
| Diff/export/evidence | diff spec | snapshot/fingerprint/diff APIs | diff/export service | snapshot/export/diff fixtures | OS-build sample if claimed |
| Automation/MCP | automation plan | object/tool APIs | script/MCP server | tool fixtures | local-only smoke |
| Plugin/SDK boundary | plugin spec | manifest/capability APIs | extension runtime | manifest/isolation fixtures | local toy extension |
| Debugger bridge | debugger integration spec | debug session APIs | LLDB bridge | fixture executable | signed local sample |
| Live debugger Workbench | live debugger spec | target/session/event/breakpoint APIs | native debugger stores | launch/attach/stop/register/memory UI tests | signed local sample plus denial cases |
| Graph/representation sync | graph spec | graph/feature-map/hex schemas | graph/layout/hex services | graph/hex/feature-map fixtures | Workbench graph smoke |
| Patching/hex | patching spec | byte/patch APIs | patch preview/export | byte/patch fixtures | standalone sample |
| Implementation epics | epic breakdown | issue map | phased branches | per-epic gates | milestone audit |
| Existing-code integration | code integration map | crate/module plan | migration slices | existing tests plus new slices | repo audit |
| Completion audit | evidence matrix | proof table | inspected current state | commands/tests | real samples if claimed |
| Corpus coverage | corpus plan | corpus manifest | fixtures/benches | fixture drift checks | opt-in real artifacts |
| UX latency | UX measurement plan | metrics/signposts | Workbench probes | UI/store tests | native smoke |
| Native app trust | trust/distribution spec | trust/helper/source/diagnostic schemas | helper auth and source grants | token/redaction fixtures | signed artifact if claimed |
| Artifact intake | intake spec | intake/source graph schemas | project ingest and artifact overview | intake/source graph fixtures | real artifact smoke if claimed |
| Wave 1 foundation | wave 1 spec | project model schema | first branch | project snapshots | fixture smoke |
| Decision/risk tracking | risk register | ADR/risk entries | wave reviews | checklist evidence | incident updates |
| End-to-end workflows | workflow playbooks | route/API map | Workbench flows | workflow smokes | real artifact sessions |
| Search/navigation | search plan | route/search APIs | indexes/stores | route/search tests | large project search |
| Type/symbolication | type plan | type/name schemas | importers/resolvers | dSYM/KDK fixtures | matched sidecars |
| DWARF/dSYM/KDK debug info | debug-info spec | debug source/profile/source-line schemas | dSYM/KDK/DWARF importers | line/function/variable/type fixtures | matched sidecars and opt-in KDK |

The table should live in a future validation document or release checklist.

## Existing Regression Gates

Existing gates should remain:

- `cargo test --workspace`
- fixture build checks
- fixture parity checks
- disassembler alpha validator
- decode bench compile
- optional LLVM comparison
- optional real shared-cache smoke
- Workbench Swift tests
- UI API tests
- Workbench fixture launch smoke

Do not replace these with new Hopper-gap tests. Add to them.

## New Model Gates

### Project Model Gate

Evidence:

- deterministic project manifest snapshot
- source registry snapshot
- address-space snapshot
- project reopen test
- source hash changed test
- schema version mismatch test

### Fact/Provenance Gate

Evidence:

- facts include source/confidence/analyzer
- conflicting facts can coexist
- user labels do not erase raw names
- ambiguous facts serialize distinctly

### Address-Space Gate

Evidence:

- standalone image address mapping test
- projected cache image mapping test
- cache vmaddr to image offset test
- fileset member mapping test
- slid runtime address mapping test later

## New Analysis Gates

### Function Discovery Gate

Fixtures:

- simple function
- branch-heavy function
- jump table
- tail call
- ObjC IMP
- C++ vtable entry
- thunk

Evidence:

- function count
- block count
- edges
- callers/callees
- unresolved edges
- overlap conflicts

### Xref Gate

Fixtures:

- direct call
- data reference
- string reference
- import call
- ObjC selector/class ref
- C++ vtable slot
- kernel table entry

Evidence:

- forward refs
- reverse refs
- stable sort
- ambiguous targets preserved

### ObjC Gate

Fixtures:

- class
- protocol
- category
- property
- ivar
- selector
- message-send
- super send
- block literal later

Evidence:

- class list
- selector implementers
- selector callers
- method IMP links
- type encoding parse
- unresolved message-send reasons

### C++ Gate

Fixtures:

- demangled symbols
- vtable
- RTTI/typeinfo
- virtual call
- constructor/destructor
- multiple inheritance
- thunk
- IOKit-style class

Evidence:

- type records
- vtable records
- slots
- inheritance graph
- virtual call candidates

### Kernel Gate

Fixtures:

- synthetic fileset
- synthetic kext metadata
- KDK/dSYM UUID match
- KDK/dSYM mismatch
- ObjC type encoding import
- C++ demangle signature candidates
- Swift demangled name facts
- function signature precedence
- calling convention and hidden-argument mapping
- stack/local/register alias records
- type import/export round trip
- type conflict resolution
- panic log
- syscall/trap table synthetic
- IOKit synthetic

Evidence:

- members listed
- address spaces created
- kext graph
- symbols attached
- panic frames symbolicated

### Type/Signature/Debug-Info Gate

Fixtures:

- ObjC type encoding corpus
- C++ demangle corpus
- Swift demangle corpus
- dSYM line-table/type corpus
- dSYM UUID exact/mismatch corpus
- compile-unit and source-file corpus
- inline-callsite corpus
- variable location expression corpus
- source-root privacy corpus
- type-conflict corpus
- type import/export corpus
- symbolication profile mismatch corpus

Evidence:

- type/signature/debug-info store snapshots
- procedure signature precedence snapshots
- Workbench type navigator and signature editor store tests
- symbolication profile switch tests
- conflict resolution and transaction replay tests

## UI Validation Gates

### Workbench Navigation Gate

Evidence:

- open project
- select function
- switch assembly/graph
- jump to xref
- back/forward
- search result jump
- selection preserved

### Workbench Object Browser Gate

Evidence:

- function list renders
- symbol list renders
- string list renders
- ObjC browser renders
- C++ browser renders when fixture exists
- kernel browser renders when fixture exists
- cache browser renders when fixture exists

### Workbench Annotation Gate

Evidence:

- add label
- add comment
- add bookmark
- undo
- redo
- save project
- reopen project
- annotation still visible

### Workbench Hex And Patch Preview Gate

Evidence:

- open hex at selected instruction
- instruction byte span is highlighted
- byte classification overlay is visible
- unavailable/virtual bytes are explicit
- patch preview shows original and replacement bytes
- unsupported patch target explains why it is unsupported
- original input hash is unchanged

### Workbench Debugger Mapping Gate

Evidence:

- configure local debug target
- launch or attach with explicit user action
- list runtime modules
- map current PC to route, or show runtime-only state
- set route-backed breakpoint
- show registers and backtrace
- show platform permission failures as problems

### Workbench Error-State Gate

Evidence:

- unsupported file
- malformed file
- missing sidecar
- KDK mismatch
- cancelled analysis
- no ObjC metadata
- empty search results

Each error state should be visible, understandable, and non-crashing.

## Performance Validation Gates

### Backend Performance

Measure:

- source registration
- Mach-O parse
- shared-cache inventory
- projected image analysis
- function indexing
- xref indexing
- ObjC analysis
- C++ analysis
- kernel member listing
- graph layout

### Frontend Responsiveness

Measure or smoke:

- app launch
- first metadata visible
- first instruction visible
- search first result
- row selection latency
- inspector update latency
- graph first paint
- scroll smoothness
- cancellation responsiveness

### Large Input Gates

Opt-in:

- current macOS dyld cache inventory
- selected Foundation/CoreFoundation/libobjc/libdispatch analysis
- KDK kernel collection inventory
- selected kernel member analysis

No broad performance claim should be made unless the relevant gate ran.

## Real Evidence Matrix

### Minimum Private Alpha

- fixture suite passes
- one local macOS dyld cache selected-image smoke
- one ObjC framework fixture or real framework sample
- one C++ fixture
- synthetic kernel fileset
- Workbench fixture smoke

### Strong Private Beta

- two macOS dyld cache versions
- at least one iOS dyld cache if legally available
- KDK-matched kernel collection
- stripped kernel collection without KDK
- Objective-C-heavy framework
- C++/IOKit-heavy binary
- Workbench performance smoke

### Public Claim Bar

Do not publicly claim Hopper-competitive until:

- project reopen works
- function/xref indexes are durable
- Workbench navigation is smooth
- ObjC browser is useful
- cache and kernel workflows are first-class
- real evidence matrix is documented

## UX Review Checklist

For each new Workbench surface:

- Does it have an empty state?
- Does it have a loading state?
- Does it have an error state?
- Is it keyboard reachable?
- Is it route-addressable?
- Does it preserve selection?
- Does it handle large lists?
- Does it show source/provenance when needed?
- Does it avoid text overlap and clipping?
- Does it avoid blocking analysis or UI?

## Documentation Completion Audit

The documentation pack can be considered substantially complete only when:

- all required docs exist
- pack README links every doc
- source docs mention current Damsel evidence and external sources
- UX/app polish is treated as first-class
- performance/responsiveness is treated as first-class
- large refactors/subsystems are explicitly named
- specs include acceptance criteria
- implementation guide covers crate/module integration
- roadmap includes sequencing and gates
- validation plan explains how to prove completion

This audit should be run before marking the documentation goal complete.

## Implementation Completion Audit

The future implementation program is complete only when current evidence proves:

- project database works
- source registry works
- function/xref indexes work
- Workbench navigation works
- annotations persist
- search works across object types
- ObjC browser works
- C++ browser works for fixtures
- kernel browser works for fixtures
- shared-cache browser works for real selected images
- performance gates pass
- validation docs record what ran

If a requirement lacks evidence, it is not complete.
