# Analysis Database Blueprint

Status: Planning and architecture blueprint. This document defines the concrete
workspace storage and indexing layer Damsel needs before it can feel close to
Hopper in day-to-day use.

Purpose: turn Damsel from accurate point queries into a durable
reverse-engineering document. The database is the product center: every view,
script, xref, graph, annotation, byte map, function summary, and future MCP tool
should read the same facts and mutations.

Implementation companion: [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
turns this blueprint into concrete `damsel-core`, `damsel-macho`,
`damsel-cli ui`, and Workbench migration modules, contracts, commands, tests,
and acceptance gates.

## Thesis

Hopper-level interaction is impossible without a mutable project database. A
linear disassembly request can show instructions. A project database lets users
build understanding over hours or days:

- open a binary or cache once
- run staged analysis
- classify bytes
- name things
- comment decisions
- follow references
- correct analysis
- compare versions
- resume later
- ask scripts or future MCP tools for structured facts

Damsel should not treat persistence as "save the current UI state." It should
persist the analysis model itself.

## Non-Goals

The first database must not become a binary patcher, debugger state store, or
general-purpose graph database.

Initial non-goals:

- writing modified Mach-O files
- live-process memory snapshots
- debugger trace databases
- collaborative multi-user editing
- full decompiler IR persistence
- remote server deployment
- generic non-Apple binary support

These can arrive later if the core model proves stable.

## Storage Stages

### Stage 0: In-Memory Model

Build pure Rust model types first. This stage is for API shape, tests, and
determinism.

Capabilities:

- create a project from a standalone Mach-O
- create a project from a projected dyld-cache image
- ingest sections, symbols, imports, exports, ObjC metadata, dyld metadata, and
  initial disassembly facts
- answer functions, xrefs, strings, runtime objects, and byte-map queries
- serialize deterministic JSON snapshots for tests

Why this matters:

- avoids committing too early to SQLite layout
- makes fixtures easy to diff
- lets CLI commands validate the model before Workbench depends on it

Exit criteria:

- snapshot fixtures are stable across runs
- object ids are deterministic
- analysis facts include provenance
- annotation transactions can be replayed

### Stage 1: Append-Only Project Log

Before designing a final store, persist user edits and analysis run metadata in
an append-only log.

Log event classes:

- project created
- source attached
- analysis pass started
- analysis pass completed
- analysis pass failed
- fact batch inserted
- user annotation inserted
- byte classification overridden
- problem suppressed
- problem hint applied
- undo transaction
- redo transaction
- schema upgraded

Benefits:

- undo/redo can share transaction boundaries with persistence
- failed analysis can be inspected
- scripts and future MCP tools get auditable mutation records
- migrations can be tested by replaying old logs

Rules:

- log records never contain raw binary bytes by default
- paths are stored with privacy controls for diagnostics export
- every mutation event has actor, timestamp, and origin
- "origin" distinguishes UI, CLI, script, import, and future MCP

### Stage 2: SQLite Or Embedded Index Store

Once model shape stabilizes, use SQLite or a comparable embedded store for
large-list and cross-index workloads.

Why SQLite is a good default:

- battle-tested local persistence
- transactions and WAL mode
- indexes for symbols, xrefs, strings, function names, selectors, classes, and
  problems
- incremental migrations
- query explain plans for performance work
- easy diagnostics exports without inventing a binary format

Reasons to defer SQLite until Stage 2:

- early schema churn would waste time
- serialized fixture snapshots are more readable while model semantics are
  settling
- the project needs object identity and invalidation rules before table layout

### Stage 3: Sharded Large-Artifact Store

Very large artifacts should not be copied into ordinary project tables.

Candidates for sharded storage:

- raw binary byte windows
- disassembly row caches
- graph layout caches
- decompiler-lite summaries
- large string indexes
- cache-wide xref shards
- KDK/DWARF type imports

Storage options:

- SQLite tables for metadata plus sidecar blobs
- memory-mapped read-only source files
- compressed row caches keyed by source hash and analyzer version
- per-source shards for dyld cache images and kernel collection members

Rule: the canonical model remains object/fact oriented. Shards are performance
artifacts and must be rebuildable.

## Core Entities

### Project

Fields:

- project id
- name
- schema version
- Damsel analyzer version
- created/updated timestamps
- source registry
- analysis profile
- active symbolication profiles
- UI state summary
- project settings

Rules:

- project id is stable inside the project file
- schema version upgrades are explicit
- analyzer version drift marks derived facts stale, not user edits

### Source

A source is any input artifact or sidecar.

Kinds:

- standalone Mach-O
- dyld shared cache
- projected dyld-cache image
- driver cache
- kernelcache
- kernel collection
- fileset member
- dSYM
- KDK bundle
- panic log
- external symbol map
- extracted object

Fields:

- source id
- kind
- role
- path or bookmark reference
- content hash if available
- UUID/build/version metadata
- architecture/platform
- parent source id
- relationship to parent
- local availability
- privacy classification

Key rule: sources are not just file paths. They are provenance anchors for
every fact in the project.

### Address Space

Address identity must be explicit. Apple system analysis has too many address
vocabularies to rely on raw integers.

Address spaces:

- file offset
- image vmaddr
- cache vmaddr
- projected image vmaddr
- kernel collection global vmaddr
- member vmaddr
- runtime address with slide
- panic-frame address

Every address stores:

- address-space id
- numeric value
- display string
- optional file offset
- optional source/member relation
- optional slide information

Rules:

- no cross-source xref uses a naked integer
- conversions are typed and may fail
- failed conversions generate problems when user-visible

### Fact

A fact is an assertion about a project object or range.

Examples:

- this range is code
- this address starts a function
- this instruction calls this target
- this selector belongs to this ObjC method
- this symbol came from a KDK dSYM
- this indirect branch likely uses this jump table
- this vtable slot likely targets this function

Fields:

- fact id
- fact kind
- target object/range
- payload
- provenance
- confidence
- analyzer pass
- run id
- stale flag
- superseded-by relation

Confidence values:

- exact
- strong
- inferred
- ambiguous
- weak
- user_asserted
- contradicted

Rules:

- inferred facts never overwrite user facts silently
- contradictions are modeled, not hidden
- stale facts can remain inspectable while excluded from default answers

### Object

Object ids unify UI, CLI, scripting, and future MCP.

Object kinds:

- project
- source
- segment
- section
- byte range
- symbol
- string
- function
- basic block
- instruction
- xref
- ObjC class
- ObjC category
- ObjC selector
- ObjC method
- C++ type
- C++ vtable
- kernel collection
- kext
- syscall/trap/MIG routine
- annotation
- problem

Rules:

- object ids are opaque to clients
- ids are deterministic where practical
- object ids resolve to routes
- deleted/superseded objects produce tombstones if referenced by old
  annotations

### Annotation

Annotations are user-authored project knowledge.

Kinds:

- label
- local label
- comment
- inline comment
- tag
- bookmark
- color
- register alias
- stack variable name
- function signature
- calling convention
- manual reference
- manual function bound
- byte classification override
- problem suppression
- problem hint

Fields:

- annotation id
- target object/range
- kind
- payload
- actor/origin
- timestamp
- transaction id
- supersedes annotation id

Rules:

- annotations are never generated as if they were analyzer facts
- imported debug names are facts, not user annotations
- user annotations win display precedence over inferred/imported names
- scripts and future MCP mutations use the same annotation model

### Problem

Problems are actionable analysis uncertainty.

Examples:

- possible function start overlap
- unresolved indirect branch
- possible switch table
- bad ObjC method list pointer
- missing dyld cache provider image
- KDK UUID mismatch
- unsupported chained-fixup format
- PAC target could not be resolved

Fields:

- problem id
- severity
- code
- route
- target object
- producing pass
- recovery suggestions
- related facts
- suppressed flag

Rules:

- missing evidence should become a visible problem when it affects workflow
- problems are queryable
- applying a hint records an annotation transaction and schedules reanalysis

## Indexes

### Primary Indexes

Required P0 indexes:

- object id -> object payload
- address/range -> byte classifications
- address -> instruction row
- address -> containing function
- function -> blocks
- function -> outgoing refs
- target -> incoming refs
- source -> symbols
- source -> strings
- name/text -> symbols, functions, strings, classes, selectors
- route -> object
- problem -> route/object
- annotation -> target

Required P1 indexes:

- selector -> implementers
- selector -> callers
- ObjC class -> methods/categories/protocols
- C++ type -> vtables/methods
- vtable -> slots
- kext -> functions/symbols/classes
- cache image -> imports/providers/dependents
- tag/bookmark/color -> targets
- source line -> functions
- type -> users

### Feature Map Index

The feature map needs compact range runs, not row-sized records.

Inputs:

- byte classifications
- function ranges
- string ranges
- ObjC metadata ranges
- C++ metadata ranges
- relocation/fixup ranges
- problems
- bookmarks/tags
- current viewport
- search hits

Outputs:

- compact runs per visible source/image/member
- overlay lanes for problems, bookmarks, search hits, and current route
- source/member boundaries for caches and kernel collections

Performance rule:

- rendering the feature map should not scan the whole database on every
  selection movement

### Search Index

Global search should rank across heterogeneous objects.

Inputs:

- symbols
- demangled names
- function names
- strings
- ObjC classes/selectors/methods
- C++ types/vtables/methods
- kext names
- source paths/install names
- comments
- tags
- problem codes/messages

Ranking signals:

- exact match
- prefix match
- fuzzy score
- object kind priority
- current source/image boost
- user annotation boost
- recent route boost

Rules:

- search results are pageable
- search never blocks UI typing
- stale index versions are explicit

## Invalidation

Analysis facts depend on source bytes, analyzer versions, user hints, imported
symbols, and project settings. Damsel needs dependency-aware invalidation.

Invalidation triggers:

- source replaced or hash changed
- analyzer version changed
- byte classification override
- manual function bound changed
- manual xref added/removed
- debug/KDK source attached or detached
- symbolication profile changed
- ObjC receiver/selector hint changed
- C++ type/vtable hint changed

Invalidation scopes:

- byte range
- function
- source/image
- cache relation graph
- kernel collection member
- whole project

Rules:

- user annotations are not invalidated by analyzer changes
- derived facts can become stale
- stale facts remain inspectable in diagnostics
- reanalysis jobs should report what they invalidated

## Transactions And Undo

Undo must be a model operation, not only a Swift UI action.

Transaction fields:

- transaction id
- origin
- actor
- timestamp
- command name
- preconditions
- mutation records
- invalidated facts
- queued jobs
- user-visible summary

Undoable commands:

- add/edit/delete label
- add/edit/delete comment
- add/remove tag/bookmark/color
- add/remove manual reference
- rename local label
- rename register
- rename stack variable
- set function signature
- set calling convention
- override byte classification
- apply problem hint
- suppress problem

Redo rules:

- redo is invalidated by a new branch of user mutations
- redo cannot silently replay if project preconditions changed
- failed redo reports the changed precondition

## Provenance Precedence

Display names and facts must have stable precedence.

Suggested name precedence:

1. user label
2. exact debug/KDK/dSYM name
3. export or local symbol
4. ObjC method owner/selector name
5. C++ demangled symbol
6. Swift demangled symbol
7. imported/stub name
8. analyzer-generated function label
9. address fallback

Suggested type precedence:

1. user-authored type/signature
2. exact debug/KDK/DWARF type
3. ObjC type encoding
4. C++ mangled-name or RTTI-derived type
5. inferred ABI signature
6. unknown

Contradictions:

- do not discard the losing fact
- show contradictions in inspector/problem list
- let user pick display preference where needed

## Schema Modules

Suggested logical modules:

- `project`
- `sources`
- `address_spaces`
- `objects`
- `facts`
- `byte_classifications`
- `functions`
- `blocks`
- `instructions`
- `references`
- `strings`
- `objc`
- `cpp`
- `kernel`
- `debug_info`
- `types`
- `annotations`
- `transactions`
- `problems`
- `jobs`
- `routes`
- `metrics`

The Rust model should not expose SQL tables directly. Public contracts should
remain domain objects.

## Migration Rules

Every migration must answer:

- what schema version it upgrades from
- whether it touches user annotations
- whether it drops derived facts
- whether reanalysis is required
- whether it is reversible
- how fixtures prove it

Migration tests:

- old project opens read-only
- old project upgrades
- user annotations survive
- stale analyzer facts are marked stale
- missing source paths produce recoverable errors
- privacy-sensitive paths are not leaked into diagnostics

## Performance Budgets

Database-level targets:

- open project manifest: under 300 ms for common cases
- show source/image list: under 500 ms
- global search first page: under 150 ms after warm index
- function detail: under 100 ms for normal functions
- assembly page fetch: under 100 ms
- xref list first page: under 150 ms
- feature map fetch: under 100 ms after classification index exists
- annotation mutation: under 100 ms excluding scheduled reanalysis

Large artifacts:

- dyld shared cache import should show useful progress quickly
- kernel collection inventory should not wait for KDK/DWARF import
- KDK type import should be cancellable and resumable
- search should degrade to partial indexes rather than block the app

## Privacy And Diagnostics

Diagnostics export must be useful without leaking private binaries.

Default diagnostics include:

- schema version
- analyzer version
- source metadata without raw bytes
- source hashes if enabled
- fact counts
- object counts
- index stats
- job timings
- problem summaries
- redacted paths
- crash/error traces

Opt-in diagnostics can include:

- raw instruction bytes for selected ranges
- strings
- path names
- project log excerpts
- query traces

Never include by default:

- whole binary files
- whole dyld caches
- KDK contents
- private strings
- user comments

## Implementation Order

1. Add pure model types for project/source/address/fact/object.
2. Add deterministic snapshot serializer.
3. Add byte classification table from sections/symbols/strings/function starts.
4. Add object id and route resolver.
5. Add annotation transaction model.
6. Add problem model.
7. Add in-memory indexes for functions, refs, strings, and names.
8. Add CLI `project create`, `project facts`, `project functions`, and
   `project xrefs`.
9. Add API endpoints over the same model.
10. Add Workbench project open and navigator over the API.
11. Add append-only event log.
12. Add SQLite store once schema churn slows.

## Acceptance Gates

The database foundation is not complete until:

- fixtures prove deterministic project snapshots
- byte classifications cover mapped ranges
- xrefs can be queried from and to addresses/functions
- annotations persist and undo/redo
- problem records are routeable
- function/detail/list/search endpoints are paged
- Workbench can navigate without rerunning disassembly for every route
- stale analyzer versions are detected
- source UUID/hash mismatch is explicit
- diagnostics export avoids raw binary data by default
