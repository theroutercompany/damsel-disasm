use crate::errors::{MachoError, Result};
use damsel_core::{
    Annotation, BinaryImage, DecodedInstruction, DisassemblyRequest, DisassemblyResult,
    DisassemblyTarget, Import, Operand, Reference, Relocation, Section, Symbol,
};
use std::collections::BTreeMap;

pub fn disassemble(image: &BinaryImage, request: &DisassemblyRequest) -> Result<DisassemblyResult> {
    let (target, start_address, bytes) = resolve_disassembly_target(image, request)?;
    let mut instructions = damsel_core::decode_aarch64(
        bytes,
        start_address,
        image.effective_instruction_limit(request),
    )?;
    synthesize_adrp_pair_references(&mut instructions);
    annotate_instructions(image, &mut instructions);
    let decoded_bytes = instructions
        .last()
        .map(|instruction| {
            instruction
                .address
                .saturating_add(u64::from(instruction.size))
                .saturating_sub(start_address) as usize
        })
        .unwrap_or(0);
    let end_address = instructions
        .last()
        .map(|instruction| {
            instruction
                .address
                .saturating_add(u64::from(instruction.size))
        })
        .unwrap_or(start_address);
    let instruction_count = instructions.len();
    let stop_reason = if image
        .effective_instruction_limit(request)
        .is_some_and(|limit| instruction_count >= limit)
    {
        damsel_core::DisassemblyStopReason::LimitReached
    } else {
        damsel_core::DisassemblyStopReason::InputExhausted
    };

    Ok(DisassemblyResult {
        target,
        start_address,
        bytes_len: bytes.len(),
        decoded_bytes,
        end_address,
        instruction_count,
        stop_reason,
        instructions,
    })
}

fn resolve_disassembly_target<'a>(
    image: &'a BinaryImage,
    request: &DisassemblyRequest,
) -> Result<(String, u64, &'a [u8])> {
    match &request.target {
        DisassemblyTarget::Section(section_name) => {
            let section = image
                .section_by_name(section_name)
                .ok_or_else(|| MachoError::SectionNotFound(section_name.clone()))?;
            let data = image
                .bytes_for_section(section)
                .ok_or_else(|| MachoError::SectionHasNoFileData(section.full_name()))?;
            let limit_bytes = match request.limit {
                Some(damsel_core::DisassemblyLimit::Instructions(count)) => {
                    Some(count.saturating_mul(4))
                }
                Some(damsel_core::DisassemblyLimit::Bytes(bytes)) => Some(bytes),
                Some(damsel_core::DisassemblyLimit::Unlimited) => None,
                None => request
                    .max_instructions
                    .map(|count| count.saturating_mul(4)),
            };
            Ok((
                section.full_name(),
                section.address,
                clamp_bytes(data, limit_bytes),
            ))
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
                Some(damsel_core::DisassemblyLimit::Instructions(count)) => count.saturating_mul(4),
                Some(damsel_core::DisassemblyLimit::Bytes(bytes)) => bytes,
                Some(damsel_core::DisassemblyLimit::Unlimited) => available,
                None => request
                    .max_instructions
                    .map(|count| count.saturating_mul(4))
                    .unwrap_or(available),
            }
            .min(available);
            let data = image
                .bytes_for_file_range(file_offset + delta, size as u64)
                .ok_or(MachoError::AddressNotMapped(*address))?;
            Ok((format!("{address:#x}"), *address, data))
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
                Some(damsel_core::DisassemblyLimit::Instructions(count)) => {
                    (count.saturating_mul(4)) as u64
                }
                Some(damsel_core::DisassemblyLimit::Bytes(bytes)) => bytes as u64,
                Some(damsel_core::DisassemblyLimit::Unlimited) => inferred_size,
                None => request
                    .max_instructions
                    .map(|count| (count.saturating_mul(4)) as u64)
                    .unwrap_or(inferred_size),
            }
            .min(inferred_size)
            .min(section.size.saturating_sub(delta));
            let data = image
                .bytes_for_file_range(file_offset + delta, size)
                .ok_or(MachoError::AddressNotMapped(symbol.address))?;
            Ok((symbol.name.clone(), symbol.address, data))
        }
    }
}

fn next_symbol_boundary(image: &BinaryImage, symbol: &Symbol, section: &Section) -> Option<u64> {
    image
        .symbols
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

fn clamp_bytes(bytes: &[u8], limit: Option<usize>) -> &[u8] {
    match limit {
        Some(limit) => &bytes[..bytes.len().min(limit)],
        None => bytes,
    }
}

fn annotate_instructions(image: &BinaryImage, instructions: &mut [DecodedInstruction]) {
    let symbol_map = image
        .symbols
        .iter()
        .filter(|symbol| symbol.defined && symbol.address != 0)
        .fold(BTreeMap::<u64, Vec<&Symbol>>::new(), |mut acc, symbol| {
            acc.entry(symbol.address).or_default().push(symbol);
            acc
        });
    let import_address_map =
        image
            .imports
            .iter()
            .fold(BTreeMap::<u64, Vec<&Import>>::new(), |mut acc, import| {
                if let Some(address) = import.address {
                    acc.entry(address).or_default().push(import);
                }
                acc
            });
    let import_name_map = image.imports.iter().fold(
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
        push_instruction_address_annotations(&mut derived, &symbol_map, instruction.address);
        let mut import_already_tagged = false;

        for reference in instruction.references.clone() {
            match reference {
                Reference::Call { target }
                | Reference::Branch { target }
                | Reference::Page { target }
                | Reference::Data { target } => {
                    import_already_tagged |= push_reference_target_annotations(
                        &mut derived,
                        &symbol_map,
                        &import_address_map,
                        target,
                    );
                }
                Reference::Import {
                    name,
                    dylib,
                    address,
                } => {
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
                    import_already_tagged = true;
                }
            }
        }

        for relocation in overlapping_relocations(&image.relocations, instruction) {
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

            if !import_already_tagged {
                if let Some(imports) =
                    import_name_map.get(&normalize_import_name(&relocation.target))
                {
                    for import in imports {
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

        for item in derived {
            push_annotation(&mut instruction.annotations, item.into_annotation());
        }
    }
}

fn synthesize_adrp_pair_references(instructions: &mut [DecodedInstruction]) {
    let mut page_base_by_register = BTreeMap::<String, u64>::new();

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();

        if lower == "adrp" {
            if let (Some(target), Some(register)) = (
                page_reference_target(instruction),
                destination_register(instruction),
            ) {
                page_base_by_register.insert(register.clone(), target);
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::Note(format!("adrp base {register} -> {target:#x}")),
                );
            }
            continue;
        }

        if let Some((base_register, displacement)) = pairing_inputs(instruction)
            .map(|(base_register, displacement)| (base_register.to_string(), displacement))
        {
            if let Some(page_base) = page_base_by_register.get(base_register.as_str()).copied() {
                if let Some(target) = add_signed(page_base, displacement) {
                    let synthesized = Reference::Data { target };
                    if !instruction.references.contains(&synthesized) {
                        instruction.references.push(synthesized);
                    }
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::Note(format!(
                            "adrp pair {base_register} + {displacement:#x} -> {target:#x}"
                        )),
                    );
                }
            }
        }

        if let Some(register) = indirect_control_register(instruction).map(str::to_string) {
            if let Some(page_base) = page_base_by_register.get(register.as_str()).copied() {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::Note(format!("adrp hook {register} -> {page_base:#x}")),
                );
            }
        }

        if let Some(destination) = destination_register(instruction) {
            page_base_by_register.remove(&destination);
        }
        if is_hard_control_flow_boundary(&lower) {
            page_base_by_register.clear();
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

fn pairing_inputs(instruction: &DecodedInstruction) -> Option<(&str, i64)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if lower.starts_with("add") {
        if let [_, Operand::Register(base), immediate, ..] = instruction.operands.as_slice() {
            let displacement = match immediate {
                Operand::ImmediateUnsigned(value) => i64::try_from(*value).ok()?,
                Operand::ImmediateSigned(value) => *value,
                Operand::Label(value) => i64::try_from(*value).ok()?,
                _ => return None,
            };
            return Some((base.as_str(), displacement));
        }
    }

    if lower.starts_with("ldr")
        || lower.starts_with("str")
        || lower.starts_with("ldp")
        || lower.starts_with("stp")
    {
        for operand in &instruction.operands {
            if let Operand::Memory {
                base, displacement, ..
            } = operand
            {
                return Some((base.as_str(), *displacement));
            }
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

fn indirect_control_register(instruction: &DecodedInstruction) -> Option<&str> {
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
            Operand::Register(register) => Some(register.as_str()),
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
    out: &mut Vec<DerivedAnnotation>,
    symbol_map: &BTreeMap<u64, Vec<&Symbol>>,
    import_map: &BTreeMap<u64, Vec<&Import>>,
    target: u64,
) -> bool {
    let mut import_added = false;
    if let Some(symbols) = symbol_map.get(&target) {
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
    if let Some(imports) = import_map.get(&target) {
        for import in imports {
            push_derived(
                out,
                DerivedAnnotation::Import {
                    dylib: import.dylib.clone(),
                    name: import.name.clone(),
                },
            );
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
    Relocation {
        address: u64,
        kind: String,
        encoding: String,
        target: String,
        addend: i64,
    },
}

impl DerivedAnnotation {
    fn into_annotation(self) -> Annotation {
        match self {
            Self::Symbol(name) => Annotation::Symbol(name),
            Self::TargetSymbol { address, name } => Annotation::TargetSymbol { address, name },
            Self::Import { dylib, name } => Annotation::Import { dylib, name },
            Self::Relocation {
                address,
                kind,
                encoding,
                target,
                addend,
            } => Annotation::Note(format!(
                "reloc kind={kind} encoding={encoding} target={target} addend={addend} addr={address:#x}"
            )),
        }
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}
