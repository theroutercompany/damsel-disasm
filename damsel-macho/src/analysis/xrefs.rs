//! Xref harvesting pass (`xref_v1`, Wave 4 spec §4 pass 4–5): run the
//! existing analysis engine over each seeded function window and promote
//! instruction references into stored `xref` facts.
//!
//! Canonical payload format (provenance pass `xref_v1`):
//! `<xref_kind>|to=<object_id>` attached to the source
//! `instruction:<0xADDR>` object. Kinds: `call`, `branch`, `data`,
//! `page`, `import`, `stub`, `stub_helper`, `import_binding`,
//! `indirect_call`, `indirect_branch`, `relocation_evidence`.
//!
//! v1 boundary policy: a function window runs from its seed address to
//! the next seed address or the end of its executable section — the
//! recursive-descent recovery worker (Unit 4.4b) refines bounds later;
//! this pass stays read-only over engine output.

use crate::MachoError;
use crate::disasm::{AnnotationIndex, disassemble_v2_with_index};
use damsel_core::{
    DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget, Reference,
};
use damsel_project::{AnalysisFact, Confidence, FactKind, FactProvenance, ObjectId, TypedStore};
use rayon::prelude::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XrefError {
    #[error("project store error during xref harvest: {0}")]
    Store(#[from] damsel_project::StoreError),
    #[error("disassembly error during xref harvest: {0}")]
    Disassembly(#[from] MachoError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XrefOutcome {
    pub function_windows: usize,
    pub instructions: usize,
    pub xref_count: usize,
}

const PASS: &str = "xref_v1";
/// Merge consecutive function windows into decode chunks of at most this
/// many bytes. Per-window decoding rebuilds a capstone instance and the
/// value-flow pass for every symbol (70k+ on large system images); chunking
/// keeps coverage identical — windows still start at seed addresses and end
/// at the next seed or section end — while collapsing the pass to hundreds
/// of decodes.
const MAX_CHUNK_BYTES: u64 = 256 * 1024;

/// Harvest xref facts for the given seeded function addresses (sorted
/// ascending by the caller; deduplicated here).
pub fn harvest_xrefs(
    store: &mut impl TypedStore,
    image: &damsel_core::BinaryImage,
    source_id: i64,
    function_addresses: &[u64],
    analyzer_version: &str,
) -> Result<XrefOutcome, XrefError> {
    let mut addresses: Vec<u64> = function_addresses.to_vec();
    addresses.sort_unstable();
    addresses.dedup();

    let executable_ranges: Vec<(u64, u64)> = image
        .sections()
        .iter()
        .filter(|s| s.executable && s.size > 0)
        .map(|s| (s.address, s.address.saturating_add(s.size)))
        .collect();

    let provenance = FactProvenance::analyzer(PASS, analyzer_version);
    let annotation_index = AnnotationIndex::build(image);
    let mut outcome = XrefOutcome {
        function_windows: 0,
        instructions: 0,
        xref_count: 0,
    };

    // Plan chunks, decode them in parallel, emit facts serially in chunk
    // order — fact order (and therefore canonical exports) stays identical
    // to the serial pass while the decode work spreads across cores.
    let mut chunks: Vec<(u64, u64, usize)> = Vec::new();
    let mut index = 0usize;
    while index < addresses.len() {
        let address = addresses[index];
        // Window end: next seed, else the end of the containing section.
        let Some(&(_start, section_end)) = executable_ranges
            .iter()
            .find(|(start, end)| address >= *start && address < *end)
        else {
            index += 1;
            continue;
        };

        // Merge following in-section seeds into one decode chunk while the
        // chunk stays within the byte cap.
        let mut next = index + 1;
        while next < addresses.len() {
            let candidate = addresses[next];
            if candidate <= address || candidate > section_end {
                break;
            }
            if candidate - address > MAX_CHUNK_BYTES {
                break;
            }
            next += 1;
        }
        let end = addresses
            .get(next)
            .copied()
            .filter(|next_address| *next_address > address && *next_address <= section_end)
            .unwrap_or(section_end);
        chunks.push((address, end, next - index));
        index = next;
    }

    let decoded: Vec<damsel_core::DisassemblyResultV2> = chunks
        .par_iter()
        .map(|&(address, end, _window_count)| {
            let request = DisassemblyRequestV2 {
                target: DisassemblyTarget::Address(address),
                range: Some(address..end),
                limit: DisassemblyLimit::Unlimited,
                options: DisassemblyOptions {
                    include_annotations: true,
                    include_value_flow: true,
                },
            };
            disassemble_v2_with_index(image, &annotation_index, &request)
        })
        .collect::<Result<Vec<_>, _>>()?;

    for (&(_address, _end, window_count), result) in chunks.iter().zip(&decoded) {
        outcome.function_windows += window_count;
        outcome.instructions += result.instructions.len();
        for instruction in &result.instructions {
            let mut emitted_here = 0usize;
            for reference in &instruction.references {
                let Some((kind, target_object)) = classify_reference(reference) else {
                    continue;
                };
                let object = ObjectId::new("instruction", format!("{:#x}", instruction.address));
                let payload = format!("{kind}|to={target_object}");
                store.put_fact(AnalysisFact {
                    source_id,
                    object_id: Some(object),
                    kind: FactKind::new("xref"),
                    payload,
                    confidence: Confidence::Strong,
                    provenance: provenance.clone(),
                })?;
                emitted_here += 1;
            }
            outcome.xref_count += emitted_here;
        }
    }
    Ok(outcome)
}

/// Map engine references onto the canonical (kind, target-object-id)
/// vocabulary. `None` skips reference classes without a durable target
/// object yet (strings arrive with the strings pass).
fn classify_reference(reference: &Reference) -> Option<(&'static str, String)> {
    match reference {
        Reference::Call { target } => Some(("call", format!("function:{target:#x}"))),
        Reference::Branch { target } => Some(("branch", format!("function:{target:#x}"))),
        Reference::IndirectCall { via } => Some(("indirect_call", format!("register:{via}"))),
        Reference::IndirectBranch { via } => Some(("indirect_branch", format!("register:{via}"))),
        Reference::Page { target } | Reference::Data { target } => {
            Some(("data", format!("data:{target:#x}")))
        }
        Reference::Import { name, dylib, .. } => {
            let _ = dylib;
            Some(("import", format!("import:{name}")))
        }
        Reference::ImportBinding { name, .. } => Some(("import_binding", format!("import:{name}"))),
        Reference::Stub {
            name, stub_address, ..
        } => match name {
            Some(name) => Some(("stub", format!("import:{name}"))),
            None => Some(("stub", format!("stub:{stub_address:#x}"))),
        },
        Reference::StubHelper { helper_address, .. } => {
            Some(("stub_helper", format!("stub:{helper_address:#x}")))
        }
        Reference::RelocationEvidence { address, kind, .. } => {
            let _ = kind;
            Some(("relocation_evidence", format!("data:{address:#x}")))
        }
    }
}
