use damsel_core::{
    Annotation, DisassemblyLimit, DisassemblyOptions, DisassemblyRequest, DisassemblyRequestV2,
    DisassemblyStopReason, DisassemblyTarget, RecoveredValueKind, RecoveredValueSource, Reference,
    TableSlotEncoding,
};
use damsel_macho::{MachoError, disassemble, disassemble_v2, load};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

#[test]
fn disasm_unknown_symbol_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Symbol("__missing_symbol__".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unknown symbol should fail");
    assert!(matches!(error, MachoError::SymbolNotFound(_)));
}

#[test]
fn disasm_unknown_section_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__missing_section__".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unknown section should fail");
    assert!(matches!(error, MachoError::SectionNotFound(_)));
}

#[test]
fn disasm_unmapped_address_errors() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Address(0x1),
        max_instructions: Some(8),
        limit: None,
        include_annotations: true,
    };
    let error = disassemble(&image, &request).expect_err("unmapped address should fail");
    assert!(matches!(error, MachoError::AddressNotMapped(0x1)));
}

#[test]
fn disasm_by_address_smoke() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let main_symbol = image.symbol_by_name("_main").expect("main symbol exists");

    let request = DisassemblyRequest {
        target: DisassemblyTarget::Address(main_symbol.address),
        max_instructions: Some(6),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble by address");
    assert_eq!(result.start_address, main_symbol.address);
    assert!(!result.instructions.is_empty());
}

#[test]
fn disasm_by_section_smoke() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: Some(6),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble by section");
    assert!(result.target.contains("__text"));
    assert!(!result.instructions.is_empty());
}

#[test]
fn disasm_honors_include_annotations_flag() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Symbol("_main".to_string()),
        max_instructions: Some(8),
        limit: None,
        include_annotations: false,
    };
    let result = disassemble(&image, &request).expect("disassemble main without annotations");
    assert!(!result.instructions.is_empty());
    assert!(
        result
            .instructions
            .iter()
            .all(|instruction| instruction.annotations.is_empty())
    );
}

#[test]
fn disasm_reports_limit_reached_stop_reason() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: None,
        limit: Some(DisassemblyLimit::Instructions(1)),
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble with hard instruction cap");
    assert_eq!(
        result.stop_reason,
        DisassemblyStopReason::InstructionLimitReached
    );
}

#[test]
fn disasm_reports_decode_halt_for_too_short_byte_window() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: None,
        limit: Some(DisassemblyLimit::Bytes(3)),
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble tiny byte window");
    assert_eq!(result.stop_reason, DisassemblyStopReason::DecodeHalt);
}

#[test]
fn disasm_v2_range_clamps_target_window() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let main_symbol = image.symbol_by_name("_main").expect("main symbol exists");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(main_symbol.address),
        range: Some(main_symbol.address..main_symbol.address.saturating_add(4)),
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: false,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble v2 with range");
    assert_eq!(result.start_address, main_symbol.address);
    assert!(
        result.decoded_bytes <= 4,
        "range should clamp decoded bytes"
    );
    assert_eq!(result.stop_reason, DisassemblyStopReason::WindowClipped);
}

#[test]
fn disasm_v2_reports_byte_limit_reached() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let main_symbol = image.symbol_by_name("_main").expect("main symbol exists");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Address(main_symbol.address),
        range: None,
        limit: DisassemblyLimit::Bytes(4),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble with byte limit");
    assert_eq!(result.stop_reason, DisassemblyStopReason::ByteLimitReached);
}

#[test]
fn disasm_v2_reports_input_exhausted_for_disjoint_range() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let text = image
        .section_by_name("__text")
        .expect("text section exists");
    let disjoint_start = text.address.saturating_add(text.size).saturating_add(0x40);
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: Some(disjoint_start..disjoint_start.saturating_add(0x20)),
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble disjoint range");
    assert_eq!(result.start_address, disjoint_start);
    assert_eq!(result.stop_reason, DisassemblyStopReason::InputExhausted);
}

#[test]
fn disasm_v2_reports_target_range_end_for_full_window() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble full text");
    assert_eq!(result.stop_reason, DisassemblyStopReason::TargetRangeEnd);
}

#[test]
fn disasm_emits_import_references_when_targets_match_import_sites() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let target = if let Some(stub) = image.dyld().stubs.first() {
        DisassemblyTarget::Address(stub.stub_address)
    } else {
        DisassemblyTarget::Section("__text".to_string())
    };

    if image.dyld().stubs.is_empty()
        && image.dyld().import_bindings.is_empty()
        && image
            .imports()
            .iter()
            .all(|import| import.address.is_none())
    {
        return;
    }

    let request = DisassemblyRequest {
        target,
        max_instructions: Some(64),
        limit: None,
        include_annotations: true,
    };
    let result = disassemble(&image, &request).expect("disassemble target window");

    let has_import_reference = result.instructions.iter().any(|instruction| {
        instruction.references.iter().any(|reference| {
            matches!(
                reference,
                Reference::Import { .. } | Reference::ImportBinding { .. } | Reference::Stub { .. }
            )
        })
    });
    assert!(
        has_import_reference,
        "expected at least one import-related reference in selected target disassembly"
    );
}

#[test]
fn disasm_emits_import_pointer_recovered_values_for_lazy_fixture_when_present() {
    let path = fixture("import-lazy");
    if !path.exists() {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load lazy helper fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(64),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble lazy helper fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .recovered_values
            .iter()
            .any(|value| value.kind == RecoveredValueKind::ImportPointer)
    }));
}

#[test]
fn disasm_emits_helper_references_for_lazy_fixture_when_present() {
    let path = fixture("import-lazy");
    if !path.exists() {
        eprintln!("import-lazy fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load lazy helper fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(128),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble lazy helper fixture");
    let has_helper_reference = result.instructions.iter().any(|instruction| {
        instruction
            .references
            .iter()
            .any(|reference| matches!(reference, Reference::StubHelper { .. }))
    });
    assert!(
        has_helper_reference,
        "expected at least one helper reference in lazy helper fixture disassembly"
    );
}

#[test]
fn disasm_include_value_flow_false_skips_semantic_artifacts() {
    let path = fixture("semantic-switch");
    if !path.exists() {
        eprintln!("semantic-switch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load semantic fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: false,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble semantic fixture");
    assert!(
        result
            .instructions
            .iter()
            .all(|instruction| instruction.recovered_values.is_empty())
    );
    assert!(result.instructions.iter().all(|instruction| {
        instruction
            .annotations
            .iter()
            .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. }))
    }));
}

#[test]
fn disasm_emits_jump_table_candidate_for_semantic_fixture_when_present() {
    let path = fixture("semantic-switch");
    if !path.exists() {
        eprintln!("semantic-switch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load semantic fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble semantic fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .annotations
            .iter()
            .any(|annotation| matches!(annotation, Annotation::JumpTableCandidate { .. }))
    }));
}

#[test]
fn disasm_emits_indirect_target_resolved_for_semantic_fixture_when_present() {
    let path = fixture("semantic-switch");
    if !path.exists() {
        eprintln!("semantic-switch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load semantic fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble semantic fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .annotations
            .iter()
            .any(|annotation| matches!(annotation, Annotation::IndirectTargetResolved { .. }))
    }));
}

#[test]
fn disasm_emits_indirect_target_annotations_for_indirect_dispatch_fixture_when_present() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble indirect-dispatch fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .annotations
            .iter()
            .any(|annotation| matches!(annotation, Annotation::IndirectTargetResolved { .. }))
    }));
}

#[test]
fn disasm_resolves_function_pointer_table_slot_for_dispatch_fixture() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_dispatch_second_slot".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble indirect-dispatch symbol");
    assert!(result.instructions.iter().any(|instruction| {
        instruction.recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer
                && value.source == RecoveredValueSource::TableLoad
        })
    }));
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::TableSlotResolved {
                    encoding: TableSlotEncoding::Absolute64,
                    ..
                }
            )
        })
    }));
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::IndirectTargetResolved {
                    reason: damsel_core::IndirectTargetReason::FunctionPointer,
                    ..
                }
            )
        })
    }));
}

#[test]
fn disasm_resolves_export_address_table_slot_for_dispatch_fixture() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_load_export_target".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble export-target symbol");
    assert!(result.instructions.iter().any(|instruction| {
        instruction.recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::ExportAddress
                && value.source == RecoveredValueSource::TableLoad
        })
    }));
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::TableSlotResolved {
                    encoding: TableSlotEncoding::Absolute64,
                    ..
                }
            )
        })
    }));
}

#[test]
fn disasm_resolves_relative_function_pointer_targets_when_present() {
    let path = fixture("relative-dispatch");
    if !path.exists() {
        eprintln!("relative-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load relative-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_relative_dispatch_second_slot".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble relative dispatch symbol");
    assert!(result.instructions.iter().any(|instruction| {
        instruction.recovered_values.iter().any(|value| {
            value.kind == RecoveredValueKind::FunctionPointer
                && value.source == damsel_core::RecoveredValueSource::RelativeTableLoad
        })
    }));
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::TableSlotResolved {
                    encoding: damsel_core::TableSlotEncoding::Relative32,
                    ..
                }
            )
        })
    }));
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::IndirectTargetResolved {
                    reason: damsel_core::IndirectTargetReason::FunctionPointer,
                    ..
                }
            )
        })
    }));
}

#[test]
fn disasm_resolves_relative_export_and_function_targets_for_loads_when_present() {
    let path = fixture("relative-dispatch");
    if !path.exists() {
        eprintln!("relative-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load relative-dispatch fixture");

    for (symbol, expected_kind) in [
        (
            "_relative_load_export_target",
            RecoveredValueKind::ExportAddress,
        ),
        (
            "_relative_load_function_target",
            RecoveredValueKind::FunctionPointer,
        ),
    ] {
        let request = DisassemblyRequestV2 {
            target: DisassemblyTarget::Symbol(symbol.to_string()),
            range: None,
            limit: DisassemblyLimit::Instructions(32),
            options: DisassemblyOptions {
                include_annotations: true,
                include_value_flow: true,
            },
        };
        let result = disassemble_v2(&image, &request)
            .unwrap_or_else(|error| panic!("disassemble {symbol}: {error}"));
        assert!(result.instructions.iter().any(|instruction| {
            instruction.recovered_values.iter().any(|value| {
                value.kind == expected_kind
                    && value.source == damsel_core::RecoveredValueSource::RelativeTableLoad
            })
        }));
        assert!(result.instructions.iter().any(|instruction| {
            instruction.annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::TableSlotResolved {
                        encoding: damsel_core::TableSlotEncoding::Relative32,
                        ..
                    }
                )
            })
        }));
    }
}

#[test]
fn disasm_avoids_generic_indirect_annotation_when_authenticated_variant_exists() {
    let path = fixture("arm64e-sample");
    if !path.exists() {
        eprintln!("arm64e-sample fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load arm64e fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble arm64e fixture");

    for instruction in &result.instructions {
        let authenticated = instruction
            .annotations
            .iter()
            .filter_map(|annotation| match annotation {
                Annotation::IndirectControlFlow { kind, via }
                    if kind.starts_with("authenticated-") =>
                {
                    Some(via)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for via in authenticated {
            let has_generic = instruction.annotations.iter().any(|annotation| {
                matches!(
                    annotation,
                    Annotation::IndirectControlFlow { kind, via: existing_via }
                        if existing_via == via && (kind == "call" || kind == "branch")
                )
            });
            assert!(
                !has_generic,
                "found generic indirect annotation for authenticated flow via {via}"
            );
        }
    }
}

#[test]
fn disasm_authenticated_annotations_present_without_value_flow() {
    let path = fixture("arm64e-sample");
    if !path.exists() {
        eprintln!("arm64e-sample fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load arm64e fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: false,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble arm64e fixture");
    let has_authenticated_indirect = result.instructions.iter().any(|instruction| {
        matches!(
            instruction.mnemonic.as_str(),
            "blraa" | "blraaz" | "blrab" | "blrabz" | "braa" | "braaz" | "brab" | "brabz"
        )
    });
    if !has_authenticated_indirect {
        eprintln!("arm64e fixture does not contain authenticated indirect branch/call; skipping");
        return;
    }
    assert!(result.instructions.iter().any(|instruction| {
        instruction.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                Annotation::IndirectControlFlow { kind, .. }
                    if kind.starts_with("authenticated-")
            )
        })
    }));
}

#[test]
fn disasm_indirect_target_resolved_reasons_are_bounded_when_present() {
    let path = fixture("semantic-switch");
    if !path.exists() {
        eprintln!("semantic-switch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load semantic fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble semantic fixture");
    let mut saw_reason = false;
    for instruction in &result.instructions {
        for annotation in &instruction.annotations {
            if let Annotation::IndirectTargetResolved { reason, .. } = annotation {
                saw_reason = true;
                assert!(
                    matches!(
                        reason,
                        damsel_core::IndirectTargetReason::RegisterState
                            | damsel_core::IndirectTargetReason::HelperTarget
                            | damsel_core::IndirectTargetReason::StubTarget
                            | damsel_core::IndirectTargetReason::ImportPointer
                            | damsel_core::IndirectTargetReason::ExportAddress
                            | damsel_core::IndirectTargetReason::FunctionPointer
                    ),
                    "unexpected indirect-target reason: {reason}"
                );
            }
        }
    }
    assert!(
        saw_reason,
        "expected at least one IndirectTargetResolved annotation"
    );
}

#[test]
fn disasm_jump_table_candidate_absent_on_non_switch_fixture() {
    let image = load(fixture("arm64-symbolized")).expect("load fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Section("__text".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(256),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble symbolized fixture");
    assert!(result.instructions.iter().all(|instruction| {
        instruction
            .annotations
            .iter()
            .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. }))
    }));
}

#[test]
fn disasm_constant_slot_dispatch_does_not_emit_jump_table_candidate() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_dispatch_second_slot".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result =
        disassemble_v2(&image, &request).expect("disassemble constant-slot dispatch symbol");
    assert!(result.instructions.iter().all(|instruction| {
        instruction
            .annotations
            .iter()
            .all(|annotation| !matches!(annotation, Annotation::JumpTableCandidate { .. }))
    }));
}

#[test]
fn disasm_emits_function_pointer_recovered_values_for_dispatch_fixture_when_present() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_dispatch_second_slot".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble indirect-dispatch fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .recovered_values
            .iter()
            .any(|value| value.kind == RecoveredValueKind::FunctionPointer)
    }));
}

#[test]
fn disasm_emits_export_address_recovered_values_for_export_fixture_when_present() {
    let path = fixture("indirect-dispatch");
    if !path.exists() {
        eprintln!("indirect-dispatch fixture not present; skipping");
        return;
    }
    let image = load(path).expect("load indirect-dispatch fixture");
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_load_export_target".to_string()),
        range: None,
        limit: DisassemblyLimit::Instructions(32),
        options: DisassemblyOptions {
            include_annotations: true,
            include_value_flow: true,
        },
    };
    let result = disassemble_v2(&image, &request).expect("disassemble indirect-dispatch fixture");
    assert!(result.instructions.iter().any(|instruction| {
        instruction
            .recovered_values
            .iter()
            .any(|value| value.kind == RecoveredValueKind::ExportAddress)
    }));
}
