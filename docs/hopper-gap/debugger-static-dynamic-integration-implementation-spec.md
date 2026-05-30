# Debugger Static-Dynamic Integration Implementation Spec

Status: implementation architecture plan. This is not an implementation claim
and it does not move Damsel's debugger from future work into current scope.

Purpose: define how Damsel should eventually bridge static project analysis to
live LLDB/GDB-style debugging in a Hopper-competitive way, while preserving
Damsel's Apple-system niche, read-only defaults, project integrity, and native
Workbench polish.

The goal is not to build a debugger before the static workbench is excellent.
The goal is to document the architecture early enough that source identity,
address spaces, route objects, symbolication profiles, patch revisions,
procedure records, and Workbench selection state can support a debugger later.

## Research Companions

- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)

## Architecture Companions

- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [API And Data Contracts](api-data-contracts.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Type And Symbolication System Plan](type-symbolication-system-plan.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Function Recovery And Procedure Database Plan](function-recovery-and-procedure-database-plan.md)
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Remote Device Debugging And Target Environment Implementation Spec](remote-device-debugging-target-environment-implementation-spec.md)

## Public Hopper Evidence

Official homepage evidence:

- Hopper markets dynamic analysis with LLDB and GDB support.
- Hopper says users can debug Mac applications while analyzing their structure
  in real time.

Official tutorial evidence:

- The assembly margin supports breakpoints.
- The same assembly surface also supports bookmarks, jump arrows, comments,
  labels, and procedure navigation.

Official debugger server evidence:

- Hopper Debugger Server allows remote debugging.
- Hopper's page now describes the server as legacy unless remote debugging is
  needed.
- The page tells users to use the Local Debugger when debugging on the same
  computer.
- The debugger server requires Xcode on the target machine.

Official changelog evidence:

- Hopper has fixed GDB remote target debugging issues.
- Hopper has improved hardened-runtime detection on attach.
- Hopper has fixed patch application before launching the debugger.
- Hopper has fixed debugger-panel issues after prior sessions.
- Hopper added debugger environment-variable support.
- Hopper added a debugger hex-column mode when PC/RIP is outside the main
  binary.
- Hopper highlights the conditional path that will be taken when the program
  counter reaches a conditional jump.
- Hopper has had debugger crashes and fixes over multiple releases.

Implication:

- A Hopper-like debugger is not an isolated process pane. It overlays live
  state onto the same assembly, graph, hex, procedure, symbol, and route model
  used by static analysis.

## Core Thesis

Debugging only becomes useful in Damsel when static and dynamic identity agree.

The hard problem is not "launch LLDB." The hard problem is:

- matching a live module to a Damsel source
- mapping runtime addresses to project routes
- mapping project routes to runtime breakpoint locations
- handling dyld shared cache images and slides
- handling dSYM/KDK/debug info identity
- handling patched source revisions
- keeping Workbench views responsive while the target stops and resumes
- ensuring debugger actions are explicit and permissioned

Without that mapping, Damsel would produce a generic debugger pane that does
not strengthen the reverse-engineering workflow.

## Non-Goals For Early Debugger Work

Do not start with:

- kernel debugging
- iOS device debugging
- remote debug server
- process memory writes
- live binary patching
- automatic attach to system processes
- debugger-driven mutation of analysis facts without provenance
- stepping UI before route mapping is correct
- expression evaluation
- time-travel debugging
- anti-debug bypass features

Initial debugger work should be local, explicit, read-mostly, and route-backed.

## Prerequisites

Required Damsel foundations:

- `AnalysisProject`
- `AnalysisSource`
- `SourceRevision`
- `AddressSpaceGraph`
- `Route`
- `ProcedureRecord`
- `SymbolicationProfile`
- `ProblemRecord`
- `JobRecord`
- stable project API
- Workbench route/navigation store
- semantic assembly rows
- synchronized hex view
- transaction and audit model

Required platform foundations:

- local LLDB availability
- Xcode command-line tools or equivalent LLDB tooling
- entitlement/SIP error detection
- code-signing and hardened-runtime diagnostics

## Product Requirements

### Launch Local Executable

User story:

- As an analyst, I can launch a local executable under the debugger from a
  Damsel project and see the stopped instruction routed into the static
  assembly view.

Required behavior:

- explicit launch command
- visible target path, arguments, environment, and working directory
- launch configuration saved in project state only with user consent
- module list after launch
- runtime slide computation
- current PC mapped to a Damsel route when possible
- clear problem when the live binary does not match the project source

### Attach To Local Process

User story:

- As an analyst, I can attach to a local process and map its images to Damsel
  sources when UUID/path/cache identity matches.

Required behavior:

- process picker
- no background attach
- no system process default selection
- hardened-runtime/SIP failure explanation
- module UUID/path display
- dyld shared cache UUID display
- mismatch problems for missing sources or changed binaries

### Breakpoint Mapping

User story:

- As an analyst, I can set a breakpoint on a Damsel route and have it map to
  the correct runtime address for the live process.

Required behavior:

- breakpoint stores route plus runtime resolution
- breakpoint survives relaunch if module identity still matches
- unresolved breakpoints are visible
- route mismatch creates a problem, not silent failure
- conditional breakpoint support is later, not first wave

### Live Current-PC Overlay

User story:

- As an analyst, I can stop at a breakpoint or step instruction and see current
  PC state in assembly, graph, hex, and inspector.

Required behavior:

- current instruction marker
- runtime/static address toggle
- current basic block highlight
- conditional branch taken-path overlay later
- hex window at PC when source bytes are mapped
- explicit "PC outside known source" state

### Register, Stack, And Backtrace Views

User story:

- As an analyst, I can inspect registers, stack frames, and backtrace, and jump
  from frames to static routes when possible.

Required behavior:

- register table
- selected frame state
- frame module/source matching
- frame address to procedure route
- stack memory window
- symbolicated frame name with provenance
- unresolved frame problem entries

### Memory Read View

User story:

- As an analyst, I can read live memory around a runtime address and compare it
  with static/project bytes when a mapping exists.

Required behavior:

- read-only in first debugger release
- runtime memory window
- static byte comparison
- protection/error reporting
- no process memory writes

## Data Model

### Debug Session

```text
DebugSession {
  id
  project_id
  kind
  target
  launch_config?
  attach_info?
  state
  transport
  created_at
  actor
  permissions
}
```

Kinds:

- local_launch
- local_attach
- remote_later

States:

- configured
- launching
- attaching
- stopped
- running
- exited
- detached
- failed

### Debug Target

```text
DebugTarget {
  executable_path?
  process_id?
  process_name?
  arguments[]
  environment[]
  working_directory?
  platform
  architecture
}
```

Privacy rule:

- environment variables can contain secrets and must be redacted in logs unless
  the user explicitly opens details.

### Runtime Module

```text
RuntimeModule {
  id
  debug_session_id
  name
  path?
  uuid?
  architecture
  load_address
  slide
  size?
  source_match?
  address_space_id?
  source_revision_id?
  dyld_cache_uuid?
}
```

Match statuses:

- exact_uuid
- exact_hash
- path_only
- dyld_cache_member
- patched_revision
- ambiguous
- missing_source
- mismatch

### Runtime Address

```text
RuntimeAddress {
  session_id
  module_id?
  value
  slide?
  mapped_project_address?
  route?
}
```

### Breakpoint Record

```text
BreakpointRecord {
  id
  project_id
  session_id?
  route
  project_address
  runtime_locations[]
  state
  enabled
  hit_count
  condition?
  commands?
  provenance
}
```

Initial state values:

- pending
- resolved
- hit
- disabled
- unresolved
- failed

Initial policy:

- no commands and no conditions until basic route mapping is stable.

### Stop Event

```text
DebugStopEvent {
  id
  session_id
  reason
  thread_id
  frame_id
  pc
  route?
  registers_summary
  timestamp
}
```

Reasons:

- breakpoint
- step
- signal
- exception
- process_start
- process_exit
- manual_pause

### Frame Record

```text
DebugFrame {
  id
  session_id
  thread_id
  index
  runtime_address
  module_id?
  symbol_name?
  procedure_id?
  route?
  confidence
  problems[]
}
```

## Static-Dynamic Mapping

### Runtime To Project

Inputs:

- runtime address
- loaded modules
- module UUID
- module path
- module load address
- source registry
- source revision hashes
- dyld shared cache UUID
- image vmaddr mapping
- project address spaces

Output:

- project route candidate set
- confidence
- mismatch problems

Algorithm:

1. Find runtime module containing address.
2. Match module to source revision by UUID/hash where possible.
3. Compute unslid image address.
4. Convert through source address-space graph.
5. Resolve to route: instruction, byte range, procedure, symbol, runtime
   object, or source.
6. If multiple candidates exist, return all with ambiguity problem.
7. If no candidate exists, return runtime-only route.

### Project To Runtime

Inputs:

- route
- project address
- source id
- active source revision
- loaded modules
- slide data

Output:

- zero or more runtime breakpoint locations

Algorithm:

1. Resolve route to project source/address.
2. Find matching runtime module by source revision UUID/hash/cache identity.
3. Apply slide.
4. Validate that runtime module contains computed address.
5. Return breakpoint location candidates.

Failure cases:

- source not loaded
- source revision differs from launch target
- source is projected from dyld cache but process uses a different cache UUID
- address maps to virtual bytes only
- optimized/inlined debug line maps to multiple runtime locations

## LLDB Transport Design

### Transport Options

Options:

- shelling out to `lldb` command-line
- LLDB Python bridge
- LLDB C++ API through a helper
- Debug Adapter Protocol, if suitable for local target workflows
- custom gdb-remote client, later

Recommendation:

- start with a helper process boundary rather than embedding arbitrary debugger
  state into the Workbench process
- keep Damsel's Rust project API separate from LLDB transport quirks
- model debugger output as typed events

Helper responsibilities:

- launch/attach
- list modules
- set/remove breakpoints
- continue/pause/step
- read registers
- read memory
- return backtrace
- emit typed stop/exit/error events

Helper non-responsibilities:

- no analysis database writes
- no source parsing
- no Workbench UI decisions
- no process memory writes in first version

## Workbench UX

### Debug Session Panel

Panel sections:

- target
- session state
- controls: launch, attach, pause, continue, step over, step into, step out,
  detach, stop
- modules
- threads
- frames
- breakpoints
- registers
- live memory
- problems

Rules:

- controls disabled until capability/session state supports them
- launch/attach require explicit user action
- failed platform operations produce problem records
- target identity remains visible while session is active

### Assembly Integration

Assembly overlays:

- breakpoint gutter marker
- current PC marker
- current frame marker
- runtime address column toggle
- static/project address column remains available
- live byte mismatch badge when runtime memory differs from static bytes

Rules:

- setting breakpoint in gutter creates route-backed breakpoint
- clicking current PC routes to static instruction when mapped
- unresolved breakpoint stays in breakpoint list with reason

### Graph Integration

Graph overlays:

- current basic block
- breakpoint blocks
- current branch candidate later
- taken conditional path later

Rules:

- graph never becomes the debugger source of truth
- graph overlays are projections of debug session route state

### Hex Integration

Hex/debug behavior:

- static hex view shows project/source bytes
- live memory view shows process bytes
- comparison mode highlights differences
- PC outside known source opens runtime memory, not fake static bytes

### Privacy And Safety UX

Prompts must explain:

- launching a process
- attaching to a process
- reading process memory
- environment variables visible to debugger
- hardened-runtime/SIP failures
- patched artifact launch

No debugger action should look like ordinary static navigation.

## API Contracts

### Configure Launch

```text
POST /api/projects/{project_id}/debug/launch-configs
```

Request:

```json
{
  "executable_source_id": "source:app",
  "source_revision_id": "source-rev:original",
  "arguments": [],
  "environment": [],
  "working_directory": null
}
```

Response:

```json
{
  "launch_config_id": "debug-launch:1",
  "problems": [],
  "warnings": []
}
```

### Start Debug Session

```text
POST /api/projects/{project_id}/debug/sessions
```

Request:

```json
{
  "kind": "local_launch",
  "launch_config_id": "debug-launch:1"
}
```

Response:

```json
{
  "session_id": "debug-session:1",
  "state": "launching",
  "job_id": "job:debug-launch:1"
}
```

### List Modules

```text
GET /api/projects/{project_id}/debug/sessions/{session_id}/modules
```

Response includes:

- runtime modules
- source matches
- slide values
- mismatch problems

### Resolve Runtime Address

```text
POST /api/projects/{project_id}/debug/sessions/{session_id}/resolve-runtime-address
```

Request:

```json
{
  "address": "0x000000010003f20"
}
```

Response:

```json
{
  "runtime_address": {},
  "route_candidates": [],
  "confidence": "strong",
  "problems": []
}
```

### Set Breakpoint

```text
POST /api/projects/{project_id}/debug/sessions/{session_id}/breakpoints
```

Request:

```json
{
  "route": {
    "kind": "instruction",
    "id": "instruction:source:1:100003f20"
  }
}
```

Response:

```json
{
  "breakpoint": {},
  "runtime_locations": [],
  "problems": []
}
```

### Read Registers

```text
GET /api/projects/{project_id}/debug/sessions/{session_id}/threads/{thread_id}/frames/{frame_id}/registers
```

### Read Memory

```text
POST /api/projects/{project_id}/debug/sessions/{session_id}/memory/read
```

Request:

```json
{
  "runtime_address": "0x000000010003f20",
  "length": 256
}
```

Response includes:

- runtime bytes
- mapped project route, if any
- static byte comparison, if available
- permission/protection errors

## CLI Contracts

Later commands:

```text
damsel debug launch <project> --config <launch-config> --json
damsel debug attach <project> --pid <pid> --json
damsel debug modules <project> <session-id> --json
damsel debug resolve <project> <session-id> --runtime-address <addr> --json
damsel debug breakpoint set <project> <session-id> --route <route> --json
damsel debug registers <project> <session-id> --thread <id> --frame <id> --json
damsel debug memory <project> <session-id> --runtime-address <addr> --len <n> --json
```

CLI rules:

- never attach without explicit pid or process selection
- JSON includes source match and confidence
- raw memory output is bounded by default
- environment values are redacted in logs

## Security Model

Capabilities:

- debug_launch
- debug_attach
- debug_control
- read_process_registers
- read_process_memory
- set_breakpoint
- write_process_memory_later
- debug_remote_later

Default profile:

- no debugger capability

Local debug profile:

- debug_launch
- debug_attach with confirmation
- debug_control
- read registers
- read memory with bounded windows
- set breakpoint

Disallowed initially:

- write process memory
- remote debug
- attach to protected/system processes by default
- debug child processes without explicit opt-in

Audit records:

- launch
- attach
- detach
- stop target
- set/remove breakpoint
- read memory
- failed permission operation

## Platform And Apple-Specific Issues

### Hardened Runtime And SIP

Damsel must report:

- attach denied by platform policy
- target is hardened
- missing entitlement
- Xcode tools missing
- debugger helper unavailable
- permissions changed during session

Do not present these as Damsel analysis failures.

### Dyld Shared Cache

Debugger mapping must handle:

- live process uses dyld shared cache mapping
- project may have a projected cache image
- cache UUID mismatch
- extracted image path may not equal live module path
- symbols sidecar may exist locally or not

Mapping should prefer UUID/cache identity over path.

### Patched Artifacts

If Damsel later launches a patched derived artifact:

- session records source revision id
- static route mapping uses patched revision
- mismatches with original source are expected and must not be reported as
  corruption
- code-signing/hardened-runtime warnings remain visible

### Kernel And DriverKit

No early support for:

- kernel debugging
- live kernel memory
- DriverKit process attach automation

Static kernel/KDK/panic analysis remains separate and read-only.

## Validation Plan

### Static-Dynamic Mapping Gate

Fixtures:

- signed local hello executable
- stripped executable
- executable with dSYM
- executable launched from a patched derived revision later
- dyld-cache imported function frame, if feasible

Evidence:

- module UUID matches project source
- runtime address maps to project route
- project route maps to breakpoint runtime address
- mismatch creates problem
- source-not-loaded creates problem

### Launch Gate

Evidence:

- local executable launches under helper
- initial stop event received
- modules listed
- current PC route resolved or runtime-only route returned
- process exit recorded

### Attach Gate

Evidence:

- attach to allowed local test process
- denied attach produces platform problem
- detach leaves project static state intact

### Breakpoint Gate

Evidence:

- route-backed breakpoint resolves
- breakpoint hit creates stop event
- current PC overlays assembly row
- unresolved breakpoint remains visible

### Registers And Backtrace Gate

Evidence:

- register read returns typed values
- frame list resolves at least one route
- unresolved frames carry confidence/problem metadata

### Memory Gate

Evidence:

- bounded memory read succeeds
- unreadable memory reports protection problem
- static comparison appears when mapping exists
- no memory write endpoint exists in first debugger release

### UX Gate

Evidence:

- Workbench remains responsive during run/stop events
- controls reflect session state
- debug errors go to problems/log panel
- route navigation and debugger overlays do not fight each other

## Milestones

### D0: Design And Model Foundation

- debug session model
- runtime module model
- runtime address model
- breakpoint model
- stop event model
- API shape

### D1: Static-Dynamic Mapping Prototype

- map synthetic loaded module to source
- map runtime address to route
- map route to runtime address
- mismatch problem records

### D2: Local Helper Spike

- helper process boundary
- launch local executable
- list modules
- stop event
- exit event

### D3: Breakpoint Overlay

- set breakpoint by route
- resolve to runtime location
- hit breakpoint
- assembly gutter overlay

### D4: Registers, Frames, And Memory

- register table
- backtrace route mapping
- read-only memory window
- static/live byte comparison

### D5: Workbench Debug UX

- session panel
- module list
- threads/frames
- breakpoints
- problems
- route synchronization

### D6: Patched Artifact Launch

- launch a copy-on-write patched artifact
- use source revision identity for route mapping
- keep code-signing warnings visible

### D7: Remote Debugging Research

- evaluate remote LLDB/debugserver workflows
- do not implement until local workflow is reliable

## Risk Register

### Risk: Debugger Becomes A Parallel App

Mitigation:

- all debug state maps through project routes
- no separate debugger-only symbols or addresses
- frames and breakpoints resolve into project objects

### Risk: Platform Failures Look Like Damsel Bugs

Mitigation:

- explicit problem codes for SIP, hardened runtime, missing Xcode tools,
  entitlement, permission, and target mismatch failures

### Risk: Attach Surprises User

Mitigation:

- no background attach
- explicit confirmation
- visible target identity
- audit log

### Risk: Runtime Mapping Is Wrong

Mitigation:

- prefer UUID/hash/cache identity over path
- include confidence and alternatives
- show mismatch problems
- fail closed for breakpoint setting when ambiguous

### Risk: UI Freezes On Debug Events

Mitigation:

- helper process boundary
- event stream into job/session store
- UI projections read snapshots
- bounded memory/register/backtrace payloads

### Risk: Debugger Encourages Unsafe Patching

Mitigation:

- no process memory writes first
- patched artifact launch only after patch model exists
- separate capability for process writes later

## Completion Criteria

This feature family is implementation-ready when:

- runtime modules can be matched to project sources with confidence
- runtime addresses and project routes are bidirectionally mapped
- breakpoints are route-backed objects
- debug sessions are explicit, permissioned, and audited
- Workbench surfaces can overlay PC and breakpoint state without owning the
  debugger model
- platform failures are represented as typed problems
- memory reads are bounded and read-only
- patched source revisions can be represented even before debugger launch

The first product-complete debugger slice requires a local executable launch,
module matching, route-backed breakpoint, PC overlay, registers, backtrace,
read-only memory, and understandable failure paths. Remote debugging, process
memory writes, kernel debugging, and iOS-device debugging remain separate
future scopes.
