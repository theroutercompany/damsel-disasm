//! MCP server (Wave 7): exposes damsel's typed analysis surface over
//! stdio using JSON-RPC 2.0 (the MCP wire protocol). Tools return
//! structured, provenance-carrying JSON — same shapes as the CLI.
//!
//! Wire: one JSON object per line on stdin/stdout. The MCP handshake
//! (`initialize` → `notifications/initialized`) precedes tool calls.

use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

pub(crate) struct McpConfig {
    pub(crate) project: Option<PathBuf>,
}

pub(crate) fn run_mcp(config: McpConfig) -> Result<(), crate::CliRunError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id_value: Option<Value> = request.get("id").cloned();
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let response: Option<Value> = match method {
            "initialize" => handle_initialize(),
            "notifications/initialized" => None,
            "tools/list" => Some(handle_list_tools()),
            "tools/call" => Some(handle_call_tool(&params, &config)),
            _ => Some(json!({
                "jsonrpc": "2.0",
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            })),
        };

        if let (Some(mut response), Some(request_id)) = (response, id_value) {
            response["id"] = request_id;
            let _ = writeln!(writer, "{}", serde_json::to_string(&response).unwrap_or_default());
            let _ = writer.flush();
        }
    }
    Ok(())
}

fn handle_initialize() -> Option<Value> {
    Some(json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": "2025-03-26",
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "damsel",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    }))
}

fn handle_list_tools() -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": {
            "tools": [
                tool("damsel_analyze", "Analyze a binary into a project database", json!({
                    "type": "object",
                    "properties": { "path": { "type": "string", "description": "Path to the binary" } },
                    "required": ["path"]
                })),
                tool("damsel_functions", "List functions in an analyzed project", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the .damsel directory or target" },
                        "name": { "type": "string", "description": "Filter by name substring" }
                    },
                    "required": ["project"]
                })),
                tool("damsel_xrefs", "Query cross-references by direction", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "to": { "type": "string", "description": "Target address (0x…)" },
                        "from": { "type": "string", "description": "Source address (0x…)" }
                    },
                    "required": ["project"]
                })),
                tool("damsel_search", "Search functions, symbols, imports, strings", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "query": { "type": "string" }
                    },
                    "required": ["project", "query"]
                })),
                tool("damsel_decompile", "Decompile a function to pseudocode", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "function": { "type": "string", "description": "Function name or 0x address" }
                    },
                    "required": ["project", "function"]
                })),
                tool("damsel_recon", "Triage a binary (entropy, packers, anti-analysis)", json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                })),
                tool("damsel_diff", "Diff two analyzed projects", json!({
                    "type": "object",
                    "properties": {
                        "left": { "type": "string" },
                        "right": { "type": "string" }
                    },
                    "required": ["left", "right"]
                })),
                tool("damsel_identify", "Identify stripped functions by fingerprint matching against a known-library corpus (strings, constants, structure)", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the analyzed project" },
                        "corpus": { "type": "string", "description": "Path to a corpus JSON file (from damsel corpus-export)" },
                        "threshold": { "type": "number", "description": "Minimum match score 0.0-1.0 (default 0.7)" }
                    },
                    "required": ["project"]
                })),
                tool("damsel_corpus_export", "Export a fingerprint corpus from a symbolized project for reuse identification", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the symbolized project" },
                        "out": { "type": "string", "description": "Output corpus JSON path" },
                        "library": { "type": "string", "description": "Library label (e.g. glibc)" }
                    },
                    "required": ["project", "out", "library"]
                }))
            ]
        }
    })
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": schema })
}

fn handle_call_tool(params: &Value, _config: &McpConfig) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "damsel_analyze" => run_cli(&["analyze", get_str(&arguments, "path")]),
        "damsel_functions" => {
            let mut args = vec!["functions", get_str(&arguments, "project")];
            if let Some(name) = arguments.get("name").and_then(|n| n.as_str()) {
                args.extend(["--name", name]);
            }
            run_cli(&args)
        }
        "damsel_xrefs" => {
            let mut args = vec!["xrefs", get_str(&arguments, "project")];
            if let Some(to) = arguments.get("to").and_then(|v| v.as_str()) {
                args.extend(["--to", to]);
            }
            if let Some(from) = arguments.get("from").and_then(|v| v.as_str()) {
                args.extend(["--from", from]);
            }
            run_cli(&args)
        }
        "damsel_search" => run_cli(&["search", get_str(&arguments, "project"), get_str(&arguments, "query")]),
        "damsel_decompile" => run_cli(&["decompile", get_str(&arguments, "project"), "--function", get_str(&arguments, "function")]),
        "damsel_recon" => run_cli(&["recon", get_str(&arguments, "path")]),
        "damsel_diff" => run_cli(&["diff", get_str(&arguments, "left"), get_str(&arguments, "right")]),
        "damsel_identify" => {
            let mut args = vec!["identify".to_string(), get_str(&arguments, "project").to_string()];
            if let Some(corpus) = arguments.get("corpus").and_then(|v| v.as_str()) {
                args.push("--corpus".to_string());
                args.push(corpus.to_string());
            }
            if let Some(threshold) = arguments.get("threshold").and_then(|v| v.as_f64()) {
                args.push("--threshold".to_string());
                args.push(threshold.to_string());
            }
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            run_cli(&refs)
        }
        "damsel_corpus_export" => {
            let args = [
                "corpus-export",
                get_str(&arguments, "project"),
                "--out",
                get_str(&arguments, "out"),
                "--library",
                get_str(&arguments, "library"),
            ];
            run_cli(&args)
        }
        _ => Err(format!("unknown tool: {name}")),
    };

    match result {
        Ok(output) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{ "type": "text", "text": output }],
                "isError": false
            }
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{ "type": "text", "text": error }],
                "isError": true
            }
        }),
    }
}

fn get_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn run_cli(args: &[&str]) -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("failed to find damsel binary: {e}"))?;
    let output = std::process::Command::new(exe)
        .args(args)
        .arg("--format")
        .arg("json")
        .output()
        .map_err(|e| format!("failed to run damsel: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}
