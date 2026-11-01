//! Ingest pass (`ingest_v1`): promote a standalone Mach-O `BinaryImage`
//! into project sources, objects, and seed facts (Wave 4 spec §4 pass 1,
//! kernel-spec ingest mapping).
//!
//! Canonical payload formats (versioned by the `ingest_v1` provenance
//! pass; changing a format bumps the pass name):
//!
//! - `byte_class`  → `<section_kind>|0x<start>-0x<end>|exec=<0|1>`
//! - `seed`        → `symbol_table|name=<n>|size=<s>` | `function_starts`
//!                 | `objc_method|selector=<s>|class_method=<0|1>`
//!                 | `entry_point`
//! - `symbol`      → `<kind>|0x<addr>|size=<s>` (non-text symbols)
//! - `import`      → `dylib=<name>|lazy=<0|1>|weak=<0|1>`
//!
//! Determinism: facts are emitted in image order (sections, symbols,
//! imports as loaded); the canonical export sorts independently, so two
//! ingests of identical bytes produce byte-identical exports.

use crate::MachoError;
use damsel_core::{BinaryImage, ProjectedBinaryImage, SymbolKind};
use damsel_project::{
    AddressSpaceId, AnalysisFact, AnalysisSourceKind, Confidence, FactKind, FactProvenance,
    ObjectId, SourceRecord, TypedStore,
};
use std::collections::BTreeSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("project store error during ingest: {0}")]
    Store(#[from] damsel_project::StoreError),
    #[error("mach-o analysis error during ingest: {0}")]
    Macho(#[from] MachoError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestOutcome {
    pub source_id: i64,
    pub object_count: usize,
    pub fact_count: usize,
}

const PASS: &str = "ingest_v1";
const CONF: Confidence = Confidence::Exact;

/// Ingest options (used by the recovery-audit gate to hide symbol seeds
/// and measure recall against hidden ground truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestOptions {
    /// Rank-1 symbol-table function seeds (default: enabled).
    pub include_symbol_seeds: bool,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            include_symbol_seeds: true,
        }
    }
}

/// Register the standalone image as a project source and emit seed facts.
pub fn ingest_binary_image(
    store: &mut impl TypedStore,
    image: &BinaryImage,
    analyzer_version: &str,
) -> Result<IngestOutcome, IngestError> {
    ingest_binary_image_with_options(store, image, analyzer_version, &IngestOptions::default())
}

/// Register a dyld shared-cache projected image as a project source and
/// emit seed facts (Wave 1 "cache ingest mapping").
///
/// The source is anchored in the cache vmaddr space and carries the cache
/// UUID plus the projected image index as its content identity, so a
/// projected UIKit and a projected Foundation never share a source.
pub fn ingest_projected_cache_image(
    store: &mut impl TypedStore,
    projected: &ProjectedBinaryImage,
    analyzer_version: &str,
) -> Result<IngestOutcome, IngestError> {
    ingest_projected_cache_image_with_options(
        store,
        projected,
        analyzer_version,
        &IngestOptions::default(),
    )
}

pub fn ingest_projected_cache_image_with_options(
    store: &mut impl TypedStore,
    projected: &ProjectedBinaryImage,
    analyzer_version: &str,
    options: &IngestOptions,
) -> Result<IngestOutcome, IngestError> {
    let provenance = &projected.provenance;
    let source = SourceSpec {
        kind: AnalysisSourceKind::SharedCacheProjectedImage,
        label: Some(provenance.install_name.clone()),
        content_hash: Some(format!(
            "dyld_cache:{}#{}",
            provenance.cache_uuid,
            provenance.image_id.image_index()
        )),
        address_space: damsel_project::names::CACHE_VMADDR,
        note_suffix: Some(format!(
            "|cache_uuid={}|image={}|member={}|image_base={:#x}",
            provenance.cache_uuid,
            provenance.install_name,
            provenance.member_name,
            provenance.image_base_vmaddr
        )),
    };
    ingest_image(store, &projected.image, analyzer_version, options, &source)
}

pub fn ingest_binary_image_with_options(
    store: &mut impl TypedStore,
    image: &BinaryImage,
    analyzer_version: &str,
    options: &IngestOptions,
) -> Result<IngestOutcome, IngestError> {
    let label = image
        .source()
        .file_path()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
    let source = SourceSpec {
        kind: AnalysisSourceKind::MachOImage,
        label,
        content_hash: None,
        address_space: damsel_project::names::IMAGE_VMADDR,
        note_suffix: None,
    };
    ingest_image(store, image, analyzer_version, options, &source)
}

/// Source identity for one ingest pass.
struct SourceSpec {
    kind: AnalysisSourceKind,
    label: Option<String>,
    content_hash: Option<String>,
    address_space: &'static str,
    /// Additional `project_note` payload appended verbatim.
    note_suffix: Option<String>,
}

fn ingest_image(
    store: &mut impl TypedStore,
    image: &BinaryImage,
    analyzer_version: &str,
    options: &IngestOptions,
    source: &SourceSpec,
) -> Result<IngestOutcome, IngestError> {
    let source_id = store.add_source(SourceRecord {
        source_id: 0,
        kind: source.kind.clone(),
        label: source.label.clone(),
        content_hash: source.content_hash.clone(),
        parent_source_id: None,
        address_spaces: vec![AddressSpaceId::new(source.address_space)],
    })?;

    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut objects: BTreeSet<String> = BTreeSet::new();
    let mut fact_count = 0usize;

    macro_rules! emit {
        ($object:expr, $kind:expr, $payload:expr $(,)?) => {{
            let object = $object;
            if let Some(oid) = &object {
                objects.insert(oid.as_str().to_string());
            }
            store.put_fact(AnalysisFact {
                source_id,
                object_id: object,
                kind: FactKind::new($kind),
                payload: $payload,
                confidence: CONF,
                provenance: provenance.clone(),
            })?;
            fact_count += 1;
        }};
    }

    // Sections → byte classification ranges (kernel spec pass 1 seeds).
    for section in image.sections() {
        let key = format!("{},{}", section.segment_name, section.name);
        let object = ObjectId::new("section", key);
        let start = section.address;
        let end = section.address.saturating_add(section.size);
        let payload = format!(
            "{}|{:#x}-{:#x}|exec={}",
            section.kind,
            start,
            end,
            u8::from(section.executable)
        );
        emit!(Some(object), "byte_class", payload);
    }

    // Text symbols → rank-1 function seeds (function-recovery plan §Seed
    // Collection). Non-text symbols → symbol facts. The audit gate hides
    // rank-1 seeds to measure recovery recall.
    for symbol in image.symbols() {
        if !symbol.defined || symbol.address == 0 {
            continue;
        }
        match &symbol.kind {
            SymbolKind::Text if options.include_symbol_seeds => {
                let object = ObjectId::new("function", format!("{:#x}", symbol.address));
                let payload = format!("symbol_table|name={}|size={}", symbol.name, symbol.size);
                emit!(Some(object), "seed", payload);
            }
            SymbolKind::Section | SymbolKind::File => {}
            kind => {
                let object = ObjectId::new("symbol", &symbol.name);
                let payload = format!("{kind}|{:#x}|size={}", symbol.address, symbol.size);
                emit!(Some(object), "symbol", payload);
            }
        }
    }

    // dyld function starts → rank-2 seeds.
    for address in &image.dyld().function_starts {
        let object = ObjectId::new("function", format!("{address:#x}"));
        emit!(Some(object), "seed", "function_starts".to_string());
    }

    // ObjC method IMPs → rank-3 seeds (owner-tagged): class and category
    // method lists both carry `methods`; walk classes, then categories.
    let mut objc_methods: Vec<(String, bool, u64)> = Vec::new();
    for class in &image.objc().classes {
        for method in &class.methods {
            if let Some(imp) = method.implementation {
                objc_methods.push((
                    method.selector.clone().unwrap_or_default(),
                    method.is_class_method,
                    imp,
                ));
            }
        }
    }
    for category in &image.objc().categories {
        for method in &category.methods {
            if let Some(imp) = method.implementation {
                objc_methods.push((
                    method.selector.clone().unwrap_or_default(),
                    method.is_class_method,
                    imp,
                ));
            }
        }
    }
    objc_methods.sort();
    objc_methods.dedup();
    for (selector, is_class_method, imp) in objc_methods {
        if imp == 0 {
            continue;
        }
        let object = ObjectId::new("function", format!("{imp:#x}"));
        let payload = format!(
            "objc_method|selector={selector}|class_method={}",
            u8::from(is_class_method)
        );
        emit!(Some(object), "seed", payload);
    }

    // Entry point → rank-5 seed.
    if let Some(entry) = image.entry_point() {
        if entry != 0 {
            let object = ObjectId::new("function", format!("{entry:#x}"));
            emit!(Some(object), "seed", "entry_point".to_string());
        }
    }

    // Imports → import objects (linkage facts arrive with the xref pass).
    for import in image.imports() {
        let object = ObjectId::new("import", &import.name);
        let payload = format!(
            "dylib={}|lazy={}|weak={}",
            import.dylib,
            u8::from(import.is_lazy),
            u8::from(import.is_weak)
        );
        emit!(Some(object), "import", payload);
    }

    // Project-level ingest marker (sorts first in canonical exports).
    emit!(
        None::<ObjectId>,
        "project_note",
        format!(
            "ingest_complete|arch={}|sections={}|symbols={}|imports={}{}",
            image.architecture(),
            image.sections().len(),
            image.symbols().len(),
            image.imports().len(),
            source.note_suffix.as_deref().unwrap_or("")
        ),
    );

    Ok(IngestOutcome {
        source_id,
        object_count: objects.len(),
        fact_count,
    })
}
