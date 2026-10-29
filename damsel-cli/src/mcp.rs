//! MCP server (Wave 7; automation spec §MCP): exposes damsel's typed
//! analysis surface as tools returning structured, provenance-carrying
//! JSON. Built on the rmcp SDK (`modelcontextprotocol/rust-sdk`).
//!
//! v1 tools: `damsel_analyze`, `damsel_functions`, `damsel_xrefs`,
//! `damsel_search`, `damsel_recon`, `damsel_decompile`, `damsel_diff`.

use crate::output::OutputSettings;
use crate::{CliRunError, output};
use std::path::PathBuf;

pub(crate) struct McpConfig {
    pub(crate) project: Option<PathBuf>,
}

pub(crate) fn run_mcp(config: McpConfig) -> Result<(), CliRunError> {
    let _ = config;
    // The rmcp server transport lands with the integration spike;
    // for now the typed tool surface is exercised through batch mode
    // and the CLI JSON contracts (same shapes, same vocabulary).
    Err(CliRunError::command(
        "mcp_not_yet_wired",
        "MCP server transport requires the rmcp serve integration; \
         tool JSON shapes are frozen and tested via batch/CLI (see \
         project_cli tests for the same contracts)",
    ))
}

#[allow(dead_code)]
fn tool_shapes() -> serde_json::Value {
    // The frozen tool contracts (evidence row 7.3). These are the
    // schemas the MCP server will serve; they are exercised today
    // through the CLI JSON tests.
    serde_json::json!({
        "damsel_analyze": {
            "input": {"path": "string"},
            "output": {"facts": "usize", "function_bounds": "usize", "xrefs": "usize", "strings": "usize"}
        },
        "damsel_functions": {
            "input": {"project": "string", "name": "string?"},
            "output": {"functions": [{"address": "string", "name": "string?", "seeds": ["string"]}]}
        },
        "damsel_xrefs": {
            "input": {"project": "string", "to": "string?", "from": "string?"},
            "output": {"xrefs": [{"from": "string", "kind": "string", "to": "string"}]}
        },
        "damsel_search": {
            "input": {"project": "string", "query": "string"},
            "output": {"results": [{"kind": "string", "name": "string", "id": "string"}]}
        },
        "damsel_recon": {
            "input": {"path": "string"},
            "output": {"verdict": "string", "stripped": "bool", "section_entropies": [{"section": "string", "entropy": "f64"}]}
        },
        "damsel_decompile": {
            "input": {"project": "string", "function": "string"},
            "output": {"pseudocode": {"lines": [{"text": "string", "source_addresses": ["string"]}]}}
        },
        "damsel_diff": {
            "input": {"left": "string", "right": "string"},
            "output": {"matched": [{"name": "string?", "left_address": "string", "right_address": "string"}]}
        }
    })
}
