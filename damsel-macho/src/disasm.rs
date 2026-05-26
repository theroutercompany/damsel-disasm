use crate::errors::{MachoError, Result};
use damsel_core::{
    Annotation, BasicBlock, BasicBlockTerminatorKind, BinaryImage, ControlFlowEdge,
    ControlFlowEdgeKind, ControlFlowEdgeKindCounts, DecodedInstruction, DisassemblyAnalysis,
    DisassemblyImportSummary, DisassemblyImportUse, DisassemblyIndirectControlSummary,
    DisassemblyIndirectUse, DisassemblyJumpTableSummary, DisassemblyJumpTableUse, DisassemblyLimit,
    DisassemblyMetrics, DisassemblyObjcReferenceKind, DisassemblyObjcReferenceSummary,
    DisassemblyObjcUse, DisassemblyOverview, DisassemblyRecoveredValueSummary,
    DisassemblyRecoveredValueUse, DisassemblyRequest, DisassemblyRequestV2, DisassemblyResult,
    DisassemblyResultV2, DisassemblyStopReason, DisassemblySummary, DisassemblyTarget,
    DisassemblyTargetSummary, DisassemblyTargetUse, Endianness, Import, ImportBindingKind,
    ImportBindingRecord, ImportBindingSource, IndirectTargetReason, Operand, RecoveredValue,
    RecoveredValueKind, RecoveredValueSource, Reference, Relocation, Section, StubHelperEntry,
    Symbol, TableSlotEncoding,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn disassemble_v2(
    image: &BinaryImage,
    request: &DisassemblyRequestV2,
) -> Result<DisassemblyResultV2> {
    Ok(disassemble_impl(image, request)?.into_v2())
}

pub fn disassemble(image: &BinaryImage, request: &DisassemblyRequest) -> Result<DisassemblyResult> {
    Ok(disassemble_impl(image, &request.to_v2())?.into_legacy())
}

pub fn analyze_disassembly(
    target: impl Into<String>,
    start_address: u64,
    end_address: u64,
    instructions: &[DecodedInstruction],
) -> DisassemblyAnalysis {
    analyze_disassembly_impl(None, target, start_address, end_address, instructions)
}

pub fn analyze_disassembly_with_image(
    image: &BinaryImage,
    target: impl Into<String>,
    start_address: u64,
    end_address: u64,
    instructions: &[DecodedInstruction],
) -> DisassemblyAnalysis {
    analyze_disassembly_impl(
        Some(image),
        target,
        start_address,
        end_address,
        instructions,
    )
}

fn analyze_disassembly_impl(
    image: Option<&BinaryImage>,
    target: impl Into<String>,
    start_address: u64,
    end_address: u64,
    instructions: &[DecodedInstruction],
) -> DisassemblyAnalysis {
    let basic_blocks = build_basic_blocks(instructions);
    let mut edges = Vec::new();
    let mut direct_calls = Vec::new();
    let mut branch_targets = Vec::new();
    let mut data_references = Vec::new();
    let mut indirect_controls = Vec::new();
    let mut imports = Vec::new();
    let mut objc_references = Vec::new();
    let mut recovered_values = Vec::new();
    let mut jump_tables = Vec::new();
    let mut return_count = 0;
    let mut reference_count = 0;
    let mut annotation_count = 0;
    let mut max_instruction_reference_count = 0;
    let mut max_instruction_annotation_count = 0;

    for instruction in instructions {
        reference_count += instruction.references.len();
        annotation_count += instruction.annotations.len();
        max_instruction_reference_count =
            max_instruction_reference_count.max(instruction.references.len());
        max_instruction_annotation_count =
            max_instruction_annotation_count.max(instruction.annotations.len());

        let lower = instruction.mnemonic.to_ascii_lowercase();
        let from_block = block_id_for_address(&basic_blocks, instruction.address).unwrap_or(0);
        let resolved_direct_target =
            instruction
                .references
                .iter()
                .find_map(|reference| match reference {
                    Reference::Call { target } | Reference::Branch { target } => Some(*target),
                    _ => None,
                });

        if is_return_mnemonic(&lower) {
            return_count += 1;
            push_edge(
                &mut edges,
                ControlFlowEdge {
                    from_block,
                    to_block: None,
                    source_address: instruction.address,
                    target_address: None,
                    kind: ControlFlowEdgeKind::Return,
                    via: non_default_return_register(instruction),
                },
            );
        }

        for reference in &instruction.references {
            match reference {
                Reference::Call { target } => {
                    push_target_use(
                        &mut direct_calls,
                        DisassemblyTargetUse {
                            instruction_address: instruction.address,
                            target_address: *target,
                        },
                    );
                    push_edge(
                        &mut edges,
                        ControlFlowEdge {
                            from_block,
                            to_block: block_id_for_address(&basic_blocks, *target),
                            source_address: instruction.address,
                            target_address: Some(*target),
                            kind: ControlFlowEdgeKind::Call,
                            via: None,
                        },
                    );
                }
                Reference::Branch { target } => {
                    let kind = if is_conditional_branch_mnemonic(&lower) {
                        ControlFlowEdgeKind::ConditionalBranch
                    } else {
                        ControlFlowEdgeKind::Branch
                    };
                    push_target_use(
                        &mut branch_targets,
                        DisassemblyTargetUse {
                            instruction_address: instruction.address,
                            target_address: *target,
                        },
                    );
                    push_edge(
                        &mut edges,
                        ControlFlowEdge {
                            from_block,
                            to_block: block_id_for_address(&basic_blocks, *target),
                            source_address: instruction.address,
                            target_address: Some(*target),
                            kind,
                            via: None,
                        },
                    );
                }
                Reference::IndirectCall { via } => {
                    push_indirect_use(
                        &mut indirect_controls,
                        DisassemblyIndirectUse {
                            instruction_address: instruction.address,
                            kind: ControlFlowEdgeKind::IndirectCall,
                            via: via.clone(),
                            resolved_target: resolved_direct_target,
                        },
                    );
                    push_edge(
                        &mut edges,
                        ControlFlowEdge {
                            from_block,
                            to_block: resolved_direct_target
                                .and_then(|target| block_id_for_address(&basic_blocks, target)),
                            source_address: instruction.address,
                            target_address: resolved_direct_target,
                            kind: ControlFlowEdgeKind::IndirectCall,
                            via: Some(via.clone()),
                        },
                    );
                }
                Reference::IndirectBranch { via } => {
                    push_indirect_use(
                        &mut indirect_controls,
                        DisassemblyIndirectUse {
                            instruction_address: instruction.address,
                            kind: ControlFlowEdgeKind::IndirectBranch,
                            via: via.clone(),
                            resolved_target: resolved_direct_target,
                        },
                    );
                    push_edge(
                        &mut edges,
                        ControlFlowEdge {
                            from_block,
                            to_block: resolved_direct_target
                                .and_then(|target| block_id_for_address(&basic_blocks, target)),
                            source_address: instruction.address,
                            target_address: resolved_direct_target,
                            kind: ControlFlowEdgeKind::IndirectBranch,
                            via: Some(via.clone()),
                        },
                    );
                }
                Reference::Data { target } | Reference::Page { target } => {
                    push_target_use(
                        &mut data_references,
                        DisassemblyTargetUse {
                            instruction_address: instruction.address,
                            target_address: *target,
                        },
                    );
                }
                Reference::Import {
                    name,
                    dylib,
                    address,
                } => push_import_use(
                    &mut imports,
                    DisassemblyImportUse {
                        instruction_address: instruction.address,
                        dylib: dylib.clone(),
                        name: name.clone(),
                        address: *address,
                    },
                ),
                Reference::ImportBinding {
                    dylib,
                    name,
                    address,
                    ..
                }
                | Reference::Stub {
                    dylib: Some(dylib),
                    name: Some(name),
                    pointer_address: address,
                    ..
                }
                | Reference::StubHelper {
                    dylib: Some(dylib),
                    name: Some(name),
                    pointer_address: address,
                    ..
                } => push_import_use(
                    &mut imports,
                    DisassemblyImportUse {
                        instruction_address: instruction.address,
                        dylib: dylib.clone(),
                        name: name.clone(),
                        address: *address,
                    },
                ),
                Reference::RelocationEvidence { .. }
                | Reference::Stub { .. }
                | Reference::StubHelper { .. } => {}
            }
        }

        for recovered in &instruction.recovered_values {
            if let Some(objc_use) =
                objc_reference_use_from_recovered(image, instruction.address, recovered)
            {
                push_objc_reference_use(&mut objc_references, objc_use);
            }
            push_recovered_value_use(
                &mut recovered_values,
                DisassemblyRecoveredValueUse {
                    instruction_address: instruction.address,
                    register: recovered.register.clone(),
                    value: recovered.value,
                    kind: recovered.kind,
                    source: recovered.source,
                },
            );
        }

        for annotation in &instruction.annotations {
            if let Some(objc_use) =
                objc_message_send_use_from_annotation(instruction.address, annotation)
            {
                push_objc_reference_use(&mut objc_references, objc_use);
            }
            if let Annotation::JumpTableCandidate {
                base,
                index_register,
                element_size,
            } = annotation
            {
                push_jump_table_use(
                    &mut jump_tables,
                    DisassemblyJumpTableUse {
                        instruction_address: instruction.address,
                        table_base: *base,
                        index_register: index_register.clone(),
                        element_size: *element_size,
                    },
                );
            }
        }
    }

    add_fallthrough_edges(instructions, &basic_blocks, &mut edges);
    let mut incoming_blocks = BTreeSet::new();
    let mut outgoing_blocks = BTreeSet::new();
    let mut intra_window_edge_count = 0;
    for edge in &edges {
        if let Some(to_block) = edge.to_block {
            intra_window_edge_count += 1;
            incoming_blocks.insert(to_block);
            outgoing_blocks.insert(edge.from_block);
        }
    }
    let first_block_id = basic_blocks.first().map(|block| block.id);
    let entry_block_count = basic_blocks
        .iter()
        .filter(|block| Some(block.id) == first_block_id || !incoming_blocks.contains(&block.id))
        .count();
    let primary_entry_reachable_blocks = primary_entry_reachable_blocks(&basic_blocks, &edges);
    let primary_entry_reachable_block_count = primary_entry_reachable_blocks.len();
    let disconnected_block_count = basic_blocks
        .len()
        .saturating_sub(primary_entry_reachable_block_count);
    let exit_block_count = basic_blocks
        .iter()
        .filter(|block| !outgoing_blocks.contains(&block.id))
        .count();
    let back_edge_targets = edges
        .iter()
        .filter(|edge| edge.is_back_edge())
        .filter_map(|edge| edge.to_block)
        .collect::<BTreeSet<_>>();
    let back_edge_count = edges.iter().filter(|edge| edge.is_back_edge()).count();
    let loop_header_count = back_edge_targets.len();
    let edge_kind_counts = ControlFlowEdgeKindCounts::from_edges(&edges);
    let external_target_edge_count = edges
        .iter()
        .filter(|edge| edge.target_address.is_some() && edge.to_block.is_none())
        .count();
    let unresolved_target_edge_count = edges
        .iter()
        .filter(|edge| edge.target_address.is_none() && edge.kind != ControlFlowEdgeKind::Return)
        .count();
    let max_basic_block_instruction_count = basic_blocks
        .iter()
        .map(|block| block.instruction_count)
        .max()
        .unwrap_or(0);
    let decoded_byte_count =
        usize::try_from(end_address.saturating_sub(start_address)).unwrap_or(usize::MAX);

    let indirect_call_count = indirect_controls
        .iter()
        .filter(|entry| entry.kind == ControlFlowEdgeKind::IndirectCall)
        .count();
    let unresolved_indirect_count = indirect_controls
        .iter()
        .filter(|entry| entry.resolved_target.is_none())
        .count();
    let objc_selector_count = objc_references
        .iter()
        .filter(|entry| entry.kind == DisassemblyObjcReferenceKind::Selector)
        .count();
    let objc_class_count = objc_references
        .iter()
        .filter(|entry| entry.kind == DisassemblyObjcReferenceKind::Class)
        .count();
    let objc_method_list_count = objc_references
        .iter()
        .filter(|entry| entry.kind == DisassemblyObjcReferenceKind::MethodList)
        .count();
    let objc_message_send_count = objc_references
        .iter()
        .filter(|entry| entry.kind == DisassemblyObjcReferenceKind::MessageSend)
        .count();
    let overview = build_disassembly_overview(
        &direct_calls,
        &branch_targets,
        &indirect_controls,
        &jump_tables,
        &data_references,
        &imports,
        &objc_references,
        &recovered_values,
    );

    DisassemblyAnalysis {
        target: target.into(),
        start_address,
        end_address,
        instruction_count: instructions.len(),
        metrics: DisassemblyMetrics {
            decoded_byte_count,
            reference_count,
            annotation_count,
            max_instruction_reference_count,
            max_instruction_annotation_count,
            max_basic_block_instruction_count,
            intra_window_edge_count,
            external_target_edge_count,
            unresolved_target_edge_count,
        },
        summary: DisassemblySummary {
            basic_block_count: basic_blocks.len(),
            entry_block_count,
            exit_block_count,
            primary_entry_reachable_block_count,
            disconnected_block_count,
            edge_count: edges.len(),
            back_edge_count,
            loop_header_count,
            edge_kind_counts,
            direct_call_count: direct_calls.len(),
            indirect_call_count,
            branch_count: branch_targets.len()
                + indirect_controls
                    .iter()
                    .filter(|entry| entry.kind == ControlFlowEdgeKind::IndirectBranch)
                    .count(),
            return_count,
            data_reference_count: data_references.len(),
            import_count: imports.len(),
            objc_selector_count,
            objc_class_count,
            objc_method_list_count,
            objc_message_send_count,
            cache_link_count: 0,
            recovered_value_count: recovered_values.len(),
            jump_table_count: jump_tables.len(),
            unresolved_indirect_count,
        },
        basic_blocks,
        edges,
        direct_calls,
        branch_targets,
        data_references,
        indirect_controls,
        imports,
        objc_references,
        cache_links: Vec::new(),
        overview,
        recovered_values,
        jump_tables,
    }
}

fn build_disassembly_overview(
    direct_calls: &[DisassemblyTargetUse],
    branch_targets: &[DisassemblyTargetUse],
    indirect_controls: &[DisassemblyIndirectUse],
    jump_tables: &[DisassemblyJumpTableUse],
    data_references: &[DisassemblyTargetUse],
    imports: &[DisassemblyImportUse],
    objc_references: &[DisassemblyObjcUse],
    recovered_values: &[DisassemblyRecoveredValueUse],
) -> DisassemblyOverview {
    DisassemblyOverview {
        call_targets: summarize_target_uses(direct_calls),
        branch_targets: summarize_target_uses(branch_targets),
        indirect_controls: summarize_indirect_control_uses(indirect_controls),
        jump_tables: summarize_jump_table_uses(jump_tables),
        data_targets: summarize_target_uses(data_references),
        imports: summarize_import_uses(imports),
        cache_links: Vec::new(),
        objc_references: summarize_objc_uses(objc_references),
        recovered_values: summarize_recovered_value_uses(recovered_values),
    }
}

fn summarize_target_uses(entries: &[DisassemblyTargetUse]) -> Vec<DisassemblyTargetSummary> {
    let mut by_target: BTreeMap<u64, DisassemblyTargetSummary> = BTreeMap::new();
    for entry in entries {
        by_target
            .entry(entry.target_address)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyTargetSummary {
                target_address: entry.target_address,
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_target.into_values().collect::<Vec<_>>();
    rows.sort_by_key(|row| (row.first_instruction_address, row.target_address));
    rows
}

fn summarize_indirect_control_uses(
    entries: &[DisassemblyIndirectUse],
) -> Vec<DisassemblyIndirectControlSummary> {
    let mut by_control: BTreeMap<
        (ControlFlowEdgeKind, String, Option<u64>),
        DisassemblyIndirectControlSummary,
    > = BTreeMap::new();
    for entry in entries {
        let key = (entry.kind, entry.via.clone(), entry.resolved_target);
        by_control
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyIndirectControlSummary {
                kind: entry.kind,
                via: entry.via.clone(),
                resolved_target: entry.resolved_target,
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_control.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.kind,
            left.via.as_str(),
            left.resolved_target,
        )
            .cmp(&(
                right.first_instruction_address,
                right.kind,
                right.via.as_str(),
                right.resolved_target,
            ))
    });
    rows
}

fn summarize_jump_table_uses(
    entries: &[DisassemblyJumpTableUse],
) -> Vec<DisassemblyJumpTableSummary> {
    let mut by_table: BTreeMap<(u64, String, u8), DisassemblyJumpTableSummary> = BTreeMap::new();
    for entry in entries {
        let key = (
            entry.table_base,
            entry.index_register.clone(),
            entry.element_size,
        );
        by_table
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyJumpTableSummary {
                table_base: entry.table_base,
                index_register: entry.index_register.clone(),
                element_size: entry.element_size,
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_table.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.table_base,
            left.index_register.as_str(),
            left.element_size,
        )
            .cmp(&(
                right.first_instruction_address,
                right.table_base,
                right.index_register.as_str(),
                right.element_size,
            ))
    });
    rows
}

fn summarize_import_uses(entries: &[DisassemblyImportUse]) -> Vec<DisassemblyImportSummary> {
    let mut by_import: BTreeMap<(String, String, Option<u64>), DisassemblyImportSummary> =
        BTreeMap::new();
    for entry in entries {
        let key = (entry.dylib.clone(), entry.name.clone(), entry.address);
        by_import
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyImportSummary {
                dylib: entry.dylib.clone(),
                name: entry.name.clone(),
                address: entry.address,
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_import.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.dylib.as_str(),
            left.name.as_str(),
            left.address,
        )
            .cmp(&(
                right.first_instruction_address,
                right.dylib.as_str(),
                right.name.as_str(),
                right.address,
            ))
    });
    rows
}

fn summarize_objc_uses(entries: &[DisassemblyObjcUse]) -> Vec<DisassemblyObjcReferenceSummary> {
    let mut by_reference: BTreeMap<
        (
            DisassemblyObjcReferenceKind,
            Option<String>,
            u64,
            Option<u64>,
            Option<String>,
            Option<String>,
        ),
        DisassemblyObjcReferenceSummary,
    > = BTreeMap::new();
    for entry in entries {
        let key = (
            entry.kind,
            entry.name.clone(),
            entry.target_address,
            entry.resolved_address,
            entry.import_dylib.clone(),
            entry.import_symbol.clone(),
        );
        by_reference
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyObjcReferenceSummary {
                kind: entry.kind,
                name: entry.name.clone(),
                target_address: entry.target_address,
                resolved_address: entry.resolved_address,
                import_dylib: entry.import_dylib.clone(),
                import_symbol: entry.import_symbol.clone(),
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_reference.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.kind,
            left.name.as_deref(),
            left.target_address,
            left.resolved_address,
            left.import_dylib.as_deref(),
            left.import_symbol.as_deref(),
        )
            .cmp(&(
                right.first_instruction_address,
                right.kind,
                right.name.as_deref(),
                right.target_address,
                right.resolved_address,
                right.import_dylib.as_deref(),
                right.import_symbol.as_deref(),
            ))
    });
    rows
}

fn summarize_recovered_value_uses(
    entries: &[DisassemblyRecoveredValueUse],
) -> Vec<DisassemblyRecoveredValueSummary> {
    let mut by_value: BTreeMap<
        (String, u64, RecoveredValueKind, RecoveredValueSource),
        DisassemblyRecoveredValueSummary,
    > = BTreeMap::new();
    for entry in entries {
        let key = (
            entry.register.clone(),
            entry.value,
            entry.kind,
            entry.source,
        );
        by_value
            .entry(key)
            .and_modify(|summary| {
                summary.count += 1;
                summary.first_instruction_address = summary
                    .first_instruction_address
                    .min(entry.instruction_address);
            })
            .or_insert(DisassemblyRecoveredValueSummary {
                register: entry.register.clone(),
                value: entry.value,
                kind: entry.kind,
                source: entry.source,
                count: 1,
                first_instruction_address: entry.instruction_address,
            });
    }
    let mut rows = by_value.into_values().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            left.first_instruction_address,
            left.register.as_str(),
            left.value,
            left.kind,
            left.source,
        )
            .cmp(&(
                right.first_instruction_address,
                right.register.as_str(),
                right.value,
                right.kind,
                right.source,
            ))
    });
    rows
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

    if matches!(request.limit, DisassemblyLimit::Bytes(_))
        && window.request_limited
        && instruction_count > 0
        && decoded_bytes < window.bytes.len()
    {
        return DisassemblyStopReason::ByteLimitReached;
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

fn build_basic_blocks(instructions: &[DecodedInstruction]) -> Vec<BasicBlock> {
    if instructions.is_empty() {
        return Vec::new();
    }

    let instruction_addresses = instructions
        .iter()
        .map(|instruction| instruction.address)
        .collect::<BTreeSet<_>>();
    let mut leaders = BTreeSet::from([instructions[0].address]);

    for (index, instruction) in instructions.iter().enumerate() {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        for reference in &instruction.references {
            if let Reference::Branch { target } | Reference::Call { target } = reference
                && instruction_addresses.contains(target)
            {
                leaders.insert(*target);
            }
        }

        if is_cfg_boundary_mnemonic(&lower)
            && let Some(next) = instructions.get(index + 1)
        {
            leaders.insert(next.address);
        }
    }

    let mut blocks = Vec::new();
    let mut current_start = instructions[0].address;
    let mut current_end = instructions[0]
        .address
        .saturating_add(u64::from(instructions[0].size));
    let mut current_terminator_address = instructions[0].address;
    let mut current_terminator_mnemonic = instructions[0].mnemonic.clone();
    let mut current_terminator_kind = terminator_kind(&instructions[0].mnemonic);
    let mut current_count = 0;

    for instruction in instructions {
        if current_count > 0 && leaders.contains(&instruction.address) {
            blocks.push(BasicBlock {
                id: blocks.len(),
                start_address: current_start,
                end_address: current_end,
                instruction_count: current_count,
                terminator_address: current_terminator_address,
                terminator_mnemonic: current_terminator_mnemonic,
                terminator_kind: current_terminator_kind,
            });
            current_start = instruction.address;
            current_count = 0;
        }
        current_end = instruction
            .address
            .saturating_add(u64::from(instruction.size));
        current_terminator_address = instruction.address;
        current_terminator_mnemonic = instruction.mnemonic.clone();
        current_terminator_kind = terminator_kind(&instruction.mnemonic);
        current_count += 1;
    }

    blocks.push(BasicBlock {
        id: blocks.len(),
        start_address: current_start,
        end_address: current_end,
        instruction_count: current_count,
        terminator_address: current_terminator_address,
        terminator_mnemonic: current_terminator_mnemonic,
        terminator_kind: current_terminator_kind,
    });
    blocks
}

fn terminator_kind(mnemonic: &str) -> BasicBlockTerminatorKind {
    let lower = mnemonic.to_ascii_lowercase();
    if is_return_mnemonic(&lower) {
        BasicBlockTerminatorKind::Return
    } else if is_terminal_trap_mnemonic(&lower) {
        BasicBlockTerminatorKind::Trap
    } else if is_conditional_branch_mnemonic(&lower) {
        BasicBlockTerminatorKind::ConditionalBranch
    } else if matches!(lower.as_str(), "br" | "braa" | "braaz" | "brab" | "brabz") {
        BasicBlockTerminatorKind::IndirectBranch
    } else if is_unconditional_transfer_mnemonic(&lower) {
        BasicBlockTerminatorKind::Branch
    } else if matches!(
        lower.as_str(),
        "blr" | "blraa" | "blraaz" | "blrab" | "blrabz"
    ) {
        BasicBlockTerminatorKind::IndirectCall
    } else if is_call_mnemonic(&lower) {
        BasicBlockTerminatorKind::Call
    } else {
        BasicBlockTerminatorKind::Fallthrough
    }
}

fn block_id_for_address(blocks: &[BasicBlock], address: u64) -> Option<usize> {
    let index = blocks.partition_point(|block| block.start_address <= address);
    let block = index.checked_sub(1).and_then(|index| blocks.get(index))?;
    (address < block.end_address).then_some(block.id)
}

fn add_fallthrough_edges(
    instructions: &[DecodedInstruction],
    blocks: &[BasicBlock],
    edges: &mut Vec<ControlFlowEdge>,
) {
    for block in blocks {
        let Some(last) = instructions
            .iter()
            .filter(|instruction| {
                instruction.address >= block.start_address
                    && instruction.address < block.end_address
            })
            .last()
        else {
            continue;
        };
        let lower = last.mnemonic.to_ascii_lowercase();
        if is_return_mnemonic(&lower)
            || is_unconditional_transfer_mnemonic(&lower)
            || is_terminal_trap_mnemonic(&lower)
        {
            continue;
        }
        let next_address = last.address.saturating_add(u64::from(last.size));
        if let Some(target_block) = blocks
            .iter()
            .find(|candidate| candidate.start_address == next_address)
        {
            push_edge(
                edges,
                ControlFlowEdge {
                    from_block: block.id,
                    to_block: Some(target_block.id),
                    source_address: last.address,
                    target_address: Some(target_block.start_address),
                    kind: ControlFlowEdgeKind::Fallthrough,
                    via: None,
                },
            );
        }
    }
}

fn push_edge(target: &mut Vec<ControlFlowEdge>, edge: ControlFlowEdge) {
    if !target.contains(&edge) {
        target.push(edge);
    }
}

fn primary_entry_reachable_blocks(
    blocks: &[BasicBlock],
    edges: &[ControlFlowEdge],
) -> BTreeSet<usize> {
    let Some(first_block) = blocks.first() else {
        return BTreeSet::new();
    };
    let mut successors = BTreeMap::<usize, Vec<usize>>::new();
    for edge in edges {
        if let Some(to_block) = edge.to_block {
            successors
                .entry(edge.from_block)
                .or_default()
                .push(to_block);
        }
    }

    let mut reachable = BTreeSet::new();
    let mut stack = vec![first_block.id];
    while let Some(block_id) = stack.pop() {
        if !reachable.insert(block_id) {
            continue;
        }
        if let Some(next_blocks) = successors.get(&block_id) {
            stack.extend(next_blocks.iter().copied());
        }
    }
    reachable
}

fn push_target_use(target: &mut Vec<DisassemblyTargetUse>, entry: DisassemblyTargetUse) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn push_indirect_use(target: &mut Vec<DisassemblyIndirectUse>, entry: DisassemblyIndirectUse) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn push_import_use(target: &mut Vec<DisassemblyImportUse>, entry: DisassemblyImportUse) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn push_objc_reference_use(target: &mut Vec<DisassemblyObjcUse>, entry: DisassemblyObjcUse) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn push_recovered_value_use(
    target: &mut Vec<DisassemblyRecoveredValueUse>,
    entry: DisassemblyRecoveredValueUse,
) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn push_jump_table_use(target: &mut Vec<DisassemblyJumpTableUse>, entry: DisassemblyJumpTableUse) {
    if !target.contains(&entry) {
        target.push(entry);
    }
}

fn objc_reference_use_from_recovered(
    image: Option<&BinaryImage>,
    instruction_address: u64,
    recovered: &RecoveredValue,
) -> Option<DisassemblyObjcUse> {
    let kind = match recovered.kind {
        RecoveredValueKind::ObjcSelector => DisassemblyObjcReferenceKind::Selector,
        RecoveredValueKind::ObjcClass => DisassemblyObjcReferenceKind::Class,
        RecoveredValueKind::ObjcMethodList => DisassemblyObjcReferenceKind::MethodList,
        _ => image.and_then(|image| {
            if image
                .objc_selector_ref_at_address(recovered.value)
                .is_some()
            {
                Some(DisassemblyObjcReferenceKind::Selector)
            } else if image.objc_class_ref_at_address(recovered.value).is_some()
                || image
                    .objc_imported_class_name_at_address(recovered.value)
                    .is_some()
            {
                Some(DisassemblyObjcReferenceKind::Class)
            } else if image
                .objc_method_list_owner_at_address(recovered.value)
                .is_some()
            {
                Some(DisassemblyObjcReferenceKind::MethodList)
            } else {
                None
            }
        })?,
    };
    let (name, resolved_address, import_dylib, import_symbol) = image
        .and_then(|image| match kind {
            DisassemblyObjcReferenceKind::Selector => image
                .objc_selector_ref_at_address(recovered.value)
                .map(|entry| {
                    (
                        entry.resolved_name.clone(),
                        entry.resolved_address,
                        None,
                        None,
                    )
                }),
            DisassemblyObjcReferenceKind::Class => {
                let class_ref = image.objc_class_ref_at_address(recovered.value);
                let imported_binding =
                    image.objc_imported_class_binding_at_address(recovered.value);
                let name = class_ref
                    .and_then(|entry| entry.resolved_name.clone())
                    .or_else(|| {
                        image
                            .objc_class_name_at_address(recovered.value)
                            .map(str::to_string)
                    })
                    .or_else(|| {
                        image
                            .objc_imported_class_name_at_address(recovered.value)
                            .map(str::to_string)
                    });
                let resolved_address =
                    class_ref
                        .and_then(|entry| entry.resolved_address)
                        .or_else(|| {
                            (class_ref.is_none()
                                && image.objc_class_name_at_address(recovered.value).is_some())
                            .then_some(recovered.value)
                        });
                let import_dylib = imported_binding.map(|binding| binding.dylib.clone());
                let import_symbol = imported_binding.map(|binding| binding.name.clone());
                Some((name, resolved_address, import_dylib, import_symbol))
            }
            DisassemblyObjcReferenceKind::MethodList => image
                .objc_method_list_owner_at_address(recovered.value)
                .map(|name| (Some(name.to_string()), Some(recovered.value), None, None)),
            DisassemblyObjcReferenceKind::MessageSend => None,
        })
        .unwrap_or((None, None, None, None));
    Some(DisassemblyObjcUse {
        instruction_address,
        target_address: recovered.value,
        resolved_address,
        kind,
        name,
        import_dylib,
        import_symbol,
    })
}

fn objc_message_send_use_from_annotation(
    instruction_address: u64,
    annotation: &Annotation,
) -> Option<DisassemblyObjcUse> {
    let Annotation::TargetSymbol { address, name } = annotation else {
        return None;
    };
    let selector = objc_message_send_selector_from_symbol(name)?;
    Some(DisassemblyObjcUse {
        instruction_address,
        target_address: *address,
        resolved_address: Some(*address),
        kind: DisassemblyObjcReferenceKind::MessageSend,
        name: Some(selector.to_string()),
        import_dylib: None,
        import_symbol: None,
    })
}

fn objc_message_send_selector_from_symbol(symbol: &str) -> Option<&str> {
    let (callee, selector) = symbol.split_once('$')?;
    if selector.is_empty() {
        return None;
    }
    matches!(
        callee,
        "_objc_msgSend"
            | "_objc_msgSendSuper"
            | "_objc_msgSendSuper2"
            | "_objc_msgSend_stret"
            | "_objc_msgSendSuper_stret"
            | "_objc_msgSendSuper2_stret"
    )
    .then_some(selector)
}

fn is_cfg_boundary_mnemonic(mnemonic: &str) -> bool {
    is_conditional_branch_mnemonic(mnemonic)
        || is_unconditional_transfer_mnemonic(mnemonic)
        || is_return_mnemonic(mnemonic)
        || is_terminal_trap_mnemonic(mnemonic)
}

fn is_conditional_branch_mnemonic(mnemonic: &str) -> bool {
    mnemonic.starts_with("b.")
        || mnemonic.starts_with("bc.")
        || matches!(mnemonic, "cbz" | "cbnz" | "tbz" | "tbnz")
}

fn is_unconditional_transfer_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "b" | "br" | "braa" | "braaz" | "brab" | "brabz" | "eret"
    )
}

fn is_return_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "ret" | "retaa" | "retab" | "eret" | "eretaa" | "eretab" | "drps"
    )
}

fn is_terminal_trap_mnemonic(mnemonic: &str) -> bool {
    matches!(mnemonic, "brk" | "hlt" | "udf")
}

fn non_default_return_register(instruction: &DecodedInstruction) -> Option<String> {
    let register = instruction
        .operands
        .first()
        .and_then(register_operand_name)?;
    let key = canonical_state_key(&register)?;
    (key != "x30").then_some(register)
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
    let mut jump_table_sources = BTreeMap::<String, JumpTableSource>::new();
    let mut relative_slot_loads = BTreeMap::<String, TableSlotEvidence>::new();
    let mut known_flags: Option<NzcvFlags> = None;
    let join_addresses = basic_block_join_addresses(instructions);

    for instruction in instructions {
        let lower = instruction.mnemonic.to_ascii_lowercase();
        let destination = destination_register(instruction);
        let mut destination_updated = false;
        let mut skip_memory_synthesis = false;
        if join_addresses.contains(&instruction.address) {
            clear_analysis_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                &mut known_flags,
            );
        }
        let flag_update = if instruction_updates_condition_flags(&lower) {
            Some(synthesize_condition_flags(
                instruction,
                &lower,
                &known_values,
                known_flags,
            ))
        } else {
            None
        };

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
                store_known_value_replacing_derived_state(
                    &mut known_values,
                    &mut jump_table_sources,
                    &mut relative_slot_loads,
                    recovered.clone(),
                );
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
            store_known_value_replacing_derived_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                recovered.clone(),
            );
            push_recovered_value(&mut instruction.recovered_values, recovered);
            destination_updated = true;
        }

        if let Some((register, value, source)) =
            synthesize_simple_register_value(instruction, &known_values, known_flags)
        {
            let value = normalize_value_for_write(&register, value);
            let recovered = RecoveredValue {
                register: register.clone(),
                value,
                kind: classify_recovered_value(image, value, RecoveredValueKind::Literal),
                source,
            };
            store_known_value_replacing_derived_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                recovered.clone(),
            );
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
                        store_known_value_replacing_derived_state(
                            &mut known_values,
                            &mut jump_table_sources,
                            &mut relative_slot_loads,
                            recovered.clone(),
                        );
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

        if let Some((recovered, slot_evidence)) =
            synthesize_register_add_value(image, instruction, &known_values, &relative_slot_loads)
        {
            store_known_value_replacing_derived_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                recovered.clone(),
            );
            push_recovered_value(&mut instruction.recovered_values, recovered.clone());
            if include_annotations {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::TableSlotResolved {
                        table_base: slot_evidence.table_base,
                        slot_address: slot_evidence.slot_address,
                        index_register: slot_evidence.index_register.clone(),
                        element_size: slot_evidence.element_size,
                        encoding: slot_evidence.encoding,
                        target: slot_evidence.target,
                    },
                );
            }
            destination_updated = true;
            remove_table_slot_evidence(&mut relative_slot_loads, &recovered.register);
        }

        if let Some((target, recovered)) = synthesize_literal_load(image, instruction, &lower) {
            push_reference_if_missing(&mut instruction.references, Reference::Data { target });
            push_recovered_value(&mut instruction.recovered_values, recovered.clone());
            store_known_value_replacing_derived_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                recovered,
            );
            destination_updated = true;
        }

        if let Some((first_slot, second_slot, first_value, second_value)) =
            synthesize_pair_load_values(image, instruction, &lower, &known_values)
        {
            push_reference_if_missing(
                &mut instruction.references,
                Reference::Data { target: first_slot },
            );
            push_reference_if_missing(
                &mut instruction.references,
                Reference::Data {
                    target: second_slot,
                },
            );
            for recovered in [first_value, second_value] {
                push_recovered_value(&mut instruction.recovered_values, recovered.clone());
                store_known_value_replacing_derived_state(
                    &mut known_values,
                    &mut jump_table_sources,
                    &mut relative_slot_loads,
                    recovered,
                );
            }
            destination_updated = true;
            skip_memory_synthesis = true;
            if include_annotations {
                push_annotation(
                    &mut instruction.annotations,
                    Annotation::Note(format!(
                        "pair load synthesis {first_slot:#x}, {second_slot:#x}"
                    )),
                );
            }
        }

        let memory_inputs = memory_base_index_displacement(instruction);
        if !skip_memory_synthesis
            && let Some((base_register, index_register, displacement, index_transform)) =
                memory_inputs
        {
            if let Some(base) = read_known_value(&known_values, base_register.as_str()) {
                if let Some(base_target) = add_signed(base.value, displacement) {
                    let element_size = jump_table_element_size(instruction);
                    let effective_target = resolve_memory_effective_address(
                        base_target,
                        index_register.as_deref(),
                        &known_values,
                        index_transform,
                    );
                    let reference_target = effective_target.unwrap_or(base_target);
                    let mut jump_table_source_key = None;
                    push_reference_if_missing(
                        &mut instruction.references,
                        Reference::Data {
                            target: reference_target,
                        },
                    );
                    if let Some(second_target) = pair_memory_second_slot_target(
                        instruction,
                        &lower,
                        reference_target,
                        index_register.as_deref(),
                        index_transform,
                    ) {
                        push_reference_if_missing(
                            &mut instruction.references,
                            Reference::Data {
                                target: second_target,
                            },
                        );
                    }
                    if load_reads_destination_value(&lower)
                        && let Some(index_register) = index_register.clone()
                    {
                        if let Some(jump_key) = destination
                            .as_deref()
                            .or(Some(base_register.as_str()))
                            .and_then(canonical_state_key)
                        {
                            jump_table_sources.insert(
                                jump_key.clone(),
                                JumpTableSource {
                                    table_base: base_target,
                                    index_register: index_register.clone(),
                                    element_size,
                                    index_transform,
                                    slot_resolution: if load_reads_relative_table_offset(&lower) {
                                        TableSlotResolution::RelativeOnly
                                    } else {
                                        TableSlotResolution::AbsoluteOnly
                                    },
                                },
                            );
                            jump_table_source_key = Some(jump_key);
                        }
                    }
                    if let Some(dest) = destination.as_ref() {
                        if load_reads_destination_value(&lower) {
                            let resolved_slot = effective_target.and_then(|slot_address| {
                                resolve_table_slot_at_address(
                                    image,
                                    base_target,
                                    slot_address,
                                    element_size,
                                    if load_reads_relative_table_offset(&lower) {
                                        TableSlotResolution::RelativeOnly
                                    } else {
                                        TableSlotResolution::AbsoluteOnly
                                    },
                                )
                            });
                            if load_reads_relative_table_offset(&lower) {
                                if let Some(slot) = resolved_slot
                                    .filter(|slot| slot.encoding == TableSlotEncoding::Relative32)
                                {
                                    if let Some(key) = canonical_state_key(dest) {
                                        remove_known_value(&mut known_values, dest);
                                        remove_jump_table_source(&mut jump_table_sources, dest);
                                        relative_slot_loads.insert(
                                            key,
                                            TableSlotEvidence {
                                                table_base: base_target,
                                                slot_address: slot.slot_address,
                                                index_register: index_register
                                                    .clone()
                                                    .unwrap_or_default(),
                                                element_size,
                                                encoding: slot.encoding,
                                                target: slot.target,
                                            },
                                        );
                                    }
                                    if include_annotations {
                                        push_annotation(
                                            &mut instruction.annotations,
                                            Annotation::TableSlotResolved {
                                                table_base: base_target,
                                                slot_address: slot.slot_address,
                                                index_register: index_register
                                                    .clone()
                                                    .unwrap_or_default(),
                                                element_size,
                                                encoding: slot.encoding,
                                                target: slot.target,
                                            },
                                        );
                                    }
                                    destination_updated = true;
                                } else if let Some(value) = effective_target
                                    && let Some(recovered) = synthesize_scalar_memory_load_value(
                                        image, &lower, dest, value,
                                    )
                                {
                                    if canonical_state_key(dest) == jump_table_source_key {
                                        remove_table_slot_evidence(&mut relative_slot_loads, dest);
                                        store_known_value(&mut known_values, recovered.clone());
                                    } else {
                                        store_known_value_replacing_derived_state(
                                            &mut known_values,
                                            &mut jump_table_sources,
                                            &mut relative_slot_loads,
                                            recovered.clone(),
                                        );
                                    }
                                    push_recovered_value(
                                        &mut instruction.recovered_values,
                                        recovered,
                                    );
                                    destination_updated = true;
                                }
                            } else {
                                if let Some(slot) = resolved_slot {
                                    let value = slot.target;
                                    let source = match slot.encoding {
                                        TableSlotEncoding::Absolute32
                                        | TableSlotEncoding::Absolute64 => {
                                            RecoveredValueSource::TableLoad
                                        }
                                        TableSlotEncoding::Relative32 => {
                                            RecoveredValueSource::RelativeTableLoad
                                        }
                                    };
                                    let recovered = RecoveredValue {
                                        register: dest.clone(),
                                        value: normalize_value_for_write(dest, value),
                                        kind: slot.kind,
                                        source,
                                    };
                                    if canonical_state_key(dest) == jump_table_source_key {
                                        remove_table_slot_evidence(&mut relative_slot_loads, dest);
                                        store_known_value(&mut known_values, recovered.clone());
                                    } else {
                                        store_known_value_replacing_derived_state(
                                            &mut known_values,
                                            &mut jump_table_sources,
                                            &mut relative_slot_loads,
                                            recovered.clone(),
                                        );
                                    }
                                    push_recovered_value(
                                        &mut instruction.recovered_values,
                                        recovered,
                                    );
                                    destination_updated = true;
                                    if include_annotations {
                                        push_annotation(
                                            &mut instruction.annotations,
                                            Annotation::TableSlotResolved {
                                                table_base: base_target,
                                                slot_address: slot.slot_address,
                                                index_register: index_register
                                                    .clone()
                                                    .unwrap_or_default(),
                                                element_size,
                                                encoding: slot.encoding,
                                                target: slot.target,
                                            },
                                        );
                                    }
                                } else {
                                    if let Some(value) = effective_target
                                        && let Some(recovered) = synthesize_scalar_memory_load_value(
                                            image, &lower, dest, value,
                                        )
                                    {
                                        if canonical_state_key(dest) == jump_table_source_key {
                                            remove_table_slot_evidence(
                                                &mut relative_slot_loads,
                                                dest,
                                            );
                                            store_known_value(&mut known_values, recovered.clone());
                                        } else {
                                            store_known_value_replacing_derived_state(
                                                &mut known_values,
                                                &mut jump_table_sources,
                                                &mut relative_slot_loads,
                                                recovered.clone(),
                                            );
                                        }
                                        push_recovered_value(
                                            &mut instruction.recovered_values,
                                            recovered,
                                        );
                                        destination_updated = true;
                                    } else if let Some(value) = effective_target
                                        && dest.to_ascii_lowercase().starts_with('x')
                                        && let Some(kind) =
                                            unresolved_load_reference_kind(image, value)
                                    {
                                        push_recovered_value(
                                            &mut instruction.recovered_values,
                                            RecoveredValue {
                                                register: dest.clone(),
                                                value,
                                                kind,
                                                source: RecoveredValueSource::AdrpLoad,
                                            },
                                        );
                                    }
                                    if jump_table_source_key.is_some() {
                                        remove_known_value(&mut known_values, dest);
                                        remove_table_slot_evidence(&mut relative_slot_loads, dest);
                                        destination_updated = true;
                                    }
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
            let mut resolved_target =
                read_known_value(&known_values, register.as_str()).and_then(|value| {
                    let reason = indirect_target_reason(image, value.value);
                    should_resolve_indirect_target(image, value.value, reason)
                        .then_some((value.value, reason))
                });

            if let Some(jump_table_source) = jump_table_source.clone() {
                if let Some(index_value) =
                    read_known_value(&known_values, jump_table_source.index_register.as_str())
                        .map(|value| value.value)
                {
                    if let Some((slot_address, target, resolved_slot)) = resolve_table_slot_target(
                        image,
                        jump_table_source.table_base,
                        index_value,
                        jump_table_source.element_size,
                        jump_table_source.index_transform,
                        jump_table_source.slot_resolution,
                    ) {
                        push_reference_if_missing(
                            &mut instruction.references,
                            Reference::Data {
                                target: slot_address,
                            },
                        );
                        resolved_target = Some((target, resolved_slot.reason));
                        if include_annotations {
                            push_annotation(
                                &mut instruction.annotations,
                                Annotation::TableSlotResolved {
                                    table_base: jump_table_source.table_base,
                                    slot_address,
                                    index_register: jump_table_source.index_register.clone(),
                                    element_size: jump_table_source.element_size,
                                    encoding: resolved_slot.encoding,
                                    target,
                                },
                            );
                        }
                    }
                }
                if include_annotations {
                    push_annotation(
                        &mut instruction.annotations,
                        Annotation::JumpTableCandidate {
                            base: jump_table_source.table_base,
                            index_register: jump_table_source.index_register,
                            element_size: jump_table_source.element_size,
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

        if !destination_updated {
            for destination in written_destination_registers(instruction, &lower) {
                remove_register_analysis_state(
                    &mut known_values,
                    &mut jump_table_sources,
                    &mut relative_slot_loads,
                    &destination,
                );
            }
        }
        for base in memory_writeback_base_registers(instruction) {
            remove_register_analysis_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
                &base,
            );
        }
        if let Some(flags) = flag_update {
            known_flags = flags;
        }

        if is_call_mnemonic(&lower) {
            clear_call_clobbered_state(
                &mut known_values,
                &mut jump_table_sources,
                &mut relative_slot_loads,
            );
            known_flags = None;
        } else if is_hard_control_flow_boundary(&lower) {
            known_values.clear();
            jump_table_sources.clear();
            relative_slot_loads.clear();
            known_flags = None;
        }
    }
}

fn basic_block_join_addresses(instructions: &[DecodedInstruction]) -> BTreeSet<u64> {
    let blocks = build_basic_blocks(instructions);
    if blocks.len() < 2 {
        return BTreeSet::new();
    }

    let mut edges = Vec::new();
    let instruction_addresses = instructions
        .iter()
        .map(|instruction| instruction.address)
        .collect::<BTreeSet<_>>();
    for instruction in instructions {
        let from_block = block_id_for_address(&blocks, instruction.address).unwrap_or(0);
        for reference in &instruction.references {
            if let Reference::Branch { target } | Reference::Call { target } = reference
                && instruction_addresses.contains(target)
            {
                let kind = match reference {
                    Reference::Call { .. } => ControlFlowEdgeKind::Call,
                    _ => ControlFlowEdgeKind::Branch,
                };
                push_edge(
                    &mut edges,
                    ControlFlowEdge {
                        from_block,
                        to_block: block_id_for_address(&blocks, *target),
                        source_address: instruction.address,
                        target_address: Some(*target),
                        kind,
                        via: None,
                    },
                );
            }
        }
    }
    add_fallthrough_edges(instructions, &blocks, &mut edges);

    let mut incoming_counts = BTreeMap::<u64, usize>::new();
    for edge in edges {
        if let Some(target_address) = edge.target_address {
            *incoming_counts.entry(target_address).or_default() += 1;
        }
    }

    blocks
        .iter()
        .filter(|block| {
            incoming_counts
                .get(&block.start_address)
                .copied()
                .unwrap_or(0)
                > 1
        })
        .map(|block| block.start_address)
        .collect()
}

fn clear_analysis_state(
    known_values: &mut BTreeMap<String, RecoveredValue>,
    jump_table_sources: &mut BTreeMap<String, JumpTableSource>,
    relative_slot_loads: &mut BTreeMap<String, TableSlotEvidence>,
    known_flags: &mut Option<NzcvFlags>,
) {
    known_values.clear();
    jump_table_sources.clear();
    relative_slot_loads.clear();
    *known_flags = None;
}

fn clear_call_clobbered_state(
    known_values: &mut BTreeMap<String, RecoveredValue>,
    jump_table_sources: &mut BTreeMap<String, JumpTableSource>,
    relative_slot_loads: &mut BTreeMap<String, TableSlotEvidence>,
) {
    for index in 0..=18 {
        let key = format!("x{index}");
        known_values.remove(&key);
        jump_table_sources.remove(&key);
        relative_slot_loads.remove(&key);
    }
    known_values.remove("x30");
    jump_table_sources.remove("x30");
    relative_slot_loads.remove("x30");
    known_values.remove("lr");
    jump_table_sources.remove("lr");
    relative_slot_loads.remove("lr");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NzcvFlags {
    n: bool,
    z: bool,
    c: bool,
    v: bool,
}

fn canonical_state_key(register: &str) -> Option<String> {
    let lower = register.to_ascii_lowercase();
    if matches!(lower.as_str(), "xzr" | "wzr" | "sp" | "wsp") {
        return None;
    }
    if lower == "fp" {
        return Some("x29".to_string());
    }
    if lower == "lr" {
        return Some("x30".to_string());
    }
    if lower == "ip0" {
        return Some("x16".to_string());
    }
    if lower == "ip1" {
        return Some("x17".to_string());
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

fn normalize_value_for_width(value: u64, width: u32) -> u64 {
    if width == 32 {
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

fn store_known_value_replacing_derived_state(
    known_values: &mut BTreeMap<String, RecoveredValue>,
    jump_table_sources: &mut BTreeMap<String, JumpTableSource>,
    relative_slot_loads: &mut BTreeMap<String, TableSlotEvidence>,
    recovered: RecoveredValue,
) {
    remove_jump_table_source(jump_table_sources, &recovered.register);
    remove_table_slot_evidence(relative_slot_loads, &recovered.register);
    store_known_value(known_values, recovered);
}

fn remove_register_analysis_state(
    known_values: &mut BTreeMap<String, RecoveredValue>,
    jump_table_sources: &mut BTreeMap<String, JumpTableSource>,
    relative_slot_loads: &mut BTreeMap<String, TableSlotEvidence>,
    register: &str,
) {
    remove_known_value(known_values, register);
    remove_jump_table_source(jump_table_sources, register);
    remove_table_slot_evidence(relative_slot_loads, register);
}

fn remove_known_value(known_values: &mut BTreeMap<String, RecoveredValue>, register: &str) {
    if let Some(key) = canonical_state_key(register) {
        known_values.remove(&key);
    }
}

fn remove_jump_table_source(
    jump_table_sources: &mut BTreeMap<String, JumpTableSource>,
    register: &str,
) {
    if let Some(key) = canonical_state_key(register) {
        jump_table_sources.remove(&key);
    }
}

fn remove_table_slot_evidence(
    table_slot_evidence: &mut BTreeMap<String, TableSlotEvidence>,
    register: &str,
) {
    if let Some(key) = canonical_state_key(register) {
        table_slot_evidence.remove(&key);
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
    if canonical_state_key(&value.register).is_none() {
        return;
    }

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

fn written_destination_registers(instruction: &DecodedInstruction, mnemonic: &str) -> Vec<String> {
    if !instruction_writes_destination(mnemonic) {
        return Vec::new();
    }

    if atomic_read_modify_write_writes_second_operand(mnemonic) {
        return instruction
            .operands
            .iter()
            .skip(1)
            .take(1)
            .filter_map(register_operand_name)
            .collect();
    }

    if atomic_pair_compare_swap_writes_first_two_operands(mnemonic) {
        return instruction
            .operands
            .iter()
            .take(2)
            .filter_map(register_operand_name)
            .collect();
    }

    if load_pair_writes_two_destinations(mnemonic) {
        return instruction
            .operands
            .iter()
            .take(2)
            .filter_map(register_operand_name)
            .collect();
    }

    destination_register(instruction).into_iter().collect()
}

fn register_operand_name(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Register(register) => Some(register.clone()),
        Operand::QualifiedRegister { register, .. } => Some(register.clone()),
        Operand::SystemRegister(register) => Some(register.clone()),
        _ => None,
    }
}

fn atomic_read_modify_write_writes_second_operand(mnemonic: &str) -> bool {
    [
        "swp", "ldadd", "ldclr", "ldeor", "ldset", "ldsmax", "ldsmin", "ldumax", "ldumin",
    ]
    .iter()
    .any(|prefix| mnemonic.starts_with(prefix))
}

fn atomic_pair_compare_swap_writes_first_two_operands(mnemonic: &str) -> bool {
    matches!(mnemonic, "casp" | "caspa" | "caspl" | "caspal")
}

fn load_pair_writes_two_destinations(mnemonic: &str) -> bool {
    mnemonic.starts_with("ldp")
        || mnemonic.starts_with("ldnp")
        || matches!(mnemonic, "ldxp" | "ldaxp")
}

fn store_pair_writes_two_memory_slots(mnemonic: &str) -> bool {
    mnemonic.starts_with("stp")
        || mnemonic.starts_with("stnp")
        || matches!(mnemonic, "stxp" | "stlxp")
}

fn pair_memory_second_slot_target(
    instruction: &DecodedInstruction,
    mnemonic: &str,
    first_slot: u64,
    index_register: Option<&str>,
    index_transform: MemoryIndexTransform,
) -> Option<u64> {
    if index_register.is_some() || index_transform != MemoryIndexTransform::unshifted_unsigned64() {
        return None;
    }
    let width = pair_memory_access_width(instruction, mnemonic)?;
    first_slot.checked_add(u64::try_from(width).ok()?)
}

fn pair_memory_access_width(instruction: &DecodedInstruction, mnemonic: &str) -> Option<usize> {
    if atomic_pair_compare_swap_writes_first_two_operands(mnemonic) {
        let first_register = register_operand_name(instruction.operands.first()?)?;
        let second_register = register_operand_name(instruction.operands.get(1)?)?;
        let first_width = pair_store_width(&first_register)?;
        let second_width = pair_store_width(&second_register)?;
        if first_width == second_width {
            return Some(first_width);
        }
        return None;
    }

    if load_pair_writes_two_destinations(mnemonic) {
        let first_register = register_operand_name(instruction.operands.first()?)?;
        let second_register = register_operand_name(instruction.operands.get(1)?)?;
        let (first_width, first_signed) = pair_load_width(&first_register, mnemonic)?;
        let (second_width, second_signed) = pair_load_width(&second_register, mnemonic)?;
        if first_width == second_width && first_signed == second_signed {
            return Some(first_width);
        }
        return None;
    }

    if store_pair_writes_two_memory_slots(mnemonic) {
        let first_index = if matches!(mnemonic, "stxp" | "stlxp") {
            1
        } else {
            0
        };
        let first_register = register_operand_name(instruction.operands.get(first_index)?)?;
        let second_register = register_operand_name(instruction.operands.get(first_index + 1)?)?;
        let first_width = pair_store_width(&first_register)?;
        let second_width = pair_store_width(&second_register)?;
        if first_width == second_width {
            return Some(first_width);
        }
    }

    None
}

fn pair_store_width(register: &str) -> Option<usize> {
    match register_value_width_for_write(register)? {
        32 => Some(4),
        64 => Some(8),
        _ => None,
    }
}

fn memory_writeback_base_registers(instruction: &DecodedInstruction) -> Vec<String> {
    instruction
        .operands
        .iter()
        .filter_map(|operand| match operand {
            Operand::Memory {
                base,
                writeback: Some(_),
                ..
            } => Some(base.clone()),
            _ => None,
        })
        .collect()
}

fn add_immediate_inputs(instruction: &DecodedInstruction) -> Option<(Option<String>, String, i64)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if matches!(lower.as_str(), "add" | "adds" | "sub" | "subs") {
        if let [destination, Operand::Register(base), immediate, ..] =
            instruction.operands.as_slice()
        {
            let optional_modifier = instruction.operands.get(3);
            let displacement = match immediate {
                Operand::ImmediateUnsigned(value) => {
                    i64::try_from(shifted_add_sub_immediate_value(*value, optional_modifier)?)
                        .ok()?
                }
                Operand::ImmediateSigned(value) if *value >= 0 => {
                    let value = u64::try_from(*value).ok()?;
                    i64::try_from(shifted_add_sub_immediate_value(value, optional_modifier)?)
                        .ok()?
                }
                Operand::Label(value) => {
                    i64::try_from(shifted_add_sub_immediate_value(*value, optional_modifier)?)
                        .ok()?
                }
                _ => return None,
            };
            let displacement = if lower.starts_with("sub") {
                displacement.checked_neg()?
            } else {
                displacement
            };
            let destination = match destination {
                Operand::Register(register) if is_zero_register(register) => return None,
                Operand::Register(register) => Some(register.clone()),
                _ => None,
            };
            return Some((destination, base.clone(), displacement));
        }
    }

    None
}

fn synthesize_pair_load_values(
    image: &BinaryImage,
    instruction: &DecodedInstruction,
    lower: &str,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(u64, u64, RecoveredValue, RecoveredValue)> {
    if !(lower.starts_with("ldp") || lower.starts_with("ldnp") || matches!(lower, "ldxp" | "ldaxp"))
    {
        return None;
    }

    let first_register = register_operand_name(instruction.operands.first()?)?;
    let second_register = register_operand_name(instruction.operands.get(1)?)?;
    let (first_width, first_signed) = pair_load_width(&first_register, lower)?;
    let (second_width, second_signed) = pair_load_width(&second_register, lower)?;
    if first_width != second_width || first_signed != second_signed {
        return None;
    }

    let (base_register, index_register, displacement, index_transform) =
        memory_base_index_displacement(instruction)?;
    if index_register.is_some() || index_transform != MemoryIndexTransform::unshifted_unsigned64() {
        return None;
    }
    let base = read_known_value(known_values, &base_register)?.value;
    let first_slot = add_signed(base, displacement)?;
    let second_slot = first_slot.checked_add(u64::try_from(first_width).ok()?)?;
    recoverable_memory_load_range(image, first_slot, first_width)?;
    recoverable_memory_load_range(image, second_slot, second_width)?;
    let first_value = read_memory_value(image, first_slot, first_width, first_signed)?;
    let second_value = read_memory_value(image, second_slot, second_width, second_signed)?;

    Some((
        first_slot,
        second_slot,
        RecoveredValue {
            register: first_register,
            value: first_value,
            kind: classify_recovered_value(image, first_value, RecoveredValueKind::Literal),
            source: RecoveredValueSource::AdrpLoad,
        },
        RecoveredValue {
            register: second_register,
            value: second_value,
            kind: classify_recovered_value(image, second_value, RecoveredValueKind::Literal),
            source: RecoveredValueSource::AdrpLoad,
        },
    ))
}

fn pair_load_width(register: &str, mnemonic: &str) -> Option<(usize, bool)> {
    let register = register.to_ascii_lowercase();
    if mnemonic.starts_with("ldpsw") && register.starts_with('x') {
        Some((4, true))
    } else if register.starts_with('w') {
        Some((4, false))
    } else if register.starts_with('x') {
        Some((8, false))
    } else {
        None
    }
}

fn synthesize_register_add_value(
    image: &BinaryImage,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    relative_slot_loads: &BTreeMap<String, TableSlotEvidence>,
) -> Option<(RecoveredValue, TableSlotEvidence)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if lower != "add" {
        return None;
    }

    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let base_register = match instruction.operands.get(1)? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let source_register = relative_slot_add_source_register(instruction.operands.get(2)?)?;

    let source_key = canonical_state_key(&source_register)?;
    let slot_evidence = relative_slot_loads.get(&source_key)?.clone();
    let base_value = read_known_value(known_values, base_register.as_str())?.value;
    if base_value != slot_evidence.table_base {
        return None;
    }

    let recovered = RecoveredValue {
        register: destination,
        value: slot_evidence.target,
        kind: classify_recovered_value(image, slot_evidence.target, RecoveredValueKind::Address),
        source: RecoveredValueSource::RelativeTableLoad,
    };
    Some((recovered, slot_evidence))
}

fn relative_slot_add_source_register(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Register(register) => Some(register.clone()),
        Operand::ShiftedRegister { register, shift } => {
            let lower = shift.to_ascii_lowercase();
            let modifier = shift_modifier_op(&lower)?;
            if !matches!(modifier, "lsl" | "sxtw" | "sxtx") {
                return None;
            }
            (parse_optional_shift_from_text(&lower)? == 0).then(|| register.clone())
        }
        _ => None,
    }
}

fn memory_base_index_displacement(
    instruction: &DecodedInstruction,
) -> Option<(String, Option<String>, i64, MemoryIndexTransform)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if !(lower.starts_with("ld")
        || lower.starts_with("st")
        || is_atomic_compare_swap_mnemonic(&lower)
        || is_prefetch_mnemonic(&lower))
    {
        return None;
    }

    for operand in &instruction.operands {
        if let Operand::Memory {
            base,
            index,
            index_modifier,
            displacement,
            ..
        } = operand
        {
            let index_transform = index_modifier
                .as_deref()
                .map(|modifier| {
                    memory_index_modifier_transform(modifier, jump_table_element_size(instruction))
                })
                .unwrap_or(Some(MemoryIndexTransform::unshifted_unsigned64()))?;
            return Some((base.clone(), index.clone(), *displacement, index_transform));
        }
    }
    None
}

fn is_prefetch_mnemonic(mnemonic: &str) -> bool {
    matches!(mnemonic, "prfm" | "prfum")
}

fn is_atomic_compare_swap_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "cas"
            | "casa"
            | "casl"
            | "casal"
            | "casb"
            | "casab"
            | "caslb"
            | "casalb"
            | "cash"
            | "casah"
            | "caslh"
            | "casalh"
            | "casp"
            | "caspa"
            | "caspl"
            | "caspal"
    )
}

fn load_reads_destination_value(mnemonic: &str) -> bool {
    mnemonic.starts_with("ldr")
        || mnemonic.starts_with("ldur")
        || mnemonic.starts_with("ldtr")
        || mnemonic.starts_with("ldar")
        || mnemonic.starts_with("ldapr")
        || mnemonic.starts_with("ldapur")
        || mnemonic.starts_with("ldaxr")
        || mnemonic.starts_with("ldxr")
}

fn load_reads_relative_table_offset(mnemonic: &str) -> bool {
    mnemonic.starts_with("ldrsw")
        || mnemonic.starts_with("ldursw")
        || mnemonic.starts_with("ldtrsw")
        || mnemonic.starts_with("ldapursw")
}

fn synthesize_scalar_memory_load_value(
    image: &BinaryImage,
    lower: &str,
    register: &str,
    address: u64,
) -> Option<RecoveredValue> {
    let (width, signed) = scalar_memory_load_width(register, lower)?;
    recoverable_memory_load_range(image, address, width)?;
    let value = read_memory_value(image, address, width, signed)?;
    Some(RecoveredValue {
        register: register.to_string(),
        value: normalize_value_for_write(register, value),
        kind: classify_recovered_value(image, value, RecoveredValueKind::Literal),
        source: RecoveredValueSource::AdrpLoad,
    })
}

fn recoverable_memory_load_range(image: &BinaryImage, address: u64, width: usize) -> Option<()> {
    let (section, _) = image.bytes_for_virtual_range(address, width)?;
    if section.executable {
        return None;
    }
    if unresolved_load_reference_kind(image, address).is_some() {
        return None;
    }
    Some(())
}

fn scalar_memory_load_width(register: &str, mnemonic: &str) -> Option<(usize, bool)> {
    let register = register.to_ascii_lowercase();
    if load_reads_relative_table_offset(mnemonic) && register.starts_with('x') {
        Some((4, true))
    } else if mnemonic.starts_with("ldrsb")
        || mnemonic.starts_with("ldursb")
        || mnemonic.starts_with("ldtrsb")
        || mnemonic.starts_with("ldapursb")
    {
        Some((1, true))
    } else if mnemonic.starts_with("ldrb")
        || mnemonic.starts_with("ldurb")
        || mnemonic.starts_with("ldtrb")
        || mnemonic.starts_with("ldarb")
        || mnemonic.starts_with("ldaprb")
        || mnemonic.starts_with("ldapurb")
        || mnemonic.starts_with("ldaxrb")
        || mnemonic.starts_with("ldxrb")
    {
        Some((1, false))
    } else if mnemonic.starts_with("ldrsh")
        || mnemonic.starts_with("ldursh")
        || mnemonic.starts_with("ldtrsh")
        || mnemonic.starts_with("ldapursh")
    {
        Some((2, true))
    } else if mnemonic.starts_with("ldrh")
        || mnemonic.starts_with("ldurh")
        || mnemonic.starts_with("ldtrh")
        || mnemonic.starts_with("ldarh")
        || mnemonic.starts_with("ldaprh")
        || mnemonic.starts_with("ldapurh")
        || mnemonic.starts_with("ldaxrh")
        || mnemonic.starts_with("ldxrh")
    {
        Some((2, false))
    } else if matches!(
        mnemonic,
        "ldr" | "ldur" | "ldtr" | "ldar" | "ldapr" | "ldapur" | "ldaxr" | "ldxr"
    ) {
        if register.starts_with('w') {
            Some((4, false))
        } else if register.starts_with('x') {
            Some((8, false))
        } else {
            None
        }
    } else {
        None
    }
}

fn memory_index_modifier_transform(
    modifier: &str,
    element_size: u8,
) -> Option<MemoryIndexTransform> {
    let lower = modifier.to_ascii_lowercase();
    let extension = match shift_modifier_op(&lower)? {
        "lsl" | "uxtx" => MemoryIndexExtension::Unsigned64,
        "uxtw" => MemoryIndexExtension::Unsigned32,
        "sxtw" => MemoryIndexExtension::Signed32,
        "sxtx" => MemoryIndexExtension::Signed64,
        _ => return None,
    };
    let Some(shift) = parse_optional_shift_from_text(&lower) else {
        return None;
    };
    if shift != 0 && shift != memory_index_scaled_shift(element_size)? {
        return None;
    }
    Some(MemoryIndexTransform { extension, shift })
}

fn memory_index_scaled_shift(element_size: u8) -> Option<u32> {
    match element_size {
        1 | 2 | 4 | 8 => Some(element_size.trailing_zeros()),
        _ => None,
    }
}

fn parse_optional_shift_from_text(text: &str) -> Option<u32> {
    parse_shift_text(text, Some(0))
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
    is_cfg_boundary_mnemonic(mnemonic)
}

fn instruction_writes_destination(mnemonic: &str) -> bool {
    if store_exclusive_writes_status(mnemonic) {
        return true;
    }

    !(mnemonic == "cmp"
        || mnemonic == "cmn"
        || mnemonic == "ccmp"
        || mnemonic == "ccmn"
        || mnemonic == "tst"
        || is_conditional_branch_mnemonic(mnemonic)
        || is_terminal_trap_mnemonic(mnemonic)
        || mnemonic.starts_with("st")
        || is_unconditional_transfer_mnemonic(mnemonic)
        || is_call_mnemonic(mnemonic)
        || is_return_mnemonic(mnemonic))
}

fn store_exclusive_writes_status(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "stxr" | "stxrb" | "stxrh" | "stlxr" | "stlxrb" | "stlxrh" | "stxp" | "stlxp"
    )
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
    let shift = match instruction.operands.get(2) {
        Some(operand) => parse_mov_wide_shift_bits(&destination, operand)?,
        None => 0,
    };
    let lane_mask = 0xffff_u64.checked_shl(shift)?;
    let shifted_imm = imm.checked_shl(shift)? & lane_mask;

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
    known_flags: Option<NzcvFlags>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    match lower.as_str() {
        "mov" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register.clone(),
                _ => return None,
            };
            match instruction.operands.get(1)? {
                Operand::Register(register) => Some((
                    destination,
                    read_simple_register_value(known_values, register)?,
                    RecoveredValueSource::Other,
                )),
                Operand::ImmediateUnsigned(value) => {
                    Some((destination, *value, RecoveredValueSource::MoveWide))
                }
                Operand::ImmediateSigned(value) => {
                    Some((destination, *value as u64, RecoveredValueSource::MoveWide))
                }
                _ => None,
            }
        }
        "and" | "ands" | "orr" | "eor" | "bic" | "bics" | "orn" | "eon" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register.clone(),
                _ => return None,
            };
            let source = match instruction.operands.get(1)? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let source_value = read_simple_register_value(known_values, source)?;
            let destination_width = register_value_width_for_write(&destination)?;
            let rhs_value = logical_operand_value(
                instruction.operands.get(2)?,
                destination_width,
                known_values,
            )?;
            let value = match lower.as_str() {
                "and" | "ands" => source_value & rhs_value,
                "orr" => source_value | rhs_value,
                "eor" => source_value ^ rhs_value,
                "bic" | "bics" => source_value & !rhs_value,
                "orn" => source_value | !rhs_value,
                "eon" => source_value ^ !rhs_value,
                _ => return None,
            };
            Some((destination, value, RecoveredValueSource::Other))
        }
        "add" | "adds" | "sub" | "subs" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register.clone(),
                _ => return None,
            };
            let base = match instruction.operands.get(1)? {
                Operand::Register(register) => read_simple_register_value(known_values, register)?,
                _ => return None,
            };
            let rhs = arithmetic_register_operand_value(
                instruction.operands.get(2)?,
                &destination,
                known_values,
            )?;
            let value = match lower.as_str() {
                "add" | "adds" => add_arithmetic_operand(base, rhs)?,
                "sub" | "subs" => subtract_arithmetic_operand(base, rhs)?,
                _ => return None,
            };
            Some((destination, value, RecoveredValueSource::Other))
        }
        "adc" | "adcs" | "sbc" | "sbcs" | "ngc" | "ngcs" => {
            synthesize_carry_register_value(&lower, instruction, known_values, known_flags)
        }
        "mul" | "madd" | "msub" | "mneg" => {
            synthesize_multiply_register_value(&lower, instruction, known_values)
        }
        "umaddl" | "smaddl" | "umsubl" | "smsubl" | "umull" | "smull" | "umnegl" | "smnegl" => {
            synthesize_long_multiply_register_value(&lower, instruction, known_values)
        }
        "umulh" | "smulh" => {
            synthesize_high_multiply_register_value(&lower, instruction, known_values)
        }
        "udiv" | "sdiv" => synthesize_divide_register_value(&lower, instruction, known_values),
        "clz" | "cls" => synthesize_count_register_value(&lower, instruction, known_values),
        "ubfx" | "sbfx" => synthesize_bitfield_extract_value(&lower, instruction, known_values),
        "ubfiz" | "sbfiz" | "bfi" | "bfxil" | "bfc" => {
            synthesize_bitfield_insert_value(&lower, instruction, known_values)
        }
        "extr" => synthesize_extract_register_value(instruction, known_values),
        "rbit" | "rev" | "rev16" | "rev32" => {
            synthesize_reverse_register_value(&lower, instruction, known_values)
        }
        "mvn" | "neg" | "negs" | "uxtb" | "uxth" | "uxtw" | "sxtb" | "sxth" | "sxtw" => {
            synthesize_unary_register_value(&lower, instruction, known_values)
        }
        "csel" | "csinc" | "csinv" | "csneg" => {
            synthesize_conditional_select_value(&lower, instruction, known_values, known_flags)
        }
        "cset" | "csetm" | "cinc" | "cinv" | "cneg" => synthesize_conditional_select_alias_value(
            &lower,
            instruction,
            known_values,
            known_flags,
        ),
        "lsl" | "lsr" | "asr" | "ror" | "lslv" | "lsrv" | "asrv" | "rorv" => {
            synthesize_shift_register_value(&lower, instruction, known_values)
        }
        _ => None,
    }
}

fn synthesize_extract_register_value(
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let high = match instruction.operands.get(1)? {
        Operand::Register(register) => {
            normalize_value_for_width(read_simple_register_value(known_values, register)?, width)
        }
        _ => return None,
    };
    let low = match instruction.operands.get(2)? {
        Operand::Register(register) => {
            normalize_value_for_width(read_simple_register_value(known_values, register)?, width)
        }
        _ => return None,
    };
    let shift = parse_bit_shift_amount(instruction.operands.get(3)?, width)?;
    let value = extract_register_pair_value(high, low, width, shift)?;
    Some((destination, value, RecoveredValueSource::Other))
}

fn extract_register_pair_value(high: u64, low: u64, width: u32, shift: u32) -> Option<u64> {
    let mask = low_bit_mask(width)?;
    if shift == 0 {
        return Some(low & mask);
    }
    let high = (high & mask).checked_shl(width - shift)?;
    let low = (low & mask).checked_shr(shift)?;
    Some((high | low) & mask)
}

fn synthesize_shift_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let variable_shift =
        mnemonic.ends_with('v') || matches!(instruction.operands.get(2)?, Operand::Register(_));
    let source_value = if variable_shift {
        read_width_checked_register_operand(instruction.operands.get(1)?, width, known_values)?
    } else {
        match instruction.operands.get(1)? {
            Operand::Register(register) => read_simple_register_value(known_values, register)?,
            _ => return None,
        }
    };
    let shift = if variable_shift {
        parse_variable_shift_amount(instruction.operands.get(2)?, width, known_values)?
    } else {
        parse_bit_shift_amount(instruction.operands.get(2)?, width)?
    };
    let value = match shift_operation(mnemonic)? {
        "lsl" => source_value.checked_shl(shift)?,
        "lsr" => source_value.checked_shr(shift)?,
        "asr" => arithmetic_shift_right(source_value, width, shift),
        "ror" => rotate_right_with_width(source_value, width, shift),
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn shift_operation(mnemonic: &str) -> Option<&'static str> {
    match mnemonic {
        "lsl" | "lslv" => Some("lsl"),
        "lsr" | "lsrv" => Some("lsr"),
        "asr" | "asrv" => Some("asr"),
        "ror" | "rorv" => Some("ror"),
        _ => None,
    }
}

fn parse_variable_shift_amount(
    operand: &Operand,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u32> {
    let raw = match operand {
        Operand::Register(register) if register_value_width(register)? == width => {
            read_simple_register_value(known_values, register)?
        }
        _ => return None,
    };
    Some((raw % u64::from(width)) as u32)
}

fn synthesize_bitfield_extract_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let source = match instruction.operands.get(1)? {
        Operand::Register(register) => register,
        _ => return None,
    };
    let lsb = parse_bitfield_immediate(instruction.operands.get(2)?)?;
    let field_width = parse_bitfield_immediate(instruction.operands.get(3)?)?;
    if field_width == 0 || field_width > width || lsb.checked_add(field_width)? > width {
        return None;
    }
    let source_value =
        normalize_value_for_width(read_simple_register_value(known_values, source)?, width);
    let extracted = (source_value >> lsb) & low_bit_mask(field_width)?;
    let value = match mnemonic {
        "ubfx" => extracted,
        "sbfx" => sign_extend_bitfield(extracted, field_width),
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn synthesize_bitfield_insert_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let (source_value, lsb, field_width) = if mnemonic == "bfc" {
        (
            0,
            parse_bitfield_immediate(instruction.operands.get(1)?)?,
            parse_bitfield_immediate(instruction.operands.get(2)?)?,
        )
    } else {
        let source = match instruction.operands.get(1)? {
            Operand::Register(register) => register,
            _ => return None,
        };
        (
            normalize_value_for_width(read_simple_register_value(known_values, source)?, width),
            parse_bitfield_immediate(instruction.operands.get(2)?)?,
            parse_bitfield_immediate(instruction.operands.get(3)?)?,
        )
    };
    let field_mask = bitfield_range_mask(lsb, field_width, width)?;
    let value = match mnemonic {
        "ubfiz" => ((source_value & low_bit_mask(field_width)?) << lsb) & low_bit_mask(width)?,
        "sbfiz" => sign_fill_inserted_bitfield(source_value, lsb, field_width, width)?,
        "bfi" => {
            let previous = read_simple_register_value(known_values, &destination)?;
            let inserted = (source_value & low_bit_mask(field_width)?) << lsb;
            (previous & !field_mask) | inserted
        }
        "bfxil" => {
            let previous = read_simple_register_value(known_values, &destination)?;
            let inserted = (source_value >> lsb) & low_bit_mask(field_width)?;
            (previous & !low_bit_mask(field_width)?) | inserted
        }
        "bfc" => {
            let previous = read_simple_register_value(known_values, &destination)?;
            previous & !field_mask
        }
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn bitfield_range_mask(lsb: u32, field_width: u32, register_width: u32) -> Option<u64> {
    if field_width == 0
        || field_width > register_width
        || lsb.checked_add(field_width)? > register_width
    {
        return None;
    }
    low_bit_mask(field_width)?.checked_shl(lsb)
}

fn sign_fill_inserted_bitfield(
    source_value: u64,
    lsb: u32,
    field_width: u32,
    register_width: u32,
) -> Option<u64> {
    let inserted = (source_value & low_bit_mask(field_width)?) << lsb;
    let sign_bit = 1_u64.checked_shl(field_width - 1)?;
    let high_start = lsb.checked_add(field_width)?;
    let high_fill = if source_value & sign_bit == 0 || high_start == register_width {
        0
    } else {
        low_bit_mask(register_width)? & !low_bit_mask(high_start)?
    };
    Some((inserted | high_fill) & low_bit_mask(register_width)?)
}

fn parse_bitfield_immediate(operand: &Operand) -> Option<u32> {
    match operand {
        Operand::ImmediateUnsigned(value) => u32::try_from(*value).ok(),
        Operand::ImmediateSigned(value) if *value >= 0 => u32::try_from(*value).ok(),
        _ => None,
    }
}

fn low_bit_mask(width: u32) -> Option<u64> {
    if width == 0 || width > 64 {
        None
    } else if width == 64 {
        Some(u64::MAX)
    } else {
        Some((1_u64 << width) - 1)
    }
}

fn sign_extend_bitfield(value: u64, width: u32) -> u64 {
    if width == 64 {
        value
    } else {
        let sign_bit = 1_u64 << (width - 1);
        if value & sign_bit == 0 {
            value
        } else {
            value | (!0_u64 << width)
        }
    }
}

fn synthesize_multiply_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let lhs = read_normalized_register_operand(instruction.operands.get(1)?, width, known_values)?;
    let rhs = read_normalized_register_operand(instruction.operands.get(2)?, width, known_values)?;
    let product = lhs.wrapping_mul(rhs);
    let value = match mnemonic {
        "mul" => product,
        "madd" => {
            let addend = read_normalized_register_operand(
                instruction.operands.get(3)?,
                width,
                known_values,
            )?;
            product.wrapping_add(addend)
        }
        "msub" => {
            let addend = read_normalized_register_operand(
                instruction.operands.get(3)?,
                width,
                known_values,
            )?;
            addend.wrapping_sub(product)
        }
        "mneg" => 0_u64.wrapping_sub(product),
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn synthesize_carry_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let carry_in = known_flags?.c;
    let (lhs, rhs) = carry_arithmetic_operands(mnemonic, instruction, width, known_values)?;
    let (value, _) = carry_arithmetic_result(mnemonic, lhs, rhs, carry_in, width)?;
    Some((destination, value, RecoveredValueSource::Other))
}

fn carry_arithmetic_operands(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(u64, u64)> {
    match mnemonic {
        "adc" | "adcs" | "sbc" | "sbcs" => {
            let lhs = read_width_checked_register_operand(
                instruction.operands.get(1)?,
                width,
                known_values,
            )?;
            let rhs = read_width_checked_register_operand(
                instruction.operands.get(2)?,
                width,
                known_values,
            )?;
            Some((lhs, rhs))
        }
        "ngc" | "ngcs" => {
            let rhs = read_width_checked_register_operand(
                instruction.operands.get(1)?,
                width,
                known_values,
            )?;
            Some((0, rhs))
        }
        _ => None,
    }
}

fn carry_arithmetic_result(
    mnemonic: &str,
    lhs: u64,
    rhs: u64,
    carry_in: bool,
    width: u32,
) -> Option<(u64, NzcvFlags)> {
    let mask = low_bit_mask(width)?;
    match mnemonic {
        "adc" | "adcs" => add_with_carry(lhs, rhs, carry_in, width),
        "sbc" | "sbcs" | "ngc" | "ngcs" => add_with_carry(lhs, !rhs & mask, carry_in, width),
        _ => None,
    }
}

fn synthesize_long_multiply_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    if register_value_width_for_write(&destination)? != 64 {
        return None;
    }
    let signed = mnemonic.starts_with('s');
    let lhs = read_width_checked_register_operand(instruction.operands.get(1)?, 32, known_values)?;
    let rhs = read_width_checked_register_operand(instruction.operands.get(2)?, 32, known_values)?;
    let product = long_multiply_product(lhs, rhs, signed);
    let value = match mnemonic {
        "umaddl" | "smaddl" => {
            read_width_checked_register_operand(instruction.operands.get(3)?, 64, known_values)?
                .wrapping_add(product)
        }
        "umsubl" | "smsubl" => {
            read_width_checked_register_operand(instruction.operands.get(3)?, 64, known_values)?
                .wrapping_sub(product)
        }
        "umull" | "smull" => product,
        "umnegl" | "smnegl" => 0_u64.wrapping_sub(product),
        _ => return None,
    };
    Some((destination, value, RecoveredValueSource::Other))
}

fn long_multiply_product(lhs: u64, rhs: u64, signed: bool) -> u64 {
    if signed {
        let lhs = i128::from((lhs as u32) as i32);
        let rhs = i128::from((rhs as u32) as i32);
        lhs.wrapping_mul(rhs) as u64
    } else {
        u64::from(lhs as u32).wrapping_mul(u64::from(rhs as u32))
    }
}

fn synthesize_high_multiply_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    if register_value_width_for_write(&destination)? != 64 {
        return None;
    }
    let lhs = read_width_checked_register_operand(instruction.operands.get(1)?, 64, known_values)?;
    let rhs = read_width_checked_register_operand(instruction.operands.get(2)?, 64, known_values)?;
    let value = match mnemonic {
        "umulh" => ((u128::from(lhs) * u128::from(rhs)) >> 64) as u64,
        "smulh" => (((lhs as i64 as i128) * (rhs as i64 as i128)) >> 64) as u64,
        _ => return None,
    };
    Some((destination, value, RecoveredValueSource::Other))
}

fn synthesize_divide_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let dividend =
        read_normalized_register_operand(instruction.operands.get(1)?, width, known_values)?;
    let divisor =
        read_normalized_register_operand(instruction.operands.get(2)?, width, known_values)?;
    let value = match mnemonic {
        "udiv" => {
            if divisor == 0 {
                0
            } else {
                dividend / divisor
            }
        }
        "sdiv" => signed_divide_with_width(dividend, divisor, width),
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn signed_divide_with_width(dividend: u64, divisor: u64, width: u32) -> u64 {
    let divisor = signed_value_for_width(divisor, width);
    if divisor == 0 {
        return 0;
    }
    let dividend = signed_value_for_width(dividend, width);
    (dividend / divisor) as u64
}

fn signed_value_for_width(value: u64, width: u32) -> i128 {
    if width == 32 {
        i128::from((value as u32) as i32)
    } else {
        i128::from(value as i64)
    }
}

fn synthesize_count_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let source_value =
        read_normalized_register_operand(instruction.operands.get(1)?, width, known_values)?;
    let value = match mnemonic {
        "clz" => u64::from(count_leading_zeros_with_width(source_value, width)),
        "cls" => u64::from(count_leading_sign_bits_with_width(source_value, width)?),
        _ => return None,
    };
    Some((destination, value, RecoveredValueSource::Other))
}

fn count_leading_zeros_with_width(value: u64, width: u32) -> u32 {
    if width == 32 {
        (value as u32).leading_zeros()
    } else {
        value.leading_zeros()
    }
}

fn count_leading_sign_bits_with_width(value: u64, width: u32) -> Option<u32> {
    let value = normalize_value_for_width(value, width);
    let sign_mask = 1_u64.checked_shl(width.checked_sub(1)?)?;
    let comparison = if value & sign_mask == 0 {
        value
    } else {
        !value & low_bit_mask(width)?
    };
    Some(count_leading_zeros_with_width(comparison, width).saturating_sub(1))
}

fn synthesize_reverse_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let source_value =
        read_normalized_register_operand(instruction.operands.get(1)?, width, known_values)?;
    let value = match mnemonic {
        "rbit" => reverse_bits_with_width(source_value, width),
        "rev" => reverse_bytes_with_width(source_value, width),
        "rev16" => reverse_bytes_in_halfwords(source_value, width)?,
        "rev32" => reverse_bytes_in_words(source_value, width)?,
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn reverse_bits_with_width(value: u64, width: u32) -> u64 {
    if width == 32 {
        u64::from((value as u32).reverse_bits())
    } else {
        value.reverse_bits()
    }
}

fn reverse_bytes_with_width(value: u64, width: u32) -> u64 {
    if width == 32 {
        u64::from((value as u32).swap_bytes())
    } else {
        value.swap_bytes()
    }
}

fn reverse_bytes_in_halfwords(value: u64, width: u32) -> Option<u64> {
    let value = normalize_value_for_width(value, width);
    let mut result = 0_u64;
    let mut offset = 0_u32;
    while offset < width {
        let halfword = (value >> offset) & 0xffff;
        let reversed = ((halfword & 0xff) << 8) | (halfword >> 8);
        result |= reversed.checked_shl(offset)?;
        offset += 16;
    }
    Some(result)
}

fn reverse_bytes_in_words(value: u64, width: u32) -> Option<u64> {
    if width != 64 {
        return None;
    }
    let low = u64::from((value as u32).swap_bytes());
    let high = u64::from(((value >> 32) as u32).swap_bytes());
    Some((high << 32) | low)
}

fn read_normalized_register_operand(
    operand: &Operand,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    match operand {
        Operand::Register(register) => {
            let value = read_simple_register_value(known_values, register)?;
            Some(normalize_value_for_width(value, width))
        }
        _ => None,
    }
}

fn read_width_checked_register_operand(
    operand: &Operand,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    match operand {
        Operand::Register(register) if register_value_width(register)? == width => {
            read_normalized_register_operand(operand, width, known_values)
        }
        _ => None,
    }
}

fn logical_operand_value(
    operand: &Operand,
    destination_width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    match operand {
        Operand::ImmediateUnsigned(value) => Some(*value),
        Operand::ImmediateSigned(value) if *value >= 0 => Some(*value as u64),
        Operand::Label(value) => Some(*value),
        Operand::Register(register) => read_simple_register_value(known_values, register),
        Operand::ShiftedRegister { register, shift } => {
            let value = read_simple_register_value(known_values, register)?;
            shifted_logical_operand_value(register, value, shift, destination_width)
        }
        _ => None,
    }
}

fn shifted_logical_operand_value(
    register: &str,
    value: u64,
    shift: &str,
    destination_width: u32,
) -> Option<u64> {
    let lower = shift.to_ascii_lowercase();
    let op = shift_modifier_op(&lower)?;
    let amount = parse_shift_text(&lower, Some(0))?;
    let width = register_value_width(register).unwrap_or(destination_width);
    if amount >= width {
        return None;
    }
    let value = normalize_value_for_width(value, width);
    match op {
        "lsl" => value.checked_shl(amount),
        "lsr" => value.checked_shr(amount),
        "asr" => Some(arithmetic_shift_right(value, width, amount)),
        "ror" => Some(rotate_right_with_width(value, width, amount)),
        _ => None,
    }
}

fn rotate_right_with_width(value: u64, width: u32, amount: u32) -> u64 {
    if width == 32 {
        u64::from((value as u32).rotate_right(amount))
    } else {
        value.rotate_right(amount)
    }
}

fn synthesize_unary_register_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let destination_width = register_value_width_for_write(&destination)?;
    let value = match mnemonic {
        "mvn" => !logical_operand_value(
            instruction.operands.get(1)?,
            destination_width,
            known_values,
        )?,
        "neg" | "negs" => {
            let operand = arithmetic_register_operand_value(
                instruction.operands.get(1)?,
                &destination,
                known_values,
            )?;
            0_u64.wrapping_sub(arithmetic_operand_wrapping_value(operand))
        }
        "uxtb" | "uxth" | "uxtw" | "sxtb" | "sxth" | "sxtw" => {
            let source = match instruction.operands.get(1)? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let source_value = read_simple_register_value(known_values, source)?;
            extend_register_alias_value(mnemonic, source_value)?
        }
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, destination_width),
        RecoveredValueSource::Other,
    ))
}

fn arithmetic_operand_wrapping_value(operand: ArithmeticOperandValue) -> u64 {
    match operand {
        ArithmeticOperandValue::Unsigned(value) => value,
        ArithmeticOperandValue::Signed(value) => value as u64,
    }
}

fn extend_register_alias_value(mnemonic: &str, value: u64) -> Option<u64> {
    match mnemonic {
        "uxtb" => Some(value & 0xff),
        "uxth" => Some(value & 0xffff),
        "uxtw" => Some(u64::from(value as u32)),
        "sxtb" => Some((value as u8 as i8) as i64 as u64),
        "sxth" => Some((value as u16 as i16) as i64 as u64),
        "sxtw" => Some((value as u32 as i32) as i64 as u64),
        _ => None,
    }
}

fn instruction_updates_condition_flags(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "adds"
            | "adcs"
            | "cmn"
            | "subs"
            | "sbcs"
            | "cmp"
            | "negs"
            | "ngcs"
            | "ands"
            | "bics"
            | "tst"
            | "ccmp"
            | "ccmn"
    )
}

fn synthesize_condition_flags(
    instruction: &DecodedInstruction,
    mnemonic: &str,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<NzcvFlags> {
    match mnemonic {
        "adds" | "subs" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let width = register_value_width_for_write(destination)?;
            let lhs = read_normalized_register_operand(
                instruction.operands.get(1)?,
                width,
                known_values,
            )?;
            let rhs = arithmetic_operand_known_value(
                instruction.operands.get(2)?,
                instruction.operands.get(3),
                destination,
                width,
                known_values,
            )?;
            match mnemonic {
                "adds" => flags_for_addition(lhs, rhs, width),
                "subs" => flags_for_subtraction(lhs, rhs, width),
                _ => None,
            }
        }
        "adcs" | "sbcs" | "ngcs" => {
            synthesize_carry_condition_flags(mnemonic, instruction, known_values, known_flags)
        }
        "cmn" | "cmp" => {
            let lhs_register = match instruction.operands.first()? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let width = register_value_width(lhs_register)?;
            let lhs = read_normalized_register_operand(
                instruction.operands.first()?,
                width,
                known_values,
            )?;
            let rhs = arithmetic_operand_known_value(
                instruction.operands.get(1)?,
                instruction.operands.get(2),
                lhs_register,
                width,
                known_values,
            )?;
            match mnemonic {
                "cmn" => flags_for_addition(lhs, rhs, width),
                "cmp" => flags_for_subtraction(lhs, rhs, width),
                _ => None,
            }
        }
        "negs" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let width = register_value_width_for_write(destination)?;
            let rhs = arithmetic_operand_known_value(
                instruction.operands.get(1)?,
                instruction.operands.get(2),
                destination,
                width,
                known_values,
            )?;
            flags_for_subtraction(0, rhs, width)
        }
        "ands" | "bics" => {
            let destination = match instruction.operands.first()? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let width = register_value_width_for_write(destination)?;
            let lhs = read_normalized_register_operand(
                instruction.operands.get(1)?,
                width,
                known_values,
            )?;
            let rhs = logical_operand_value(instruction.operands.get(2)?, width, known_values)?;
            let result = if mnemonic == "bics" {
                lhs & !rhs
            } else {
                lhs & rhs
            };
            flags_for_logical_result(result, width)
        }
        "tst" => {
            let lhs_register = match instruction.operands.first()? {
                Operand::Register(register) => register,
                _ => return None,
            };
            let width = register_value_width(lhs_register)?;
            let lhs = read_normalized_register_operand(
                instruction.operands.first()?,
                width,
                known_values,
            )?;
            let rhs = logical_operand_value(instruction.operands.get(1)?, width, known_values)?;
            flags_for_logical_result(lhs & rhs, width)
        }
        "ccmp" | "ccmn" => {
            synthesize_conditional_compare_flags(mnemonic, instruction, known_values, known_flags)
        }
        _ => None,
    }
}

fn synthesize_conditional_compare_flags(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<NzcvFlags> {
    let lhs_register = match instruction.operands.first()? {
        Operand::Register(register) => register,
        _ => return None,
    };
    let width = register_value_width(lhs_register)?;
    let fallback_flags = nzcv_flags_from_operand(instruction.operands.get(2)?)?;
    let condition = condition_operand_text(instruction.operands.get(3)?)?;
    if !condition_holds(condition, known_flags?)? {
        return Some(fallback_flags);
    }

    let lhs = read_normalized_register_operand(instruction.operands.first()?, width, known_values)?;
    let rhs = conditional_compare_operand_value(instruction.operands.get(1)?, width, known_values)?;
    match mnemonic {
        "ccmp" => flags_for_subtraction(lhs, rhs, width),
        "ccmn" => flags_for_addition(lhs, rhs, width),
        _ => None,
    }
}

fn synthesize_carry_condition_flags(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<NzcvFlags> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register,
        _ => return None,
    };
    let width = register_value_width_for_write(destination)?;
    let carry_in = known_flags?.c;
    let (lhs, rhs) = carry_arithmetic_operands(mnemonic, instruction, width, known_values)?;
    let (_, flags) = carry_arithmetic_result(mnemonic, lhs, rhs, carry_in, width)?;
    Some(flags)
}

fn conditional_compare_operand_value(
    operand: &Operand,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    let value = match operand {
        Operand::ImmediateUnsigned(value) if *value <= 31 => *value,
        Operand::ImmediateSigned(value) if (0..=31).contains(value) => {
            u64::try_from(*value).ok()?
        }
        Operand::Register(register) if register_value_width(register)? == width => {
            read_normalized_register_operand(operand, width, known_values)?
        }
        _ => return None,
    };
    Some(normalize_value_for_width(value, width))
}

fn nzcv_flags_from_operand(operand: &Operand) -> Option<NzcvFlags> {
    let value = match operand {
        Operand::ImmediateUnsigned(value) => u8::try_from(*value).ok()?,
        Operand::ImmediateSigned(value) if *value >= 0 => u8::try_from(*value).ok()?,
        _ => return None,
    };
    nzcv_flags_from_immediate(value)
}

fn nzcv_flags_from_immediate(value: u8) -> Option<NzcvFlags> {
    if value > 0b1111 {
        return None;
    }
    Some(NzcvFlags {
        n: value & 0b1000 != 0,
        z: value & 0b0100 != 0,
        c: value & 0b0010 != 0,
        v: value & 0b0001 != 0,
    })
}

fn arithmetic_operand_known_value(
    operand: &Operand,
    optional_modifier: Option<&Operand>,
    reference_register: &str,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    let value = match operand {
        Operand::ImmediateUnsigned(value) => {
            shifted_add_sub_immediate_value(*value, optional_modifier)?
        }
        Operand::ImmediateSigned(value) if *value >= 0 => {
            shifted_add_sub_immediate_value(u64::try_from(*value).ok()?, optional_modifier)?
        }
        Operand::Label(value) => shifted_add_sub_immediate_value(*value, optional_modifier)?,
        Operand::Register(_) | Operand::ShiftedRegister { .. } => {
            arithmetic_operand_wrapping_value(arithmetic_register_operand_value(
                operand,
                reference_register,
                known_values,
            )?)
        }
        _ => return None,
    };
    Some(normalize_value_for_width(value, width))
}

fn shifted_add_sub_immediate_value(value: u64, optional_modifier: Option<&Operand>) -> Option<u64> {
    let shift = match optional_modifier {
        None => 0,
        Some(Operand::Other(text)) | Some(Operand::Name(text)) => {
            let lower = text.to_ascii_lowercase();
            if shift_modifier_op(&lower)? != "lsl" {
                return None;
            }
            parse_shift_text(&lower, None)?
        }
        Some(_) => return None,
    };
    if !matches!(shift, 0 | 12) {
        return None;
    }
    value.checked_shl(shift)
}

fn flags_for_addition(lhs: u64, rhs: u64, width: u32) -> Option<NzcvFlags> {
    let mask = low_bit_mask(width)?;
    let lhs = lhs & mask;
    let rhs = rhs & mask;
    let sum = u128::from(lhs) + u128::from(rhs);
    let result = (sum & u128::from(mask)) as u64;
    let carry = sum > u128::from(mask);
    let sign = 1_u64.checked_shl(width.checked_sub(1)?)?;
    let overflow = ((lhs ^ result) & (rhs ^ result) & sign) != 0;
    flags_for_result(result, width, carry, overflow)
}

fn add_with_carry(lhs: u64, rhs: u64, carry_in: bool, width: u32) -> Option<(u64, NzcvFlags)> {
    let mask = low_bit_mask(width)?;
    let lhs = lhs & mask;
    let rhs = rhs & mask;
    let carry = u128::from(carry_in);
    let sum = u128::from(lhs) + u128::from(rhs) + carry;
    let result = (sum & u128::from(mask)) as u64;
    let carry_out = sum > u128::from(mask);
    let sign = 1_u64.checked_shl(width.checked_sub(1)?)?;
    let overflow = ((lhs ^ result) & (rhs ^ result) & sign) != 0;
    Some((
        result,
        flags_for_result(result, width, carry_out, overflow)?,
    ))
}

fn flags_for_subtraction(lhs: u64, rhs: u64, width: u32) -> Option<NzcvFlags> {
    let mask = low_bit_mask(width)?;
    let lhs = lhs & mask;
    let rhs = rhs & mask;
    let result = lhs.wrapping_sub(rhs) & mask;
    let carry = lhs >= rhs;
    let sign = 1_u64.checked_shl(width.checked_sub(1)?)?;
    let overflow = ((lhs ^ rhs) & (lhs ^ result) & sign) != 0;
    flags_for_result(result, width, carry, overflow)
}

fn flags_for_logical_result(value: u64, width: u32) -> Option<NzcvFlags> {
    flags_for_result(value, width, false, false)
}

fn flags_for_result(value: u64, width: u32, c: bool, v: bool) -> Option<NzcvFlags> {
    let value = normalize_value_for_width(value, width);
    let sign = 1_u64.checked_shl(width.checked_sub(1)?)?;
    Some(NzcvFlags {
        n: value & sign != 0,
        z: value == 0,
        c,
        v,
    })
}

fn synthesize_conditional_select_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let condition_outcome = known_flags.and_then(|flags| {
        conditional_select_condition(instruction)
            .and_then(|condition| condition_holds(condition, flags))
    });
    let selected_value = match condition_outcome {
        Some(true) => conditional_select_true_value(instruction, width, known_values)?,
        Some(false) => conditional_select_false_value(mnemonic, instruction, width, known_values)?,
        None => {
            let true_value = conditional_select_true_value(instruction, width, known_values)?;
            let false_value =
                conditional_select_false_value(mnemonic, instruction, width, known_values)?;
            if true_value != false_value {
                return None;
            }
            true_value
        }
    };

    Some((destination, selected_value, RecoveredValueSource::Other))
}

fn conditional_select_true_value(
    instruction: &DecodedInstruction,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    match instruction.operands.get(1)? {
        Operand::Register(register) => Some(normalize_value_for_width(
            read_simple_register_value(known_values, register)?,
            width,
        )),
        _ => None,
    }
}

fn conditional_select_false_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    width: u32,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<u64> {
    let false_source = match instruction.operands.get(2)? {
        Operand::Register(register) => read_simple_register_value(known_values, register)?,
        _ => return None,
    };
    let false_value = match mnemonic {
        "csel" => false_source,
        "csinc" => false_source.wrapping_add(1),
        "csinv" => !false_source,
        "csneg" => 0_u64.wrapping_sub(false_source),
        _ => return None,
    };
    Some(normalize_value_for_width(false_value, width))
}

fn synthesize_conditional_select_alias_value(
    mnemonic: &str,
    instruction: &DecodedInstruction,
    known_values: &BTreeMap<String, RecoveredValue>,
    known_flags: Option<NzcvFlags>,
) -> Option<(String, u64, RecoveredValueSource)> {
    let destination = match instruction.operands.first()? {
        Operand::Register(register) => register.clone(),
        _ => return None,
    };
    let width = register_value_width_for_write(&destination)?;
    let condition = conditional_select_alias_condition(mnemonic, instruction)?;
    let condition_holds = condition_holds(condition, known_flags?)?;
    let value = match mnemonic {
        "cset" => u64::from(condition_holds),
        "csetm" => {
            if condition_holds {
                low_bit_mask(width)?
            } else {
                0
            }
        }
        "cinc" | "cinv" | "cneg" => {
            let source = read_width_checked_register_operand(
                instruction.operands.get(1)?,
                width,
                known_values,
            )?;
            match mnemonic {
                "cinc" if condition_holds => source.wrapping_add(1),
                "cinv" if condition_holds => !source,
                "cneg" if condition_holds => 0_u64.wrapping_sub(source),
                "cinc" | "cinv" | "cneg" => source,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((
        destination,
        normalize_value_for_width(value, width),
        RecoveredValueSource::Other,
    ))
}

fn conditional_select_alias_condition<'a>(
    mnemonic: &str,
    instruction: &'a DecodedInstruction,
) -> Option<&'a str> {
    let index = match mnemonic {
        "cset" | "csetm" => 1,
        "cinc" | "cinv" | "cneg" => 2,
        _ => return None,
    };
    condition_operand_text(instruction.operands.get(index)?)
}

fn conditional_select_condition(instruction: &DecodedInstruction) -> Option<&str> {
    condition_operand_text(instruction.operands.get(3)?)
}

fn condition_operand_text(operand: &Operand) -> Option<&str> {
    match operand {
        Operand::Condition(condition) | Operand::Name(condition) | Operand::Other(condition) => {
            Some(condition.trim())
        }
        _ => None,
    }
}

fn condition_holds(condition: &str, flags: NzcvFlags) -> Option<bool> {
    let condition = condition.trim_start_matches('.').to_ascii_lowercase();
    match condition.as_str() {
        "eq" => Some(flags.z),
        "ne" => Some(!flags.z),
        "cs" | "hs" => Some(flags.c),
        "cc" | "lo" => Some(!flags.c),
        "mi" => Some(flags.n),
        "pl" => Some(!flags.n),
        "vs" => Some(flags.v),
        "vc" => Some(!flags.v),
        "hi" => Some(flags.c && !flags.z),
        "ls" => Some(!flags.c || flags.z),
        "ge" => Some(flags.n == flags.v),
        "lt" => Some(flags.n != flags.v),
        "gt" => Some(!flags.z && flags.n == flags.v),
        "le" => Some(flags.z || flags.n != flags.v),
        "al" => Some(true),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum ArithmeticOperandValue {
    Unsigned(u64),
    Signed(i64),
}

fn arithmetic_register_operand_value(
    operand: &Operand,
    destination: &str,
    known_values: &BTreeMap<String, RecoveredValue>,
) -> Option<ArithmeticOperandValue> {
    match operand {
        Operand::Register(register) => Some(ArithmeticOperandValue::Unsigned(
            read_simple_register_value(known_values, register)?,
        )),
        Operand::ShiftedRegister { register, shift } => {
            let value = read_simple_register_value(known_values, register)?;
            let destination_width = register_value_width(destination)?;
            shifted_arithmetic_operand_value(register, value, shift, destination_width)
        }
        _ => None,
    }
}

fn shifted_arithmetic_operand_value(
    register: &str,
    value: u64,
    shift: &str,
    destination_width: u32,
) -> Option<ArithmeticOperandValue> {
    let lower = shift.to_ascii_lowercase();
    let op = shift_modifier_op(&lower)?;
    let amount = parse_shift_text(&lower, Some(0))?;
    let source_width = register_value_width(register).unwrap_or(destination_width);
    match op {
        "lsl" => {
            if amount >= source_width {
                return None;
            }
            Some(ArithmeticOperandValue::Unsigned(value.checked_shl(amount)?))
        }
        "lsr" => {
            if amount >= source_width {
                return None;
            }
            Some(ArithmeticOperandValue::Unsigned(value.checked_shr(amount)?))
        }
        "asr" => {
            if amount >= source_width {
                return None;
            }
            Some(ArithmeticOperandValue::Signed(
                arithmetic_shift_right_signed(value, source_width, amount),
            ))
        }
        "uxtb" => extend_arithmetic_operand_unsigned(value & 0xff, amount),
        "uxth" => extend_arithmetic_operand_unsigned(value & 0xffff, amount),
        "uxtw" => extend_arithmetic_operand_unsigned(u64::from(value as u32), amount),
        "uxtx" => extend_arithmetic_operand_unsigned(value, amount),
        "sxtb" => extend_arithmetic_operand_signed((value as u8 as i8) as i64, amount),
        "sxth" => extend_arithmetic_operand_signed((value as u16 as i16) as i64, amount),
        "sxtw" => extend_arithmetic_operand_signed((value as u32 as i32) as i64, amount),
        "sxtx" => extend_arithmetic_operand_signed(i64::from_ne_bytes(value.to_ne_bytes()), amount),
        _ => None,
    }
}

fn extend_arithmetic_operand_unsigned(value: u64, shift: u32) -> Option<ArithmeticOperandValue> {
    if shift > 4 {
        return None;
    }
    Some(ArithmeticOperandValue::Unsigned(value.checked_shl(shift)?))
}

fn extend_arithmetic_operand_signed(value: i64, shift: u32) -> Option<ArithmeticOperandValue> {
    if shift > 4 {
        return None;
    }
    let scale = 1_i64.checked_shl(shift)?;
    Some(ArithmeticOperandValue::Signed(value.checked_mul(scale)?))
}

fn add_arithmetic_operand(base: u64, operand: ArithmeticOperandValue) -> Option<u64> {
    match operand {
        ArithmeticOperandValue::Unsigned(value) => base.checked_add(value),
        ArithmeticOperandValue::Signed(value) => add_signed(base, value),
    }
}

fn subtract_arithmetic_operand(base: u64, operand: ArithmeticOperandValue) -> Option<u64> {
    match operand {
        ArithmeticOperandValue::Unsigned(value) => base.checked_sub(value),
        ArithmeticOperandValue::Signed(value) => add_signed(base, value.checked_neg()?),
    }
}

fn register_value_width(register: &str) -> Option<u32> {
    let lower = register.to_ascii_lowercase();
    if lower.starts_with('w') {
        Some(32)
    } else if lower.starts_with('x') {
        Some(64)
    } else {
        None
    }
}

fn register_value_width_for_write(register: &str) -> Option<u32> {
    register_value_width(register).or_else(|| {
        let key = canonical_state_key(register)?;
        register_value_width(&key)
    })
}

fn parse_bit_shift_amount(operand: &Operand, width: u32) -> Option<u32> {
    let shift = match operand {
        Operand::ImmediateUnsigned(value) => u32::try_from(*value).ok()?,
        Operand::ImmediateSigned(value) if *value >= 0 => u32::try_from(*value).ok()?,
        Operand::Other(text) | Operand::Name(text) => parse_shift_text(text, None)?,
        _ => return None,
    };
    (shift < width).then_some(shift)
}

fn arithmetic_shift_right(value: u64, width: u32, shift: u32) -> u64 {
    arithmetic_shift_right_signed(value, width, shift) as u64
}

fn arithmetic_shift_right_signed(value: u64, width: u32, shift: u32) -> i64 {
    if width == 32 {
        i64::from((value as u32 as i32) >> shift)
    } else {
        (value as i64) >> shift
    }
}

fn read_simple_register_value(
    known_values: &BTreeMap<String, RecoveredValue>,
    register: &str,
) -> Option<u64> {
    if is_zero_register(register) {
        return Some(0);
    }
    read_known_value(known_values, register).map(|value| value.value)
}

fn is_zero_register(register: &str) -> bool {
    matches!(register.to_ascii_lowercase().as_str(), "xzr" | "wzr")
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

fn parse_mov_wide_shift_bits(destination: &str, operand: &Operand) -> Option<u32> {
    let shift = parse_shift_bits(operand)?;
    if destination.to_ascii_lowercase().starts_with('w') && shift > 16 {
        return None;
    }
    Some(shift)
}

fn parse_shift_from_text(text: &str) -> Option<u32> {
    let lower = text.to_ascii_lowercase();
    if shift_modifier_op(&lower)? != "lsl" {
        return None;
    }
    parse_shift_text(text, None)
}

fn parse_shift_text(text: &str, default_without_shift: Option<u32>) -> Option<u32> {
    let text = text.trim();
    let Some((_, suffix)) = text.split_once('#') else {
        return if text.chars().all(|ch| ch.is_ascii_alphabetic()) {
            default_without_shift
        } else {
            None
        };
    };
    let suffix = suffix.trim();
    if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<u32>().ok()
}

fn shift_modifier_op(text: &str) -> Option<&str> {
    let prefix = match text.trim().split_once('#') {
        Some((prefix, _)) => prefix.trim(),
        None => text.trim(),
    };
    let mut parts = prefix.split_ascii_whitespace();
    let op = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(op)
}

fn resolve_memory_effective_address(
    base_target: u64,
    index_register: Option<&str>,
    known_values: &BTreeMap<String, RecoveredValue>,
    index_transform: MemoryIndexTransform,
) -> Option<u64> {
    let Some(index_register) = index_register else {
        return Some(base_target);
    };
    let index = read_known_value(known_values, index_register)?.value;
    index_transform.apply(base_target, index)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemoryIndexTransform {
    extension: MemoryIndexExtension,
    shift: u32,
}

impl MemoryIndexTransform {
    fn unshifted_unsigned64() -> Self {
        Self {
            extension: MemoryIndexExtension::Unsigned64,
            shift: 0,
        }
    }

    fn apply(self, base: u64, index_value: u64) -> Option<u64> {
        match self.extension {
            MemoryIndexExtension::Unsigned32 => {
                let offset = u64::from(index_value as u32).checked_shl(self.shift)?;
                base.checked_add(offset)
            }
            MemoryIndexExtension::Unsigned64 => {
                let offset = index_value.checked_shl(self.shift)?;
                base.checked_add(offset)
            }
            MemoryIndexExtension::Signed32 => {
                let index = i64::from((index_value as u32) as i32);
                let scale = 1_i64.checked_shl(self.shift)?;
                add_signed(base, index.checked_mul(scale)?)
            }
            MemoryIndexExtension::Signed64 => {
                let index = i64::from_ne_bytes(index_value.to_ne_bytes());
                let scale = 1_i64.checked_shl(self.shift)?;
                add_signed(base, index.checked_mul(scale)?)
            }
        }
    }

    fn index_within_limit(self, index_value: u64, limit: u64) -> bool {
        match self.extension {
            MemoryIndexExtension::Unsigned32 => u64::from(index_value as u32) <= limit,
            MemoryIndexExtension::Unsigned64 => index_value <= limit,
            MemoryIndexExtension::Signed32 => {
                i64::from((index_value as u32) as i32).unsigned_abs() <= limit
            }
            MemoryIndexExtension::Signed64 => {
                i64::from_ne_bytes(index_value.to_ne_bytes()).unsigned_abs() <= limit
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryIndexExtension {
    Unsigned32,
    Unsigned64,
    Signed32,
    Signed64,
}

#[derive(Debug, Clone)]
struct JumpTableSource {
    table_base: u64,
    index_register: String,
    element_size: u8,
    index_transform: MemoryIndexTransform,
    slot_resolution: TableSlotResolution,
}

#[derive(Debug, Clone)]
struct TableSlotEvidence {
    table_base: u64,
    slot_address: u64,
    index_register: String,
    element_size: u8,
    encoding: TableSlotEncoding,
    target: u64,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedTableSlot {
    slot_address: u64,
    target: u64,
    kind: RecoveredValueKind,
    reason: IndirectTargetReason,
    encoding: TableSlotEncoding,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedTableSlotReason {
    reason: IndirectTargetReason,
    encoding: TableSlotEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableSlotResolution {
    AbsoluteOnly,
    RelativeOnly,
}

fn resolve_table_slot_at_address(
    image: &BinaryImage,
    table_base: u64,
    slot_address: u64,
    element_size: u8,
    resolution: TableSlotResolution,
) -> Option<ResolvedTableSlot> {
    let (_, bytes) = image.bytes_for_virtual_range(slot_address, usize::from(element_size))?;

    if element_size == 8 {
        if resolution != TableSlotResolution::AbsoluteOnly {
            return None;
        }
        let raw_bytes: [u8; 8] = bytes.try_into().ok()?;
        let raw = match image.endianness() {
            Endianness::Little => u64::from_le_bytes(raw_bytes),
            Endianness::Big => u64::from_be_bytes(raw_bytes),
        };
        let kind = classify_recovered_value(image, raw, RecoveredValueKind::Address);
        if raw != 0
            && (is_executable_target(image, raw)
                || !matches!(
                    kind,
                    RecoveredValueKind::Address | RecoveredValueKind::CString
                ))
        {
            return Some(ResolvedTableSlot {
                slot_address,
                target: raw,
                kind,
                reason: indirect_target_reason(image, raw),
                encoding: TableSlotEncoding::Absolute64,
            });
        }
        return None;
    }

    let raw_bytes: [u8; 4] = bytes.try_into().ok()?;
    if resolution == TableSlotResolution::RelativeOnly {
        let relative = match image.endianness() {
            Endianness::Little => i32::from_le_bytes(raw_bytes),
            Endianness::Big => i32::from_be_bytes(raw_bytes),
        } as i64;
        let relative_target = add_signed(table_base, relative)?;
        if is_executable_target(image, relative_target)
            || image.dyld().export_by_address(relative_target).is_some()
        {
            return Some(ResolvedTableSlot {
                slot_address,
                target: relative_target,
                kind: classify_recovered_value(image, relative_target, RecoveredValueKind::Address),
                reason: indirect_target_reason(image, relative_target),
                encoding: TableSlotEncoding::Relative32,
            });
        }
        return None;
    }

    let raw_u32 = match image.endianness() {
        Endianness::Little => u32::from_le_bytes(raw_bytes),
        Endianness::Big => u32::from_be_bytes(raw_bytes),
    };
    let absolute = u64::from(raw_u32);
    let kind = classify_recovered_value(image, absolute, RecoveredValueKind::Address);
    if absolute != 0
        && (is_executable_target(image, absolute)
            || !matches!(
                kind,
                RecoveredValueKind::Address | RecoveredValueKind::CString
            ))
    {
        return Some(ResolvedTableSlot {
            slot_address,
            target: absolute,
            kind,
            reason: indirect_target_reason(image, absolute),
            encoding: TableSlotEncoding::Absolute32,
        });
    }

    None
}

fn synthesize_literal_load(
    image: &BinaryImage,
    instruction: &DecodedInstruction,
    lower: &str,
) -> Option<(u64, RecoveredValue)> {
    if !matches!(lower, "ldr" | "ldrsw") {
        return None;
    }
    let target = literal_load_target_operand(instruction)?;
    let register = destination_register(instruction)?;
    instruction
        .references
        .iter()
        .any(
            |reference| matches!(reference, Reference::Data { target: data_target } if *data_target == target),
        )
        .then_some(())?;
    let value = read_literal_load_value(image, target, &register, lower)?;
    Some((
        target,
        RecoveredValue {
            register,
            value,
            kind: classify_recovered_value(image, value, RecoveredValueKind::Literal),
            source: RecoveredValueSource::LiteralLoad,
        },
    ))
}

fn literal_load_target_operand(instruction: &DecodedInstruction) -> Option<u64> {
    if instruction
        .operands
        .iter()
        .any(|operand| matches!(operand, Operand::Memory { .. }))
    {
        return None;
    }
    match instruction.operands.get(1)? {
        Operand::ImmediateUnsigned(value) | Operand::Label(value) => Some(*value),
        Operand::ImmediateSigned(value) if *value >= 0 => u64::try_from(*value).ok(),
        _ => None,
    }
}

fn read_literal_load_value(
    image: &BinaryImage,
    address: u64,
    register: &str,
    mnemonic: &str,
) -> Option<u64> {
    let (width, signed) = literal_load_width(register, mnemonic)?;
    read_memory_value(image, address, width, signed)
}

fn read_memory_value(image: &BinaryImage, address: u64, width: usize, signed: bool) -> Option<u64> {
    let (_, bytes) = image.bytes_for_virtual_range(address, width)?;
    match (width, signed, image.endianness()) {
        (1, true, _) => Some(i64::from(bytes.first().copied()? as i8) as u64),
        (1, false, _) => Some(u64::from(bytes.first().copied()?)),
        (2, true, Endianness::Little) => {
            Some(i64::from(i16::from_le_bytes(bytes.try_into().ok()?)) as u64)
        }
        (2, true, Endianness::Big) => {
            Some(i64::from(i16::from_be_bytes(bytes.try_into().ok()?)) as u64)
        }
        (2, false, Endianness::Little) => {
            Some(u64::from(u16::from_le_bytes(bytes.try_into().ok()?)))
        }
        (2, false, Endianness::Big) => Some(u64::from(u16::from_be_bytes(bytes.try_into().ok()?))),
        (4, true, Endianness::Little) => {
            Some(i64::from(i32::from_le_bytes(bytes.try_into().ok()?)) as u64)
        }
        (4, true, Endianness::Big) => {
            Some(i64::from(i32::from_be_bytes(bytes.try_into().ok()?)) as u64)
        }
        (4, false, Endianness::Little) => {
            Some(u64::from(u32::from_le_bytes(bytes.try_into().ok()?)))
        }
        (4, false, Endianness::Big) => Some(u64::from(u32::from_be_bytes(bytes.try_into().ok()?))),
        (8, false, Endianness::Little) => Some(u64::from_le_bytes(bytes.try_into().ok()?)),
        (8, false, Endianness::Big) => Some(u64::from_be_bytes(bytes.try_into().ok()?)),
        _ => None,
    }
}

fn literal_load_width(register: &str, mnemonic: &str) -> Option<(usize, bool)> {
    let register = register.to_ascii_lowercase();
    if mnemonic == "ldrsw" && register.starts_with('x') {
        Some((4, true))
    } else if mnemonic == "ldr" && register.starts_with('w') {
        Some((4, false))
    } else if mnemonic == "ldr" && register.starts_with('x') {
        Some((8, false))
    } else {
        None
    }
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

fn unresolved_load_reference_kind(image: &BinaryImage, address: u64) -> Option<RecoveredValueKind> {
    let kind = classify_recovered_value(image, address, RecoveredValueKind::Address);
    if matches!(
        kind,
        RecoveredValueKind::ObjcSelector
            | RecoveredValueKind::ObjcClass
            | RecoveredValueKind::ObjcMethodList
    ) || image.objc_selector_ref_at_address(address).is_some()
        || image.objc_class_ref_at_address(address).is_some()
        || image.objc_imported_class_name_at_address(address).is_some()
        || image.objc_method_list_owner_at_address(address).is_some()
    {
        Some(kind)
    } else {
        None
    }
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

fn should_resolve_indirect_target(
    image: &BinaryImage,
    target: u64,
    reason: IndirectTargetReason,
) -> bool {
    if target == 0 {
        return false;
    }
    if is_executable_target(image, target) {
        return true;
    }
    if reason == IndirectTargetReason::RegisterState
        && looks_like_sign_extended_negative_literal(target)
    {
        return false;
    }
    true
}

fn looks_like_sign_extended_negative_literal(target: u64) -> bool {
    target >> 32 == u64::from(u32::MAX) || (target <= u64::from(u32::MAX) && target >> 16 == 0xffff)
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
    index_transform: MemoryIndexTransform,
    resolution: TableSlotResolution,
) -> Option<(u64, u64, ResolvedTableSlotReason)> {
    if element_size != 4 && element_size != 8 {
        return None;
    }
    if !index_transform.index_within_limit(index_value, 0x1000) {
        return None;
    }
    let slot_address = index_transform.apply(base, index_value)?;
    let slot = resolve_table_slot_at_address(image, base, slot_address, element_size, resolution)?;
    Some((
        slot.slot_address,
        slot.target,
        ResolvedTableSlotReason {
            reason: slot.reason,
            encoding: slot.encoding,
        },
    ))
}

fn jump_table_element_size(instruction: &DecodedInstruction) -> u8 {
    let lower = instruction.mnemonic.to_ascii_lowercase();
    if lower.starts_with("ldrb")
        || lower.starts_with("ldurb")
        || lower.starts_with("ldrsb")
        || lower.starts_with("ldursb")
        || lower.starts_with("ldtrb")
        || lower.starts_with("ldtrsb")
        || lower.starts_with("ldarb")
        || lower.starts_with("ldaprb")
        || lower.starts_with("ldapurb")
        || lower.starts_with("ldapursb")
        || lower.starts_with("ldaxrb")
        || lower.starts_with("ldxrb")
        || lower.starts_with("strb")
        || lower.starts_with("sturb")
    {
        1
    } else if lower.starts_with("ldrh")
        || lower.starts_with("ldurh")
        || lower.starts_with("ldrsh")
        || lower.starts_with("ldursh")
        || lower.starts_with("ldtrh")
        || lower.starts_with("ldtrsh")
        || lower.starts_with("ldarh")
        || lower.starts_with("ldaprh")
        || lower.starts_with("ldapurh")
        || lower.starts_with("ldapursh")
        || lower.starts_with("ldaxrh")
        || lower.starts_with("ldxrh")
        || lower.starts_with("strh")
        || lower.starts_with("sturh")
    {
        2
    } else if lower.starts_with("ldrsw")
        || lower.starts_with("ldursw")
        || lower.starts_with("ldtrsw")
        || lower.starts_with("ldapursw")
        || lower.starts_with("ldpsw")
    {
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
        Platform, RecoveredValueSource, Section, SliceInfo, Symbol, SymbolKind, TableSlotEncoding,
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

    fn synthetic_high_half_executable_image() -> BinaryImage {
        let bytes: Arc<[u8]> = vec![0u8; 64].into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic-high-half".to_string()),
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
                address: 0xffff_ffff_ffff_f000,
                size: 64,
                file_offset: Some(0),
                file_size: 64,
                kind: "Text".to_string(),
                executable: true,
            }],
            vec![],
            vec![],
            vec![],
            ObjcMetadata::default(),
            damsel_core::DyldMetadata::default(),
            bytes,
        )
    }

    fn synthetic_stripped_absolute_table_image(text_base: u64, target: u64) -> BinaryImage {
        let mut bytes = vec![0u8; 72];
        bytes[64..72].copy_from_slice(&target.to_le_bytes());
        let bytes: Arc<[u8]> = bytes.into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic-stripped-absolute-table".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: 72,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![
                Section {
                    segment_name: "__TEXT".to_string(),
                    name: "__text".to_string(),
                    address: text_base,
                    size: 64,
                    file_offset: Some(0),
                    file_size: 64,
                    kind: "Text".to_string(),
                    executable: true,
                },
                Section {
                    segment_name: "__DATA_CONST".to_string(),
                    name: "__const".to_string(),
                    address: 0x4000,
                    size: 8,
                    file_offset: Some(64),
                    file_size: 8,
                    kind: "Data".to_string(),
                    executable: false,
                },
            ],
            vec![],
            vec![],
            vec![],
            ObjcMetadata::default(),
            damsel_core::DyldMetadata::default(),
            bytes,
        )
    }

    fn synthetic_absolute32_table_image(absolute_target: u32, relative_target: u64) -> BinaryImage {
        let mut bytes = vec![0u8; 68];
        bytes[64..68].copy_from_slice(&absolute_target.to_le_bytes());
        let bytes: Arc<[u8]> = bytes.into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic-absolute32-table".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: 68,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![
                Section {
                    segment_name: "__TEXT".to_string(),
                    name: "__text".to_string(),
                    address: u64::from(absolute_target),
                    size: 64,
                    file_offset: Some(0),
                    file_size: 64,
                    kind: "Text".to_string(),
                    executable: true,
                },
                Section {
                    segment_name: "__TEXT".to_string(),
                    name: "__other_text".to_string(),
                    address: relative_target,
                    size: 64,
                    file_offset: Some(0),
                    file_size: 64,
                    kind: "Text".to_string(),
                    executable: true,
                },
                Section {
                    segment_name: "__DATA_CONST".to_string(),
                    name: "__const".to_string(),
                    address: 0x4000,
                    size: 4,
                    file_offset: Some(64),
                    file_size: 4,
                    kind: "Data".to_string(),
                    executable: false,
                },
            ],
            vec![Symbol {
                name: "_absolute32_dispatch_target".to_string(),
                address: u64::from(absolute_target),
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
            damsel_core::DyldMetadata::default(),
            bytes,
        )
    }

    fn synthetic_literal_image(literal_address: u64, literal_bytes: &[u8]) -> BinaryImage {
        let mut bytes = vec![0u8; 64 + literal_bytes.len()];
        bytes[64..].copy_from_slice(literal_bytes);
        let bytes: Arc<[u8]> = bytes.into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic-literal".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            Endianness::Little,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: bytes.len() as u64,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![
                Section {
                    segment_name: "__TEXT".to_string(),
                    name: "__text".to_string(),
                    address: 0x3000,
                    size: 64,
                    file_offset: Some(0),
                    file_size: 64,
                    kind: "Text".to_string(),
                    executable: true,
                },
                Section {
                    segment_name: "__DATA_CONST".to_string(),
                    name: "__const".to_string(),
                    address: literal_address,
                    size: literal_bytes.len() as u64,
                    file_offset: Some(64),
                    file_size: literal_bytes.len() as u64,
                    kind: "Data".to_string(),
                    executable: false,
                },
            ],
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

    fn synthetic_table_image(relative_target: u64) -> BinaryImage {
        synthetic_table_image_with_endianness(relative_target, Endianness::Little)
    }

    fn synthetic_table_image_with_endianness(
        relative_target: u64,
        endianness: Endianness,
    ) -> BinaryImage {
        let mut bytes = vec![0u8; 96];
        let relative = i32::try_from((relative_target as i64) - 0x4000_i64)
            .expect("relative target offset fits into i32");
        match endianness {
            Endianness::Little => {
                bytes[64..68].copy_from_slice(&relative.to_le_bytes());
                bytes[72..80].copy_from_slice(&0x3000_u64.to_le_bytes());
            }
            Endianness::Big => {
                bytes[64..68].copy_from_slice(&relative.to_be_bytes());
                bytes[72..80].copy_from_slice(&0x3000_u64.to_be_bytes());
            }
        }
        let bytes: Arc<[u8]> = bytes.into();
        BinaryImage::from_memory_bytes(
            Some("disasm-synthetic-table".to_string()),
            BinaryFormat::MachO,
            Architecture::Arm64,
            endianness,
            None,
            Some(Platform::unknown("macos")),
            SliceInfo {
                offset: 0,
                size: 96,
                is_universal: false,
                cpu_subtype: 0,
            },
            vec![],
            vec![
                Section {
                    segment_name: "__TEXT".to_string(),
                    name: "__text".to_string(),
                    address: 0x3000,
                    size: 64,
                    file_offset: Some(0),
                    file_size: 64,
                    kind: "Text".to_string(),
                    executable: true,
                },
                Section {
                    segment_name: "__DATA_CONST".to_string(),
                    name: "__const".to_string(),
                    address: 0x4000,
                    size: 32,
                    file_offset: Some(64),
                    file_size: 32,
                    kind: "Data".to_string(),
                    executable: false,
                },
            ],
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
    fn block_id_for_address_uses_half_open_block_ranges() {
        let blocks = vec![
            BasicBlock {
                id: 0,
                start_address: 0x1000,
                end_address: 0x1008,
                instruction_count: 2,
                terminator_address: 0x1004,
                terminator_mnemonic: "add".to_string(),
                terminator_kind: BasicBlockTerminatorKind::Fallthrough,
            },
            BasicBlock {
                id: 1,
                start_address: 0x1008,
                end_address: 0x1010,
                instruction_count: 2,
                terminator_address: 0x100c,
                terminator_mnemonic: "ret".to_string(),
                terminator_kind: BasicBlockTerminatorKind::Return,
            },
        ];

        assert_eq!(block_id_for_address(&blocks, 0x0fff), None);
        assert_eq!(block_id_for_address(&blocks, 0x1000), Some(0));
        assert_eq!(block_id_for_address(&blocks, 0x1004), Some(0));
        assert_eq!(block_id_for_address(&blocks, 0x1008), Some(1));
        assert_eq!(block_id_for_address(&blocks, 0x100c), Some(1));
        assert_eq!(block_id_for_address(&blocks, 0x1010), None);
    }

    #[test]
    fn analysis_splits_basic_blocks_at_in_window_call_targets() {
        let instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Call { target: 0x1010 }],
                ..instruction(0x1000, "bl", vec![Operand::Label(0x1010)])
            },
            instruction(0x1004, "add", vec![]),
            instruction(0x1008, "add", vec![]),
            instruction(0x100c, "add", vec![]),
            instruction(0x1010, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-call-target", 0x1000, 0x1014, &instructions);
        let target_block = analysis
            .basic_blocks
            .iter()
            .find(|block| block.start_address == 0x1010)
            .expect("call target should start a block");

        assert_eq!(target_block.end_address, 0x1014);
        assert_eq!(target_block.terminator_address, 0x1010);
        assert_eq!(target_block.terminator_mnemonic, "ret");
        assert_eq!(
            target_block.terminator_kind,
            BasicBlockTerminatorKind::Return
        );
        assert!(
            analysis
                .basic_blocks
                .iter()
                .all(|block| block.start_address == 0x1010
                    || !(0x1010 > block.start_address && 0x1010 < block.end_address)),
            "call target must not sit in the middle of a predecessor block: {:?}",
            analysis.basic_blocks
        );
        assert!(
            analysis.edges.iter().any(|edge| {
                edge.source_address == 0x1000
                    && edge.kind == ControlFlowEdgeKind::Call
                    && edge.target_address == Some(0x1010)
                    && edge.to_block == Some(target_block.id)
            }),
            "call edge should resolve to the block that starts at the call target: {:?}",
            analysis.edges
        );
    }

    #[test]
    fn analysis_classifies_basic_block_terminator_kinds() {
        let cases = [
            ("add", BasicBlockTerminatorKind::Fallthrough),
            ("bl", BasicBlockTerminatorKind::Call),
            ("blr", BasicBlockTerminatorKind::IndirectCall),
            ("b", BasicBlockTerminatorKind::Branch),
            ("br", BasicBlockTerminatorKind::IndirectBranch),
            ("b.eq", BasicBlockTerminatorKind::ConditionalBranch),
            ("cbz", BasicBlockTerminatorKind::ConditionalBranch),
            ("ret", BasicBlockTerminatorKind::Return),
            ("eret", BasicBlockTerminatorKind::Return),
            ("brk", BasicBlockTerminatorKind::Trap),
        ];

        for (mnemonic, expected) in cases {
            let instructions = vec![instruction(0x1000, mnemonic, vec![])];
            let analysis = analyze_disassembly(
                format!("synthetic-{mnemonic}"),
                0x1000,
                0x1004,
                &instructions,
            );

            assert_eq!(
                analysis.basic_blocks[0].terminator_kind, expected,
                "{mnemonic}"
            );
            assert_eq!(analysis.basic_blocks[0].terminator_address, 0x1000);
            assert_eq!(analysis.basic_blocks[0].terminator_mnemonic, mnemonic);
        }
    }

    #[test]
    fn analysis_reports_entry_and_exit_block_counts_for_conditional_split() {
        let instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Branch { target: 0x1008 }],
                ..instruction(0x1000, "b.eq", vec![Operand::Label(0x1008)])
            },
            instruction(0x1004, "ret", vec![]),
            instruction(0x1008, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-split", 0x1000, 0x100c, &instructions);

        assert_eq!(analysis.summary.basic_block_count, 3);
        assert_eq!(analysis.summary.entry_block_count, 1);
        assert_eq!(analysis.summary.exit_block_count, 2);
        assert_eq!(analysis.summary.primary_entry_reachable_block_count, 3);
        assert_eq!(analysis.summary.disconnected_block_count, 0);
        assert_eq!(analysis.summary.edge_kind_counts.fallthrough_count, 1);
        assert_eq!(
            analysis.summary.edge_kind_counts.conditional_branch_count,
            1
        );
        assert_eq!(analysis.summary.edge_kind_counts.return_count, 2);
    }

    #[test]
    fn analysis_treats_bc_conditional_branches_as_splits() {
        let instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Branch { target: 0x1008 }],
                ..instruction(0x1000, "bc.eq", vec![Operand::Label(0x1008)])
            },
            instruction(0x1004, "ret", vec![]),
            instruction(0x1008, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-bc-split", 0x1000, 0x100c, &instructions);

        assert_eq!(analysis.summary.basic_block_count, 3);
        assert_eq!(analysis.summary.entry_block_count, 1);
        assert_eq!(analysis.summary.exit_block_count, 2);
        assert_eq!(analysis.summary.edge_kind_counts.fallthrough_count, 1);
        assert_eq!(
            analysis.summary.edge_kind_counts.conditional_branch_count,
            1
        );
        assert!(analysis.edges.iter().any(|edge| {
            edge.source_address == 0x1000
                && edge.target_address == Some(0x1008)
                && edge.kind == ControlFlowEdgeKind::ConditionalBranch
        }));
    }

    #[test]
    fn analysis_reports_multiple_entry_blocks_after_terminal_boundary() {
        let instructions = vec![
            instruction(0x1000, "ret", vec![]),
            instruction(0x1004, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-disjoint", 0x1000, 0x1008, &instructions);

        assert_eq!(analysis.summary.basic_block_count, 2);
        assert_eq!(analysis.summary.entry_block_count, 2);
        assert_eq!(analysis.summary.exit_block_count, 2);
        assert_eq!(analysis.summary.primary_entry_reachable_block_count, 1);
        assert_eq!(analysis.summary.disconnected_block_count, 1);
    }

    #[test]
    fn analysis_treats_drps_as_terminal_return_boundary() {
        let instructions = vec![
            instruction(0x1000, "drps", vec![]),
            instruction(0x1004, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-drps", 0x1000, 0x1008, &instructions);

        assert_eq!(analysis.summary.basic_block_count, 2);
        assert_eq!(analysis.summary.entry_block_count, 2);
        assert_eq!(analysis.summary.exit_block_count, 2);
        assert_eq!(analysis.summary.primary_entry_reachable_block_count, 1);
        assert_eq!(analysis.summary.disconnected_block_count, 1);
        assert_eq!(analysis.summary.return_count, 2);
        assert_eq!(analysis.summary.edge_kind_counts.return_count, 2);
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == ControlFlowEdgeKind::Return
                && edge.source_address == 0x1000
                && edge.via.is_none()
        }));
    }

    #[test]
    fn analysis_treats_authenticated_exception_returns_as_terminal_boundaries() {
        for mnemonic in ["eretaa", "eretab"] {
            let instructions = vec![
                instruction(0x1000, mnemonic, vec![]),
                instruction(0x1004, "ret", vec![]),
            ];

            let analysis = analyze_disassembly(
                format!("synthetic-{mnemonic}"),
                0x1000,
                0x1008,
                &instructions,
            );

            assert_eq!(analysis.summary.basic_block_count, 2, "{mnemonic}");
            assert_eq!(analysis.summary.entry_block_count, 2, "{mnemonic}");
            assert_eq!(analysis.summary.exit_block_count, 2, "{mnemonic}");
            assert_eq!(
                analysis.summary.primary_entry_reachable_block_count, 1,
                "{mnemonic}"
            );
            assert_eq!(analysis.summary.disconnected_block_count, 1, "{mnemonic}");
            assert_eq!(analysis.summary.return_count, 2, "{mnemonic}");
            assert_eq!(
                analysis.summary.edge_kind_counts.return_count, 2,
                "{mnemonic}"
            );
            assert_eq!(
                analysis.summary.edge_kind_counts.fallthrough_count, 0,
                "{mnemonic}"
            );
            assert!(analysis.edges.iter().any(|edge| {
                edge.kind == ControlFlowEdgeKind::Return
                    && edge.source_address == 0x1000
                    && edge.via.is_none()
            }));
        }
    }

    #[test]
    fn analysis_treats_terminal_traps_as_boundary_exits() {
        for mnemonic in ["brk", "hlt", "udf"] {
            let instructions = vec![
                instruction(0x1000, mnemonic, vec![]),
                instruction(0x1004, "ret", vec![]),
            ];

            let analysis = analyze_disassembly(
                format!("synthetic-{mnemonic}"),
                0x1000,
                0x1008,
                &instructions,
            );

            assert_eq!(analysis.summary.basic_block_count, 2, "{mnemonic}");
            assert_eq!(analysis.summary.entry_block_count, 2, "{mnemonic}");
            assert_eq!(analysis.summary.exit_block_count, 2, "{mnemonic}");
            assert_eq!(
                analysis.summary.primary_entry_reachable_block_count, 1,
                "{mnemonic}"
            );
            assert_eq!(analysis.summary.disconnected_block_count, 1, "{mnemonic}");
            assert_eq!(
                analysis.summary.edge_kind_counts.fallthrough_count, 0,
                "{mnemonic}"
            );
            assert_eq!(
                analysis.summary.edge_kind_counts.return_count, 1,
                "{mnemonic}"
            );
            assert!(analysis.edges.iter().all(|edge| {
                edge.source_address != 0x1000 || edge.kind != ControlFlowEdgeKind::Fallthrough
            }));
        }
    }

    #[test]
    fn analysis_reports_non_default_return_register() {
        let instructions = vec![instruction(
            0x1000,
            "ret",
            vec![Operand::Register("x8".to_string())],
        )];

        let analysis = analyze_disassembly("synthetic-ret-register", 0x1000, 0x1004, &instructions);

        assert_eq!(analysis.summary.return_count, 1);
        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == ControlFlowEdgeKind::Return
                && edge.source_address == 0x1000
                && edge.via.as_deref() == Some("x8")
        }));
    }

    #[test]
    fn analysis_omits_default_return_register() {
        let instructions = vec![instruction(
            0x1000,
            "ret",
            vec![Operand::Register("x30".to_string())],
        )];

        let analysis = analyze_disassembly("synthetic-ret-default", 0x1000, 0x1004, &instructions);

        assert!(analysis.edges.iter().any(|edge| {
            edge.kind == ControlFlowEdgeKind::Return
                && edge.source_address == 0x1000
                && edge.via.is_none()
        }));
    }

    #[test]
    fn analysis_reports_back_edges_for_local_branch_loops() {
        let instructions = vec![
            instruction(0x1000, "subs", vec![]),
            DecodedInstruction {
                references: vec![Reference::Branch { target: 0x1000 }],
                ..instruction(0x1004, "b.ne", vec![Operand::Label(0x1000)])
            },
            instruction(0x1008, "ret", vec![]),
        ];

        let analysis = analyze_disassembly("synthetic-loop", 0x1000, 0x100c, &instructions);

        assert_eq!(analysis.summary.basic_block_count, 2);
        assert_eq!(analysis.summary.entry_block_count, 1);
        assert_eq!(analysis.summary.primary_entry_reachable_block_count, 2);
        assert_eq!(analysis.summary.disconnected_block_count, 0);
        assert_eq!(analysis.summary.edge_count, 3);
        assert_eq!(analysis.summary.back_edge_count, 1);
        assert_eq!(analysis.summary.loop_header_count, 1);
        assert_eq!(analysis.summary.edge_kind_counts.fallthrough_count, 1);
        assert_eq!(
            analysis.summary.edge_kind_counts.conditional_branch_count,
            1
        );
        assert_eq!(analysis.summary.edge_kind_counts.return_count, 1);
        assert!(analysis.edges.iter().any(ControlFlowEdge::is_back_edge));
    }

    #[test]
    fn analysis_does_not_count_backward_calls_as_back_edges() {
        let instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Call { target: 0x1000 }],
                ..instruction(0x1000, "bl", vec![Operand::Label(0x1000)])
            },
            instruction(0x1004, "ret", vec![]),
        ];

        let analysis =
            analyze_disassembly("synthetic-backward-call", 0x1000, 0x1008, &instructions);

        assert_eq!(analysis.summary.back_edge_count, 0);
        assert_eq!(analysis.summary.loop_header_count, 0);
        assert!(!analysis.edges.iter().any(ControlFlowEdge::is_back_edge));
    }

    #[test]
    fn synthesize_analysis_references_clears_state_at_in_window_call_target_joins() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(0x220),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Call { target: 0x1010 }],
                ..instruction(0x1004, "bl", vec![Operand::Label(0x1010)])
            },
            instruction(0x1008, "nop", vec![]),
            instruction(0x100c, "nop", vec![]),
            instruction(
                0x1010,
                "add",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions
                .iter()
                .find(|instruction| instruction.address == 0x1010)
                .expect("call target instruction")
                .recovered_values
                .is_empty(),
            "callee entry must not inherit caller layout state through x19: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_clears_state_after_drps_boundaries() {
        let image = synthetic_image();
        for mnemonic in ["drps", "eretaa", "eretab", "brk", "hlt", "udf"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x19".to_string()),
                        Operand::ImmediateUnsigned(0x220),
                    ],
                ),
                instruction(0x1004, mnemonic, vec![]),
                instruction(
                    0x1008,
                    "add",
                    vec![
                        Operand::Register("x20".to_string()),
                        Operand::Register("x19".to_string()),
                        Operand::ImmediateUnsigned(4),
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2].recovered_values.is_empty(),
                "instructions after {mnemonic} must not inherit linear register state: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_clears_state_after_indirect_branch_boundaries() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(0x220),
                ],
            ),
            instruction(0x1004, "br", vec![Operand::Register("x8".to_string())]),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "instructions after an indirect branch must not inherit linear state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_clears_state_after_register_conditional_branches() {
        let image = synthetic_image();
        for (mnemonic, operands) in [
            (
                "cbz",
                vec![Operand::Register("x0".to_string()), Operand::Label(0x1010)],
            ),
            ("bc.eq", vec![Operand::Label(0x1010)]),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x19".to_string()),
                        Operand::ImmediateUnsigned(0x220),
                    ],
                ),
                DecodedInstruction {
                    references: vec![Reference::Branch { target: 0x1010 }],
                    ..instruction(0x1004, mnemonic, operands)
                },
                instruction(
                    0x1008,
                    "add",
                    vec![
                        Operand::Register("x20".to_string()),
                        Operand::Register("x19".to_string()),
                        Operand::ImmediateUnsigned(4),
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2].recovered_values.is_empty(),
                "instructions after a conditional branch must not inherit linear state for {mnemonic}: {:?}",
                instructions
            );
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
    fn synthesize_analysis_references_tracks_signed_mov_immediates() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateSigned(-1),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateSigned(-2),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateSigned(42),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.register == "x8"
                && value.value == u64::MAX
                && value.kind == RecoveredValueKind::Literal
                && value.source == RecoveredValueSource::MoveWide
        }));
        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "w9"
                && value.value == u64::from((-2_i32) as u32)
                && value.kind == RecoveredValueKind::Literal
                && value.source == RecoveredValueSource::MoveWide
        }));
        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 42
                && value.kind == RecoveredValueKind::Literal
                && value.source == RecoveredValueSource::MoveWide
        }));
    }

    #[test]
    fn synthesize_analysis_references_does_not_resolve_sentinel_indirect_targets() {
        let image = synthetic_image();
        for (register, operand, target) in [
            ("x8", Operand::ImmediateUnsigned(0), 0),
            ("x8", Operand::ImmediateSigned(-1), u64::MAX),
            ("x8", Operand::ImmediateSigned(-2), u64::MAX - 1),
            ("w8", Operand::ImmediateSigned(-1), u64::from(u32::MAX)),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![Operand::Register(register.to_string()), operand],
                ),
                instruction(0x1004, "br", vec![Operand::Register("x8".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[0]
                    .recovered_values
                    .iter()
                    .any(|value| value.value == target),
                "sentinel constants should still be visible as recovered values for {register}: {:?}",
                instructions
            );
            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::IndirectBranch {
                        via: "x8".to_string()
                    }),
                "sentinel constants should still produce indirect control-flow references: {:?}",
                instructions
            );
            assert!(
                !instructions[1]
                    .references
                    .contains(&Reference::Branch { target }),
                "sentinel constants must not fabricate concrete branch targets: {:?}",
                instructions
            );
            assert!(
                !instructions[1].annotations.iter().any(|annotation| {
                    matches!(annotation, Annotation::IndirectTargetResolved { target: existing, .. } if *existing == target)
                }),
                "sentinel constants must not fabricate resolved-target annotations: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_resolves_high_half_executable_register_targets() {
        let image = synthetic_high_half_executable_image();
        let target = 0xffff_ffff_ffff_f010;
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(target),
                ],
            ),
            instruction(0x1004, "br", vec![Operand::Register("x8".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Branch { target }),
            "mapped high-half executable addresses must resolve through register state: {:?}",
            instructions[1].references
        );
        assert!(
            instructions[1].annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::IndirectTargetResolved {
                        via,
                        target: existing,
                        reason: IndirectTargetReason::RegisterState,
                    } if via == "x8" && *existing == target
                )
            }),
            "high-half register targets should keep a resolved-target annotation: {:?}",
            instructions[1].annotations
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_fp_alias() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x29".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "add",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::Register("fp".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "x0"
                && value.value == 0x3004
                && value.source == RecoveredValueSource::AdrpAdd
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_lr_alias() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x30".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(0x1004, "blr", vec![Operand::Register("lr".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Call { target: 0x1234 }),
            "lr must resolve through the same state key as x30: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_ip_aliases() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x16".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(0x1004, "blr", vec![Operand::Register("ip0".to_string())]),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("ip1".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x17".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Call { target: 0x1234 }),
            "ip0 must resolve through the same state key as x16: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "ip1 writes must resolve through the same state key as x17: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_reads_zero_register_as_literal_zero() {
        let image = synthetic_image();
        let mut instructions = vec![instruction(
            0x1000,
            "mov",
            vec![
                Operand::Register("x8".to_string()),
                Operand::Register("xzr".to_string()),
            ],
        )];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.register == "x8"
                && value.value == 0
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_logical_immediate_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0b1010),
                ],
            ),
            instruction(
                0x1004,
                "orr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0b0101),
                ],
            ),
            instruction(
                0x1008,
                "eor",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0b0011),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "x9"
                && value.value == 0b1111
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0b1100
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_logical_register_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0b1100),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0b1010),
                ],
            ),
            instruction(
                0x1008,
                "and",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "orr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Register("xzr".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "eor",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0b1000
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "x11"
                && value.value == 0b1000
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 0b0110
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_logical_alias_and_shifted_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0b1100),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0b1010),
                ],
            ),
            instruction(
                0x1008,
                "bic",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "orn",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "eon",
                vec![
                    Operand::Register("w12".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "orr",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("xzr".to_string()),
                    Operand::ShiftedRegister {
                        register: "x8".to_string(),
                        shift: "lsl #3".to_string(),
                    },
                ],
            ),
            instruction(
                0x1018,
                "mvn",
                vec![
                    Operand::Register("w14".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0b0100
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "w11"
                && value.value == u64::from(0b1100_u32 | !0b1010_u32)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "w12"
                && value.value == u64::from(0b1100_u32 ^ !0b1010_u32)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 0b1100000
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[6].recovered_values.iter().any(|value| {
            value.register == "w14"
                && value.value == u64::from(!0b1010_u32)
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_unary_extend_and_neg_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0xffff_ff80),
                ],
            ),
            instruction(
                0x1004,
                "sxtb",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("w8".to_string()),
                ],
            ),
            instruction(
                0x1008,
                "uxtb",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w8".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "sxth",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("w8".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "uxtw",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("w8".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "neg",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x10".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "x9"
                && value.value == ((-128_i64) as u64)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0x80
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "w11"
                && value.value == 0xffff_ff80
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 0xffff_ff80
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 0_u64.wrapping_sub(0x80)
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_tracks_flag_setting_register_arithmetic() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "adds",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "subs",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0x3008
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "x11"
                && value.value == 0x3000
                && value.source == RecoveredValueSource::Other
        }));
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 })
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_carry_register_arithmetic() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0xffff),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "adc",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "sbc",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "ngc",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x1_0000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-carry adc should add the carry-in bit: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x11"
                    && value.value == 0_u64.wrapping_sub(0xffff)
                    && value.source == RecoveredValueSource::Other
            }),
            "known-carry sbc should subtract without an extra borrow: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 0_u64.wrapping_sub(0xffff)
                    && value.source == RecoveredValueSource::Other
            }),
            "known-carry ngc should negate without an extra borrow: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_carry_arithmetic_flags() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(u64::from(u32::MAX)),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "adcs",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "cset",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Condition("cs".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "cset",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "w10"
                    && value.value == 0
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit adcs should wrap at W-register width: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x11"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "adcs carry-out should feed later condition recovery: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "adcs zero result should feed later condition recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_carry_arithmetic_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "adc",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "known-carry adc table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "known-carry adc table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "known-carry adc table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_carry_arithmetic_with_unknown_flags() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "adc",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("xzr".to_string()),
                    Operand::Register("xzr".to_string()),
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.is_empty(),
            "carry arithmetic must not synthesize values with unknown carry flags"
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "carry arithmetic with unknown flags must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_width_mismatched_carry_arithmetic_sources() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "adc",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("xzr".to_string()),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "carry arithmetic must reject source registers that do not match destination width"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "invalid carry arithmetic must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_multiply_register_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(6),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(7),
                ],
            ),
            instruction(
                0x1008,
                "mul",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "madd",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "msub",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "mneg",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 42
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "x11"
                && value.value == 84
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 42
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 0_u64.wrapping_sub(42)
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_normalizes_multiply_widths() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(u64::from(u32::MAX)),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
            instruction(
                0x1008,
                "mul",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "w10"
                    && value.value == 0xffff_fffe
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit multiply results should wrap at W-register width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_multiply_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x100c,
                "mul",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 8
                    && value.source == RecoveredValueSource::Other
            }),
            "multiply-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "multiply-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "multiply-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_madd_for_table_addresses() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x100c,
                "madd",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x12".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x4008
                    && value.source == RecoveredValueSource::Other
            }),
            "madd-derived table addresses should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "madd-derived table addresses should resolve slot loads: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "madd-derived table addresses should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unknown_multiply_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "mul",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x12".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unknown multiply operands must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown multiply operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_long_and_high_multiply_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0xffff_fffe),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(3),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::ImmediateUnsigned(10),
                ],
            ),
            instruction(
                0x100c,
                "umaddl",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("x20".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "smaddl",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("x20".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "umsubl",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("x20".to_string()),
                ],
            ),
            instruction(
                0x1018,
                "smsubl",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("x20".to_string()),
                ],
            ),
            instruction(
                0x101c,
                "umull",
                vec![
                    Operand::Register("x14".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1020,
                "smull",
                vec![
                    Operand::Register("x15".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1024,
                "umnegl",
                vec![
                    Operand::Register("x16".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x1028,
                "smnegl",
                vec![
                    Operand::Register("x17".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x102c,
                "mov",
                vec![
                    Operand::Register("x18".to_string()),
                    Operand::ImmediateUnsigned(0x8000_0000_0000_0000),
                ],
            ),
            instruction(
                0x1030,
                "mov",
                vec![
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
            instruction(
                0x1034,
                "umulh",
                vec![
                    Operand::Register("x21".to_string()),
                    Operand::Register("x18".to_string()),
                    Operand::Register("x19".to_string()),
                ],
            ),
            instruction(
                0x1038,
                "mov",
                vec![
                    Operand::Register("x22".to_string()),
                    Operand::ImmediateUnsigned((-2_i64) as u64),
                ],
            ),
            instruction(
                0x103c,
                "mov",
                vec![
                    Operand::Register("x23".to_string()),
                    Operand::ImmediateUnsigned(0x4000_0000_0000_0000),
                ],
            ),
            instruction(
                0x1040,
                "smulh",
                vec![
                    Operand::Register("x24".to_string()),
                    Operand::Register("x22".to_string()),
                    Operand::Register("x23".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 0x3_0000_0004
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x11"
                && value.value == 4
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 10_u64.wrapping_sub(0x2_ffff_fffa)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[6].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 16
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[7].recovered_values.iter().any(|value| {
            value.register == "x14"
                && value.value == 9
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[8].recovered_values.iter().any(|value| {
            value.register == "x15"
                && value.value == ((-6_i64) as u64)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[9].recovered_values.iter().any(|value| {
            value.register == "x16"
                && value.value == 0_u64.wrapping_sub(9)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[10].recovered_values.iter().any(|value| {
            value.register == "x17"
                && value.value == 6
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[13].recovered_values.iter().any(|value| {
            value.register == "x21"
                && value.value == 1
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[16].recovered_values.iter().any(|value| {
            value.register == "x24"
                && value.value == u64::MAX
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_uses_long_multiply_for_table_addresses() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x100c,
                "umaddl",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("w11".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x12".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x4008
                    && value.source == RecoveredValueSource::Other
            }),
            "long multiply-add table addresses should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "loads through long multiply-add table addresses should resolve data refs: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "long multiply-add table addresses should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_high_multiply_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x8000_0000_0000_0000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
            instruction(
                0x100c,
                "umulh",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "high-multiply-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "high-multiply-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "high-multiply-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unknown_long_multiply_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x100c,
                "umaddl",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("w12".to_string()),
                    Operand::Register("w11".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "unknown long multiply operands must not synthesize values"
        );
        assert!(
            !instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown long multiply operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_bitfield_insert_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0xabcd),
                ],
            ),
            instruction(
                0x1004,
                "ubfiz",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::ImmediateUnsigned(0xff),
                ],
            ),
            instruction(
                0x100c,
                "sbfiz",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("w10".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1010,
                "mov",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned(0xffff_0000),
                ],
            ),
            instruction(
                0x1014,
                "mov",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::ImmediateUnsigned(0xabc),
                ],
            ),
            instruction(
                0x1018,
                "bfi",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x13".to_string()),
                    Operand::ImmediateUnsigned(8),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x101c,
                "bfxil",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x13".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1020,
                "bfc",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned(8),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "w9"
                && value.value == 0xcd0
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "w11"
                && value.value == 0xffff_fff0
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[6].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 0xffff_bc00
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[7].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 0xffff_bcab
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[8].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 0xffff_00ab
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_uses_bitfield_insert_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "bfi",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "bitfield-insert-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "bitfield-insert-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "bitfield-insert-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_bitfield_insert_ranges() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "ubfiz",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(63),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "out-of-range bitfield inserts must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "out-of-range bitfield inserts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unknown_bitfield_insert_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "bfi",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned(0),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unknown bitfield insert operands must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown bitfield insert operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_divide_register_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(42),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(6),
                ],
            ),
            instruction(
                0x1008,
                "udiv",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned((-42_i64) as u64),
                ],
            ),
            instruction(
                0x1010,
                "sdiv",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x11".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "udiv",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("xzr".to_string()),
                ],
            ),
            instruction(
                0x1018,
                "sdiv",
                vec![
                    Operand::Register("x14".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("xzr".to_string()),
                ],
            ),
            instruction(
                0x101c,
                "mov",
                vec![
                    Operand::Register("w15".to_string()),
                    Operand::ImmediateUnsigned(0x8000_0000),
                ],
            ),
            instruction(
                0x1020,
                "mov",
                vec![
                    Operand::Register("w16".to_string()),
                    Operand::ImmediateUnsigned(u64::from(u32::MAX)),
                ],
            ),
            instruction(
                0x1024,
                "sdiv",
                vec![
                    Operand::Register("w17".to_string()),
                    Operand::Register("w15".to_string()),
                    Operand::Register("w16".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 7
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == ((-7_i64) as u64)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 0
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[6].recovered_values.iter().any(|value| {
            value.register == "x14"
                && value.value == 0
                && value.source == RecoveredValueSource::Other
        }));
        assert!(
            instructions[9].recovered_values.iter().any(|value| {
                value.register == "w17"
                    && value.value == 0x8000_0000
                    && value.source == RecoveredValueSource::Other
            }),
            "signed division overflow should keep Arm's low-result bits at W width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_divide_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(16),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(16),
                ],
            ),
            instruction(
                0x100c,
                "udiv",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "divide-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "divide-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "divide-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unknown_divide_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "udiv",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x12".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unknown divide operands must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown divide operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_bit_count_and_reverse_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x10),
                ],
            ),
            instruction(
                0x1004,
                "clz",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1008,
                "cls",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned((-16_i64) as u64),
                ],
            ),
            instruction(
                0x1010,
                "cls",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "rbit",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1018,
                "mov",
                vec![
                    Operand::Register("w14".to_string()),
                    Operand::ImmediateUnsigned(0x1122_3344),
                ],
            ),
            instruction(
                0x101c,
                "rev",
                vec![
                    Operand::Register("w15".to_string()),
                    Operand::Register("w14".to_string()),
                ],
            ),
            instruction(
                0x1020,
                "rev16",
                vec![
                    Operand::Register("w16".to_string()),
                    Operand::Register("w14".to_string()),
                ],
            ),
            instruction(
                0x1024,
                "mov",
                vec![
                    Operand::Register("x17".to_string()),
                    Operand::ImmediateUnsigned(0x1122_3344_5566_7788),
                ],
            ),
            instruction(
                0x1028,
                "rev32",
                vec![
                    Operand::Register("x18".to_string()),
                    Operand::Register("x17".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "x9"
                && value.value == 59
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == 58
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[4].recovered_values.iter().any(|value| {
            value.register == "x12"
                && value.value == 59
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[5].recovered_values.iter().any(|value| {
            value.register == "x13"
                && value.value == 0x0800_0000_0000_0000
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[7].recovered_values.iter().any(|value| {
            value.register == "w15"
                && value.value == 0x4433_2211
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[8].recovered_values.iter().any(|value| {
            value.register == "w16"
                && value.value == 0x2211_4433
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[10].recovered_values.iter().any(|value| {
            value.register == "x18"
                && value.value == 0x4433_2211_8877_6655
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_uses_bit_transforms_for_table_indices() {
        for (mnemonic, source) in [("clz", 0x4000_0000_u64), ("rev16", 0x0100)] {
            let image = synthetic_table_image(0x3000);
            let mut instructions = vec![
                DecodedInstruction {
                    references: vec![Reference::Page { target: 0x4000 }],
                    ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
                },
                instruction(
                    0x1004,
                    "mov",
                    vec![
                        Operand::Register("w9".to_string()),
                        Operand::ImmediateUnsigned(source),
                    ],
                ),
                instruction(
                    0x1008,
                    mnemonic,
                    vec![
                        Operand::Register("w9".to_string()),
                        Operand::Register("w9".to_string()),
                    ],
                ),
                instruction(
                    0x100c,
                    "ldr",
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: Some("x9".to_string()),
                            index_modifier: Some("lsl #3".to_string()),
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2].recovered_values.iter().any(|value| {
                    value.register == "w9"
                        && value.value == 1
                        && value.source == RecoveredValueSource::Other
                }),
                "{mnemonic}-derived table indices should stay known: {:?}",
                instructions
            );
            assert!(
                instructions[3]
                    .references
                    .contains(&Reference::Data { target: 0x4008 }),
                "{mnemonic}-derived table indices should resolve slot addresses: {:?}",
                instructions
            );
            assert!(
                instructions[4]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 }),
                "{mnemonic}-derived table indices should resolve indirect branches: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unknown_bit_transform_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "rbit",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x12".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unknown bit-transform operands must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown bit-transform operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_bitfield_extract_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0xabcd),
                ],
            ),
            instruction(
                0x1004,
                "ubfx",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "sbfx",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x100c,
                "sbfx",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.register == "x9"
                && value.value == 0xbc
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.register == "x10"
                && value.value == ((-68_i64) as u64)
                && value.source == RecoveredValueSource::Other
        }));
        assert!(instructions[3].recovered_values.iter().any(|value| {
            value.register == "w11"
                && value.value == 0xffff_ffbc
                && value.source == RecoveredValueSource::Other
        }));
    }

    #[test]
    fn synthesize_analysis_references_uses_bitfield_extract_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x10),
                ],
            ),
            instruction(
                0x1008,
                "ubfx",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "bitfield-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "bitfield-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "bitfield-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_bitfield_extract_ranges() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "ubfx",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(63),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "out-of-range bitfield extracts must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "out-of-range bitfield extracts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_rotate_and_extract_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x10),
                ],
            ),
            instruction(
                0x1004,
                "ror",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0xa),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "extr",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x1014,
                "extr",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "ror should recover known rotate-immediate values: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 0xa000_0000_0000_0000
                    && value.source == RecoveredValueSource::Other
            }),
            "extr should combine operand1:operand2 before extracting: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x13"
                    && value.value == 0x10
                    && value.source == RecoveredValueSource::Other
            }),
            "extr with shift zero should recover the low source register: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_rotate_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x10),
                ],
            ),
            instruction(
                0x1008,
                "ror",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "rotate-derived table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "rotate-derived table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "rotate-derived table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_extract_shifts() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "extr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(64),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid extr shifts must not synthesize values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "invalid extr shifts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_extended_aliases_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "uxtw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("w9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "standalone uxtw aliases should keep table indices known: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "extended alias index should resolve the table slot address: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "extended alias index should resolve the indirect branch: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_shifted_logical_operands() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "orr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("xzr".to_string()),
                    Operand::ShiftedRegister {
                        register: "x9".to_string(),
                        shift: "lsl #64".to_string(),
                    },
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid shifted logical operands must not synthesize wrapped values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "invalid shifted logical operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_shift_immediate_values() {
        let image = synthetic_image();
        for (mnemonic, source, shift, expected) in [
            ("lsl", 0x3, 2, 0xc),
            ("lsr", 0x40, 2, 0x10),
            ("asr", (-16_i64) as u64, 2, (-4_i64) as u64),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(source),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(shift),
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1].recovered_values.iter().any(|value| {
                    value.register == "x9"
                        && value.value == expected
                        && value.source == RecoveredValueSource::Other
                }),
                "immediate shift should recover {expected:#x} for {mnemonic}: {:?}",
                instructions
            );
        }

        let mut instructions = vec![
            instruction(
                0x2000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0xfffffff0),
                ],
            ),
            instruction(
                0x2004,
                "asr",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(2),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "w9"
                    && value.value == 0xfffffffc
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit arithmetic shifts should sign-extend within W-register width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_register_shift_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(65),
                ],
            ),
            instruction(
                0x1008,
                "lsl",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "lsrv",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "mov",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned((-4_i64) as u64),
                ],
            ),
            instruction(
                0x1014,
                "asrv",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x12".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(
                0x1018,
                "mov",
                vec![
                    Operand::Register("w14".to_string()),
                    Operand::ImmediateUnsigned(0x8000_0001),
                ],
            ),
            instruction(
                0x101c,
                "mov",
                vec![
                    Operand::Register("w15".to_string()),
                    Operand::ImmediateUnsigned(33),
                ],
            ),
            instruction(
                0x1020,
                "rorv",
                vec![
                    Operand::Register("w16".to_string()),
                    Operand::Register("w14".to_string()),
                    Operand::Register("w15".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 2
                    && value.source == RecoveredValueSource::Other
            }),
            "register-count lsl should use count modulo X-register width: {:?}",
            instructions
        );
        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x11"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "explicit lsrv should recover known register-count shifts: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x13"
                    && value.value == ((-2_i64) as u64)
                    && value.source == RecoveredValueSource::Other
            }),
            "asrv should sign-extend after applying a modulo register count: {:?}",
            instructions
        );
        assert!(
            instructions[8].recovered_values.iter().any(|value| {
                value.register == "w16"
                    && value.value == 0xc000_0000
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit rorv should use count modulo W-register width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_register_shifts_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(3),
                ],
            ),
            instruction(
                0x100c,
                "lsrv",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x11".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "register-shifted table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "register-shifted table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "register-shifted table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_register_shifts_with_unknown_counts() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "lslv",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "register shifts must not synthesize values with unknown shift counts"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "register shifts with unknown counts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_width_mismatched_register_shift_sources() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "lslv",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "register shifts must reject source registers that do not match destination width"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "invalid register shifts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_shifted_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "lsl",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(3),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 8
                    && value.source == RecoveredValueSource::Other
            }),
            "the shifted index register should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "the load should use the shifted byte offset: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "the shifted index should resolve the indirect branch: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_shifted_register_add_table_addresses() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ShiftedRegister {
                        register: "x9".to_string(),
                        shift: "lsl #3".to_string(),
                    },
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x4008
                    && value.source == RecoveredValueSource::Other
            }),
            "shifted register add should recover the precomputed table address: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "loads through precomputed shifted table addresses should recover data refs: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "precomputed shifted table addresses should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_shifted_add_immediate_table_addresses() {
        let mut bytes = vec![0u8; 8];
        bytes.copy_from_slice(&0x3000_u64.to_le_bytes());
        let image = synthetic_literal_image(0x5000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                    Operand::Other("lsl #12".to_string()),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x5000
                    && value.source == RecoveredValueSource::AdrpAdd
            }),
            "shifted add immediate should recover the scaled address: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x5000 }),
            "loads through shifted add-immediate addresses should use the scaled target: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "shifted add-immediate table addresses should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_shifted_sub_immediate_table_addresses() {
        let mut bytes = vec![0u8; 8];
        bytes.copy_from_slice(&0x3000_u64.to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x5000),
                ],
            ),
            instruction(
                0x1004,
                "sub",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                    Operand::Other("lsl #12".to_string()),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x4000
                    && value.source == RecoveredValueSource::AdrpAdd
            }),
            "shifted sub immediate should recover the scaled address: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4000 }),
            "loads through shifted sub-immediate addresses should use the scaled target: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "shifted sub-immediate table addresses should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_add_immediate_shifts() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                    Operand::Other("lsl #1".to_string()),
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid add-immediate shifts must not synthesize values"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "invalid add-immediate shifts must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_tracks_signed_extended_register_add_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4010),
                ],
            ),
            instruction(
                0x1004,
                "movn",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ShiftedRegister {
                        register: "w9".to_string(),
                        shift: "sxtw #3".to_string(),
                    },
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x4008
                    && value.source == RecoveredValueSource::Other
            }),
            "signed extended register adds should subtract negative scaled offsets: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "signed extended register add offsets should still resolve table targets: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_deterministic_conditional_selects() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "csel",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "csel with equal source values is deterministic despite an unknown condition: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "deterministic csel values should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_conditional_selects_from_known_flags() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(5),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(5),
                ],
            ),
            instruction(
                0x1010,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("ne".to_string()),
                ],
            ),
            instruction(0x1018, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known equal flags should select the true csel source: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 0x4000
                    && value.source == RecoveredValueSource::Other
            }),
            "known equal flags should reject ne and select the false csel source: {:?}",
            instructions
        );
        assert!(
            instructions[6]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "known-flag csel values should feed indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_known_selects_with_unknown_unchosen_arm() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x12".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x12".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("ne".to_string()),
                ],
            ),
            instruction(0x1018, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true csel should not require the unknown false source: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x13"
                    && value.value == 0x4000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-false csel should not require the unknown true source: {:?}",
            instructions
        );
        assert!(
            instructions[6]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "known select with an unknown unchosen arm should still feed branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_known_select_variant_false_arms() {
        let cases = [
            ("csinc", 0x2fff, 0x3000, "increment"),
            ("csinv", !0x3000_u64, 0x3000, "invert"),
            ("csneg", 0_u64.wrapping_sub(0x3000), 0x3000, "negate"),
        ];

        for (mnemonic, false_input, expected, operation) in cases {
            let image = synthetic_image();
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x5000),
                    ],
                ),
                instruction(
                    0x1004,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(false_input),
                    ],
                ),
                instruction(
                    0x1008,
                    "mov",
                    vec![
                        Operand::Register("x11".to_string()),
                        Operand::ImmediateUnsigned(1),
                    ],
                ),
                instruction(
                    0x100c,
                    "cmp",
                    vec![
                        Operand::Register("x11".to_string()),
                        Operand::ImmediateUnsigned(2),
                    ],
                ),
                instruction(
                    0x1010,
                    mnemonic,
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                        Operand::Condition("eq".to_string()),
                    ],
                ),
                instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[4].recovered_values.iter().any(|value| {
                    value.register == "x10"
                        && value.value == expected
                        && value.source == RecoveredValueSource::Other
                }),
                "{mnemonic} should recover the false arm after known-false eq and {operation}: {:?}",
                instructions
            );
            assert!(
                instructions[5]
                    .references
                    .contains(&Reference::Branch { target: expected }),
                "{mnemonic} known-flag false arm should feed indirect branches: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_recovers_conditional_set_alias_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1004,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cset",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x100c,
                "csetm",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Condition("ne".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "csetm",
                vec![
                    Operand::Register("w12".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "w9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true cset should recover 1: {:?}",
            instructions
        );
        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0
                    && value.source == RecoveredValueSource::Other
            }),
            "known-false csetm should recover 0: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "w12"
                    && value.value == u64::from(u32::MAX)
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit known-true csetm should recover an all-ones W mask: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_conditional_transform_alias_values() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x2fff),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("w14".to_string()),
                    Operand::ImmediateUnsigned(u64::from(u32::MAX)),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1014,
                "cinc",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1018,
                "cinv",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("ne".to_string()),
                ],
            ),
            instruction(
                0x101c,
                "cneg",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1020,
                "cinc",
                vec![
                    Operand::Register("w15".to_string()),
                    Operand::Register("w14".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true cinc should increment its source: {:?}",
            instructions
        );
        assert!(
            instructions[6].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-false cinv should leave its source unchanged: {:?}",
            instructions
        );
        assert!(
            instructions[7].recovered_values.iter().any(|value| {
                value.register == "x13"
                    && value.value == 0_u64.wrapping_sub(0x3000)
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true cneg should negate its source: {:?}",
            instructions
        );
        assert!(
            instructions[8].recovered_values.iter().any(|value| {
                value.register == "w15"
                    && value.value == 0
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit known-true cinc should wrap at W-register width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_conditional_set_aliases_for_table_indices() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "cset",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::Other
            }),
            "known cset table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "known cset table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "known cset table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_selects_after_true_conditional_compares() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(5),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(5),
                ],
            ),
            instruction(
                0x1010,
                "ccmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(4),
                    Operand::ImmediateUnsigned(0),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("ne".to_string()),
                ],
            ),
            instruction(0x1018, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true ccmp should replace flags with the comparison result: {:?}",
            instructions
        );
        assert!(
            instructions[6]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "ccmp-derived flags should feed indirect branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_selects_after_false_conditional_compare_fallbacks() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "ccmp",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned(31),
                    Operand::ImmediateUnsigned(0b0100),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1018, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-false ccmp should use the NZCV fallback without reading compare operands: {:?}",
            instructions
        );
        assert!(
            instructions[6]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "fallback flags should feed indirect branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_selects_after_conditional_compare_negative() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(u64::MAX),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::Register("x8".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "ccmn",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(1),
                    Operand::ImmediateUnsigned(0),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1018, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "known-true ccmn should replace flags with the addition result: {:?}",
            instructions
        );
        assert!(
            instructions[6]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "ccmn-derived flags should feed indirect branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_prewrite_operands_for_flagged_self_updates() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x21".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x100c,
                "subs",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1010,
                "csel",
                vec![
                    Operand::Register("x22".to_string()),
                    Operand::Register("x20".to_string()),
                    Operand::Register("x21".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x22".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "x8"
                    && value.value == 0
                    && value.source == RecoveredValueSource::AdrpAdd
            }),
            "subs should still recover its written destination value: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x22"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::Other
            }),
            "flags must be computed from the original self-update operand: {:?}",
            instructions
        );
        assert!(
            instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "self-update flags should feed indirect branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_deterministic_conditional_select_variants() {
        let cases = [
            ("csinc", 0x2fff, 0x3000, "increment"),
            ("csinv", !0x3000_u64, 0x3000, "invert"),
            ("csneg", 0_u64.wrapping_sub(0x3000), 0x3000, "negate"),
        ];

        for (mnemonic, false_input, expected, operation) in cases {
            let image = synthetic_image();
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(expected),
                    ],
                ),
                instruction(
                    0x1004,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(false_input),
                    ],
                ),
                instruction(
                    0x1008,
                    mnemonic,
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                        Operand::Condition("eq".to_string()),
                    ],
                ),
                instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2].recovered_values.iter().any(|value| {
                    value.register == "x10"
                        && value.value == expected
                        && value.source == RecoveredValueSource::Other
                }),
                "{mnemonic} should recover when the false arm after {operation} matches the true arm: {:?}",
                instructions
            );
            assert!(
                instructions[3]
                    .references
                    .contains(&Reference::Branch { target: expected }),
                "{mnemonic} deterministic values should feed indirect branches: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_normalizes_conditional_select_variant_widths() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(u64::from(u32::MAX)),
                ],
            ),
            instruction(
                0x1008,
                "csinc",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "w10"
                    && value.value == 0
                    && value.source == RecoveredValueSource::Other
            }),
            "32-bit csinc should wrap and compare at W-register width: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_divergent_conditional_select_variants() {
        for mnemonic in ["csinc", "csinv", "csneg"] {
            let image = synthetic_image();
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x3000),
                    ],
                ),
                instruction(
                    0x1008,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x100c,
                    mnemonic,
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                        Operand::Condition("eq".to_string()),
                    ],
                ),
                instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[3].recovered_values.is_empty(),
                "{mnemonic} with different possible values must not guess a condition outcome"
            );
            assert!(
                !instructions[4]
                    .references
                    .contains(&Reference::Branch { target: 0x1234 })
                    && !instructions[4]
                        .references
                        .contains(&Reference::Branch { target: 0x3000 }),
                "divergent {mnemonic} must clear stale concrete targets: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_divergent_conditional_selects() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x100c,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "csel with different source values must not guess a condition outcome"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "divergent csel must clear stale destination state: {:?}",
            instructions
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 })
                && !instructions[4]
                    .references
                    .contains(&Reference::Branch { target: 0x4000 }),
            "divergent csel must not fabricate either possible target: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_clears_known_flags_after_unknown_flag_setter() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1014,
                "cmp",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1018,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x101c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[6].recovered_values.is_empty(),
            "unknown later compare must clear known flags before csel: {:?}",
            instructions
        );
        assert!(
            !instructions[7]
                .references
                .contains(&Reference::Branch { target: 0x1234 })
                && !instructions[7]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 })
                && !instructions[7]
                    .references
                    .contains(&Reference::Branch { target: 0x4000 }),
            "unknown flag setters must not leave stale or fabricated csel branch targets: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_does_not_use_conditional_compare_fallback_with_unknown_flags()
    {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x100c,
                "ccmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(31),
                    Operand::ImmediateUnsigned(0b0100),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x1010,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4].recovered_values.is_empty(),
            "ccmp fallback is only valid after a known input condition: {:?}",
            instructions
        );
        assert!(
            !instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x1234 })
                && !instructions[5]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 })
                && !instructions[5]
                    .references
                    .contains(&Reference::Branch { target: 0x4000 }),
            "unknown ccmp input flags must clear stale and fabricated csel targets: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_conditional_aliases_with_unknown_flags() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "cset",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.is_empty(),
            "conditional aliases must not synthesize values with unknown flags"
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "conditional aliases with unknown flags must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_width_mismatched_conditional_alias_sources() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "cmp",
                vec![
                    Operand::Register("w8".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "cinc",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("w8".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "conditional aliases must reject source registers that do not match destination width"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "invalid conditional aliases must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_width_mismatched_conditional_compare_sources() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1010,
                "mov",
                vec![
                    Operand::Register("w12".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1014,
                "cmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1018,
                "ccmp",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("w12".to_string()),
                    Operand::ImmediateUnsigned(0),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(
                0x101c,
                "csel",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
            instruction(0x1020, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[7].recovered_values.is_empty(),
            "width-mismatched ccmp sources must clear flags instead of comparing through aliases: {:?}",
            instructions
        );
        assert!(
            !instructions[8]
                .references
                .contains(&Reference::Branch { target: 0x1234 })
                && !instructions[8]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 })
                && !instructions[8]
                    .references
                    .contains(&Reference::Branch { target: 0x4000 }),
            "invalid ccmp source width must not leave stale or fabricated csel targets: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_clears_known_flags_across_calls() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x21".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x22".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "cmp",
                vec![
                    Operand::Register("x22".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(0x1010, "bl", vec![Operand::Label(0x2000)]),
            instruction(
                0x1014,
                "csel",
                vec![
                    Operand::Register("x23".to_string()),
                    Operand::Register("x20".to_string()),
                    Operand::Register("x21".to_string()),
                    Operand::Condition("eq".to_string()),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[5].recovered_values.is_empty(),
            "calls must clear known flags even when callee-saved source registers survive: {:?}",
            instructions
        );
    }

    #[test]
    fn condition_holds_evaluates_aarch64_conditions() {
        let greater_unsigned_and_signed = NzcvFlags {
            n: false,
            z: false,
            c: true,
            v: false,
        };
        assert_eq!(
            condition_holds("hi", greater_unsigned_and_signed),
            Some(true)
        );
        assert_eq!(
            condition_holds("ls", greater_unsigned_and_signed),
            Some(false)
        );
        assert_eq!(
            condition_holds("gt", greater_unsigned_and_signed),
            Some(true)
        );
        assert_eq!(
            condition_holds("le", greater_unsigned_and_signed),
            Some(false)
        );

        let signed_less = NzcvFlags {
            n: true,
            z: false,
            c: false,
            v: false,
        };
        assert_eq!(condition_holds("lt", signed_less), Some(true));
        assert_eq!(condition_holds("ge", signed_less), Some(false));
        assert_eq!(condition_holds(".mi", signed_less), Some(true));
        assert_eq!(condition_holds("al", signed_less), Some(true));
        assert_eq!(condition_holds("nv", signed_less), None);
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_register_add_shifts() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x4008),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1004, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x100c,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ShiftedRegister {
                        register: "x9".to_string(),
                        shift: "lsl #64".to_string(),
                    },
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x10".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1014, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "invalid shifted register add operands must not synthesize a value"
        );
        assert!(
            !instructions[5]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "invalid shifted register add operands must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_shift_immediates() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "lsl",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(64),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid shifts must not synthesize wrapped register values"
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "invalid shifted indexes must not resolve through stale register state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_does_not_emit_zero_register_writes() {
        let image = synthetic_image();
        let mut instructions = vec![instruction(
            0x1000,
            "mov",
            vec![
                Operand::Register("xzr".to_string()),
                Operand::ImmediateUnsigned(0x1234),
            ],
        )];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "writes to xzr/wzr are sinks and must not appear as recovered state"
        );
    }

    #[test]
    fn synthesize_analysis_references_ignores_invalid_mov_wide_shifts() {
        let image = synthetic_image();
        let mut instructions = vec![instruction(
            0x1000,
            "movz",
            vec![
                Operand::Register("x0".to_string()),
                Operand::ImmediateUnsigned(1),
                Operand::ImmediateUnsigned(64),
            ],
        )];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "invalid mov-wide shifts must not panic or synthesize misleading values"
        );
    }

    #[test]
    fn synthesize_analysis_references_ignores_width_invalid_mov_wide_shifts() {
        let image = synthetic_image();
        let mut instructions = vec![instruction(
            0x1000,
            "movz",
            vec![
                Operand::Register("w0".to_string()),
                Operand::ImmediateUnsigned(1),
                Operand::ImmediateUnsigned(32),
            ],
        )];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "32-bit mov-wide destinations only support 0/16-bit shifts"
        );
    }

    #[test]
    fn synthesize_analysis_references_ignores_malformed_mov_wide_shift_text() {
        let image = synthetic_image();
        for shift in ["lsl #-16", "ror #16", "lsl #16 trailing"] {
            let mut instructions = vec![instruction(
                0x1000,
                "movz",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(1),
                    Operand::Other(shift.to_string()),
                ],
            )];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[0].recovered_values.is_empty(),
                "malformed shift text must not be scraped into a valid mov-wide lane: {shift}"
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_call_clobbered_registers() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x120),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(0x220),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Call { target: 0x3000 }],
                ..instruction(0x1008, "bl", vec![Operand::Label(0x3000)])
            },
            instruction(
                0x100c,
                "add",
                vec![
                    Operand::Register("x1".to_string()),
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x1010,
                "add",
                vec![
                    Operand::Register("x20".to_string()),
                    Operand::Register("x19".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3]
                .recovered_values
                .iter()
                .all(|value| value.register != "x1"),
            "x0 is call-clobbered and must not flow past bl"
        );
        assert!(
            instructions[4]
                .recovered_values
                .iter()
                .any(|value| { value.register == "x20" && value.value == 0x224 })
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_negative_sub_immediates() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "sub",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateSigned(-4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.is_empty(),
            "negative sub immediates are malformed and must not synthesize values"
        );
        assert!(
            !instructions[1]
                .references
                .contains(&Reference::Data { target: 0x3004 }),
            "negative sub immediates must not be flipped into positive data references"
        );
    }

    #[test]
    fn synthesize_analysis_references_ignores_zero_destination_add_immediates() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "subs",
                vec![
                    Operand::Register("xzr".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.is_empty(),
            "zero-destination add/sub forms discard their result and must not synthesize values"
        );
        assert!(
            !instructions[1]
                .references
                .contains(&Reference::Data { target: 0x2ffc }),
            "zero-destination compare forms must not produce synthetic data references"
        );
    }

    #[test]
    fn synthesize_analysis_references_preserves_register_values_across_stores() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "str",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::Memory {
                        base: "sp".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "blr", vec![Operand::Register("x8".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Call { target: 0x1234 }),
            "store source registers must keep their recovered values: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_emits_data_refs_for_prefetch_operands() {
        let image = synthetic_image();
        for mnemonic in ["prfm", "prfum"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x3000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Other("PLDL1KEEP".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 8,
                            writeback: None,
                        },
                    ],
                ),
                instruction(
                    0x1008,
                    "add",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(4),
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x3008 }),
                "known-base prefetch operands should emit data references for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.is_empty(),
                "prefetch must not synthesize destination values for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[2].recovered_values.iter().any(|value| {
                    value.register == "x9"
                        && value.value == 0x3004
                        && value.source == RecoveredValueSource::AdrpAdd
                }),
                "prefetch operands must not clobber the known base register for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_preserves_register_values_across_conditional_compares() {
        let image = synthetic_image();
        for mnemonic in ["ccmp", "ccmn"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0),
                        Operand::ImmediateUnsigned(0),
                        Operand::Name("eq".to_string()),
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x8".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "conditional compare sources must keep their recovered values across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_recovers_values_for_acquire_exclusive_loads() {
        let image = synthetic_table_image(0x3000);
        for mnemonic in ["ldar", "ldapr", "ldapur", "ldaxr", "ldxr"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4008),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4008 }),
                "acquire/exclusive load should keep its data reference for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.iter().any(|value| {
                    value.register == "x9"
                        && value.value == 0x3000
                        && value.kind == RecoveredValueKind::FunctionPointer
                        && value.source == RecoveredValueSource::TableLoad
                }),
                "acquire/exclusive loads should recover pointer table contents for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x3000 }),
                "recovered acquire/exclusive load values should feed indirect calls for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_recovers_values_for_unprivileged_loads() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4008),
                ],
            ),
            instruction(
                0x1004,
                "ldtr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "unprivileged load should keep its data reference: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 0x3000
                    && value.kind == RecoveredValueKind::FunctionPointer
                    && value.source == RecoveredValueSource::TableLoad
            }),
            "unprivileged load should recover pointer table contents: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Call { target: 0x3000 }),
            "recovered unprivileged load value should feed indirect calls: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_byte_half_unprivileged_loads() {
        for (mnemonic, register, literal, expected) in [
            ("ldtrb", "w9", vec![0x80], 0x80),
            ("ldtrsb", "x9", vec![0x80], 0xffff_ffff_ffff_ff80),
            ("ldtrh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldtrsh", "x9", vec![0x00, 0x80], 0xffff_ffff_ffff_8000),
        ] {
            let image = synthetic_literal_image(0x4000, &literal);
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register(register.to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "unprivileged byte/half load should keep its data reference for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.iter().any(|value| {
                    value.register == register
                        && value.value == expected
                        && value.kind == RecoveredValueKind::Literal
                        && value.source == RecoveredValueSource::AdrpLoad
                }),
                "unprivileged byte/half load should recover signedness for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_recovers_byte_half_acquire_exclusive_loads() {
        for (mnemonic, register, literal, expected) in [
            ("ldarb", "w9", vec![0x80], 0x80),
            ("ldaprb", "w9", vec![0x80], 0x80),
            ("ldapurb", "w9", vec![0x80], 0x80),
            ("ldaxrb", "w9", vec![0x80], 0x80),
            ("ldxrb", "w9", vec![0x80], 0x80),
            ("ldapursb", "x9", vec![0x80], 0xffff_ffff_ffff_ff80),
            ("ldarh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldaprh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldapurh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldaxrh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldxrh", "w9", vec![0x34, 0x12], 0x1234),
            ("ldapursh", "x9", vec![0x00, 0x80], 0xffff_ffff_ffff_8000),
        ] {
            let image = synthetic_literal_image(0x4000, &literal);
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register(register.to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "acquire/exclusive byte/half load should keep its data reference for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.iter().any(|value| {
                    value.register == register
                        && value.value == expected
                        && value.kind == RecoveredValueKind::Literal
                        && value.source == RecoveredValueSource::AdrpLoad
                }),
                "acquire/exclusive byte/half load should recover signedness for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_emits_data_refs_for_broader_load_store_family() {
        let image = synthetic_image();
        for (mnemonic, operands) in [
            (
                "ldar",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            (
                "stxr",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            (
                "ldadd",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            (
                "cas",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x3000),
                    ],
                ),
                instruction(0x1004, mnemonic, operands),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x3000 }),
                "load/store-family memory operands should recover known data references for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_emits_pair_store_slot_refs() {
        let image = synthetic_literal_image(0x4000, &[0; 16]);
        for (mnemonic, operands, second_slot) in [
            (
                "stp",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
                0x4008,
            ),
            (
                "stnp",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Register("w10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
                0x4004,
            ),
            (
                "stxp",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
                0x4008,
            ),
            (
                "stlxp",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Register("w9".to_string()),
                    Operand::Register("w10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
                0x4004,
            ),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(0x1004, mnemonic, operands),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "pair stores should record the first slot address for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].references.contains(&Reference::Data {
                    target: second_slot
                }),
                "pair stores should record the second slot address for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.is_empty(),
                "pair stores must not synthesize loaded values for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_compare_swap_expected_registers() {
        let image = synthetic_image();
        for (mnemonic, expected_register, desired_register) in [
            ("cas", "x9", "x10"),
            ("casal", "x9", "x10"),
            ("casb", "w9", "w10"),
            ("casalh", "w9", "w10"),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register(expected_register.to_string()),
                        Operand::Register(desired_register.to_string()),
                        Operand::Memory {
                            base: "sp".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "compare-and-swap instructions must invalidate the expected-value register across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_emits_pair_compare_swap_slot_refs() {
        let image = synthetic_literal_image(0x4000, &[0; 16]);
        for (mnemonic, first_register, second_register, first_new, second_new, second_slot) in [
            ("casp", "x9", "x10", "x11", "x12", 0x4008),
            ("caspal", "w9", "w10", "w11", "w12", 0x4004),
        ] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register(first_register.to_string()),
                        Operand::Register(second_register.to_string()),
                        Operand::Register(first_new.to_string()),
                        Operand::Register(second_new.to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "pair compare-and-swap should record the first slot address for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].references.contains(&Reference::Data {
                    target: second_slot
                }),
                "pair compare-and-swap should record the second slot address for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.is_empty(),
                "pair compare-and-swap must not synthesize loaded values for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_atomic_read_modify_write_destinations() {
        let image = synthetic_image();
        for mnemonic in ["swp", "swpal", "ldadd", "ldaddal"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Memory {
                            base: "sp".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x8".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "atomic read-modify-write instructions must invalidate their load-result register across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_preserves_atomic_read_modify_write_sources() {
        let image = synthetic_image();
        for mnemonic in ["swp", "swpal", "ldadd", "ldaddal"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Memory {
                            base: "sp".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "atomic read-modify-write source registers must keep their recovered values across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_pair_compare_swap_expected_registers() {
        let image = synthetic_image();
        for mnemonic in ["casp", "caspal"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                        Operand::Register("x10".to_string()),
                        Operand::Register("x11".to_string()),
                        Operand::Memory {
                            base: "sp".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "pair compare-and-swap instructions must invalidate both expected-value registers across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_exclusive_pair_load_destinations() {
        let image = synthetic_image();
        for mnemonic in ["ldxp", "ldaxp"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                        Operand::Memory {
                            base: "sp".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "exclusive pair loads must invalidate both destination registers across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_exclusive_store_status_registers() {
        let image = synthetic_image();
        for mnemonic in ["stxr", "stxp"] {
            let mut store_operands = vec![
                Operand::Register("w8".to_string()),
                Operand::Register("x9".to_string()),
            ];
            if mnemonic == "stxp" {
                store_operands.push(Operand::Register("x10".to_string()));
            }
            store_operands.push(Operand::Memory {
                base: "sp".to_string(),
                index: None,
                index_modifier: None,
                displacement: 0,
                writeback: None,
            });

            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x1234),
                    ],
                ),
                instruction(0x1004, mnemonic, store_operands),
                instruction(0x1008, "blr", vec![Operand::Register("x8".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Call { target: 0x1234 }),
                "exclusive-store status writes must invalidate stale register values across {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_memory_writeback_bases() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: Some(", #0x8".to_string()),
                    },
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x1".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x3000 }),
            "the writeback load should still use the incoming base before invalidation"
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Data { target: 0x3000 }),
            "writeback must invalidate the base register before later memory synthesis"
        );
        assert!(
            instructions[2].recovered_values.is_empty(),
            "writeback must not leave a stale base for later recovered values"
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_pair_load_destinations() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x5678),
                ],
            ),
            instruction(
                0x1008,
                "ldp",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "sp".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x100c,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x1010,
                "add",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "ldp must invalidate its first destination register: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.is_empty(),
            "ldp must invalidate its second destination register: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_pair_load_values() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x2000_u64.to_le_bytes());
        bytes.extend_from_slice(&0x3000_u64.to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldp",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x4000 }),
            "pair loads should record the first slot address: {:?}",
            instructions
        );
        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "pair loads should record the second slot address: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == 0x2000
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "pair loads should recover the first memory slot: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "pair loads should recover the second memory slot: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "pair-loaded function pointers should feed indirect branch recovery: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_exclusive_pair_load_values() {
        for (first_register, second_register, literal, second_slot) in [
            (
                "x9",
                "x10",
                {
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(&0x2000_u64.to_le_bytes());
                    bytes.extend_from_slice(&0x3000_u64.to_le_bytes());
                    bytes
                },
                0x4008,
            ),
            (
                "w9",
                "w10",
                {
                    let mut bytes = Vec::new();
                    bytes.extend_from_slice(&0x2000_u32.to_le_bytes());
                    bytes.extend_from_slice(&0x3000_u32.to_le_bytes());
                    bytes
                },
                0x4004,
            ),
        ] {
            for mnemonic in ["ldxp", "ldaxp"] {
                let image = synthetic_literal_image(0x4000, &literal);
                let mut instructions = vec![
                    DecodedInstruction {
                        references: vec![Reference::Page { target: 0x4000 }],
                        ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
                    },
                    instruction(
                        0x1004,
                        mnemonic,
                        vec![
                            Operand::Register(first_register.to_string()),
                            Operand::Register(second_register.to_string()),
                            Operand::Memory {
                                base: "x8".to_string(),
                                index: None,
                                index_modifier: None,
                                displacement: 0,
                                writeback: None,
                            },
                        ],
                    ),
                    instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
                ];

                synthesize_analysis_references(&image, &mut instructions, true);

                assert!(
                    instructions[1]
                        .references
                        .contains(&Reference::Data { target: 0x4000 }),
                    "exclusive pair loads should record the first slot address for {mnemonic} {first_register}: {:?}",
                    instructions
                );
                assert!(
                    instructions[1].references.contains(&Reference::Data {
                        target: second_slot
                    }),
                    "exclusive pair loads should record the second slot address for {mnemonic} {second_register}: {:?}",
                    instructions
                );
                assert!(
                    instructions[1].recovered_values.iter().any(|value| {
                        value.register == first_register
                            && value.value == 0x2000
                            && value.kind == RecoveredValueKind::ExportAddress
                            && value.source == RecoveredValueSource::AdrpLoad
                    }),
                    "exclusive pair loads should recover the first slot for {mnemonic} {first_register}: {:?}",
                    instructions
                );
                assert!(
                    instructions[1].recovered_values.iter().any(|value| {
                        value.register == second_register
                            && value.value == 0x3000
                            && value.kind == RecoveredValueKind::FunctionPointer
                            && value.source == RecoveredValueSource::AdrpLoad
                    }),
                    "exclusive pair loads should recover the second slot for {mnemonic} {second_register}: {:?}",
                    instructions
                );
                assert!(
                    instructions[2]
                        .references
                        .contains(&Reference::Branch { target: 0x3000 }),
                    "exclusive pair-loaded function pointers should feed indirect branch recovery for {mnemonic} {second_register}: {:?}",
                    instructions
                );
            }
        }
    }

    #[test]
    fn synthesize_analysis_references_does_not_recover_pair_load_values_from_executable_sections() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1008,
                "ldp",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x3000 }),
            "known pair-load addresses should still emit data refs: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x3008 }),
            "known pair-load addresses should emit the second slot even when value recovery is blocked: {:?}",
            instructions
        );
        assert!(
            instructions[2].recovered_values.is_empty(),
            "pair loads from executable bytes must not synthesize values: {:?}",
            instructions
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "unrecoverable pair loads must clear stale first-destination state: {:?}",
            instructions
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0 }),
            "pair loads from zero-filled executable bytes must not fabricate zero branch targets: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_signed_pair_load_values() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(-4_i32).to_le_bytes());
        bytes.extend_from_slice(&2_i32.to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldpsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x9"
                    && value.value == ((-4_i64) as u64)
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldpsw should sign-extend the first 32-bit slot: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 2
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldpsw should sign-extend the second 32-bit slot: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_pair_load_values_for_table_indices() {
        let mut bytes = vec![0u8; 32];
        bytes[8..16].copy_from_slice(&0x3000_u64.to_le_bytes());
        bytes[16..24].copy_from_slice(&0_u64.to_le_bytes());
        bytes[24..32].copy_from_slice(&1_u64.to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "add",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(16),
                ],
            ),
            instruction(
                0x1008,
                "ldp",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x12".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x11".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x10".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x11".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 1
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "pair-loaded table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "pair-loaded table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "pair-loaded table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_scalar_memory_load_values() {
        let mut bytes = vec![0u8; 20];
        bytes[0] = 0x80;
        bytes[2..4].copy_from_slice(&(-2_i16).to_le_bytes());
        bytes[4..8].copy_from_slice(&0x1122_3344_u32.to_le_bytes());
        bytes[8..16].copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
        bytes[16..20].copy_from_slice(&(-4_i32).to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrb",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "ldrsh",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 2,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("w11".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 4,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1010,
                "ldr",
                vec![
                    Operand::Register("x12".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 8,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1014,
                "ldrsw",
                vec![
                    Operand::Register("x13".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 16,
                        writeback: None,
                    },
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "w9"
                    && value.value == 0x80
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldrb should recover zero-extended byte contents: {:?}",
            instructions
        );
        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == ((-2_i64) as u64)
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldrsh should recover sign-extended halfword contents: {:?}",
            instructions
        );
        assert!(
            instructions[3].recovered_values.iter().any(|value| {
                value.register == "w11"
                    && value.value == 0x1122_3344
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldr w should recover 32-bit contents: {:?}",
            instructions
        );
        assert!(
            instructions[4].recovered_values.iter().any(|value| {
                value.register == "x12"
                    && value.value == 0x1122_3344_5566_7788
                    && value.kind == RecoveredValueKind::Literal
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldr x should recover readable 64-bit scalar contents: {:?}",
            instructions
        );
        assert!(
            instructions[5].recovered_values.iter().any(|value| {
                value.register == "x13"
                    && value.value == ((-4_i64) as u64)
                    && value.kind == RecoveredValueKind::Literal
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "ldrsw should recover sign-extended word contents when it is not a relative table slot: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_recovers_scalar_acquire_exclusive_load_values() {
        let mut bytes = vec![0u8; 16];
        bytes[0..4].copy_from_slice(&0x1122_3344_u32.to_le_bytes());
        bytes[8..16].copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
        let image = synthetic_literal_image(0x4000, &bytes);

        for mnemonic in ["ldar", "ldapr", "ldapur", "ldaxr", "ldxr"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("w9".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(
                    0x1008,
                    "add",
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(8),
                    ],
                ),
                instruction(
                    0x100c,
                    mnemonic,
                    vec![
                        Operand::Register("x11".to_string()),
                        Operand::Memory {
                            base: "x10".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "known acquire/exclusive scalar load addresses should be emitted for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.iter().any(|value| {
                    value.register == "w9"
                        && value.value == 0x1122_3344
                        && value.kind == RecoveredValueKind::Literal
                        && value.source == RecoveredValueSource::AdrpLoad
                }),
                "w-register acquire/exclusive loads should recover scalar contents for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[3]
                    .references
                    .contains(&Reference::Data { target: 0x4008 }),
                "known acquire/exclusive scalar load addresses should be emitted for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[3].recovered_values.iter().any(|value| {
                    value.register == "x11"
                        && value.value == 0x1122_3344_5566_7788
                        && value.kind == RecoveredValueKind::Literal
                        && value.source == RecoveredValueSource::AdrpLoad
                }),
                "x-register acquire/exclusive loads should recover scalar contents for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unreadable_acquire_load_destinations() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x5000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1008,
                "ldar",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unreadable acquire loads must not synthesize values"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "unreadable acquire loads must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_scalar_load_values_for_table_indices() {
        let mut bytes = vec![0u8; 24];
        bytes[8..16].copy_from_slice(&0x3000_u64.to_le_bytes());
        bytes[16] = 1;
        let image = synthetic_literal_image(0x4000, &bytes);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrb",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 16,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("w9".to_string()),
                        index_modifier: Some("uxtw #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "w9"
                    && value.value == 1
                    && value.source == RecoveredValueSource::AdrpLoad
            }),
            "scalar-loaded table indices should stay known: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "scalar-loaded table indices should resolve slot addresses: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "scalar-loaded table indices should resolve indirect branches: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_invalidates_unreadable_scalar_load_destinations() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "unreadable scalar loads must not synthesize values"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "unreadable scalar loads must clear stale destination state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_clears_ambiguous_join_state() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x100),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Branch { target: 0x1010 }],
                ..instruction(0x1004, "b", vec![Operand::Label(0x1010)])
            },
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x200),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Branch { target: 0x1010 }],
                ..instruction(0x100c, "b", vec![Operand::Label(0x1010)])
            },
            instruction(
                0x1010,
                "add",
                vec![
                    Operand::Register("x1".to_string()),
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4]
                .recovered_values
                .iter()
                .all(|value| value.register != "x1"),
            "join block has two possible x0 values and must not inherit either one"
        );
    }

    #[test]
    fn synthesize_literal_load_classifies_export_addresses() {
        let image = synthetic_literal_image(0x4000, &0x2000_u64.to_le_bytes());
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x4000 }],
            ..instruction(
                0x1000,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::ExportAddress && value.value == 0x2000
        }));
    }

    #[test]
    fn synthesize_literal_load_classifies_function_pointers() {
        let image = synthetic_literal_image(0x4000, &0x3000_u64.to_le_bytes());
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x4000 }],
            ..instruction(
                0x1000,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer && value.value == 0x3000
        }));
    }

    #[test]
    fn synthesize_literal_load_reads_signed_word_contents() {
        let image = synthetic_literal_image(0x4000, &(-4_i32).to_le_bytes());
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x4000 }],
            ..instruction(
                0x1000,
                "ldrsw",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[0].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::Literal && value.value == (-4_i64 as u64)
        }));
    }

    #[test]
    fn synthesize_literal_load_does_not_recover_unreadable_literal_addresses() {
        let image = synthetic_image();
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x2000 }],
            ..instruction(
                0x1000,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x2000),
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "literal loads must read literal contents instead of treating the literal address as the loaded value"
        );
    }

    #[test]
    fn synthesize_literal_load_ignores_memory_form_data_refs() {
        let image = synthetic_literal_image(0x4000, &0x3000_u64.to_le_bytes());
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x4000 }],
            ..instruction(
                0x1000,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::Memory {
                        base: "x1".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "memory-form ldr must not recover a literal value just because a data reference is present"
        );
    }

    #[test]
    fn synthesize_literal_load_requires_operand_and_data_ref_to_match() {
        let image = synthetic_literal_image(0x4000, &0x3000_u64.to_le_bytes());
        let mut instructions = vec![DecodedInstruction {
            references: vec![Reference::Data { target: 0x4000 }],
            ..instruction(
                0x1000,
                "ldr",
                vec![
                    Operand::Register("x0".to_string()),
                    Operand::ImmediateUnsigned(0x5000),
                ],
            )
        }];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[0].recovered_values.is_empty(),
            "literal loads must not recover through an unrelated data reference: {:?}",
            instructions
        );
        assert!(
            instructions[0]
                .references
                .contains(&Reference::Data { target: 0x4000 }),
            "existing unrelated data references should be preserved for other evidence: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_literal_load_ignores_non_literal_load_mnemonics() {
        let image = synthetic_literal_image(0x4000, &0x3000_u64.to_le_bytes());
        for mnemonic in ["ldrb", "ldrh", "ldrsh"] {
            let mut instructions = vec![DecodedInstruction {
                references: vec![Reference::Data { target: 0x4000 }],
                ..instruction(
                    0x1000,
                    mnemonic,
                    vec![
                        Operand::Register("w0".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                )
            }];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[0].recovered_values.is_empty(),
                "only exact ldr/ldrsw literal forms should recover literal values for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_relative_slot_recovers_function_pointer() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x100c, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer
                && value.source == RecoveredValueSource::RelativeTableLoad
                && value.value == 0x3000
        }));
        assert!(instructions.iter().any(|instruction| {
            instruction.annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::TableSlotResolved {
                        encoding: TableSlotEncoding::Relative32,
                        target: 0x3000,
                        ..
                    }
                )
            })
        }));
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Call { target: 0x3000 })
        );
        assert!(instructions[3].annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::IndirectTargetResolved {
                    target: 0x3000,
                    reason: IndirectTargetReason::FunctionPointer,
                    ..
                }
            )
        }));
    }

    #[test]
    fn synthesize_relative_slot_recovers_unscaled_signed_word_loads() {
        let image = synthetic_table_image(0x3000);
        for mnemonic in ["ldursw", "ldtrsw", "ldapursw"] {
            let mut instructions = vec![
                DecodedInstruction {
                    references: vec![Reference::Page { target: 0x4000 }],
                    ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
                },
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(
                    0x1008,
                    "add",
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Register("x8".to_string()),
                        Operand::Register("x9".to_string()),
                    ],
                ),
                instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1].recovered_values.is_empty(),
                "signed word table loads should keep relative-slot evidence for add, not recover a direct target for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[2].recovered_values.iter().any(|value| {
                    value.register == "x10"
                        && value.value == 0x3000
                        && value.kind == RecoveredValueKind::FunctionPointer
                        && value.source == RecoveredValueSource::RelativeTableLoad
                }),
                "unscaled signed word table loads should recover the add-derived target for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[3]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 }),
                "unscaled signed word table loads should feed indirect branches for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_does_not_fabricate_unscaled_acquire_byte_half_targets() {
        let image = synthetic_table_image(0x3000);
        for mnemonic in ["ldapurb", "ldapursb", "ldapurh", "ldapursh"] {
            let mut instructions = vec![
                instruction(
                    0x1000,
                    "mov",
                    vec![
                        Operand::Register("x8".to_string()),
                        Operand::ImmediateUnsigned(0x4000),
                    ],
                ),
                instruction(
                    0x1004,
                    mnemonic,
                    vec![
                        Operand::Register("w9".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: None,
                            index_modifier: None,
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x1008, "br", vec![Operand::Register("x9".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[1]
                    .references
                    .contains(&Reference::Data { target: 0x4000 }),
                "known unscaled acquire load addresses should still be emitted for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                instructions[1].recovered_values.iter().all(|value| {
                    !matches!(
                        value.kind,
                        RecoveredValueKind::FunctionPointer | RecoveredValueKind::ExportAddress
                    )
                }),
                "byte/half unscaled acquire loads must not be promoted to pointer table slots for {mnemonic}: {:?}",
                instructions
            );
            assert!(
                !instructions[2]
                    .references
                    .contains(&Reference::Branch { target: 0x3000 }),
                "byte/half unscaled acquire loads must not fabricate branch targets for {mnemonic}: {:?}",
                instructions
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_respects_table_slot_endianness() {
        let big_endian_image = synthetic_table_image_with_endianness(0x3000, Endianness::Big);
        let mut absolute_instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4008),
                ],
            ),
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "blr", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&big_endian_image, &mut absolute_instructions, true);

        assert!(
            absolute_instructions[1]
                .recovered_values
                .iter()
                .any(|value| {
                    value.register == "x9"
                        && value.value == 0x3000
                        && value.kind == RecoveredValueKind::FunctionPointer
                        && value.source == RecoveredValueSource::TableLoad
                }),
            "big-endian absolute table slots should decode as stored: {:?}",
            absolute_instructions
        );
        assert!(
            absolute_instructions[2]
                .references
                .contains(&Reference::Call { target: 0x3000 }),
            "big-endian absolute table slots should feed indirect calls: {:?}",
            absolute_instructions
        );

        let high_target = 0xffff_ffff_ffff_f010;
        let high_image =
            synthetic_stripped_absolute_table_image(0xffff_ffff_ffff_f000, high_target);
        let mut stripped_high_instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&high_image, &mut stripped_high_instructions, true);

        assert!(
            stripped_high_instructions[1]
                .recovered_values
                .iter()
                .any(|value| {
                    value.register == "x9"
                        && value.value == high_target
                        && value.kind == RecoveredValueKind::Address
                        && value.source == RecoveredValueSource::TableLoad
                }),
            "absolute table slots should recover stripped executable targets: {:?}",
            stripped_high_instructions
        );
        assert!(
            stripped_high_instructions[1]
                .annotations
                .iter()
                .any(|annotation| {
                    matches!(
                        annotation,
                        Annotation::TableSlotResolved {
                            encoding: TableSlotEncoding::Absolute64,
                            target,
                            ..
                        } if *target == high_target
                    )
                }),
            "stripped executable table slots should keep table-slot evidence: {:?}",
            stripped_high_instructions
        );
        assert!(
            stripped_high_instructions[2]
                .references
                .contains(&Reference::Branch {
                    target: high_target
                }),
            "stripped executable absolute slots should feed indirect branches: {:?}",
            stripped_high_instructions
        );

        let mut relative_instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x2000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x2004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x2008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x200c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&big_endian_image, &mut relative_instructions, true);

        assert!(
            relative_instructions[2]
                .recovered_values
                .iter()
                .any(|value| {
                    value.register == "x10"
                        && value.value == 0x3000
                        && value.kind == RecoveredValueKind::FunctionPointer
                        && value.source == RecoveredValueSource::RelativeTableLoad
                }),
            "big-endian relative table slots should decode signed displacements correctly: {:?}",
            relative_instructions
        );
        assert!(
            relative_instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "big-endian relative table slots should feed indirect branches: {:?}",
            relative_instructions
        );
    }

    #[test]
    fn synthesize_absolute32_table_load_prefers_unsigned_absolute_target() {
        let image = synthetic_absolute32_table_image(0x3000, 0x7000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x4000),
                ],
            ),
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x9".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1].recovered_values.iter().any(|value| {
                value.register == "w9"
                    && value.value == 0x3000
                    && value.kind == RecoveredValueKind::FunctionPointer
                    && value.source == RecoveredValueSource::TableLoad
            }),
            "unsigned word table loads should recover the absolute32 slot target: {:?}",
            instructions
        );
        assert!(
            instructions[1].annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::TableSlotResolved {
                        element_size: 4,
                        encoding: TableSlotEncoding::Absolute32,
                        target: 0x3000,
                        ..
                    }
                )
            }),
            "unsigned word table loads should report absolute32 evidence: {:?}",
            instructions
        );
        assert!(
            instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unsigned word table loads should feed the absolute target into indirect branches: {:?}",
            instructions
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x7000 }),
            "unsigned word table loads must not reinterpret the slot as relative32: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_relative_slot_recovers_unshifted_extended_add_target() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ShiftedRegister {
                        register: "w9".to_string(),
                        shift: "sxtw".to_string(),
                    },
                ],
            ),
            instruction(0x100c, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer
                && value.source == RecoveredValueSource::RelativeTableLoad
                && value.value == 0x3000
        }));
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Call { target: 0x3000 }),
            "unshifted sign-extended add operands should resolve relative table targets"
        );
    }

    #[test]
    fn synthesize_relative_slot_recovers_export_address() {
        let image = synthetic_table_image(0x2000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x100c, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions[2].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::ExportAddress
                && value.source == RecoveredValueSource::RelativeTableLoad
                && value.value == 0x2000
        }));
        assert!(instructions[3].annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::IndirectTargetResolved {
                    target: 0x2000,
                    reason: IndirectTargetReason::ExportAddress,
                    ..
                }
            )
        }));
    }

    #[test]
    fn synthesize_relative_slot_sub_does_not_resolve_add_target() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "sub",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::Register("x9".to_string()),
                ],
            ),
            instruction(0x100c, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "relative table-slot targets are add-derived and must not attach to sub: {:?}",
            instructions
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Call { target: 0x3000 }),
            "sub must not inherit the add-pattern target as an indirect call"
        );
    }

    #[test]
    fn synthesize_relative_slot_shifted_add_does_not_resolve_unshifted_target() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x1008,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ShiftedRegister {
                        register: "x9".to_string(),
                        shift: "lsl #2".to_string(),
                    },
                ],
            ),
            instruction(0x100c, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].recovered_values.is_empty(),
            "relative table-slot targets must not ignore shifted add operands: {:?}",
            instructions
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Call { target: 0x3000 }),
            "shifted add must not inherit the unshifted relative-slot target"
        );
    }

    #[test]
    fn synthesize_relative_slot_load_clears_prior_register_value() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1004, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1008,
                "ldrsw",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x100c,
                "add",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3].recovered_values.is_empty(),
            "ldrsw relative slot evidence must replace stale known register values: {:?}",
            instructions
        );
        assert!(instructions[2].annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::TableSlotResolved {
                    encoding: TableSlotEncoding::Relative32,
                    target: 0x3000,
                    ..
                }
            )
        }));
    }

    #[test]
    fn synthesize_analysis_references_clears_stale_jump_table_source_after_register_overwrite() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(
                0x100c,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "indirect branch should resolve through the current x10 value: {:?}",
            instructions
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "stale jump-table source must not override the current x10 value: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_does_not_create_jump_table_sources_for_stores() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::ImmediateUnsigned(0x1234),
                ],
            ),
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1004, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1008,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "str",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x1234 }),
            "store source registers should keep resolving through their current value: {:?}",
            instructions
        );
        assert!(
            !instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "indexed stores must not create jump-table source state: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_byte_stride_for_unmodified_register_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "unmodified register offsets are byte offsets, not element indexes: {:?}",
            instructions
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4040 }),
            "unmodified register offsets must not be scaled by load width"
        );
        assert!(
            instructions[2].recovered_values.iter().any(|value| {
                value.register == "x10"
                    && value.value == 0x3000
                    && value.kind == RecoveredValueKind::FunctionPointer
                    && value.source == RecoveredValueSource::TableLoad
            }),
            "the load should read the 8-byte slot at base + byte offset: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "the indirect branch should resolve through the recovered slot target"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x4040 }),
            "the indirect branch must not resolve through a scaled-offset fabrication"
        );
    }

    #[test]
    fn synthesize_analysis_references_accepts_unshifted_extended_memory_index_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(8),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("w9".to_string()),
                        index_modifier: Some("uxtw".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "unshifted uxtw register offsets should default to byte stride: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unshifted uxtw offsets should still resolve table loads"
        );
    }

    #[test]
    fn synthesize_analysis_references_accepts_signed_extended_memory_index_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("w9".to_string()),
                        index_modifier: Some("sxtw #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "positive sxtw offsets should scale by the encoded shift: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "positive sxtw offsets should resolve table loads"
        );
    }

    #[test]
    fn synthesize_analysis_references_sign_extends_negative_memory_index_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "add",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x10),
                ],
            ),
            instruction(
                0x1008,
                "movn",
                vec![
                    Operand::Register("w9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x100c,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("w9".to_string()),
                        index_modifier: Some("sxtw #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1010, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[3]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "sxtw must treat 0xffffffff as -1 and subtract the scaled offset: {:?}",
            instructions
        );
        assert!(
            !instructions[3].references.contains(&Reference::Data {
                target: 0x800004008
            }),
            "sxtw must not treat 0xffffffff as a huge unsigned index"
        );
        assert!(
            instructions[4]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "negative sxtw offsets should resolve table loads"
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_shift_stride_for_modified_register_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "shifted register offsets should scale by the encoded shift: {:?}",
            instructions
        );
        assert!(
            instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "the shifted offset should still resolve the table slot target"
        );
    }

    #[test]
    fn synthesize_analysis_references_does_not_recover_unresolved_load_addresses() {
        let image = synthetic_image();
        let mut instructions = vec![
            instruction(
                0x1000,
                "mov",
                vec![
                    Operand::Register("x8".to_string()),
                    Operand::ImmediateUnsigned(0x3000),
                ],
            ),
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x3000 }),
            "the known load address should still be emitted as a data reference: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.is_empty(),
            "loads must not recover the address being read as the loaded value: {:?}",
            instructions
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "indirect control flow must not resolve through the load address itself"
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_invalid_memory_index_shifts() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #4".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].references.is_empty(),
            "invalid scaled register offsets must not synthesize a data reference: {:?}",
            instructions
        );
        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid scaled register offsets must not recover a value"
        );
        assert!(
            instructions[3]
                .annotations
                .iter()
                .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. })),
            "invalid scaled register offsets must not create jump-table provenance"
        );
    }

    #[test]
    fn synthesize_analysis_references_rejects_malformed_memory_index_shift_text() {
        let image = synthetic_table_image(0x3000);
        for modifier in ["lsl #-3", "lsl junk", "uxtw junk #0"] {
            let mut instructions = vec![
                DecodedInstruction {
                    references: vec![Reference::Page { target: 0x4000 }],
                    ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
                },
                instruction(
                    0x1004,
                    "mov",
                    vec![
                        Operand::Register("x9".to_string()),
                        Operand::ImmediateUnsigned(1),
                    ],
                ),
                instruction(
                    0x1008,
                    "ldr",
                    vec![
                        Operand::Register("x10".to_string()),
                        Operand::Memory {
                            base: "x8".to_string(),
                            index: Some("x9".to_string()),
                            index_modifier: Some(modifier.to_string()),
                            displacement: 0,
                            writeback: None,
                        },
                    ],
                ),
                instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
            ];

            synthesize_analysis_references(&image, &mut instructions, true);

            assert!(
                instructions[2].references.is_empty(),
                "malformed shift text must not be scraped into a valid memory index: {modifier}"
            );
            assert!(
                instructions[2].recovered_values.is_empty(),
                "malformed memory-index shifts must not recover a table value: {modifier}"
            );
            assert!(
                instructions[3]
                    .annotations
                    .iter()
                    .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. })),
                "malformed memory-index shifts must not create jump-table provenance: {modifier}"
            );
        }
    }

    #[test]
    fn synthesize_analysis_references_uses_byte_element_size_for_byte_load_shifts() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(1),
                ],
            ),
            instruction(
                0x1008,
                "ldrb",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #2".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].references.is_empty(),
            "ldrb reads one byte, so lsl #2 must not be accepted from the w-register result width: {:?}",
            instructions
        );
        assert!(
            instructions[2].recovered_values.is_empty(),
            "invalid byte-load shifts must not recover a fabricated value"
        );
        assert!(
            instructions[3]
                .annotations
                .iter()
                .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. })),
            "invalid byte-load shifts must not create jump-table provenance"
        );
    }

    #[test]
    fn synthesize_analysis_references_uses_halfword_element_size_for_halfword_load_shifts() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(4),
                ],
            ),
            instruction(
                0x1008,
                "ldrh",
                vec![
                    Operand::Register("w10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("lsl #1".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4008 }),
            "ldrh should scale by two bytes, not four from the w-register result width: {:?}",
            instructions
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Data { target: 0x4010 }),
            "halfword loads must not use the 32-bit destination width as their address scale"
        );
    }

    #[test]
    fn synthesize_analysis_references_does_not_recover_unknown_index_load_values() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: None,
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[1]
                .references
                .contains(&Reference::Data { target: 0x4000 }),
            "unknown-index loads may still expose the table/base reference: {:?}",
            instructions
        );
        assert!(
            instructions[1].recovered_values.is_empty(),
            "unknown-index loads must not recover the base address as the loaded value: {:?}",
            instructions
        );
        assert!(
            !instructions[2]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unknown-index loads must not resolve indirect control flow through a fabricated value"
        );
        assert!(
            instructions[2]
                .annotations
                .iter()
                .any(|annotation| matches!(
                    annotation,
                    Annotation::JumpTableCandidate {
                        base: 0x4000,
                        index_register,
                        element_size: 8
                    } if index_register == "x9"
                )),
            "unknown-index loads should keep jump-table provenance without a concrete target: {:?}",
            instructions
        );
    }

    #[test]
    fn synthesize_analysis_references_ignores_modified_memory_index_offsets() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "mov",
                vec![
                    Operand::Register("x9".to_string()),
                    Operand::ImmediateUnsigned(0),
                ],
            ),
            instruction(
                0x1008,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: Some("x9".to_string()),
                        index_modifier: Some("ror #3".to_string()),
                        displacement: 0,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x100c, "br", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(
            instructions[2].references.is_empty(),
            "unsupported modified register-offset memory operands must not synthesize an unmodified data reference: {:?}",
            instructions
        );
        assert!(
            instructions[2].recovered_values.is_empty(),
            "unsupported modified register-offset memory operands must not recover a value with unmodified index semantics"
        );
        assert!(
            !instructions[3]
                .references
                .contains(&Reference::Branch { target: 0x3000 }),
            "unsupported modified register-offset memory operands must not create jump-table state"
        );
    }

    #[test]
    fn synthesize_constant_slot_dispatch_does_not_emit_jump_table_candidate() {
        let image = synthetic_table_image(0x3000);
        let mut instructions = vec![
            DecodedInstruction {
                references: vec![Reference::Page { target: 0x4000 }],
                ..instruction(0x1000, "adrp", vec![Operand::Register("x8".to_string())])
            },
            instruction(
                0x1004,
                "ldr",
                vec![
                    Operand::Register("x10".to_string()),
                    Operand::Memory {
                        base: "x8".to_string(),
                        index: None,
                        index_modifier: None,
                        displacement: 8,
                        writeback: None,
                    },
                ],
            ),
            instruction(0x1008, "blr", vec![Operand::Register("x10".to_string())]),
        ];

        synthesize_analysis_references(&image, &mut instructions, true);

        assert!(instructions.iter().all(|instruction| {
            instruction
                .annotations
                .iter()
                .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. }))
        }));
        assert!(instructions[1].recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer
                && value.source == RecoveredValueSource::TableLoad
                && value.value == 0x3000
        }));
        assert!(instructions[1].annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::TableSlotResolved {
                    encoding: TableSlotEncoding::Absolute64,
                    target: 0x3000,
                    ..
                }
            )
        }));
    }
}

fn push_annotation(target: &mut Vec<Annotation>, annotation: Annotation) {
    if !target.contains(&annotation) {
        target.push(annotation);
    }
}
