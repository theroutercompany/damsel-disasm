use crate::errors::{MachoError, Result};
use damsel_core::{
    Annotation, BinaryImage, DecodedInstruction, DisassemblyLimit, DisassemblyRequest,
    DisassemblyRequestV2, DisassemblyResult, DisassemblyResultV2, DisassemblyStopReason,
    DisassemblyTarget, Import, Operand, Reference, Relocation, Section, Symbol,
};
use std::collections::BTreeMap;

pub fn disassemble_v2(
    image: &BinaryImage,
    request: &DisassemblyRequestV2,
) -> Result<DisassemblyResultV2> {
    Ok(disassemble_impl(image, request)?.into_v2())
}

pub fn disassemble(image: &BinaryImage, request: &DisassemblyRequest) -> Result<DisassemblyResult> {
    Ok(disassemble_impl(image, &request.to_v2())?.into_legacy())
}

fn disassemble_impl(
    image: &BinaryImage,
    request: &DisassemblyRequestV2,
) -> Result<DisassemblyComputation> {
    let window = resolve_disassembly_target_v2(image, request)?;
    let mut instructions =
        damsel_core::decode_aarch64(window.bytes, window.start_address, request.limit.instruction_cap())?;
    let include_annotations = request.options.include_annotations;
    if !include_annotations {
        for instruction in &mut instructions {
            instruction.annotations.clear();
        }
    }
    synthesize_analysis_references(&mut instructions, include_annotations);
    annotate_instructions(image, &mut instructions, include_annotations);
    let decoded_bytes = instructions
        .last()
        .map(|instruction| {
            instruction
                .address
                .saturating_add(u64::from(instruction.size))
                .saturating_sub(window.start_address) as usize
        })
        .unwrap_or(0);
    let end_address = instructions
        .last()
        .map(|instruction| {
            instruction
                .address
                .saturating_add(u64::from(instruction.size))
        })
        .unwrap_or(window.start_address);
    let instruction_count = instructions.len();
    let stop_reason = determine_stop_reason(request, &window, decoded_bytes, instruction_count);

    Ok(DisassemblyComputation {
        target: window.target,
        start_address: window.start_address,
        bytes_len: window.bytes.len(),
        decoded_bytes,
        end_address,
        instruction_count,
        stop_reason,
        instructions,
    })
}

struct DisassemblyComputation {
    target: String,
    start_address: u64,
    bytes_len: usize,
    decoded_bytes: usize,
    end_address: u64,
    instruction_count: usize,
    stop_reason: DisassemblyStopReason,
    instructions: Vec<DecodedInstruction>,
}

impl DisassemblyComputation {
    fn into_legacy(self) -> DisassemblyResult {
        DisassemblyResult {
            target: self.target,
            start_address: self.start_address,
            bytes_len: self.bytes_len,
            decoded_bytes: self.decoded_bytes,
            end_address: self.end_address,
            instruction_count: self.instruction_count,
            stop_reason: self.stop_reason,
            instructions: self.instructions,
        }
    }

    fn into_v2(self) -> DisassemblyResultV2 {
        DisassemblyResultV2 {
            target: self.target,
            start_address: self.start_address,
            decoded_bytes: self.decoded_bytes,
            end_address: self.end_address,
            instruction_count: self.instruction_count,
            stop_reason: self.stop_reason,
            instructions: self.instructions,
        }
    }
}

struct TargetWindow<'a> {
    target: String,
    start_address: u64,
    bytes: &'a [u8],
    request_limited: bool,
}

fn resolve_disassembly_target_v2<'a>(
    image: &'a BinaryImage,
    request: &DisassemblyRequestV2,
) -> Result<TargetWindow<'a>> {
    let window = match &request.target {
        DisassemblyTarget::Section(section_name) => {
            let section = image
                .section_by_name(section_name)
                .ok_or_else(|| MachoError::SectionNotFound(section_name.clone()))?;
            let data = image
                .bytes_for_section(section)
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let (bytes, request_limited) = clamp_to_request_limit_v2(request.limit, data);
            TargetWindow {
                target: section.full_name(),
                start_address: section.address,
                bytes,
                request_limited,
            }
        }
        DisassemblyTarget::Address(address) => {
            let section = image
                .containing_section(*address)
                .ok_or(MachoError::AddressNotMapped(*address))?;
            let file_offset = section
                .file_offset
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let delta = address.saturating_sub(section.address);
            let available = section.size.saturating_sub(delta) as usize;
            let size = match request.limit {
                DisassemblyLimit::Instructions(count) => count.saturating_mul(4),
                DisassemblyLimit::Bytes(bytes) => bytes,
                DisassemblyLimit::Unlimited => available,
            }
            .min(available);
            let data = image
                .bytes_for_file_range(file_offset + delta, size as u64)
                .ok_or(MachoError::AddressNotMapped(*address))?;
            TargetWindow {
                target: format!("{address:#x}"),
                start_address: *address,
                bytes: data,
                request_limited: size < available,
            }
        }
        DisassemblyTarget::Symbol(symbol_name) => {
            let symbol = image
                .symbol_by_name(symbol_name)
                .ok_or_else(|| MachoError::SymbolNotFound(symbol_name.clone()))?;
            let section = image
                .containing_section(symbol.address)
                .ok_or(MachoError::AddressNotMapped(symbol.address))?;
            let file_offset = section
                .file_offset
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let delta = symbol.address.saturating_sub(section.address);
            let inferred_size = if symbol.size > 0 {
                symbol.size
            } else {
                next_symbol_boundary(image, symbol, section)
                    .unwrap_or(section.address.saturating_add(section.size))
                    .saturating_sub(symbol.address)
            };
            let size = match request.limit {
                DisassemblyLimit::Instructions(count) => (count.saturating_mul(4)) as u64,
                DisassemblyLimit::Bytes(bytes) => bytes as u64,
                DisassemblyLimit::Unlimited => inferred_size,
            }
            .min(inferred_size)
            .min(section.size.saturating_sub(delta));
            let data = image
                .bytes_for_file_range(file_offset + delta, size)
                .ok_or(MachoError::AddressNotMapped(symbol.address))?;
            TargetWindow {
                target: symbol.name.clone(),
                start_address: symbol.address,
                bytes: data,
                request_limited: size < inferred_size.min(section.size.saturating_sub(delta)),
            }
        }
    };

    clamp_target_window_to_range(window, request.range.as_ref())
}

fn next_symbol_boundary(image: &BinaryImage, symbol: &Symbol, section: &Section) -> Option<u64> {
    image
        .symbols()
        .iter()
        .filter(|candidate| {
            candidate.defined
                && candidate.address > symbol.address
                && image
                    .containing_section(candidate.address)
                    .map(|candidate_section| candidate_section.full_name() == section.full_name())
                    .unwrap_or(false)
        })
        .map(|candidate| candidate.address)
        .min()
}

fn clamp_to_request_limit_v2(limit: DisassemblyLimit, bytes: &[u8]) -> (&[u8], bool) {
    let limit_bytes = match limit {
        DisassemblyLimit::Instructions(count) => Some(count.saturating_mul(4)),
        DisassemblyLimit::Bytes(bytes) => Some(bytes),
        DisassemblyLimit::Unlimited => None,
    };
    match limit_bytes {
        Some(limit) if limit < bytes.len() => (&bytes[..limit], true),
        _ => (bytes, false),
    }
}

fn clamp_target_window_to_range<'a>(
    window: TargetWindow<'a>,
    range: Option<&std::ops::Range<u64>>,
) -> Result<TargetWindow<'a>> {
    let Some(range) = range else {
        return Ok(window);
    };
    if range.start >= range.end {
        return Ok(TargetWindow {
            target: window.target,
            start_address: range.start,
            bytes: &window.bytes[..0],
            request_limited: true,
        });
    }

    let window_end = window.start_address.saturating_add(window.bytes.len() as u64);
    let clamped_start = range.start.max(window.start_address);
    let clamped_end = range.end.min(window_end);
    if clamped_start >= clamped_end {
        return Ok(TargetWindow {
            target: window.target,
            start_address: range.start,
            bytes: &window.bytes[..0],
            request_limited: true,
        });
    }

    let start_offset = usize::try_from(clamped_start.saturating_sub(window.start_address))
        .map_err(|_| MachoError::AddressNotMapped(clamped_start))?;
    let end_offset = usize::try_from(clamped_end.saturating_sub(window.start_address))
        .map_err(|_| MachoError::AddressNotMapped(clamped_end))?;

    Ok(TargetWindow {
        target: window.target,
        start_address: clamped_start,
        bytes: &window.bytes[start_offset..end_offset],
        request_limited: true,
    })
}

fn determine_stop_reason(
    request: &DisassemblyRequestV2,
    window: &TargetWindow<'_>,
    decoded_bytes: usize,
    instruction_count: usize,
) -> DisassemblyStopReason {
    if window.bytes.is_empty() {
        return DisassemblyStopReason::InputExhausted;
    }

    if image_limit_reached(
        request,
        instruction_count,
        decoded_bytes,
        window.bytes.len(),
    ) {
        return DisassemblyStopReason::LimitReached;
    }

    if instruction_count == 0 || decoded_bytes == 0 || decoded_bytes < window.bytes.len() {
        return DisassemblyStopReason::DecodeHalt;
    }

    if window.request_limited {
        return DisassemblyStopReason::LimitReached;
    }

    DisassemblyStopReason::TargetRangeEnd
}

fn image_limit_reached(
    request: &DisassemblyRequestV2,
    instruction_count: usize,
    decoded_bytes: usize,
    window_len: usize,
) -> bool {
    match request.limit {
        DisassemblyLimit::Instructions(count) => instruction_count >= count,
        DisassemblyLimit::Bytes(bytes) => {
            decoded_bytes >= bytes.min(window_len) && bytes <= window_len
        }
        DisassemblyLimit::Unlimited => false,
    }
}

fn annotate_instructions(
    image: &BinaryImage,
    instructions: &mut [DecodedInstruction],
    include_annotations: bool,
) {
    let symbol_map = image
        .symbols()
        .iter()
        .filter(|symbol| symbol.defined && symbol.address != 0)
        .fold(BTreeMap::<u64, Vec<&Symbol>>::new(), |mut acc, symbol| {
            acc.entry(symbol.address).or_default().push(symbol);
            acc
        });
    let import_address_map =
        image
            .imports()
            .iter()
            .fold(BTreeMap::<u64, Vec<&Import>>::new(), |mut acc, import| {
                if let Some(address) = import.address {
                    acc.entry(address).or_default().push(import);
                }
                acc
            });
    let import_name_map = image.imports().iter().fold(
        BTreeMap::<String, Vec<&Import>>::new(),
        |mut acc, import| {
            acc.entry(normalize_import_name(&import.name))
                .or_default()
                .push(import);
            acc
        },
    );

    for instruction in instructions {
        let mut derived = Vec::new();
        if include_annotations {
            push_instruction_address_annotations(&mut derived, &symbol_map, instruction.address);
        }
        let mut import_already_tagged = false;
        let mut synthesized_import_refs = Vec::<Reference>::new();

        if let Some(stub) = image.dyld().stub_for_address(instruction.address) {
            push_reference_if_missing(
                &mut instruction.references,
                Reference::Stub {
                    stub_address: stub.stub_address,
                    section: stub.section.clone(),
                    pointer_address: stub.pointer_address,
                    dylib: stub.dylib.clone(),
                    name: stub.name.clone(),
                    source: stub.source,
                },
            );
            if let (Some(dylib), Some(name)) = (&stub.dylib, &stub.name) {
                push_reference_if_missing(
                    &mut instruction.references,
                    Reference::Import {
                        name: name.clone(),
                        dylib: dylib.clone(),
                        address: stub.pointer_address,
                    },
                );
                if include_annotations {
                    push_derived(
                        &mut derived,
                        DerivedAnnotation::ImportBindingEvidence {
                            dylib: dylib.clone(),
                            name: name.clone(),
                            address: stub.pointer_address,
                            offset: None,
                            addend: 0,
                            source: stub.source,
                        },
                    );
                }
                import_already_tagged = true;
            }
        }

        for reference in instruction.references.clone() {
            match reference {
                Reference::Call { target }
                | Reference::Branch { target }
                | Reference::Page { target }
                | Reference::Data { target } => {
                    import_already_tagged |= push_reference_target_annotations(
                        include_annotations.then_some(&mut derived),
                        &symbol_map,
                        &import_address_map,
                        &import_name_map,
                        image,
                        &mut synthesized_import_refs,
                        target,
                    );
                    import_already_tagged |= push_dyld_target_hooks(
                        include_annotations.then_some(&mut derived),
                        image,
                        &mut instruction.references,
                        target,
                    );
                }
                Reference::IndirectCall { via } => {
                    if include_annotations {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::IndirectControlFlow {
                                kind: "call".to_string(),
                                via,
                            },
                        );
                    }
                }
                Reference::IndirectBranch { via } => {
                    if include_annotations {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::IndirectControlFlow {
                                kind: "branch".to_string(),
                                via,
                            },
                        );
                    }
                }
                Reference::Import {
                    name,
                    dylib,
                    address,
                } => {
                    if include_annotations {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::Import {
                                dylib: dylib.clone(),
                                name: name.clone(),
                            },
                        );
                        if let Some(address) = address {
                            push_derived(
                                &mut derived,
                                DerivedAnnotation::TargetSymbol {
                                    address,
                                    name: format!("{dylib}:{name}"),
                                },
                            );
                        }
                    }
                    import_already_tagged = true;
                }
                Reference::ImportBinding {
                    dylib,
                    name,
                    address,
                    offset,
                    addend,
                    source,
                    is_weak: _,
                } => {
                    push_reference_if_missing(
                        &mut synthesized_import_refs,
                        Reference::Import {
                            name: name.clone(),
                            dylib: dylib.clone(),
                            address,
                        },
                    );
                    if include_annotations {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::ImportBindingEvidence {
                                dylib,
                                name,
                                address,
                                offset,
                                addend,
                                source,
                            },
                        );
                    }
                    import_already_tagged = true;
                }
                Reference::Stub {
                    stub_address: _,
                    section: _,
                    pointer_address: _,
                    dylib,
                    name,
                    source,
                } => {
                    if include_annotations {
                        if let (Some(dylib), Some(name)) = (dylib, name) {
                            push_derived(
                                &mut derived,
                                DerivedAnnotation::ImportBindingEvidence {
                                    dylib,
                                    name,
                                    address: None,
                                    offset: None,
                                    addend: 0,
                                    source,
                                },
                            );
                        }
                    }
                }
                Reference::RelocationEvidence {
                    address,
                    kind,
                    encoding,
                    target,
                    addend,
                } => {
                    if include_annotations {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::Relocation {
                                address,
                                kind,
                                encoding,
                                target,
                                addend,
                            },
                        );
                    }
                }
            }
        }

        for relocation in overlapping_relocations(image.relocations(), instruction) {
            if include_annotations {
                push_derived(
                    &mut derived,
                    DerivedAnnotation::Relocation {
                        address: relocation.address,
                        kind: relocation.kind.clone(),
                        encoding: relocation.encoding.clone(),
                        target: relocation.target.clone(),
                        addend: relocation.addend,
                    },
                );
            }

            if let Some(imports) = import_name_map.get(&normalize_import_name(&relocation.target)) {
                for import in imports {
                    push_reference_if_missing(
                        &mut synthesized_import_refs,
                        Reference::Import {
                            name: import.name.clone(),
                            dylib: import.dylib.clone(),
                            address: import.address,
                        },
                    );
                    if include_annotations && !import_already_tagged {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::Import {
                                dylib: import.dylib.clone(),
                                name: import.name.clone(),
                            },
                        );
                        import_already_tagged = true;
                    }
                }
            }
        }

        for reference in synthesized_import_refs {
            if !instruction.references.contains(&reference) {
                instruction.references.push(reference);
            }
        }

        if include_annotations {
            for item in derived {
                push_annotation(&mut instruction.annotations, item.into_annotation());
            }
        }
    }
}

fn synthesize_analysis_references(
    instructions: &mut [DecodedInstruction],
    include_annotations: bool,
) {
    let mut known_values = BTreeMap::<String, u64>::new();

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let destination = destination_register(instruction);
        let mut destination_updated = false;

        if lower == "adrp" || lower == "adr" {
            if let (Some(target), Some(register)) =
                (page_reference_target(instruction), destination.as_ref())
            {
                known_values.insert(register.clone(), target);
                destination_updated = true;
                if include_annotations {
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::Note(format!("page base {register} -> {target:#x}")),
                    );
                }
            }
        }

        if let Some((register, value)) = synthesize_mov_wide_value(instruction, &known_values) {
            known_values.insert(register, value);
            destination_updated = true;
        }

        let add_inputs = add_immediate_inputs(instruction);
        if let Some((dest, base_register, displacement)) = add_inputs {
            if let Some(base) = known_values.get(base_register.as_str()).copied() {
                if let Some(target) = add_signed(base, displacement) {
                    if let Some(dest) = dest {
                        known_values.insert(dest, target);
                        destination_updated = true;
                    }
                    push_reference_if_missing(
                        &mut instruction.references,
                        Reference::Data { target },
                    );
                    if include_annotations {
                        push_annotation(
                            &mut instruction.annotations,
                            Annotation::Note(format!(
                                "add synthesis {} + {displacement:#x} -> {target:#x}",
                                base_register
                            )),
                        );
                    }
                }
            }
        }

        let memory_inputs = memory_base_displacement(instruction);
        if let Some((base_register, displacement)) = memory_inputs {
            if let Some(base) = known_values.get(base_register.as_str()).copied() {
                if let Some(target) = add_signed(base, displacement) {
                    push_reference_if_missing(
                        &mut instruction.references,
                        Reference::Data { target },
                    );
                    if include_annotations {
                        push_annotation(
                            &mut instruction.annotations,
                            Annotation::Note(format!(
                                "memory synthesis {} + {displacement:#x} -> {target:#x}",
                                base_register
                            )),
                        );
                    }
                }
            }
        }

        let indirect_register = indirect_control_register(instruction);
        if let Some(register) = indirect_register {
            let indirect_reference = if is_call_mnemonic(&lower) {
                Reference::IndirectCall {
                    via: register.clone(),
                }
            } else {
                Reference::IndirectBranch {
                    via: register.clone(),
                }
            };
            push_reference_if_missing(&mut instruction.references, indirect_reference);

            if let Some(target) = known_values.get(register.as_str()).copied() {
                let resolved_reference = if is_call_mnemonic(&lower) {
                    Reference::Call { target }
                } else {
                    Reference::Branch { target }
                };
                push_reference_if_missing(&mut instruction.references, resolved_reference);
                if include_annotations {
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::IndirectControlFlow {
                            kind: if is_call_mnemonic(&lower) {
                                "call".to_string()
                            } else {
                                "branch".to_string()
                            },
                            via: register.clone(),
                        },
                    );
                }
            } else if include_annotations {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::IndirectControlFlow {
                        kind: if is_call_mnemonic(&lower) {
                            "call".to_string()
                        } else {
                            "branch".to_string()
                        },
                        via: register,
                    },
                );
            }
        }

        if let Some(destination) = destination {
            if !destination_updated && instruction_writes_destination(&lower) {
                known_values.remove(&destination);
            }
        }

        if is_hard_control_flow_boundary(&lower) && !matches!(lower.as_str(), "bl" | "blr") {
            known_values.clear();
        }
    }
}

fn page_reference_target(instruction: &DecodedInstruction) -> Option<u64> {
    instruction
        .references
        .iter()
        .find_map(|reference| match reference {
            Reference::Page { target } => Some(*target),
            _ => None,
        })
}

fn destination_register(instruction: &DecodedInstruction) -> Option<String> {
    match instruction.operands.first() {
        Some(Operand::Register(register)) => Some(register.clone()),
        Some(Operand::QualifiedRegister { register, .. }) => Some(register.clone()),
        Some(Operand::SystemRegister(register)) => Some(register.clone()),
        _ => None,
    }
}

fn add_immediate_inputs(instruction: &DecodedInstruction) -> Option<(Option<String>, String, i64)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if lower.starts_with("add") || lower == "sub" || lower.starts_with("subs") {
        if let [destination, Operand::Register(base), immediate, ..] =
            instruction.operands.as_slice()
        {
            let displacement = match immediate {
                Operand::ImmediateUnsigned(value) => i64::try_from(*value).ok()?,
                Operand::ImmediateSigned(value) => *value,
                Operand::Label(value) => i64::try_from(*value).ok()?,
                _ => return None,
            };
            let displacement = if lower.starts_with("sub") {
                displacement.saturating_neg()
            } else {
                displacement
            };
            let destination = match destination {
                Operand::Register(register) => Some(register.clone()),
                _ => None,
            };
            return Some((destination, base.clone(), displacement));
        }
    }

    None
}

fn memory_base_displacement(instruction: &DecodedInstruction) -> Option<(String, i64)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if !(lower.starts_with("ldr")
        || lower.starts_with("ldp")
        || lower.starts_with("str")
        || lower.starts_with("stp")
        || lower.starts_with("ldur")
        || lower.starts_with("stur"))
    {
        return None;
    }

    for operand in &instruction.operands {
        if let Operand::Memory {
            base, displacement, ..
        } = operand
        {
            return Some((base.clone(), *displacement));
        }
    }
    None
}

fn add_signed(base: u64, delta: i64) -> Option<u64> {
    if delta >= 0 {
        base.checked_add(delta as u64)
    } else {
        base.checked_sub(delta.unsigned_abs())
    }
}

fn indirect_control_register(instruction: &DecodedInstruction) -> Option<String> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if !matches!(
        lower.as_str(),
        "blr"
            | "br"
            | "blraa"
            | "blraaz"
            | "blrab"
            | "blrabz"
            | "braa"
            | "braaz"
            | "brab"
            | "brabz"
    ) {
        return None;
    }
    instruction
        .operands
        .iter()
        .find_map(|operand| match operand {
            Operand::Register(register) => Some(register.clone()),
            Operand::Memory { base, .. } => Some(base.clone()),
            _ => None,
        })
}

fn is_hard_control_flow_boundary(mnemonic: &str) -> bool {
    mnemonic == "ret"
        || mnemonic == "eret"
        || mnemonic.starts_with("b.")
        || mnemonic == "b"
        || mnemonic == "bl"
}

fn instruction_writes_destination(mnemonic: &str) -> bool {
    !(mnemonic == "cmp"
        || mnemonic == "cmn"
        || mnemonic == "tst"
        || mnemonic == "tbz"
        || mnemonic == "tbnz"
        || mnemonic == "cbz"
        || mnemonic == "cbnz"
        || mnemonic.starts_with("b.")
        || mnemonic == "b"
        || mnemonic == "bl"
        || mnemonic == "blr"
        || mnemonic == "ret")
}

fn is_call_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "bl" | "blr" | "blraa" | "blraaz" | "blrab" | "blrabz"
    )
}

fn synthesize_mov_wide_value(
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, u64>,
) -> Option<(String, u64)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if !(lower.starts_with("movz") || lower.starts_with("movn") || lower.starts_with("movk")) {
        return None;
    }

    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let imm = match instruction.operands.get(1)? {
        Operand::ImmediateUnsigned(value) => *value & 0xffff,
        Operand::ImmediateSigned(value) if *value >= 0 => (*value as u64) & 0xffff,
        _ => return None,
    };
    let shift = instruction
        .operands
        .get(2)
        .and_then(parse_shift_bits)
        .unwrap_or(0);
    let lane_mask = 0xffff_u64.checked_shl(shift).unwrap_or(0);
    let shifted_imm = (imm << shift) & lane_mask;

    let value = if lower.starts_with("movz") {
        shifted_imm
    } else if lower.starts_with("movn") {
        !shifted_imm
    } else {
        let previous = known_values.get(&destination).copied()?;
        (previous & !lane_mask) | shifted_imm
    };

    Some((destination, value))
}

fn parse_shift_bits(operand: &Operand) -> Option<u32> {
    let shift = match operand {
        Operand::ImmediateUnsigned(value) => u32::try_from(*value).ok()?,
        Operand::ImmediateSigned(value) if *value >= 0 => u32::try_from(*value).ok()?,
        Operand::Other(text) | Operand::Name(text) => parse_shift_from_text(text)?,
        _ => return None,
    };
    if shift <= 48 && shift % 16 == 0 {
        Some(shift)
    } else {
        None
    }
}

fn parse_shift_from_text(text: &str) -> Option<u32> {
    let digits = text
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

fn push_reference_if_missing(target: &mut Vec<Reference>, reference: Reference) {
    if !target.contains(&reference) {
        target.push(reference);
    }
}

fn push_instruction_address_annotations(
    out: &mut Vec<DerivedAnnotation>,
    symbol_map: &BTreeMap<u64, Vec<&Symbol>>,
    address: u64,
) {
    if let Some(symbols) = symbol_map.get(&address) {
        for symbol in symbols {
            push_derived(out, DerivedAnnotation::Symbol(symbol.name.clone()));
        }
    }
}

fn push_reference_target_annotations(
    mut out: Option<&mut Vec<DerivedAnnotation>>,
    symbol_map: &BTreeMap<u64, Vec<&Symbol>>,
    import_map: &BTreeMap<u64, Vec<&Import>>,
    import_name_map: &BTreeMap<String, Vec<&Import>>,
    image: &BinaryImage,
    synthesized_import_refs: &mut Vec<Reference>,
    target: u64,
) -> bool {
    let mut import_added = false;

    if let Some(symbols) = symbol_map.get(&target) {
        if let Some(out) = out.as_mut() {
            for symbol in symbols {
                push_derived(
                    out,
                    DerivedAnnotation::TargetSymbol {
                        address: target,
                        name: symbol.name.clone(),
                    },
                );
            }
        }

        for symbol in symbols {
            let normalized = normalize_import_name(&symbol.name);
            if let Some(imports) = import_name_map.get(&normalized) {
                for import in imports {
                    push_reference_if_missing(
                        synthesized_import_refs,
                        Reference::Import {
                            name: import.name.clone(),
                            dylib: import.dylib.clone(),
                            address: import.address,
                        },
                    );
                    if let Some(out) = out.as_mut() {
                        push_derived(
                            out,
                            DerivedAnnotation::Import {
                                dylib: import.dylib.clone(),
                                name: import.name.clone(),
                            },
                        );
                    }
                    import_added = true;
                }
            }
        }
    }

    if let Some(imports) = import_map.get(&target) {
        for import in imports {
            push_reference_if_missing(
                synthesized_import_refs,
                Reference::Import {
                    name: import.name.clone(),
                    dylib: import.dylib.clone(),
                    address: import.address,
                },
            );
            if let Some(out) = out.as_mut() {
                push_derived(
                    out,
                    DerivedAnnotation::Import {
                        dylib: import.dylib.clone(),
                        name: import.name.clone(),
                    },
                );
            }
            import_added = true;
        }
    }

    // Stub hook: attempt symbol-name import resolution in stub sections even when the
    // target is not a concrete import address.
    if let Some(section) = image.containing_section(target) {
        if section.name.contains("stub") {
            if let Some(symbols) = symbol_map.get(&target) {
                for symbol in symbols {
                    let normalized = normalize_import_name(&symbol.name);
                    if let Some(imports) = import_name_map.get(&normalized) {
                        for import in imports {
                            push_reference_if_missing(
                                synthesized_import_refs,
                                Reference::Import {
                                    name: import.name.clone(),
                                    dylib: import.dylib.clone(),
                                    address: import.address,
                                },
                            );
                            if let Some(out) = out.as_mut() {
                                push_derived(
                                    out,
                                    DerivedAnnotation::Import {
                                        dylib: import.dylib.clone(),
                                        name: import.name.clone(),
                                    },
                                );
                            }
                            import_added = true;
                        }
                    }
                }
            }
        }
    }

    import_added
}

fn push_dyld_target_hooks(
    mut out: Option<&mut Vec<DerivedAnnotation>>,
    image: &BinaryImage,
    instruction_references: &mut Vec<Reference>,
    target: u64,
) -> bool {
    let mut import_added = false;

    for binding in image.dyld().bindings_at_address(target) {
        push_reference_if_missing(
            instruction_references,
            Reference::ImportBinding {
                dylib: binding.dylib.clone(),
                name: binding.name.clone(),
                address: binding.address,
                offset: binding.offset,
                addend: binding.addend,
                source: binding.source,
                is_weak: binding.is_weak,
            },
        );
        push_reference_if_missing(
            instruction_references,
            Reference::Import {
                name: binding.name.clone(),
                dylib: binding.dylib.clone(),
                address: binding.address,
            },
        );
        if let Some(out) = out.as_mut() {
            push_derived(
                out,
                DerivedAnnotation::ImportBindingEvidence {
                    dylib: binding.dylib.clone(),
                    name: binding.name.clone(),
                    address: binding.address,
                    offset: binding.offset,
                    addend: binding.addend,
                    source: binding.source,
                },
            );
        }
        import_added = true;
    }

    if let Some(stub) = image.dyld().stub_for_address(target) {
        push_reference_if_missing(
            instruction_references,
            Reference::Stub {
                stub_address: stub.stub_address,
                section: stub.section.clone(),
                pointer_address: stub.pointer_address,
                dylib: stub.dylib.clone(),
                name: stub.name.clone(),
                source: stub.source,
            },
        );
        if let (Some(dylib), Some(name)) = (&stub.dylib, &stub.name) {
            push_reference_if_missing(
                instruction_references,
                Reference::Import {
                    name: name.clone(),
                    dylib: dylib.clone(),
                    address: stub.pointer_address,
                },
            );
            if let Some(out) = out.as_mut() {
                push_derived(
                    out,
                    DerivedAnnotation::ImportBindingEvidence {
                        dylib: dylib.clone(),
                        name: name.clone(),
                        address: stub.pointer_address,
                        offset: None,
                        addend: 0,
                        source: stub.source,
                    },
                );
            }
            import_added = true;
        }
    }

    import_added
}

fn overlapping_relocations<'a>(
    relocations: &'a [Relocation],
    instruction: &DecodedInstruction,
) -> impl Iterator<Item = &'a Relocation> {
    let start = instruction.address;
    let end = instruction
        .address
        .saturating_add(u64::from(instruction.size));
    relocations
        .iter()
        .filter(move |relocation| relocation.address >= start && relocation.address < end)
}

fn normalize_import_name(name: &str) -> String {
    name.trim_start_matches('_').to_ascii_lowercase()
}

fn push_derived(target: &mut Vec<DerivedAnnotation>, annotation: DerivedAnnotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DerivedAnnotation {
    Symbol(String),
    TargetSymbol {
        address: u64,
        name: String,
    },
    Import {
        dylib: String,
        name: String,
    },
    IndirectControlFlow {
        kind: String,
        via: String,
    },
    Relocation {
        address: u64,
        kind: String,
        encoding: String,
        target: String,
        addend: i64,
    },
    ImportBindingEvidence {
        dylib: String,
        name: String,
        address: Option<u64>,
        offset: Option<u64>,
        addend: i64,
        source: damsel_core::ImportBindingSource,
    },
}

impl DerivedAnnotation {
    fn into_annotation(self) -> Annotation {
        match self {
            Self::Symbol(name) => Annotation::Symbol(name),
            Self::TargetSymbol { address, name } => Annotation::TargetSymbol { address, name },
            Self::Import { dylib, name } => Annotation::Import { dylib, name },
            Self::IndirectControlFlow { kind, via } => {
                Annotation::IndirectControlFlow { kind, via }
            }
            Self::Relocation {
                address,
                kind,
                encoding,
                target,
                addend,
            } => Annotation::Relocation {
                address,
                kind,
                encoding,
                target,
                addend,
            },
            Self::ImportBindingEvidence {
                dylib,
                name,
                address,
                offset,
                addend,
                source,
            } => Annotation::ImportBindingEvidence {
                dylib,
                name,
                address,
                offset,
                addend,
                source,
            },
        }
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}
