# Apple Runtime Analysis Roadmap

Status: Planned. This roadmap describes intended implementation waves. Shared-cache-specific items below are planned unless explicitly marked as current.

## Summary

This roadmap turns `damsel` from a Mach-O-first image analyzer into a stronger Apple runtime analysis tool in three waves:

1. shared-cache inventory/query/symbolication MVP
2. projected-image workflows and debugger-oriented resolution
3. deeper shared-cache-native analysis and system-framework spelunking

The roadmap preserves:

- Apple-only focus
- Mach-O-first foundations
- `arm64` / `arm64e` boundary in the first shared-cache wave
- current standalone Mach-O behavior as the compatibility baseline

## Wave 1: Shared-Cache Inventory, Query, and Symbolication MVP

### Themes

- make the dyld shared cache loadable as a first-class container
- expose deterministic image inventory and mappings
- support cache-wide address lookup and symbol/export resolution
- ship the first `cache` command family

### Major Subsystems Touched

- `damsel-core`
  - shared-cache model types and indexes
  - typed symbolication results
- `damsel-macho`
  - deterministic cache-set discovery
  - cache/container parsing
  - image inventory and mapping indexes
  - projected-image internals for metadata reuse
- `damsel-cli`
  - `cache info`
  - `cache images`
  - `cache image`
  - `cache exports`
  - `cache lookup-address`
  - `cache resolve-symbol`
- fixtures / CI
  - shared-cache corpus hash checks
  - typed malformed/incomplete/ambiguous coverage

### Verification Expectations

- typed loader tests for valid, malformed, incomplete, and unsupported-architecture cache sets
- split-cache discovery tests with deterministic member ordering
- deterministic image-list tests
- address lookup tests for `exact_symbol`, `nearest_symbol`, and `mapping_only`
- symbol/image ambiguity tests with `cache_image_ambiguous`
- CLI JSON contract tests for all six cache commands with exact key ordering
- text snapshots for representative success and typed failure flows

### Explicit Non-Goals

- generalized cache-backed disassembly
- debugger transport or live-process integration
- cache mutation or rebuilding
- non-Apple formats
- `doctor` capability expansion

### Exit Criteria

- one cache set can be opened and queried read-only
- image inventory is deterministic
- address lookup returns mapping/image/symbol context with locked address vocabulary:
  - `cache_vmaddr`
  - `image_base_vmaddr`
  - `image_offset`
  - `member_file_offset`
- symbol resolution works across the cache with stable text/JSON output
- current standalone Mach-O commands remain unchanged
- root docs are truthful about implemented versus planned shared-cache surfaces

## Wave 2: Projected-Image Workflows and Debugger-Oriented Resolution

### Themes

- bridge from cache container to per-image analysis
- allow selected cache images to reuse existing Mach-O analysis workflows
- improve debugging-oriented address and symbol resolution quality

### Major Subsystems Touched

- `damsel-macho`
  - projected image materialization
  - cache-to-image section/symbol projection
- `damsel-core`
  - typed projected-image identity and provenance
  - richer symbolication result types
- `damsel-cli`
  - cache-selected image workflows that reuse current renderers
  - debugger-oriented lookup output and contextual hints
- UI
  - optional image browser/selection flows if UI direction remains active

### Verification Expectations

- projection-fidelity tests against equivalent standalone Mach-O fixtures where possible
- section/symbol/dyld parity checks on projected images
- disassembly smoke tests for projected images
- regressions proving standalone image workflows remain unchanged

### Explicit Non-Goals

- full debugger transport integration
- process attachment/injection
- cache rebuilding or mutation
- architecture expansion outside the Apple-focused direction

### Exit Criteria

- selected cache images can be projected into per-image analysis views
- a limited set of existing image-oriented analysis flows operates on projected images without format-specific rewrites
- debugging-oriented resolution is materially better than Wave 1 inventory/query workflows

## Wave 3: Shared-Cache-Native Analysis and System-Framework Spelunking

### Themes

- make cache-native exploration first-class
- deepen framework spelunking and runtime-linkage understanding
- improve research-oriented navigation across cache-contained frameworks

### Major Subsystems Touched

- `damsel-core`
  - richer cross-image relation models
  - cache-native symbol/reference structures
- `damsel-macho`
  - deeper cache-native dyld and linkage analysis
  - cross-image import/export/reexport navigation
- `damsel-cli`
  - richer cache-native explorer commands and filters
- UI
  - higher-value browsing/navigation surfaces if still aligned with product direction

### Verification Expectations

- cross-image linkage tests
- framework-boundary traversal tests
- richer cache-native snapshots
- fixture-matrix expansion for framework-spelunking and symbolication scenarios

### Explicit Non-Goals

- abandoning standalone Mach-O workflows
- generic non-Apple binary analysis
- operational system-modification tooling

### Exit Criteria

- cache-native workflows are useful without immediate projection into standalone-image semantics
- system-framework spelunking is meaningfully supported
- symbolication/address-resolution quality is sufficient for low-level Apple runtime research workflows

## Cross-Wave Rules

- keep current Mach-O image workflows truthful and stable
- keep shared-cache work read-only until a separate decision says otherwise
- keep non-Apple formats out of the near-term roadmap
- preserve typed failure classes and deterministic CLI/JSON output
- keep release and README docs honest about implemented versus planned runtime surfaces

## Present vs Planned Boundary

Implemented present:

- standalone Mach-O analysis
- dyld metadata extraction for standalone images
- image-oriented CLI and UI flows

Planned future:

- all shared-cache-specific surfaces in this roadmap

## Cross References

- [ADR-0001: Apple Runtime Focus](../adr/ADR-0001-apple-runtime-focus.md)
- [ADR-0002: Shared-Cache Container Model](../adr/ADR-0002-shared-cache-container-model.md)
- [Apple Runtime Analysis Architecture](../architecture/apple-runtime-analysis-architecture.md)
- [dyld Shared Cache v1 Spec](../specs/dyld-shared-cache-v1.md)
