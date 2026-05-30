# Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec

Status: implementation-grade product and architecture specification. This is
not an implementation claim.

Purpose: define the command-routing, keyboard, navigation, focus, selection,
history, transaction, undo, and native macOS document behavior Damsel needs in
order to feel competitive with Hopper Disassembler for the Apple systems niche.
This spec turns scattered requirements from the Hopper-gap pack into one
implementable subsystem plan.

Primary Damsel audience:

- native Workbench maintainers
- Rust project-kernel maintainers
- API and CLI contract maintainers
- future automation and MCP maintainers
- test and benchmark maintainers

Primary user audience:

- macOS and iOS reverse engineers
- Apple systems researchers
- XNU, driver, kext, and dyld shared-cache analysts
- Objective-C, C++, Objective-C++, Swift-adjacent, and arm64e researchers
- maintainers who need to compare Apple OS builds and preserve evidence

## Companion Documents

Core companions:

- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Native App Polish Plan](native-app-polish-plan.md)
- [Competitive UX And App Feature Plan](competitive-ux-app-plan.md)
- [Search, Navigation, And Index Plan](search-navigation-index-plan.md)
- [Project Kernel And Typed Byte Map Implementation Spec](project-kernel-byte-map-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Patching, Hex, Assembly, And Transaction Implementation Spec](patching-hex-assembly-transaction-implementation-spec.md)
- [Automation, MCP, And Scripting Implementation Spec](automation-mcp-scripting-implementation-spec.md)
- [Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec](preferences-keymaps-layout-theming-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Refactor And Subsystem Plan](refactor-subsystem-plan.md)
- [Quality And Validation Plan](quality-validation-plan.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)

Research companions:

- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Hopper SDK And Document Architecture Research](hopper-sdk-and-document-architecture-research.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)
- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)

## Public Hopper Evidence Used

This spec is grounded in Hopper's public product and tutorial surface, plus the
Hopper-gap pack's previous SDK/header research.

Official public sources:

- <https://www.hopperapp.com/index.html>
- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/faq.html>
- <https://www.hopperapp.com/download>
- <https://www.hopperapp.com/HopperDebuggerServer/>

Relevant public claims and workflows:

- Hopper markets a fast native Cocoa interface with keyboard shortcuts and
  native macOS integration.
- Hopper presents assembly, control-flow graph, pseudocode, and hex views as
  synchronized representations of the same document.
- Hopper's tutorial describes a left symbol/string pane, a center
  representation workspace, and a right context-sensitive inspector.
- Hopper's tutorial describes byte-type transformations available from toolbar
  buttons and keyboard shortcuts, with undo and redo.
- Hopper's tutorial describes graph cursor movement with arrow keys, graph
  minimap behavior, graph node grouping, graph node colors, and graph text
  editing.
- Hopper's tutorial describes a navigation stack, keyboard back navigation, and
  toolbar back/forward items.
- Hopper's tutorial describes a navigation bar that shows byte categories and
  cursor position across the file.
- Hopper's tutorial describes inspector modules for instruction encoding,
  operand formatting, comments, colors, tags, references, and procedures.
- Hopper's tutorial and FAQ describe hex editing, assembler invocation, NOP
  region generation, and undoable modifications.
- Hopper's homepage describes CFG PDF export, graph annotations, pseudocode,
  Objective-C selector/message extraction, Swift name decoding, local debugger
  support, scripting, MCP, and SDK extensibility.

SDK evidence already inspected elsewhere in this pack:

- `HPDocument` exposes current address, selected range, cursor movement,
  navigation-stack operations, background process state, wait/cancel behavior,
  UI messages, alerts, and virtual-address reads and writes.
- `HPDisassembledFile` exposes names, comments, byte types, operand formats,
  colors, tags, problems, procedure creation/removal, assembler helpers, NOP
  helpers, type database operations, and undo/redo transactions.
- `HPProcedure` exposes procedure-local details such as basic blocks, callers,
  callees, local labels, register aliases, stack facts, method signatures,
  calling conventions, and tags.

Inference:

- Hopper's public surface implies a document command system even where public
  pages do not name one directly. Menus, shortcuts, toolbar buttons, inspector
  controls, graph gestures, Python scripts, SDK plugins, MCP tools, and debugger
  overlays all have to mutate or navigate a shared document/session model.
- Damsel should not mimic Hopper by adding isolated menu items. It needs a
  typed command infrastructure shared by native UI, CLI, automation, and future
  plugins.

## Current Damsel Workbench Evidence

Repo files inspected for this spec:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift`
- `damsel-app/Sources/DamselAppCore/Support/DamselCommands.swift`
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift`
- `damsel-app/Sources/DamselAppCore/Views/ContentView.swift`
- `damsel-app/Sources/DamselAppCore/Views/InstructionSurfaceView.swift`
- `damsel-app/Sources/DamselAppCore/Views/ControlPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/InspectorPane.swift`
- `damsel-app/Sources/DamselAppCore/Views/VirtualizedInstructionTable.swift`

Current strengths:

- AppKit entry point creates a real macOS app shell.
- Main menu already includes File, Edit, Analyze, Navigate, Window, and Help.
- File menu supports Open Binary and Open Recent.
- Analyze menu supports Disassemble, Refresh Disassembly, and Cancel Analysis.
- Navigate menu supports Find in Instructions and Clear Instruction Filter.
- Help menu supports Copy Diagnostics.
- Edit menu exposes generic Undo and Redo selectors plus copy commands.
- Menu validation gates some commands through `WorkbenchStore` state.
- `InstructionSurfaceView` listens for notification-based focus and clear
  filter commands.
- `WorkbenchStore` can load a file, run disassembly, cancel work, track
  selected instruction address, filter instruction rows, and produce
  diagnostics.

Current gaps:

- `DamselCommands.swift` is only a small set of `Notification.Name` constants.
- Menu actions are one-off AppKit selectors instead of typed command
  descriptors.
- There is no global command registry.
- There is no command palette.
- There is no command metadata schema for title, shortcut, enablement,
  authorization, route preconditions, mutation kind, undo behavior, latency
  budget, or audit policy.
- Swift menu validation reaches directly into `WorkbenchStore`.
- The app has generic Undo and Redo menu items, but no project transaction
  stack behind them.
- Selection is an optional instruction address rather than a canonical route.
- Navigation history is not persisted.
- There is no back/forward stack.
- There is no branchable jump history.
- There is no route repair when analysis objects change.
- Focus changes are notification broadcasts rather than command results.
- Jobs are represented primarily as busy state and status text.
- Current commands are not exposed to CLI JSON, MCP, automation, or plugin
  surfaces.
- Commands do not yet understand dyld shared-cache images, kernel collections,
  KDK/dSYM sidecars, panic frames, ObjC selectors, C++ objects, or XNU runtime
  routes.

## Product Thesis

Navigation is not a secondary feature in a reverse-engineering app. Navigation
is the product loop:

1. Notice a string, selector, class, symbol, xref, graph edge, panic frame, or
   suspicious value.
2. Jump to it.
3. Inspect context.
4. Mark up the document.
5. Return to the previous place.
6. Branch to the next related object.
7. Preserve the path as evidence.

Hopper is competitive partly because it treats this loop as an interactive
native document experience: shortcuts work, views stay synchronized, the
inspector follows the cursor, graph and hex are editable, navigation has a
stack, user edits are undoable, and analysis can continue while the user works.

Damsel's Apple systems niche raises the bar:

- A single action might jump from a dyld cache import to a provider image.
- A panic frame route might resolve through a KDK symbol profile.
- An Objective-C selector might map to multiple implementations across cache
  images and categories.
- A C++ virtual call might need a vtable, class hierarchy, and IOKit family
  context.
- A kernel collection route might identify both the fileset member and the
  global runtime address.
- A debug-info source-line route might depend on dSYM/KDK matching.

Therefore Damsel needs command and navigation infrastructure that is richer
than address jumps.

## Non Goals

This spec does not implement:

- a full decompiler
- binary patching enablement for shared caches or kernel collections
- live debugging
- third-party plugin ABI stability
- cloud sync
- collaborative editing

This spec does define command and transaction infrastructure that future
decompiler, patching, debugger, and plugin features will need.

## Desired User Feel

The user should feel that every operation has a predictable place:

- menus reveal the available product surface
- keyboard shortcuts are discoverable and reliable
- command palette finds both commands and objects
- context menus show object-specific actions
- disabled commands explain why they are disabled
- navigation never loses the previous route
- back and forward work across all representations
- selection is preserved when switching assembly, graph, hex, metadata, source,
  and summary views
- comments, names, tags, bookmarks, and manual facts are undoable
- long actions show progress, support cancellation, and return partial results
- command results can be reproduced by CLI JSON or automation where safe
- errors attach to routes and commands instead of appearing as vague banners

## Architecture Overview

Target layers:

```text
Native AppKit and SwiftUI shell
  Main menu builder
  Toolbar controller
  Command palette
  Context menu builders
  Accessibility command exposure

Swift Workbench state
  CommandRegistry
  CommandDispatcher
  CommandContext
  KeymapStore
  FocusStore
  SelectionStore
  RouteStore
  NavigationStore
  TransactionStore
  ActivityStore
  JobStore
  ProblemStore

Local UI API
  command introspection
  route resolution
  navigation state persistence
  transaction creation
  undo and redo
  job dispatch and cancellation
  command activity log

Rust project kernel
  AnalysisProject
  ProjectSession
  RouteResolver
  SearchIndex
  AnnotationStore
  TransactionLog
  JobScheduler
  ProblemIndex
  AuditLog
```

Core rule:

- UI surfaces do not directly mutate analysis state.
- UI surfaces invoke typed commands.
- Commands resolve preconditions against a typed context.
- Mutating commands create transactions.
- Transactions update project facts and UI session state through events.
- Undo and redo replay inverse project transactions where safe.

## Command Model

### Command Descriptor

Every command has a descriptor.

```text
CommandDescriptor {
  id
  stable_name
  title
  menu_title
  palette_title
  short_description
  long_description
  group
  subgroup
  default_shortcuts
  alternate_shortcuts
  allowed_surfaces
  context_requirements
  route_requirements
  selection_requirements
  source_requirements
  privilege_requirements
  mutation_kind
  undo_policy
  job_policy
  latency_budget
  audit_policy
  privacy_policy
  availability_policy
  result_kind
}
```

Descriptor fields should be stable enough for:

- menu construction
- command palette search
- keyboard shortcut display
- automation discovery
- documentation generation
- test snapshots
- accessibility labels
- plugin manifest validation
- user keymap customization

### Command Ids

Command ids should be namespaced:

```text
app.open_binary
app.open_project
project.save
project.attach_dsym
project.attach_kdk
analysis.run_default
analysis.cancel_current
navigate.back
navigate.forward
navigate.goto_address
navigate.goto_symbol
navigate.goto_route
navigate.show_xrefs
navigate.show_callers
navigate.show_callees
selection.copy_address
selection.copy_instruction
selection.copy_route
annotation.set_comment
annotation.rename_symbol
annotation.toggle_bookmark
annotation.apply_tag
byte.convert_to_code
byte.convert_to_data
byte.convert_to_string
procedure.create
procedure.rename_local_label
procedure.rename_register
objc.show_selector_implementations
objc.show_message_senders
cache.jump_to_provider_image
kernel.attach_panic_log
debug.map_runtime_address
automation.copy_reproduction_command
```

Rules:

- ids are lowercase ASCII
- ids are never localized
- ids are stable across releases unless explicitly deprecated
- command id changes require compatibility aliases
- plugin-provided ids use a reverse-DNS or package namespace

### Command Groups

Top-level groups:

- Application
- File
- Project
- Analyze
- Navigate
- Search
- View
- Selection
- Annotation
- Type
- Procedure
- Graph
- Hex
- ObjC
- C++
- Swift
- Dyld Cache
- Kernel
- Debug Info
- Debugger
- Automation
- Tools
- Help

Why groups matter:

- menus need command placement
- command palette needs ranking
- keymap editor needs organization
- tests need coverage categories
- plugin capabilities need boundaries

### Allowed Surfaces

A command declares where it can appear:

```text
allowed_surfaces:
  - main_menu
  - toolbar
  - command_palette
  - context_menu
  - inspector
  - navigator
  - representation_overlay
  - cli
  - mcp
  - plugin
  - test_driver
```

Examples:

- `app.open_binary`: main menu, toolbar, command palette, test driver
- `annotation.set_comment`: main menu, command palette, context menu,
  inspector, CLI, MCP with permission
- `hex.patch_bytes`: hidden until write-mode design is enabled
- `debug.attach_process`: debugger build only, explicit entitlement gate

### Context Requirements

Commands are enabled by context.

Context dimensions:

- app lifecycle state
- helper/server state
- project loaded
- project dirty
- selected source kind
- selected route kind
- selected representation
- selected byte range
- selected instruction
- selected function/procedure
- selected graph node
- selected ObjC object
- selected C++ object
- selected cache image
- selected kernel member
- active debug session
- active job state
- write-mode enabled
- trust/permission state

Context requirements must be structured, not arbitrary closures, because:

- menus need cheap validation
- command palette needs ranking and disabled reasons
- automation needs machine-readable preconditions
- tests need stable snapshots
- plugins need manifest-time validation

### Disabled Reasons

Every disabled command should be able to explain itself.

Examples:

```text
No project is open.
The selected route is not a procedure.
Analysis is still indexing xrefs for this image.
This source is read-only.
Attach a matching dSYM before opening source lines.
Open a dyld shared cache before jumping to provider images.
Debugger support is not enabled in this build.
The selected route was removed by a newer transaction.
```

Disabled reasons appear in:

- menu item tooltips where available
- command palette secondary text
- status strip
- accessibility descriptions
- diagnostics
- tests

## Command Execution Model

### Command Context

`CommandContext` is the snapshot used to evaluate and execute a command.

```text
CommandContext {
  app_id
  window_id
  document_id?
  project_id?
  session_id?
  active_route?
  active_representation?
  selection?
  focus_owner?
  visible_source?
  loaded_indexes
  active_jobs
  trust_state
  user_intent
  invocation_surface
  timestamp
}
```

Rules:

- context is captured before execution
- context carries route ids, not raw row indexes
- context includes invocation surface for audit and privacy rules
- context can be serialized for tests
- context does not include raw private paths unless explicitly needed

### Command Result

```text
CommandResult {
  command_id
  status
  user_message?
  route_delta?
  selection_delta?
  focus_delta?
  transaction_id?
  job_id?
  problem_ids
  activity_id
  refreshed_queries
  disabled_reason?
  recoverable_actions
}
```

Result statuses:

- completed
- accepted_job
- cancelled
- no_op
- disabled
- failed_recoverable
- failed_unrecoverable
- requires_confirmation
- requires_permission

### Command Dispatcher

Responsibilities:

- validate command id
- capture command context
- evaluate availability
- request confirmation where required
- dispatch local-only UI commands
- dispatch API-backed project commands
- create activity entries
- connect jobs to progress UI
- update route, selection, and focus stores
- register undo operations
- publish command-completed events

The dispatcher is the only object that should bridge menu selectors,
notifications, toolbar buttons, command palette entries, context menus,
keyboard shortcuts, and automation calls.

### Command Handler Kinds

Handler kinds:

- `ui_local`: changes only focus, panes, sort order, filters, or window state
- `session`: changes route, representation, selection, split, or history
- `project_read`: queries facts but does not mutate project state
- `project_transaction`: mutates annotations or manual analysis facts
- `analysis_job`: starts, stops, reprioritizes, or cancels a job
- `external_intent`: opens Finder, copies to pasteboard, exports a report, or
  requests debugger permission

Examples:

- `navigate.back`: session
- `view.show_graph`: session
- `selection.copy_address`: external_intent
- `annotation.set_comment`: project_transaction
- `analysis.run_default`: analysis_job
- `project.attach_dsym`: project_transaction plus analysis_job

## Native Menu Architecture

### Current Problem

Today the app builds menus manually in `DamselWorkbenchApp.swift`. This is fine
for the current alpha app, but it does not scale to a Hopper-class surface.

Problems:

- menu item title, shortcut, target, validation, and action are duplicated
- adding a command requires AppKit selector work and store validation changes
- no command palette can reuse the menu definitions
- disabled explanations are unavailable or ad hoc
- plugin and automation command discovery cannot reuse native menu data
- undo/redo cannot know which project transaction a menu action created

### Target Menu Builder

The menu builder consumes command descriptors.

```text
MenuSpec {
  title
  items:
    - command_id
    - separator
    - submenu
}
```

Menu construction flow:

1. `CommandRegistry` loads core descriptors.
2. `MenuLayoutProvider` selects descriptors for each menu.
3. `MainMenuController` builds `NSMenuItem` objects.
4. Each `NSMenuItem` stores a command id in `representedObject`.
5. All menu actions call one selector: `performRegisteredCommand:`.
6. Menu validation asks `CommandDispatcher` for availability.
7. Tooltips and accessibility descriptions come from disabled reasons.

### Menu Layout

Application menu:

- About Damsel Workbench
- Settings
- Services
- Hide
- Hide Others
- Show All
- Quit

File menu:

- Open Binary
- Open Shared Cache
- Open Kernel Collection
- Open Project
- Open Recent
- Close Project
- Save Project
- Save Project As
- Attach dSYM
- Attach KDK
- Attach Panic Log
- Export Selection
- Export Evidence Package
- Reveal Source In Finder
- Copy Source Path
- Close Window

Edit menu:

- Undo
- Redo
- Cut
- Copy
- Paste
- Select All
- Copy Selected Address
- Copy Selected Instruction
- Copy Selected Row
- Copy Route
- Copy Reproduction Command
- Set Comment
- Rename Symbol
- Toggle Bookmark
- Apply Tag

Analyze menu:

- Analyze Project
- Analyze Selected Source
- Analyze Selected Function
- Reanalyze Current Route
- Pause Analysis
- Cancel Current Job
- Cancel All Jobs
- Show Analysis Jobs
- Show Problems
- Rebuild Indexes
- Refresh Derived Facts

Navigate menu:

- Back
- Forward
- Jump To Address
- Jump To Symbol
- Jump To Route
- Show Current Route In Navigator
- Show Segment List
- Show Section List
- Show Function List
- Show String List
- Show Xrefs
- Show Callers
- Show Callees
- Show Recent Routes
- Show Bookmarks
- Show Tags

View menu:

- Assembly
- Graph
- Hex
- Pseudocode Summary
- Metadata
- Source Lines
- Feature Map
- Inspector
- Navigator
- Problems Drawer
- Jobs Drawer
- Activity Log
- Increase Text Size
- Decrease Text Size
- Reset Workspace Layout

Modify menu:

- Convert To Code
- Convert To Data
- Convert To String
- Convert To Undefined
- Create Procedure
- Adjust Procedure Bounds
- Add Manual Reference
- Remove Manual Reference
- Rename Local Label
- Rename Register
- Set Operand Format
- Set Byte Color
- Group Graph Nodes
- Ungroup Graph Nodes
- Edit Graph Node Text
- Patch Bytes
- Assemble Instruction
- NOP Region

Apple menu extensions:

- ObjC Selector Implementations
- ObjC Message Senders
- ObjC Class Browser
- C++ Type Browser
- IOKit Class Browser
- Dyld Cache Image Map
- Jump To Provider Image
- Kernel Collection Members
- Attach Panic Log
- Symbolicate Panic Frame

Help menu:

- Damsel Help
- Command Reference
- Copy Diagnostics
- Export Support Bundle
- Show Privacy And Source Access
- Show Keyboard Shortcuts

### Why A Modify Menu Matters

Hopper exposes file and analysis modification as a clear product area. Damsel
should eventually do the same, even while most write-like features remain
disabled.

Early Modify menu entries can be read-mostly or annotation-only:

- Rename Symbol
- Set Comment
- Apply Tag
- Toggle Bookmark
- Add Manual Reference
- Set Operand Format
- Convert Byte Type

Binary patching entries should remain disabled until the patching spec's
copy-on-write revision model exists.

## Keyboard Model

### Requirements

Damsel needs keyboard behavior that is predictable across:

- menus
- command palette
- navigator
- assembly table
- graph canvas
- hex view
- inspector
- source lines
- problems drawer
- jobs drawer
- search results

The current app has only a few shortcuts:

- Command-O for Open Binary
- Command-Return for Disassemble
- Command-R for Refresh Disassembly
- Command-period for Cancel Analysis
- Command-F for Find in Instructions
- Command-Shift-F for Clear Instruction Filter
- Command-Option-D for Copy Diagnostics

This is a reasonable start, but Hopper-class use requires a complete keymap.

### Default Shortcut Philosophy

Rules:

- keep macOS standard shortcuts for file, edit, help, and window behavior
- use Command for app-level actions
- use Option for alternate route actions
- use Shift for reverse direction or broader scope
- use single-letter non-command shortcuts only when the focus owner is a
  representation that can safely interpret them
- avoid stealing text-editing shortcuts from search fields and inspector forms
- allow keymap customization later through descriptors

### Proposed Global Shortcuts

```text
Command-O              Open
Command-S              Save project
Command-Shift-S        Save project as
Command-W              Close window
Command-Z              Undo
Command-Shift-Z        Redo
Command-F              Focus global search or active search
Command-Shift-F        Clear active filter
Command-L              Jump to address or route
Command-P              Command palette
Command-Option-P       Show problems
Command-Option-J       Show jobs
Command-Option-D       Copy diagnostics
Command-Return         Analyze selected scope
Command-period         Cancel current job
Command-left           Navigate back
Command-right          Navigate forward
Command-1              Assembly
Command-2              Graph
Command-3              Hex
Command-4              Metadata
Command-5              Source lines
Command-0              Reset representation split
```

### Representation-Scoped Shortcuts

Assembly focus:

```text
Return                 Follow route under cursor
Escape                 Navigate back
Backspace              Navigate back
Space                  Toggle bookmark
N                      Rename symbol or local label
;                      Set comment
X                      Show xrefs
C                      Convert to code
D                      Convert to data
A                      Convert to string
U                      Convert to undefined
G                      Jump to address
Tab                    Cycle inspector module focus
```

Graph focus:

```text
Arrow keys             Move between graph nodes
Return                 Enter selected node
Escape                 Navigate back or leave node edit
Space                  Toggle bookmark for node route
G                      Group selected nodes
Shift-G                Ungroup selected nodes
;                      Set node comment
N                      Rename node-local label
F                      Fit graph
```

Hex focus:

```text
Arrow keys             Move byte cursor
Shift-arrow            Extend byte selection
Return                 Navigate to owning route
Escape                 Navigate back
Command-E              Edit bytes, disabled until write mode
```

Navigator focus:

```text
Return                 Open selected object route
Right                  Expand group or open child
Left                   Collapse group or go to parent
Command-F              Filter current navigator
Option-Return          Open route in split
```

Command palette focus:

```text
Return                 Run selected command or open route
Command-Return         Run without closing when supported
Escape                 Close palette
Tab                    Cycle result category
```

### Keymap Store

`KeymapStore` should load:

- built-in defaults
- platform defaults
- user overrides
- plugin-provided suggested shortcuts
- disabled shortcut list

Conflict handling:

- direct conflicts are detected at startup
- text-input reserved shortcuts cannot be assigned globally
- representation-scoped shortcuts can overlap if focus owners are distinct
- command palette displays conflicts in diagnostics

Persistence:

- keymap preferences are app preferences, not project facts
- project-specific shortcuts are out of scope

## Focus Model

### Current Problem

Focus is currently handled by notifications for the instruction filter. That
works for a narrow alpha flow, but it will become fragile once the app has
multiple navigators, drawers, graph, hex, inspector fields, and command palette.

### Target Focus Store

```text
FocusTarget {
  window_id
  region
  route?
  element_kind
  field_id?
}
```

Regions:

- toolbar
- navigator
- assembly
- graph
- hex
- source
- inspector
- feature_map
- problems
- jobs
- command_palette
- modal_sheet

Focus transitions:

- commands request focus changes through `CommandResult.focus_delta`
- UI accepts or rejects focus based on live view availability
- rejected focus changes produce a recoverable message
- focus changes are logged in debug diagnostics when keyboard tests run

### Focus And Text Input

Rules:

- text fields own ordinary text-editing keys
- representation shortcuts are disabled when an editable text field is focused
- Escape closes transient overlays before navigating back
- global shortcuts that mutate project state require explicit command routing
- inspector edits use transactions when committed, not on every keystroke

### Accessibility

Focus targets need accessibility names:

- "Navigator, Functions"
- "Assembly row, address 0x100003f20"
- "Graph node, basic block 3"
- "Hex byte range, 8 bytes"
- "Inspector, References"
- "Problems drawer"
- "Command palette"

Keyboard-only users should be able to:

- open a project
- run analysis
- search objects
- navigate to a result
- switch representations
- inspect xrefs
- add a comment
- undo the comment
- export selected evidence

## Selection Model

### Current Problem

`WorkbenchStore` currently stores `selectedInstructionAddress: UInt64?`. That
is not enough for Hopper-level workflows.

Missing selection targets:

- source
- segment
- section
- byte range
- instruction
- basic block
- graph node
- graph edge
- function/procedure
- symbol
- string
- xref
- ObjC class
- ObjC method
- ObjC selector
- C++ class
- vtable
- virtual method slot
- Swift symbol
- dyld cache image
- cache import/export/reexport
- kernel collection member
- kext
- syscall
- MIG routine
- panic frame
- source file
- source line
- problem
- job
- activity entry

### Target Selection

```text
Selection {
  primary_route
  anchor_route?
  range?
  representation
  granularity
  source_revision
  timestamp
}
```

Granularity values:

- point
- byte_range
- instruction_range
- basic_block
- graph_node_set
- function
- object
- list_row
- search_result
- source_line_range

Rules:

- selection is always route-backed when possible
- list row indexes are not persisted
- selection carries source revision when byte-level content matters
- selection can be copied as a route
- selection can be converted to commands
- multi-selection is supported for tags, bookmarks, graph grouping, and export

### Selection Synchronization

When selection changes in one representation:

- assembly scrolls to route
- graph highlights owning node
- hex highlights byte range
- feature map moves cursor
- inspector refreshes context modules
- navigator highlights object when applicable
- status strip updates address/source summary

Loop prevention:

- selection events carry an origin
- stores ignore echo events from their own origin
- selection update coalescing prevents flicker
- route resolution uses version stamps

## Route Model

### Route Shape

Routes should follow the broader search/navigation plan.

```text
Route {
  project_id
  route_id
  kind
  target_id
  source_id?
  image_id?
  member_id?
  address_space?
  vmaddr?
  file_offset?
  representation?
  subrange?
  symbolication_profile_id?
  source_revision?
  derivation?
}
```

Important route kinds:

- project
- source
- segment
- section
- byte_range
- address
- instruction
- basic_block
- procedure
- graph_node
- graph_edge
- symbol
- string
- xref
- objc_class
- objc_method
- objc_selector
- objc_message_send
- cpp_type
- cpp_vtable
- swift_symbol
- cache_image
- cache_symbol_provider
- kernel_collection
- kernel_member
- kext
- syscall
- mig_routine
- panic_frame
- source_line
- problem
- job
- activity

### Route Resolution

Route resolver responsibilities:

- validate project id
- validate source revision
- map object ids to current facts
- map addresses to source/image/member context
- convert global dyld-cache addresses to image-local addresses
- convert kernel collection addresses to member context
- apply symbolication profile
- handle stale routes
- return display labels
- return suggested representations
- return recoverable alternatives

### Stale Route Handling

Routes go stale when:

- source is missing
- source hash changed
- project migration rewrote ids
- analysis invalidated a procedure
- user transaction removed a manual object
- dSYM/KDK profile changed
- cache image projection changed
- source revision changed after patching

Stale route response:

```text
RouteResolution {
  status: stale
  reason
  previous_display_label
  candidate_routes
  recovery_commands
}
```

Example recovery commands:

- Relocate Source
- Re-run Analysis
- Use Newest Procedure Boundary
- Open Nearest Address
- Open Provider Image
- Switch Symbolication Profile
- Remove Broken Bookmark

## Navigation Stack

### Requirements

Navigation stack must work across:

- assembly
- graph
- hex
- source lines
- metadata browsers
- search results
- xref panes
- ObjC browsers
- C++ browsers
- dyld cache views
- kernel views
- debugger overlays

### Navigation Entry

```text
NavigationEntry {
  route
  selection
  representation
  viewport
  source_label
  user_label
  reason
  timestamp
}
```

Reasons:

- user_jump
- search_result
- follow_xref
- back
- forward
- inspector_link
- navigator_selection
- command_palette
- debugger_pc
- panic_frame
- automation
- restore_session

### Stack Behavior

Rules:

- pushing a new route clears forward history
- back and forward do not create new entries
- repeated route clicks within a small time window coalesce
- scroll-only changes update the current viewport but do not push
- representation-only changes can update current entry or push depending on
  user intent
- command palette route opens push history
- inspector links push history
- source-line jumps push history

### Branch History

Reverse engineering is exploratory. Back/forward is not enough once users
branch repeatedly.

Damsel should persist:

- linear back/forward stack for immediate navigation
- recent route list
- session jump graph
- bookmarks
- named trails later

The jump graph can support:

- "return to last ObjC path"
- "show my route trail for this finding"
- "export evidence path"
- "restore previous analysis trail"

### Navigation Persistence

Persist per project session:

- active route
- back stack
- forward stack
- recent routes
- viewport state
- representation state
- focus region

Rules:

- corrupt session state is recoverable
- routes are validated lazily during restoration
- project opens before full restoration completes
- missing source warnings do not block route history display

## Command Palette

### Why It Matters

Hopper's public pages emphasize keyboard and native app speed. A command
palette is Damsel's chance to exceed Hopper's discoverability while preserving
native feel.

The palette should search:

- commands
- addresses
- symbols
- functions
- strings
- ObjC classes
- ObjC selectors
- ObjC methods
- C++ classes
- vtables
- Swift symbols
- cache images
- kernel members
- kexts
- syscalls
- MIG routines
- panic frames
- source files
- problems
- jobs
- recent routes
- bookmarks
- tags

### Palette Result

```text
PaletteResult {
  kind
  title
  subtitle
  badge
  route?
  command_id?
  disabled_reason?
  confidence
  provenance
  ranking_features
}
```

### Ranking Rules

Rank higher:

- exact symbol/address matches
- current image/member matches
- recently visited routes
- user bookmarks
- high-confidence recovered facts
- ObjC selectors in current class/category
- provider images for current unresolved import
- KDK/dSYM matches for current kernel route
- commands valid in current context

Rank lower:

- stale routes
- low-confidence inferred facts
- commands disabled in current context
- results in unrelated source/member unless query is global

### Palette Categories

Initial categories:

- Commands
- Routes
- Symbols
- Functions
- Strings
- ObjC
- C++
- Cache
- Kernel
- Problems
- Recent

Category filters can be typed:

```text
> set comment
@0x100003f20
#objc_msgSend
:objc UIViewController
:cache Foundation
:kernel IOUserClient
```

### Palette Execution

Palette routes:

- pushing navigation stack
- selecting route
- switching representation if result has a preferred view
- updating inspector

Palette commands:

- executing commands through dispatcher
- showing confirmation for mutating actions
- surfacing disabled reasons

## Context Menus

Context menus should be generated from command descriptors and the selected
route.

Assembly row context:

- Copy Address
- Copy Instruction
- Copy Row
- Show Xrefs
- Show Callers
- Show Callees
- Rename Symbol
- Set Comment
- Toggle Bookmark
- Apply Tag
- Convert To Code
- Convert To Data
- Open In Graph
- Open In Hex
- Copy Route

Graph node context:

- Open Basic Block
- Show Predecessors
- Show Successors
- Group Nodes
- Ungroup Nodes
- Set Node Color
- Edit Node Text
- Set Comment
- Copy Route

Hex context:

- Copy Bytes
- Copy Address Range
- Open Owning Instruction
- Set Byte Type
- Edit Bytes, disabled until write mode
- Copy Route

ObjC selector context:

- Show Implementations
- Show Message Senders
- Show Method IMP
- Show Class
- Copy Selector
- Apply Tag
- Copy Route

Cache image context:

- Open Image
- Show Imports
- Show Exports
- Show Reexports
- Jump To Provider
- Compare With Other OS Build
- Copy Install Name
- Copy Route

Kernel member context:

- Open Member
- Show Kext Info
- Show IOKit Classes
- Show Syscalls
- Attach KDK
- Attach Panic Log
- Copy Route

## Undo And Redo

### Current Problem

The Edit menu exposes Undo and Redo through generic selectors, but the app does
not have project transactions to undo.

This creates a misleading UX if the menu appears enabled by text controls but
does not undo analysis edits. Hopper-level polish requires Undo and Redo to
apply to user-visible analysis mutations.

### Transaction Rule

Every user mutation of project analysis state is a transaction.

Transaction examples:

- set comment
- clear comment
- rename symbol
- rename local label
- rename register
- apply tag
- remove tag
- set color
- toggle bookmark
- add manual reference
- remove manual reference
- set operand format
- convert byte type
- create procedure
- adjust procedure bounds
- bind ObjC message send
- bind C++ virtual call target
- attach dSYM
- attach KDK
- attach panic log
- accept analyzer suggestion
- suppress problem

Future write-mode transaction examples:

- patch bytes
- assemble instruction
- NOP region
- export patched copy

### Transaction Record

```text
Transaction {
  id
  project_id
  kind
  command_id
  actor
  invocation_surface
  created_at
  source_revision_before
  source_revision_after?
  route_before
  route_after?
  operations
  inverse_operations
  affected_routes
  invalidation_scope
  audit_summary
  privacy_summary
  undo_state
  redo_state
}
```

### Operation Record

```text
TransactionOperation {
  op_id
  kind
  target_route
  target_object_id?
  field
  old_value
  new_value
  provenance
  preconditions
  postconditions
}
```

### Undo Policies

Policies:

- `not_undoable`: view-only or external action
- `session_undoable`: UI session state only
- `project_undoable`: project facts can be undone
- `source_revision_undoable`: future byte patches with source revisions
- `requires_confirmation_to_undo`: high-impact transaction

Examples:

- Copy Address: not undoable
- Toggle Inspector: session undoable if window layout history exists
- Set Comment: project undoable
- Attach KDK: project undoable if profile attachment can be removed safely
- Patch Bytes: source revision undoable later

### Undo Manager Bridge

Swift should bridge the project transaction stack to `UndoManager`, but
`UndoManager` must not be the source of truth.

Flow:

1. User runs `annotation.set_comment`.
2. Command dispatcher sends API request.
3. Rust project kernel creates transaction.
4. API response returns transaction id and undo label.
5. Swift transaction store updates.
6. Swift registers an undo action that calls project API.
7. Edit menu displays "Undo Set Comment" when possible.
8. Undo command calls project API, not a local closure over stale state.
9. Project kernel validates preconditions and applies inverse operations.
10. UI receives events for affected routes and refreshes views.

### Redo

Redo rules:

- redo applies only if preconditions still match
- new project mutation after undo clears redo branch unless branch history is
  explicitly introduced later
- redo failure returns changed preconditions and recovery actions
- redo labels are stable

### Undo Across Automation

Commands invoked by CLI, MCP, or plugins should use the same transaction model.

Rules:

- automation-created mutations are undoable unless explicitly marked otherwise
- activity log records invocation surface
- UI can undo automation-created project transactions
- automation can request undo by transaction id with permission
- hidden project mutations are forbidden

### Undo And Analysis Facts

Analyzer-derived facts are not user transactions by default. However, user
actions that accept or reject an analyzer suggestion are transactions.

Examples:

- analyzer discovers a function: not user undoable
- user creates a procedure manually: undoable
- analyzer suggests an ObjC message binding: not undoable until accepted
- user accepts binding: undoable
- reanalysis invalidates a function: job fact change, not direct user undo

## Activity And Audit Log

### Why It Matters

Command execution needs a visible history. Reverse engineering often produces
evidence. Users need to know what was changed, what was inferred, and what was
exported.

### Activity Entry

```text
ActivityEntry {
  id
  project_id
  command_id
  title
  invocation_surface
  actor
  started_at
  finished_at?
  status
  route
  selection
  transaction_id?
  job_id?
  problem_ids
  privacy_flags
  reproduction
}
```

### Activity Surfaces

- status strip recent message
- bottom activity drawer
- command result popover
- diagnostics report
- evidence package
- CLI JSON
- MCP result metadata

### Privacy Rules

Activity logs should avoid leaking:

- full private source paths by default
- raw proprietary strings unless user exports evidence
- symbol names from private artifacts in generic diagnostics
- debugger process details unless debugger diagnostics are requested

Instead use:

- source ids
- redacted basenames
- route ids
- hashes
- user-approved evidence snippets

## Job Dispatch And Cancellation

### Command To Job Bridge

Commands that start long work return `accepted_job`.

Examples:

- analyze project
- rebuild indexes
- layout graph
- import dSYM
- import KDK profile
- scan ObjC selector senders
- scan C++ vtables
- project dyld cache image
- symbolicate panic log
- create evidence package

### Job Policy

```text
JobPolicy {
  starts_job
  cancellable
  resumable
  allows_partial_results
  priority
  progress_kind
  max_foreground_latency_ms
}
```

### UI Behavior

Rules:

- command accepts quickly or fails quickly
- job progress updates event stream
- Cancel command targets the active job or selected job
- Cancel All requires confirmation if destructive to useful partial work
- partial results are visible when safe
- job failures become problem entries with route context

### Menu And Palette Validation

Analysis commands should not merely toggle a global busy state.

Examples:

- Run Analysis: enabled when project loaded and no incompatible exclusive job
- Cancel Current Job: enabled when a cancellable job has focus
- Cancel All Jobs: enabled when any cancellable jobs exist
- Rebuild Indexes: enabled when project loaded, disabled if migration required
- Import dSYM: enabled when project source supports UUID matching

## Error And Problem Model

### Command Errors

Error classes:

- invalid context
- route not found
- stale route
- missing source
- missing permission
- helper unavailable
- job rejected
- transaction conflict
- analysis precondition failed
- unsupported source kind
- unsupported architecture
- privacy redaction required
- external tool unavailable

### Error Surface Rules

- simple transient errors can show status strip messages
- route-specific errors become problem records
- repeated errors are coalesced
- recoverable errors offer commands
- fatal project errors open a drawer with details
- diagnostics never claim success when command failed

### Problem Commands

Problems are navigable.

Commands:

- Open Problem Route
- Show Affected Facts
- Retry Command
- Suppress Problem
- Export Problem Evidence
- Copy Problem Diagnostics

## API Contracts

### Command Introspection

```text
GET /api/commands
```

Returns:

- command descriptors visible to API clients
- command ids
- titles
- groups
- required capabilities
- mutation kind
- undo policy
- supported surfaces
- parameter schema

### Command Availability

```text
POST /api/projects/{project_id}/commands/{command_id}/availability
```

Request:

```json
{
  "route": "route:procedure:project-1:image-2:100003f20",
  "selection": {
    "kind": "instruction",
    "route": "route:instruction:project-1:image-2:100003f30"
  },
  "surface": "command_palette"
}
```

Response:

```json
{
  "enabled": true,
  "disabled_reason": null,
  "requires_confirmation": false,
  "requires_permission": false
}
```

### Command Execution

```text
POST /api/projects/{project_id}/commands/{command_id}/execute
```

Request:

```json
{
  "route": "route:instruction:project-1:image-2:100003f30",
  "surface": "native_menu",
  "parameters": {
    "comment": "validates entitlement before opening user client"
  }
}
```

Response:

```json
{
  "status": "completed",
  "activity_id": "activity-123",
  "transaction_id": "txn-456",
  "route_delta": null,
  "selection_delta": null,
  "problem_ids": []
}
```

### Transaction Stack

```text
GET /api/projects/{project_id}/transactions
POST /api/projects/{project_id}/transactions/{transaction_id}/undo
POST /api/projects/{project_id}/transactions/{transaction_id}/redo
```

Response fields:

- undo label
- redo label
- can undo
- can redo
- top undo transaction id
- top redo transaction id
- disabled reason

### Navigation API

```text
GET /api/projects/{project_id}/session/navigation
POST /api/projects/{project_id}/session/navigation/push
POST /api/projects/{project_id}/session/navigation/back
POST /api/projects/{project_id}/session/navigation/forward
POST /api/projects/{project_id}/session/navigation/replace-current
DELETE /api/projects/{project_id}/session/navigation
```

Rules:

- native UI can keep a local fast cache
- project session persists authoritative history
- route resolver validates entries lazily
- API responses include stale-route recovery data

### Command Activity API

```text
GET /api/projects/{project_id}/activity
GET /api/projects/{project_id}/activity/{activity_id}
POST /api/projects/{project_id}/activity/{activity_id}/copy-reproduction
```

Use:

- diagnostics
- evidence packages
- support bundles
- automation replay

## Swift Implementation Plan

### New Swift Types

Suggested files:

```text
damsel-app/Sources/DamselAppCore/Commands/CommandDescriptor.swift
damsel-app/Sources/DamselAppCore/Commands/CommandRegistry.swift
damsel-app/Sources/DamselAppCore/Commands/CommandDispatcher.swift
damsel-app/Sources/DamselAppCore/Commands/CommandContext.swift
damsel-app/Sources/DamselAppCore/Commands/CommandResult.swift
damsel-app/Sources/DamselAppCore/Commands/CommandAvailability.swift
damsel-app/Sources/DamselAppCore/Commands/KeymapStore.swift
damsel-app/Sources/DamselAppCore/Stores/RouteStore.swift
damsel-app/Sources/DamselAppCore/Stores/NavigationStore.swift
damsel-app/Sources/DamselAppCore/Stores/SelectionStore.swift
damsel-app/Sources/DamselAppCore/Stores/FocusStore.swift
damsel-app/Sources/DamselAppCore/Stores/TransactionStore.swift
damsel-app/Sources/DamselAppCore/Stores/ActivityStore.swift
```

### WorkbenchStore Migration

Current `WorkbenchStore` can remain as a facade during migration.

Phase shape:

```text
WorkbenchStore
  serverStore
  projectStore
  commandRegistry
  commandDispatcher
  routeStore
  navigationStore
  selectionStore
  focusStore
  jobStore
  problemStore
  transactionStore
```

Migration rule:

- do not add more broad command-specific methods to `WorkbenchStore`
- new UI action code should call `commandDispatcher.run(command_id, context)`
- existing methods can be wrapped as initial command handlers

### Native Menu Controller

Add:

```text
MainMenuController
  buildMainMenu()
  validateCommandMenuItem(_:)
  performCommandMenuItem(_:)
```

Benefits:

- `DamselWorkbenchApplication` becomes lifecycle shell
- menu layout becomes testable
- menu validation becomes descriptor-driven
- command palette can reuse descriptors

### Command Palette View

Add:

```text
CommandPaletteView
CommandPaletteStore
PaletteResultRow
PaletteSearchService
```

Initial implementation:

- local command descriptor search
- recent routes
- visible instruction rows
- loaded symbol suggestions

Later:

- project search endpoint
- ObjC/C++/kernel/cache indexes
- problem and job search

### Route And Selection Integration

Initial route migration can wrap instruction addresses:

```text
route:instruction:<project-or-session>:<address>
```

But the model must not stop there. Even the first implementation should use
route-shaped Swift values so future graph, ObjC, cache, and kernel routes do
not require another UI rewrite.

## Rust Implementation Plan

### Project Kernel Types

Add or align with project-kernel specs:

```text
CommandId
RouteId
Route
RouteResolution
Selection
NavigationEntry
ProjectSession
Transaction
TransactionOperation
ActivityEntry
CommandAvailability
CommandExecutionRequest
CommandExecutionResponse
```

### Command Registry Location

Short term:

- Swift owns native-only commands
- Rust owns project/API command descriptors
- descriptors are merged in Swift

Long term:

- Rust exposes project command descriptors
- Swift overlays native app command descriptors
- plugin commands register through a manifest

### Transaction Store

Transaction store responsibilities:

- append transaction
- append inverse operations
- validate preconditions
- apply transaction atomically
- apply inverse transaction
- clear redo branch on new mutation
- publish affected-route events
- persist audit entry

### Command Execution Boundary

Rust API should execute only project and analysis commands. Swift should keep
native-only app commands local.

Examples:

- Swift local: show command palette, focus navigator, toggle inspector
- Rust API: set comment, rename symbol, attach dSYM, jump route resolution,
  start analysis job

## CLI And Automation Parity

### Why Parity Matters

Hopper has Python scripting, SDK extension points, and MCP support. Damsel's
competitive niche should make scripted Apple-system workflows safer and more
auditable than ad hoc UI state.

### CLI Command Families

Future CLI shapes:

```text
damsel project command list --project file.damsel
damsel project command availability --project file.damsel --command annotation.set-comment --route ...
damsel project command run --project file.damsel --command annotation.set-comment --route ... --json
damsel project undo --project file.damsel --json
damsel project redo --project file.damsel --json
damsel project navigation list --project file.damsel --json
damsel project route open --project file.damsel --route ... --json
```

### Automation Rules

- UI and CLI use same route ids
- UI and CLI use same transaction stack
- automation can request dry-run availability
- mutation commands require explicit mutation permission
- commands return activity ids
- evidence export can include command trail

## Apple Systems Command Families

### ObjC Commands

Commands:

- Show Selector Implementations
- Show Selector Senders
- Show Method IMP
- Show Class Methods
- Show Category Methods
- Show Protocol Requirements
- Bind Message Send Target
- Copy ObjC Signature
- Apply Class Tag

Context:

- ObjC selector route
- ObjC class route
- method route
- instruction route with `objc_msgSend` style call

Transactions:

- manual binding
- method rename
- selector tag
- comment

### C++ And ObjC++ Commands

Commands:

- Show Vtable
- Show Virtual Call Targets
- Show Constructors
- Show Destructors
- Rename Type
- Rename Method
- Bind Virtual Call Target
- Apply IOKit Family Tag

Context:

- vtable route
- type route
- procedure route
- indirect call instruction route

Transactions:

- type rename
- virtual target binding
- tag
- comment

### Dyld Shared-Cache Commands

Commands:

- Open Cache Image
- Show Image Imports
- Show Image Exports
- Show Reexports
- Jump To Provider Image
- Jump To Importer Images
- Compare Cache Image Across OS Builds
- Open Projected Image
- Copy Install Name

Context:

- cache route
- image route
- import route
- export route
- symbol route

Navigation:

- preserve global cache address and image-local address
- show provider chain route trail
- keep source/member context in status strip

### Kernel And XNU Commands

Commands:

- Open Kernel Collection Member
- Show Kext Info
- Show IOKit Classes
- Show User Client Methods
- Show Syscalls
- Show MIG Routines
- Attach KDK
- Attach Panic Log
- Symbolicate Panic Frame
- Jump From Panic Frame To Procedure
- Compare Across KDK Profiles

Context:

- kernel collection route
- fileset member route
- kext route
- procedure route
- panic frame route
- KDK profile state

Transactions:

- attach KDK
- attach panic log
- accepted symbolication profile
- manual panic frame binding

## Inspector Command Integration

Inspector modules should not be special cases. They are command surfaces.

Instruction Encoding module:

- Copy Bytes
- Open In Hex
- Set CPU Mode where supported

Operand Format module:

- Set Signed Hex
- Set Unsigned Hex
- Set Decimal
- Set Address
- Clear Format Override

Comment module:

- Set Comment
- Clear Comment
- Copy Comment

Colors And Tags module:

- Apply Tag
- Remove Tag
- Set Color
- Clear Color

References module:

- Show Xrefs
- Add Manual Reference
- Remove Manual Reference
- Jump To Reference

Procedure module:

- Rename Procedure
- Rename Local Label
- Rename Register
- Show Callers
- Show Callees
- Adjust Bounds
- Apply Calling Convention

Apple Runtime module:

- Show ObjC Sender
- Show Selector Implementations
- Show C++ Virtual Targets
- Show KDK Symbol Source

## Representation Commands

### Assembly Commands

Core commands:

- select row
- follow branch target
- follow call target
- follow data reference
- copy address
- copy instruction
- set comment
- rename symbol
- show xrefs
- open in graph
- open in hex
- set operand format

### Graph Commands

Core commands:

- select node
- select edge
- move cursor to nearest node
- group nodes
- ungroup nodes
- set node color
- edit node text
- fit graph
- export graph
- open node in assembly
- open node in hex

### Hex Commands

Core commands:

- select byte
- select byte range
- open owning instruction
- open owning data object
- copy bytes
- copy address range
- convert byte type
- edit bytes when write mode exists

### Feature Map Commands

Core commands:

- jump to position
- filter by byte type
- filter by source/member
- show legend
- open selected lane route
- copy map summary

## Confirmation Policy

Commands require confirmation when:

- they remove user facts
- they invalidate a large amount of analysis
- they cancel many jobs
- they export private evidence
- they attach a source with privacy implications
- they enable debugging entitlements
- they write or patch bytes

Confirmation should show:

- command title
- affected route
- affected source
- affected fact count
- undo availability
- privacy impact
- recovery path

## Performance Budgets

Command system budget:

- menu validation under 8 ms for visible menus
- command palette open under 100 ms
- command palette first local results under 50 ms
- command palette indexed results under 200 ms for warm project
- command dispatch acknowledgement under 100 ms
- local focus or view commands under 16 ms after acceptance
- navigation back/forward under 100 ms for warm route
- route resolution under 50 ms for warm route
- undo acknowledgement under 100 ms for simple annotation
- transaction apply under 250 ms for simple annotation
- job command acceptance under 100 ms

Large-project rules:

- command palette never blocks on full index scan
- disabled reason lookup uses cached context
- menu validation never triggers network-heavy or disk-heavy work
- stale-route recovery can be lazy
- activity log is paged
- transaction history is paged
- command descriptors are cached

## Testing Strategy

### Unit Tests

Swift:

- command descriptor decoding
- command registry duplicate id detection
- keymap conflict detection
- menu layout generation
- command availability mapping
- focus delta application
- navigation stack push/back/forward
- selection serialization
- undo label mapping

Rust:

- route resolution
- transaction apply
- transaction inverse
- redo invalidation
- activity log append
- command availability
- command execution request validation

### Integration Tests

Workbench test flows:

- Open binary by menu command.
- Run analysis by command descriptor.
- Focus filter by shortcut.
- Clear filter by command.
- Select instruction and copy address.
- Navigate to a symbol from command palette.
- Navigate back and forward.
- Add comment through inspector command.
- Undo comment through menu command.
- Redo comment.
- Cancel analysis job.
- Open problem route.

API tests:

- command list JSON snapshot
- availability disabled reason snapshot
- execute set comment command
- undo transaction
- redo transaction
- stale route recovery response
- activity log entry redaction

Corpus tests:

- standalone Mach-O navigation stack
- dyld shared-cache provider jump
- ObjC selector implementation jump
- C++ vtable route jump
- kernel collection member route jump
- panic frame route with KDK profile

### UI Automation Tests

Keyboard-only acceptance:

- launch Workbench
- open fixture
- run analysis
- command palette search for symbol
- jump to symbol
- switch graph
- navigate back
- add comment
- undo comment
- export selected evidence summary

Screen-reader acceptance:

- focused region has accessible name
- command palette results announce kind and disabled reason
- menu items expose shortcut and state
- selected route is announced when representation changes

### Performance Tests

Benchmarks:

- command palette open time
- first local results time
- indexed results time
- menu validation pass time
- navigation route resolution time
- undo simple annotation time
- transaction history page time
- activity log page time

Fixtures:

- small Mach-O
- large app binary
- dyld shared-cache projected image
- synthetic large symbol index
- synthetic many-route navigation history
- synthetic many-transaction project

## Migration Plan

### Phase 1: Descriptor Spine

Deliverables:

- Swift `CommandDescriptor`
- Swift `CommandRegistry`
- descriptors for existing commands
- command id constants replace notification-only names
- menu items carry command ids
- existing selectors route through dispatcher where practical
- tests for registry and menu layout

Exit criteria:

- existing Open Binary, Disassemble, Cancel, Find, Clear Filter, Copy
  Diagnostics, and copy selected instruction commands work through descriptors
- menu validation uses command availability objects
- command palette can list existing commands, even before route search exists

### Phase 2: Route And Selection Spine

Deliverables:

- Swift `Route`
- Swift `Selection`
- instruction address selection wrapped as route
- selection store
- route store
- copy route command
- route display labels
- tests for route serialization

Exit criteria:

- selected instruction is represented as route-backed selection
- inspector and table can consume selection route
- copy route works
- stale or missing selected instruction produces a disabled reason instead of
  silent failure

### Phase 3: Navigation Stack

Deliverables:

- navigation store
- back and forward commands
- route push on symbol/address/search jump
- recent routes
- session persistence
- tests for coalescing and forward clearing

Exit criteria:

- user can jump and return with keyboard
- back/forward works across current instruction routes
- recent route list appears in command palette
- navigation persists for reopened project/session where available

### Phase 4: Command Palette MVP

Deliverables:

- command palette UI
- local command search
- visible instruction/address search
- recent route search
- disabled reason display
- keyboard tests

Exit criteria:

- Command-P opens palette
- palette can run existing commands
- palette can jump to visible instruction or recent route
- disabled commands explain why

### Phase 5: Project Transactions

Deliverables:

- transaction API shape
- annotation transaction for comments and labels
- Swift transaction store
- UndoManager bridge
- undo/redo menu labels
- transaction tests

Exit criteria:

- set comment creates transaction
- undo removes comment
- redo restores comment
- activity log records command and transaction
- CLI/API can inspect transaction stack

### Phase 6: Jobs And Problems Integration

Deliverables:

- job commands
- job activity entries
- cancel command targets selected job
- problem route commands
- command failures become problem entries

Exit criteria:

- long-running analysis command returns job id
- cancel command is route/context aware
- failed command creates navigable problem when appropriate
- diagnostics include command and job trail

### Phase 7: Apple Niche Command Families

Deliverables:

- ObjC selector commands
- dyld provider/importer commands
- kernel member commands
- dSYM/KDK attach commands
- panic frame commands
- route-specific context menus

Exit criteria:

- command palette can find ObjC, cache, and kernel commands
- disabled commands explain missing indexes or sidecars
- Apple-system routes push navigation history
- commands share same activity and transaction model

### Phase 8: Representation Command Coverage

Deliverables:

- graph commands
- hex commands
- feature map commands
- inspector command modules
- synchronized selection commands

Exit criteria:

- assembly, graph, hex, and feature map share one selection
- representation switches preserve route
- context menus are command generated
- keyboard-only route movement works in graph and hex

## Acceptance Criteria

Product acceptance:

- every visible menu item maps to a command descriptor
- every command has a stable id
- every disabled command can explain why
- command palette can run commands and open routes
- back/forward works across route kinds, not only addresses
- selection is route-backed
- undo/redo works for comments and labels before broader mutation work
- activity log records mutating commands
- command results can be represented in JSON
- keyboard-only user can complete a basic analysis loop

Architecture acceptance:

- new command code does not add more one-off notification commands
- mutating commands go through transaction API
- project transactions are not stored only in Swift
- route resolution is centralized
- command availability is structured
- menu validation is cheap
- command descriptors can be snapshotted

Apple niche acceptance:

- route model can represent dyld cache image, cache provider, ObjC selector,
  C++ vtable, kernel member, KDK profile, and panic frame routes
- command families are documented for ObjC, C++, dyld cache, and kernel/XNU
- disabled reasons explain missing sidecars, indexes, source grants, or
  symbolication profiles
- command trails can be exported into evidence packages later

## Risks

### Risk: Command System Becomes Too Abstract

Mitigation:

- migrate existing commands first
- keep descriptors data-oriented
- keep handlers small
- keep local UI commands local
- require tests for each descriptor

### Risk: Menu Validation Becomes Slow

Mitigation:

- no disk, network, or heavy API calls during validation
- cache context snapshots
- precompute command availability for visible menus
- lazy-load expensive disabled details only in palette/details panes

### Risk: Undo Diverges Between UI And Automation

Mitigation:

- project kernel owns transaction truth
- Swift `UndoManager` only bridges
- CLI/MCP use same transaction ids
- transaction tests cover UI-created and API-created mutations

### Risk: Routes Become Stringly Typed

Mitigation:

- route strings are external ids only
- internal code uses typed route structs
- route parser validates kind-specific fields
- tests cover stale and malformed routes

### Risk: Apple-Specific Commands Get Bolted On Later

Mitigation:

- include Apple route kinds in the first route model
- include ObjC/cache/kernel groups in the first command namespace
- require command palette and context menu architecture to handle domain groups
- add disabled commands early so the product surface is visible

## Documentation Updates Required

When this spec is accepted into the pack, cross-link it from:

- pack README
- research sources
- target architecture
- current code integration map
- API and data contracts
- native Workbench implementation spec
- native app polish plan
- competitive UX plan
- search/navigation plan
- integration roadmap
- implementation epics
- refactor subsystem plan
- quality validation plan
- corpus benchmark plan
- evidence matrix
- Apple runtime roadmap

## Summary

Hopper-class interaction is not only "more views." It is a document command
system:

- typed commands
- stable route ids
- route-backed selection
- navigation history
- focus ownership
- structured disabled reasons
- undoable project transactions
- visible activity logs
- cheap menu validation
- command palette search
- shared UI/CLI/automation semantics

Damsel can use this subsystem to become competitive in a way that fits its own
niche. The generic Hopper loop becomes Apple-system aware: jump from cache
imports to providers, from selectors to implementations, from panic frames to
KDK-symbolicated procedures, from C++ vtables to IOKit families, and from every
claim to route-backed evidence.
