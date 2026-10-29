# Research Sources

Status: External and internal source record for the RE Power program.
**Live-verified 2026-10-28** across two research rounds (30+ direct
primary-source fetches: docs.rs, crates.io API, GitHub raw at exact pinned
tags and mirrored official trees, vendor COPYING files, spec sites, and the
Wayback Machine for drifted URLs) — see §0. Two corrections found and
applied (capstone feature semantics, MCP transports); ABI/toolchain claims
are now line-cited where the host allowed. The organization's web-search
connector remains disabled (a free tier exists; enabling is an owner
action), so search-engine retrieval was not used; direct fetches were.

Verification levels:

- **V✓ — live-verified**: fetched from the primary source on 2026-10-28;
  method and URL recorded in §0 or the row itself.
- **V2 — stable**: long-lived public facts not re-fetchable this pass
  (canonical host blocked or URL drifted); spot-check before external use.
- **V3 — to verify**: version-specific or fast-moving; MUST be re-checked
  before an exit review relies on it.

## 0. Verification Log (2026-10-28)

Fetched directly (all via HTTPS, one session):

| Source | Method | Outcome |
| --- | --- | --- |
| `capstone-rs` `Cargo.toml` at tag `capstone-v0.14.0` (exact pinned version) | raw.githubusercontent.com | **Correction**: `full` = non-Diet mode only; architectures come from `arch_*`/`support_all_archs` features. Our workspace (`default-features=false`, `arch_arm64` only) does **not** compile the x86 backend today |
| `capstone` crate features page + crates.io API | docs.rs, crates.io | capstone 0.14.0 current; `arch_x86` feature exists and maps to `capstone-sys/arch_x86` |
| `iced-x86` module index + crate page | docs.rs | 1.21.0; `Encoder`, `BlockEncoder`, `CodeAssembler` structs exist; license MIT — Wave 8 encoder plan **verified and upgraded** |
| `rusqlite` features page | docs.rs | 0.41.1 (current); `bundled` feature exists — D-3 store choice verified |
| `gimli` all-items + method index | docs.rs | 0.34.0; `read::EhFrame` with `entries()`, `fde_for_address`, `cie_from_offset`; `EhHdrTable` — FDE seeding API verified |
| `object` `read::elf` module index | docs.rs | 0.40.0; `VersionTable`, `DynamicRelocationIterator`, `DynamicTable`, `SymbolTable` — ELF version/dynamic parsing verified |
| `goblin` `elf` module index | docs.rs | 0.10.7; `VersymSection`, `VerneedSection`, `VerdefSection`, `Dynamic`, `RelocSection` verified |
| License badges | docs.rs crate pages | cpp_demangle, rustc-demangle, gimli, object: MIT OR Apache-2.0; goblin: MIT/ISC; capstone: MIT; iced-x86: MIT |
| Keystone `COPYING` | raw.githubusercontent.com | **GPL-2.0 confirmed** from the license text itself |
| crates.io search: demanglers | crates.io API | `cxxfilt` does **not** exist as a crate; verified alternatives in §3 |
| MCP specification (2026-07-28) navigation + transports | modelcontextprotocol.io | Transports are **stdio + Streamable HTTP**; the old "HTTP with SSE" transport is not in the current spec (see Deprecated Features) — Wave 7 wording corrected |
| Competitor homepages | direct fetch | hex-rays.com (IDA Pro title verified), ghidra-sre.org (redirects to NSA GitHub), binary.ninja reachable; rizinorg.github.io returned no title (JS-rendered) |

Fetches attempted and **blocked** this pass (recorded honestly; facts below
keep their prior status):

- Oracle Linkers and Libraries Guide: URLs drifted (404) — versym/verneed
  structure stays V1-by-synthesis (gABI/LSB stable) but unverified by fetch.
- sourceware glibc wiki IFUNC page: moved (404).
- Itanium C++ ABI official site: TLS failure direct — retrieved via Wayback
  snapshot instead (verified, see Round 2).

### Round 2 (2026-10-28, continued)

| Source | Method | Outcome |
| --- | --- | --- |
| x86-64 psABI (official hjl-tools tree via GitHub mirror `honggyukim/x86-64-ABI`) | raw.githubusercontent.com, chapter `.tex` files | **V✓ with line cites**: red zone (`-128(%RSP)`, "known as the red zone", low-level-sys-info.tex ~507/537); callee-saved RBX/RBP/r12–r15 incl. "RBP optionally frame pointer" and "R15 optionally GOT base" (~737–750); `R_X86_64_IRELATIVE` = 37, "similar to R_X86_64_RELATIVE… to avoid name…" (object-files.tex ~483/638); `GLOB_DAT` = 6, `JUMP_SLOT` = 7 (~457) |
| GNU ld `Options` (sourceware via Wayback 2025-01-04) | web.archive.org | **V✓**: `-z ibtplt` "Generate Intel IBT enabled PLT entries"; `-z ibt` sets `GNU_PROPERTY_X86_FEATURE_1_IBT` in `.note.gnu.property` and implies `ibtplt`; `-z shstk`; `-z relro` creates `PT_GNU_RELRO` |
| LLVM `lld` x86-64 backend source | raw.githubusercontent.com | **V✓**: `.plt.sec` name and dual-PLT layout verified in-source ("one in .plt.sec (writePlt), and the other in .plt (writeIBTPlt)") |
| LSB 5.0 Core, eh_frame chapter | refspecs.linuxbase.org | **V✓**: CIE/FDE record structure, FDE `initial location` field (the function-start fact FDE seeding relies on), augmentation encoding |
| Itanium C++ ABI (official `itanium-cxx-abi.github.io`) | Wayback snapshot | **V✓**: vtable group, address point, "typeinfo pointer… always present"; mangling `TV <type> # virtual table`, `TT # VTT structure` |
| Swift `docs/ABI/Mangling.rst` (swiftlang/swift main) | raw.githubusercontent.com | **V✓**: `mangled-name ::= '$s' global` (stable mangling, Swift 4.0+) |
| Rust symbol mangling v0 (RFC 2603) | raw.githubusercontent.com | **V✓**: `_RN…` grammar, legacy-vs-v0 |
| LLVM `llvm-readelf` / `llvm-objdump` command guides | llvm.org | **V✓**: readelf `-s/--symbols`, `-r/--relocs`, `-V/--version-info`, `-W/--wide`, `--dyn-syms`; objdump `-d`, `--start/--stop-address`, `--macho` |
| MCP server/tools spec (2026-07-28) | modelcontextprotocol.io | **V✓**: tools return `structuredContent`; optional `outputSchema` (JSON Schema 2020-12); `annotations` for trust/safety — Wave 7 tool contract maps 1:1 |
| `sqlite.org/pragma.html` | direct | **V✓**: `user_version` pragma exists as documented (D-3 migration mechanism) |
| ByteWeight (USENIX Security '14) | usenix.org | **V✓ page exists**: "Learning to Recognize Functions in Binary Code" (function-start ML baseline for Wave 4 heuristics) |
| Diaphora README | raw.githubusercontent.com | **V✓**: diff heuristics — "dozens of heuristics based on graph theory, assembler, bytes, functions' features", "similarity ratio", "pseudo-code based heuristics" |
| Obfuscator-LLVM wiki (Features) | raw.githubusercontent.com (wiki) | **V✓**: exactly three passes — Instructions Substitution (`-sub`), Bogus Control Flow (`-bcf`), Control Flow Flattening (`-fla`) — the classes Wave 7 recon detects |
| UPX source (`p_lx_elf.cpp`) | raw.githubusercontent.com (devel) | **V✓**: ELF pack-header layout comment "checksum UPX! lsize version format" — the `UPX!` magic recon matches |
| Workspace dependency freshness | crates.io API | All 9 workspace deps resolved: capstone pinned=current; object 0.39→**0.40** available (breaking-range; adopt at Wave 5B — its `VersionTable` API was verified on 0.40); reqwest 0.12→0.13, criterion 0.5→0.8 breaking-range available; axum/clap/insta/serde/tokio minor updates available. No emergencies |

### Round 3 (2026-10-28, foundation de-risking)

| Source | Method | Outcome |
| --- | --- | --- |
| **Empirical x86 backend spike** (scratch crate, compiled and run locally: capstone 0.14 `std+full+arch_x86` vs iced-x86 1.21, 12-instruction x86-64 window incl. `push rbp; mov rbp,rsp; lea [rip+…]; call rax; add; cmp; jne; ret; syscall`) | cargo build + run | **Answers Wave 5 Open Question 1**: capstone x86 detail is sufficient for the semantic producer — implicit regs exposed via `InsnDetail::regs_read/regs_write`, **including `rflags`** (`add`/`cmp` write it, `jcc` reads it, `lea [rip+…]` reads `rip`), plus prefix/modrm/sib encoding detail. **Caveat found**: implicit lists have gaps — `syscall` reports no register effects (real clobbers: `rcx`, `r11`, `rax`, all flags) → a small curated intrinsic table for syscall/trap-class ops is required. iced-x86's six-state per-bit rflags model (`rflags_read/written/modified/cleared/set/undefined`) correctly reports `syscall` as r=w=mod=511 — recommend iced as the **flags-metadata oracle** for Wave 6 x86 stages and as the differential test oracle for the capstone producer |
| `object` ELF source (docs.rs source view) | docs.rs `src/object/read/elf/file.rs.html` | **Architectural gap found**: dynamic symbols parse from `SHT_DYNSYM` sections; source carries an explicit TODO — "get dynamic symbols from DT_SYMTAB if there are no sections". Section-header-stripped ELF is unsupported by `object`; `goblin` exposes raw DT entries (`Dynamic.dyns`) but not segment-only dynsym either → **Wave 5B must own the PT_DYNAMIC → DT_SYMTAB/DT_STRTAB/DT_GNU_HASH walker** with vaddr→file-offset mapping via `PT_LOAD` |
| `rmcp` crate (official MCP Rust SDK) | crates.io + docs.rs | **V✓**: 3.8.0, updated 2026-10-26, repo `modelcontextprotocol/rust-sdk`; `Transport`, `IntoTransport`, `ServerHandler`, `RoleServer` surface — Wave 7 builds on rmcp rather than hand-rolling the protocol |
| SQLite WAL documentation | sqlite.org/wal.html | **V✓ line-cited**: "readers do not block writers and a writer does not block readers. Reading and writing can proceed concurrently" + "there can only be one writer at a time" — exactly the D-3 single-writer/multi-reader design |
| LLVM PointerAuth documentation | llvm.org/docs/PointerAuth.html | **V✓**: `llvm.ptrauth.sign/.auth/.strip/.resign` intrinsic grammar with key/discriminator args — Wave 6's `PtrAuth{op, signed, key, disc}` intrinsic class maps 1:1 to a verified source; also confirms arm64e + PAuth-ELF scope |
| Capstone `X86InsnDetail` / `InsnDetail` API pages | docs.rs | **V✓**: fields `operands/prefix/modrm/sib/rex/disp/opcode/addr_size`; implicit-reg access confirmed at API level (spike proved it at runtime) |
| Ghidra analysis help topic | raw.githubusercontent.com | 404 (help tree moved); function-start heuristic comparison stays V2 with ByteWeight as the grounded reference |

## 1. Competitor Product Surfaces

| Source | Use | Status |
| --- | --- | --- |
| Hopper product pages, tutorial, FAQ, SDK headers (in-repo research: `docs/hopper-gap/hopper-*.md`) | Competitive axis, UX bar, SDK boundary | **V✓** (in-repo, link-level evidence) |
| IDA Pro product page | Decompiler/debugger positioning | **V✓** (homepage title fetched 2026-10-28: "A Powerful Disassembler, Decompiler & Debugger") |
| Ghidra project (ghidra-sre.org → github.com/NationalSecurityAgency/ghidra) | Free-workbench baseline, SLEIGH/P-code | **V✓** (redirect target fetched) |
| Binary Ninja site | API-first/IL positioning | V2 (site reachable; feature claims not page-verified) |
| rizin/radare2 docs and Cutter | OSS baseline, command surface | V2 (docs site JS-rendered; fetch gave no content) |
| angr/VEX papers and docs | Programmatic ELF bar | V2 |
| BinExport/BinDiff ecosystem | Diff interop pressure | V3 (format stability must be checked before any export-compat claim) |

## 2. Formats, ISA, And ABI

| Source | Use | Status |
| --- | --- | --- |
| System V ABI (ELF, x86-64 psABI) | ELF structure, SysV convention, relocation classes | **V✓** (official tree via GitHub mirror, line-cited — §0 Round 2) |
| Arm A64 ISA reference and AAPCS64 | arm64 semantics, register roles | V2 |
| Mach-O reference (Apple) and `dyld` source | Waves 1–3 grounding | **V✓** (repo-internal) |
| GNU symbol versioning (`.gnu.version`/`.gnu.version_r`) | Versioned imports | **V✓ indirectly**: parser support verified in `object` and `goblin` (§0); spec text itself V2 (Oracle guide blocked) |
| Itanium C++ ABI (vtable, RTTI, mangling) | C++ recovery | **V✓** (official site via Wayback — §0 Round 2) |
| Intel/AMD x86-64 manuals | x86_64 semantics, CET/`endbr64` | V2 |
| LLVM docs (`llvm-objdump`, `llvm-readelf`, PointerAuth) | External comparison, PAC | **V✓** for the two command guides (§0 Round 2); PointerAuth doc V2 |
| Swift ABI mangling docs | Swift name facts (Wave 8) | **V✓** (swiftlang/swift `Mangling.rst` fetched) |
| Rust symbol mangling v0 RFC/docs | Rust names (Wave 5) | **V✓** (RFC 2603 fetched) |
| ELF `.eh_frame`/`.eh_frame_hdr` (LSB Core 5.0) | FDE function seeds | **V✓** (LSB chapter fetched; gimli API also verified — §0) |
| `.note.gnu.build-id` conventions | Artifact identity | V2 |
| GNU ld `-z` options (`ibtplt`, `ibt`, `shstk`, `relro`) | PLT/RELRO layout facts | **V✓** (ld docs via Wayback — §0 Round 2) |
| MCP specification (2026-07-28) | Transport and tool-shape contracts | **V✓** (§0 both rounds: transports + tools/structuredContent) |

### 1b. Technique Sources (Recon/Diff/Function-Discovery Grounding)

| Source | Use | Status |
| --- | --- | --- |
| ByteWeight, USENIX Security '14 (Bao et al.) | ML baseline for function-start discovery (Wave 4 heuristics context) | **V✓** (page verified) |
| Diaphora README | Diff heuristics vocabulary (graph/bytes/pseudo-code, similarity ratio) for Wave 7 scoring design | **V✓** |
| Obfuscator-LLVM wiki | The three canonical obfuscation passes recon detects: `-fla`, `-bcf`, `-sub` | **V✓** |
| UPX source | Packed-ELF pack-header magic (`UPX!`) for packer signatures | **V✓** |
| BinExport/BinDiff formats | Diff interop pressure | V3 (format stability must be checked before any export-compat claim) |

## 3. Libraries, Crates, Licenses

All rows below live-verified 2026-10-28 via docs.rs + crates.io API (§0).

| Crate | Verified facts | Status / decision use |
| --- | --- | --- |
| `capstone` 0.14.0 (pinned) + `capstone-sys` | MIT; `full` = non-Diet mode **not** all-archs; `arch_x86` exists (`→ capstone-sys/arch_x86`); our workspace currently enables only `arch_arm64` | **Correction applied across docs**: x86 decode needs the `arch_x86` feature added — one workspace line, still no new native dependency |
| `object` 0.40.0 (ours: 0.39) | MIT OR Apache-2.0; `read::elf::VersionTable`, `DynamicRelocationIterator`, `DynamicTable` | **V✓**; primary ELF parse backend for Wave 5B; minor version bump available |
| `goblin` 0.10.7 (ours: 0.10.5) | MIT/ISC; `VersymSection`/`VerneedSection`/`VerdefSection`/`Dynamic` | **V✓**; fallback backend confirmed capable |
| `gimli` 0.34.0 | MIT OR Apache-2.0; `read::EhFrame::{entries, fde_for_address, cie_from_offset}`, `EhHdrTable` | **V✓**; `.eh_frame` seeding plan verified at API level |
| `iced-x86` 1.21.0 | MIT; `Encoder`, `BlockEncoder`, `CodeAssembler` | **V✓**; Wave 8 x86 patch encoding verified — richer than planned (a real assembler API exists) |
| `rusqlite` 0.41.1 | MIT; `bundled` feature | **V✓**; D-3 store choice verified |
| `rmcp` 3.8.0 | official MCP Rust SDK (`modelcontextprotocol/rust-sdk`), active 2026-10 | **V✓**; Wave 7 MCP server foundation (§0 Round 3) |
| `cpp_demangle` 0.5.1 | MIT OR Apache-2.0; updated 2025-11 | **V✓**; default Itanium demangler candidate |
| `rustc-demangle` 0.1.28 | MIT OR Apache-2.0; updated 2026-10 (active) | **V✓**; Rust v0/legacy demangling |
| `swift-demangler` 0.2.0 (2026-10-02) | exists, recent, structured output | **V✓ existence**; Wave 8 Swift-names candidate (quality unverified — evaluate in a spike) |
| `itanium-demangler` 0.1.0 (2026-10-17) | exists, new | **V✓ existence**; evaluate alongside cpp_demangle |
| `macho-demangle` 0.5.6 | exists; Rust/C++/Swift, no subprocess | **V✓ existence**; Apple-lane candidate |
| `cxxfilt` | **does not exist** on crates.io/docs.rs | **Correction**: removed from consideration; use the verified set above |
| Keystone | `COPYING` fetched: GPL-2.0 | **V✓**; rejected on licensing for the closed-source product |
| LLVM-MC | Apache-2.0 w/ LLVM exceptions (license permissive; not fetched this pass) | V2; rejected as a C++ toolchain dependency, not licensing |
| `rayon` | MIT OR Apache-2.0 (well-known) | V2; parallel analysis |

## 4. Internal Specifications Consumed

All in-repo, V✓ (existence and cross-links checked in the doc-suite review
passes):

- `docs/hopper-gap/analysis-database-blueprint.md`
- `docs/hopper-gap/project-kernel-byte-map-implementation-spec.md`
- `docs/hopper-gap/function-recovery-and-procedure-database-plan.md`
- `docs/hopper-gap/decompiler-ir-strategy.md` + function-IR, quality, and
  pseudocode-workbench specs
- `docs/hopper-gap/automation-studio-scripting-plugin-workbench-implementation-spec.md`
- `docs/hopper-gap/apple-system-diffing-export-evidence-implementation-spec.md`
- `docs/hopper-gap/performance-responsiveness-architecture.md`,
  `large-project-performance-scheduler-indexing-implementation-spec.md`,
  `corpus-benchmark-regression-plan.md`
- `docs/hopper-gap/command-navigation-undo-workbench-implementation-spec.md`,
  `apple-runtime-workbench-browsers-implementation-spec.md`,
  `evidence-grounded-ai-assistant-workbench-implementation-spec.md`
- `docs/hopper-gap/patch-studio-assembler-workbench-implementation-spec.md`,
  `patching-hex-assembly-transaction-implementation-spec.md`
- `docs/hopper-gap/apple-sdk-semantic-packs-implementation-spec.md`,
  `swift-runtime-metadata-implementation-spec.md`,
  `type-signature-symbolication-implementation-spec.md`,
  `dwarf-dsym-kdk-debug-info-implementation-spec.md`,
  `processor-semantics-apple-abi-implementation-spec.md`
- `docs/roadmaps/apple-runtime-analysis-roadmap.md`,
  `docs/roadmaps/disassembler-v2-alpha-plan.md`
- `docs/adr/ADR-0001`, `ADR-0002`, `ADR-0003`

## 5. Refresh Procedure

1. Before any wave exit review: re-verify all V3 rows and any V2 row the
   wave depends on; prefer direct fetch (docs.rs/crates.io/GitHub raw at the
   pinned tag) over search.
2. Before any external/public positioning: re-verify competitor rows with
   live sources (enable the org web-search tier or fetch manually).
3. Record verification date, method, and URL per row when refreshed (§0
   format).
4. New external dependencies require a row here in the same PR that adopts
   them.
5. Pinned-version claims must be verified at the exact tag/version in use,
   not `latest` (the capstone `full`-feature correction in §0 is the
   cautionary example).
