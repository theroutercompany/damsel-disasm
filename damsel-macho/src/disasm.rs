use crate::errors::{MachoError, Result};
use damsel_core::{
    Annotation, BinaryImage, DecodedInstruction, DisassemblyRequest, DisassemblyResult,
    DisassemblyTarget, Import, Reference, Section, Symbol,
};
use std::collections::BTreeMap;

pub fn disassemble(image: &BinaryImage, request: &DisassemblyRequest) -> Result<DisassemblyResult> {
    let (target, start_address, bytes) = resolve_disassembly_target(image, request)?;
    let mut instructions =
        damsel_core::decode_aarch64(bytes, start_address, request.max_instructions)?;
    annotate_instructions(image, &mut instructions);

    Ok(DisassemblyResult {
        target,
        start_address,
        bytes_len: bytes.len(),
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
            let limit_bytes = request
                .max_instructions
                .map(|count| count.saturating_mul(4));
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
            let size = request
                .max_instructions
                .map(|count| count.saturating_mul(4))
                .unwrap_or(available)
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
            let size = request
                .max_instructions
                .map(|count| (count.saturating_mul(4)) as u64)
                .unwrap_or(inferred_size)
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
    let import_map =
        image
            .imports
            .iter()
            .fold(BTreeMap::<u64, Vec<&Import>>::new(), |mut acc, import| {
                if let Some(address) = import.address {
                    acc.entry(address).or_default().push(import);
                }
                acc
            });

    for instruction in instructions {
        if let Some(symbols) = symbol_map.get(&instruction.address) {
            for symbol in symbols {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::Symbol(symbol.name.clone()),
                );
            }
        }

        for reference in instruction.references.clone() {
            match reference {
                Reference::Call { target }
                | Reference::Branch { target }
                | Reference::Page { target }
                | Reference::Data { target } => {
                    if let Some(symbols) = symbol_map.get(&target) {
                        for symbol in symbols {
                            push_annotation(
                                &mut instruction.annotations,
                                Annotation::TargetSymbol {
                                    address: target,
                                    name: symbol.name.clone(),
                                },
                            );
                        }
                    }
                    if let Some(imports) = import_map.get(&target) {
                        for import in imports {
                            push_annotation(
                                &mut instruction.annotations,
                                Annotation::Import {
                                    dylib: import.dylib.clone(),
                                    name: import.name.clone(),
                                },
                            );
                        }
                    }
                }
                Reference::Import { .. } => {}
            }
        }

        for relocation in image.relocations.iter().filter(|relocation| {
            relocation.address >= instruction.address
                && relocation.address
                    < instruction
                        .address
                        .saturating_add(u64::from(instruction.size))
        }) {
            push_annotation(
                &mut instruction.annotations,
                Annotation::Note(format!(
                    "reloc {} -> {}",
                    relocation.kind, relocation.target
                )),
            );
        }
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}
