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
    /// Reserved: default project for tools that accept relative targets.
    #[allow(dead_code)]
    pub(crate) project: Option<PathBuf>,
}

/// Server-side caches for the agent loop: repeated reads reuse open
/// stores, and image materialization (fatal for shared-cache projects —
/// projecting an image dominates its analysis cost) happens once per
/// project instead of once per call.
pub(crate) struct McpState {
    stores: std::collections::HashMap<PathBuf, damsel_project::StoreHandle>,
    images: std::collections::HashMap<PathBuf, damsel_core::BinaryImage>,
    sessions: std::collections::HashMap<PathBuf, damsel_macho::SharedCacheSession>,
}

impl McpState {
    pub(crate) fn new() -> Self {
        Self {
            stores: std::collections::HashMap::new(),
            images: std::collections::HashMap::new(),
            sessions: std::collections::HashMap::new(),
        }
    }

    fn project_dir(&self, project: &str) -> Result<PathBuf, String> {
        crate::resolve_project_dir(std::path::Path::new(project)).map_err(|e| e.to_string())
    }

    /// Run `f` against the (cached) store for a project.
    fn with_store<R>(
        &mut self,
        project: &str,
        f: impl FnOnce(&damsel_project::StoreHandle) -> Result<R, String>,
    ) -> Result<R, String> {
        let dir = self.project_dir(project)?;
        if !self.stores.contains_key(&dir) {
            let meta = damsel_project::AnalysisProjectMeta {
                project_id: damsel_project::ProjectId::new("mcp"),
                name: String::new(),
                analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            };
            let store = damsel_project::open_store(
                &dir.join("project.db"),
                damsel_project::Create::Existing,
                &meta,
            )
            .map_err(|e| e.to_string())?;
            self.stores.insert(dir.clone(), store);
        }
        f(self.stores.get(&dir).expect("just inserted"))
    }

    /// Run `f` against a mutably-borrowed (cached) store, for annotations.
    fn with_store_mut<R>(
        &mut self,
        project: &str,
        f: impl FnOnce(&mut damsel_project::StoreHandle) -> Result<R, String>,
    ) -> Result<R, String> {
        let dir = self.project_dir(project)?;
        if !self.stores.contains_key(&dir) {
            let meta = damsel_project::AnalysisProjectMeta {
                project_id: damsel_project::ProjectId::new("mcp"),
                name: String::new(),
                analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
            };
            let store = damsel_project::open_store(
                &dir.join("project.db"),
                damsel_project::Create::Existing,
                &meta,
            )
            .map_err(|e| e.to_string())?;
            self.stores.insert(dir.clone(), store);
        }
        f(self.stores.get_mut(&dir).expect("just inserted"))
    }

    /// The analyzed image for a project, materialized once and cached.
    /// Shared-cache sessions are cached per cache root, so projecting a
    /// second image from the same cache skips the subcache reads.
    fn image_for(&mut self, project: &str) -> Result<damsel_core::BinaryImage, String> {
        let dir = self.project_dir(project)?;
        if let Some(image) = self.images.get(&dir) {
            return Ok(image.clone());
        }
        let image = match crate::project_image_source(&dir).map_err(|e| e.to_string())? {
            Some(crate::ProjectImageSource::SharedCache { cache, selector }) => {
                let session = match self.sessions.entry(cache.clone()) {
                    std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        let session = damsel_macho::inspect_shared_cache(&cache)
                            .map_err(|e| e.to_string())?;
                        entry.insert(session)
                    }
                };
                session
                    .project_image(&selector)
                    .map_err(|e| e.to_string())?
                    .image
            }
            _ => crate::load_project_image(&dir).map_err(|e| e.to_string())?,
        };
        self.images.insert(dir, image.clone());
        Ok(image)
    }
}

impl Default for McpState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn run_mcp(config: McpConfig) -> Result<(), crate::CliRunError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut state = McpState::new();

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
            "tools/call" => Some(handle_call_tool(&params, &config, &mut state)),
            _ => Some(json!({
                "jsonrpc": "2.0",
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            })),
        };

        if let (Some(mut response), Some(request_id)) = (response, id_value) {
            response["id"] = request_id;
            let _ = writeln!(
                writer,
                "{}",
                serde_json::to_string(&response).unwrap_or_default()
            );
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
    let mut list = json!({
        "jsonrpc": "2.0",
        "result": {
            "tools": [
                tool("damsel_analyze", "Analyze a binary into a project database", json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to the binary, or to a dyld shared cache root when `image` is set" },
                        "image": { "type": "string", "description": "Projected image selector inside a dyld shared cache (install name, id, or basename)" }
                    },
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
                tool("damsel_strings", "List harvested strings (values stay private to the target image)", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "class": { "type": "string", "description": "Filter by class (url, path, format, selector_like)" },
                        "limit": { "type": "integer", "description": "Maximum rows" }
                    },
                    "required": ["project"]
                })),
                tool("damsel_summary", "D0 function summary: calls, imports, strings, problems (facts only, no image needed)", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "function": { "type": "string", "description": "Function name (renames included) or 0x address" }
                    },
                    "required": ["project", "function"]
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
                tool("damsel_query", "Query the call graph: shortest path between functions, reachability, or direct callers", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the .damsel directory or target" },
                        "from": { "type": "string", "description": "Source function (0xADDR or name; renames included)" },
                        "to": { "type": "string", "description": "Sink function (0xADDR or name)" },
                        "callers_of": { "type": "string", "description": "List direct callers of this function instead" }
                    },
                    "required": ["project"]
                })),
                tool("damsel_annotate", "Attach a user annotation to a function (rename, comment, or tag)", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the .damsel directory or target" },
                        "function": { "type": "string", "description": "Target function (0xADDR or current name; renames included)" },
                        "rename": { "type": "string", "description": "New name for the function (replaces any prior rename)" },
                        "comment": { "type": "string", "description": "Comment to attach" },
                        "tag": { "type": "string", "description": "Tag to attach" }
                    },
                    "required": ["project", "function"]
                })),
                tool("damsel_annotate_undo", "Undo the newest annotation for the acting user", json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string", "description": "Path to the .damsel directory or target" }
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
    });
    // Every tool returns a JSON document; declare a permissive output
    // schema so typed clients accept the structuredContent payloads.
    if let Some(tools) = list["result"]["tools"].as_array_mut() {
        for entry in tools {
            entry["outputSchema"] = json!({ "type": "object" });
        }
    }
    list
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": schema })
}

fn handle_call_tool(params: &Value, _config: &McpConfig, state: &mut McpState) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match name {
        "damsel_analyze" => {
            let path = get_str(&arguments, "path");
            match arguments.get("image").and_then(|v| v.as_str()) {
                Some(image) => run_cli(&["analyze", path, "--image", image]),
                None => run_cli(&["analyze", path]),
            }
        }
        // In-process tools: typed reads and mutations over the project
        // store, no subprocess per call (plan feature 19, slice 1).
        "damsel_functions" => in_process_functions(state, &arguments),
        "damsel_xrefs" => in_process_xrefs(state, &arguments),
        "damsel_search" => in_process_search(state, &arguments),
        "damsel_strings" => in_process_strings(state, &arguments),
        "damsel_summary" => in_process_summary(state, &arguments),
        "damsel_annotate" => in_process_annotate(state, &arguments),
        "damsel_annotate_undo" => in_process_annotate_undo(state, &arguments),
        "damsel_decompile" => in_process_decompile(state, &arguments),
        "damsel_recon" => run_cli(&["recon", get_str(&arguments, "path")]),
        "damsel_diff" => run_cli(&[
            "diff",
            get_str(&arguments, "left"),
            get_str(&arguments, "right"),
        ]),
        "damsel_identify" => {
            let mut args = vec![
                "identify".to_string(),
                get_str(&arguments, "project").to_string(),
            ];
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
        Ok(output) => {
            // Typed clients get structuredContent (the tools emit JSON);
            // text-only clients keep the raw JSON string.
            let mut result = json!({
                "content": [{ "type": "text", "text": output }],
                "isError": false
            });
            if let Ok(parsed) =
                serde_json::from_str::<Value>(result["content"][0]["text"].as_str().unwrap_or(""))
            {
                result["structuredContent"] = parsed;
            }
            json!({ "jsonrpc": "2.0", "result": result })
        }
        Err(error) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{ "type": "text", "text": error }],
                "isError": true
            }
        }),
    }
}

fn in_process_functions(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    use damsel_project::ProjectStore;
    let name_filter = arguments.get("name").and_then(|v| v.as_str());
    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l as usize);
    let facts = state.with_store(get_str(arguments, "project"), |store| {
        store.facts().map_err(|e| e.to_string())
    })?;
    let mut bounds = std::collections::BTreeMap::new();
    for fact in &facts {
        if fact.kind.as_str() == "function_bound" {
            if let Some(object) = &fact.object_id {
                bounds.insert(object.as_str().to_string(), fact.payload.clone());
            }
        }
    }
    let mut rows: Vec<Value> = state
        .with_store(get_str(arguments, "project"), |store| {
            damsel_project::function_summaries(store).map_err(|e| e.to_string())
        })?
        .into_iter()
        .into_iter()
        .filter(|summary| {
            name_filter.is_none_or(|needle| {
                summary
                    .name
                    .as_deref()
                    .is_some_and(|name| name.contains(needle))
            })
        })
        .map(|summary| {
            let object = format!("function:{:#x}", summary.address);
            json!({
                "address": format!("{:#x}", summary.address),
                "name": summary.name,
                "bound": bounds.get(&object),
                "seeds": summary.seeds,
            })
        })
        .collect();
    let count = rows.len();
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    Ok(json!({
        "command": "functions",
        "count": count,
        "truncated": limit.is_some_and(|l| count > l),
        "functions": rows,
    })
    .to_string())
}

fn in_process_xrefs(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    use damsel_project::ProjectStore;
    let to_filter = arguments.get("to").and_then(|v| v.as_str());
    let from_filter = arguments.get("from").and_then(|v| v.as_str());
    let kind_filter = arguments.get("kind").and_then(|v| v.as_str());
    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l as usize);
    let mut rows: Vec<Value> = Vec::new();
    let facts = state.with_store(get_str(arguments, "project"), |store| {
        store.facts().map_err(|e| e.to_string())
    })?;
    for fact in &facts {
        if fact.kind.as_str() != "xref" {
            continue;
        }
        let Some(object) = &fact.object_id else {
            continue;
        };
        let from = object.as_str();
        let Some((kind, to)) = fact.payload.split_once("|to=") else {
            continue;
        };
        if let Some(want) = to_filter {
            if !to.contains(want.trim_start_matches("0x")) {
                continue;
            }
        }
        if let Some(want) = from_filter {
            if !from.contains(want.trim_start_matches("0x")) {
                continue;
            }
        }
        if let Some(want) = kind_filter {
            if kind != want {
                continue;
            }
        }
        rows.push(json!({ "from": from, "kind": kind, "to": to }));
        if let Some(limit) = limit {
            if rows.len() >= limit {
                break;
            }
        }
    }
    Ok(json!({
        "command": "xrefs",
        "count": rows.len(),
        "xrefs": rows,
    })
    .to_string())
}

fn in_process_search(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    use damsel_project::ProjectStore;
    let query = get_str(arguments, "query");
    if query.is_empty() {
        return Err("query is required".to_string());
    }
    let kind_filter = arguments.get("kind").and_then(|v| v.as_str());
    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .map_or(50, |l| l as usize);
    let mut rows: Vec<Value> = Vec::new();
    let summaries = state.with_store(get_str(arguments, "project"), |store| {
        damsel_project::function_summaries(store).map_err(|e| e.to_string())
    })?;
    for summary in &summaries {
        if let Some(name) = &summary.name {
            if name.contains(query) && kind_filter.is_none_or(|k| k == "function") {
                rows.push(json!({
                    "kind": "function",
                    "name": name,
                    "address": format!("{:#x}", summary.address),
                }));
            }
        }
    }
    let facts = state.with_store(get_str(arguments, "project"), |store| {
        store.facts().map_err(|e| e.to_string())
    })?;
    for fact in &facts {
        let Some(object) = &fact.object_id else {
            continue;
        };
        let (kind, name) = match (object.kind(), fact.kind.as_str()) {
            ("symbol", "symbol") => ("symbol", object.key().to_string()),
            ("import", "import") => ("import", object.key().to_string()),
            ("section", "byte_class") => ("section", object.key().to_string()),
            _ => continue,
        };
        if kind_filter.is_some_and(|k| k != kind) {
            continue;
        }
        if name.contains(query) {
            rows.push(json!({ "kind": kind, "name": name }));
        }
    }
    let count = rows.len();
    rows.truncate(limit);
    Ok(json!({
        "command": "search",
        "query": query,
        "count": count,
        "truncated": count > limit,
        "results": rows,
    })
    .to_string())
}

fn in_process_strings(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    use damsel_project::ProjectStore;
    let class_filter = arguments.get("class").and_then(|v| v.as_str());
    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|l| l as usize);
    let facts = state.with_store(get_str(arguments, "project"), |store| {
        store.facts().map_err(|e| e.to_string())
    })?;
    let mut classes = std::collections::BTreeMap::new();
    for fact in &facts {
        if fact.kind.as_str() == "string_class" {
            if let Some(object) = &fact.object_id {
                classes.insert(object.as_str().to_string(), fact.payload.clone());
            }
        }
    }
    let mut rows: Vec<Value> = Vec::new();
    for fact in &facts {
        if fact.kind.as_str() != "string" {
            continue;
        }
        let Some(object) = &fact.object_id else {
            continue;
        };
        let (encoding, len_part) = fact.payload.split_once("|len=").unwrap_or(("utf8", "0"));
        let class = classes.get(object.as_str()).cloned();
        if let Some(want) = class_filter {
            if class.as_deref() != Some(want) {
                continue;
            }
        }
        rows.push(json!({
            "address": object.key(),
            "encoding": encoding,
            "bytes": len_part.parse::<u64>().unwrap_or(0),
            "class": class,
            // Values stay out of the store by privacy design; the CLI
            // reads them from the target image when present.
            "value": Value::Null,
        }));
    }
    let count = rows.len();
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    Ok(json!({
        "command": "strings",
        "count": count,
        "values_available": false,
        "strings": rows,
    })
    .to_string())
}

fn in_process_summary(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    let report = state.with_store(get_str(arguments, "project"), |store| {
        use damsel_project::ProjectStore;
        let source = store
            .sources()
            .map_err(|e| e.to_string())?
            .first()
            .ok_or_else(|| "project has no sources".to_string())?
            .source_id;
        damsel_decomp::summarize_named(store, source, get_str(arguments, "function"))
            .map_err(|e| e.to_string())
    })?;
    Ok(json!({ "command": "summary", "summary": report }).to_string())
}

fn in_process_annotate(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    use damsel_project::{AnnotationKind, ObjectId};
    let target = get_str(arguments, "function");
    if target.is_empty() {
        return Err("function is required".to_string());
    }
    let summaries = state.with_store(get_str(arguments, "project"), |store| {
        damsel_project::function_summaries(store).map_err(|e| e.to_string())
    })?;
    let object = if let Some(hex) = target.strip_prefix("0x") {
        let address = u64::from_str_radix(hex, 16).map_err(|_| format!("bad address: {target}"))?;
        ObjectId::new("function", format!("{address:#x}"))
    } else {
        let matches: Vec<_> = summaries
            .iter()
            .filter(|summary| summary.name.as_deref() == Some(target))
            .collect();
        match matches.as_slice() {
            [one] => ObjectId::new("function", format!("{:#x}", one.address)),
            [] => return Err(format!("function_not_found: no function named `{target}`")),
            many => {
                return Err(format!(
                    "ambiguous_function: `{target}` matches {} functions; pass 0xADDR",
                    many.len()
                ));
            }
        }
    };
    let actor = std::env::var("DAMSEL_ACTOR").unwrap_or_else(|_| "mcp".to_string());
    let mut stored: Vec<Value> = Vec::new();
    for (kind, key) in [
        (AnnotationKind::Rename, "rename"),
        (AnnotationKind::Comment, "comment"),
        (AnnotationKind::Tag, "tag"),
    ] {
        let Some(value) = arguments.get(key).and_then(|v| v.as_str()) else {
            continue;
        };
        let outcome = state.with_store_mut(get_str(arguments, "project"), |store| {
            damsel_project::put_annotation(store, &object, kind, value, &actor)
                .map_err(|e| e.to_string())
        })?;
        stored.push(json!({
            "kind": kind.as_str(),
            "value": value,
            "event_seq": outcome.event_seq,
        }));
    }
    if stored.is_empty() {
        return Err("pass rename, comment, or tag".to_string());
    }
    Ok(json!({
        "command": "annotate",
        "action": "added",
        "object": object.as_str(),
        "actor": actor,
        "annotations": stored,
    })
    .to_string())
}

fn in_process_annotate_undo(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    let actor = std::env::var("DAMSEL_ACTOR").unwrap_or_else(|_| "mcp".to_string());
    let undone = state.with_store_mut(get_str(arguments, "project"), |store| {
        damsel_project::undo_last_annotation(store, &actor).map_err(|e| e.to_string())
    })?;
    Ok(json!({
        "command": "annotate",
        "action": "undo",
        "actor": actor,
        "annotation": undone.map(|record| json!({
            "object": record.object_id.as_str(),
            "kind": record.kind.as_str(),
            "value": record.value,
            "actor": record.actor,
        })),
    })
    .to_string())
}

fn in_process_decompile(state: &mut McpState, arguments: &Value) -> Result<String, String> {
    let project = get_str(arguments, "project");
    let function = get_str(arguments, "function");
    // ELF projects keep the CLI's dedicated x86_64 path (subprocess, still
    // structuredContent) until the lift shares this core.
    let dir = state.project_dir(project)?;
    if let Some(crate::ProjectImageSource::File(target)) =
        crate::project_image_source(&dir).map_err(|e| e.to_string())?
    {
        if crate::is_elf_file(&target) {
            return run_cli(&["decompile", project, "--function", function]);
        }
    }
    let image = state.image_for(project)?;
    // Resolve project names (renames included) to addresses; the engine
    // resolves symbols against the image, not the annotation layer.
    let selector = if function.starts_with("0x") {
        function.to_string()
    } else {
        state.with_store(project, |store| {
            let summaries = damsel_project::function_summaries(store).map_err(|e| e.to_string())?;
            let matches: Vec<_> = summaries
                .iter()
                .filter(|summary| summary.name.as_deref() == Some(function))
                .collect();
            match matches.as_slice() {
                [one] => Ok(format!("{:#x}", one.address)),
                [] => Err(format!(
                    "function_not_found: no function named `{function}`"
                )),
                many => Err(format!(
                    "ambiguous_function: `{function}` matches {} functions; pass 0xADDR",
                    many.len()
                )),
            }
        })?
    };
    let decompiled =
        crate::decompile_project_function(&image, &selector).map_err(|e| e.to_string())?;
    Ok(json!({
        "command": "decompile",
        "pseudocode": serde_json::to_value(&decompiled.pseudocode).unwrap_or_default(),
        "frame": decompiled.frame,
        "unknown_count": decompiled.unknown_count,
    })
    .to_string())
}

fn get_str<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn run_cli(args: &[&str]) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("failed to find damsel binary: {e}"))?;
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
