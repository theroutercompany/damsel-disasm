# Processor Semantics And Apple ABI Implementation Spec

Status: implementation-ready planning document.

Purpose: define the missing processor, CPU-mode, calling-convention, and
Apple ABI semantics layer required before Damsel can credibly compete with
Hopper for macOS, iOS, dyld shared-cache, Objective-C, C++, Objective-C++,
Swift-adjacent, and XNU-style system binary workflows.

This document is not another decompiler plan. It is the foundation under the
decompiler, graph, patching, debugger, Apple runtime browsers, assistant, and
automation surfaces. Hopper's public feature surface and SDK make one thing
clear: a Hopper-class disassembler needs architecture semantics that survive
outside one rendered instruction row.

## Evidence Summary

Public Hopper evidence:

- Hopper's homepage presents it as a fast native macOS disassembler with
  control-flow graph, decompiler, debugger, Objective-C and Swift awareness,
  Python scripting, MCP support, and an extensible SDK.
- Hopper's FAQ says it supports Intel 32/64-bit, ARMv6, ARMv7, ARM64, and
  PowerPC processors, and that additional CPUs can be added through the SDK.
- Hopper's FAQ says it supports Mach-O for Mac and iOS, PE, and ELF.
- Hopper's tutorial says Hopper automatically assigns byte types after file
  load, lets the user change types, and backs those changes with undo/redo.
- Hopper's tutorial says procedures are prerequisites for assembly, CFG, and
  pseudocode representations because procedures carry basic-block and stack
  information.
- Hopper's tutorial exposes CPU-mode UX directly: for processors with multiple
  modes, such as ARM and Thumb, the inspector can let the user change the CPU
  mode at the current address.
- Hopper's tutorial makes switch/case hints a first-class analyst operation.
- Hopper's changelog repeatedly mentions AArch64, AArch64e, ARMv7, Intel,
  Swift demangling, Objective-C stubs, authenticated selector trampolines,
  conditional ARM64 instructions, PAC procedure creation, Apple Silicon
  decompilation, kernel cache support, dyld cache support, and large binary
  analysis performance.
- Hopper SDK headers expose `CPUDefinition`, `CPUContext`,
  `HPDisassembledFile`, `FileLoader`, `DisasmStruct`, `CommonTypes`, and
  calling-convention enums. Even the older public SDK headers are enough to
  show the intended architecture contract.

Apple and ABI evidence:

- Apple's ARM64 ABI guidance says Apple platforms reserve `x18`, require
  meaningful frame records through `x29`, require 16-byte stack alignment, and
  diverge from the generic ARM64 ABI in stack argument behavior.
- Apple's pointer-authentication guidance and Clang's arm64e documentation
  make PAC a real ABI issue, not a cosmetic instruction suffix.
- Clang documents C, C++, Objective-C, and arm64e pointer-authentication
  schemas, including function pointers, return addresses, vtables, Objective-C
  runtime pointers, Objective-C methods, class data, `isa`, `super`, and
  selector pointers.
- XNU's public Mach-O headers define CPU type/subtype fields, `MH_FILESET`,
  `LC_DYLD_CHAINED_FIXUPS`, `LC_FILESET_ENTRY`, `MH_DYLIB_IN_CACHE`, fat
  archives, and fileset-style kernel collection structure.
- Swift ABI and mangling documentation show that Swift symbols and metadata
  are not merely strings; symbolic references and calling-convention facts are
  binary-analysis concerns.

Damsel repo evidence:

- `damsel-core::Architecture` currently names `Arm64`, `Arm64e`, and `X86_64`.
- `damsel-core::Operand` already preserves register, immediate, memory,
  shifted-register, qualified-register, multi-register, system-register,
  condition, and name operands.
- `damsel-core::DecodedInstruction` currently holds address, size, opcode,
  mnemonic, operands, recovered values, references, and annotations.
- `damsel-core::DisassemblyAnalysis` already contains basic blocks, edges,
  direct calls, branch targets, data references, indirect controls, imports,
  ObjC references, cache links, recovered values, jump tables, and summary
  information.
- `damsel-macho::disasm` currently builds CFG and target-use facts from decoded
  instruction rows and known image metadata.
- `Cargo.toml` currently uses `capstone` with `arch_arm64`, which is a useful
  decoder layer but not a Hopper-class architecture-semantic layer.

## Thesis

Damsel should treat processor support as four related layers:

1. Decode: turn bytes into architecture-specific instruction rows.
2. Semantics: turn rows into stable reads, writes, control transfers, stack
   effects, target expressions, CPU-mode facts, and ABI hints.
3. Runtime overlay: interpret semantic facts through ObjC, C++, Swift, dyld,
   libdispatch, IOKit, kernel, and PAC/chained-fixup context.
4. Workbench product surface: expose editable facts, CPU-mode overrides,
   switch hints, ABI problems, graph explanations, pseudocode source maps,
   patch plans, debugger mappings, scripts, MCP tools, and assistant evidence.

Today, Damsel has the start of layers 1 and 2 for arm64/arm64e standalone
Mach-O and shared-cache workflows. Hopper closeness requires making layer 2 a
durable subsystem rather than incidental logic inside `disasm.rs`.

The key architectural shift:

- stop asking only "what text should this instruction render as?"
- start asking "what facts does this instruction prove, assume, or leave
  unresolved for this architecture, ABI, binary container, and runtime?"

## Non-Goals

This spec does not require:

- replacing Capstone immediately
- building a full custom decoder before project persistence exists
- claiming source-level recovery from ABI hints
- executing binaries or bypassing platform protections
- supporting every Hopper processor family before the Apple-focused lane is
  strong
- adding unsafe mutation to the main opened source file
- making x86_64 parity block arm64e/XNU progress

This spec does require:

- a typed semantic layer that can sit above Capstone or future decoders
- Apple ABI profiles that are explicit and testable
- confidence/provenance on every nontrivial inferred fact
- Workbench/API contracts that make CPU-mode and ABI uncertainty visible
- compatibility with future decompiler, debugger, patching, automation, and
  assistant work

## Product Target

The target user experience:

- Open a Mach-O, universal binary, dyld shared-cache image, DriverKit cache
  member, kernel collection member, app binary, dSYM/KDK-symbolicated image,
  or extracted IPSW/OTA artifact.
- Damsel selects an architecture profile from container metadata and exposes
  it in the Workbench.
- The user can see whether the file is `arm64`, `arm64e`, `x86_64`,
  `x86_64h`, legacy `armv7`, legacy `armv7s`, or another future slice.
- The user can inspect ABI profile, pointer-auth profile, CPU modes, register
  roles, calling-convention assumptions, frame-policy assumptions, and
  unsupported features.
- The assembly view shows not only mnemonic text but also semantic badges:
  call, tailcall, thunk, branch, conditional branch, switch candidate,
  PAC-authenticated return, authenticated indirect call, ObjC message send,
  C++ virtual call, Swift thunk, dyld stub, kernel trap, IOKit dispatch, or
  unresolved control.
- The graph view can explain why edges exist, which CPU-mode decision created
  them, whether they came from a direct target, recovered PC-relative value,
  jump table, switch hint, loader import, ObjC metadata, or user override.
- The inspector can show register reads/writes, stack delta, frame state,
  argument registers, return registers, clobbers, condition flags, PAC facts,
  and unresolved semantic problems.
- The decompiler can consume architecture-neutral low-level facts rather than
  reparsing rendered strings.
- The patch studio can ask the semantics layer whether an assembler/NOP edit
  preserves instruction alignment, CPU mode, branch reachability, and PAC
  policy.
- The debugger can map static register roles and ABI facts to live frame,
  argument, return, and ObjC/C++/Swift runtime state.
- Automation, MCP, and assistant tools can cite semantic facts by stable ids.

## Core Architecture

Add a `damsel-semantics` crate or a `damsel-core::semantics` module family.
The exact crate boundary can wait until implementation, but the model boundary
should be clear.

Recommended modules:

- `architecture_profile`
- `cpu_mode`
- `register_profile`
- `instruction_semantics`
- `control_semantics`
- `stack_semantics`
- `abi_profile`
- `calling_convention`
- `apple_arm64`
- `apple_arm64e`
- `apple_x86_64`
- `legacy_arm32`
- `objc_message_semantics`
- `cpp_abi_semantics`
- `swift_abi_semantics`
- `xnu_semantics`
- `semantic_problems`
- `semantic_overrides`
- `semantic_exports`

Do not hide this behind UI-only models. It must become a core fact layer that
CLI, API, native Workbench, tests, scripts, MCP, assistant, debugger, and
exporters all consume.

## Data Model

### ArchitectureProfile

`ArchitectureProfile` describes the processor family and binary-container
slice that all later semantic facts depend on.

Fields:

- `id`: stable profile id, such as `apple-arm64`, `apple-arm64e`,
  `apple-x86_64`, `apple-x86_64h`, `apple-armv7`, `apple-armv7s`
- `display_name`
- `cpu_family`: normalized family
- `cpu_subfamily`: normalized subtype
- `pointer_width_bits`
- `integer_width_bits`
- `endianness`
- `instruction_alignment`
- `minimum_instruction_size`
- `maximum_instruction_size`
- `default_syntax`
- `supported_syntaxes`
- `cpu_modes`
- `default_cpu_mode`
- `abi_profiles`
- `loader_source`: Mach-O header, fat slice, dyld cache header, fileset entry,
  user override, or plugin
- `confidence`
- `provenance`
- `problems`

Why:

- Hopper's `CPUDefinition` exposes CPU families, subfamilies, address width,
  integer width, endianess, syntaxes, CPU mode names, register classes, stack
  pointer detection, frame pointer detection, program counter detection, NOPs,
  assembler support, and decompiler support.
- Damsel currently has `Architecture`, but not a complete profile.

### CpuModeProfile

`CpuModeProfile` describes instruction-set modes within a processor family.

Fields:

- `mode_id`
- `display_name`
- `instruction_alignment`
- `address_low_bit_policy`
- `mode_switch_sources`
- `valid_range_overrides`
- `default_from_architecture`
- `user_editable`
- `problems`

Initial modes:

- `aarch64`
- `arm`
- `thumb`
- `x86_64`

Why:

- Hopper's tutorial exposes ARM/Thumb mode editing in the inspector.
- Hopper's `CPUContext` exposes `cpuModeFromAddress`,
  `addressForcesACPUMode`, `estimateCPUModeAtVirtualAddress`,
  `instructionConditionsCPUModeAtTargetAddress`, and
  `cpuModeForNextInstruction`.
- Damsel's current Apple lane is mostly arm64/arm64e, but legacy iOS and
  future multi-arch imports need this model before the UI grows around it.

### RegisterProfile

`RegisterProfile` maps register names to roles.

Fields:

- `register_id`
- `aliases`
- `width_bits`
- `class`: general, SIMD, floating, system, condition, pseudo, stack, argument,
  temporary, PAC-state, platform, thread, dyld-scratch, kernel-special
- `role`: stack pointer, frame pointer, program counter, link register,
  return-value, argument, callee-saved, caller-saved, reserved, scratch,
  platform-reserved, condition-flags
- `abi_visibility`
- `readable_in_debugger`
- `writeable_in_debugger`
- `semantic_notes`

Initial Apple arm64 profile:

- `x0` through `x7`: integer argument and return-value lane
- `x8`: indirect-result lane
- `x16`, `x17`: interprocedural scratch registers often used by dyld stubs
- `x18`: Apple-reserved platform register
- `x19` through `x28`: callee-saved lane
- `x29`: frame pointer
- `x30`: link register
- `sp`: stack pointer
- `pc`: program counter view
- `v0` through `v7`: FP/SIMD argument and return lane
- `v8` through `v15`: partially preserved FP/SIMD lane
- condition flags: NZCV facts

Initial x86_64 profile:

- argument registers by ABI profile
- return registers
- stack pointer
- frame/base pointer
- instruction pointer
- flags register
- SIMD registers
- Darwin-specific Swift async exception where applicable

Why:

- Damsel currently stores registers as strings inside `Operand`.
- Hopper's SDK asks CPU plugins to identify stack pointer, frame pointer,
  program counter, special registers, register classes, and names.
- The Workbench cannot answer "what is this function argument?" or "why is
  this a tailcall?" without register roles.

### InstructionSemantics

`InstructionSemantics` attaches stable facts to each decoded instruction.

Fields:

- `instruction_id`
- `address`
- `size`
- `cpu_mode`
- `decoded_source`
- `reads`
- `writes`
- `implicit_reads`
- `implicit_writes`
- `flag_reads`
- `flag_writes`
- `stack_effect`
- `frame_effect`
- `control_effect`
- `memory_effects`
- `constant_effects`
- `pc_relative_values`
- `auth_effects`
- `call_effect`
- `return_effect`
- `objc_effect`
- `cpp_effect`
- `swift_effect`
- `dyld_effect`
- `kernel_effect`
- `confidence`
- `provenance`
- `problems`

This is not a decompiler IR node. It is a local fact record that can feed IR,
CFG, debugger, patching, graph, search, summaries, and exports.

### StackEffect

Fields:

- `sp_delta`
- `sp_delta_confidence`
- `frame_pointer_established`
- `frame_pointer_destroyed`
- `saved_registers`
- `restored_registers`
- `local_stack_range`
- `red_zone_use`
- `stack_argument_reads`
- `stack_argument_writes`
- `stack_alignment_observed`
- `stack_alignment_required`
- `problems`

Initial Apple arm64 rules:

- record 16-byte stack-alignment expectations
- record use of 128-byte red zone in user-mode profile
- treat `x29` frame records as meaningful where the Apple ABI says they must
  be meaningful
- distinguish leaf, tailcall, and frame-omitted cases

Initial kernel profile difference:

- do not blindly apply user-mode red-zone assumptions to XNU/kernel
  collection contexts
- mark profile differences explicitly

### ControlEffect

Fields:

- `kind`: fallthrough, direct branch, conditional branch, direct call,
  indirect call, indirect branch, return, halt, trap, exception, tailcall
- `condition`
- `direct_target`
- `target_expression`
- `target_register`
- `fallthrough`
- `cpu_mode_target`
- `call_convention_hint`
- `return_register`
- `switch_candidate`
- `jump_table_candidate`
- `thunk_candidate`
- `tailcall_candidate`
- `semantic_reason`
- `confidence`
- `problems`

Why:

- Hopper's `CPUContext` exposes branch analysis, switch detection, called
  addresses, callsites, thunk destination recovery, and halt detection.
- Damsel's current CFG builder sees `Reference` rows but needs richer target
  expression provenance.

### MemoryEffect

Fields:

- `kind`: load, store, address-only, prefetch, atomic, barrier, unknown
- `address_expression`
- `base_register`
- `index_register`
- `scale`
- `displacement`
- `width_bits`
- `sign_extension`
- `zero_extension`
- `alignment`
- `target_address`
- `target_section`
- `target_symbol`
- `target_runtime_object`
- `provenance`

Why:

- Hopper's `CPUContext` distinguishes instructions that directly reference
  memory, instructions that only load an address, and instructions that should
  not produce false memory references.
- Damsel needs this for PC-relative ObjC selector recovery, dyld stub recovery,
  vtable recovery, IOKit method table recovery, and chained-fixup context.

### AbiProfile

`AbiProfile` describes how a semantic instruction stream maps to language and
platform behavior.

Fields:

- `id`
- `architecture_profile_id`
- `platform`: macOS, iOS, watchOS, tvOS, DriverKit, XNU, unknown
- `environment`: user, kernel, simulator, dyld-cache, driver-cache,
  kernel-collection
- `calling_conventions`
- `default_calling_convention`
- `objc_calling_convention`
- `swift_calling_convention`
- `cpp_abi`
- `pointer_auth_profile`
- `chained_fixup_profile`
- `red_zone_policy`
- `frame_pointer_policy`
- `stack_alignment`
- `reserved_registers`
- `dyld_scratch_registers`
- `provenance`
- `problems`

Initial profiles:

- `apple-arm64-user`
- `apple-arm64e-user`
- `apple-arm64e-dyld-cache`
- `apple-arm64e-xnu`
- `apple-x86_64-user`
- `apple-x86_64-dyld-cache`
- `apple-armv7-ios-legacy`
- `apple-thumbv7-ios-legacy`

### CallingConventionProfile

Fields:

- `id`
- `language`: C, Objective-C, Objective-C++, C++, Swift, block, syscall,
  trap, IOKit, unknown
- `argument_locations`
- `return_locations`
- `indirect_result_location`
- `self_location`
- `selector_location`
- `error_result_location`
- `async_context_location`
- `varargs_policy`
- `callee_saved_registers`
- `caller_saved_registers`
- `stack_argument_policy`
- `this_pointer_policy`
- `vtable_pointer_policy`
- `block_invoke_policy`
- `tailcall_policy`
- `provenance`

Initial facts:

- Objective-C message sends on arm64 put `self` and `_cmd` in the first two
  integer argument lanes.
- Swift on arm64 needs explicit support for Swift-specific roles such as
  `self`, error return, and async context when evidence is available.
- C++ on Apple targets needs `this` pointer and vtable call recognition.
- XNU and IOKit workflows need C++ ABI support because IOKit is heavily C++.

### PointerAuthProfile

Fields:

- `id`
- `enabled`
- `target_architecture`
- `known_keys`
- `return_address_policy`
- `function_pointer_policy`
- `vtable_policy`
- `objc_method_policy`
- `objc_selector_policy`
- `objc_class_policy`
- `isa_policy`
- `super_policy`
- `block_policy`
- `dyld_stub_policy`
- `kernel_policy`
- `strip_policy`
- `auth_failure_display_policy`
- `problems`

Initial behavior:

- recognize PAC and AUT instructions as semantic events
- preserve signed-vs-stripped pointer provenance when chained fixups or
  metadata indicate pointer authentication
- mark targets as authenticated, unauthenticated, stripped, resigned,
  unresolved, or impossible to prove
- show when an indirect call target is likely PAC-protected
- avoid pretending that a signed pointer equals a raw address without
  provenance

Why:

- Hopper's changelog explicitly mentions AArch64 authenticated Objective-C
  selector trampolines and PAC procedure creation.
- Clang's arm64e docs define language-level PAC behavior for C, C++,
  Objective-C, and runtime pointers.
- XNU/system analysis will frequently encounter arm64e system binaries where
  PAC is central to control-flow interpretation.

### SemanticProblem

Fields:

- `id`
- `severity`: note, warning, error, unsupported
- `category`: decode, cpu-mode, abi, frame, branch, switch, PAC, ObjC, C++,
  Swift, dyld, kernel, loader, debug-info, user-override
- `route`
- `message`
- `evidence`
- `suggested_actions`
- `suppression_policy`

Problem examples:

- `arm64e-pac-target-not-stripped`
- `x18-used-as-general-register-on-apple-arm64`
- `objc-msgsend-selector-unresolved`
- `objc-msgsend-self-unresolved`
- `swift-symbolic-reference-not-decoded`
- `jump-table-targets-need-user-hint`
- `thumb-mode-derived-from-low-bit`
- `kernel-red-zone-assumption-disabled`
- `x86_64h-subtype-treated-as-generic-x86_64`
- `fileset-member-profile-missing`
- `capstone-decoded-but-semantic-lifter-missing`

## Pipeline

### Stage 1: Source And Slice Classification

Inputs:

- standalone Mach-O
- fat/universal Mach-O
- app bundle executable
- dyld shared cache root or projected image
- DriverKit cache image
- kernel collection member
- KDK/dSYM sidecar
- panic log route
- extracted IPSW/OTA artifact

Outputs:

- source id
- slice id
- Mach-O header facts
- CPU type/subtype facts
- fileset entry facts
- dyld-cache container facts
- selected `ArchitectureProfile`
- selected `AbiProfile`
- unresolved classification problems

Implementation notes:

- Reuse current `damsel-macho::loader` and `SharedCacheSession`.
- Add an explicit `ArchitectureProfileSelection` result rather than returning
  only `Architecture`.
- For fat/universal files, keep every candidate profile, even if the current
  command opens only one slice.
- For kernel collections, derive profile from the fileset member and outer
  container rather than only the inner Mach-O header.
- For dyld shared-cache images, record whether the file came from cache,
  projected image, sidecar, or standalone extraction.
- For arm64e, preserve subtype and pointer-auth expectations from the start.

Acceptance:

- fixture tests for arm64, arm64e, x86_64, x86_64h, fat arm64+x86_64, and
  MH_FILESET kernel-style members
- malformed CPU subtype test
- unknown subtype degrades with explicit problem
- CLI JSON includes profile id and source provenance

### Stage 2: Decode

Inputs:

- byte provider
- selected architecture profile
- selected CPU mode
- requested range
- syntax profile

Outputs:

- `DecodedInstruction`
- decode problems
- raw decoder metadata

Implementation notes:

- Keep Capstone for initial arm64/arm64e decoding.
- Add a decoder adapter trait so future x86_64, ARMv7, Thumb, or custom
  decoder work does not leak into the semantic model.
- Never parse rendered instruction strings to recover semantic fields when the
  decoder or model can provide structured data.
- Preserve instruction bytes in a routeable byte window or byte-span fact.
- Keep decoder provenance so problems can say "decoded by Capstone arm64" or
  "decoded by fallback raw bytes".

Acceptance:

- current disassembly snapshots remain stable unless intentionally expanded
- decoded byte spans round-trip to byte provider ranges
- unknown decode does not poison following semantic facts

### Stage 3: Local Semantic Lifting

Inputs:

- decoded instruction
- architecture profile
- CPU mode profile
- register profile
- byte provider
- image metadata

Outputs:

- `InstructionSemantics`
- local value facts
- local control facts
- local memory facts
- local problems

Implementation notes:

- Start with a table-driven arm64 lifter for the instruction families that
  already matter in Damsel:
  - branches and calls
  - conditional branches
  - returns
  - ADR/ADRP
  - LDR literal
  - ADD/SUB immediate
  - MOV/MOVK/MOVZ
  - PAC/AUT/RETAB-style authentication families where available
  - load/store pair frame prolog/epilog patterns
  - compare/test plus conditional branch patterns
  - TBZ/TBNZ
  - BR/BLR
  - dyld stub-friendly indirect branches
- Expand from fixtures and public Apple-binary corpus, not from abstract ISA
  completeness.
- Represent confidence. A recovered `ADRP+ADD` page/address pair should not
  have the same confidence as a direct branch immediate.

Acceptance:

- semantic snapshot for each instruction family
- value-recovery snapshot for `ADRP+ADD`, `ADRP+LDR`, direct branch, indirect
  branch, and ObjC selector load patterns
- semantic problem snapshot for unknown and unsupported instructions

### Stage 4: Function And Frame Semantics

Inputs:

- local instruction semantics
- known symbols
- entry points
- metadata seeds
- user procedure overrides
- existing `DisassemblyAnalysis`

Outputs:

- procedure semantic summary
- stack frame facts
- argument/return hypotheses
- saved/restored register facts
- callsite facts
- tailcall/thunk facts
- no-return facts
- switch candidates
- problems

Implementation notes:

- Move procedure and CFG inference toward a stable procedure database, but do
  not block initial implementation on full persistence.
- Use current `BasicBlock`, `ControlFlowEdge`, `DisassemblyIndirectUse`,
  `DisassemblyJumpTableUse`, and `DisassemblyRecoveredValueUse` as migration
  anchors.
- Add a `SemanticFunctionSummary` that can exist beside current summaries.
- Record user hints as transactions, not as text edits.

Acceptance:

- direct calls and branches match current output
- indirect calls include target expression and register provenance
- returns include return register when non-default
- frame setup and teardown are visible in JSON
- no-return and tailcall uncertainty is explicit

### Stage 5: ABI Inference

Inputs:

- function semantic summaries
- callsite facts
- symbol names
- ObjC metadata
- C++ demangling and RTTI
- Swift demangling and Swift symbol facts
- dyld import/stub facts
- debug-info profiles

Outputs:

- function signature hypotheses
- calling-convention selection
- argument locations
- return locations
- receiver/selector facts
- `this` pointer facts
- Swift role facts
- syscall/trap facts
- confidence/provenance

Implementation notes:

- Keep ABI hypotheses distinct from user-confirmed function signatures.
- A call to `_objc_msgSend` should produce an ObjC message-send semantic fact,
  but selector and receiver confidence should depend on recovered values.
- A C++ vtable dispatch should produce a vtable-call candidate, not a confirmed
  method call, until RTTI/vtable facts support it.
- Swift demangled names can seed signatures, but Swift symbolic references and
  metadata facts need stronger provenance.
- XNU syscall/trap/Iokit facts need dedicated profiles because they are not
  generic C calls.

Acceptance:

- ObjC message-send fixture with self/selector locations
- C++ member-call fixture with `this` location
- Swift symbol fixture with demangle-derived signature seed
- dyld stub fixture with import-backed callsite
- false-positive fixture where no runtime fact is emitted

### Stage 6: Runtime Overlay

Inputs:

- ABI facts
- ObjC metadata
- C++/RTTI/vtable facts
- Swift demangling/metadata facts
- dyld shared-cache linkage
- kernel collection facts
- KDK/dSYM debug info
- panic-log route facts

Outputs:

- runtime object links
- Workbench browser routes
- graph overlays
- semantic search indexes
- assistant evidence cards
- export facts

Implementation notes:

- Runtime overlay should be a consumer of semantic facts, not a replacement
  for them.
- ObjC selector browsers should cite the semantic callsites that reference
  selectors.
- C++/IOKit browsers should cite vtable and function-summary evidence.
- Kernel browsers should cite fileset, KDK, dSYM, panic, and semantic call
  evidence separately.
- Assistant responses must cite route/fact ids rather than narrative guesses.

Acceptance:

- Workbench runtime cards link to exact semantic facts
- CLI JSON can export semantic route ids
- evidence package includes semantic version and profile id

## Workbench Surfaces

### Architecture Inspector

Display:

- architecture profile
- ABI profile
- CPU mode
- pointer width
- integer width
- endianess
- syntax mode
- loader source
- pointer-auth state
- supported semantic lifter version
- unsupported feature list

Actions:

- change syntax profile
- add CPU-mode override where legal
- add analysis profile note
- copy profile facts as JSON
- open profile documentation

Guardrails:

- changing CPU mode creates a transaction and reanalysis job
- unsupported CPU mode is explicit
- profile override is separate from loader facts

### Instruction Semantics Inspector

Display:

- reads
- writes
- implicit reads/writes
- stack delta
- frame state
- condition facts
- control target expression
- memory effect
- recovered value expression
- PAC effect
- ABI interpretation
- runtime overlay
- confidence and provenance

Actions:

- copy semantic fact id
- add or remove switch hint
- mark tailcall candidate
- mark no-return target
- set operand format
- add comment with fact citation
- open all callers/callees using this semantic role

Guardrails:

- analyst edits are transactions
- edits never silently rewrite original bytes
- low-confidence facts remain visibly low confidence

### ABI Callsite Card

Display:

- call kind
- callee
- calling convention
- argument locations
- receiver and selector if ObjC
- `this` pointer if C++
- Swift roles if Swift
- return location
- unresolved arguments
- PAC/dyld/trampoline facts

Actions:

- open callee
- open receiver class
- open selector
- open vtable
- open Swift symbol
- add signature override
- compare with debug-info signature

### CPU Mode Timeline

Display:

- ranges of inferred CPU mode
- mode-changing branches
- user overrides
- decode failures by mode
- mixed-mode warnings

Initial use:

- mostly legacy ARM/Thumb.
- still valuable for Hopper parity because Hopper exposes CPU mode as a user
  operation, and Damsel should not hard-code "Apple means AArch64 only".

### PAC And Chained-Fixup Panel

Display:

- authenticated code pointers
- authenticated data pointers
- stripped targets
- unresolved signed pointers
- chained fixup source
- dyld import source
- ObjC/class/selector/vtable relation
- auth instruction sites

Actions:

- show raw pointer
- show stripped address if known
- show signing schema if known
- cite fixup or instruction evidence
- export PAC problem report

Guardrails:

- never present a guessed stripped pointer as verified
- separate `raw`, `stripped`, `authenticated`, and `resolved` states

## CLI And API Contracts

### CLI Commands

Add:

```text
damsel semantics profile <path>
damsel semantics instruction <path> --address <va>
damsel semantics function <path> --target <symbol|address>
damsel semantics callsites <path> --target <symbol|address|selector>
damsel semantics problems <path>
damsel semantics export <path> --format jsonl
```

Existing `disasm` output can remain, but new semantic commands should avoid
breaking existing snapshots.

### API Routes

Add:

```text
GET /api/projects/{project_id}/architecture-profile
GET /api/projects/{project_id}/abi-profile
GET /api/projects/{project_id}/semantic/instructions
GET /api/projects/{project_id}/semantic/instructions/{instruction_id}
GET /api/projects/{project_id}/semantic/functions/{function_id}
GET /api/projects/{project_id}/semantic/callsites
GET /api/projects/{project_id}/semantic/problems
POST /api/projects/{project_id}/semantic/overrides/cpu-mode
POST /api/projects/{project_id}/semantic/overrides/switch-hint
POST /api/projects/{project_id}/semantic/overrides/calling-convention
```

### JSON Shape

Example:

```json
{
  "instruction_id": "insn:main:0x100003f80",
  "address": "0x100003f80",
  "architecture_profile": "apple-arm64e-user",
  "cpu_mode": "aarch64",
  "mnemonic": "blr",
  "control_effect": {
    "kind": "indirect-call",
    "target_register": "x16",
    "target_expression": "reg:x16",
    "call_convention_hint": "apple-arm64-c",
    "confidence": "medium"
  },
  "reads": ["x16"],
  "writes": ["x30"],
  "abi_effect": {
    "argument_locations": ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"],
    "return_locations": ["x0", "x1"]
  },
  "provenance": [
    {
      "kind": "decoded-instruction",
      "route": "disasm:0x100003f80"
    }
  ],
  "problems": []
}
```

## Integration With Current Code

### `damsel-core::Architecture`

Keep the enum but stop asking it to carry every profile fact.

Add:

- `ArchitectureProfileId`
- `ArchitectureProfile`
- `AbiProfileId`
- `AbiProfile`
- `CpuModeId`
- `SemanticVersion`

Migration:

- `Architecture::Arm64` maps to profile candidates, not one final behavior.
- `Architecture::Arm64e` maps to pointer-auth-capable profiles.
- `Architecture::X86_64` maps to Darwin x86_64 user/dyld profiles and later
  Linux/ELF profiles if needed.

### `damsel-core::Operand`

Keep as decoded operand representation.

Add:

- `OperandSemanticRole`
- `RegisterId`
- `RegisterRole`
- `MemoryExpression`
- `ValueExpression`

Do not mutate `Operand` into an all-purpose semantic object. It is useful as
decoded structure. The semantic layer can point at operands by index.

### `DecodedInstruction`

Keep as row-level decoded output.

Add either:

- `semantic_id: Option<InstructionSemanticId>` in a future response model

or:

- separate map from instruction route to `InstructionSemantics`

Prefer the second for compatibility. Current CLI/UI consumers should not need
the semantic payload until they opt in.

### `DisassemblyAnalysis`

Keep the current structure and add semantic companions:

- `SemanticFunctionSummary`
- `SemanticCallsiteSummary`
- `SemanticFrameSummary`
- `SemanticProblemSummary`
- `SemanticProfileSummary`

Migration:

- current `direct_calls`, `branch_targets`, `indirect_controls`,
  `recovered_values`, `jump_tables`, `objc_references`, and `cache_links` can
  become projection summaries generated from richer semantic facts.
- in early phases, produce both from the same analysis pass.

### `damsel-macho::disasm`

Split into:

- decode adapter
- semantic lifter
- CFG/procedure analyzer
- runtime overlay linker
- summary projector

Current `analyze_disassembly_impl` can stay while the semantic lifter grows,
but new code should avoid adding more architecture-specific special cases into
the same loop.

### Workbench Store

The Swift Workbench should not own semantics. It should cache and present
route-backed semantic pages.

Add future stores:

- `ArchitectureProfileStore`
- `SemanticInstructionStore`
- `SemanticFunctionStore`
- `SemanticProblemStore`
- `SemanticOverrideStore`

The current `WorkbenchStore` is already under pressure from navigation,
copy/filter, upload, server, and selected-instruction state. Semantic stores
should be split rather than bolted onto the main store.

## Implementation Slices

### Slice 1: Profile Model

Deliver:

- profile ids
- architecture profile model
- ABI profile model
- register profile model
- JSON snapshots
- CLI profile command

Acceptance:

- arm64, arm64e, x86_64 profile snapshots
- unknown architecture problem snapshot
- Workbench can show profile as read-only text/card data

### Slice 2: Arm64 Register Roles

Deliver:

- register id normalization
- arm64 register role table
- x18 reserved warning
- x29 frame role
- x30 link role
- x16/x17 dyld scratch role
- argument/return lane roles

Acceptance:

- fixtures for common prolog/epilog
- fixtures for BL/BLR/RET
- fixture warning when x18 is treated as general-purpose in Apple profile

### Slice 3: Local Arm64 Control Semantics

Deliver:

- branch/call/return control effects
- condition facts
- indirect call/branch target register facts
- fallthrough facts
- semantic projection into current CFG summary

Acceptance:

- existing disasm tests still pass
- new semantic snapshots for direct branch, conditional branch, BL, BLR, BR,
  RET, TBZ, TBNZ

### Slice 4: PC-Relative Value Semantics

Deliver:

- ADR
- ADRP
- ADD immediate pair recovery
- LDR literal
- page target provenance
- section/symbol/cache target relation

Acceptance:

- recovered value facts match existing `RecoveredValue` where available
- false pairings are explicitly low confidence
- ObjC selector and dyld stub fixtures cite value provenance

### Slice 5: Frame And Stack Semantics

Deliver:

- stack delta tracking
- frame setup/destruction detection
- saved/restored register facts
- red-zone profile fact
- stack alignment problem facts

Acceptance:

- leaf function fixture
- framed function fixture
- tailcall fixture
- malformed stack fixture
- kernel profile disables user red-zone assumptions

### Slice 6: ObjC Message Semantics

Deliver:

- `_objc_msgSend` and known runtime call recognition
- receiver/selector argument facts
- selector load provenance
- method-owner link if metadata supports it
- unresolved receiver/selector problems

Acceptance:

- standalone ObjC fixture
- dyld-cache projected image fixture
- selector trampoline fixture
- authenticated selector trampoline fixture marked PAC-aware

### Slice 7: arm64e/PAC Semantics

Deliver:

- PAC/AUT instruction event facts
- authenticated return markers
- function pointer signed/stripped/resolved states
- ObjC runtime pointer PAC states
- chained-fixup provenance link
- PAC problem panel data

Acceptance:

- arm64e fixture snapshots
- signed pointer not presented as verified raw target
- current arm64 tests unaffected

### Slice 8: x86_64 Darwin Semantic Baseline

Deliver:

- profile model
- register roles
- direct control semantics
- stack/frame baseline
- ObjC message ABI support
- Swift async role marker where evidence exists

Acceptance:

- x86_64 standalone fixture
- x86_64 unsupported decode degrades clearly until decoder exists
- profile command works even when disassembly is unavailable

### Slice 9: Legacy ARM/Thumb Baseline

Deliver:

- ARMv7 and Thumb profile ids
- CPU-mode model
- low-bit mode switch semantics
- profile-only support before full decode if necessary

Acceptance:

- profile snapshots for legacy iOS slices
- CPU-mode override transaction model
- no crash on unsupported legacy decode

### Slice 10: XNU And Kernel ABI Profile

Deliver:

- XNU/kernel profile ids
- fileset member profile selection
- kernel red-zone policy
- syscall/trap callsite kinds
- IOKit C++ dispatch hints
- KDK/dSYM signature provenance hook

Acceptance:

- synthetic MH_FILESET fixture
- panic route fixture links frame/profile facts
- kernel profile facts export without user-mode assumptions

## Testing Strategy

Unit tests:

- profile model serialization
- register role lookup
- register alias normalization
- CPU-mode selection
- stack effect rules
- control effect rules
- ABI callsite mapping
- PAC state transitions

Snapshot tests:

- profile JSON
- instruction semantics JSON
- function summary JSON
- semantic problem JSON
- CLI output
- Workbench API payloads

Fixture families:

- tiny arm64 direct call
- tiny arm64 conditional branch
- tiny arm64 indirect call
- tiny arm64 frame
- tiny arm64 leaf
- tiny ObjC selector call
- tiny C++ vtable call
- tiny Swift demangled symbol
- arm64e authenticated return
- arm64e authenticated selector/trampoline
- dyld stub
- dyld shared-cache projected image
- MH_FILESET kernel-style member
- malformed CPU subtype
- unknown instruction bytes

Corpus tests:

- existing synthetic Mach-O fixtures
- existing shared-cache tests
- real-cache opt-in evidence package
- KDK/dSYM opt-in profile tests
- large-binary performance benchmarks

Negative tests:

- signed pointer treated as raw address
- x18 silently used as general register
- user-mode red-zone rule applied to kernel profile
- ObjC selector guessed without value evidence
- C++ vtable guessed without table evidence
- Swift symbolic reference parsed from unsafe uncontrolled input
- CPU-mode override silently mutates original profile

Performance gates:

- semantics page query under target latency
- profile selection under target latency
- semantic lifter streaming without full-file blocking
- large function partial result limit
- Workbench scroll not blocked by semantic enrichment

## Migration Plan

Phase A:

- Add profile models and read-only CLI profile output.
- No UI mutation.
- No decompiler dependency.

Phase B:

- Add arm64 local semantics for control/value/frame basics.
- Generate current summaries from semantic facts.
- Keep existing output stable.

Phase C:

- Add ObjC and dyld runtime overlay consumption.
- Link semantic facts to existing ObjC/cache summaries.

Phase D:

- Add arm64e/PAC and kernel ABI profiles.
- Surface PAC and kernel assumptions in Workbench/API.

Phase E:

- Add Workbench semantic inspectors and CPU-mode override transactions.

Phase F:

- Feed semantic facts into decompiler IR, patch studio, debugger, automation,
  MCP, assistant, diff/export, and evidence packages.

## Risks

Risk: building a second disassembler.

Mitigation:

- keep decode adapter separate from semantic lifter
- start with selected instruction families
- preserve Capstone where it is good enough

Risk: semantic facts become overconfident.

Mitigation:

- every inferred fact has confidence and provenance
- low-confidence runtime overlays are visually distinct
- negative fixtures enforce refusal

Risk: Workbench becomes sluggish.

Mitigation:

- semantic pages are paged and cancellable
- expensive semantic passes run as jobs
- initial row rendering does not wait on whole-project semantic enrichment

Risk: ABI profile mistakes poison ObjC/C++/Swift reasoning.

Mitigation:

- profile ids are visible
- profile override is explicit
- debug-info/KDK facts can override guesses with provenance

Risk: kernel and user-mode assumptions get mixed.

Mitigation:

- XNU/kernel profiles are distinct
- red-zone and frame rules are profile-owned
- panic/KDK routes cite profile selection

Risk: arm64e/PAC is reduced to instruction names.

Mitigation:

- pointer-auth state is modeled at pointer-value level
- raw/stripped/authenticated/resolved states are distinct
- chained fixup and runtime pointer evidence are linked

## Acceptance Criteria For Hopper-Competitive Progress

Damsel is closer to Hopper when:

- architecture support is visible as a profile, not hidden inside a decoder
- CPU mode can be inspected and overridden safely
- register roles are stable facts, not string conventions
- CFG edges explain their semantic provenance
- switch hints attach to semantic records and reanalysis jobs
- ObjC message sends carry receiver/selector/callsite evidence
- C++ callsites carry `this`/vtable evidence where available
- Swift symbols and metadata are treated as ABI facts with safety constraints
- arm64e/PAC facts are explicit and not flattened into raw addresses
- XNU/kernel profiles avoid user-mode assumptions
- debugger, patching, decompiler, automation, and assistant surfaces consume
  the same semantic facts
- every uncertain fact can be inspected, cited, exported, suppressed, or
  corrected through a transaction

## Source Links

- Hopper homepage: <https://www.hopperapp.com/index.html>
- Hopper tutorial: <https://www.hopperapp.com/tutorial.html>
- Hopper FAQ: <https://www.hopperapp.com/faq.html>
- Hopper changelog: <https://www.hopperapp.com/rss/html_changelog.php>
- Hopper SDK v4 public mirror: <https://github.com/makigumo/HopperSDK-v4>
- Hopper SDK `CPUDefinition`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUDefinition.h>
- Hopper SDK `CPUContext`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CPUContext.h>
- Hopper SDK `HPDisassembledFile`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/HPDisassembledFile.h>
- Hopper SDK `FileLoader`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/FileLoader.h>
- Hopper SDK `CommonTypes`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/CommonTypes.h>
- Hopper SDK `DisasmStruct`: <https://raw.githubusercontent.com/makigumo/HopperSDK-v4/master/include/Hopper/DisasmStruct.h>
- Apple ARM64 ABI guidance: <https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms>
- Apple pointer authentication guidance: <https://developer.apple.com/documentation/security/preparing-your-app-to-work-with-pointer-authentication>
- Clang pointer authentication: <https://clang.llvm.org/docs/PointerAuthentication.html>
- Arm AAPCS64: <https://github.com/ARM-software/abi-aa/blob/main/aapcs64/aapcs64.rst>
- XNU Mach-O loader header: <https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/EXTERNAL_HEADERS/mach-o/loader.h>
- XNU fat archive header: <https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/EXTERNAL_HEADERS/mach-o/fat.h>
- Swift ABI mangling: <https://github.com/swiftlang/swift/blob/main/docs/ABI/Mangling.rst>
