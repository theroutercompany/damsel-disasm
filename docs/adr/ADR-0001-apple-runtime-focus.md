# ADR-0001: Apple Runtime Focus

- Status: Accepted
- Date: 2026-04-13

## Context

`damsel` is currently implemented as a private Mach-O analysis tool focused on Apple Silicon binaries. The implemented present is:

- Mach-O parsing only
- `arm64` / `arm64e` thin and universal slice handling
- per-image dyld metadata extraction
- Objective-C metadata extraction
- AArch64 disassembly
- CLI and UI surfaces oriented around a single loaded `BinaryImage`

The current repo explicitly does not support ELF, PE, dyld shared cache, or other non-Mach-O formats. At the same time, the existing codebase is already strongly Apple-specific:

- `damsel-macho` owns Mach-O loading and dyld metadata extraction
- `damsel-core` models dyld exports, bindings, stubs, and Objective-C metadata
- `damsel-cli` already exposes `dyld`, `objc`, `slices`, and disassembly workflows

The next major question is whether `damsel` should broaden into a generic cross-format disassembler or deepen into a more complete Apple runtime analysis tool.

## Decision

`damsel` is an Apple-platform analysis tool, not a cross-format disassembler.

The project focus is locked around:

- Mach-O executable and dylib analysis
- dyld metadata and runtime-linkage understanding
- dyld shared cache inspection and query workflows
- symbolication and address-resolution workflows
- debugging-oriented binary/runtime research
- system-framework spelunking for Apple platforms

The immediate and near-term direction is depth, not breadth.

## Non-Goals

This direction explicitly does not commit the project to:

- ELF support
- PE / COFF support
- generic cross-platform binary analysis
- non-Apple runtime loader support
- public/stable support guarantees in this pass
- public release positioning in this pass

## Alternatives Considered

### 1. Generic multi-format disassembler

Expand the tool into a broader binary-analysis surface across Mach-O, ELF, and PE.

Rejected because:

- it would split implementation effort across incompatible loader, relocation, symbol, and runtime-linkage models
- it would dilute verification depth on the Apple-specific workflows the current codebase already serves well
- it would make dyld and shared-cache work compete with unrelated format expansion

### 2. Remain Mach-O only and avoid dyld shared cache

Continue deepening only standalone Mach-O image analysis and stop at existing per-image dyld metadata.

Rejected because:

- modern Apple runtime understanding increasingly depends on shared-cache-backed system images
- symbolication, debugging, and framework spelunking become much less useful without cache awareness
- the current dyld-oriented implementation naturally points toward cache-aware expansion

### 3. Debugger-first tooling before shared-cache inspection

Prioritize process/debugger integration before building shared-cache inventory and query primitives.

Rejected because:

- debugger-oriented features still need authoritative cache/image/symbol/address models underneath
- shared-cache inspection is the better foundation for later debugging and symbolication workflows

## Consequences

Positive consequences:

- the codebase can optimize for Apple runtime fidelity instead of generic abstractions
- dyld and shared-cache work can extend existing concepts instead of being bolted on
- documentation, tests, fixtures, and release posture can stay honest and focused
- future debugging and symbolication features have a clear runtime model to build on

Costs and tradeoffs:

- non-Apple format support is intentionally deprioritized
- some future abstractions may be Apple-specific by design
- shared-cache work will increase loader/model complexity and test-corpus demands
- release and support docs must keep stating that planned cache/runtime surfaces are not yet implemented

## Directional Implications

- `BinaryImage` remains the implemented present for per-image workflows.
- dyld shared cache support will be added as an Apple-runtime expansion, not as a generic container framework.
- future CLI growth should preserve current Mach-O commands and add Apple-runtime-specific surfaces beside them.

## See Also

- [ADR-0002: Shared-Cache Container Model](./ADR-0002-shared-cache-container-model.md)
- [Apple Runtime Analysis Architecture](../architecture/apple-runtime-analysis-architecture.md)
- [dyld Shared Cache v1 Spec](../specs/dyld-shared-cache-v1.md)
- [Apple Runtime Analysis Roadmap](../roadmaps/apple-runtime-analysis-roadmap.md)
