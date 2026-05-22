use reqwest::blocking::{Client, Response, multipart};
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures/bin").join(name)
}

struct UiServer {
    child: Child,
    base_url: String,
}

impl Drop for UiServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl UiServer {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_damsel-cli"))
            .args(["ui", "--no-open", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn ui server");
        let stdout = child.stdout.take().expect("stdout pipe");
        let mut stdout = BufReader::new(stdout);
        let mut line = String::new();
        stdout.read_line(&mut line).expect("read ui server line");
        let base_url = line
            .split_whitespace()
            .last()
            .expect("ui server url")
            .trim()
            .to_string();
        let client = http_client();
        wait_until_ready(&client, &base_url);
        Self { child, base_url }
    }
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("http client")
}

fn wait_until_ready(client: &Client, base_url: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(response) = client.get(base_url).send()
            && response.status().is_success()
        {
            break;
        }
        assert!(Instant::now() < deadline, "ui server did not become ready");
        thread::sleep(Duration::from_millis(50));
    }
}

fn upload_fixture(client: &Client, server: &UiServer, name: &str) -> Value {
    let path = fixture(name);
    let file_name = path.file_name().expect("fixture file").to_string_lossy();
    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(std::fs::read(&path).expect("read fixture"))
            .file_name(file_name.into_owned()),
    );
    let response = client
        .post(format!("{}/api/images", server.base_url))
        .multipart(form)
        .send()
        .expect("upload image");
    assert!(
        response.status().is_success(),
        "upload failed: {response:?}"
    );
    response.json().expect("upload json")
}

fn parse_json(response: Response) -> Value {
    response.json().expect("json body")
}

#[test]
fn ui_upload_returns_metadata_and_default_section() {
    let client = http_client();
    let server = UiServer::spawn();

    let payload = upload_fixture(&client, &server, "semantic-switch");

    assert_eq!(payload["fileName"], "semantic-switch");
    assert_eq!(payload["architecture"], "arm64");
    assert_eq!(payload["defaultSection"], "__TEXT:__text");
    assert!(payload["imageId"].as_str().is_some());
    assert!(payload["entryPoint"].is_number());
    assert!(
        payload["sections"]
            .as_array()
            .is_some_and(|sections| !sections.is_empty())
    );
}

#[test]
fn ui_disasm_supports_section_symbol_and_address_targets() {
    let client = http_client();
    let server = UiServer::spawn();
    let upload = upload_fixture(&client, &server, "semantic-switch");
    let image_id = upload["imageId"].as_str().expect("image id");

    let section_response = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "section", "value": upload["defaultSection"] },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": true, "includeAnalysis": true },
        }))
        .send()
        .expect("section request");
    assert!(section_response.status().is_success());
    let section_json = parse_json(section_response);
    let first_address = section_json["instructions"][0]["address"]
        .as_u64()
        .expect("section address");
    assert!(
        section_json["analysis"]["summary"]["basic_block_count"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );

    let symbol_response = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "symbol", "value": "_main" },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("symbol request");
    assert!(symbol_response.status().is_success());
    let symbol_json = parse_json(symbol_response);
    assert_eq!(symbol_json["target"], "_main");
    assert!(
        symbol_json["instructions"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );

    let address_response = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "address", "value": format!("0x{first_address:x}") },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("address request");
    assert!(address_response.status().is_success());
    let address_json = parse_json(address_response);
    assert_eq!(address_json["start_address"].as_u64(), Some(first_address));
    assert_eq!(
        address_json["instructions"][0]["address"].as_u64(),
        Some(first_address)
    );
}

#[test]
fn ui_disasm_returns_typed_errors_for_invalid_windows_and_missing_targets() {
    let client = http_client();
    let server = UiServer::spawn();
    let upload = upload_fixture(&client, &server, "semantic-switch");
    let image_id = upload["imageId"].as_str().expect("image id");

    let invalid_window = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "section", "value": upload["defaultSection"] },
            "window": { "bytes": 32, "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("invalid window request");
    assert_eq!(invalid_window.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(parse_json(invalid_window)["code"], "invalid_args");

    let unknown_symbol = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "symbol", "value": "_missing_symbol" },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("unknown symbol request");
    assert_eq!(unknown_symbol.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(parse_json(unknown_symbol)["code"], "symbol_not_found");

    let unknown_section = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "section", "value": "__TEXT:__missing" },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("unknown section request");
    assert_eq!(unknown_section.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(parse_json(unknown_section)["code"], "section_not_found");

    let unmapped_address = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "address", "value": "0x1" },
            "window": { "limit": 8 },
            "options": { "includeAnnotations": true, "includeValueFlow": false },
        }))
        .send()
        .expect("unmapped address request");
    assert_eq!(unmapped_address.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(parse_json(unmapped_address)["code"], "address_not_mapped");
}

#[test]
fn ui_shell_smoke_test_serves_html_assets_and_api() {
    let client = http_client();
    let server = UiServer::spawn();

    let html = client
        .get(&server.base_url)
        .send()
        .expect("fetch html")
        .text()
        .expect("html body");
    assert!(html.contains("data-page=\"damsel-workbench\""));
    assert!(html.contains("data-theme=\"dark\""));
    assert!(html.contains("data-component=\"floating-dock\""));
    assert!(html.contains("data-component=\"floating-load\""));
    assert!(html.contains("Drop Mach-O or choose file"));

    let js = client
        .get(format!("{}/app.js", server.base_url))
        .send()
        .expect("fetch js")
        .text()
        .expect("js body");
    assert!(js.contains("syncWorkbenchState"));
    assert!(js.contains("includeAnalysis"));
    assert!(js.contains("renderAnalysisEdges"));
    assert!(js.contains("Basic Blocks"));

    let upload = upload_fixture(&client, &server, "semantic-switch");
    let image_id = upload["imageId"].as_str().expect("image id");
    let disasm = client
        .post(format!("{}/api/images/{image_id}/disasm", server.base_url))
        .json(&serde_json::json!({
            "target": { "kind": "section", "value": upload["defaultSection"] },
            "window": { "limit": 12 },
            "options": { "includeAnnotations": true, "includeValueFlow": true, "includeAnalysis": true },
        }))
        .send()
        .expect("smoke disasm request");
    assert!(disasm.status().is_success());
    let disasm_json = parse_json(disasm);
    assert!(
        disasm_json["instructions"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    assert!(
        disasm_json["analysis"]["edges"]
            .as_array()
            .is_some_and(|edges| !edges.is_empty())
    );
}
