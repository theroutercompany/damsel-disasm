use criterion::{Criterion, black_box, criterion_group, criterion_main};
use damsel_core::{DisassemblyRequest, DisassemblyTarget};
use damsel_macho::{disassemble, load};
use std::path::Path;

struct Scenario {
    name: &'static str,
    fixture: &'static str,
    target: DisassemblyTarget,
    max_instructions: usize,
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
        },
        Scenario {
            name: "arm64_symbolized::_main",
            fixture: "arm64-symbolized",
            target: DisassemblyTarget::Symbol("_main".to_string()),
            max_instructions: 64,
        },
        Scenario {
            name: "arm64_stripped::__text",
            fixture: "arm64-stripped",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
        },
        Scenario {
            name: "objc_sample::__text",
            fixture: "objc-sample",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
        },
        Scenario {
            name: "import_rich::__text",
            fixture: "import-rich",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 256,
        },
        Scenario {
            name: "arm64e_sample::__text",
            fixture: "arm64e-sample",
            target: DisassemblyTarget::Section("__text".to_string()),
            max_instructions: 128,
        },
    ];

    for scenario in scenarios {
        let fixture = fixture_root.join(scenario.fixture);
        let image = load(&fixture).expect("load benchmark fixture");
        let request = DisassemblyRequest {
            target: scenario.target.clone(),
            max_instructions: Some(scenario.max_instructions),
            limit: None,
            include_annotations: true,
        };

        criterion.bench_function(scenario.name, |bench| {
            bench.iter(|| disassemble(black_box(&image), black_box(&request)).expect("disassemble"))
        });
    }
}

criterion_group!(benches, decode_benchmarks);
criterion_main!(benches);
