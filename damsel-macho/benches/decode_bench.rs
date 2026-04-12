use criterion::{Criterion, black_box, criterion_group, criterion_main};
use damsel_core::{DisassemblyRequest, DisassemblyTarget};
use damsel_macho::{disassemble, load};
use std::path::Path;

fn decode_benchmarks(criterion: &mut Criterion) {
    if std::env::consts::ARCH != "aarch64" {
        return;
    }

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/bin/arm64-symbolized");
    let image = load(&fixture).expect("load benchmark fixture");
    let request = DisassemblyRequest {
        target: DisassemblyTarget::Section("__text".to_string()),
        max_instructions: Some(256),
    };

    criterion.bench_function("arm64_symbolized::__text", |bench| {
        bench.iter(|| disassemble(black_box(&image), black_box(&request)).expect("disassemble"))
    });
}

criterion_group!(benches, decode_benchmarks);
criterion_main!(benches);
