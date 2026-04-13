# dyld Shared Cache v1 Engineering Spec

Status: Planned. This is a decision-complete spec for the first shared-cache implementation wave. None of the cache-specific surfaces in this document are implemented yet.

## Summary

v1 adds **read-only dyld shared cache inspection** to `damsel` for Apple Silicon cache sets.

v1 goals:

- open a dyld shared cache as a first-class container
- enumerate cache images and mappings
- inspect one selected cache image's metadata
- inspect exports for a selected cache image
- resolve cache VM addresses to image/symbol context
- resolve symbols across the cache for symbolication and spelunking workflows

v1 does **not** promise general cache-native disassembly or full debugger integration.

## Current Implemented Present

Already implemented today:

- standalone Mach-O loading into `BinaryImage`
- typed dyld metadata on `BinaryImage`
- image-oriented CLI commands

Not implemented today:

- any shared-cache loader
- cache container model
- cache image inventory or lookup
- cache VM address resolution
- cache symbolication CLI

## v1 Scope

Included:

- read-only cache inspection
- `arm64` / `arm64e` cache sets only
- deterministic cache-set discovery from a provided file path
- typed cache image inventory
- typed cache address lookup
- typed cross-image symbol/export lookup
- a new `cache` CLI command family

Excluded from v1:

- ELF or PE support
- write/edit/rebuild cache flows
- process attachment or debugger integration
- generalized cache-backed `disasm` / `dyld` / `objc` / `info` reuse from the existing top-level commands
- shared-cache mutation, patching, or signing workflows

## Crate Ownership

v1 uses the existing workspace split.

- `damsel-core`
  - owns shared data model types for cache metadata and resolution results
- `damsel-macho`
  - owns shared-cache parsing, cache-set discovery, cache image inventory, and projected-image plumbing
- `damsel-cli`
  - owns the `cache` command family and rendering

Do not introduce a fourth workspace crate in v1.

## Planned Internal Types

### `SharedCacheSource`

Purpose:

- represent the on-disk source for one cache set

Required responsibilities:

- accept a path to any cache member file
- canonicalize the cache root/member set deterministically
- discover sibling subcaches needed for a complete view
- record the ordered member list used for analysis

Minimum fields:

- `input_path: PathBuf`
- `canonical_root_path: PathBuf`
- `member_paths: Vec<PathBuf>`
- `local_symbols_paths: Vec<PathBuf>`

### `SharedCacheHeader`

Purpose:

- expose cache-level identity and summary metadata

Minimum fields:

- `architecture: Architecture`
- `platform: Option<Platform>`
- `uuid: String`
- `mapping_count: usize`
- `image_count: usize`
- `subcache_count: usize`
- `local_symbols_present: bool`

### `CacheImageId`

Purpose:

- provide stable internal identity for one cache-contained image

Rule:

- `CacheImageId` is a stable string: `<cache_uuid>:<image_index>`
- `image_index` is the cache image-table order and must not depend on display sorting

### `CacheImageRecord`

Purpose:

- describe one cache image without requiring immediate projection into `BinaryImage`

Minimum fields:

- `id: CacheImageId`
- `image_index: u32`
- `install_name: String`
- `uuid: Option<String>`
- `image_base_vmaddr: u64`
- `end_vmaddr: u64`
- `member_path: PathBuf`
- `mapping_names: Vec<String>`

### `ProjectedImageView` / `ProjectedBinaryImage`

Purpose:

- adapt one cache image into a per-image analysis view that can later reuse existing image-oriented analysis flows

v1 rule:

- projection support may exist internally in v1, but broad user-facing reuse of top-level image commands is deferred

### `AddressResolver`

Purpose:

- resolve a cache VM address to container, image, and symbol/export context

Minimum responsibilities:

- identify containing cache mapping
- identify containing `CacheImageRecord` when one can be resolved
- resolve exact symbol/export match when available
- return nearest lower symbol/export and offset when exact match is absent

### `SymbolicationResult`

Purpose:

- return one typed lookup result for `lookup-address` and `resolve-symbol`

Lookup result kinds:

- `exact_symbol`
- `nearest_symbol`
- `mapping_only`

Minimum fields:

- `query_kind`
- `query_value`
- `cache_uuid`
- `image_id: Option<CacheImageId>`
- `install_name: Option<String>`
- `cache_vmaddr`
- `image_base_vmaddr: Option<u64>`
- `image_offset: Option<u64>`
- `member_file_offset`
- `exact_symbol: Option<String>`
- `nearest_symbol: Option<String>`
- `symbol_offset: Option<u64>`
- `export_kind: Option<ExportKind>`

## Input Model and Deterministic Discovery

The CLI accepts a `<cache>` argument for every cache command.

v1 discovery algorithm:

1. `<cache>` may point to root cache file, numbered subcache member, or `.symbols` sidecar.
2. Root-candidate derivation:
   - if basename ends with `.symbols`, strip `.symbols`
   - else if basename ends with `.<decimal>`, strip that numeric suffix
   - else treat the input path as root candidate
3. Parse the root candidate and use `subcache_suffixes()` order as the authoritative required member list.
4. Member ordering is fixed as:
   - root first
   - required numbered members in `subcache_suffixes()` order
   - optional `.symbols` sidecar last, only if present and UUID-compatible
5. Missing required members fail with `cache_incomplete`.
6. UUID mismatch across required members fails with `cache_incomplete`.

## CLI Contract

All cache commands are **planned**, not implemented yet.

### `damsel cache info <cache>`

Behavior:

- open the cache set
- print cache-level metadata only

Text output must include:

- canonical root path
- architecture
- platform if known
- cache UUID
- image count
- mapping count
- subcache count
- local-symbol availability

JSON contract:

- `command = "cache_info"`
- `data.header`
- `data.members`

### `damsel cache images <cache>`

Behavior:

- list cache-contained images in deterministic order

Default order:

- ascending `install_name`

Supported filters/options in v1:

- `--name <substring>` (ASCII case-insensitive substring)
- `--limit <count>`
- `--sort name|address`

Limit semantics:

- apply operations in this order: filter -> sort -> limit

JSON contract:

- `command = "cache_images"`
- `data.metadata.total`
- `data.metadata.returned`
- `data.metadata.truncated`
- `data.images[]`

### `damsel cache image <cache> <image>`

Behavior:

- resolve one image and print metadata for that image only

Image resolution rules, in order:

1. exact `CacheImageId` match (case-sensitive)
2. exact install-name match (case-sensitive)
3. exact basename match if unique (case-sensitive)

Failure semantics:

- no match -> `cache_image_not_found`
- basename ambiguity -> `cache_image_ambiguous`

JSON contract:

- `command = "cache_image"`
- `data.image`

### `damsel cache exports <cache> <image>`

Behavior:

- resolve one cache image
- enumerate its exported symbols

Supported filters/options in v1:

- `--name <substring>` (ASCII case-insensitive substring)
- `--kind <regular|reexport|resolver|stub-and-resolver|weak-definition|absolute|thread-local|unknown>`
- `--flag <weak-definition|reexport|stub-and-resolver|thread-local|absolute>`
- `--sort address|name`

JSON contract:

- `command = "cache_exports"`
- `data.image`
- `data.metadata.total`
- `data.metadata.returned`
- `data.metadata.truncated`
- `data.exports[]`

### `damsel cache lookup-address <cache> <vmaddr>`

Behavior:

- parse `<vmaddr>` with the same address parser style as current CLI commands
- interpret input as `cache_vmaddr`
- resolve mapping/image/symbol context

Result semantics:

- `exact_symbol`, `nearest_symbol`, `mapping_only`
- if mapped but no containing image is identified: return `mapping_only` success
- if not mapped: fail with `address_not_mapped`

JSON contract:

- `command = "cache_lookup_address"`
- `data.result`

### `damsel cache resolve-symbol <cache> <symbol>`

Behavior:

- resolve symbol/export matches across the whole cache
- symbol name matching is exact and case-sensitive

Supported options in v1:

- `--image <substring>` (ASCII case-insensitive substring on install name)
- `--limit <count>`

Default order:

- install name ascending, then symbol name ascending

Limit semantics:

- apply operations in this order: filter -> sort -> limit

JSON contract:

- `command = "cache_resolve_symbol"`
- `data.metadata.total`
- `data.metadata.returned`
- `data.metadata.truncated`
- `data.matches[]`

## Address Semantics

This spec locks four address classes:

- `cache_vmaddr`
  - absolute VM address in the shared-cache address space
- `image_base_vmaddr`
  - image base address for the resolved `CacheImageRecord`
- `image_offset`
  - `cache_vmaddr - image_base_vmaddr`
- `member_file_offset`
  - absolute file offset inside the owning cache member file

Rules:

- `lookup-address` inputs are always interpreted as `cache_vmaddr`
- outputs must label all address classes explicitly
- text output must never print unlabeled addresses when more than one address class is in play
- JSON output must use the exact field names above

## Image Identity Rules

- every `CacheImageRecord` exposes both `id` and `install_name`
- `install_name` is primary user-facing identity
- `CacheImageId` is stable machine identity
- basename lookup is convenience behavior and must only succeed when unique

## Failure Model

v1 must use typed failures end-to-end.

Planned library-side failure classes:

- `UnsupportedInputKind`
- `UnsupportedSharedCacheArchitecture`
- `MalformedSharedCache`
- `IncompleteSharedCacheSet`
- `CacheImageNotFound`
- `CacheImageAmbiguous`
- `AddressNotMapped`
- `SymbolNotFound`

Planned CLI error envelope mapping:

- `unsupported_input`
- `unsupported_architecture`
- `malformed_input`
- `cache_incomplete`
- `cache_image_not_found`
- `cache_image_ambiguous`
- `address_not_mapped`
- `symbol_not_found`

Rules:

- missing required subcache members map to `cache_incomplete`
- malformed cache headers/mappings/image tables map to `malformed_input`
- unsupported architecture maps to `unsupported_architecture`
- optional local-symbol sidecar absence must degrade lookup quality rather than fail commands in v1

## Output Principles

Text mode:

- concise, operator-readable, explicit about address classes
- deterministic ordering
- ambiguity and partial-resolution outcomes are explicit

JSON mode:

- deterministic key ordering
- additive evolution only
- every record includes enough identity to be joined later (`cache_uuid`, `image_id`, `install_name` where applicable)
- collection commands always include `metadata.total`, `metadata.returned`, `metadata.truncated`

## Verification Requirements

The first implementation wave should include:

- typed parser tests for supported and malformed cache sets
- fixture coverage for split-cache discovery and missing-member handling
- deterministic image-inventory tests
- exact and nearest address-resolution tests
- mapped-without-image `mapping_only` tests
- symbol lookup ambiguity tests (`cache_image_ambiguous`)
- CLI JSON contract tests for every `cache` subcommand
- text snapshots for representative happy-path and failure-path flows

## Explicit Deferrals

Deferred beyond v1:

- projecting cache images into current top-level `info`, `symbols`, `dyld`, `objc`, and `disasm` commands as a general UX promise
- debugger integration or live-process symbolication
- cache mutation or rebuilding
- cross-platform or non-Apple format expansion

## Cross References

- [ADR-0001: Apple Runtime Focus](../adr/ADR-0001-apple-runtime-focus.md)
- [ADR-0002: Shared-Cache Container Model](../adr/ADR-0002-shared-cache-container-model.md)
- [Apple Runtime Analysis Architecture](../architecture/apple-runtime-analysis-architecture.md)
- [Apple Runtime Analysis Roadmap](../roadmaps/apple-runtime-analysis-roadmap.md)
