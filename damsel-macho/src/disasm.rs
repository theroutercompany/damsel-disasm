use crate::errors::{MachoError, Result};
use damsel_core::{
    Annotation, BinaryImage, DecodedInstruction, DisassemblyLimit, DisassemblyRequest,
    DisassemblyRequestV2, DisassemblyResult, DisassemblyResultV2, DisassemblyStopReason,
    DisassemblyTarget, Import, ImportBindingKind, ImportBindingRecord, ImportBindingSource,
    IndirectTargetReason, Operand, RecoveredValue, RecoveredValueKind, RecoveredValueSource,
    Reference, Relocation, Section, StubHelperEntry, Symbol,
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
    let mut instructions = damsel_core::decode_aarch64(
        window.bytes,
        window.start_address,
        request.limit.instruction_cap(),
    )?;
    let include_annotations = request.options.include_annotations;
    let include_value_flow = request.options.include_value_flow;
    if !include_annotations {
        for instruction in &mut instructions {
            instruction.annotations.clear();
        }
    }
    if !include_value_flow {
        for instruction in &mut instructions {
            instruction.recovered_values.clear();
        }
    }
    if include_value_flow {
        synthesize_analysis_references(image, &mut instructions, include_annotations);
    }
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

    let window_end = window
        .start_address
        .saturating_add(window.bytes.len() as u64);
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
        return match request.limit {
            DisassemblyLimit::Instructions(_) => DisassemblyStopReason::InstructionLimitReached,
            DisassemblyLimit::Bytes(_) => DisassemblyStopReason::ByteLimitReached,
            DisassemblyLimit::Unlimited => DisassemblyStopReason::TargetRangeEnd,
        };
    }

    if instruction_count == 0 || decoded_bytes == 0 || decoded_bytes < window.bytes.len() {
        return DisassemblyStopReason::DecodeHalt;
    }

    if window.request_limited {
        return DisassemblyStopReason::WindowClipped;
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
        let lower = instruction.mnemonic.to_ascii_lowercase();
        if include_annotations {
            push_instruction_address_annotations(&mut derived, &symbol_map, instruction.address);
        }
        let mut import_already_tagged = false;
        let mut synthesized_import_refs = Vec::<Reference>::new();

        if let Some(helper) = image.dyld().helper_for_address(instruction.address) {
            import_already_tagged |= push_helper_context(
                include_annotations.then_some(&mut derived),
                image,
                &mut instruction.references,
                helper,
            );
        }

        if let Some(stub) = image.dyld().stub_for_address(instruction.address) {
            push_reference_if_missing(
                &mut instruction.references,
                Reference::Stub {
                    stub_address: stub.stub_address,
                    section: stub.section.clone(),
                    pointer_section: stub.pointer_section.clone(),
                    pointer_address: stub.pointer_address,
                    helper_address: stub.helper_address,
                    binding_ordinal: stub.binding_ordinal,
                    stub_kind: stub.stub_kind.clone(),
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
                            binding_kind: match stub.stub_kind {
                                damsel_core::StubKind::Lazy => ImportBindingKind::Lazy,
                                damsel_core::StubKind::NonLazy => ImportBindingKind::NonLazy,
                            },
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
                    let kind = indirect_control_kind(&lower);
                    if include_annotations
                        && !has_indirect_control_flow_annotation(
                            &instruction.annotations,
                            &via,
                            &kind,
                        )
                    {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::IndirectControlFlow { kind, via },
                        );
                    }
                }
                Reference::IndirectBranch { via } => {
                    let kind = indirect_control_kind(&lower);
                    if include_annotations
                        && !has_indirect_control_flow_annotation(
                            &instruction.annotations,
                            &via,
                            &kind,
                        )
                    {
                        push_derived(
                            &mut derived,
                            DerivedAnnotation::IndirectControlFlow { kind, via },
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
                    binding_kind,
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
                                binding_kind,
                                source,
                            },
                        );
                    }
                    import_already_tagged = true;
                }
                Reference::Stub {
                    stub_address: _,
                    section: _,
                    pointer_section: _,
                    pointer_address: _,
                    helper_address: _,
                    binding_ordinal: _,
                    stub_kind,
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
                                    binding_kind: match stub_kind {
                                        damsel_core::StubKind::Lazy => ImportBindingKind::Lazy,
                                        damsel_core::StubKind::NonLazy => {
                                            ImportBindingKind::NonLazy
                                        }
                                    },
                                    source,
                                },
                            );
                        }
                    }
                }
                Reference::StubHelper {
                    helper_address: _,
                    target_stub: _,
                    stub_section: _,
                    pointer_address: _,
                    pointer_section: _,
                    binding_ordinal: _,
                    dylib,
                    name,
                } => {
                    if include_annotations {
                        if let (Some(dylib), Some(name)) = (dylib, name) {
                            push_derived(&mut derived, DerivedAnnotation::Import { dylib, name });
                        }
                    }
                    import_already_tagged = true;
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
    image: &BinaryImage,
    instructions: &mut [DecodedInstruction],
    include_annotations: bool,
) {
    let mut known_values = BTreeMap::<String, RecoveredValue>::new();
    let mut jump_table_sources = BTreeMap::<String, (u64, String, u8)>::new();

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let destination = destination_register(instruction);
        let mut destination_updated = false;

        if lower == "adrp" || lower == "adr" {
            if let (Some(target), Some(register)) =
                (page_reference_target(instruction), destination.as_ref())
            {
                let target = normalize_value_for_write(register, target);
                let recovered = RecoveredValue {
                    register: register.clone(),
                    value: target,
                    kind: classify_recovered_value(image, target, RecoveredValueKind::Address),
                    source: RecoveredValueSource::Adr,
                };
                store_known_value(&mut known_values, recovered.clone());
                push_recovered_value(&mut instruction.recovered_values, recovered);
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
            let value = normalize_value_for_write(&register, value);
            let recovered = RecoveredValue {
                register: register.clone(),
                value,
                kind: classify_recovered_value(image, value, RecoveredValueKind::Literal),
                source: RecoveredValueSource::MoveWide,
            };
            store_known_value(&mut known_values, recovered.clone());
            push_recovered_value(&mut instruction.recovered_values, recovered);
            destination_updated = true;
        }

        if let Some((register, value, source)) =
            synthesize_simple_register_value(instruction, &known_values)
        {
            let value = normalize_value_for_write(&register, value);
            let recovered = RecoveredValue {
                register: register.clone(),
                value,
                kind: classify_recovered_value(image, value, RecoveredValueKind::Literal),
                source,
            };
            store_known_value(&mut known_values, recovered.clone());
            push_recovered_value(&mut instruction.recovered_values, recovered);
            destination_updated = true;
        }

        let add_inputs = add_immediate_inputs(instruction);
        if let Some((dest, base_register, displacement)) = add_inputs {
            if let Some(base) = read_known_value(&known_values, base_register.as_str()) {
                if let Some(target) = add_signed(base.value, displacement) {
                    if let Some(dest) = dest {
                        let target = normalize_value_for_write(&dest, target);
                        let recovered = RecoveredValue {
                            register: dest.clone(),
                            value: target,
                            kind: classify_recovered_value(
                                image,
                                target,
                                RecoveredValueKind::Address,
                            ),
                            source: RecoveredValueSource::AdrpAdd,
                        };
                        store_known_value(&mut known_values, recovered.clone());
                        push_recovered_value(&mut instruction.recovered_values, recovered);
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

        if let Some((target, recovered)) = synthesize_literal_load(image, instruction, &lower) {
            push_reference_if_missing(&mut instruction.references, Reference::Data { target });
            push_recovered_value(&mut instruction.recovered_values, recovered.clone());
            store_known_value(&mut known_values, recovered);
            destination_updated = true;
        }

        let memory_inputs = memory_base_index_displacement(instruction);
        if let Some((base_register, index_register, displacement)) = memory_inputs {
            if let Some(base) = read_known_value(&known_values, base_register.as_str()) {
                if let Some(base_target) = add_signed(base.value, displacement) {
                    let element_size = jump_table_element_size(instruction);
                    let effective_target = resolve_memory_effective_address(
                        base_target,
                        index_register.as_deref(),
                        &known_values,
                        element_size,
                    );
                    let reference_target = effective_target.unwrap_or(base_target);
                    push_reference_if_missing(
                        &mut instruction.references,
                        Reference::Data {
                            target: reference_target,
                        },
                    );
                    if let Some(index_register) = index_register.clone() {
                        if let Some(jump_key) = destination
                            .as_deref()
                            .or(Some(base_register.as_str()))
                            .and_then(canonical_state_key)
                        {
                            jump_table_sources.insert(
                                jump_key,
                                (base_target, index_register.clone(), element_size),
                            );
                        }
                    }
                    if let Some(dest) = destination.as_ref() {
                        if lower.starts_with("ldr") || lower.starts_with("ldur") {
                            let loaded_value = effective_target.and_then(|slot_address| {
                                resolve_loaded_pointer_value(
                                    image,
                                    slot_address,
                                    lower.as_str(),
                                    dest.as_str(),
                                )
                            });
                            let value = loaded_value
                                .as_ref()
                                .map(|(resolved, _)| *resolved)
                                .unwrap_or(reference_target);
                            let recovered = RecoveredValue {
                                register: dest.clone(),
                                value: normalize_value_for_write(dest, value),
                                kind: loaded_value
                                    .map(|(_, kind)| kind)
                                    .unwrap_or_else(|| {
                                        classify_recovered_value(
                                            image,
                                            value,
                                            RecoveredValueKind::Address,
                                        )
                                    }),
                                source: if loaded_value.is_some() {
                                    RecoveredValueSource::TableLoad
                                } else {
                                    RecoveredValueSource::AdrpLoad
                                },
                            };
                            store_known_value(&mut known_values, recovered.clone());
                            push_recovered_value(&mut instruction.recovered_values, recovered);
                            destination_updated = true;
                            if include_annotations {
                                if let Some((target, _)) = loaded_value {
                                    push_annotation(
                                        &mut instruction.annotations,
                                        Annotation::TableSlotResolved {
                                            table_base: base_target,
                                            slot_address: reference_target,
                                            index_register: index_register
                                                .clone()
                                                .unwrap_or_default(),
                                            element_size,
                                            target,
                                        },
                                    );
                                }
                            }
                        }
                    }
                    if include_annotations {
                        push_annotation(
                            &mut instruction.annotations,
                            Annotation::Note(format!(
                                "memory synthesis {} + {displacement:#x} -> {reference_target:#x}",
                                base_register
                            )),
                        );
                    }
                }
            }
        }

        let indirect_register = indirect_control_register(instruction);
        if let Some(register) = indirect_register {
            let indirect_kind = indirect_control_kind(&lower);
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

            let jump_table_source = canonical_state_key(&register)
                .and_then(|key| jump_table_sources.get(&key).cloned());
            let mut resolved_target = read_known_value(&known_values, register.as_str())
                .map(|value| (value.value, indirect_target_reason(image, value.value)));

            if let Some((base, index_register, element_size)) = jump_table_source.clone()
                && let Some(index_value) = read_known_value(&known_values, index_register.as_str())
                    .map(|value| value.value)
            {
                if let Some((slot_address, target, reason)) =
                    resolve_table_slot_target(image, base, index_value, element_size)
                {
                    push_reference_if_missing(
                        &mut instruction.references,
                        Reference::Data {
                            target: slot_address,
                        },
                    );
                    resolved_target = Some((target, reason));
                    if include_annotations {
                        push_annotation(
                            &mut instruction.annotations,
                            Annotation::TableSlotResolved {
                                table_base: base,
                                slot_address,
                                index_register: index_register.clone(),
                                element_size,
                                target,
                            },
                        );
                    }
                }
                if include_annotations {
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::JumpTableCandidate {
                            base,
                            index_register,
                            element_size,
                        },
                    );
                }
            }

            if let Some((target, reason)) = resolved_target {
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
                            kind: indirect_kind.clone(),
                            via: register.clone(),
                        },
                    );
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::IndirectTargetResolved {
                            via: register.clone(),
                            target,
                            reason,
                        },
                    );
                }
            } else if include_annotations {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::IndirectControlFlow {
                        kind: indirect_kind.clone(),
                        via: register.clone(),
                    },
                );
            }
        }

        if let Some(destination) = destination {
            if !destination_updated && instruction_writes_destination(&lower) {
                remove_known_value(&mut known_values, &destination);
            }
        }

        if is_hard_control_flow_boundary(&lower) && !matches!(lower.as_str(), "bl" | "blr") {
            known_values.clear();
            jump_table_sources.clear();
        }
    }
}

fn canonical_state_key(register: &str) -> Option<String> {
    let lower = register.to_ascii_lowercase();
    if matches!(lower.as_str(), "xzr" | "wzr" | "sp" | "wsp") {
        return None;
    }
    if let Some(index) = lower.strip_prefix('x').or_else(|| lower.strip_prefix('w')) {
        if !index.is_empty() && index.chars().all(|ch| ch.is_ascii_digit()) {
            return Some(format!("x{index}"));
        }
    }
    Some(lower)
}

fn normalize_value_for_write(register: &str, value: u64) -> u64 {
    if register.to_ascii_lowercase().starts_with('w') {
        value & u64::from(u32::MAX)
    } else {
        value
    }
}

fn read_known_value(
    known_values: &BTreeMap<String, RecoveredValue>,
    register: &str,
) -> Option<RecoveredValue> {
    let key = canonical_state_key(register)?;
    let mut recovered = known_values.get(&key)?.clone();
    recovered.register = register.to_string();
    recovered.value = normalize_value_for_write(register, recovered.value);
    Some(recovered)
}

fn store_known_value(
    known_values: &mut BTreeMap<String, RecoveredValue>,
    mut recovered: RecoveredValue,
) {
    let Some(key) = canonical_state_key(&recovered.register) else {
        return;
    };
    recovered.value = normalize_value_for_write(&recovered.register, recovered.value);
    recovered.register = key.clone();
    known_values.insert(key, recovered);
}

fn remove_known_value(known_values: &mut BTreeMap<String, RecoveredValue>, register: &str) {
    if let Some(key) = canonical_state_key(register) {
        known_values.remove(&key);
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

fn push_recovered_value(target: &mut Vec<RecoveredValue>, value: RecoveredValue) {
    if !target.iter().any(|existing| {
        existing.register == value.register
            && existing.value == value.value
            && existing.source == value.source
    }) {
        target.push(value);
    }
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

fn memory_base_index_displacement(
    instruction: &DecodedInstruction,
) -> Option<(String, Option<String>, i64)> {
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
            base,
            index,
            displacement,
            ..
        } = operand
        {
            return Some((base.clone(), index.clone(), *displacement));
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

fn indirect_control_kind(mnemonic: &str) -> String {
    if matches!(mnemonic, "blraa" | "blraaz" | "blrab" | "blrabz") {
        "authenticated-call".to_string()
    } else if matches!(mnemonic, "braa" | "braaz" | "brab" | "brabz") {
        "authenticated-branch".to_string()
    } else if is_call_mnemonic(mnemonic) {
        "call".to_string()
    } else {
        "branch".to_string()
    }
}

fn is_hard_control_flow_boundary(mnemonic: &str) -> bool {
    mnemonic == "ret"
        || mnemonic == "retaa"
        || mnemonic == "retab"
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
        || mnemonic == "retaa"
        || mnemonic == "retab"
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
    known_values: &BTreeMap<String, RecoveredValue>,
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
        let previous = read_known_value(known_values, &destination)?.value;
        (previous & !lane_mask) | shifted_imm
    };

    Some((destination, value))
}

fn synthesize_simple_register_value(
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    match lower.as_str() {
        "mov" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register.clone(),
                _ => return None,
            };
            let source = match instruction.operands.get(1)? {
                Operand::Register(register) => register,
                _ => return None,
            };
            Some((
                destination,
                read_known_value(known_values, source)?.value,
                RecoveredValueSource::Other,
            ))
        }
        "and" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register.clone(),
                _ => return None,
            };
            let source = match instruction.operands.get(1)? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let mask = match instruction.operands.get(2)? {
                Operand::ImmediateUnsigned(value) => *value,
                Operand::ImmediateSigned(value) if *value >= 0 => *value as u64,
                Operand::Label(value) => *value,
                _ => return None,
            };
            Some((
                destination,
                read_known_value(known_values, source)?.value & mask,
                RecoveredValueSource::Other,
            ))
        }
        _ => None,
    }
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

fn resolve_memory_effective_address(
    base_target: u64,
    index_register: Option<&str>,
    known_values: &BTreeMap<String, RecoveredValue>,
    element_size: u8,
) -> Option<u64> {
    let Some(index_register) = index_register else {
        return Some(base_target);
    };
    let index = read_known_value(known_values, index_register)?.value;
    let slot_offset = index.checked_mul(u64::from(element_size))?;
    base_target.checked_add(slot_offset)
}

fn resolve_loaded_pointer_value(
    image: &BinaryImage,
    slot_address: u64,
    mnemonic: &str,
    destination: &str,
) -> Option<(u64, RecoveredValueKind)> {
    if mnemonic.starts_with("ldrsw") || destination.starts_with('w') {
        return None;
    }
    if image.dyld().bindings_at_address(slot_address).next().is_some()
        || image.dyld().stub_for_pointer_address(slot_address).is_some()
    {
        return None;
    }
    let (_, bytes) = image.bytes_for_virtual_range(slot_address, 8)?;
    let raw = u64::from_le_bytes(bytes.try_into().ok()?);
    if raw == 0 {
        return None;
    }
    if image.dyld().helper_for_address(raw).is_some() {
        return Some((raw, RecoveredValueKind::Address));
    }
    let kind = classify_recovered_value(image, raw, RecoveredValueKind::Address);
    if matches!(kind, RecoveredValueKind::Address | RecoveredValueKind::CString) {
        return None;
    }
    Some((raw, kind))
}

fn synthesize_literal_load(
    image: &BinaryImage,
    instruction: &DecodedInstruction,
    lower: &str,
) -> Option<(u64, RecoveredValue)> {
    if !(lower.starts_with("ldr") || lower.starts_with("ldrsw")) {
        return None;
    }
    let register = destination_register(instruction)?;
    let target = instruction
        .references
        .iter()
        .find_map(|reference| match reference {
            Reference::Data { target } => Some(*target),
            _ => None,
        })?;
    Some((
        target,
        RecoveredValue {
            register,
            value: target,
            kind: classify_recovered_value(image, target, RecoveredValueKind::Literal),
            source: RecoveredValueSource::LiteralLoad,
        },
    ))
}

fn classify_recovered_value(
    image: &BinaryImage,
    value: u64,
    fallback: RecoveredValueKind,
) -> RecoveredValueKind {
    if image.dyld().bindings_at_address(value).next().is_some() {
        return RecoveredValueKind::ImportPointer;
    }
    if image.dyld().stub_for_address(value).is_some() {
        return RecoveredValueKind::StubAddress;
    }
    if image.dyld().export_by_address(value).is_some() {
        return RecoveredValueKind::ExportAddress;
    }
    if image.symbols().iter().any(|symbol| {
        symbol.address == value && matches!(symbol.kind, damsel_core::SymbolKind::Text)
    }) {
        return RecoveredValueKind::FunctionPointer;
    }
    if image.objc_selector_name_at_address(value).is_some() {
        return RecoveredValueKind::ObjcSelector;
    }
    if image.objc_class_name_at_address(value).is_some() {
        return RecoveredValueKind::ObjcClass;
    }
    if image.objc_method_list_owner_at_address(value).is_some() {
        return RecoveredValueKind::ObjcMethodList;
    }
    if image.read_c_string_at_address(value, 128).is_some() {
        return RecoveredValueKind::CString;
    }
    if image.containing_section(value).is_some() {
        return RecoveredValueKind::Address;
    }
    fallback
}

fn indirect_target_reason(image: &BinaryImage, target: u64) -> IndirectTargetReason {
    if image.dyld().helper_for_address(target).is_some() {
        IndirectTargetReason::HelperTarget
    } else if image.dyld().stub_for_address(target).is_some() {
        IndirectTargetReason::StubTarget
    } else if image.dyld().export_by_address(target).is_some() {
        IndirectTargetReason::ExportAddress
    } else if image.dyld().bindings_at_address(target).next().is_some() {
        IndirectTargetReason::ImportPointer
    } else if image.symbols().iter().any(|symbol| {
        symbol.address == target && matches!(symbol.kind, damsel_core::SymbolKind::Text)
    }) {
        IndirectTargetReason::FunctionPointer
    } else {
        IndirectTargetReason::RegisterState
    }
}

fn is_executable_target(image: &BinaryImage, target: u64) -> bool {
    image
        .containing_section(target)
        .map(|section| section.executable)
        .unwrap_or(false)
}

fn resolve_table_slot_target(
    image: &BinaryImage,
    base: u64,
    index_value: u64,
    element_size: u8,
) -> Option<(u64, u64, IndirectTargetReason)> {
    if element_size != 4 && element_size != 8 {
        return None;
    }
    if index_value > 0x1000 {
        return None;
    }
    let slot_offset = index_value.checked_mul(u64::from(element_size))?;
    let slot_address = base.checked_add(slot_offset)?;
    let (_, bytes) = image.bytes_for_virtual_range(slot_address, usize::from(element_size))?;

    if element_size == 8 {
        let raw = u64::from_le_bytes(bytes.try_into().ok()?);
        if is_executable_target(image, raw) || image.dyld().export_by_address(raw).is_some() {
            return Some((slot_address, raw, indirect_target_reason(image, raw)));
        }
        return None;
    }

    let raw_u32 = u32::from_le_bytes(bytes.try_into().ok()?);
    let absolute = u64::from(raw_u32);
    if is_executable_target(image, absolute) || image.dyld().export_by_address(absolute).is_some() {
        return Some((slot_address, absolute, indirect_target_reason(image, absolute)));
    }

    let relative = i32::from_le_bytes(raw_u32.to_le_bytes()) as i64;
    let relative_target = add_signed(base, relative)?;
    if is_executable_target(image, relative_target)
        || image.dyld().export_by_address(relative_target).is_some()
    {
        return Some((
            slot_address,
            relative_target,
            indirect_target_reason(image, relative_target),
        ));
    }

    None
}

fn jump_table_element_size(instruction: &DecodedInstruction) -> u8 {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if lower.starts_with("ldrsw") {
        4
    } else if destination_register(instruction)
        .as_deref()
        .is_some_and(|register| register.starts_with('w'))
    {
        4
    } else {
        8
    }
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
    _image: &BinaryImage,
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
    import_added
}

fn push_dyld_target_hooks(
    mut out: Option<&mut Vec<DerivedAnnotation>>,
    image: &BinaryImage,
    instruction_references: &mut Vec<Reference>,
    target: u64,
) -> bool {
    let mut import_added = false;

    if let Some(helper) = image.dyld().helper_for_address(target) {
        import_added |= push_helper_context(out.take(), image, instruction_references, helper);
    }

    for binding in image.dyld().bindings_at_address(target) {
        push_reference_if_missing(
            instruction_references,
            Reference::ImportBinding {
                dylib: binding.dylib.clone(),
                name: binding.name.clone(),
                address: binding.address,
                offset: binding.offset,
                addend: binding.addend,
                binding_kind: binding.binding_kind,
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
                    binding_kind: binding.binding_kind,
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
                pointer_section: stub.pointer_section.clone(),
                pointer_address: stub.pointer_address,
                helper_address: stub.helper_address,
                binding_ordinal: stub.binding_ordinal,
                stub_kind: stub.stub_kind.clone(),
                dylib: stub.dylib.clone(),
                name: stub.name.clone(),
                source: stub.source,
            },
        );
        if let Some(helper) = image.dyld().helper_for_stub_address(stub.stub_address) {
            push_reference_if_missing(instruction_references, Reference::from_stub_helper(helper));
        }
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
                        binding_kind: match stub.stub_kind {
                            damsel_core::StubKind::Lazy => ImportBindingKind::Lazy,
                            damsel_core::StubKind::NonLazy => ImportBindingKind::NonLazy,
                        },
                        source: stub.source,
                    },
                );
            }
            import_added = true;
        }
    }

    import_added
}

fn push_helper_context(
    mut out: Option<&mut Vec<DerivedAnnotation>>,
    image: &BinaryImage,
    instruction_references: &mut Vec<Reference>,
    helper: &StubHelperEntry,
) -> bool {
    push_reference_if_missing(instruction_references, Reference::from_stub_helper(helper));

    if let Some(binding) = resolve_helper_binding(image, helper) {
        push_reference_if_missing(instruction_references, Reference::from_binding(binding));
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
                    binding_kind: binding.binding_kind,
                    source: binding.source,
                },
            );
        }
        return true;
    }

    if let (Some(dylib), Some(name)) = (&helper.dylib, &helper.name) {
        push_reference_if_missing(
            instruction_references,
            Reference::Import {
                name: name.clone(),
                dylib: dylib.clone(),
                address: helper.pointer_address,
            },
        );
        if let Some(out) = out.as_mut() {
            push_derived(
                out,
                DerivedAnnotation::Import {
                    dylib: dylib.clone(),
                    name: name.clone(),
                },
            );
            push_derived(
                out,
                DerivedAnnotation::ImportBindingEvidence {
                    dylib: dylib.clone(),
                    name: name.clone(),
                    address: helper.pointer_address,
                    offset: None,
                    addend: 0,
                    binding_kind: ImportBindingKind::Lazy,
                    source: ImportBindingSource::Stub,
                },
            );
        }
        return true;
    }

    false
}

fn resolve_helper_binding<'a>(
    image: &'a BinaryImage,
    helper: &StubHelperEntry,
) -> Option<&'a ImportBindingRecord> {
    if let Some(pointer_address) = helper.pointer_address {
        if let Some(binding) = image
            .dyld()
            .bindings_at_address(pointer_address)
            .find(|binding| binding.binding_kind == ImportBindingKind::Lazy)
        {
            return Some(binding);
        }
    }

    if let Some(binding_ordinal) = helper.binding_ordinal {
        if let Some(binding) = image.dyld().import_bindings.iter().find(|binding| {
            binding.ordinal == Some(binding_ordinal)
                && binding.binding_kind == ImportBindingKind::Lazy
        }) {
            return Some(binding);
        }
    }

    match (&helper.dylib, &helper.name) {
        (Some(dylib), Some(name)) => {
            let mut matches = image.dyld().import_bindings.iter().filter(|binding| {
                binding.dylib == *dylib
                    && binding.name == *name
                    && binding.binding_kind == ImportBindingKind::Lazy
            });
            let first = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            Some(first)
        }
        _ => None,
    }
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

fn has_indirect_control_flow_annotation(annotations: &[Annotation], via: &str, kind: &str) -> bool {
    annotations.iter().any(|annotation| {
        matches!(
            annotation,
            Annotation::IndirectControlFlow {
                kind: existing_kind,
                via: existing_via,
            } if existing_via == via && existing_kind == kind
        )
    })
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
        binding_kind: ImportBindingKind,
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
                binding_kind,
                source,
            } => Annotation::ImportBindingEvidence {
                dylib,
                name,
                address,
                offset,
                addend,
                binding_kind,
                source,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use damsel_core::{
        Architecture, BinaryFormat, BinaryImage, Endianness, ExportFlags, ExportKind, ObjcMetadata,
        Platform, Section, SliceInfo, Symbol, SymbolKind,
    };
    use std::sync::Arc;

    fn synthetic_image() -> BinaryImage {
        let bytes: Arc<[u8]> = vec![0u8; 64].into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: 64,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![Section {
                segment_name: "__TEXT".to_string(),
                name: "__text".to_string(),
                address: 0x3000,
                size: 64,
                file_offset: Some(0),
                file_size: 64,
                kind: "Text".to_string(),
                executable: true,
            }],
            vec![Symbol {
                name: "_dispatch_target".to_string(),
                address: 0x3000,
                size: 16,
                kind: SymbolKind::Text,
                defined: true,
                global: true,
                weak: false,
                section: Some("__text".to_string()),
            }],
            vec![],
            vec![],
            ObjcMetadata::default(),
            damsel_core::DyldMetadata {
                exported_symbols: vec![damsel_core::ExportRecord {
                    name: "_exported".to_string(),
                    address: Some(0x2000),
                    raw_flags: "Regular".to_string(),
                    flags: ExportFlags::from_bits(0),
                    kind: ExportKind::Regular,
                    reexport_target: None,
                    resolver_target: None,
                }],
                ..damsel_core::DyldMetadata::default()
            },
            bytes,
        )
    }

    fn instruction(address: u64, mnemonic: &str, operands: Vec<Operand>) -> DecodedInstruction {
        DecodedInstruction {
            address,
            size: 4,
            opcode: 0,
            mnemonic: mnemonic.to_string(),
            operands,
            recovered_values: Vec::new(),
            references: Vec::new(),
            annotations: Vec::new(),
        }
    }

    #[test]
    fn synthesize_analysis_references_tracks_w_to_x_aliases() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "movz",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(0x1004, "blr", vec![Operand::Register("x8".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Call { target: 0x1234 })
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_x_to_w_to_x_aliases() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "movz",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1004,
                "add",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Call { target: 0x1238 })
        );
    }

    #[test]
    fn synthesize_literal_load_classifies_export_addresses() {
        let image = synthetic_image();
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x2000 }],
            ..instruction(0x1000, "ldr", vec![Operand::Register("x0".to_string())])
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::ExportAddress && value.value == 0x2000
        }));
    }

    #[test]
    fn synthesize_literal_load_classifies_function_pointers() {
        let image = synthetic_image();
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x3000 }],
            ..instruction(0x1000, "ldr", vec![Operand::Register("x0".to_string())])
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer && value.value == 0x3000
        }));
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}
