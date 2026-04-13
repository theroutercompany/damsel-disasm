# ADR-0002: Shared-Cache Container Model

- Status: Accepted
- Date: 2026-04-13

## Context

The implemented present is centered on one loaded image at a time:

- `damsel-macho` loads a file or in-memory byte buffer into a `BinaryImage`
- `BinaryImage` carries sections, symbols, imports, relocations, ObjC metadata, and `DyldMetadata`
- CLI commands operate on a single Mach-O image path

That model is a good fit for standalone Mach-O binaries, but it is not sufficient for dyld shared cache analysis. A shared cache is not just "one more Mach-O file":

- it is a container of many runtime images
- it has cache-level mappings and address semantics
- useful workflows are often cache-first rather than image-first
- symbolication and system-framework spelunking require address-to-image-to-symbol resolution across the whole cache

The key architecture choice is whether to treat the shared cache as:

- an extract-first source of one image at a time, or
- a first-class runtime container with image inventory/query APIs

## Decision

dyld shared cache support is container-first.

The future model is:

- a `SharedCacheSource` resolves the on-disk cache input set
- a `SharedCache` represents the cache as a first-class analysis container
- `CacheImageRecord` values describe images inside the cache
- selected cache images can later be projected into `BinaryImage`-compatible views
- cache inventory, lookup, and resolution workflows operate on the container directly

The shared cache is not treated as a bag of standalone Mach-O files.

The future CLI direction is also locked:

- add a new top-level `cache` command family
- do not overload existing `dyld` commands with cache-container responsibilities

## Alternatives Considered

### 1. Extract-first model

Treat the cache mainly as a place to pull out a single image, then route everything through existing per-image flows.

Rejected because:

- address-resolution and symbolication become awkward and lossy
- the container-level mapping and image inventory semantics remain implicit
- important workflows would still need a separate cache index layer later
- it encourages repeated ad hoc extraction instead of a durable shared model

### 2. Extend `dyld` as the cache surface

Keep the CLI flatter by adding cache features to the existing `dyld` command family.

Rejected because:

- `dyld` today is per-image metadata presentation
- cache inventory and query work is operationally different from per-image dyld inspection
- mixing container and image semantics would make future CLI behavior harder to reason about

### 3. Hybrid from day one

Require both a full container model and general projected-image integration in the first wave.

Rejected because:

- it front-loads too much implementation risk
- a read-only inventory/query foundation should be established before broad projection flows are promised

## Consequences

Positive consequences:

- cache-wide address and image resolution can be implemented directly
- symbolication features have a clean home
- projected-image workflows can reuse current per-image analysis later without collapsing abstractions
- CLI growth stays understandable: image-first commands remain image-first, cache-first commands remain cache-first

Costs and tradeoffs:

- the system gains a second major runtime abstraction beside `BinaryImage`
- shared cache parsing, sidecar discovery, and indexing will require new typed models and tests
- there will be an intermediate period where cache queries exist before general cache-backed disassembly is exposed

## Directional Implications

- `BinaryImage` remains the per-image abstraction for implemented and future projected-image flows.
- `SharedCache` is a sibling abstraction, not a replacement for `BinaryImage`.
- projected images should preserve current Mach-O analysis semantics where possible.
- the first implementation wave should prioritize inventory, address lookup, and symbol resolution over generalized cache-native disassembly UX.

## See Also

- [ADR-0001: Apple Runtime Focus](./ADR-0001-apple-runtime-focus.md)
- [Apple Runtime Analysis Architecture](../architecture/apple-runtime-analysis-architecture.md)
- [dyld Shared Cache v1 Spec](../specs/dyld-shared-cache-v1.md)
- [Apple Runtime Analysis Roadmap](../roadmaps/apple-runtime-analysis-roadmap.md)
