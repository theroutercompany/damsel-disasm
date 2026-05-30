# Remote Device Debugging And Target Environment Implementation Spec

Status: implementation-ready planning document.

Purpose: define the remote, device, simulator, and target-environment
debugging architecture Damsel would need after the local debugger Workbench
becomes reliable. This spec extends the static-dynamic and live-debugger specs
with the parts that are different when the process runs somewhere other than
the current Mac helper: remote macOS hosts, iOS/iPadOS devices, simulators,
DriverKit-style targets, and XNU/kernel-adjacent workflows.

This is a planning spec, not an implementation claim. It should not move
remote or iOS debugging into the first debugger milestone. It exists so the
project, source, symbolication, trust, Workbench, automation, and diagnostics
architecture does not paint Damsel into a local-only corner.

## Why This Exists

Local debugging is already large. Remote and device debugging is not simply
"LLDB but with a socket." It adds:

- target machine identity
- transport identity
- debug server lifecycle
- platform plugin selection
- remote filesystem mapping
- deployed executable identity
- device pairing and Developer Mode
- simulator runtime identity
- remote module and dyld-cache matching
- device support symbols
- platform-denial and entitlement diagnostics
- disconnection and reconnect state
- privacy and redaction for remote paths, environment, registers, memory, and
  logs
- strong separation between user-space app debugging and kernel debugging

Hopper's public Debugger Server page makes this split visible: the remote
debugger server is legacy and only needed for debugging on a different
computer, while local debugging should use the local debugger. LLDB's public
remote-debugging documentation makes the architecture explicit: remote
debugging uses a client/server model, the remote system runs the server, the
local system runs the client, and macOS/iOS use `debugserver` for remote-gdb
functionality.

Damsel should eventually use this model, but with a Damsel-specific advantage:
every remote runtime object must resolve back to the same static project
routes, Apple runtime facts, source revisions, KDK/dSYM profiles, dyld cache
images, decompiler source maps, and evidence exports.

## Evidence Summary

Hopper evidence:

- Hopper Debugger Server allows remote debugging.
- Hopper's Debugger Server page says the server is legacy unless remote
  debugging is required.
- Hopper says Xcode is required on the target machine for Debugger Server.
- Hopper recommends the Local Debugger when debugging does not happen on a
  different computer.
- Hopper's tutorial shows breakpoints in the assembly margin and debugger
  integration as part of the same representation surface.
- Hopper changelog evidence already captured in this pack includes attach
  dialogs, hardened-runtime detection, environment variables, GDB remote target
  handling, PC/RIP outside the main binary, debugger panel issues, and patch
  application before launch.

LLDB and Apple platform evidence:

- Apple's LLDB guide documents launch, attach, breakpoints, watchpoints,
  process control, thread lists, backtraces, frames, variables, and command
  execution.
- LLDB remote debugging uses client/server architecture with local and remote
  systems communicating through the gdb-remote protocol.
- LLDB remote debugging requires additional preparation when the process runs
  on a remote system, especially when OS or architecture differs.
- LLDB lists platform plugins including host, remote macOS, remote iOS,
  simulator targets, Darwin kernel, and other remote platforms.
- On macOS and iOS, LLDB remote-gdb functionality is implemented by
  `debugserver`.
- Apple Developer Mode is required for development workflows on physical
  devices in modern iOS-family platforms.
- Apple's debugger entitlement/hardened-runtime documentation establishes that
  debug attachment is a code-signing and runtime-policy issue, not merely a
  transport issue.
- Apple's KDK/kernel-extension debugging documentation establishes kernel
  debugging as a specialized KDK/LLDB workflow, separate from ordinary app
  attach.

Damsel current-state evidence:

- Damsel already has static Mach-O, dyld, Objective-C, shared-cache,
  disassembly-analysis, projected-image, and evidence-package foundations.
- Existing Hopper-gap specs define local debugger route mapping, live debugger
  Workbench surfaces, trust/distribution policy, KDK/dSYM debug-info, artifact
  intake, Apple runtime browsers, processor semantics, and decompiler quality.
- Remote debugging is currently noted as later research, not fully specified
  as a product/workbench architecture.

## Product Thesis

Damsel should treat remote/device debugging as a target-environment overlay on
the same analysis project.

The local project remains the source of truth for:

- source artifacts
- container graphs
- static address spaces
- symbolication profiles
- procedure/function records
- route ids
- annotations and hints
- patch revisions
- decompiler source maps
- Apple runtime object records
- evidence export policy

The remote target contributes:

- target identity
- connection and transport identity
- runtime process identity
- runtime module list
- runtime slide/load address map
- thread/frame/register/memory state
- debug-server capabilities
- remote filesystem paths
- device/simulator metadata
- platform failure reasons
- runtime event stream

The Workbench should never blur these two planes. If a runtime module cannot
be matched to a project source, it should be visible as a runtime-only module.
If a project route cannot be resolved into the current runtime, the breakpoint
or navigation target should remain pending with an explanation.

## Scope

In scope:

- remote macOS user-space debugging
- local simulator debugging as a target environment
- iOS/iPadOS development-device debugging where legal tooling and pairing
  permit it
- remote LLDB/debugserver transport modeling
- device/simulator/source identity modeling
- remote module and dyld-cache matching
- remote launch/install/attach plans
- reconnect and disconnect state
- read-only runtime memory windows
- route-backed breakpoints
- stop-event records
- ObjC/C++/Swift/arm64e runtime interpretation at stop points
- KDK/dSYM/source symbolication profiles
- diagnostics and redaction
- automation and assistant read endpoints with capability gates

Out of scope for first remote/device milestone:

- bypassing iOS platform protections
- attaching to arbitrary App Store apps
- process memory writing
- kernel live debugging as a default Workbench mode
- jailbreak-only workflows
- remote code execution shell features exposed to scripts or assistant tools
- shipping or copying debugserver onto targets outside allowed toolchains
- automatic device pairing
- stealth attach
- hidden network listeners

## Target Environment Model

### TargetEnvironment

Fields:

- `target_environment_id`
- `kind`: local-macos, remote-macos, ios-device, ipados-device,
  ios-simulator, tvos-device, watchos-device, visionos-device, driverkit-host,
  darwin-kernel, unknown
- `display_name`
- `host_identity`
- `device_identity`
- `os_name`
- `os_version`
- `kernel_version`
- `architecture`
- `platform_plugin`
- `developer_mode_state`
- `pairing_state`
- `xcode_requirement_state`
- `toolchain_state`
- `transport_capabilities`
- `debug_capabilities`
- `source_access_capabilities`
- `symbolication_capabilities`
- `trust_state`
- `last_seen_at`
- `problems`

Why:

- LLDB platform plugins and Apple device workflows are not interchangeable.
- The same binary may be local, simulator, remote macOS, or physical iOS with
  different module, symbol, and permission behavior.

### TargetHostIdentity

Fields:

- `host_id`
- `hostname`
- `user_label`
- `local_or_remote`
- `os_version`
- `architecture`
- `developer_tools_version`
- `xcode_path`
- `debugserver_path`
- `lldb_version`
- `network_identity`
- `trust_fingerprint`
- `last_connection_result`
- `problems`

### DeviceIdentity

Fields:

- `device_id`
- `kind`
- `name`
- `udid_or_opaque_identifier`
- `product_type`
- `os_version`
- `developer_mode_state`
- `pairing_state`
- `connection_kind`: usb, network, simulator, unknown
- `symbol_cache_state`
- `device_support_state`
- `allowed_debug_scopes`
- `problems`

Privacy rule:

- Device identifiers must be redacted in diagnostics unless explicitly included
  in an evidence export.

## Transport Model

### DebugTransport

Fields:

- `transport_id`
- `target_environment_id`
- `kind`: local-loopback, tcp, usb-mediated, xcode-device, simulator,
  remote-gdb, lldb-platform, manual, unknown
- `endpoint`
- `protocol`: gdb-remote, lldb-platform, debugserver, local-helper, unknown
- `authentication_state`
- `encryption_state`
- `allowed_origins`
- `connection_state`
- `latency_ms`
- `last_error`
- `audit_log_id`

Transport requirements:

- No hidden listeners.
- No wildcard remote bind without explicit user choice.
- Remote endpoint and target identity must be shown before connect.
- Reconnect attempts must be bounded and visible.
- Assistant/automation cannot open a new remote transport without permission.

### DebugServerInstance

Fields:

- `debug_server_id`
- `target_environment_id`
- `transport_id`
- `server_kind`: debugserver, lldb-server, HopperDebuggerServer, local-helper,
  external
- `version`
- `launch_mode`: user-started, helper-started, externally-managed
- `pid`
- `listening_endpoint`
- `capabilities`
- `start_time`
- `lifecycle_state`
- `stdout_log_ref`
- `stderr_log_ref`
- `redaction_state`
- `problems`

Policy:

- Damsel should not assume it can install or launch a server everywhere.
- Server startup must be a plan with user-visible steps.
- External server mode is first-class; many targets will be prepared outside
  Damsel.

## Remote Launch And Attach Plans

### RemoteDebugPlan

Fields:

- `plan_id`
- `project_id`
- `target_environment_id`
- `mode`: launch, attach, wait-for-launch, connect-only, inspect-core,
  kernel-context
- `executable_source`
- `runtime_bundle_id`
- `remote_install_path`
- `arguments`
- `environment`
- `working_directory`
- `requires_upload`
- `requires_install`
- `requires_signing`
- `requires_device_registration`
- `requires_developer_mode`
- `requires_xcode`
- `requires_debugserver`
- `expected_modules`
- `symbolication_profiles`
- `privacy_policy`
- `risk_summary`
- `preflight_steps`
- `blocking_problems`

Preflight output:

- whether the target can be debugged
- whether the executable matches the project source
- whether source/revision identity matches a patched artifact
- whether the target architecture differs
- whether required symbols are available
- whether environment/arguments will be logged or redacted
- whether attach may fail due to platform policy

### AttachCandidate

Fields:

- `candidate_id`
- `target_environment_id`
- `pid`
- `process_name`
- `bundle_id`
- `executable_path`
- `architecture`
- `owner`
- `debuggable_state`
- `reason_if_not_debuggable`
- `matched_project_source`
- `matched_confidence`
- `requires_confirmation`

Rules:

- Never attach by fuzzy process name without a confirmation sheet.
- Never silently retry attach to a different pid after process restart.
- If attach is denied, preserve platform error text but redact sensitive paths
  where diagnostics require it.

## Runtime Source And Module Matching

### RuntimeModuleMatch

Fields:

- `runtime_module_id`
- `project_source_id`
- `match_kind`: uuid, build-id, install-name, path, bundle-id, dyld-cache-image,
  fileset-member, symbol-only, fuzzy, none
- `confidence`
- `runtime_load_address`
- `runtime_slide`
- `static_preferred_address`
- `uuid`
- `architecture`
- `source_revision_id`
- `symbolication_profile_id`
- `dyld_cache_identity`
- `problems`

Why:

- On remote devices, the runtime path may not equal the local path.
- dyld shared-cache images and system frameworks require matching by UUID,
  install name, cache identity, and symbol profile rather than path alone.
- A patched or re-signed artifact may share a name but not bytes.

### DeviceSymbolSource

Fields:

- `symbol_source_id`
- `target_environment_id`
- `kind`: dSYM, KDK, device-support, dyld-cache-symbols, app-debug-symbols,
  external
- `version`
- `build`
- `uuid_matches`
- `source_path`
- `privacy_state`
- `import_state`
- `problems`

Workbench behavior:

- show missing device symbols as a concrete problem
- offer attachment/import flows
- do not silently use mismatched symbols

## Runtime Event Stream

### RuntimeEvent

Fields:

- `event_id`
- `session_id`
- `sequence`
- `timestamp`
- `kind`: connected, disconnected, launched, attached, module-loaded,
  module-unloaded, breakpoint-resolved, breakpoint-hit, watchpoint-hit,
  stopped, continued, stepped, signal, exception, thread-created,
  thread-exited, process-exited, stdout, stderr, diagnostic, reconnecting,
  reconnect-failed
- `runtime_routes`
- `project_routes`
- `payload_ref`
- `redaction_state`
- `problems`

Requirements:

- The UI must be able to replay the event stream after a disconnect.
- Large payloads such as memory windows and logs should be stored as bounded
  references, not in huge Swift arrays.
- Runtime events should feed the same activity log and evidence export model
  as static analysis jobs.
- Casebook timeline records should preserve target-environment identity,
  device/simulator privacy flags, source-symbol status, and stop-report routes
  without storing raw device logs by default.

## Remote Workbench Surfaces

### Target Environment Browser

Display:

- local macOS
- remote macOS hosts
- connected devices
- simulators
- known symbol caches
- toolchain status
- developer mode/pairing status
- last connection
- problems

Actions:

- add remote target
- connect
- disconnect
- run preflight
- open symbol cache
- forget target
- export redacted target diagnostics

### Debug Plan Sheet

Sections:

- target
- mode
- executable/source revision
- launch arguments
- environment
- install/upload requirements
- symbolication profiles
- transport
- privacy/redaction
- risks
- preflight result

### Remote Module View

Display:

- module name
- runtime load address
- slide
- source match confidence
- dyld cache identity
- dSYM/KDK/device-symbol state
- route resolution state
- unresolved reasons

### Device Runtime Problems Drawer

Problem categories:

- device not paired
- Developer Mode disabled
- Xcode/toolchain unavailable
- debugserver unavailable
- transport refused
- target architecture unsupported
- process not debuggable
- entitlement denied
- hardened runtime denial
- symbol cache missing
- source mismatch
- dyld cache mismatch
- KDK mismatch
- remote path redacted
- disconnected
- reconnect failed

### Remote Stop Report

Display:

- stop reason
- target environment
- process/thread/frame
- current route
- module/source match
- register summary
- ObjC/C++/Swift/PAC interpretation
- memory context
- source/decompiler route if available
- runtime-only explanation if not
- export/copy options

## iOS And Simulator Policy

### Physical iOS/iPadOS Devices

Supported only when:

- the workflow is legal and user-authorized
- the device is paired/trusted where required
- Developer Mode is enabled where required
- the app/process is debuggable under platform policy
- required symbols can be matched or missing-symbol state is visible

Unsupported by default:

- arbitrary App Store app attach
- bypassing FairPlay/DRM or platform protections
- jailbreak-only debugserver workflows
- stealth runtime inspection
- process memory writes

User-facing language:

- "This target is not debuggable with ordinary Apple development tooling."
- "Damsel can still analyze a decrypted/provided binary statically if you
  legally provide one, but it will not bypass device protections."

### iOS Simulator

Simulator-specific records:

- simulator runtime id
- simulator device id
- host dyld cache relation
- app container path
- launch service state
- architecture translation state

Simulator advantages:

- easier launch and attach
- deterministic fixture runner
- good early validation target for route-backed breakpoints and stop reports

Simulator caveats:

- simulator behavior is not physical-device behavior
- dyld/cache/module identity may differ
- performance and PAC behavior may differ

## Kernel And XNU Boundary

Damsel's XNU lane is important, but live kernel debugging should remain a
separate product gate.

Kernel-context modes:

- static kernel collection analysis
- KDK/dSYM symbolication
- panic-log route mapping
- optional kernel debug plan record

First remote/device debugger should not:

- attach to the live kernel
- read kernel memory
- control a kernel debug session
- require disabling system protections

But the model should reserve:

- `darwin-kernel` target environment
- KDK profile
- kernel debug transport placeholder
- panic-frame-to-route mapping
- kernel debug preflight problems

Why:

- LLDB exposes a Darwin kernel platform plugin.
- Apple's KDK debugging docs establish a specialized workflow.
- Damsel's product advantage should still be kernel-aware, even when live
  kernel debugging is deferred.

## API Contracts

Routes:

```text
GET /api/debug/target-environments
POST /api/debug/target-environments
GET /api/debug/target-environments/{id}
POST /api/debug/target-environments/{id}/preflight
POST /api/debug/target-environments/{id}/connect
POST /api/debug/target-environments/{id}/disconnect
GET /api/debug/target-environments/{id}/attach-candidates
POST /api/projects/{project_id}/debug/remote-plans
GET /api/projects/{project_id}/debug/remote-plans/{plan_id}
POST /api/projects/{project_id}/debug/remote-plans/{plan_id}/start
GET /api/projects/{project_id}/debug/sessions/{session_id}/events
GET /api/projects/{project_id}/debug/sessions/{session_id}/runtime-modules
GET /api/projects/{project_id}/debug/sessions/{session_id}/stop-report/latest
POST /api/projects/{project_id}/debug/sessions/{session_id}/reconnect
GET /api/projects/{project_id}/debug/sessions/{session_id}/diagnostics
```

CLI:

```text
damsel debug targets list --json
damsel debug targets add-remote --host <host> --json
damsel debug targets preflight <target-id> --json
damsel debug targets connect <target-id> --json
damsel debug attach-candidates <target-id> --json
damsel debug remote-plan <project> --target <target-id> --mode <launch|attach> --json
damsel debug remote-start <project> <plan-id> --json
damsel debug events <project> <session-id> --json
damsel debug modules <project> <session-id> --json
damsel debug stop-report <project> <session-id> --json
```

Automation gates:

- target list: allowed read-only
- preflight: allowed read-only
- connect: permissioned
- launch: permissioned
- attach: explicit confirmation
- memory read: bounded and permissioned
- process control: permissioned
- remote server start: explicit confirmation

## Diagnostics And Redaction

Redact by default:

- remote usernames
- home directory paths
- device UDIDs
- bundle container paths
- launch arguments
- environment variables
- process memory bytes
- register values when exported outside local session
- stdout/stderr unless user includes logs
- network endpoints unless target diagnostics include them

Diagnostic bundle contents:

- target environment summary
- transport state
- preflight problems
- version/toolchain summary
- runtime module match summary
- symbolication profile summary
- redacted event summary
- helper/debug-server crash summaries
- reproduction steps without secrets

## Implementation Slices

### RD1: Target Environment Registry

Deliver:

- target environment model
- local macOS target record
- simulator target placeholders
- remote target placeholder
- CLI/API list

Validation:

- JSON snapshots
- redaction snapshots
- duplicate target handling

### RD2: Remote Preflight Plans

Deliver:

- remote debug plan model
- preflight steps
- toolchain/version problem records
- source match preflight
- symbolication profile preflight

Validation:

- missing Xcode problem
- missing debugserver problem
- unsupported target problem
- source mismatch problem

### RD3: Transport Abstraction

Deliver:

- transport model
- connect/disconnect lifecycle
- audit log
- bounded reconnect policy
- no hidden listener rule

Validation:

- refused connection fixture
- reconnect fixture
- redacted endpoint export

### RD4: Remote Module Matching

Deliver:

- runtime module match records
- UUID/path/install-name/dyld-cache matching
- device symbol source model

Validation:

- exact UUID match
- dyld cache image match
- source mismatch
- missing dSYM/device-symbol problem

### RD5: Remote Stop Event And Report

Deliver:

- runtime event stream
- remote stop report
- thread/frame/register summary payloads
- route mapping

Validation:

- breakpoint stop event
- signal/exception stop event
- runtime-only frame
- redacted register export

### RD6: Simulator Debug Smoke

Deliver:

- simulator target environment
- launch plan
- route-backed breakpoint
- stop report

Validation:

- simulator launch/attach fixture where host supports it
- skipped-with-reason fixture where simulator is unavailable

### RD7: iOS Device Preflight

Deliver:

- device target record
- Developer Mode/pairing/toolchain state
- allowed/unsupported attach explanations
- no bypass policy

Validation:

- developer mode disabled fixture
- unpaired device fixture
- unsupported App Store attach fixture
- development app allowed fixture where environment supports it

### RD8: Kernel Context Placeholder

Deliver:

- `darwin-kernel` target environment placeholder
- KDK profile relation
- panic route relation
- live kernel debugging blocked state

Validation:

- KDK available/missing snapshot
- panic frame route snapshot
- live kernel blocked problem

### RD9: Native Workbench Target Browser

Deliver:

- target environment browser
- debug plan sheet
- remote module view
- runtime problems drawer
- stop report view

Validation:

- Swift model decode tests
- empty/no-target state
- target problem rendering
- attach confirmation state
- disconnect/reconnect state

## Risks

Risk: Damsel accidentally promises iOS bypass capability.

Mitigation:

- explicit unsupported states
- no jailbreak-only automation
- no hidden debugserver deployment
- static analysis remains available for legally provided binaries

Risk: remote debugging becomes a shell.

Mitigation:

- no general remote shell exposed to scripts or assistant
- platform shell features are not modeled as first-class user commands in early
  milestones
- only debugger-specific operations with capability gates

Risk: runtime and static artifacts mismatch.

Mitigation:

- source revision identity
- UUID/build-id matching
- dyld cache identity
- route confidence and runtime-only states

Risk: target diagnostics leak private device or path data.

Mitigation:

- redaction-by-default
- per-export inclusion choices
- local-only diagnostic mode

Risk: remote event streams freeze the Workbench.

Mitigation:

- bounded event pages
- backpressure
- helper isolation
- cancellable subscriptions

Risk: kernel debugging scope explosion.

Mitigation:

- kernel context placeholder only
- KDK/dSYM/panic static workflows first
- live kernel transport requires a future explicit product gate

## Acceptance Criteria

Remote/device debugging architecture is credible when:

- target environments are first-class records
- remote transport is explicit and audited
- device/simulator states are visible before launch/attach
- unsupported iOS and kernel workflows produce honest problem records
- debug plans explain upload/install/signing/toolchain/symbol requirements
- runtime modules match project sources with confidence and provenance
- stop reports route back to static project objects where possible
- runtime-only state is visible when route mapping fails
- remote events are paged, replayable, and redacted
- automation and assistant tools cannot attach or launch without permission
- diagnostics explain platform denials without leaking sensitive target data

## Source Links

- Hopper Debugger Server: <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Apple LLDB standalone debugger guide: <https://developer.apple.com/library/archive/documentation/IDEs/Conceptual/gdb_to_lldb_transition_guide/document/lldb-terminal-workflow-tutorial.html>
- Apple LLDB command summary: <https://developer.apple.com/library/archive/documentation/General/Conceptual/lldb-guide/chapters/A3-GDB-Summary.html>
- LLDB remote debugging: <https://lldb.llvm.org/use/remote.html>
- Apple debugging tool entitlement: <https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.debugger>
- Apple Developer Mode: <https://developer.apple.com/documentation/xcode/enabling-developer-mode-on-a-device>
- Apple KDK/kernel-extension debugging: <https://developer.apple.com/documentation/apple-silicon/debugging-a-custom-kernel-extension>
- Xcode command-line tool reference: <https://developer.apple.com/documentation/xcode/xcode-command-line-tool-reference>
