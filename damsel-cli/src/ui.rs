use crate::{
    CliRunError, DisasmFlagArgs, execute_disassembly, map_macho_error, output, parse_address,
};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use damsel_core::{BinaryImage, Section};
use damsel_macho::{analyze_disassembly, load_bytes};
use serde::Deserialize;
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
    from: Option<u64>,
    to: Option<u64>,
    bytes: Option<usize>,
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
        analyze_disassembly(
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
