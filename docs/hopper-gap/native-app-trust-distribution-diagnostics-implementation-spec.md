# Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec

Status: implementation-grade architecture spec. This is not an implementation
claim.

Purpose: define the native macOS trust layer that Damsel needs before it can
feel close to Hopper as a serious reverse-engineering workstation: signed and
notarized distribution, hardened-runtime and debugger entitlement planning,
privacy-preserving source access, local helper/server security, crash and hang
diagnostics, update policy, plugin trust, and user-visible recovery flows.

This document focuses on macOS because Damsel's competitive lane is Apple
systems analysis. The design should still keep iOS and XNU artifacts central:
dyld shared caches, driver caches, kernel collections, KDK/dSYM sidecars,
panic logs, and stripped Objective-C/C++ system binaries create unusual file
access, privacy, and diagnostic risks.

## Why This Exists

Hopper is not only a disassembler core. Public evidence shows that it is a
native Mac product with a managed distribution, update, debugger, plugin, crash
fix, hang fix, performance, and compatibility surface.

Damsel currently has the beginning of a native Workbench, but its trust layer is
mostly absent:

- no project-level file access grants
- no security-scoped bookmark design
- no signed/notarized app bundle plan
- no hardened-runtime entitlement matrix
- no update mechanism
- no crash/hang diagnostic bundle
- no local helper authentication model
- no plugin trust store
- no native permission recovery flow

Those are not cosmetic details. A reverse-engineering workstation asks users to
open private binaries, system caches, kernel artifacts, crash logs, and sidecar
symbol files. Trust failures here can leak file paths or strings, leave helper
processes running, fail under Gatekeeper, confuse debugger entitlement errors,
or make every crash unreproducible.

## Public Hopper Evidence

### Official Product And Download Evidence

Hopper's official homepage describes it as "The fast and native macOS
disassembler" and emphasizes a polished Cocoa interface, keyboard shortcuts,
native macOS integration, MCP support, CFG, decompiler, Objective-C and Swift
awareness, debugger support, Python scripting, and an extensible SDK.

The official download page says the Mac build is a Universal Binary 2 package
compatible with Intel and Apple Silicon CPUs. It exposes a changelog and RSS
feed, keeps previous versions for license and macOS compatibility, and offers a
public SDK archive.

The official FAQ says Hopper's update plan is managed by license validity,
licenses remain usable after the update plan expires, and the app itself can
install the latest version allowed by the user's license.

Damsel implication:

- distribution and update policy are part of product architecture, not after
  release polish
- previous-version and compatibility policy must be explicit once builds leave
  local development
- Workbench must surface local trust state, version state, server/helper state,
  and diagnostic state in native terms

### Official Release API Evidence

The public release API currently reports:

- macOS app `6.3.0`, published `2026-05-19 16:58:15`
- macOS deployment target `12.4`
- macOS DMG file length `34754814`
- SHA1 file hash
- Sparkle-style `signature`
- Sparkle-style `edSignature`
- SDK `6.3.0`, also published `2026-05-19 16:58:15`
- Linux packages `6.3.3`, published `2026-05-22 19:20:00`

Damsel implication:

- release metadata needs machine-readable version, platform, build, minimum OS,
  file length, hash, signature, channel, and public/private status fields
- if Damsel adopts Sparkle later, EdDSA archive signatures and Apple code
  signing must be designed together
- update eligibility should be separated from update discovery so local builds,
  private prereleases, and future public builds can share one release model

### Official Changelog Evidence

The changelog repeatedly treats app reliability, compatibility, performance,
and integration boundaries as product work:

- `6.3.0` fixes a Swift demangler crash.
- `6.2.9` fixes Python interpreter compatibility.
- `6.2.7` improves large-binary load time, large-function decompilation, and
  Intel binary analysis.
- `6.2.6` optimizes analysis time for very large binaries.
- `6.2.5` rewrites the MCP server in C++ to avoid a Swift dependency and to
  share code between macOS and Linux; it also fixes a minimized-window MCP
  crash.
- `6.2.4` fixes a decompiler memory leak and an MCP document lookup issue.
- `6.2.2` fixes MCP and help behavior with multiple installed versions.
- `6.2.1` fixes an Intel crash and defers MCP-triggered Hopper launch until the
  first real request.
- `6.2.0` improves large database load time, memory footprint, MCP reliability,
  and UI details.
- `6.0.11`, `6.0.10`, `6.0.7`, `6.0.5`, and `6.0.2` fix analysis,
  decompiler, and UI hangs.
- `5.16.1` and `5.16.0` improve hardened-runtime detection and debugger attach
  behavior.
- `5.15.5` fixes debugger session problems and improves attach-to-process UI.
- `5.15.0`, `5.14.2`, `5.11.5`, `5.8.6`, `5.8.0`, `5.7.8`, `5.7.7`,
  `5.7.6`, and `5.7.5` show continuing dyld shared-cache compatibility,
  extraction, symbol, and crash/hang work.
- `5.11.6` cleans up license and update plan purchase code.
- `5.8.1` adds a startup-crash-loop escape for user plugins.
- `5.3.4` fixes multi-account license installation.

Damsel implication:

- crash/hang/deadlock fixes need first-class diagnostic capture
- local MCP/helper reliability belongs in the same trust layer as Workbench
  startup
- "multiple installed versions" and "old project/data file" compatibility need
  explicit model versioning
- plugin failure isolation must be planned before plugins exist
- debugger attach failures must be surfaced as entitlement/SIP/runtime problems,
  not generic "debugger failed" banners

### Official SDK Evidence

The public SDK 6.3.0 quick-tour document says the SDK supports file loader,
CPU, and tool plugins. It states that on macOS, starting with macOS 11.0,
plugins need to be properly signed with an Apple developer certificate in order
to be loaded by Hopper.

The SDK describes plugin install scopes:

- system scope under `/Library/Application Support/Hopper/PlugIns/v4`
- user scope under `~/Library/Application Support/Hopper/PlugIns/v4`
- type-specific subdirectories for loaders, CPUs, and tools
- bundles with type-specific extensions

It also describes loader progress callbacks, background work, plugin UUIDs that
are stored into Hopper databases, loader options that can require user input,
and file loader responsibility for segment/section creation.

Damsel implication:

- plugin trust is a persistence and database problem, not only a dynamic-load
  problem
- extension-produced facts must include plugin identity, version, signing
  status, capability grant, and crash/isolation state
- user and system plugin scopes should be treated differently
- if plugins are ever native dylibs/bundles, code signing is mandatory for the
  macOS trust story

### Debugger Evidence

The official debugger server page says Hopper Debugger Server supports remote
debugging, requires Xcode on the target machine, and is now legacy unless
remote debugging is needed because local debugging should use the Local
Debugger.

The changelog adds hardened-runtime detection and attach-to-process UI work.

Damsel implication:

- local debugging should be designed first
- remote debugging is a separate trust and transport problem
- the Workbench must explain Xcode, developer tools, SIP, hardened runtime,
  code signing, entitlement, and target identity failures in domain language
- debugger-related entitlements should not be added blindly; they need a build
  flavor and policy matrix

### Third-Party Package Evidence

Desktop Insights reports Hopper as a Swift macOS app, Universal Apple
Silicon/Intel package, signed, notarized, hardened runtime enabled, using
Sparkle and Protocol Buffers, and carrying debugger/code-signing runtime
entitlements such as JIT, unsigned executable memory, debugger,
disable-executable-page-protection, and disable-library-validation.

This is not official Hopper documentation. Treat it as lower-confidence package
inspection evidence. It still matters because it matches the shape implied by
Hopper's public release API, update framework evidence, debugger surface, and
native macOS positioning.

Damsel implication:

- entitlement decisions should be explicit and testable
- debugger and code-loading capability should be separated from ordinary
  analysis builds where practical
- a future public app should verify signing, notarization, hardened runtime, and
  nested helper signatures as a release gate

### Apple Platform Evidence

Apple's platform security documentation says apps distributed outside the App
Store on macOS 10.15 or later must be signed with Developer ID and notarized to
run under default Gatekeeper settings. It also says code signing and
notarization have distinct roles, and Mandatory Access Controls use code
signing to enable protected entitlements.

Apple also documents Gatekeeper as verifying identified developer status,
notarization, and unchanged software before first launch. Apple documents the
macOS file access model as requiring consent for protected locations such as
Documents, Downloads, Desktop, iCloud Drive, and network volumes, and explicit
Privacy settings for full storage access, Accessibility, and Automation.

Damsel implication:

- signing, notarization, entitlements, file access grants, and diagnostics are
  a single trust architecture
- local dev builds can be ad hoc, but public or private distributed builds need
  policy and validation
- Workbench must distinguish file access denial, sandbox denial, Full Disk
  Access absence, helper crash, and decoder error

### Sparkle Evidence

Sparkle's documentation describes EdDSA signatures, appcasts, Developer ID
distribution, notarized disk images, proper signing of helper tools, update
archive signatures, and key rotation constraints.

Damsel implication:

- if Damsel adopts Sparkle, the release model must track `SUFeedURL`,
  `SUPublicEDKey`, archive signatures, feed signing policy, minimum system
  version, and update channel
- update UI must handle read-only mount and app translocation style failures
- update checks should not run before privacy and network policy are explicit

## Current Damsel Baseline

Current local evidence:

- `damsel-app/Package.swift` defines a Swift package executable
  `DamselWorkbench` and library `DamselAppCore` for macOS 14.
- `DamselServerProcess` launches a `damsel-cli ui --no-open --port 0` helper
  process, parses the ready URL from stdout, and terminates the process on
  deinit.
- `DamselServerProcess` can use `DAMSEL_UI_BASE_URL` or `DAMSEL_CLI_PATH` from
  the environment.
- `DamselAPIClient.uploadImage` reads the selected file into `Data` and posts a
  multipart upload to `/api/images`.
- `WorkbenchStore` owns server bootstrap, selected file URL, loaded image,
  disassembly, symbol suggestions, busy state, banner messages, target inputs,
  filter text, and selected instruction address in one object.
- The Workbench currently has useful request-id protections against stale load
  and disassembly responses.

Current gaps:

- local server has no session token
- local server trust is based on an unqualified loopback URL
- external base URL is accepted from environment when the scheme starts with
  `http`
- source access is upload based, not project-grant based
- whole files are loaded into Swift memory before upload
- selected file paths can live directly in app state
- no source access persistence, revocation, or bookmark repair model exists
- no helper identity, helper version, helper signature, or binary path trust is
  tracked
- no update state, release channel, or compatibility state exists
- no crash/hang report model exists
- no diagnostics redaction model exists in the app layer
- no Workbench UI exists for permissions, privacy, helper status, or diagnostic
  export

## Design Principles

### User Trust Before Automation

Every future automation surface, MCP tool, plugin, debugger action, and export
pipeline must share the same local privacy rules:

- no raw bytes leave the machine by default
- no raw strings leave diagnostics by default
- no absolute source paths leave diagnostics by default
- user annotations are private by default
- external clients need explicit capabilities
- local helper state is visible and revocable

### Least Capability By Build Flavor

Damsel should not ship one all-powerful app identity if narrower build flavors
can reduce risk:

- `dev`: local development, ad hoc signing, no update channel
- `internal`: Developer ID signed if possible, private update channel optional
- `analysis`: public analysis build, no debugger entitlement by default
- `debugger`: explicit debugger-capable build, documented entitlements
- `research`: local-only build with experimental plugin/debugger features

The first public-quality build should target analysis, project, diagnostics,
and local helper reliability before debugger or plugin load capabilities.

### Native Failure Reasons

All platform failures should produce typed problems:

- `source_access_denied`
- `security_bookmark_stale`
- `full_disk_access_required`
- `automation_permission_denied`
- `helper_signature_mismatch`
- `helper_version_mismatch`
- `helper_startup_timeout`
- `helper_bound_remote_address`
- `local_session_token_missing`
- `notarization_missing`
- `hardened_runtime_missing`
- `debugger_entitlement_missing`
- `developer_tools_missing`
- `target_hardened_runtime_blocked`
- `plugin_signature_invalid`
- `diagnostics_redaction_required`
- `update_signature_invalid`

The UI should translate these into concrete recovery actions without erasing
the exact code and provenance.

### Privacy Is A Data Model

Privacy cannot live only in UI copy. The project database and API contracts
need privacy classifications:

- raw path
- file name
- content hash
- Mach-O UUID
- install name
- section/segment names
- symbol names
- Objective-C selector/class names
- C++ demangled names
- strings
- comments
- panic log text
- raw bytes
- stack frames
- KDK/dSYM metadata
- environment values
- local process list entries
- debugger target identity

Each diagnostic/export/API surface must declare which classes it may include.

## Target Architecture

```text
Workbench app
  TrustStore
  PermissionStore
  SourceAccessStore
  HelperStore
  DiagnosticStore
  UpdateStore
  CrashRecoveryStore
  PluginTrustStore
  DebuggerTrustStore
  PrivacyPolicyStore
  WorkbenchSessionStore
  Damsel API client
  Local helper launcher

Local helper process
  Authenticated loopback listener
  Project/source registry
  Source access service
  Analysis job runtime
  Diagnostics collector
  Redaction engine
  Capability checker
  Future MCP/plugin/debugger bridges

Rust project runtime
  AnalysisProject
  Source grants
  Address spaces
  Facts/provenance
  Jobs/problems
  Diagnostics/export policy
  Audit log
```

## Core Model Types

### Trust Policy

```rust
pub struct TrustPolicy {
    pub app_build: AppBuildIdentity,
    pub helper_policy: HelperTrustPolicy,
    pub source_policy: SourcePrivacyPolicy,
    pub diagnostics_policy: DiagnosticsPolicy,
    pub update_policy: UpdatePolicy,
    pub plugin_policy: PluginPolicy,
    pub debugger_policy: DebuggerPolicy,
}
```

Rules:

- stored per app install and partially copied into projects
- project copy records the policy under which evidence was produced
- policy version is included in diagnostic bundles
- policy changes are audit-log entries

### App Build Identity

```rust
pub struct AppBuildIdentity {
    pub app_version: String,
    pub build_number: String,
    pub git_revision: Option<String>,
    pub build_flavor: BuildFlavor,
    pub minimum_macos: String,
    pub bundle_identifier: String,
    pub signing: SigningState,
    pub notarization: NotarizationState,
    pub hardened_runtime: HardenedRuntimeState,
}
```

Build flavors:

- `dev`
- `internal`
- `analysis`
- `debugger`
- `research`

Signing states:

- `unsigned`
- `ad_hoc`
- `developer_id`
- `apple_development`
- `app_store`
- `unknown`

Notarization states:

- `not_applicable`
- `not_checked`
- `notarized`
- `not_notarized`
- `invalid`

### Helper Identity

```rust
pub struct HelperIdentity {
    pub executable_path_redacted: RedactedPath,
    pub version: String,
    pub protocol_version: u32,
    pub content_hash: String,
    pub signing: SigningState,
    pub launch_mode: HelperLaunchMode,
    pub bind_address: String,
    pub port: u16,
    pub session_id: String,
}
```

Rules:

- helper must bind only to loopback by default
- helper readiness includes protocol version and content hash
- Workbench refuses a helper with mismatched protocol version
- external base URLs are disabled in release builds unless explicitly enabled
  by a signed preference or developer mode
- helper output is captured for diagnostics with redaction

### Local Server Session

```rust
pub struct LocalServerSession {
    pub session_id: String,
    pub token_hash: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub client_pid: Option<u32>,
    pub helper_pid: Option<u32>,
    pub allowed_origins: Vec<String>,
}
```

Rules:

- Workbench generates a random session token before launching the helper
- helper receives token through inherited file descriptor or environment in dev
  mode, with release preference for less-leaky transport
- every state-changing request requires the token
- read-only requests require the token in release builds
- helper rejects remote hosts and wildcard origins
- diagnostics record token presence, never token value

### Source Access Grant

```rust
pub struct SourceAccessGrant {
    pub id: String,
    pub project_id: String,
    pub source_id: String,
    pub source_kind: SourceKind,
    pub display_name: String,
    pub redacted_path: RedactedPath,
    pub content_hash: Option<String>,
    pub file_size: Option<u64>,
    pub mach_o_uuid: Option<String>,
    pub bookmark: Option<SecurityBookmarkRecord>,
    pub privacy_class: SourcePrivacyClass,
    pub scope: SourceGrantScope,
    pub status: SourceGrantStatus,
}
```

Source kinds:

- standalone Mach-O
- universal slice
- dyld shared cache
- dyld subcache
- driver cache
- projected cache image
- kernel collection
- fileset member
- KDK bundle
- dSYM bundle
- panic log
- exported evidence package
- user sidecar

Grant scopes:

- session only
- project persistent
- app persistent

Statuses:

- available
- missing
- access denied
- bookmark stale
- hash changed
- unsupported location
- requires Full Disk Access
- revoked

### Security Bookmark Record

```rust
pub struct SecurityBookmarkRecord {
    pub bookmark_id: String,
    pub created_at: String,
    pub last_resolved_at: Option<String>,
    pub stale: bool,
    pub scope: SourceGrantScope,
    pub original_path_redacted: RedactedPath,
}
```

Rules:

- Workbench owns bookmark creation and repair on macOS
- Rust core never serializes raw bookmark data into human-readable exports
- project files store bookmark data only when the user chooses persistent source
  access
- diagnostics include bookmark status, not bookmark payload

### Diagnostics Policy

```rust
pub struct DiagnosticsPolicy {
    pub include_raw_paths: bool,
    pub include_file_names: bool,
    pub include_hashes: bool,
    pub include_symbol_names: bool,
    pub include_strings: bool,
    pub include_user_comments: bool,
    pub include_raw_bytes: bool,
    pub include_panic_log_text: bool,
    pub include_process_list: bool,
}
```

Default:

- raw paths: false
- file names: true
- hashes: true
- symbol names: true
- strings: false
- user comments: false
- raw bytes: false
- panic log text: false
- process list: false

### Diagnostic Bundle

```rust
pub struct DiagnosticBundle {
    pub id: String,
    pub created_at: String,
    pub app_build: AppBuildIdentity,
    pub helper: Option<HelperIdentity>,
    pub project_summary: Option<ProjectDiagnosticSummary>,
    pub source_summaries: Vec<SourceDiagnosticSummary>,
    pub job_summaries: Vec<JobDiagnosticSummary>,
    pub problem_summaries: Vec<ProblemDiagnosticSummary>,
    pub crash_reports: Vec<CrashReportSummary>,
    pub hang_reports: Vec<HangReportSummary>,
    pub update_state: Option<UpdateState>,
    pub redaction_report: RedactionReport,
}
```

Rules:

- export is explicit user action
- bundle is viewable before sharing
- redaction report lists included data classes
- no raw bytes are included unless user opts in for a single bundle
- each bundle includes exact Damsel version and schema version

### Crash Report Summary

```rust
pub struct CrashReportSummary {
    pub process_kind: ProcessKind,
    pub process_name: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub crashed_thread: Option<String>,
    pub top_frames_redacted: Vec<String>,
    pub project_id: Option<String>,
    pub active_job_id: Option<String>,
    pub redaction: RedactionReport,
}
```

Process kinds:

- workbench
- helper
- analyzer worker
- plugin worker
- debugger bridge
- mcp bridge

### Hang Report Summary

```rust
pub struct HangReportSummary {
    pub process_kind: ProcessKind,
    pub detected_at: String,
    pub duration_ms: u64,
    pub active_route: Option<String>,
    pub active_job_id: Option<String>,
    pub last_progress_at: Option<String>,
    pub queue_depths: Vec<QueueDepthSummary>,
    pub thread_samples_redacted: Vec<String>,
}
```

### Update State

```rust
pub struct UpdateState {
    pub channel: ReleaseChannel,
    pub current_version: String,
    pub latest_known_version: Option<String>,
    pub eligibility: UpdateEligibility,
    pub last_checked_at: Option<String>,
    pub signature_status: UpdateSignatureStatus,
    pub minimum_macos_status: MinimumOSStatus,
}
```

Channels:

- none
- local dev
- internal nightly
- private prerelease
- public stable

Rules:

- update checks are disabled by default in dev builds
- future public builds require signed update archives
- update UI clearly distinguishes available, ineligible by OS, ineligible by
  channel, and failed signature verification

### Plugin Trust Record

```rust
pub struct PluginTrustRecord {
    pub plugin_id: String,
    pub uuid: String,
    pub name: String,
    pub version: String,
    pub scope: PluginScope,
    pub path_redacted: RedactedPath,
    pub signing: SigningState,
    pub notarization: NotarizationState,
    pub capabilities: Vec<PluginCapability>,
    pub loaded: bool,
    pub last_failure: Option<PluginFailureSummary>,
}
```

Rules:

- plugins are disabled until the project kernel, capability model, and helper
  isolation exist
- unsigned native plugins are dev-only
- plugin-produced facts record plugin id, uuid, version, signing state, and
  capability grant
- plugin crash loops cause auto-disable and a recovery UI, following Hopper's
  public changelog lesson

### Debugger Trust Record

```rust
pub struct DebuggerTrustRecord {
    pub enabled: bool,
    pub build_flavor: BuildFlavor,
    pub xcode_tools_status: ToolStatus,
    pub entitlement_status: EntitlementStatus,
    pub sip_status: SipStatus,
    pub target_hardened_runtime: Option<HardenedRuntimeState>,
    pub last_attach_problem: Option<String>,
}
```

Rules:

- debugger features are hidden or disabled in non-debugger builds unless
  developer mode is explicit
- failures become problem records with recovery actions
- remote debugging is a separate feature with separate transport security

## Workbench UI Requirements

### Trust And Permissions Panel

Add a native panel or settings section with:

- app version and build flavor
- helper status and protocol version
- helper executable trust state
- source grants and bookmark status
- diagnostics privacy defaults
- update channel and last check
- plugin trust state when plugins exist
- debugger capability state when debugger exists

Actions:

- restart helper
- reveal helper binary in Finder in dev builds
- revoke source grant
- repair stale bookmark
- open Privacy and Security settings guidance
- export diagnostic bundle
- view redaction policy
- check for update when update channel exists

### First-Run Trust Flow

First launch should explain:

- Damsel analyzes local files.
- Large artifacts can be registered by path rather than uploaded into app
  memory.
- Diagnostics are local and redacted by default.
- The local helper listens only on loopback and uses a session token.
- Experimental debugger, plugin, MCP, and automation features require explicit
  enablement.

This should be concise and native, not a marketing page.

### Source Access Recovery

When a project source is unavailable:

- show source display name and kind
- show redacted path or user-approved full path
- show reason code
- offer Locate File
- offer Repair Bookmark
- offer Continue Without Source
- offer Remove Source From Project only if it does not destroy user work
- offer Full Disk Access guidance only when the reason is specific

### Helper Status UI

Status strip and diagnostics panel should distinguish:

- starting helper
- ready helper
- helper protocol mismatch
- helper signature mismatch
- helper startup timeout
- helper crashed
- helper terminated after ready
- helper bound wrong address
- external helper disallowed

### Crash Recovery UI

On restart after crash:

- show whether Workbench, helper, plugin, analyzer, debugger, or MCP bridge
  crashed
- offer reopen last project
- offer disable suspect plugin or experimental feature
- offer export diagnostics
- never claim user annotations are saved unless project store confirms it

### Update UI

Future update UI should show:

- current version
- channel
- latest available version
- minimum OS requirement
- signature verification status
- release notes link
- "manual download" fallback when auto-update is disabled

No update action should obscure project state or run while an unsafe write-like
project operation is in flight.

## Local Helper Security

### Binding And Authentication

Requirements:

- bind to `127.0.0.1` or `::1`, never `0.0.0.0`
- use random high port or Unix-domain socket where practical
- require session token on requests
- disable external base URL in release builds unless developer mode is active
- validate helper protocol version
- validate helper binary hash in release packaging
- keep helper lifetime tied to Workbench session unless user selects persistent
  background mode later

### API Changes

Add headers:

```text
Authorization: DamselLocal <opaque-session-token>
X-Damsel-Client: workbench
X-Damsel-Protocol-Version: 1
```

Add endpoints:

```text
GET /api/trust/status
GET /api/diagnostics/summary
POST /api/diagnostics/export
GET /api/helper/identity
POST /api/helper/shutdown
GET /api/sources/{source_id}/access
POST /api/sources/{source_id}/grant
POST /api/sources/{source_id}/revoke
```

Compatibility:

- current upload endpoints remain available in dev and compatibility mode
- project-backed local path registration is the default for Workbench once
  source grants exist
- browser demos can use upload mode with explicit size limits

### Process Lifecycle

The helper manager must track:

- launch attempt id
- process id
- executable path
- resolved binary source
- stdout/stderr ring buffers
- last readiness line
- port and bind address
- session id
- termination reason
- restart count

Rules:

- on Workbench quit, terminate helper and wait bounded time
- if helper ignores terminate, show dev diagnostic; release builds can kill
  after a bounded grace period
- prevent orphan helper from accepting stale tokens
- handle old helper from another Damsel version as mismatch, not reuse

## Distribution And Signing

### Build Artifacts

Target artifacts:

- `DamselWorkbench.app`
- bundled `damsel-cli` helper
- optional future updater framework
- optional future plugin helper
- optional future debugger bridge helper
- optional future MCP bridge helper

Each nested executable needs:

- deterministic version metadata
- code signature state
- hardened runtime state when distributed
- entitlements record
- release manifest entry

### Entitlement Matrix

Initial analysis build:

- no debugger entitlement
- no JIT entitlement
- no library validation disabling
- no automation entitlement unless a specific feature requires it
- file access through user selection and security-scoped bookmarks when
  sandboxed

Future debugger build:

- debugger entitlement only if verified necessary
- developer tools/Xcode requirement surfaced in UI
- hardened runtime target detection and failure messages
- no remote debugging transport by default

Future plugin build:

- prefer helper-process plugins before native in-process dynamic loading
- if native plugins exist, require signing for distributed builds
- plugin load policy visible in Trust panel

### Notarization Gates

Release validation should check:

- app bundle signs strictly
- nested helper signs strictly
- hardened runtime enabled for distribution flavor
- notarization submitted and accepted
- stapled ticket on DMG or app where applicable
- Gatekeeper assessment passes on a clean machine
- app launches outside build tree
- helper launches from app bundle
- Workbench can open a fixture without quarantine/translocation failure

### Update Gates

If Sparkle is adopted:

- `SUPublicEDKey` is configured for update-capable builds
- update archives are signed with EdDSA
- appcast feed uses HTTPS
- feed and release-note signing policy is documented
- minimum macOS version is included
- old update files are retained according to channel policy
- key rotation procedure is documented
- failure states are tested with invalid signature and old OS fixtures

## Diagnostics And Redaction

### Diagnostic Bundle Contents

Default bundle:

- app version, build number, build flavor
- helper version, protocol version, launch mode
- OS version and architecture
- project schema version
- source kind summaries
- source sizes and hashes when allowed
- Mach-O UUIDs
- analyzer run ids
- job status summaries
- problem codes and counts
- recent error messages
- bounded helper stdout/stderr logs
- crash summaries
- hang summaries
- performance counters
- redaction policy and redaction report

Excluded by default:

- raw bytes
- raw strings
- full source paths
- user comments
- panic log raw text
- process lists
- environment variables
- session tokens
- bookmark payloads
- external client identifiers beyond redacted category

### Redaction Engine

Implement a shared redaction engine used by:

- diagnostics export
- evidence package export
- crash report summaries
- job failure reports
- MCP/tool activity logs
- plugin crash records
- update failure reports

Rules:

- redaction decisions are deterministic
- every included privacy class is listed
- users can preview a bundle before sharing
- raw opt-in is per bundle, not a persistent global toggle

### Crash Capture

Practical first implementation:

- helper termination reason
- exit code and signal
- active job id
- last route/project id
- recent problem records
- bounded stderr ring buffer

Later implementation:

- Workbench crash marker
- last clean shutdown marker
- OS crash log discovery with user consent
- thread sample for hang reports
- plugin/helper isolation crash records

### Hang Detection

Detect:

- helper startup timeout
- no job progress for configured duration
- Workbench request waiting beyond budget
- plugin worker non-response
- debugger bridge non-response

Report:

- active job
- queue depth
- last progress
- route/project context
- cancellation status
- memory pressure summary

## Source Access And Privacy

### Path Registration Instead Of Upload

Workbench should migrate from `Data(contentsOf:)` whole-file upload to
project-backed source registration:

```text
POST /api/projects/{project_id}/sources/register-path
```

Payload:

```json
{
  "display_name": "dyld_shared_cache_arm64e",
  "path_grant_id": "grant:local:123",
  "source_kind": "dyld_shared_cache",
  "privacy_class": "private_system_artifact",
  "expected_hash": null
}
```

The helper resolves the grant through Workbench-mediated security access when
sandboxed, or through direct path access in local dev mode.

### Protected Locations

Protected locations require specific error classification:

- Desktop
- Documents
- Downloads
- iCloud Drive
- network volumes
- removable volumes
- Full Disk Access protected locations
- `/System/Volumes/Data` derived paths
- developer toolchains
- KDK bundles
- crash and panic log directories

Damsel should ask for user file selection first. Full Disk Access should be a
last-resort recovery path, not a default onboarding step.

### Project Persistence

Project files store:

- source ids
- redacted display names
- content hashes
- UUID/build metadata
- security bookmark payload only if user allows persistent grants
- source privacy classes
- source relationship graph
- grant status

Project files do not store:

- raw bytes by default
- raw panic log text unless imported as project content by user
- diagnostic bundles
- external client tokens
- plugin crash dumps with raw data

## Plugin Trust

### Short-Term Rule

No user-loaded native plugins until:

- project database exists
- source access policy exists
- capability model exists
- diagnostics redaction exists
- helper-process isolation exists
- plugin crash-loop disable exists

### Long-Term Loading Model

Preferred order:

1. out-of-process helper plugins with JSON/Cap'n Proto style contracts
2. WASM or other sandboxed analyzers if performance and language fit are good
3. native dynamic plugins only for carefully versioned, signed, opt-in cases

Native plugins must record:

- signing state
- notarization state if applicable
- scope
- bundle id
- plugin UUID
- declared capabilities
- loaded capabilities
- facts produced
- diagnostics produced
- failures

### Crash Loop Recovery

If plugin startup fails repeatedly:

- disable plugin automatically
- mark project facts from that plugin as stale or unavailable
- preserve user annotations
- show recovery UI
- include redacted plugin failure in diagnostics

## Debugger Trust

### Build Policy

Debugger capability should be unavailable until:

- project/source/address identity is stable
- local helper token model exists
- route-backed breakpoints exist
- debug session records exist
- entitlement matrix is documented
- signed local fixture test exists

### Failure Classification

Debugger failures should distinguish:

- Xcode tools missing
- LLDB unavailable
- target binary missing
- target hash mismatch
- target hardened runtime blocks attach
- SIP or system policy blocks attach
- Damsel build lacks entitlement
- user denied permission
- process exited before attach
- runtime module did not map to project source

### Static-Dynamic Privacy

Debugger diagnostics must not include:

- raw process memory
- full command-line arguments
- environment variables
- user home path
- target strings

Unless the user opts into a single diagnostic bundle.

## Update And Release Policy

### Private First

Initial state:

- no automatic update checks
- local version visible in About and diagnostics
- manual install/rebuild instructions remain outside product UI
- private prerelease manifest can be added later

### Release Manifest

Define a Damsel release manifest even before Sparkle:

```json
{
  "product": "DamselWorkbench",
  "version": "0.3.0",
  "build": "42",
  "channel": "private-prerelease",
  "published_at": "2026-05-26T00:00:00Z",
  "minimum_macos": "14.0",
  "artifacts": [
    {
      "kind": "dmg",
      "url": "https://example.invalid/DamselWorkbench-0.3.0.dmg",
      "length": 0,
      "sha256": "placeholder",
      "ed25519": "placeholder"
    }
  ]
}
```

Do not wire network update checks until:

- channel policy is explicit
- signature verification exists
- privacy policy mentions update checks
- failure UI exists

## Implementation Phases

### Phase T0: Policy And Model Skeleton

Deliverables:

- `TrustPolicy`
- `AppBuildIdentity`
- `HelperIdentity`
- `DiagnosticsPolicy`
- `DiagnosticBundle`
- `RedactionPolicy`
- problem codes for trust failures
- Workbench trust status model

Implementation:

- add Rust model types in `damsel-core`
- add Swift mirror types in `DamselModels.swift`
- add `/api/trust/status` returning static/dev values
- add diagnostics summary endpoint with no raw data
- add app About/Trust panel skeleton

Validation:

- model serde snapshots
- Swift decode tests
- trust endpoint test
- redaction default test

### Phase T1: Local Helper Hardening

Deliverables:

- session token generation
- helper identity endpoint
- helper bind address validation
- protocol version validation
- release-build external URL policy
- bounded stdout/stderr ring buffer
- helper shutdown endpoint

Implementation:

- update `DamselServerProcess` to generate and pass session id/token
- update `DamselAPIClient` to attach auth header
- update CLI UI server to require token in Workbench mode
- reject non-loopback bind addresses
- track helper process termination reason

Validation:

- missing-token request fails
- wrong-token request fails
- correct-token request succeeds
- helper identity snapshot
- external base URL disallowed in release-mode test
- startup timeout diagnostic test

### Phase T2: Source Access Grants

Deliverables:

- `SourceAccessGrant`
- `SecurityBookmarkRecord`
- source privacy classes
- source availability statuses
- path registration endpoint
- stale bookmark repair UI
- upload compatibility size limit

Implementation:

- add Workbench file-selection grant store
- add project source registration by path
- add security-scoped bookmark support if/when sandboxed
- add missing-source and access-denied problem records
- keep current upload endpoint for compatibility

Validation:

- source grant snapshot
- missing source fixture
- hash-changed source fixture
- stale bookmark fixture
- upload compatibility test
- large source does not load into Swift `Data` path in project mode

### Phase T3: Diagnostics Export And Redaction

Deliverables:

- diagnostic bundle endpoint
- redaction engine
- preview UI
- redaction report
- helper log ring buffer
- problem/job summaries

Implementation:

- collect project/job/problem summaries
- redact paths/strings/comments/raw bytes by default
- export bundle as JSON and zip later if needed
- add Workbench export sheet

Validation:

- diagnostics default excludes raw paths
- diagnostics default excludes raw strings
- opt-in raw class is per bundle
- redaction report lists included classes
- malformed project diagnostic still exports

### Phase T4: Crash And Hang Recovery

Deliverables:

- clean shutdown marker
- helper crash summary
- active job crash marker
- hang detection for helper startup and stalled jobs
- crash recovery UI

Implementation:

- record last active project/session
- record last helper termination
- sample job progress timestamps
- add recovery card on next launch

Validation:

- simulated helper crash produces recovery state
- stalled job produces hang problem
- recovery does not claim unsaved annotations were saved
- diagnostics include crash summary without raw bytes

### Phase T5: Signing And Release Validation

Deliverables:

- release build script or documented Xcode archive path
- entitlements file for analysis build
- release manifest
- signing validation command
- notarization checklist
- clean-machine launch checklist

Implementation:

- add app bundle metadata
- embed helper version metadata
- sign app and helper in release path
- add CI/manual validation script where secrets permit

Validation:

- codesign verify passes for app and helper
- helper launches from app bundle
- Gatekeeper assessment documented for distributed artifact
- diagnostics show signed/notarized/hardened states when available

### Phase T6: Update Channel

Deliverables:

- `UpdateState`
- manual release manifest check
- no-auto-check default
- Sparkle design decision if adopted
- signature failure UI

Implementation:

- add update state model and settings UI
- support local/private manifest parsing
- defer Sparkle integration until distribution channel exists

Validation:

- invalid signature fixture
- minimum macOS ineligible fixture
- channel disabled fixture
- update check failure creates typed problem

### Phase T7: Debugger Entitlement Readiness

Deliverables:

- debugger trust record
- Xcode tools detection
- entitlement status detection
- target hardened-runtime problem codes
- signed fixture executable plan

Implementation:

- add read-only checks before LLDB integration
- expose disabled debugger reasons in UI
- keep debugger bridge behind feature flag/build flavor

Validation:

- missing tools problem
- entitlement missing problem
- target missing problem
- no debugger UI action in analysis build

### Phase T8: Plugin Trust Readiness

Deliverables:

- plugin manifest schema
- plugin trust record
- capability grants
- crash-loop disable design
- signed plugin policy

Implementation:

- do not load native plugins yet
- add schema and tests
- add Trust panel placeholder for extension policy

Validation:

- unsigned plugin denied in release policy
- dev plugin allowed only in dev policy
- plugin fact provenance snapshot
- simulated plugin crash-loop disable

## API Contract Additions

### Trust Status

```json
{
  "app": {
    "version": "0.3.0",
    "build": "dev",
    "flavor": "dev",
    "minimum_macos": "14.0",
    "signing": "ad_hoc",
    "notarization": "not_applicable",
    "hardened_runtime": "not_checked"
  },
  "helper": {
    "version": "0.3.0",
    "protocol_version": 1,
    "bind_address": "127.0.0.1",
    "session_present": true
  },
  "privacy": {
    "diagnostics_include_raw_paths": false,
    "diagnostics_include_strings": false,
    "diagnostics_include_raw_bytes": false
  }
}
```

### Source Grant

```json
{
  "grant_id": "grant:1",
  "source_kind": "dyld_shared_cache",
  "display_name": "dyld_shared_cache_arm64e",
  "redacted_path": "~/.../dyld_shared_cache_arm64e",
  "scope": "project_persistent",
  "status": "available",
  "privacy_class": "private_system_artifact"
}
```

### Diagnostic Export Request

```json
{
  "project_id": "project:1",
  "include_classes": [
    "file_names",
    "hashes",
    "symbol_names"
  ],
  "exclude_classes": [
    "raw_paths",
    "strings",
    "raw_bytes",
    "user_comments"
  ]
}
```

### Typed Trust Problem

```json
{
  "id": "problem:trust:7",
  "severity": "error",
  "code": "helper_signature_mismatch",
  "message": "The local Damsel helper does not match the Workbench build.",
  "target": {
    "id": "helper:current",
    "kind": "helper"
  },
  "recovery": "Restart the helper from the current app bundle.",
  "details": {
    "expected_protocol_version": 1,
    "actual_protocol_version": 1
  }
}
```

## Current-Code Integration Map

### `DamselServerProcess.swift`

Needed changes:

- generate session token
- pass token and protocol version to helper
- store helper identity after readiness
- validate loopback URL
- reject remote URL in release builds
- capture bounded stdout/stderr
- record termination reason
- expose restart/shutdown actions

### `DamselAPIClient.swift`

Needed changes:

- auth header injection
- trust status endpoint
- diagnostics endpoint
- source grant endpoints
- project path-registration endpoint
- typed trust/problem errors
- response redaction hooks for logs

### `WorkbenchStore.swift`

Needed changes:

- split helper state into `HelperStore`
- split trust state into `TrustStore`
- split source grants into `SourceAccessStore`
- split diagnostics into `DiagnosticStore`
- avoid storing raw file paths in broad app state beyond the source access
  layer
- make busy state job-specific

### `DamselModels.swift`

Needed models:

- `TrustStatusResponse`
- `AppBuildIdentity`
- `HelperIdentity`
- `SourceAccessGrant`
- `DiagnosticsPolicy`
- `DiagnosticBundleSummary`
- `UpdateState`
- `PluginTrustRecord`
- `DebuggerTrustRecord`
- typed trust problem enums

### Rust CLI UI Server

Needed changes:

- Workbench-mode token requirement
- helper identity endpoint
- trust status endpoint
- diagnostics summary/export endpoint
- path registration endpoint
- redaction engine
- source access statuses
- local bind enforcement

### Project Runtime

Needed changes:

- source grants become part of project metadata
- project open checks grant status
- missing/denied/stale sources become problems
- diagnostics summarize project state without raw data
- analysis facts record build/trust policy version

## Validation Matrix

| Requirement | Fixture/Test | Evidence |
| --- | --- | --- |
| Helper requires token | UI API test | wrong/missing token returns typed denial |
| Helper binds loopback only | server startup test | non-loopback bind rejected |
| Helper protocol checked | Swift unit test | mismatch becomes `helper_version_mismatch` |
| Source grants model | serde snapshot | grant status and privacy class preserved |
| Bookmark stale recovery | Swift store test | stale bookmark creates repair action |
| Diagnostics redaction | Rust and Swift tests | raw path/string/bytes absent by default |
| Crash summary | simulated helper exit | crash record includes active job without raw data |
| Hang summary | stalled job test | stalled job creates hang problem |
| Signing state | release validation script | codesign/notarization state recorded |
| Update signature | manifest fixture | invalid update is rejected |
| Plugin trust | policy fixture | unsigned native plugin denied outside dev |
| Debugger trust | fixture executable plan | entitlement/tool failures are typed |

## Non-Goals For First Implementation

- no automatic update framework integration
- no public update feed
- no native plugin loading
- no remote debugging
- no automatic Full Disk Access prompt
- no upload of diagnostic bundles to any server
- no raw byte crash dumps
- no in-place mutation of system binaries, shared caches, or kernel collections
- no claim of notarized distribution until a notarized artifact is produced

## Completion Criteria

This subsystem is implementation-ready when:

- all core model types are represented in Rust and Swift
- local helper authentication is designed into API clients and server routes
- source access grants replace whole-file upload for Workbench project mode
- diagnostics export has default redaction tests
- helper crash/hang summaries exist
- trust and permissions UI has a concrete view model
- signing/notarization/update policy is documented with gates
- debugger and plugin trust are explicitly deferred behind build flavor and
  capability checks
- research sources are cross-linked from the docs pack

This subsystem is implemented only when:

- token enforcement tests pass
- source grant tests pass
- diagnostics redaction tests pass
- helper lifecycle tests pass
- Workbench can open a local fixture through project source registration
- a diagnostic bundle preview excludes raw paths, strings, comments, and bytes
  by default
- release validation proves signing/notarization claims for a real artifact if
  such claims are made

## Source Links

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper download page: <https://www.hopperapp.com/download.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper release API: <https://www.hopperapp.com/include/files-api.php?request=releases&public=true>
- Hopper Debugger Server: <https://www.hopperapp.com/HopperDebuggerServer/>
- Hopper SDK 6.3.0: <https://www.hopperapp.com/downloader/public/HopperSDK-6.3.0.zip>
- Desktop Insights Hopper package inspection:
  <https://desktopinsights.vercel.app/apps/hopper-disassembler>
- Apple app code signing process:
  <https://support.apple.com/guide/security/app-code-signing-process-sec3ad8e6e53/web>
- Apple Gatekeeper and runtime protection:
  <https://support.apple.com/guide/security/gatekeeper-and-runtime-protection-sec5599b66df/web>
- Apple file access controls:
  <https://support.apple.com/guide/security/controlling-app-access-to-files-secddd1d86a6/web>
- Sparkle documentation: <https://sparkle-project.org/documentation/>
