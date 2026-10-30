# damsel

`damsel` is a cross-platform binary analysis and reverse-engineering CLI
for macOS (Mach-O) and Linux (ELF) binaries.

Current analysis scope:
- **Mach-O**: standalone parsing plus dyld shared-cache inventory, symbolication,
  projected-image analysis, and cache-native linkage exploration (Waves 1–3).
- **ELF**: typed ELF64-LE loading, dynamic linkage (versioned symbols, PLT/GOT,
  IFUNC), relocations, build-id, and segment-only parsing for hard-stripped
  binaries (Waves 4–5).
- **ISAs**: arm64/arm64e and x86_64 with a shared analysis pipeline.
- **Decompiler**: D0–D5 pipeline (summaries, IR, SSA, stack frames,
  structuring, pseudocode renderer) with provenance-carrying output.
- **Automation**: batch analysis, binary diffing, recon triage, and an MCP
  server for AI agents.

## Commands

### Analysis
```sh
damsel analyze <binary>           # Create a project database
damsel functions <project>        # List discovered functions
damsel xrefs <project> --to 0x…   # Cross-references (both directions)
damsel strings <project>          # Harvested strings
damsel search <project> <query>   # Unified search (functions/symbols/imports/strings)
```

### Decompiler
```sh
damsel summary <project> --function _main          # D0 function summary
damsel summary <project> --function _main --markdown # Markdown report
damsel decompile <project> --function _main        # Pseudocode with provenance
```

### Triage & Automation
```sh
damsel recon <binary>             # Entropy, packers, anti-analysis indicators
damsel batch --inputs list.txt --out results/  # Corpus sweep (JSONL)
damsel diff <left> <right>        # Function-level binary comparison
damsel diff <left> <right> --markdown  # Markdown report
damsel diff <left> <right> --html      # Self-contained HTML report
damsel serve                      # MCP server over stdio (7 tools)
```

### Project Management
```sh
damsel project show <project>     # Metadata, sources, fact counts
damsel project export <project>   # Canonical deterministic export
```

### Mach-O Specific
```sh
damsel info|sections|symbols|imports|relocs|dyld|slices|objc <binary>
damsel disasm <binary> --symbol _main [--analysis] [--show-values]
damsel cache <subcommand> <cache> # 17 shared-cache subcommands
damsel doctor                     # Host/tool readiness
```

## Quality on Real Binaries

Decompiler coverage on production glibc (NixOS):

| Function | Lines | Unknowns |
|---|---|---|
| `_int_malloc` | 106 | **0** |
| `__GI_getenv` | 53 | **0** |
| `__GI_memcpy` | 12 | **0** |
| `__assert` | 7 | **0** |
| `_dl_start` | 6 | **0** |
| `__GI_strlen` | 14 | 1 |
| `__GI_memset` | 29 | 2 |
| `__GI___libc_malloc` | 44 | 1 |

General-purpose C functions decompile at near-zero unknown rate.
SIMD-optimized variants (SSE2/AVX) have higher rates because vector
instructions are honestly reported as unknown — nothing is fabricated.

Function recovery on real binaries: `ls` (1,798 functions, 78,142 xrefs),
`libc.so.6` (3,953 functions, 80,765 xrefs).

## MCP Server

`damsel serve` implements the Model Context Protocol over stdio with 7 tools:
`damsel_analyze`, `damsel_functions`, `damsel_xrefs`, `damsel_search`,
`damsel_decompile`, `damsel_recon`, `damsel_diff`. Tools return the same
typed JSON as the CLI.

## UI Server

`damsel ui` starts a local web server with:
- Binary upload and disassembly browsing (Mach-O)
- Project-backed analysis endpoints: functions, xrefs, decompile, search,
  strings, summary
- A native macOS Workbench app (`damsel-app/`) that wraps the same server

## Support Matrix

| Host | Analysis | Fixture rebuild | Bench |
|---|---|---|---|
| macOS (arm64/x86_64) | Supported | Supported | Supported |
| Linux x86_64 | Supported | Supported (ELF) | Supported |
| Linux arm64 | Supported | Supported (ELF) | Supported |
| Windows | Degraded | Unsupported | Degraded |

## Verification

```sh
cargo test --workspace                     # 51 suites
bin/function-recovery-audit.sh            # recall≥95%, fp≤2% gate
bin/elf-evidence-package.sh <outdir>      # self-verifying evidence package
bin/disassembler-v2-alpha-validate.sh     # full validation sweep
```

## Documentation

- [RE Power Roadmap](./docs/roadmaps/re-power-roadmap.md) — accepted program
- [RE Power Pack](./docs/re-power/README.md) — specs, audit, evidence matrix
- [Implementation Log](./docs/re-power/implementation-log.md) — live status
- [Apple Runtime Roadmap](./docs/roadmaps/apple-runtime-analysis-roadmap.md) — Waves 1–3
- [Hopper Gap Pack](./docs/hopper-gap/README.md) — Apple-track specs
- [ADR-0003](./docs/adr/ADR-0003-cross-platform-re-scope.md) — cross-platform scope

## Local Development

```sh
cargo test --workspace
nix develop                              # pinned toolchain
nix build .#default                      # build the CLI
```

## Release

Private prereleases (nightly + beta). See [RELEASE.md](./RELEASE.md).
