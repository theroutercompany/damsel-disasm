use criterion::{Criterion, black_box, criterion_group, criterion_main};
use damsel_core::{
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequest, DisassemblyRequestV2,
    DisassemblyTarget,
};
use damsel_macho::{disassemble, disassemble_v2, load};
use std::path::Path;

#[derive(Clone)]
enum ScenarioKind {
    Legacy,
    ValueFlow(bool),
}

#[derive(Clone)]
struct Scenario {
    name: &'static str,
    fixture: &'static str,
    target: DisassemblyTarget,
    max_instructions: usize,
    kind: ScenarioKind,
}

fn decode_benchmarks(criterion: &mut Criterion) {
    if std::env::consts::ARCH != "aarch64" {
        return;
    }

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/bin");
    let scenarios = [
        Scenario {
            name: "arm64_symbolized::__text",
            fixture: "arm64-symbolized",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "arm64_symbolized::_main",
            fixture: "arm64-symbolized",
            target: DisassemblyTarget::Symbol("_main".to_string()),
            max_instructions: 64,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "arm64_stripped::__text",
            fixture: "arm64-stripped",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "objc_sample::__text",
            fixture: "objc-sample",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "import_rich::__text",
            fixture: "import-rich",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "import_lazy::__text",
            fixture: "import-lazy",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "import_lazy::__text:value_flow_off",
            fixture: "import-lazy",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(false),
        },
        Scenario {
            name: "import_lazy::__text:value_flow_on",
            fixture: "import-lazy",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "import_lazy::__stub_helper:first_helper",
            fixture: "import-lazy",
            target: DisassemblyTarget::Address(0x1),
            max_instructions: 8,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "semantic_switch::__text",
            fixture: "semantic-switch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "semantic_switch::__text:value_flow_off",
            fixture: "semantic-switch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(false),
        },
        Scenario {
            name: "semantic_switch::__text:value_flow_on",
            fixture: "semantic-switch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "export_kinds::__text",
            fixture: "export-kinds",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "indirect_dispatch::__text:value_flow_off",
            fixture: "indirect-dispatch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(false),
        },
        Scenario {
            name: "indirect_dispatch::__text:value_flow_on",
            fixture: "indirect-dispatch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "indirect_dispatch::_dispatch_second_slot:value_flow_on",
            fixture: "indirect-dispatch",
            target: DisassemblyTarget::Symbol("_dispatch_second_slot".to_string()),
            max_instructions: 32,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "indirect_dispatch::_load_export_target:value_flow_on",
            fixture: "indirect-dispatch",
            target: DisassemblyTarget::Symbol("_load_export_target".to_string()),
            max_instructions: 32,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "relative_dispatch::__text:value_flow_off",
            fixture: "relative-dispatch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(false),
        },
        Scenario {
            name: "relative_dispatch::__text:value_flow_on",
            fixture: "relative-dispatch",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "relative_dispatch::_relative_dispatch_second_slot:value_flow_on",
            fixture: "relative-dispatch",
            target: DisassemblyTarget::Symbol("_relative_dispatch_second_slot".to_string()),
            max_instructions: 32,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "relative_dispatch::_relative_load_export_target:value_flow_on",
            fixture: "relative-dispatch",
            target: DisassemblyTarget::Symbol("_relative_load_export_target".to_string()),
            max_instructions: 32,
            kind: ScenarioKind::ValueFlow(true),
        },
        Scenario {
            name: "duplicate_symbol_ordinal::__text",
            fixture: "duplicate-symbol-ordinal",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
            kind: ScenarioKind::Legacy,
        },
        Scenario {
            name: "arm64e_sample::__text",
            fixture: "arm64e-sample",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 128,
            kind: ScenarioKind::Legacy,
        },
    ];

    for mut scenario in scenarios {
        let fixture = fixture_root.join(scenario.fixture);
        let image = load(&fixture).expect("load benchmark fixture");
        if scenario.name == "import_lazy::__stub_helper:first_helper" {
            scenario.target = DisassemblyTarget::Address(
                image
                    .dyld()
                    .stub_helpers
                    .first()
                    .map(|helper| helper.helper_address)
                    .expect("import-lazy fixture should expose stub helper"),
            );
        }

        match scenario.kind {
            ScenarioKind::Legacy => {
                let request = DisassemblyRequest {
                    target: scenario.target.clone(),
                    max_instructions: Some(scenario.max_instructions),
                    limit: None,
                    include_annotations: true,
                };
                criterion.bench_function(scenario.name, |bench| {
                    bench.iter(|| {
                        disassemble(black_box(&image), black_box(&request)).expect("disassemble")
                    })
                });
            }
            ScenarioKind::ValueFlow(include_value_flow) => {
                let request = DisassemblyRequestV2 {
                    target: scenario.target.clone(),
                    range: None,
                    limit: DisassemblyLimit::Instructions(scenario.max_instructions),
                    options: DisassemblyOptions {
                        include_annotations: true,
                        include_value_flow,
                    },
                };
                criterion.bench_function(scenario.name, |bench| {
                    bench.iter(|| {
                        disassemble_v2(black_box(&image), black_box(&request))
                            .expect("disassemble v2")
                    })
                });
            }
        }
    }
}

criterion_group!(benches, decode_benchmarks);
criterion_main!(benches);
