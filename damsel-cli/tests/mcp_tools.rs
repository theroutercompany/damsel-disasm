//! MCP server contract tests (plan feature 19, slice 1): in-process tools,
//! structuredContent payloads, and the annotate → functions → undo agent
//! loop over stdio JSON-RPC.

use assert_cmd::Command;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn smoke_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("damsel-mcp-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create smoke dir");
    dir
}

struct McpSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpSession {
    fn start(project: &Path) -> Self {
        use std::process::Command as StdCommand;
        let bin = assert_cmd::cargo_bin!("damsel-cli");
        let mut child = StdCommand::new(bin)
            .arg("serve")
            .env("DAMSEL_ACTOR", "tester")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .current_dir(project)
            .spawn()
            .expect("spawn damsel serve");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let line = serde_json::to_string(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut buffer = String::new();
            let read = self.stdout.read_line(&mut buffer).expect("read line");
            assert!(read > 0, "server closed stdin");
            if buffer.trim().is_empty() {
                continue;
            }
            let response: Value = serde_json::from_str(buffer.trim()).expect("json line");
            if response.get("id").and_then(|v| v.as_u64()) == Some(id) {
                return response;
            }
        }
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        )
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_lists_twelve_tools_with_output_schemas() {
    let dir = smoke_dir("tools-list");
    let mut session = McpSession::start(&dir);
    session.request("initialize", serde_json::json!({}));
    let list = session.request("tools/list", serde_json::json!({}));
    let tools = list["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 13, "tool inventory: {tools:#?}");
    for entry in tools {
        assert!(
            entry["outputSchema"].is_object(),
            "missing outputSchema on {}",
            entry["name"]
        );
    }
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for expected in [
        "damsel_analyze",
        "damsel_functions",
        "damsel_xrefs",
        "damsel_search",
        "damsel_strings",
        "damsel_summary",
        "damsel_decompile",
        "damsel_recon",
        "damsel_diff",
        "damsel_identify",
        "damsel_annotate",
        "damsel_annotate_undo",
        "damsel_corpus_export",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }
}

#[test]
fn mcp_agent_loop_functions_annotate_undo_in_process() {
    let dir = smoke_dir("agent-loop");
    let target = dir.join("arm64-symbolized");
    std::fs::copy(repo_root().join("fixtures/bin/arm64-symbolized"), &target).unwrap();

    let mut session = McpSession::start(&dir);
    session.request("initialize", serde_json::json!({}));

    // analyze (subprocess tool) returns structuredContent too.
    let analyzed = session.call_tool(
        "damsel_analyze",
        serde_json::json!({ "path": target.display().to_string() }),
    );
    assert!(
        !analyzed["result"]["isError"].as_bool().unwrap_or(true),
        "analyze failed: {analyzed:#?}"
    );
    let structured = analyzed["result"]["structuredContent"].clone();
    assert_eq!(structured["command"], "analyze");
    assert!(structured["facts"].as_u64().unwrap_or(0) > 0);

    let project = dir.join("arm64-symbolized.damsel");
    let functions = session.call_tool(
        "damsel_functions",
        serde_json::json!({ "project": project.display().to_string(), "name": "_main" }),
    );
    let structured = functions["result"]["structuredContent"].clone();
    assert_eq!(structured["command"], "functions");
    assert_eq!(structured["count"], 1);
    let address = structured["functions"][0]["address"]
        .as_str()
        .unwrap()
        .to_string();

    // Agent rename writes through the in-process mutation path.
    session.call_tool(
        "damsel_annotate",
        serde_json::json!({
            "project": project.display().to_string(),
            "function": "_main",
            "rename": "entrypoint",
        }),
    );
    let renamed = session.call_tool(
        "damsel_functions",
        serde_json::json!({ "project": project.display().to_string(), "name": "entrypoint" }),
    );
    let structured = renamed["result"]["structuredContent"].clone();
    assert_eq!(
        structured["count"], 1,
        "renamed function visible through in-process functions: {structured:#?}"
    );
    assert_eq!(
        structured["functions"][0]["address"].as_str(),
        Some(address.as_str())
    );

    // Summary resolves renamed functions (facts only).
    let summary = session.call_tool(
        "damsel_summary",
        serde_json::json!({
            "project": project.display().to_string(),
            "function": "entrypoint",
        }),
    );
    assert!(
        !summary["result"]["isError"].as_bool().unwrap_or(true),
        "summary failed: {summary:#?}"
    );

    // Decompile runs in-process against the cached image (two calls:
    // materialization + cache hit) and returns typed pseudocode.
    for _ in 0..2 {
        let decompiled = session.call_tool(
            "damsel_decompile",
            serde_json::json!({
                "project": project.display().to_string(),
                "function": "entrypoint",
            }),
        );
        assert!(
            !decompiled["result"]["isError"].as_bool().unwrap_or(true),
            "decompile failed: {decompiled:#?}"
        );
        let structured = decompiled["result"]["structuredContent"].clone();
        assert_eq!(structured["command"], "decompile");
        assert!(
            structured["pseudocode"]["lines"]
                .as_array()
                .is_some_and(|l| !l.is_empty()),
            "expected pseudocode lines"
        );
    }

    // Xrefs for the function return typed rows.
    let xrefs = session.call_tool(
        "damsel_xrefs",
        serde_json::json!({ "project": project.display().to_string(), "from": &address }),
    );
    let structured = xrefs["result"]["structuredContent"].clone();
    assert_eq!(structured["command"], "xrefs");

    // Undo reverts the rename for the acting user.
    session.call_tool(
        "damsel_annotate_undo",
        serde_json::json!({ "project": project.display().to_string() }),
    );
    let reverted = session.call_tool(
        "damsel_functions",
        serde_json::json!({ "project": project.display().to_string(), "name": "_main" }),
    );
    let structured = reverted["result"]["structuredContent"].clone();
    assert_eq!(structured["count"], 1, "seed name returns after undo");
}
