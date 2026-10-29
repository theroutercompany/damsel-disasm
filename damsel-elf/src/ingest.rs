//! Ingest pass for ELF images (`ingest_v1`, Wave 5B.2): mirrors
//! `damsel_macho::ingest_binary_image` payloads so Wave 4 queries work
//! unchanged across formats. Engine passes (xrefs/recovery/strings) wait
//! on the x86_64 decode mapper — this pass is seed truth only.

use crate::image::ElfDynamicEntry;
use crate::ElfImage;
use damsel_project::{
    AddressSpaceId, AnalysisFact, AnalysisSourceKind, Confidence, FactKind, FactProvenance,
    ObjectId, SourceRecord, TypedStore,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ElfIngestError {
    #[error("project store error during ELF ingest: {0}")]
    Store(#[from] damsel_project::StoreError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfIngestOutcome {
    pub source_id: i64,
    pub object_count: usize,
    pub fact_count: usize,
}

const PASS: &str = "ingest_v1";

pub fn ingest_elf_image(
    store: &mut impl TypedStore,
    image: &ElfImage,
    analyzer_version: &str,
) -> Result<ElfIngestOutcome, ElfIngestError> {
    let source_id = store.add_source(SourceRecord {
        source_id: 0,
        kind: AnalysisSourceKind::ElfObject,
        label: image.label().map(str::to_string),
        content_hash: None,
        parent_source_id: None,
        address_spaces: vec![AddressSpaceId::new(damsel_project::names::ELF_VADDR)],
    })?;

    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let mut objects = std::collections::BTreeSet::new();
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
                confidence: Confidence::Exact,
                provenance: provenance.clone(),
            })?;
            fact_count += 1;
        }};
    }

    // Sections → byte classification ranges (exec flag from sh_flags).
    for section in &image.sections {
        if section.size == 0 {
            continue;
        }
        let object = ObjectId::new("section", &section.name);
        let payload = format!(
            "{}|{:#x}-{:#x}|exec={}",
            section.kind,
            section.address,
            section.address.saturating_add(section.size),
            u8::from(section.executable)
        );
        emit!(Some(object), "byte_class", payload);
    }

    // Defined text symbols → rank-1 function seeds; imports → import facts.
    for symbol in &image.symbols {
        if !symbol.defined || symbol.address == 0 {
            if symbol.is_import && symbol.dynamic {
                let object = ObjectId::new("import", &symbol.name);
                let payload = "dylib=<dynamic>|lazy=0|weak=0".to_string();
                emit!(Some(object), "import", payload);
            }
            continue;
        }
        if symbol.is_text {
            let object = ObjectId::new("function", format!("{:#x}", symbol.address));
            let payload = format!("symbol_table|name={}|size={}", symbol.name, symbol.size);
            emit!(Some(object), "seed", payload);
        }
    }

    // Dynamic relocations → binding facts (PLT/GOT; Wave 5B.6).
    for relocation in &image.relocations {
        let payload = match &relocation.symbol {
            Some(symbol) => format!(
                "{}|got={:#x}|name={}{}|addend={}",
                relocation.kind,
                relocation.offset,
                symbol.name,
                symbol
                    .version
                    .as_deref()
                    .map(|v| format!("@{v}"))
                    .unwrap_or_default(),
                relocation.addend
            ),
            None => format!(
                "{}|got={:#x}|addend={}",
                relocation.kind, relocation.offset, relocation.addend
            ),
        };
        emit!(None::<ObjectId>, "reloc", payload);
    }

    // Dynamic linkage → dependency facts (mirrors cache image-deps).
    for entry in &image.dynamic {
        if let ElfDynamicEntry::Needed(needed) = entry {
            let payload = format!("dt_needed|{needed}");
            emit!(None::<ObjectId>, "linkage", payload);
        }
    }

    emit!(
        None::<ObjectId>,
        "project_note",
        format!(
            "ingest_complete|arch={}|pie={}|sections={}|symbols={}",
            image.architecture,
            u8::from(image.is_position_independent),
            image.sections.len(),
            image.symbols.len()
        ),
    );

    Ok(ElfIngestOutcome {
        source_id,
        object_count: objects.len(),
        fact_count,
    })
}
