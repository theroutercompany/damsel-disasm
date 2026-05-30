# Preferences, Keymaps, Layout Persistence, And Theming Implementation Spec

Status: implementation-grade product and architecture specification. This is
not an implementation claim.

Purpose: define the user preference, keymap, workspace layout, theme,
accessibility, and discoverability subsystem Damsel needs to feel like a
Hopper-competitive native reverse-engineering workstation rather than a fixed
demo surface over point-query analysis.

This document exists because preferences are not cosmetic in a disassembler.
They decide how bytes, instructions, comments, graph nodes, hex columns,
symbol filters, analysis jobs, privacy policies, keyboard shortcuts, and
Apple-system workflows appear during long reverse-engineering sessions.

Primary Damsel audience:

- native Workbench maintainers
- command/navigation maintainers
- Rust project-kernel maintainers
- API/CLI contract maintainers
- performance and scheduler maintainers
- accessibility and diagnostics maintainers
- future automation/plugin/assistant maintainers

Primary user audience:

- macOS and iOS reverse engineers who spend hours in the same project
- Apple systems researchers switching among dyld caches, kernel collections,
  KDKs, dSYMs, app bundles, and panic logs
- Objective-C, C++, Objective-C++, Swift-adjacent, and XNU analysts
- security researchers who need repeatable reports without leaking private
  paths, comments, or local machine state

## Companion Documents

Core companions:

- [Native App Polish Plan](native-app-polish-plan.md)
- [Native Workbench Hopper-Grade Implementation Spec](native-workbench-hopper-grade-implementation-spec.md)
- [Command, Navigation, Undo, And Native Workbench Interaction Implementation Spec](command-navigation-undo-workbench-implementation-spec.md)
- [Semantic Workbench Surfaces Implementation Spec](semantic-workbench-surfaces-implementation-spec.md)
- [Graph, Feature Map, Hex, And Representation Sync Implementation Spec](graph-feature-map-hex-representation-sync-implementation-spec.md)
- [Semantic Editing, Analyst Knowledge, And Reanalysis Hints Implementation Spec](semantic-editing-reanalysis-hints-implementation-spec.md)
- [Large-Project Performance, Scheduler, And Indexing Implementation Spec](large-project-performance-scheduler-indexing-implementation-spec.md)
- [Native App Trust, Distribution, Permissions, And Diagnostics Implementation Spec](native-app-trust-distribution-diagnostics-implementation-spec.md)
- [Apple Runtime, ObjC/C++/XNU, And System Browser Workbench Implementation Spec](apple-runtime-workbench-browsers-implementation-spec.md)
- [API And Data Contracts](api-data-contracts.md)
- [Quality And Validation Plan](quality-validation-plan.md)
- [Evidence And Acceptance Matrix](evidence-and-acceptance-matrix.md)

Research companions:

- [Hopper Public Research Notes](hopper-public-research.md)
- [Hopper Deep Public-Web Research Addendum](hopper-deep-public-web-research.md)
- [Hopper Interaction, Debugger, And Product Surface Research](hopper-interaction-debugger-product-research.md)
- [Hopper Public-Web Feature Architecture Audit, 2026](hopper-public-web-feature-architecture-audit-2026.md)
- [Hopper Feature Architecture Public Evidence](hopper-feature-architecture-public-evidence.md)

## Public Source Evidence Used

Source URLs:

- <https://www.hopperapp.com/tutorial.html>
- <https://www.hopperapp.com/index.html>
- <https://makigumo.github.io/>
- <https://www.samsclass.info/127/proj/p7xa-hopper.htm>
- <https://developer.apple.com/documentation/foundation/userdefaults>
- <https://developer.apple.com/design/human-interface-guidelines/dark-mode>
- <https://developer.apple.com/documentation/appkit/nswindow>

Official Hopper tutorial evidence:

- Hopper exposes application preferences for displaying instruction encodings
  between address and mnemonic.
- Hopper exposes application preferences that can disable fuzzy symbol search.
- Hopper's hex mode uses adaptive column counts by default and can be forced
  to a fixed column count such as 16.
- Hopper presents multiple synchronized representations: assembly, CFG,
  pseudocode, and hex.
- Hopper's UI is split into left symbol/string pane, central representation
  workspace, and right contextual inspector.
- Hopper's tutorial describes toolbar/keyboard byte-type actions.
- Hopper's tutorial describes keyboard movement among graph basic blocks.
- Hopper's tutorial describes showable/hideable panes through its product UI.
- Hopper has a navigation stack and toolbar navigation controls.
- Hopper's navigation bar uses byte-category color lanes.
- Hopper exposes contextual inspector modules for instruction encoding,
  operand format, comments, colors, tags, references, and procedure details.

Official Hopper homepage evidence:

- Hopper markets a "fast and native macOS disassembler".
- Hopper explicitly emphasizes keyboard shortcuts and native macOS
  integration.
- Hopper describes a polished Cocoa interface.
- Hopper displays assembly, CFG, and pseudocode at the same time.
- Hopper exposes scriptability and SDK extensibility, which implies settings
  and UI behavior must coexist with automation and plugin workflows.
- Hopper now markets an AI/MCP integration surface, which raises the importance
  of user-controlled privacy and context settings.

Public ecosystem evidence:

- A public Hopper plugin index lists tool plugins, scripts, loader plugins,
  CPU plugins, and a theme entry.
- A public instructional Hopper lab walks users through hiding/showing panes,
  using the navigation bar, and enabling the hex column through preferences.

Apple platform evidence:

- Apple's `UserDefaults` documentation positions defaults as persistent
  app-specific and system-wide settings, and warns not to store personal or
  sensitive information in defaults because the data is not encrypted.
- Apple's Dark Mode guidance says users expect apps to respect the system
  appearance and encourages semantic/adaptive colors rather than hard-coded
  colors.
- Apple's `NSWindow` documentation exposes frame autosave and restoration
  concepts that a native Workbench can use for window/session behavior.

Inference:

- Hopper-level polish is partly a preference architecture problem. Users
  expect control over visual density, instruction bytes, search behavior, hex
  layout, pane visibility, graph behavior, keyboard shortcuts, and theme.
- Damsel cannot store every preference in one Swift `UserDefaults` blob. Some
  choices are personal, some are project evidence, some are session state, and
  some are security/privacy policy.
- Preferences must be observable by CLI, Workbench, automation, and
  diagnostics because they change rendered evidence, performance behavior, and
  privacy decisions.

## Current Damsel Evidence

Current Workbench evidence:

- `damsel-app/Sources/DamselWorkbench/DamselWorkbenchApp.swift` owns the app
  entry point and menu wiring.
- `damsel-app/Sources/DamselAppCore/Support/DamselCommands.swift` currently
  contains a small set of notification names, not a typed command/keymap
  system.
- `damsel-app/Sources/DamselAppCore/Stores/WorkbenchStore.swift` currently
  owns broad Workbench state including loaded file, disassembly, selected
  instruction, filters, busy state, and diagnostics.
- `ContentView` owns the current three-pane Workbench composition.
- `ControlPane`, `InstructionSurfaceView`, `InspectorPane`, and
  `VirtualizedInstructionTable` expose current UI surfaces.
- There is no dedicated `PreferenceStore`, `KeymapStore`, `ThemeStore`,
  `WorkspaceLayoutStore`, or `HelpStore`.
- There is no project preference API.
- There is no preference export/import path.
- There is no settings validation or migration story.
- There is no explicit redaction boundary between machine-local settings and
  project/evidence state.

Existing docs that this spec refines:

- `native-app-polish-plan.md` names selected theme, window state, keyboard
  model, command palette, accessibility, visual density, and macOS conventions.
- `native-workbench-hopper-grade-implementation-spec.md` names future
  application/session stores and route/navigation stores.
- `command-navigation-undo-workbench-implementation-spec.md` defines command
  registry and key routing.
- `semantic-workbench-surfaces-implementation-spec.md` defines semantic rows
  and synchronized representations.
- `large-project-performance-scheduler-indexing-implementation-spec.md`
  defines performance budgets and scheduler behavior.

Current gaps:

- No Settings window.
- No keymap editor.
- No command shortcut conflict model.
- No persistent pane/split/tab layout store.
- No renderer preference registry.
- No theme token system.
- No high-contrast or color-blindness-aware feature-map palette model.
- No per-project analysis profile.
- No per-project privacy/export policy.
- No user-level sidecar path registry for KDK/dSYM/shared-cache roots.
- No safe import/export of preferences.
- No preference-backed snapshot tests.
- No "reset broken layout" recovery path.

## Product Thesis

A reverse-engineering workstation is used under sustained cognitive load. The
user is reading dense code, cross-referencing objects, annotating uncertainty,
and comparing artifacts that may not fit in memory. Small preference failures
become real product failures:

- an unreadable theme slows analysis
- a hidden hex column hides evidence
- an unstable pane layout breaks flow
- a shortcut conflict makes navigation feel unreliable
- restoring a giant old graph on project open freezes the app
- exporting a diagnostic bundle leaks local source paths
- a project shared with a teammate carries private keybindings or KDK paths
- a screenshot has low-contrast byte category colors
- a search preference changes what the user thinks "not found" means

Damsel should treat preferences and workspace personalization as a typed,
versioned subsystem with:

- clear ownership boundaries
- schema migration
- validation
- redaction
- test fixtures
- performance budgets
- observable command integration
- project vs user vs session separation

## Non Goals

This spec does not implement:

- cloud preference sync
- user accounts
- collaborative multi-user settings
- a marketplace for themes/plugins
- arbitrary CSS or user script injection into the Workbench
- live editing of Rust analysis algorithms from preferences
- storing secrets in preferences
- changing default safety policies to make risky actions easier

This spec does define extension points that future plugin/theme packages can
use after trust and capability models exist.

## Ownership Model

### App Preferences

App preferences are per-user, per-machine, and not part of project evidence.

Examples:

- theme choice
- system/light/dark appearance preference
- font family and size
- keymap profile
- reduced motion preference
- default hex column mode
- whether instruction bytes are visible by default
- search fuzzy ranking preference
- recent project display policy
- default analysis profile for new projects
- diagnostics redaction default
- developer metrics visibility

Storage:

- macOS `UserDefaults` for nonsensitive simple values
- local app support JSON or SQLite for structured profiles where needed
- never store secrets, tokens, raw private paths, panic logs, or comments in
  unencrypted defaults

### Project Preferences

Project preferences are part of the Damsel project because they affect how the
artifact is interpreted, rendered, validated, or exported.

Examples:

- active analysis profile
- enabled symbolication profile
- project-specific renderer defaults
- whether user annotations are included in evidence exports
- selected Apple runtime browser defaults
- KDK/dSYM attachment policy by project
- decompiler dead-code display preference for this project
- graph layout cache policy
- saved workspace route tabs
- project-specific privacy/export policy

Storage:

- Damsel project database
- versioned and migratable
- redacted in exported evidence according to policy

### Workspace Session State

Workspace session state is project-associated but not necessarily evidence.

Examples:

- window frame
- split pane sizes
- open tabs
- selected route
- navigation stack
- scroll positions
- inspector expansion state
- graph zoom/pan
- selected navigator filter
- current search query

Storage:

- project session table for route-relevant state
- local window restoration for machine-specific geometry
- recoverable and safely discardable

### Security And Privacy Policy

Security/privacy preferences need stricter handling than visual preferences.

Examples:

- include comments in diagnostics
- include local paths in diagnostics
- include raw bytes in evidence export
- allow MCP read-only context sharing
- allow assistant use of comments
- allow plugin package loading
- allow debugger entitlements
- default external tool invocation policy

Storage:

- app-level policy defaults
- project-level overrides when explicitly saved
- capability grants in trust/permission store
- audit log for changes that affect export or automation

### External Source Registries

Some settings point to local Apple artifacts:

- dyld shared-cache roots
- KDK roots
- dSYM search roots
- IPSW/OTA extraction roots
- crash/panic log folders
- device symbol folders
- custom fixture corpus roots

Rules:

- store stable source ids and security bookmarks where needed
- display redacted path labels by default
- never write raw local paths into public diagnostics by default
- project files may store source identity and sidecar match evidence without
  storing machine-specific absolute paths unless the user chooses it

## Preference Taxonomy

### Assembly Renderer Preferences

Settings:

- show instruction bytes
- show file offset column
- show image offset column
- show cache VM address
- show runtime slid address when debugging
- address casing
- address prefix style
- operand format default
- symbolic operand preference
- show recovered value comments
- show jump arrows
- show branch direction colors
- show PAC/chained-fixup badges
- show ObjC selector badges
- show C++ vtable badges
- show source-line gutter
- show breakpoint gutter
- show bookmark gutter
- show comment lane
- wrap long operands
- compact mode
- row height
- font family
- font size
- line spacing

Design rules:

- renderer preferences must not mutate analysis facts
- renderer preferences should be inspectable in diagnostics without including
  private content
- exported evidence should record renderer preference snapshots when visual
  layout matters
- changing a renderer preference should not trigger analysis re-run

### Graph Preferences

Settings:

- default graph direction
- default layout algorithm
- show minimap
- show edge labels
- show branch condition labels
- show comments in nodes
- show instruction bytes in nodes
- node density
- edge routing style
- highlight selected route
- collapse trivial blocks
- show runtime overlay badges
- show ObjC/C++/XNU overlay badges
- auto-layout on function open
- maximum auto-layout node count
- use list fallback threshold
- graph animation speed

Design rules:

- graph layout is derived presentation state, not CFG truth
- user node colors/groups/text are semantic edits, not theme preferences
- failed layout should fall back to list mode
- layout preferences must be bounded by performance budgets

### Hex/Data Preferences

Settings:

- auto column count
- fixed column count
- default fixed column count
- byte grouping
- endian display
- ASCII side column
- Unicode side column
- highlight current instruction bytes
- highlight current data object bytes
- show typed-byte lane
- show patch preview lane
- show fixup/PAC pointer lane
- show file offsets
- show VM addresses
- default edit mode locked/unlocked

Design rules:

- fixed column count must not overflow compact windows
- editing mode defaults to locked for safety
- hex preferences must stay separate from patch transactions
- huge byte windows must remain bounded regardless of display preference

### Search And Navigator Preferences

Settings:

- fuzzy search enabled
- exact search preference
- case sensitivity
- regex mode
- search scope default
- include comments
- include tags
- include user annotations
- include imported symbols
- include decompiler text
- include private paths
- recent route boost
- tag filters
- kind filters
- result grouping
- maximum preview length

Design rules:

- search preference changes should be visible in search UI
- "not found" must indicate active scope/filter
- private-path search requires explicit opt-in
- search ranking preferences should not change the underlying index truth

### Decompiler Preferences

Settings:

- default decompiler view mode
- show confidence markers
- show source-map gutters
- show unresolved variables
- show dead-code policy
- show low-confidence warnings inline
- include user hints by default
- include ObjC/C++/XNU overlays
- pseudocode font
- pseudocode line spacing
- maximum auto-decompile function size
- background decompile priority

Design rules:

- decompiler preferences may invalidate cached render output but not IR truth
- changing dead-code policy must be recorded in decompiler run metadata when
  it affects generated pseudocode
- user hints are semantic edits, not preferences

### Analysis Profile Preferences

Profiles:

- `triage`
- `balanced`
- `deep`
- `apple_runtime_deep`
- `kernel_xnu`
- `large_cache_safe`
- `debugger_live`
- `automation_batch`

Profile settings:

- auto-analysis depth
- cache-wide indexing enabled
- ObjC dispatch inference enabled
- C++ RTTI/vtable analysis enabled
- IOKit recognizers enabled
- XNU recognizers enabled
- decompiler scheduling
- debug-info import scheduling
- real-artifact opt-in
- memory budget
- CPU concurrency
- background job priority
- maximum graph auto-layout size
- maximum decompile function size

Design rules:

- analysis profile changes are project events
- changes must explain what will be invalidated or recomputed
- deep profiles should not silently turn on risky export or debugger features
- profile snapshots should appear in performance reports

### Apple-System Preferences

Settings:

- preferred shared-cache source roots
- KDK search roots
- dSYM search roots
- device symbol roots
- panic log import defaults
- IPSW/OTA extraction root
- cache projection policy
- symbolication trust threshold
- mismatched KDK handling
- ObjC browser default grouping
- C++ browser default grouping
- IOKit browser security-note visibility
- XNU interface browser default scope

Design rules:

- local source roots are private machine state
- project stores source identity and match receipts
- KDK/dSYM trust policy must not upgrade mismatches into exact matches
- Apple-system preferences should be visible in artifact-intake preflight

### Automation And AI Preferences

Settings:

- enable local MCP server
- default MCP read scope
- allow MCP access to comments
- allow MCP access to raw bytes
- allow assistant access to user annotations
- assistant model profile
- assistant privacy mode
- tool-call confirmation policy
- script run default capability profile
- plugin load policy
- automation activity retention

Design rules:

- automation preferences are capability gates, not cosmetic toggles
- high-risk mutation always requires explicit grant or review
- privacy mode must be visible in assistant/MCP UI
- changing automation policy creates an audit event

### Diagnostics And Export Preferences

Settings:

- include user comments
- include local paths
- include raw bytes
- include panic text
- include assistant prompts
- include script output
- include metrics
- include logs
- include screenshots
- include keymap/theme/layout snapshots
- redaction preset

Presets:

- `safe_default`
- `support_private`
- `evidence_private`
- `evidence_redacted`
- `developer_internal`

Design rules:

- safe default excludes comments, raw bytes, local paths, prompts, and panic
  contents
- export sheets must preview what categories are included
- diagnostics preferences should be sticky but visible every time
- evidence exports should include enough settings metadata to reproduce visual
  interpretation without leaking private values

## Keymap Architecture

### Command Registry Integration

Keymaps must bind to command ids, not Swift selectors.

Example command ids:

- `file.open_binary`
- `file.open_project`
- `project.save`
- `navigate.back`
- `navigate.forward`
- `navigate.jump_to_route`
- `search.global`
- `search.current_view`
- `view.toggle_navigator`
- `view.toggle_inspector`
- `view.show_assembly`
- `view.show_graph`
- `view.show_hex`
- `view.show_pseudocode`
- `annotation.set_comment`
- `semantic_edit.rename`
- `analysis.run_balanced`
- `analysis.cancel`
- `debugger.toggle_breakpoint`
- `preferences.open`

Keymap entries:

```json
{
  "command_id": "view.show_hex",
  "shortcut": "shift+cmd+h",
  "scope": "global",
  "when": "project_open",
  "source": "default"
}
```

### Scopes

Scopes:

- `global`
- `project`
- `assembly`
- `graph`
- `hex`
- `pseudocode`
- `navigator`
- `inspector`
- `debugger`
- `patch_studio`
- `automation_studio`
- `assistant`
- `modal`

Rules:

- local scope may override global only when focus is inside that surface
- modal shortcuts must be explicit and minimal
- destructive commands require confirmation even if shortcut exists
- disabled commands must explain the failing precondition

### Conflict Detection

Conflict kinds:

- exact same shortcut in same scope
- ambiguous shortcut across active scopes
- shortcut conflicts with text editing
- shortcut conflicts with macOS reserved command
- shortcut conflicts with accessibility navigation
- shortcut bound to unavailable command

Conflict response:

- show both commands
- show active scopes
- suggest alternatives
- allow remove, replace, or scope-specific override
- provide reset to default

### Keymap Profiles

Built-in profiles:

- Damsel default
- Hopper-inspired
- Ghidra-inspired
- IDA-inspired
- Vim navigation overlay
- accessibility-minimal

Rules:

- compatibility profiles should not claim exact parity
- imported keymaps must be validated
- export should not include private project paths
- keymap profile switch should be reversible

## Theme Architecture

### Theme Token Model

Themes should define semantic tokens, not hard-coded view colors.

Token groups:

- app background
- pane background
- inspector background
- table background
- primary text
- secondary text
- disabled text
- selection background
- focus ring
- separator
- address text
- mnemonic text
- operand text
- comment text
- string text
- symbol text
- register text
- immediate text
- branch arrow forward
- branch arrow backward
- procedure byte lane
- code byte lane
- data byte lane
- string byte lane
- undefined byte lane
- PAC/fixup badge
- ObjC badge
- C++ badge
- XNU badge
- problem warning
- problem error
- confidence low
- confidence medium
- confidence high

### Built-In Themes

Minimum built-ins:

- system adaptive
- light
- dark
- high contrast light
- high contrast dark
- color-blind safe
- print/export light

Design rules:

- default should respect system appearance
- custom themes must pass contrast validation before activation
- feature-map colors must remain distinguishable under high contrast and
  color-blind safe themes
- theme changes should not reload the project or invalidate analysis
- screenshots/evidence exports may embed a theme snapshot

### Theme Package Boundary

Future plugin/theme packages may provide themes only after trust policy exists.

Theme package manifest:

```json
{
  "id": "theme.example.research-dark",
  "name": "Research Dark",
  "version": "1.0.0",
  "schema_version": 1,
  "author": "Example",
  "tokens": {},
  "minimum_damsel_version": "0.1.0"
}
```

Rules:

- theme packages cannot run code
- theme packages cannot request source access
- theme packages cannot include scripts
- invalid color tokens fall back safely
- theme import records package provenance

## Layout And Session Persistence

### Layout State Objects

Workspace layout record:

```json
{
  "layout_id": "layout:project:default",
  "project_id": "project:abc",
  "schema_version": 1,
  "active_route": "function:main:100003f20",
  "active_representation": "assembly",
  "navigator": {
    "visible": true,
    "selected_tab": "symbols",
    "filters": {
      "text": "",
      "kinds": ["function", "objc_method"]
    }
  },
  "inspector": {
    "visible": true,
    "expanded_modules": ["summary", "references", "comments"]
  },
  "workspace": {
    "tabs": [
      {
        "tab_id": "tab:1",
        "route": "function:main:100003f20",
        "representation": "assembly"
      }
    ],
    "split": "single"
  }
}
```

### Window State

Machine-local window state:

- frame
- screen id if available
- toolbar visibility
- sidebar widths
- drawer heights
- last active project id

Rules:

- restore window frame asynchronously
- clamp frame to visible screen
- if screen topology changed, center the window
- corrupt window state should be discarded without touching project state

### Route Repair

Persisted layout routes can go stale.

Repair policies:

- exact route id
- same object fingerprint
- same symbol/name
- same function entry address
- nearest containing function
- manual review
- drop route

UI:

- "Restored previous workspace"
- "Some tabs could not be restored"
- list stale tabs
- offer reopen nearest route
- offer reset layout

### Layout Profiles

Built-in layout profiles:

- triage
- deep analysis
- ObjC runtime
- C++/IOKit
- kernel/panic
- decompiler
- debugging
- patch review
- automation

Profiles set:

- visible navigators
- default representation
- inspector modules
- drawer layout
- initial search scope
- default Apple runtime browser page

Profiles do not set:

- source paths
- comments
- symbolication trust
- privacy export choices

## Settings Window UX

### Settings Tabs

Recommended tabs:

- General
- Appearance
- Assembly
- Graph
- Hex
- Search
- Analysis
- Apple Artifacts
- Keybindings
- Automation And AI
- Privacy And Diagnostics
- Advanced

### General

Controls:

- default open behavior
- recent item policy
- restore previous windows
- confirm before closing dirty project
- default layout profile
- reset layout
- reset all warnings

### Appearance

Controls:

- follow system appearance
- theme
- font family
- assembly font size
- table density
- graph density
- reduce motion
- high contrast
- color-blind safe feature-map palette

### Assembly

Controls:

- show instruction bytes
- show offsets
- address style
- operand display defaults
- jump arrow visibility
- gutter lanes
- comment lane visibility
- badge visibility

### Graph

Controls:

- default layout
- minimap
- edge labels
- node comments
- auto-layout threshold
- list fallback threshold
- animation

### Hex

Controls:

- auto columns
- fixed columns
- ASCII side column
- grouping
- edit mode default
- typed-byte lane

### Search

Controls:

- fuzzy search
- case sensitivity
- regex default
- include annotations
- include imported symbols
- include decompiler text
- result grouping
- recent boost

### Analysis

Controls:

- default profile
- concurrency
- memory budget
- background priority
- decompiler scheduling
- cache-wide indexing
- real-artifact opt-in

### Apple Artifacts

Controls:

- shared-cache roots
- KDK roots
- dSYM roots
- device symbol roots
- IPSW/OTA roots
- panic log import defaults
- mismatched sidecar policy

### Keybindings

Controls:

- profile
- search command
- edit shortcut
- conflict list
- reset command
- export profile
- import profile

### Automation And AI

Controls:

- MCP enabled
- MCP read scope
- assistant privacy mode
- comments available to assistant
- raw bytes available to tools
- tool confirmation policy
- script capability default
- plugin load policy

### Privacy And Diagnostics

Controls:

- diagnostics preset
- include comments
- include local paths
- include raw bytes
- include panic contents
- include assistant prompts
- include metrics
- preview export categories

### Advanced

Controls:

- developer metrics panel
- experimental features
- project schema debug info
- clear caches
- reset app preferences
- export settings bundle
- import settings bundle

## Quick Settings Surfaces

Some preferences should be reachable without leaving analysis.

Assembly toolbar popover:

- show bytes
- show offsets
- compact rows
- comment lane

Graph toolbar popover:

- minimap
- edge labels
- layout
- list fallback

Hex toolbar popover:

- auto columns
- fixed 16 columns
- ASCII side column
- edit lock

Search popover:

- exact/fuzzy
- scope
- include comments
- include decompiler

Diagnostics export sheet:

- redaction preset
- included categories
- preview

Command palette:

- "Toggle Instruction Bytes"
- "Use Fixed Hex Columns"
- "Switch Theme"
- "Reset Layout"
- "Open Keybindings"
- "Use Kernel Layout"
- "Enable Color-Blind Feature Map"

## API Contracts

### App Preferences

App preferences are mostly local, but the local UI API should expose validated
read/write operations so Workbench and CLI do not duplicate schema logic.

```http
GET /api/app/preferences
PATCH /api/app/preferences
GET /api/app/preferences/schema
POST /api/app/preferences/reset
```

Response:

```json
{
  "schema_version": 1,
  "preferences": {
    "appearance.theme": "system",
    "assembly.show_instruction_bytes": true,
    "search.fuzzy_enabled": true,
    "hex.column_mode": "auto"
  },
  "redacted": [],
  "problems": []
}
```

### Project Preferences

```http
GET /api/projects/{project_id}/preferences
PATCH /api/projects/{project_id}/preferences
GET /api/projects/{project_id}/preferences/effective
POST /api/projects/{project_id}/preferences/preview
```

Effective preference response should merge:

- built-in defaults
- app preferences
- project preferences
- layout profile
- command-scope override
- accessibility/system appearance

### Keymaps

```http
GET /api/app/keymaps
GET /api/app/keymaps/{profile_id}
POST /api/app/keymaps/{profile_id}/bindings
DELETE /api/app/keymaps/{profile_id}/bindings/{binding_id}
POST /api/app/keymaps/validate
POST /api/app/keymaps/import
POST /api/app/keymaps/export
```

Validation response:

```json
{
  "valid": false,
  "conflicts": [
    {
      "kind": "same_scope",
      "shortcut": "cmd+f",
      "scope": "global",
      "commands": ["search.global", "filter.current_view"]
    }
  ]
}
```

### Themes

```http
GET /api/app/themes
GET /api/app/themes/{theme_id}
POST /api/app/themes/import
POST /api/app/themes/validate
PATCH /api/app/preferences
```

Validation checks:

- schema version
- required tokens
- contrast ratios
- feature-map lane distinguishability
- invalid color values
- package provenance

### Workspace Layout

```http
GET /api/projects/{project_id}/workspace-layouts/current
PUT /api/projects/{project_id}/workspace-layouts/current
POST /api/projects/{project_id}/workspace-layouts/repair
POST /api/projects/{project_id}/workspace-layouts/reset
GET /api/projects/{project_id}/workspace-layouts/profiles
```

Repair response:

```json
{
  "restored": 3,
  "repaired": 1,
  "dropped": 1,
  "items": [
    {
      "old_route": "function:old:100003f20",
      "state": "repaired",
      "new_route": "function:new:100003f20",
      "reason": "same_object_fingerprint"
    }
  ]
}
```

## CLI Contracts

Commands:

```sh
damsel app prefs list
damsel app prefs get assembly.show-instruction-bytes
damsel app prefs set assembly.show-instruction-bytes true
damsel app prefs reset --scope assembly
damsel app keymap list
damsel app keymap validate keymap.json
damsel app keymap import keymap.json
damsel app keymap export --profile default
damsel app theme list
damsel app theme validate theme.json
damsel app theme import theme.json
damsel project prefs list --project file.damsel
damsel project prefs set --project file.damsel analysis.profile kernel-xnu
damsel project layout reset --project file.damsel
damsel project layout repair --project file.damsel
```

Rules:

- CLI should not expose raw defaults file paths as the primary API
- JSON output by default for automation
- human output only with `--human`
- sensitive preference values redacted unless explicitly requested
- invalid settings return typed problem codes

## Swift Implementation Plan

### New Stores

Add:

- `PreferenceStore`
- `ProjectPreferenceStore`
- `KeymapStore`
- `ThemeStore`
- `WorkspaceLayoutStore`
- `AccessibilityPreferenceStore`
- `DiagnosticsPreferenceStore`

Temporary migration:

- keep `WorkbenchStore` as facade where needed
- move preference state out of `WorkbenchStore`
- keep command availability dependent on `CommandStore`, not raw preference
  lookups

### `PreferenceStore`

Responsibilities:

- load defaults
- validate values
- publish effective preferences
- persist app preferences
- synchronize changes to relevant stores
- emit preference change events
- expose reset/import/export

Does not:

- own project analysis state
- own source paths requiring security bookmarks
- store raw comments
- store secrets

### `ProjectPreferenceStore`

Responsibilities:

- load project preferences
- compute app/project effective values
- preview project preference impact
- persist project preference changes through project API
- mark dirty project state when appropriate
- participate in evidence export metadata

### `KeymapStore`

Responsibilities:

- load built-in and user keymap profiles
- validate shortcut conflicts
- bind key events to command ids
- expose searchable keybinding list
- export/import profile JSON
- provide conflict UI data

### `ThemeStore`

Responsibilities:

- expose semantic color tokens
- react to system appearance
- validate imported themes
- provide contrast diagnostics
- provide feature-map lane palette
- publish theme snapshots for export

### `WorkspaceLayoutStore`

Responsibilities:

- track split/pane/tab/drawer state
- persist route-aware layout state
- repair stale routes
- reset broken layout
- clamp window geometry
- avoid blocking project open

## Rust Implementation Plan

### Project Kernel

Add project tables:

- `project_preferences`
- `workspace_layouts`
- `workspace_layout_items`
- `preference_change_events`
- `preference_effective_snapshots`

### Local UI API

Add routes:

- app preference schema
- project preference schema
- effective preference query
- layout save/load/repair/reset
- theme validation
- keymap validation

### Validation

Centralize validation in Rust for:

- project preferences
- analysis profile settings
- privacy/export policy
- source registry preference references
- workspace route repair

Swift can validate early for UI responsiveness, but Rust remains authoritative
for project-affecting settings.

## Migration Plan

### PR1: Preference Inventory And Defaults

Deliverables:

- preference key registry
- defaults table
- typed preference values
- Swift `PreferenceStore`
- read-only settings debug panel
- tests for default values

### PR2: Renderer Preferences

Deliverables:

- assembly show-bytes preference
- hex column mode preference
- search fuzzy preference
- renderer preference payload in API
- Workbench quick toggles
- snapshot tests

### PR3: Keymap Foundation

Deliverables:

- command-id based keymap model
- built-in Damsel default profile
- shortcut conflict detection
- settings keymap list
- command palette shortcut display

### PR4: Theme Tokens

Deliverables:

- semantic token model
- system/light/dark themes
- high-contrast themes
- feature-map palette validation
- theme switcher
- contrast tests

### PR5: Layout Persistence

Deliverables:

- workspace layout schema
- pane/tab/split persistence
- selected route restore
- route repair
- reset layout command
- corrupt layout fallback tests

### PR6: Project Preferences And Analysis Profiles

Deliverables:

- project preference API
- analysis profile model
- profile preview
- invalidation explanation
- performance budget settings
- project evidence snapshot

### PR7: Apple Artifact Preferences

Deliverables:

- source root registries
- KDK/dSYM/device symbol path policies
- redacted labels
- artifact-intake integration
- diagnostics redaction tests

### PR8: Privacy, Diagnostics, Automation, AI

Deliverables:

- diagnostics/export presets
- MCP/assistant privacy preferences
- capability preference UI
- audit records for high-risk policy changes
- export preview

### PR9: Preference Import/Export

Deliverables:

- settings bundle schema
- import validation
- conflict preview
- safe export redaction
- CLI import/export

## Validation Plan

Unit tests:

- default preference registry
- preference type validation
- invalid enum rejection
- keymap conflict detection
- theme token validation
- contrast validation
- layout route repair
- diagnostics redaction presets
- analysis profile validation

Snapshot tests:

- settings schema JSON
- effective preference JSON
- keymap profile JSON
- theme token JSON
- layout JSON
- diagnostics preference summary

Workbench tests:

- toggle instruction bytes updates assembly rows
- fixed hex columns survive reopen
- fuzzy search toggle changes search mode label
- keymap conflict appears in settings
- theme switch updates feature-map colors
- reset layout recovers from corrupt layout
- project open is not blocked by graph layout restore
- diagnostics export preview reflects redaction preset

Performance tests:

- preference read under 5 ms in Swift hot path
- effective preference query under 50 ms
- keymap lookup under 1 ms
- theme switch under 100 ms excluding view redraw
- layout restore first paint under 500 ms
- corrupt layout fallback under 100 ms

Manual QA:

- light/dark system switch
- high contrast mode
- reduced motion
- multiple monitors
- changed monitor topology
- large dyld cache project
- project moved to new path
- missing KDK/dSYM roots
- keyboard-only settings editing

## Diagnostics

New problem codes:

- `preferences.invalid_value`
- `preferences.unknown_key`
- `preferences.unsupported_schema`
- `preferences.sensitive_value_rejected`
- `preferences.project_override_conflict`
- `keymap.shortcut_conflict`
- `keymap.reserved_shortcut`
- `keymap.invalid_scope`
- `theme.invalid_token`
- `theme.contrast_failure`
- `theme.missing_required_token`
- `layout.stale_route`
- `layout.corrupt_state`
- `layout.restore_failed`
- `privacy.export_requires_confirmation`
- `source_root.missing`
- `source_root.permission_denied`

Diagnostic export should include:

- app version
- preference schema version
- non-sensitive effective preference summary
- redacted theme id
- keymap profile id
- layout schema version
- redaction preset
- analysis profile id
- settings-related problem codes

Diagnostic export should exclude by default:

- raw comments
- local absolute paths
- panic contents
- raw bytes
- assistant prompts
- secret tokens
- imported theme package body
- custom keymap labels if they contain private strings

## Acceptance Criteria

Documentation acceptance for this spec:

- Hopper public preference and native-UX evidence is mapped to Damsel
  subsystems.
- App preferences, project preferences, workspace state, privacy policy, and
  external source registries have separate ownership rules.
- Assembly, graph, hex, search, decompiler, analysis, Apple artifact,
  automation, diagnostics, keymap, and theme preferences are enumerated.
- API and CLI contracts exist.
- Swift and Rust implementation slices are staged.
- Performance, redaction, accessibility, and recovery requirements are explicit.

Future implementation acceptance:

- settings have a typed registry and migration path
- instruction-byte, hex-column, and fuzzy-search preferences work in the app
- keybindings bind to command ids and detect conflicts
- themes use semantic tokens and pass contrast validation
- project layout persists and repairs stale routes
- project preferences are stored separately from app preferences
- KDK/dSYM/shared-cache path policies do not leak raw paths by default
- diagnostics/export presets are previewable and redacted by default
- preference hot paths do not block navigation or rendering
- corrupt preferences/layouts fall back safely

## Risk Register

### Risk: Preferences Become Unstructured View State

Mitigation:

- typed preference registry
- schema snapshots
- app/project/session ownership checks
- no direct ad hoc `UserDefaults` access from arbitrary views

### Risk: Project Files Leak User Machine State

Mitigation:

- separate app preferences from project preferences
- redact source roots
- store sidecar identity separately from absolute path
- diagnostics default to safe redaction

### Risk: Theme Customization Breaks Semantic Meaning

Mitigation:

- semantic color tokens
- contrast validation
- feature-map lane distinguishability checks
- accessible built-in themes

### Risk: Keymap Conflicts Make The App Feel Broken

Mitigation:

- command-id registry
- scoped keymaps
- conflict validation
- disabled-command explanations
- reset-to-default profile

### Risk: Layout Restore Freezes Large Projects

Mitigation:

- first paint before expensive restoration
- lazy graph/decompiler restore
- stale-route repair after project open
- layout reset command

### Risk: Analysis Profiles Hide Expensive Work

Mitigation:

- profile preview
- invalidation explanation
- visible job queue changes
- performance metrics per profile

### Risk: Automation Preferences Become Security Theater

Mitigation:

- capability gates are enforced in Rust/API
- high-risk changes create audit records
- assistant/MCP privacy mode visible in UI
- raw bytes/comments require explicit opt-in

## First Implementation Recommendation

Do not start by building a beautiful Settings window with fake controls.

Start with:

1. typed preference registry
2. assembly instruction-byte preference
3. hex column mode preference
4. fuzzy search preference
5. command-id keymap registry
6. system/light/dark theme tokens
7. layout reset and selected-route restore
8. diagnostics redaction preset

These prove the core architecture because they touch every required boundary:
Swift view rendering, Rust API contracts, app preferences, project/session
state, command routing, theme tokens, and privacy.

After those are real, the larger settings panes become honest UI over working
state instead of promises.
