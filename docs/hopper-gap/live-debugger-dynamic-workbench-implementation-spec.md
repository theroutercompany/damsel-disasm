# Live Debugger, Dynamic Analysis, And Runtime Workbench Implementation Spec

Status: implementation-grade planning spec. This is not an implementation
claim and it does not move live debugging into current product scope.

Purpose: define the native debugger and dynamic-analysis Workbench that Damsel
would need to compete credibly with Hopper while staying truthful, Apple-system
focused, permission-aware, and responsive. The existing static-dynamic
integration spec defines route/address mapping. This spec defines the larger
product and architecture surface: launch, attach, target identity, module
matching, breakpoints, stop events, stepping, registers, frames, memory,
watch-like observation, iOS/macOS constraints, LLDB/debugserver boundaries,
entitlements, diagnostics, persistence, command integration, tests, and rollout.

## Why This Spec Exists

Hopper's public materials do not present debugging as a separate terminal. The
homepage markets dynamic analysis with LLDB and GDB support. The tutorial shows
breakpoints in the same assembly margin that also supports labels, comments,
bookmarks, jump arrows, and procedure navigation. The Debugger Server page
states that the remote server exists for remote debugging, that local debugging
should use Hopper's local debugger, and that Xcode is required on the target
machine. Public changelog notes recorded elsewhere in this pack show debugger
work around attach dialogs, hardened-runtime detection, environment variables,
patches before launching, PC/RIP outside the main binary, conditional-path
highlighting, GDB remote target handling, debugger panels, and crash fixes.

That evidence points to a product requirement: a competitive debugger is not
"embed LLDB in a pane." It is live runtime state overlaid onto the same project,
source, address, byte, graph, procedure, symbolication, pseudocode, problem,
command, and trust systems as static analysis.

Damsel should only build this after the static Workbench foundations are strong,
but the architecture must be specified now because many earlier systems need to
leave room for it:

- project/source identity
- address-space graph
- source revisions and patches
- route model
- function/procedure records
- graph/source-map records
- dSYM/KDK symbolication profiles
- dyld shared-cache projection
- command registry
- undo/activity/audit model
- local helper trust
- diagnostics redaction
- app entitlements and distribution policy

If those systems ignore debugger needs, retrofitting live debugging later will
be painful and unsafe.

## Companion Documents

Read this spec with:

- [Debugger Static-Dynamic Integration Implementation Spec](debugger-static-dynamic-integration-implementation-spec.md)
- [Debugger, Automation, Extension, And MCP Plan](debugger-automation-extension-plan.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Decompiler, Pseudocode, And Native Workbench Representation Implementation Spec](decompiler-pseudocode-workbench-implementation-spec.md)
- [DWARF, dSYM, KDK, And Source-Level Debug-Info Implementation Spec](dwarf-dsym-kdk-debug-info-implementation-spec.md)
- [Apple Artifact Intake, Acquisition, And Source Registration Implementation Spec](apple-artifact-intake-source-registration-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Performance And Responsiveness Architecture](performance-responsiveness-architecture.md)
- [API And Data Contracts](api-data-contracts.md)
- [Quality And Validation Plan](quality-validation-plan.md)

## Source-Grounded Requirements

### Hopper Evidence

The public Hopper evidence implies:

- debugger state should share static routes and representation panes
- local debugging is the primary workflow on the same Mac
- remote debugging exists but should be a separate transport path
- breakpoint UI belongs in the assembly margin and later graph/pseudocode
  gutters
- dynamic state should map into the document instead of replacing it
- hardened-runtime and attach failures need clear user-facing explanations
- debugger environment variables, launch settings, and attach dialogs are
  product features
- PC outside the main binary must still be inspectable, often through a hex or
  module-centered view
- patched or revised sources affect launch/debugger semantics
- conditional branch path highlighting is a meaningful debugger overlay

### Apple Platform Evidence

Apple platform constraints imply:

- debugging a process may require developer tooling and entitlements
- hardened runtime, SIP, sandboxing, code signing, and task-for-pid restrictions
  can make attach fail even when the user has the binary
- app-store-style sandbox assumptions are not enough for a reverse-engineering
  workstation
- local helper boundaries should be explicit and authenticated
- iOS device debugging is not the same product as local macOS debugging
- kernel debugging is not the same product as user-space debugging
- KDK/dSYM/source privacy must be handled before debugger exports and
  diagnostics

### Damsel Niche

Damsel's debugger should specialize in:

- macOS and iOS user-space binaries
- app bundles, command-line tools, frameworks, extensions, and helper processes
- dyld shared-cache module matching
- ObjC message-send and selector evidence at stop points
- C++/ObjC++ virtual-call and RTTI context at stop points
- arm64e/PAC-aware indirect branch/call explanations
- KDK/dSYM-backed source and symbolication overlays
- panic-log-to-static context as a neighbor workflow, not live kernel
  debugging first
- evidence-rich stop reports and reproducible debug session records

## Product Thesis

The debugger is a runtime overlay on an analysis project.

Every live object should resolve, when possible, to project objects:

- process -> debug target record
- loaded module -> `AnalysisSource` or cache image
- runtime address -> route
- breakpoint -> route plus runtime resolution
- stop event -> route, function, block, source line, pseudocode line
- frame -> module, function, inline callsite, symbolication profile
- register value -> typed address, object, selector, class, ivar, vtable, or
  unknown
- memory window -> runtime address range plus static source relation when safe
- thread -> call stack plus current route

The Workbench should always preserve the difference between:

- static evidence
- live runtime observation
- user assertion
- debugger transport output
- inferred relationship

Runtime observations can enrich analysis, but they should not silently mutate
static facts.

## Non-Goals

Do not include in the first live-debugger product:

- kernel debugging
- iOS device debugging
- remote debug server
- automatic attach to system processes
- anti-debug bypasses
- malware-style stealth debugging
- arbitrary expression evaluation against target memory
- target memory writes
- live patch application
- reverse execution or time-travel debugging
- whole-process tracing
- watchpoints as a first-wave feature
- debugger-driven decompiler rewriting
- automatic persistence of sensitive launch arguments or environment variables

The first debugger should be local, explicit, user-initiated, read-mostly,
route-backed, and safely diagnosable.

## Workbench UX

### Debugger Modes

The Workbench should expose debugger state through modes:

- Not Configured
- Configured
- Launching
- Attaching
- Running
- Stopped
- Stepping
- Exited
- Detached
- Failed

Each mode should have clear allowed commands. The UI should never show a
"Step" button as enabled when there is no stopped target, and should never show
"Attach" as if it is a harmless static-analysis operation.

### Debugger Bar

Add a compact debugger bar when a project has a debug configuration or active
session:

- target name
- process id when attached/launched
- state
- selected thread
- current module
- current route status
- Run/Continue
- Pause
- Step Over
- Step Into
- Step Out
- Stop
- Detach
- Breakpoints
- Session Problems

Keep this bar quiet when debugging is not configured. Static analysis should
not feel like a debugger app by default.

### Launch Configuration Sheet

The launch sheet should include:

- executable path
- app bundle path when relevant
- arguments
- environment variables
- working directory
- inherited environment policy
- architecture slice
- debugger backend
- stop at entry option
- stop on Objective-C exceptions option, later
- dSYM/KDK/source profile
- source revision/patch revision
- privacy reminder for saved arguments and environment

Defaults should be safe:

- no environment variables saved unless user chooses
- no inherited secrets shown in logs by default
- stop at entry available but not required
- launch target must match a project source or be explicitly marked external

### Attach Sheet

The attach sheet should include:

- process list
- bundle id/process name/path/pid/user
- platform and architecture
- whether process appears to match a project source
- hardened runtime/SIP likelihood where detectable
- signing/team id hints where available
- warning for privileged/system processes
- attach capability status
- search/filter

No automatic attach. No default system process selection. Attaching is a
privileged, potentially surprising action.

### Stopped State Layout

When the target stops:

- assembly centers on current PC if mapped
- graph highlights current basic block if mapped
- pseudocode highlights source-mapped line if available
- hex can show current runtime memory window
- inspector switches to stop context
- thread list shows stop reason
- backtrace shows frames
- registers show current thread state
- problems show unresolved module/route/symbolication issues

If the PC is outside known static sources, Damsel should show a module/runtime
address view instead of pretending the route exists.

### Debugger Inspector

The inspector should have sections:

- Stop Summary
- Current Route
- Source Mapping
- Module Identity
- Function/Procedure
- Register Values
- Stack Frame
- Runtime Memory
- ObjC Runtime Context
- C++ Runtime Context
- PAC/Arm64e Context
- Debug Info
- Breakpoint Context
- Problems
- Transport Log

The inspector is where runtime observation becomes useful evidence.

### Breakpoint Gutter

Breakpoint controls should appear in:

- assembly gutter
- graph node header, later
- pseudocode gutter, after source maps are stable
- source-line gutter, after debug-source routing is stable

Breakpoint states:

- enabled resolved
- disabled resolved
- pending unresolved
- failed
- hit
- conditional, later
- one-shot, later

Tooltips should explain unresolved and failed states with the route, module,
slide, and backend error.

### Current-PC Overlay

The PC overlay should show:

- current instruction marker
- current graph node
- current pseudocode/source line if mapped
- runtime address
- static address
- slide/base relation
- module name
- thread id
- stop reason

When stopped at a conditional branch, Damsel should eventually highlight the
path that will be taken, but only when register/flag interpretation is
trustworthy.

## Data Model

### DebugTarget

Fields:

- `id`
- `project_id`
- `name`
- `kind`
- `source_id`
- `bundle_id`
- `executable_path_grant_id`
- `default_arguments_secret_ref`
- `default_environment_secret_ref`
- `working_directory_grant_id`
- `architecture`
- `backend`
- `created_at`
- `updated_at`

Kinds:

- `macos_executable`
- `macos_app_bundle`
- `macos_helper`
- `ios_app_remote_future`
- `remote_gdb_future`
- `kernel_future`

### DebugLaunchConfiguration

Fields:

- `id`
- `target_id`
- `arguments`
- `environment`
- `working_directory`
- `inherit_environment`
- `stop_at_entry`
- `architecture`
- `source_revision_id`
- `patch_revision_id`
- `symbolication_profile_id`
- `privacy_policy`

Environment and arguments may contain secrets. They should use redaction-aware
storage and diagnostics.

### DebugSession

Fields:

- `id`
- `project_id`
- `target_id`
- `backend`
- `state`
- `pid`
- `process_name`
- `platform`
- `architecture`
- `started_at`
- `ended_at`
- `launch_configuration_id`
- `module_snapshot_id`
- `selected_thread_id`
- `selected_frame_id`
- `stop_event_id`
- `problems`
- `capabilities`

### RuntimeModule

Fields:

- `id`
- `session_id`
- `load_address`
- `slide`
- `size`
- `path`
- `install_name`
- `uuid`
- `architecture`
- `source_match`
- `cache_image_match`
- `symbolication_profile_match`
- `is_main_executable`
- `is_shared_cache_image`
- `is_system_library`
- `is_external`

### RuntimeAddressResolution

Fields:

- `runtime_address`
- `module_id`
- `static_address`
- `route`
- `source_id`
- `function_id`
- `basic_block_id`
- `instruction_id`
- `pseudocode_line_id`
- `source_line_id`
- `confidence`
- `problems`

This object is the shared contract between debugger, graph, assembly, hex,
pseudocode, source, and automation.

### DebugBreakpoint

Fields:

- `id`
- `project_id`
- `route`
- `enabled`
- `kind`
- `scope`
- `condition`
- `ignore_count`
- `one_shot`
- `created_from_view`
- `resolved_locations`
- `backend_breakpoint_id`
- `last_error`
- `hit_count`
- `created_at`
- `updated_at`

First wave kinds:

- instruction route
- function entry
- source line, after debug-source maps

Later kinds:

- conditional
- one-shot
- exception
- symbolic
- watchpoint

### DebugStopEvent

Fields:

- `id`
- `session_id`
- `timestamp`
- `reason`
- `thread_id`
- `frame_id`
- `pc`
- `runtime_resolution`
- `breakpoint_id`
- `signal`
- `exception`
- `module_id`
- `transport_summary`
- `problem_ids`

Stop reasons:

- breakpoint
- step
- signal
- exception
- entry
- manual_interrupt
- process_exit
- module_load
- module_unload
- unknown

### DebugThread

Fields:

- `id`
- `session_id`
- `backend_thread_id`
- `name`
- `queue_name`
- `state`
- `stop_reason`
- `selected_frame_id`
- `frames`

### DebugFrame

Fields:

- `id`
- `thread_id`
- `index`
- `pc`
- `sp`
- `fp`
- `lr`
- `module_id`
- `runtime_resolution`
- `symbol_name`
- `source_location`
- `inline_frames`
- `arguments`
- `confidence`

### DebugRegisterSet

Fields:

- `id`
- `session_id`
- `thread_id`
- `frame_id`
- `architecture`
- `registers`
- `flags`
- `timestamp`

Registers should carry value interpretations:

- integer
- pointer candidate
- selector candidate
- class candidate
- object candidate
- stack address
- heap address
- module address
- PAC-signed pointer candidate
- unknown

### DebugMemoryWindow

Fields:

- `id`
- `session_id`
- `address`
- `size`
- `bytes`
- `protection`
- `module_id`
- `source_relation`
- `read_error`
- `redaction`

First wave memory is read-only.

## API Contracts

### List Debug Targets

Endpoint:

```text
GET /api/projects/{project_id}/debug/targets
```

### Create Debug Target

Endpoint:

```text
POST /api/projects/{project_id}/debug/targets
```

Request:

```json
{
  "kind": "macos_executable",
  "name": "SampleTool",
  "source_id": "source-main",
  "executable_path_grant_id": "grant-123",
  "architecture": "arm64e",
  "backend": "lldb_local"
}
```

### Launch Session

Endpoint:

```text
POST /api/projects/{project_id}/debug/sessions
```

Request:

```json
{
  "target_id": "target-1",
  "mode": "launch",
  "configuration": {
    "arguments": ["--sample"],
    "environment": {},
    "working_directory_grant_id": "grant-cwd",
    "inherit_environment": false,
    "stop_at_entry": true,
    "symbolication_profile_id": "symprof-1"
  }
}
```

### Attach Session

Endpoint:

```text
POST /api/projects/{project_id}/debug/sessions
```

Request:

```json
{
  "target_id": "target-1",
  "mode": "attach",
  "pid": 12345,
  "confirm_privileged_attach": true
}
```

### Session Events

Endpoint:

```text
GET /api/projects/{project_id}/debug/sessions/{session_id}/events
```

Use server-sent events or an equivalent local event stream. Event kinds:

- `session_state_changed`
- `module_loaded`
- `module_unloaded`
- `stopped`
- `continued`
- `thread_changed`
- `frame_changed`
- `breakpoint_resolved`
- `breakpoint_failed`
- `memory_read`
- `problem_added`
- `transport_log`

### Control Session

Endpoint:

```text
POST /api/projects/{project_id}/debug/sessions/{session_id}/control
```

Request:

```json
{
  "command": "step_over",
  "thread_id": "thread-1"
}
```

Commands:

- `continue`
- `pause`
- `step_over`
- `step_into`
- `step_out`
- `stop`
- `detach`
- `select_thread`
- `select_frame`

### Breakpoints

Endpoint:

```text
POST /api/projects/{project_id}/debug/breakpoints
```

Request:

```json
{
  "route": {
    "kind": "instruction",
    "source_id": "source-main",
    "address": "0x100003f20"
  },
  "enabled": true
}
```

Endpoint:

```text
PATCH /api/projects/{project_id}/debug/breakpoints/{breakpoint_id}
```

Supported changes:

- enable/disable
- delete
- update condition, later
- update ignore count, later

### Memory Read

Endpoint:

```text
GET /api/projects/{project_id}/debug/sessions/{session_id}/memory
```

Query:

```text
?address=0x100003f20&size=256
```

Response must include read errors and redaction state.

## Backend Architecture

### Process Boundary

Do not run LLDB control directly inside the native UI process.

Use a local helper process:

- authenticated by per-session token
- limited to the selected project/session
- logs through redaction policy
- terminates when session ends
- exposes a narrow local API
- refuses commands outside declared capabilities

Reasons:

- LLDB can crash or hang
- debugger APIs may require unusual entitlements
- transport output may contain secrets
- target control should be cancellable/recoverable
- plugin/automation should not get direct debugger internals

### Backend Options

Potential backends:

- `lldb_local_cli`
- `lldb_local_library`
- `gdb_remote`
- `debugserver_remote_future`

Initial recommendation:

- start with a local helper that controls LLDB through a narrow abstraction
- keep backend output parsed into structured events
- do not expose a raw LLDB REPL as the first UI

Raw command escape hatches can come later for advanced users, behind warnings
and audit logs.

### LLDB Abstraction

Define a transport trait:

```rust
trait DebugTransport {
    fn capabilities(&self) -> DebugCapabilities;
    async fn launch(&mut self, config: LaunchConfig) -> Result<SessionState>;
    async fn attach(&mut self, pid: u32) -> Result<SessionState>;
    async fn continue_process(&mut self) -> Result<()>;
    async fn interrupt(&mut self) -> Result<()>;
    async fn step_over(&mut self, thread: ThreadId) -> Result<()>;
    async fn step_into(&mut self, thread: ThreadId) -> Result<()>;
    async fn step_out(&mut self, thread: ThreadId) -> Result<()>;
    async fn set_breakpoint(&mut self, location: RuntimeBreakpointLocation) -> Result<BackendBreakpointId>;
    async fn remove_breakpoint(&mut self, id: BackendBreakpointId) -> Result<()>;
    async fn read_registers(&mut self, thread: ThreadId, frame: FrameId) -> Result<RegisterSet>;
    async fn read_memory(&mut self, address: u64, size: usize) -> Result<MemoryBytes>;
    async fn modules(&mut self) -> Result<Vec<RuntimeModuleSnapshot>>;
    async fn threads(&mut self) -> Result<Vec<ThreadSnapshot>>;
    async fn shutdown(&mut self) -> Result<()>;
}
```

The rest of Damsel should depend on structured transport events, not on LLDB
string output.

### Event Normalization

The helper should normalize backend events into Damsel events:

- module load/unload
- process state change
- thread stop
- breakpoint hit
- signal/exception
- register snapshot
- frame snapshot
- memory read result
- transport error

Every event should include:

- session id
- monotonic sequence number
- timestamp
- backend event id if available
- redaction classification

## Static-Dynamic Mapping

The existing static-dynamic spec owns the mapping algorithm. This spec requires
the Workbench to consume it in all debugger views.

### Module Matching

Match runtime modules by:

- Mach-O UUID
- path
- install name
- bundle identity
- code signature cdhash, when available
- dyld shared-cache UUID and image identity
- source hash
- architecture slice

Never rely on path alone for proof.

### Runtime Address Resolution

Resolution steps:

1. Find runtime module containing address.
2. Match module to project source or cache image.
3. Compute static address using load base/slide.
4. Resolve static address to route.
5. Resolve route to function/basic block/instruction.
6. Resolve source/debug/pseudocode/graph maps if available.
7. Emit confidence and problems.

### Mismatch Problems

Problems:

- module not in project
- UUID mismatch
- architecture mismatch
- source revision mismatch
- patch revision mismatch
- dSYM/KDK mismatch
- dyld shared-cache mismatch
- address outside mapped source
- debug info line table mismatch
- route cannot be resolved

The debugger should remain useful in partial state, but it must label
mismatches.

## Breakpoint Architecture

### Route-Backed Breakpoints

User sets a breakpoint on a route, not a raw backend address.

Resolution depends on:

- module match
- source revision
- static address
- runtime slide
- backend breakpoint capability

If the target is not running, the breakpoint remains pending and resolves when
matching modules load.

### Breakpoint Persistence

Persist:

- route
- enabled state
- user label/comment
- optional target scope
- creation view
- hit count history
- last resolution result

Do not persist:

- stale backend breakpoint ids as durable truth
- runtime addresses without module identity
- conditions that include secrets without redaction

### Breakpoint Safety

Breakpoint actions should be audited:

- set
- remove
- enable
- disable
- resolved
- failed
- hit

The activity log should make debugger actions visible because they affect a
live process.

## Register And Value Interpretation

Registers should not be raw numbers only.

Interpretation layers:

- runtime module pointer
- static route pointer
- stack pointer
- heap pointer
- ObjC object candidate
- ObjC class candidate
- ObjC selector candidate
- C++ vtable pointer candidate
- PAC-signed pointer candidate
- tagged pointer candidate
- small integer
- ASCII/UTF-8 preview when safe
- unknown

Each interpretation needs confidence and provenance:

- direct module range
- Objective-C runtime metadata
- dyld shared-cache metadata
- debug info variable location
- C++ RTTI/vtable model
- pointer authentication pattern
- heuristic

This is one of Damsel's best niche opportunities: Hopper-like debugging with
Apple-runtime-aware live value interpretation.

## Stack And Frame Views

Frame rows should show:

- frame index
- symbol/function
- module
- static route
- source file/line if available
- inline frames
- arguments when recoverable
- return address
- stack pointer
- confidence
- problems

Stack memory should support:

- current frame range
- raw bytes
- pointer annotations
- route links
- ObjC/C++/kernel-like value interpretations
- copy/export with redaction

First wave should avoid writing memory.

## Runtime Memory Views

Memory reads should be explicit:

- current PC window
- selected register pointer window
- selected stack window
- selected module range

Memory view modes:

- hex
- pointers
- strings
- instructions, if executable
- object preview, later

Safety:

- cap read size
- handle unreadable pages
- label live memory as runtime observation
- never confuse runtime bytes with source bytes
- do not cache sensitive memory by default

## Objective-C Dynamic Context

At a stop event, ObjC context should show:

- likely receiver register
- selector register
- `_objc_msgSend` call site
- selector string
- class candidates
- method candidates
- IMP target candidates
- type encoding
- retain/release/autorelease pattern hints
- tagged pointer hints
- exception state, later

For an ObjC message-send breakpoint, the inspector should be able to explain:

- which static selector evidence existed before run
- what runtime receiver/class evidence was observed
- whether runtime observation confirmed or contradicted static inference

Do not mutate static method binding automatically from one observed receiver.

## C++ And ObjC++ Dynamic Context

At virtual call sites:

- show object pointer
- vtable pointer
- module/source relation of vtable
- slot index
- candidate method
- RTTI name when available
- constructor/destructor context
- dynamic-cast/type-info evidence
- confidence

This should feed a "runtime observation" panel and optional user hint, not an
automatic permanent conclusion.

## Arm64e And PAC Dynamic Context

For arm64e stops:

- mark authenticated branch/call patterns
- show signed pointer candidates
- show whether pointer was stripped/authenticated by nearby instructions
- map authenticated targets to runtime modules when possible
- label unresolved PAC target issues

The debugger should help analysts understand arm64e control flow without
pretending PAC does not exist.

## Dyld Shared Cache And System Libraries

The debugger must handle:

- main executable outside cache
- system libraries inside dyld shared cache
- shared-cache images loaded at runtime
- cache UUID matching
- image install names
- slide computation per image
- stubs into cache functions
- ObjC metadata in cache images

If the user stops in a system framework that Damsel has not registered, the UI
should offer:

- register current cache
- open matching cache image if available
- show runtime-only module view
- keep breakpoint pending/unmapped

## iOS Strategy

Do not include iOS device debugging in the first wave.

Prepare model room for:

- remote target kind
- device identity
- app bundle identity
- debugserver transport
- developer signing/profile constraints
- platform dyld shared-cache identity
- unavailable/permission failure states

The first iOS-adjacent value should come from static analysis of iOS app
bundles, frameworks, dyld shared caches, dSYMs, and crash/panic logs, not from
promising remote device debugging early.

## Kernel And XNU Strategy

Do not build live kernel debugging first.

Nearer-term XNU value:

- panic log route mapping
- KDK/dSYM-backed symbolication
- kernel collection source registration
- static-to-panic-frame context
- IOKit external-method analysis
- syscall/MIG table navigation

Future kernel debugging would require:

- separate transport
- separate trust model
- separate target configuration
- separate safety policy
- likely device/VM setup docs

Keep this separate from user-space debugger UX.

## Permissions, Trust, And Diagnostics

### Permission Model

Debugger permissions:

- `debug.launch`
- `debug.attach`
- `debug.control`
- `debug.read_registers`
- `debug.read_memory`
- `debug.manage_breakpoints`
- `debug.export_session`
- `debug.remote_future`

Each automation/plugin/MCP surface must declare capabilities. Human UI commands
still need visible confirmation for launch/attach and privileged actions.

### Error Taxonomy

User-facing errors:

- LLDB unavailable
- Xcode tools missing
- target path missing
- target architecture unsupported
- target not executable
- attach denied
- hardened runtime prevents attach
- SIP/system policy prevents attach
- sandbox prevents operation
- target exited
- module mismatch
- dSYM/KDK mismatch
- memory read denied
- backend timeout
- helper crashed
- unsupported backend command

Each error should have:

- short title
- explanation
- likely cause
- safe next action
- diagnostic detail toggle
- redaction classification

### Diagnostics

Debugger diagnostics may include:

- process ids
- paths
- arguments
- environment
- register values
- memory bytes
- symbols
- source paths
- crash text

Default diagnostics should redact:

- arguments
- environment values
- local user paths
- raw memory
- source snippets
- device identifiers

The user can explicitly export a fuller package for private debugging.

## Command Integration

Command families:

- Configure Debug Target
- Launch Under Debugger
- Attach To Process
- Continue
- Pause
- Step Over
- Step Into
- Step Out
- Stop Debugging
- Detach
- Set Breakpoint
- Remove Breakpoint
- Enable Breakpoint
- Disable Breakpoint
- Show Breakpoints
- Select Thread
- Select Frame
- Show Runtime Module
- Show PC In Assembly
- Show PC In Graph
- Show PC In Pseudocode
- Show PC In Hex
- Read Memory At Address
- Read Memory From Register
- Copy Stop Summary
- Export Debug Session
- Open Debugger Diagnostics

Command validation must use:

- session state
- selected route
- selected thread/frame
- backend capabilities
- permissions
- target platform
- active problems

## Workbench Store Architecture

Add stores:

- `DebugTargetStore`
- `DebugSessionStore`
- `DebugCommandStore`
- `DebugBreakpointStore`
- `DebugModuleStore`
- `DebugThreadStore`
- `DebugFrameStore`
- `DebugRegisterStore`
- `DebugMemoryStore`
- `DebugProblemStore`
- `DebugDiagnosticsStore`

Responsibilities:

`DebugTargetStore`:

- target definitions
- launch configurations
- attach candidates
- redaction-aware saved settings

`DebugSessionStore`:

- current session state
- event stream
- selected thread/frame
- lifecycle commands

`DebugBreakpointStore`:

- route-backed breakpoints
- resolution state
- hit counts
- gutter state

`DebugModuleStore`:

- loaded modules
- static source matching
- dyld cache matching
- mismatch problems

`DebugRegisterStore`:

- register snapshots
- value interpretations
- selected register memory routes

`DebugMemoryStore`:

- explicit memory windows
- read errors
- redaction policy

`DebugProblemStore`:

- attach/launch/module/mapping/backend problems
- suggested actions

Do not put all debugger state into `WorkbenchStore`.

## Performance And Responsiveness

### Budgets

Targets:

- debugger bar state update: under 50 ms after event received
- stop event route mapping: under 100 ms for known module
- register table render: under 100 ms
- frame list render: under 100 ms
- current PC center/highlight: under 150 ms
- breakpoint toggle UI response: under 100 ms
- memory read cancellation: under 100 ms
- attach process list refresh: under 500 ms for normal local lists

### Event Backpressure

Debugger event streams can burst. The Workbench should:

- coalesce register updates
- drop obsolete hover-only memory reads
- preserve stop/continue/module events
- never block UI on transport logs
- bound retained event history

### Main Thread Rule

No debugger transport work on the main thread.

LLDB/helper calls, process scans, module matching, symbolication, memory reads,
and diagnostics packaging must be asynchronous and cancellable.

## Persistence Policy

Persist:

- target definitions
- route-backed breakpoints
- non-sensitive launch settings when user allows
- session summaries
- module match summaries
- stop summaries, if user enables session history
- debugger problems

Do not persist by default:

- environment variable values
- raw memory
- full register history
- target stdout/stderr if sensitive
- attach process snapshots
- raw LLDB logs

Session history should be opt-in and redaction-aware.

## Automation And MCP Boundary

Read-only automation can eventually:

- list debug targets
- list breakpoints
- list active session state
- fetch current stop summary
- fetch current frame list
- fetch module match problems

Mutation/control automation should be extremely gated:

- launch
- attach
- continue
- step
- memory read
- breakpoint mutation

No background agent should attach to a process or control execution without an
explicit human-approved capability and visible activity log entry.

## Implementation Phases

### Phase 1: Data Contracts And Disabled UI

Deliver:

- debug target/session/breakpoint/module/thread/frame/register/memory models
- command definitions with disabled reasons
- debugger bar placeholder
- breakpoint gutter disabled state
- API stubs returning "debugger unavailable"
- trust/diagnostic error taxonomy

Acceptance:

- Workbench can explain why debugger commands are disabled
- static views reserve breakpoint/current-PC overlay concepts
- no live process control exists yet

### Phase 2: Module Matching Harness

Deliver:

- runtime module snapshot fixtures
- module-to-source matching API
- dyld cache image matching
- UUID/path/architecture mismatch problems
- route resolution from simulated runtime addresses

Acceptance:

- fixture runtime modules map to sources and cache images
- mismatches create structured problems
- no LLDB dependency required

### Phase 3: Local Helper Spike

Deliver:

- helper process prototype
- session token auth
- launch local executable
- module list
- stop at entry
- terminate/detach
- structured event stream
- helper crash recovery

Acceptance:

- signed local sample can launch and stop
- helper crash does not crash Workbench
- missing LLDB/Xcode tooling shows clear error

### Phase 4: Route-Backed Breakpoints

Deliver:

- set/remove/enable/disable breakpoint commands
- route-to-runtime breakpoint resolution
- unresolved/pending breakpoint states
- breakpoint gutter
- hit count and stop event link

Acceptance:

- breakpoint survives relaunch when module identity matches
- unresolved breakpoint stays visible with explanation

### Phase 5: Stop Event Workbench

Deliver:

- stopped-state debugger bar
- PC overlay in assembly
- current graph node highlight
- hex PC window
- thread/frame/register panes
- stop summary inspector

Acceptance:

- stop event maps to static route in known module
- PC outside known source opens runtime module view
- UI remains responsive during step/continue cycles

### Phase 6: Register, Stack, And Memory Interpretation

Deliver:

- register value interpretation
- stack memory window
- pointer-to-route mapping
- ObjC selector/class/object candidates
- C++ vtable candidates
- PAC pointer candidates
- read-only memory views

Acceptance:

- fixture stop snapshots show typed value interpretations
- unresolved/weak interpretations remain labeled
- memory read errors are non-fatal

### Phase 7: Pseudocode And Source Integration

Deliver:

- PC to pseudocode line mapping
- breakpoint from pseudocode gutter
- source-line breakpoint mapping
- inline frame display
- debug-info profile mismatch warnings

Acceptance:

- source and pseudocode views update from stop events
- stale/missing source maps are explicit

### Phase 8: Apple Runtime Dynamic Context

Deliver:

- ObjC message-send stop inspector
- C++ virtual-call stop inspector
- dyld shared-cache module context
- arm64e/PAC stop context
- runtime observation records

Acceptance:

- runtime observations can be exported as evidence
- static facts are not silently rewritten

### Phase 9: Session Export And Diagnostics

Deliver:

- redacted stop summary export
- full private session package option
- diagnostic bundle
- helper/backend logs with redaction
- privacy review gate

Acceptance:

- export proves route mappings and stop events
- secrets are redacted by default

### Phase 10: Remote/iOS/Kernel Design Gates

Deliver:

- separate design docs for remote GDB/debugserver
- iOS device debugging feasibility review
- kernel debugging non-goal review
- transport capability model extension

Acceptance:

- no accidental blending of local macOS debugger and remote/iOS/kernel work

## Validation Matrix

### Fixtures

Required fixtures:

- simulated module list with exact UUID match
- simulated module list with path match but UUID mismatch
- dyld shared-cache image match
- architecture mismatch
- route-to-runtime address resolution
- runtime address outside known source
- breakpoint pending before module load
- breakpoint resolved after module load
- stop at entry
- breakpoint hit
- step stop
- signal stop
- process exit
- register snapshot with pointer candidates
- stack memory with route pointers
- ObjC message-send stop context
- C++ virtual-call stop context
- arm64e/PAC pointer context
- memory read denied
- helper crash
- LLDB unavailable
- hardened-runtime attach denied

### UI Tests

Required UI tests:

- disabled debugger commands explain missing backend
- create launch configuration
- attach sheet requires explicit process choice
- toggle breakpoint from assembly gutter
- unresolved breakpoint displays problem
- launch signed sample and stop at entry, when environment permits
- PC overlay appears in assembly
- select frame maps route
- read memory from register
- export redacted stop summary

### Performance Tests

Required performance tests:

- event-stream coalescing
- stop event route mapping latency
- register table render latency
- breakpoint toggle latency
- module list matching latency
- memory read cancellation latency
- helper crash recovery latency

### Security/Trust Tests

Required tests:

- helper rejects invalid session token
- automation without `debug.attach` cannot attach
- logs redact launch environment values
- diagnostics omit raw memory by default
- attach denial maps to user-facing problem
- session export redacts local paths by default

## Acceptance Criteria

Damsel should not claim Hopper-competitive debugging until:

- launch and attach are explicit, permissioned, and diagnosable
- live modules map to project sources/cache images by UUID/architecture, not
  path alone
- breakpoints are route-backed and survive relaunch when identity matches
- unresolved breakpoints and module mismatches are visible problems
- current PC overlays assembly/graph/hex and later pseudocode/source
- registers, frames, and memory are structured Workbench models
- ObjC/C++/arm64e runtime interpretations are confidence-labeled
- LLDB/helper failures do not crash the app
- hardened-runtime/SIP/tooling errors are understandable
- debugger events do not block the main thread
- session exports are redacted by default
- automation cannot control the debugger without explicit capability
- iOS, remote, and kernel debugging are separate future products

## Summary

The debugger is a high-value Hopper feature, but the right Damsel version is not
a raw LLDB transcript. It is a native runtime overlay on top of Damsel's project
kernel, route model, symbolication profiles, graph/hex/pseudocode surfaces,
Apple runtime facts, trust model, and diagnostics pipeline.

The critical implementation idea is to make dynamic state structured,
route-backed, permissioned, cancellable, redacted, and confidence-labeled. That
keeps the feature aligned with Damsel's Apple-system niche and prevents a future
debugger from becoming an isolated pane that competes with, rather than
strengthens, the static Workbench.
