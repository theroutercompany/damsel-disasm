//! External-LLM refiner end-to-end (plan feature 17, slice 2): a
//! one-shot local HTTP mock stands in for the provider API — proving
//! the full loop (contexts → prompt → HTTP with auth → completion
//! parsing → annotation-layer apply with the `model:openai/<model>`
//! actor) without a real key.

use assert_cmd::cargo_bin;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Serve exactly one POST, capture the request, reply with the given
/// JSON body. Returns the server address and the captured request.
fn one_shot_server(response_body: String) -> (String, std::sync::mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    let address = listener.local_addr().expect("addr").to_string();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = [0u8; 64 * 1024];
        let size = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..size]).into_owned();
        let _ = sender.send(request);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    (address, receiver)
}

fn openai_completion(text: &str) -> String {
    serde_json::json!({
        "id": "chatcmpl-mock",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": text}}]
    })
    .to_string()
}

fn analyze_fixture(dir: &Path, name: &str) -> PathBuf {
    let target = dir.join(name);
    std::fs::copy(repo_root().join("fixtures/bin").join(name), &target).expect("copy fixture");
    let status = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("analyze")
        .arg(&target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("analyze");
    assert!(status.success(), "analyze failed");
    target
}

#[test]
fn llm_refine_round_trip_through_mock_provider() {
    let dir = std::env::temp_dir().join(format!("damsel-llm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let target = analyze_fixture(&dir, "arm64-stripped");

    // arm64-stripped: the function at 0x1000004a0 (_helper in the
    // symbolized twin) carries no symbol here — the rename target the
    // mock proposes for.
    let completion = r#"[
        {"address":"0x1000004a0","name":"add_five_helper","evidence":"leaf arithmetic on w0"}
    ]"#;
    let (address, receiver) = one_shot_server(openai_completion(completion));

    let output = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("refine")
        .arg(&target)
        .arg("--llm")
        .arg("--apply")
        .arg("--format")
        .arg("json")
        .env("DAMSEL_LLM_API_KEY", "test-key-123")
        .env("DAMSEL_LLM_MODEL", "test-model")
        .env("DAMSEL_LLM_BASE_URL", format!("http://{address}"))
        .env("DAMSEL_LLM_PROVIDER", "openai")
        .output()
        .expect("refine --llm");
    assert!(
        output.status.success(),
        "refine failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The applied proposal carries the provider/model actor.
    let body: Value = serde_json::from_slice(&output.stdout).expect("refine json");
    assert_eq!(body["actor"], "model:openai/test-model", "{body:#?}");
    assert_eq!(body["applied_count"], 1, "{body:#?}");
    assert_eq!(body["items"][0]["proposed"], "add_five_helper");

    // The request the CLI sent: bearer auth, model, and a prompt that
    // carries the context (address + import counts) but no string
    // content.
    let request = receiver.recv().expect("captured request");
    assert!(request.starts_with("POST /chat/completions"), "{request}");
    // Header names arrive lowercased (HTTP/2-style) — match on value.
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-key-123"),
        "{request}"
    );
    let body_start = request.find("\r\n\r\n").expect("body start");
    let body: Value = serde_json::from_str(&request[body_start + 4..]).expect("request body json");
    assert_eq!(body["model"], "test-model");
    let prompt = body["messages"][0]["content"].as_str().expect("prompt");
    assert!(prompt.contains("0x1000004a0"), "context address in prompt");
    assert!(prompt.contains("strict JSON"), "{prompt}");

    // The annotation landed and is attributed to the model actor.
    let list = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("annotate")
        .arg(&target)
        .arg("--list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("annotate list");
    let body: Value = serde_json::from_slice(&list.stdout).expect("annotations json");
    let annotation = body["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["value"] == "add_five_helper")
        .expect("annotation applied");
    assert_eq!(annotation["actor"], "model:openai/test-model");

    // The renamed function is findable.
    let functions = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("functions")
        .arg(&target)
        .arg("--name")
        .arg("add_five_helper")
        .arg("--format")
        .arg("json")
        .output()
        .expect("functions");
    let body: Value = serde_json::from_slice(&functions.stdout).expect("functions json");
    assert_eq!(body["count"], 1, "{body:#?}");
}

#[test]
fn llm_refine_without_key_is_a_typed_error() {
    let dir = std::env::temp_dir().join(format!("damsel-llm-nokey-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let target = analyze_fixture(&dir, "arm64-symbolized");

    let output = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("refine")
        .arg(&target)
        .arg("--llm")
        .env_remove("DAMSEL_LLM_API_KEY")
        .output()
        .expect("refine --llm");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("refine_unconfigured"), "{stderr}");
    assert!(stderr.contains("DAMSEL_LLM_API_KEY"), "{stderr}");
}

/// Frida live attach (plan feature 12, verification): with frida on
/// PATH (env-gated), the bridge captures the objc-sample's real main
/// call sequence — Greeter init/greeting.
#[test]
fn frida_live_attach_captures_objc_calls() {
    let frida = std::process::Command::new("which")
        .arg("frida")
        .output()
        .expect("which");
    if !frida.status.success() {
        eprintln!("skipping: frida not on PATH");
        return;
    }
    let dir = std::env::temp_dir().join(format!("damsel-frida-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let target = dir.join("objc-sample");
    std::fs::copy(repo_root().join("fixtures/bin/objc-sample"), &target).unwrap();
    let status = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("analyze")
        .arg(&target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("analyze");
    assert!(status.success());

    let output = std::process::Command::new(cargo_bin!("damsel-cli"))
        .arg("frida")
        .arg(&target)
        .arg("--class")
        .arg("Greeter")
        .arg("--duration")
        .arg("5")
        .arg("--max-calls")
        .arg("10")
        .arg("--format")
        .arg("json")
        .output()
        .expect("frida run");
    assert!(
        output.status.success(),
        "frida failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: Value = serde_json::from_slice(&output.stdout).expect("frida json");
    let calls = body["calls"].as_array().expect("calls");
    assert!(
        calls
            .iter()
            .any(|c| c["method_name"] == "init" && c["class_name"] == "Greeter"),
        "init captured: {calls:#?}"
    );
    assert!(
        calls
            .iter()
            .any(|c| c["method_name"] == "greeting" && c["class_name"] == "Greeter"),
        "greeting captured: {calls:#?}"
    );
}
