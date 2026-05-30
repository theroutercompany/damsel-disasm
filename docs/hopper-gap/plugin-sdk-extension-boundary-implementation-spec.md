# Plugin And SDK Extension Boundary Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not add a public Damsel plugin ABI today.

Purpose: define the extension boundaries Damsel should grow toward if it wants
to be competitive with Hopper's SDK without prematurely freezing unstable Rust
internals. This covers loader plugins, sidecar importers, architecture
semantics providers, analyzer passes, renderer/exporter tools, Workbench
commands, plugin manifests, capability checks, sandboxing, versioning,
provenance, cancellation, and validation.

Hopper's SDK proves that extensibility is part of the product, but it also
shows that different extension classes have different contracts. A file loader
is not a CPU backend. A CPU backend is not a menu tool. A menu tool is not an
MCP server. Damsel should preserve those boundaries from the start.

## Research Companions

- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)

## Architecture Companions

- [Current Code Integration Map](current-code-integration-map.md)
- [Refactor And Subsystem Plan](refactor-subsystem-plan.md)
- [API And Data Contracts](api-data-contracts.md)
- [Analysis Database Blueprint](analysis-database-blueprint.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Apple Runtime And Kernel Object Model Implementation Spec](apple-runtime-kernel-object-model-implementation-spec.md)

## Public Hopper SDK Evidence

Official SDK facts:

- Hopper plugins conform to a common `HopperPlugin` protocol.
- A plugin reports SDK version, UUID, plugin type, name, description, author,
  copyright, version, and command-line identifiers.
- Hopper's public SDK defines at least three plugin types: file loader, CPU,
  and tool.
- File loaders detect file types from bytes, return detected file type
  objects, provide CPU family/subfamily/address-width hints, expose optional
  loader UI choices, create segments and sections, set mapped data, entrypoints,
  potential procedures, labels, and comments, and report progress during long
  loads.
- File loaders can load debug data against an already loaded file.
- File loaders can extract an inner file from a composite container and later
  adjust the final disassembled file after extraction.
- CPU plugins expose architecture metadata, registers, syntax variants, CPU
  modes, stack/frame/program-counter recognition, NOP bytes, assembler
  capability, decompiler capability, branch analysis, switch analysis, thunk
  detection, rendering, and decompiler cooperation.
- Tool plugins return menu descriptions with selector names that Hopper invokes
  against the current document.
- The SDK quick tour says macOS plugins need proper signing on modern macOS.
- The SDK uses install paths split by plugin type.

Public plugin ecosystem evidence:

- Public Hopper plugin lists include CPU plugins and loader plugins for many
  file formats and architectures.
- Public scripts and MCP wrappers show user demand for tool-like extensions
  even when the public SDK surface is limited.

## Core Thesis

Damsel should not expose its internal Rust structs as a plugin ABI.

Instead, Damsel should evolve through layers:

1. Internal traits for loaders, sidecars, analyzers, architecture semantics,
   renderers, and exporters.
2. Stable project/fact/query APIs used by CLI and Workbench.
3. Manifest-driven external tools that communicate through versioned JSON.
4. Sandboxed or helper-process plugins for untrusted third-party code.
5. A public SDK only after schema, provenance, cancellation, and performance
   contracts are stable.

The first useful "SDK" may be a stable CLI/API/MCP contract, not a dynamic
library ABI.

## Non-Goals For Early Waves

Do not implement first:

- arbitrary dynamic library loading into the Workbench process
- stable Rust ABI promises
- plugin access to raw mutable database handles
- plugin access to Swift UI internals
- plugin execution without cancellation
- plugin mutation without transactions
- unbounded file-system access
- plugin-defined UI inside core Workbench layout
- third-party CPU decompiler backends

Early extension work should be internal-trait-first and manifest-first.

## Extension Classes

### Loader Extension

Purpose:

- recognize and load a source artifact

Examples:

- alternate Apple firmware container
- custom extracted dyld-cache format
- private lab symbol-map package
- future IPSW asset pack

Inputs:

- bounded byte header
- file name/path metadata
- user loader options

Outputs:

- source records
- address spaces
- segments
- sections
- mapped byte providers
- entrypoints
- potential procedure seeds
- initial byte classifications
- initial names/comments/facts
- problems
- load metrics

Rules:

- loader cannot mutate existing project facts except through source
  registration
- loader cannot run expensive full analysis inline
- loader emits progress and supports cancellation
- loader records version and provenance

### Sidecar Importer

Purpose:

- attach debug/symbol/type/profile data to an existing source

Examples:

- dSYM
- KDK bundle
- `.symbols` sidecar
- private symbol map
- panic log
- generated type headers

Inputs:

- existing project/source
- sidecar bytes/path
- matching policy

Outputs:

- symbolication profile
- names
- types
- line tables
- source match or mismatch problems
- provenance facts

Rules:

- sidecar facts never silently overwrite user annotations
- sidecar match confidence is explicit
- UUID/build/hash mismatch becomes a problem

### Architecture Semantics Provider

Purpose:

- provide semantics above raw instruction decode

Examples:

- AArch64/AArch64e provider
- future x86_64 provider
- future custom firmware CPU provider

Outputs:

- normalized instruction semantics
- branch/call/return effects
- register effects
- stack/frame effects
- memory reference candidates
- switch candidates
- thunk candidates
- assembler/NOP support
- rendering spans

Rules:

- first-party only until Damsel's semantics contract is stable
- output must be deterministic
- unsupported instructions produce explicit partial semantics

### Analyzer Pass

Purpose:

- derive project facts from existing sources/facts

Examples:

- function seed collector
- xref builder
- ObjC message recognizer
- C++ vtable recognizer
- kernel MIG recognizer
- decompiler-lite summary pass

Inputs:

- declared fact dependencies
- selected source/procedure/range scope
- analysis options

Outputs:

- facts
- problems
- metrics
- invalidation dependencies

Rules:

- analyzer writes facts, not annotations
- analyzer cannot overwrite user facts
- analyzer is cancellable
- analyzer emits stale-version metadata
- analyzer can be re-run deterministically

### Renderer And Exporter

Purpose:

- convert project facts into reports, graph files, external formats, or copied
  selections

Examples:

- Markdown report
- JSON evidence package
- DOT graph
- BinExport-like file later
- selector/call graph export
- patch manifest export

Inputs:

- stable query APIs
- privacy profile
- output path

Outputs:

- report artifact
- export manifest
- redaction log

Rules:

- exporter cannot bypass privacy controls
- exporter cannot mutate project facts unless explicitly declared
- large exports are jobs

### Workbench Command Extension

Purpose:

- add commands that operate on the current project and route

Examples:

- export selected function summary
- annotate stack strings
- apply naming convention
- open external editor with decompiled summary later

Rules:

- command declares required route kinds
- command declares capabilities
- command appears in command palette only when applicable
- command output is a transaction, job, route, report, or problem
- command cannot directly mutate Swift state

## Plugin Manifest

```json
{
  "id": "com.example.damsel.stack-strings",
  "name": "Stack String Annotator",
  "version": "0.1.0",
  "sdk_version": "0.1",
  "entrypoint": {
    "kind": "process",
    "command": "stack-strings-plugin"
  },
  "classes": ["analyzer", "workbench_command"],
  "capabilities": [
    "procedure.read",
    "source.read_bytes",
    "annotation.write"
  ],
  "produces": ["annotation.comment", "problem"],
  "privacy": {
    "requires_raw_bytes": true,
    "network": false
  }
}
```

Required manifest fields:

- id
- name
- version
- sdk version
- extension classes
- entrypoint
- capabilities
- produced fact/annotation kinds
- privacy requirements
- minimum Damsel schema version

Optional fields:

- supported architectures
- supported source kinds
- supported platforms
- command palette metadata
- configuration schema
- resource limits
- signature identity

## Process Boundary

Recommended first external-plugin model:

```text
Damsel Project API <-> Plugin Runner Process <-> Plugin
```

Why:

- avoids loading untrusted code into Workbench
- permits timeout/cancellation
- permits resource limits
- keeps crashes isolated
- works with multiple languages
- makes schema versioning explicit

Plugin runner responsibilities:

- validate manifest
- grant capabilities
- stream inputs
- collect outputs
- enforce limits
- normalize errors
- record provenance

Plugin non-responsibilities:

- no direct DB handles
- no Workbench UI pointers
- no raw project file mutation
- no implicit network access

## Internal Traits Before Public SDK

Suggested internal Rust traits:

```text
SourceLoader
SidecarImporter
ArchitectureSemanticsProvider
AnalysisPass
ReportExporter
WorkbenchCommandProvider
```

Each trait should expose:

- id
- version
- input contract
- output contract
- dependencies
- capabilities
- cancellation token
- metrics sink
- provenance id

This lets first-party Damsel subsystems use plugin-like boundaries before
Damsel commits to third-party extension ABI.

## Versioning

Version every layer:

- plugin manifest version
- public SDK version
- project schema version
- fact schema version
- tool schema version
- analyzer version
- architecture semantics version

Compatibility rules:

- incompatible plugin refuses to load with a typed problem
- compatible plugin can run with feature flags
- analyzer version changes mark old facts stale
- plugin output records plugin id and version
- project reopen does not require missing optional plugins unless their facts
  are needed for a view

## Provenance Model

Every plugin-produced object includes:

- plugin id
- plugin version
- analyzer/pass id
- input source ids
- input fact ids or result versions
- timestamp
- confidence
- capability profile

User-visible origin labels:

- built-in analyzer
- imported sidecar
- external plugin
- script
- MCP tool
- user

Rule:

- plugin facts are never presented as core parser truth unless their provenance
  says so.

## Capability And Permission Model

Capability classes:

- source read metadata
- source read bytes
- fact read
- graph read
- runtime object read
- annotation write
- fact write
- problem write
- job control
- report export
- patch preview
- patch apply
- debug read
- debug control
- network access
- filesystem read
- filesystem write

Defaults:

- built-in first-party analyzers get fact write within their namespace
- external analyzers get fact write only for declared kinds
- external tools get no annotation mutation unless granted
- network access denied by default
- patch/debug denied by default

## Workbench Plugin UX

Surfaces:

- plugin manager
- installed plugins
- plugin status
- capability grants
- per-project plugin enablement
- plugin activity log
- plugin problems
- plugin updates later

Workbench should show:

- what a plugin can read
- what a plugin can write
- whether it can access raw bytes
- whether it can access network
- whether it can start jobs
- whether it can mutate annotations
- whether it can affect exports

No plugin should silently add a UI surface. Commands appear through the command
registry with route applicability and capability requirements.

## CLI Contracts

Commands later:

```text
damsel plugin list
damsel plugin inspect <plugin-id>
damsel plugin validate <manifest>
damsel plugin enable <project> <plugin-id>
damsel plugin disable <project> <plugin-id>
damsel project analyze --plugin <plugin-id>
damsel project export --plugin <plugin-id>
```

Rules:

- JSON first
- no plugin auto-enable by default
- validation does not execute plugin code
- execution emits job id and provenance

## Integration With Existing Damsel

Current good anchors:

- `damsel-core` already centralizes typed data contracts
- `damsel-macho` already has loader/analyzer boundaries in practice
- `damsel-cli` already proves JSON contracts through snapshot tests
- Workbench already delegates analysis to CLI/server instead of embedding
  parsers in Swift

Required future refactors:

- move source identity into project registry
- make analyzers produce facts/problems instead of view-only structs
- add analyzer run metadata
- add cancellation to long-running passes
- add capability checks to mutation APIs
- make report/export paths jobs
- add provenance ids to built-in analyzers before external plugins exist

## Security Model

Threats:

- plugin reads private binaries or strings
- plugin exfiltrates data over network
- plugin corrupts project database
- plugin hangs analysis scheduler
- plugin returns invalid facts
- plugin crashes Workbench
- plugin produces misleading report

Controls:

- helper process boundary
- capability grants
- network denied by default
- bounded byte windows
- schema validation
- output validation
- cancellation and timeouts
- resource limits
- provenance display
- plugin-specific problem records
- no direct database handles

## Validation Plan

### Manifest Gate

- valid manifest snapshot
- missing required field
- unsupported SDK version
- unknown capability
- unsupported source kind
- invalid id format

### Loader Gate

- detects synthetic container
- emits source records
- emits address spaces
- reports progress
- handles cancellation
- rejects malformed input with problem

### Sidecar Gate

- matches dSYM/KDK-style UUID fixture
- rejects mismatch
- imports names/types with provenance
- preserves user labels

### Analyzer Gate

- declares dependencies
- emits facts/problems
- deterministic rerun
- cancellation
- stale analyzer version marks old facts

### Exporter Gate

- report export snapshot
- privacy redaction
- output path failure
- large export job state

### Workbench Command Gate

- command appears only for supported route
- command declares capabilities
- mutation creates transaction
- failure creates problem

### Isolation Gate

- plugin crash does not crash Workbench
- timeout cancels plugin
- invalid output rejected
- network-denied plugin cannot access network in supported sandbox

## Milestones

### S0: Internal Boundary Cleanup

- define first-party loader/analyzer/exporter traits
- add provenance ids to built-in passes
- add analyzer version metadata

### S1: Manifest Schema

- plugin manifest model
- validation CLI
- capability vocabulary
- schema snapshots

### S2: First-Party Analyzer As Plugin-Like Pass

- wrap one built-in analyzer with the manifest/dependency/output shape
- prove facts/problems/metrics/cancellation

### S3: Exporter Boundary

- report/export plugin shape
- privacy profile
- export job

### S4: External Helper Prototype

- helper process runner
- JSON protocol
- timeout/cancellation
- toy analyzer fixture

### S5: Workbench Plugin Manager Skeleton

- installed plugin list
- inspect capabilities
- enable/disable per project
- activity/problems

### S6: Public SDK Draft

- document stable contracts
- sample external analyzer
- sample exporter
- no raw DB handles

## Risks

### Risk: Public ABI Freezes Bad Internals

Mitigation:

- internal traits first
- JSON/process boundary before dynamic library ABI
- version schemas aggressively

### Risk: Plugins Bypass Provenance

Mitigation:

- provenance required in accepted outputs
- output validator injects plugin id/version
- UI shows origin

### Risk: Plugins Become Security Liability

Mitigation:

- no auto-enable
- no network by default
- helper process boundary
- capability prompts

### Risk: Extension Ecosystem Distracts From Core Apple Niche

Mitigation:

- first-party Apple analyzers remain priority
- plugins extend source import, reports, and niche workflows
- generic CPU/file breadth stays optional

## Completion Criteria

The extension boundary is implementation-ready when:

- Damsel has internal loader/analyzer/exporter traits
- built-in analyzers record id/version/provenance
- plugin manifests validate without executing code
- capabilities are shared with automation/MCP
- external helper process can run a toy analyzer without DB handles
- plugin output is schema-validated and cancellable
- Workbench can show plugin capabilities and activity
- unsupported plugins fail with typed problems

The feature family is not product-complete until external plugins can safely
add at least one analyzer and one exporter to a project without crashing
Workbench, corrupting facts, bypassing permissions, or hiding provenance.

