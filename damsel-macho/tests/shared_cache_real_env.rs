use damsel_core::{
    CacheImageRecord, CacheLookupResult, DisassemblyLimit, DisassemblyOptions,
    DisassemblyRequestV2, DisassemblyTarget,
};
use damsel_macho::{analyze_disassembly, disassemble_v2, inspect_shared_cache};
use std::collections::BTreeSet;
use std::path::PathBuf;

const REAL_CACHE_ROOT_ENV: &str = "DAMSEL_REAL_DYLD_SHARED_CACHE_ROOT";
const REAL_CACHE_IMAGE_SAMPLE_LIMIT_ENV: &str = "DAMSEL_REAL_DYLD_SHARED_CACHE_IMAGE_SAMPLE_LIMIT";
const REAL_CACHE_INSTRUCTION_LIMIT_ENV: &str = "DAMSEL_REAL_DYLD_SHARED_CACHE_INSTRUCTION_LIMIT";
const DEFAULT_IMAGE_SAMPLE_LIMIT: usize = 6;
const DEFAULT_INSTRUCTION_LIMIT: usize = 8;
const PREFERRED_IMAGE_INSTALL_NAMES: &[&str] = &[
    "/usr/lib/libSystem.B.dylib",
    "/usr/lib/libobjc.A.dylib",
    "/usr/lib/system/libdispatch.dylib",
    "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation",
    "/System/Library/Frameworks/Foundation.framework/Versions/C/Foundation",
];

#[test]
fn real_shared_cache_env_projected_image_smoke() {
    let Some(root) = std::env::var_os(REAL_CACHE_ROOT_ENV) else {
        return;
    };
    let root = PathBuf::from(root);
    let sample_limit = positive_usize_env(
        REAL_CACHE_IMAGE_SAMPLE_LIMIT_ENV,
        DEFAULT_IMAGE_SAMPLE_LIMIT,
    );
    let instruction_limit =
        positive_usize_env(REAL_CACHE_INSTRUCTION_LIMIT_ENV, DEFAULT_INSTRUCTION_LIMIT);
    let session = inspect_shared_cache(&root).expect("inspect real shared cache");
    let cache = session.cache();
    assert!(
        !cache.images().is_empty(),
        "real cache exposes at least one image"
    );

    println!(
        "real-cache root={} uuid={} arch={} images={} members={} sample_limit={} instruction_limit={}",
        root.display(),
        cache.header().cache_uuid,
        cache.header().architecture,
        cache.header().image_count,
        cache.members().len(),
        sample_limit,
        instruction_limit
    );

    let samples = select_real_cache_candidates(cache.images(), sample_limit);
    assert!(
        !samples.is_empty(),
        "real cache sample selection produced at least one image"
    );

    let mut export_lookup_count = 0usize;
    let mut successful_sample_count = 0usize;
    let mut skipped_unbacked_exec_count = 0usize;
    let mut skipped_empty_exec_count = 0usize;
    for image in samples {
        if successful_sample_count >= sample_limit {
            break;
        }

        let projected = session
            .project_image(image.id.as_str())
            .unwrap_or_else(|error| {
                panic!("project real cache image {}: {error}", image.install_name)
            });

        assert!(
            !projected.image.sections().is_empty(),
            "projected image {} has sections",
            image.install_name
        );
        assert!(
            !projected.image.symbols().is_empty() || !projected.image.imports().is_empty(),
            "projected image {} has symbols or imports",
            image.install_name
        );

        let mut disassembled_section = None;
        let mut has_backed_executable_section = false;
        for section in projected.image.sections().iter().filter(|section| {
            section.executable && section.file_offset.is_some() && section.file_size > 0
        }) {
            has_backed_executable_section = true;
            let result = disassemble_v2(
                &projected.image,
                &DisassemblyRequestV2 {
                    target: DisassemblyTarget::Section(section.name.clone()),
                    range: None,
                    limit: DisassemblyLimit::Instructions(instruction_limit),
                    options: DisassemblyOptions {
                        include_annotations: true,
                        include_value_flow: true,
                    },
                },
            )
            .unwrap_or_else(|error| {
                panic!(
                    "disassemble projected image {} section {}: {error}",
                    image.install_name, section.name
                )
            });
            if !result.instructions.is_empty() {
                disassembled_section = Some((section.name.clone(), result));
                break;
            }
        }

        let Some((section_name, result)) = disassembled_section else {
            if has_backed_executable_section {
                skipped_empty_exec_count += 1;
                println!(
                    "skip image={} reason=no-decoded-instructions-in-backed-executable-sections",
                    image.install_name
                );
            } else {
                skipped_unbacked_exec_count += 1;
                println!(
                    "skip image={} reason=no-file-backed-executable-section",
                    image.install_name
                );
            }
            continue;
        };

        assert!(
            !result.instructions.is_empty(),
            "projected image {} disassembles at least one instruction",
            image.install_name
        );
        let analysis = analyze_disassembly(
            result.target.clone(),
            result.start_address,
            result.end_address,
            &result.instructions,
        );
        assert_eq!(analysis.instruction_count, result.instructions.len());
        assert!(
            analysis.summary.basic_block_count > 0,
            "projected image {} has at least one basic block",
            image.install_name
        );

        let exports = session
            .exports_for_image(image.id.as_str())
            .unwrap_or_else(|error| {
                panic!(
                    "exports for projected image {}: {error}",
                    image.install_name
                )
            });
        if let Some(export) = exports.iter().find(|record| record.cache_vmaddr.is_some()) {
            let lookup = session
                .lookup_cache_vmaddr(export.cache_vmaddr.expect("export address"))
                .unwrap_or_else(|error| {
                    panic!(
                        "lookup export for projected image {}: {error}",
                        image.install_name
                    )
                });
            match lookup {
                CacheLookupResult::ExactSymbol { .. }
                | CacheLookupResult::NearestSymbol { .. }
                | CacheLookupResult::MappingOnly { .. } => {}
            }
            export_lookup_count += 1;
        }

        let dependencies = session
            .image_dependencies(image.id.as_str())
            .unwrap_or_else(|error| {
                panic!(
                    "dependencies for projected image {}: {error}",
                    image.install_name
                )
            });

        let reexports = session
            .reexports(image.id.as_str())
            .unwrap_or_else(|error| {
                panic!(
                    "reexports for projected image {}: {error}",
                    image.install_name
                )
            });

        println!(
            "sample image={} section={} instructions={} blocks={} edges={} exports={} dependencies={} reexports={}",
            image.install_name,
            section_name,
            result.instructions.len(),
            analysis.summary.basic_block_count,
            analysis.summary.edge_count,
            exports.len(),
            dependencies.len(),
            reexports.len()
        );
        successful_sample_count += 1;
    }

    assert_eq!(
        successful_sample_count,
        sample_limit.min(cache.images().len()),
        "real-cache smoke should find the requested number of disassemblable image samples; skipped_unbacked_exec_count={skipped_unbacked_exec_count} skipped_empty_exec_count={skipped_empty_exec_count}"
    );
    assert!(
        export_lookup_count > 0,
        "at least one sampled real-cache export address can be looked up"
    );
}

fn positive_usize_env(name: &str, default: usize) -> usize {
    match std::env::var(name) {
        Ok(value) => {
            let parsed = value
                .parse::<usize>()
                .unwrap_or_else(|error| panic!("{name} must be a positive integer: {error}"));
            assert!(parsed > 0, "{name} must be greater than zero");
            parsed
        }
        Err(std::env::VarError::NotPresent) => default,
        Err(error) => panic!("{name} is not valid UTF-8: {error}"),
    }
}

fn select_real_cache_candidates(
    images: &[CacheImageRecord],
    sample_limit: usize,
) -> Vec<&CacheImageRecord> {
    let mut selected = Vec::<usize>::new();
    let mut seen = BTreeSet::<usize>::new();

    for install_name in PREFERRED_IMAGE_INSTALL_NAMES {
        if let Some(index) = images
            .iter()
            .position(|image| image.install_name == *install_name)
        {
            push_sample_index(&mut selected, &mut seen, index);
        }
    }

    let image_count = images.len();
    if image_count > 0 {
        let spaced_count = sample_limit.saturating_mul(4).min(image_count);
        if spaced_count == 1 {
            push_sample_index(&mut selected, &mut seen, 0);
        } else {
            for ordinal in 0..spaced_count {
                let index = ordinal * (image_count - 1) / (spaced_count - 1);
                push_sample_index(&mut selected, &mut seen, index);
            }
        }
    }

    for index in 0..images.len() {
        push_sample_index(&mut selected, &mut seen, index);
    }

    selected
        .into_iter()
        .map(|index| &images[index])
        .collect::<Vec<_>>()
}

fn push_sample_index(selected: &mut Vec<usize>, seen: &mut BTreeSet<usize>, index: usize) {
    if seen.insert(index) {
        selected.push(index);
    }
}
