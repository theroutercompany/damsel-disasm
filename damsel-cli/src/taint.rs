//! Taint query orchestration (plan feature 10, slice 2): build the
//! taint engine over a project's lifted functions and answer
//! parameter-reachability queries. See `damsel_decomp::taint` for the
//! analysis model.

use damsel_decomp::TaintEngine;
use damsel_macho::disassemble_v2;
use damsel_project::{Create, ProjectStore, open_store};
use std::collections::BTreeMap;
use std::path::Path;

/// One function's resolved bound.
struct Bound {
    #[allow(dead_code)]
    name: Option<String>,
    start: u64,
    end: u64,
}

/// Lift every bounded function in the project.
pub fn lifted_functions(
    project_dir: &Path,
) -> Result<
    (
        BTreeMap<u64, damsel_decomp::IrFunction>,
        BTreeMap<u64, String>,
    ),
    String,
> {
    let meta = damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("taint"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let store = open_store(&project_dir.join("project.db"), Create::Existing, &meta)
        .map_err(|e| e.to_string())?;
    let image = crate::load_project_image(&std::path::PathBuf::from(project_dir))
        .map_err(|e| e.to_string())?;

    // Bounds from function_bound facts (payload `start-end|…`).
    let mut bounds: BTreeMap<u64, Bound> = BTreeMap::new();
    for fact in store.facts().map_err(|e| e.to_string())? {
        if fact.kind.as_str() != "function_bound" {
            continue;
        }
        let Some(object) = &fact.object_id else {
            continue;
        };
        let Ok(address) = u64::from_str_radix(object.key().trim_start_matches("0x"), 16) else {
            continue;
        };
        let Some(start) = fact
            .payload
            .split('-')
            .next()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        let Some(end) = fact
            .payload
            .split('-')
            .nth(1)
            .map(|r| r.split('|').next().unwrap_or(""))
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        if end > start {
            bounds.insert(
                address,
                Bound {
                    name: None,
                    start,
                    end,
                },
            );
        }
    }

    let mut functions = BTreeMap::new();
    let mut names = BTreeMap::new();
    for summary in damsel_project::function_summaries(&store).map_err(|e| e.to_string())? {
        let Some(bound) = bounds.get(&summary.address) else {
            continue;
        };
        let request = damsel_core::DisassemblyRequestV2 {
            target: damsel_core::DisassemblyTarget::Address(bound.start),
            range: Some(bound.start..bound.end),
            limit: damsel_core::DisassemblyLimit::Unlimited,
            options: damsel_core::DisassemblyOptions {
                include_annotations: false,
                include_value_flow: false,
            },
        };
        let Ok(result) = disassemble_v2(&image, &request) else {
            continue;
        };
        if result.instructions.is_empty() {
            continue;
        }
        functions.insert(
            summary.address,
            damsel_decomp::lift_arm64(&result.instructions),
        );
        if let Some(name) = summary.name {
            names.insert(summary.address, name);
        }
    }
    Ok((functions, names))
}

/// Build the engine for a project directory.
pub fn engine_for(project_dir: &Path) -> Result<TaintEngine, String> {
    let (functions, names) = lifted_functions(project_dir)?;
    Ok(TaintEngine::new(&functions, names))
}
