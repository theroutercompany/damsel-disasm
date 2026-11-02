use crate::{
    CliRunError, DisasmFlagArgs, execute_disassembly, map_macho_error, output, parse_address,
};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use damsel_core::{BinaryImage, Section};
use damsel_macho::{analyze_disassembly_with_image, load_bytes};
use serde::{Deserialize, Deserializer, de};
use serde_json::json;
use std::collections::HashMap;
use std::io::{self, Write as _};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");
const STYLES_CSS: &str = include_str!("../ui/styles.css");
const MAX_UPLOAD_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(crate) struct UiConfig {
    pub(crate) port: u16,
    pub(crate) open_browser: bool,
}

#[derive(Clone)]
struct AppState {
    store: Arc<ImageStore>,
}

struct ImageStore {
    next_id: AtomicU64,
    images: Mutex<HashMap<String, StoredImage>>,
}

struct StoredImage {
    image: Arc<BinaryImage>,
}

#[derive(Clone)]
struct StoredImageHandle {
    image: Arc<BinaryImage>,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    error: output::ErrorResponse,
}

async fn project_functions(
    State(_state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    // For now, project_id is the path to the .damsel directory.
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return ApiError {
            status: StatusCode::NOT_FOUND,
            error: crate::output::ErrorResponse {
                code: "project_not_found",
                message: format!("no project at {}", project_path.display()),
                details: None,
            },
        }
        .into_response();
    }
    let meta = damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("ui"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to open project".into(),
                details: None,
            },
        }
        .into_response();
    };
    let Ok(functions) = damsel_project::function_summaries(&store) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to list functions".into(),
                details: None,
            },
        }
        .into_response();
    };
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);
    let filtered: Vec<_> = functions
        .into_iter()
        .filter(|f| {
            query
                .get("name")
                .map(|pattern| {
                    f.name
                        .as_deref()
                        .is_some_and(|n| n.contains(pattern.as_str()))
                })
                .unwrap_or(true)
        })
        .take(limit)
        .map(|f| {
            json!({
                "address": format!("{:#x}", f.address),
                "name": f.name,
                "seeds": f.seeds,
            })
        })
        .collect();
    Json(json!({ "functions": filtered, "count": filtered.len() })).into_response()
}

async fn project_xrefs(
    State(_state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return ApiError {
            status: StatusCode::NOT_FOUND,
            error: crate::output::ErrorResponse {
                code: "project_not_found",
                message: format!("no project at {}", project_path.display()),
                details: None,
            },
        }
        .into_response();
    }
    let meta = damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("ui"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to open project".into(),
                details: None,
            },
        }
        .into_response();
    };
    let Some(to_addr) = query.get("to").and_then(|v| {
        v.trim_start_matches("0x")
            .parse::<u64>()
            .ok()
            .or_else(|| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok())
    }) else {
        return ApiError {
            status: StatusCode::BAD_REQUEST,
            error: crate::output::ErrorResponse {
                code: "invalid_args",
                message: "missing or invalid `to` address".into(),
                details: None,
            },
        }
        .into_response();
    };
    let Ok(facts) = damsel_project::ProjectStore::facts(&store) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to read facts".into(),
                details: None,
            },
        }
        .into_response();
    };
    let xrefs: Vec<_> = facts
        .iter()
        .filter(|f| f.kind.as_str() == "xref")
        .filter_map(|f| {
            let object = f.object_id.as_ref()?;
            if object.kind() != "instruction" { return None; }
            let from = u64::from_str_radix(object.key().trim_start_matches("0x"), 16).ok()?;
            let (_, target) = f.payload.split_once("|to=")?;
            let (_, addr) = target.split_once(':')?;
            let target_addr = u64::from_str_radix(addr.trim_start_matches("0x"), 16).ok()?;
            if target_addr == to_addr {
                Some(json!({ "from": format!("{from:#x}"), "kind": f.payload.split('|').next().unwrap_or(""), "to": target }))
            } else { None }
        })
        .collect();
    Json(json!({ "xrefs": xrefs, "count": xrefs.len() })).into_response()
}

async fn project_decompile(
    State(_state): State<AppState>,
    Path((project_id, function)): Path<(String, String)>,
) -> Response {
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return ApiError {
            status: StatusCode::NOT_FOUND,
            error: crate::output::ErrorResponse {
                code: "project_not_found",
                message: format!("no project at {}", project_path.display()),
                details: None,
            },
        }
        .into_response();
    }
    let meta = damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("ui"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to open project".into(),
                details: None,
            },
        }
        .into_response();
    };
    let Ok(sources) = damsel_project::ProjectStore::sources(&store) else {
        return ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error: crate::output::ErrorResponse {
                code: "store_error",
                message: "failed to list sources".into(),
                details: None,
            },
        }
        .into_response();
    };
    let Some(source) = sources.first() else {
        return ApiError {
            status: StatusCode::NOT_FOUND,
            error: crate::output::ErrorResponse {
                code: "project_not_found",
                message: "project has no sources".into(),
                details: None,
            },
        }
        .into_response();
    };
    match damsel_decomp::summarize_named(&store, source.source_id, &function) {
        Ok(report) => Json(json!({ "summary": report })).into_response(),
        Err(e) => ApiError {
            status: StatusCode::NOT_FOUND,
            error: crate::output::ErrorResponse {
                code: "function_not_found",
                message: e.to_string(),
                details: None,
            },
        }
        .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct SymbolQuery {
    q: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisasmApiRequest {
    target: ApiDisasmTarget,
    #[serde(default)]
    window: DisasmWindow,
    #[serde(default)]
    options: DisasmOptions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiDisasmTarget {
    kind: TargetKind,
    value: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TargetKind {
    Section,
    Symbol,
    Address,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisasmWindow {
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    from: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_u64")]
    to: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_usize")]
    bytes: Option<usize>,
    #[serde(default, deserialize_with = "deserialize_optional_usize")]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisasmOptions {
    #[serde(default = "default_true")]
    include_annotations: bool,
    #[serde(default)]
    include_value_flow: bool,
    #[serde(default)]
    include_analysis: bool,
}

impl Default for DisasmOptions {
    fn default() -> Self {
        Self {
            include_annotations: true,
            include_value_flow: false,
            include_analysis: false,
        }
    }
}

impl ImageStore {
    fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            images: Mutex::new(HashMap::new()),
        }
    }

    fn insert(&self, image: BinaryImage) -> String {
        let image_id = format!("{:x}", self.next_id.fetch_add(1, Ordering::Relaxed));
        self.images.lock().expect("image store lock").insert(
            image_id.clone(),
            StoredImage {
                image: Arc::new(image),
            },
        );
        image_id
    }

    fn get(&self, image_id: &str) -> Option<StoredImageHandle> {
        self.images
            .lock()
            .expect("image store lock")
            .get(image_id)
            .map(|stored| StoredImageHandle {
                image: Arc::clone(&stored.image),
            })
    }
}

impl ApiError {
    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            error: output::ErrorResponse {
                code,
                message: message.into(),
                details: None,
            },
        }
    }
}

impl From<CliRunError> for ApiError {
    fn from(error: CliRunError) -> Self {
        let status = match error.code {
            "image_not_found" => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        };
        Self {
            status,
            error: error.to_error_response(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        json_body(self.status, output::render_error_json(&self.error, false))
    }
}

pub(crate) fn run(config: UiConfig) -> Result<(), CliRunError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CliRunError::command("ui_runtime_error", error.to_string()))?
        .block_on(run_async(config))
}

async fn run_async(config: UiConfig) -> Result<(), CliRunError> {
    let state = AppState {
        store: Arc::new(ImageStore::new()),
    };
    let app = Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles))
        .route("/favicon.ico", get(favicon))
        .route("/api/images", post(upload_image))
        .route("/api/images/{image_id}/symbols", get(symbol_suggestions))
        .route("/api/images/{image_id}/disasm", post(disassemble_image))
        .route("/api/images/{image_id}/decompile", post(decompile_image))
        .route("/api/images/{image_id}/cfg", post(cfg_image))
        .route(
            "/api/projects/{project_id}/functions",
            get(project_functions),
        )
        .route("/api/projects/{project_id}/xrefs", get(project_xrefs))
        .route(
            "/api/projects/{project_id}/decompile/{function}",
            get(project_decompile),
        )
        .route("/api/projects/{project_id}/search", get(project_search))
        .route("/api/projects/{project_id}/strings", get(project_strings))
        .route(
            "/api/projects/{project_id}/summary/{function}",
            get(project_summary),
        )
        .route(
            "/api/projects/{project_id}/pseudocode/{function}",
            get(project_pseudocode),
        )
        .route(
            "/api/projects/{project_id}/cfg/{function}",
            get(project_cfg),
        )
        .route("/api/images/{image_id}/hex", get(image_hex))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", config.port))
        .await
        .map_err(|error| CliRunError::command("ui_bind_error", error.to_string()))?;
    let address = listener
        .local_addr()
        .map_err(|error| CliRunError::command("ui_bind_error", error.to_string()))?;
    let url = format!("http://{address}");

    println!("UI available at {url}");
    io::stdout()
        .flush()
        .map_err(|error| CliRunError::command("ui_stdout_error", error.to_string()))?;

    if config.open_browser
        && let Err(error) = webbrowser::open(&url)
    {
        eprintln!("warning: failed to open browser automatically: {error}");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| CliRunError::command("ui_server_error", error.to_string()))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn app_js() -> Response {
    text_body("application/javascript; charset=utf-8", APP_JS)
}

async fn styles() -> Response {
    text_body("text/css; charset=utf-8", STYLES_CSS)
}

async fn favicon() -> Response {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(axum::body::Body::empty())
        .expect("empty favicon response")
}

/// GET /api/projects/{id}/pseudocode/{function} — the decompiled
/// pseudocode (plan feature 20b): lines with source addresses.
async fn project_pseudocode(
    State(_state): State<AppState>,
    Path((project_id, function)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let project_dir = std::path::PathBuf::from(&project_id);
    let image = crate::load_project_image(&project_dir).map_err(ApiError::from)?;
    let decompiled = crate::decompile_project_function(&image, &function).map_err(|error| {
        let status = if matches!(error.code, "function_not_found" | "symbol_not_found") {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::BAD_REQUEST
        };
        ApiError {
            status,
            error: error.to_error_response(),
        }
    })?;
    let lines: Vec<serde_json::Value> = decompiled
        .pseudocode
        .lines
        .iter()
        .map(|line| {
            json!({
                "text": line.text,
                "addresses": line.source_addresses.iter().map(|a| format!("{a:#x}")).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(json!({
        "function": function,
        "unknown_count": decompiled.unknown_count,
        "lines": lines,
    }))
    .into_response())
}

/// GET /api/projects/{id}/cfg/{function} — the control-flow graph (plan
/// feature 20c): lifted IR blocks with statement counts and edges
/// (branch / conditional-taken / fallthrough), laid out by the client.
async fn project_cfg(
    State(_state): State<AppState>,
    Path((project_id, function)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let project_dir = std::path::PathBuf::from(&project_id);
    let image = crate::load_project_image(&project_dir).map_err(ApiError::from)?;
    let selector = function
        .strip_prefix("0x")
        .map(|hex| {
            u64::from_str_radix(hex, 16)
                .map(damsel_core::DisassemblyTarget::Address)
                .unwrap_or_else(|_| damsel_core::DisassemblyTarget::Symbol(function.clone()))
        })
        .unwrap_or_else(|| damsel_core::DisassemblyTarget::Symbol(function.clone()));
    let request = damsel_core::DisassemblyRequestV2 {
        target: selector,
        range: None,
        limit: damsel_core::DisassemblyLimit::Unlimited,
        options: damsel_core::DisassemblyOptions {
            include_annotations: false,
            include_value_flow: false,
        },
    };
    let result = damsel_macho::disassemble_v2(&image, &request)
        .map_err(|_| ApiError::not_found("function_not_found", function.clone()))?;
    if result.instructions.is_empty() {
        return Err(ApiError::not_found("function_not_found", function));
    }
    let lifted = damsel_decomp::lift_arm64(&result.instructions);

    let blocks: Vec<serde_json::Value> = lifted
        .blocks
        .iter()
        .map(|block| {
            let terminator = &block.terminator;
            let (kind, targets) = match terminator {
                damsel_decomp::IrTerminator::Return => ("return".to_string(), vec![]),
                damsel_decomp::IrTerminator::Branch(target) => {
                    ("branch".to_string(), vec![format!("{target:#x}")])
                }
                damsel_decomp::IrTerminator::CondBranch {
                    taken, fallthrough, ..
                } => (
                    "cond_branch".to_string(),
                    vec![format!("{taken:#x}"), format!("{fallthrough:#x}")],
                ),
                damsel_decomp::IrTerminator::WindowEnd => ("window_end".to_string(), vec![]),
                damsel_decomp::IrTerminator::Trap => ("trap".to_string(), vec![]),
                damsel_decomp::IrTerminator::Unknown { mnemonic } => {
                    (format!("unknown:{mnemonic}"), vec![])
                }
            };
            json!({
                "id": block.id,
                "address": format!("{:#x}", block.start_address),
                "statements": block.statements.len(),
                "terminator": kind,
                "targets": targets,
            })
        })
        .collect();
    let edges: Vec<serde_json::Value> = lifted
        .blocks
        .iter()
        .filter_map(|block| {
            let source = format!("{:#x}", block.start_address);
            match &block.terminator {
                damsel_decomp::IrTerminator::Branch(target) => {
                    Some(vec![json!({ "from": source, "to": format!("{target:#x}"), "kind": "branch" })])
                }
                damsel_decomp::IrTerminator::CondBranch { taken, fallthrough, .. } => Some(vec![
                    json!({ "from": source, "to": format!("{taken:#x}"), "kind": "taken" }),
                    json!({ "from": source, "to": format!("{fallthrough:#x}"), "kind": "fallthrough" }),
                ]),
                _ => None,
            }
        })
        .flatten()
        .collect();

    Ok(Json(json!({
        "function": function,
        "blocks": blocks,
        "edges": edges,
    }))
    .into_response())
}

#[derive(Debug, Deserialize)]
struct HexQuery {
    address: Option<String>,
    length: Option<u64>,
}

/// GET /api/images/{id}/hex?address=0x..&length=N — raw bytes for the
/// hex view (plan feature 20b): section-resolved, hex + ascii rows.
async fn image_hex(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
    Query(query): Query<HexQuery>,
) -> Result<Response, ApiError> {
    let stored = lookup_image(&state, &image_id)?;
    let length = query.length.unwrap_or(256).min(4096);
    let address = match query.address.as_deref() {
        None => {
            // Default: the __TEXT:__text section start.
            stored
                .image
                .sections()
                .iter()
                .find(|section| section.full_name() == "__TEXT:__text")
                .map(|section| section.address)
                .ok_or_else(|| ApiError::not_found("hex_unavailable", "no text section"))?
        }
        Some(raw) => parse_address(raw).map_err(|error| {
            CliRunError::invalid_args(format!("invalid address `{raw}`: {error}"))
        })?,
    };

    // Resolve through the containing section.
    let containing = stored.image.sections().iter().find(|section| {
        address >= section.address && address < section.address.saturating_add(section.size)
    });
    let Some(section) = containing else {
        return Err(ApiError::not_found(
            "address_unmapped",
            format!("{address:#x} is not inside any section"),
        ));
    };
    let section_bytes = stored
        .image
        .bytes_for_section(section)
        .ok_or_else(|| ApiError::not_found("hex_unavailable", "section bytes unreadable"))?;
    let offset_into = (address - section.address) as usize;
    let available = section_bytes.len().saturating_sub(offset_into);
    let take = (length as usize).min(available);
    let bytes = &section_bytes[offset_into..offset_into + take];

    // 16-byte rows: offset, hex pairs, ascii.
    let rows: Vec<serde_json::Value> = bytes
        .chunks(16)
        .enumerate()
        .map(|(index, chunk)| {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
            let ascii: String = chunk
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            json!({
                "address": format!("{:#x}", address + (index * 16) as u64),
                "hex": hex,
                "ascii": ascii,
            })
        })
        .collect();

    Ok(Json(json!({
        "address": format!("{address:#x}"),
        "section": section.full_name(),
        "length": take,
        "rows": rows,
    }))
    .into_response())
}

async fn project_search(
    State(_state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return not_found("project_not_found", &project_path.display().to_string());
    }
    let Some(q) = query.get("q") else {
        return bad_request("invalid_args", "missing `q` parameter");
    };
    let meta = open_meta();
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return internal_error("store_error", "failed to open project");
    };
    let Ok(functions) = damsel_project::function_summaries(&store) else {
        return internal_error("store_error", "failed to list functions");
    };
    let results: Vec<_> = functions
        .into_iter()
        .filter(|f| {
            f.name
                .as_deref()
                .is_some_and(|n| n.to_lowercase().contains(&q.to_lowercase()))
        })
        .take(query.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(50))
        .map(|f| json!({ "kind": "function", "name": f.name, "address": format!("{:#x}", f.address) }))
        .collect();
    Json(json!({ "results": results, "count": results.len() })).into_response()
}

async fn project_strings(
    State(_state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return not_found("project_not_found", &project_path.display().to_string());
    }
    let meta = open_meta();
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return internal_error("store_error", "failed to open project");
    };
    let Ok(facts) = damsel_project::ProjectStore::facts(&store) else {
        return internal_error("store_error", "failed to read facts");
    };
    let strings: Vec<_> = facts
        .iter()
        .filter(|f| f.kind.as_str() == "string")
        .filter_map(|f| {
            let object = f.object_id.as_ref()?;
            let address = u64::from_str_radix(object.key().trim_start_matches("0x"), 16).ok()?;
            Some(json!({
                "address": format!("{address:#x}"),
                "kind": f.payload.split('|').next().unwrap_or(""),
                "object": object.as_str(),
            }))
        })
        .take(
            query
                .get("limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(100),
        )
        .collect();
    Json(json!({ "strings": strings, "count": strings.len() })).into_response()
}

async fn project_summary(
    State(_state): State<AppState>,
    Path((project_id, function)): Path<(String, String)>,
) -> Response {
    let project_path = std::path::PathBuf::from(&project_id);
    let db = project_path.join("project.db");
    if !db.is_file() {
        return not_found("project_not_found", &project_path.display().to_string());
    }
    let meta = open_meta();
    let Ok(store) = damsel_project::open_store(&db, damsel_project::Create::Existing, &meta) else {
        return internal_error("store_error", "failed to open project");
    };
    let Ok(sources) = damsel_project::ProjectStore::sources(&store) else {
        return internal_error("store_error", "failed to list sources");
    };
    let Some(source) = sources.first() else {
        return not_found("project_not_found", "project has no sources");
    };
    match damsel_decomp::summarize_named(&store, source.source_id, &function) {
        Ok(report) => Json(json!({ "summary": serde_json::to_value(&report).unwrap_or_default() }))
            .into_response(),
        Err(e) => not_found("function_not_found", &e.to_string()),
    }
}

fn open_meta() -> damsel_project::AnalysisProjectMeta {
    damsel_project::AnalysisProjectMeta {
        project_id: damsel_project::ProjectId::new("ui"),
        name: String::new(),
        analyzer_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn not_found(code: &str, message: &str) -> Response {
    ApiError {
        status: StatusCode::NOT_FOUND,
        error: crate::output::ErrorResponse {
            code: leak_code(code),
            message: message.to_string(),
            details: None,
        },
    }
    .into_response()
}

fn bad_request(code: &str, message: &str) -> Response {
    ApiError {
        status: StatusCode::BAD_REQUEST,
        error: crate::output::ErrorResponse {
            code: leak_code(code),
            message: message.to_string(),
            details: None,
        },
    }
    .into_response()
}

fn internal_error(code: &str, message: &str) -> Response {
    ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        error: crate::output::ErrorResponse {
            code: leak_code(code),
            message: message.to_string(),
            details: None,
        },
    }
    .into_response()
}

async fn upload_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|error| {
            CliRunError::command(
                "upload_error",
                format!("invalid multipart payload: {error}"),
            )
        })?
        .ok_or_else(|| CliRunError::invalid_args("expected one uploaded file"))?;
    let file_name = field
        .file_name()
        .map(ToString::to_string)
        .unwrap_or_else(|| "uploaded-macho".to_string());
    let bytes = field.bytes().await.map_err(|error| {
        CliRunError::command(
            "upload_error",
            format!("failed to read uploaded file: {error}"),
        )
    })?;
    if bytes.is_empty() {
        return Err(CliRunError::invalid_args("uploaded file is empty").into());
    }

    let image = load_bytes(Some(file_name.clone()), bytes.to_vec()).map_err(map_macho_error)?;
    let default_section = choose_default_section(image.sections());
    let sections = image
        .sections()
        .iter()
        .map(|section| {
            json!({
                "name": section.full_name(),
                "shortName": section.name,
                "segmentName": section.segment_name,
                "address": section.address,
                "size": section.size,
                "executable": section.executable,
            })
        })
        .collect::<Vec<_>>();
    let image_id = state.store.insert(image);
    let stored = state.store.get(&image_id).ok_or_else(|| {
        ApiError::not_found("image_not_found", format!("image not found: {image_id}"))
    })?;

    Ok(Json(json!({
        "imageId": image_id,
        "fileName": file_name,
        "architecture": stored.image.architecture().to_string(),
        "entryPoint": stored.image.entry_point(),
        "sections": sections,
        "defaultSection": default_section,
    })))
}

async fn symbol_suggestions(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
    Query(query): Query<SymbolQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let stored = lookup_image(&state, &image_id)?;
    let needle = query.q.unwrap_or_default().trim().to_string();
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    if needle.is_empty() {
        return Ok(Json(json!({ "symbols": [] })));
    }
    let needle_lower = needle.to_ascii_lowercase();
    let mut symbols = stored
        .image
        .symbols()
        .iter()
        .map(|symbol| symbol.name.clone())
        .filter(|name| name.to_ascii_lowercase().contains(&needle_lower))
        .collect::<Vec<_>>();
    symbols.sort_by(|left, right| {
        let left_lower = left.to_ascii_lowercase();
        let right_lower = right.to_ascii_lowercase();
        let left_prefix = left_lower.starts_with(&needle_lower);
        let right_prefix = right_lower.starts_with(&needle_lower);
        right_prefix
            .cmp(&left_prefix)
            .then_with(|| left_lower.cmp(&right_lower))
    });
    symbols.dedup();
    symbols.truncate(limit);
    Ok(Json(json!({ "symbols": symbols })))
}

/// Lift a function from a stored image by API target (symbol or
/// address) — shared by the decompile and cfg image routes.
fn lift_image_function(
    stored: &StoredImageHandle,
    request: &DisasmApiRequest,
) -> Result<damsel_decomp::IrFunction, ApiError> {
    let selector = match request.target.kind {
        TargetKind::Address => format!(
            "{:#x}",
            parse_address(&request.target.value).map_err(|error| {
                CliRunError::invalid_args(format!(
                    "invalid address `{}`: {error}",
                    request.target.value
                ))
            })?
        ),
        _ => request.target.value.clone(),
    };
    let decompiled =
        crate::decompile_project_function(&stored.image, &selector).map_err(|error| {
            let status = if matches!(error.code, "function_not_found" | "symbol_not_found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            ApiError {
                status,
                error: error.to_error_response(),
            }
        })?;
    // Re-lift for the IR function (decompile_project_function returns
    // rendered pseudocode; the CFG needs blocks).
    let request_v2 = damsel_core::DisassemblyRequestV2 {
        target: match request.target.kind {
            TargetKind::Address => damsel_core::DisassemblyTarget::Address(
                parse_address(&request.target.value).unwrap_or_default(),
            ),
            _ => damsel_core::DisassemblyTarget::Symbol(request.target.value.clone()),
        },
        range: None,
        limit: damsel_core::DisassemblyLimit::Unlimited,
        options: damsel_core::DisassemblyOptions {
            include_annotations: false,
            include_value_flow: false,
        },
    };
    let _ = &decompiled;
    let result = damsel_macho::disassemble_v2(&stored.image, &request_v2)
        .map_err(|_| ApiError::not_found("function_not_found", request.target.value.clone()))?;
    if result.instructions.is_empty() {
        return Err(ApiError::not_found(
            "function_not_found",
            request.target.value.clone(),
        ));
    }
    Ok(damsel_decomp::lift_arm64(&result.instructions))
}

/// POST /api/images/{id}/decompile — pseudocode for an uploaded image
/// (plan feature 20b): target {kind, value} like the disasm route.
async fn decompile_image(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
    Json(request): Json<DisasmApiRequest>,
) -> Result<Response, ApiError> {
    let stored = lookup_image(&state, &image_id)?;
    let selector = request.target.value.clone();
    let lifted = lift_image_function(&stored, &request)?;
    let pseudocode = damsel_decomp::render_pseudocode(&lifted, Some(&selector));
    let lines: Vec<serde_json::Value> = pseudocode
        .lines
        .iter()
        .map(|line| {
            json!({
                "text": line.text,
                "addresses": line.source_addresses.iter().map(|a| format!("{a:#x}")).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(Json(json!({
        "function": selector,
        "unknownCount": lifted.unknown_count,
        "lines": lines,
    }))
    .into_response())
}

/// POST /api/images/{id}/cfg — the control-flow graph for an uploaded
/// image function (plan feature 20c).
async fn cfg_image(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
    Json(request): Json<DisasmApiRequest>,
) -> Result<Response, ApiError> {
    let stored = lookup_image(&state, &image_id)?;
    let selector = request.target.value.clone();
    let lifted = lift_image_function(&stored, &request)?;

    let blocks: Vec<serde_json::Value> = lifted
        .blocks
        .iter()
        .map(|block| {
            let terminator = &block.terminator;
            let (kind, targets) = match terminator {
                damsel_decomp::IrTerminator::Return => ("return".to_string(), vec![]),
                damsel_decomp::IrTerminator::Branch(target) => {
                    ("branch".to_string(), vec![format!("{target:#x}")])
                }
                damsel_decomp::IrTerminator::CondBranch {
                    taken, fallthrough, ..
                } => (
                    "cond_branch".to_string(),
                    vec![format!("{taken:#x}"), format!("{fallthrough:#x}")],
                ),
                damsel_decomp::IrTerminator::WindowEnd => ("window_end".to_string(), vec![]),
                damsel_decomp::IrTerminator::Trap => ("trap".to_string(), vec![]),
                damsel_decomp::IrTerminator::Unknown { mnemonic } => {
                    (format!("unknown:{mnemonic}"), vec![])
                }
            };
            json!({
                "id": block.id,
                "address": format!("{:#x}", block.start_address),
                "statements": block.statements.len(),
                "terminator": kind,
                "targets": targets,
            })
        })
        .collect();
    let edges: Vec<serde_json::Value> = lifted
        .blocks
        .iter()
        .filter_map(|block| {
            let source = format!("{:#x}", block.start_address);
            match &block.terminator {
                damsel_decomp::IrTerminator::Branch(target) => {
                    Some(vec![json!({ "from": source, "to": format!("{target:#x}"), "kind": "branch" })])
                }
                damsel_decomp::IrTerminator::CondBranch { taken, fallthrough, .. } => Some(vec![
                    json!({ "from": source, "to": format!("{taken:#x}"), "kind": "taken" }),
                    json!({ "from": source, "to": format!("{fallthrough:#x}"), "kind": "fallthrough" }),
                ]),
                _ => None,
            }
        })
        .flatten()
        .collect();

    Ok(Json(json!({
        "function": selector,
        "blocks": blocks,
        "edges": edges,
    }))
    .into_response())
}

async fn disassemble_image(
    State(state): State<AppState>,
    Path(image_id): Path<String>,
    Json(request): Json<DisasmApiRequest>,
) -> Result<Response, ApiError> {
    let stored = lookup_image(&state, &image_id)?;
    let address = match request.target.kind {
        TargetKind::Address => Some(parse_address(&request.target.value).map_err(|error| {
            CliRunError::invalid_args(format!(
                "invalid address value `{}`: {error}",
                request.target.value
            ))
        })?),
        _ => None,
    };

    let args = match request.target.kind {
        TargetKind::Section => DisasmFlagArgs {
            symbol: None,
            swift_symbol: None,
            addr: None,
            section: Some(request.target.value),
            objc_owner: None,
            objc_selector: None,
            objc_method_kind: None,
            count: None,
            limit: request.window.limit,
            bytes: request.window.bytes,
            from: request.window.from,
            to: request.window.to,
        },
        TargetKind::Symbol => DisasmFlagArgs {
            symbol: Some(request.target.value),
            swift_symbol: None,
            addr: None,
            section: None,
            objc_owner: None,
            objc_selector: None,
            objc_method_kind: None,
            count: None,
            limit: request.window.limit,
            bytes: request.window.bytes,
            from: request.window.from,
            to: request.window.to,
        },
        TargetKind::Address => DisasmFlagArgs {
            symbol: None,
            swift_symbol: None,
            addr: address,
            section: None,
            objc_owner: None,
            objc_selector: None,
            objc_method_kind: None,
            count: None,
            limit: request.window.limit,
            bytes: request.window.bytes,
            from: None,
            to: request.window.to,
        },
    };

    let execution = execute_disassembly(
        &stored.image,
        args,
        request.options.include_annotations,
        request.options.include_value_flow || request.options.include_analysis,
    )?;
    let base_view = execution.view();
    let analysis = request.options.include_analysis.then(|| {
        analyze_disassembly_with_image(
            &stored.image,
            base_view.target.to_string(),
            base_view.start_address,
            base_view.end_address,
            base_view.instructions,
        )
    });
    let view = analysis
        .as_ref()
        .map(|analysis| execution.view_with_analysis(analysis))
        .unwrap_or_else(|| execution.view());
    let body = output::render_disassembly_json(
        view,
        output::DisassemblyRenderOptions {
            include_annotations: request.options.include_annotations,
            include_references: true,
            include_values: request.options.include_value_flow,
        },
        false,
    );
    Ok(json_body(StatusCode::OK, body))
}

fn lookup_image(state: &AppState, image_id: &str) -> Result<StoredImageHandle, ApiError> {
    state.store.get(image_id).ok_or_else(|| {
        ApiError::not_found("image_not_found", format!("image not found: {image_id}"))
    })
}

fn choose_default_section(sections: &[Section]) -> Option<String> {
    sections
        .iter()
        .find(|section| section.full_name() == "__TEXT:__text")
        .or_else(|| sections.iter().find(|section| section.executable))
        .or_else(|| sections.first())
        .map(Section::full_name)
}

fn text_body(content_type: &'static str, body: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, HeaderValue::from_static(content_type))
        .body(axum::body::Body::from(body))
        .expect("static asset response")
}

fn json_body(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        )
        .body(axum::body::Body::from(body))
        .expect("json response")
}

fn default_true() -> bool {
    true
}

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<serde_json::Value>::deserialize(deserializer)? else {
        return Ok(None);
    };
    optional_u64_from_json(value).map_err(de::Error::custom)
}

fn deserialize_optional_usize<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: Deserializer<'de>,
{
    let Some(value) = Option::<serde_json::Value>::deserialize(deserializer)? else {
        return Ok(None);
    };
    optional_u64_from_json(value)
        .map_err(de::Error::custom)?
        .map(|value| {
            usize::try_from(value)
                .map_err(|_| format!("integer value {value} does not fit in usize"))
        })
        .transpose()
        .map_err(de::Error::custom)
}

fn optional_u64_from_json(value: serde_json::Value) -> Result<Option<u64>, String> {
    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| format!("integer value `{number}` is not an unsigned 64-bit integer"))
            .map(Some),
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            parse_address(&trimmed.replace('_', "")).map(Some)
        }
        other => Err(format!(
            "expected an unsigned integer or integer string, got {other}"
        )),
    }
}

fn leak_code(code: &str) -> &'static str {
    Box::leak(code.to_string().into_boxed_str())
}
