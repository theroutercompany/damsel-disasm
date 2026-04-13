use damsel_core::{
    Annotation, BinaryImage, CapabilityStatus, CompatibilityCapability, CompatibilityIssue,
    CompatibilityPolicy, CompatibilityToolRequirement, DecodedInstruction, ExportFlagName,
    ExportKind, HostArchitecture, HostPlatform, Import, ImportBindingKind, ImportBindingRecord,
    ImportBindingSource, ObjcCategoryRecord, ObjcCategoryRecordSource, ObjcClassRecord,
    ObjcIvarRecord, ObjcMethodOwnerKind, ObjcMethodRecord, ObjcNameSource, ObjcPointerKind,
    ObjcPointerRef, ObjcPropertyRecord, ObjcProtocolRecord, ObjcSelectorSource, RecoveredValue,
    Reference, Relocation, Section, SliceDescriptor, StubEntry, StubHelperEntry, StubKind, Symbol,
};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Stdio;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutputSettings {
    pub format: OutputFormat,
    pub pretty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoctorCheckTarget {
    All,
    MachoAnalysis,
    FixtureRebuild,
    FixtureDriftCheck,
    BenchCompile,
    BenchRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoctorRequireStatus {
    Supported,
    SupportedWithDegradedFeatures,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DoctorCheckRequest {
    pub target: DoctorCheckTarget,
    pub require_status: DoctorRequireStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoctorCheckOutcome {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DyldViewOptions {
    pub show_dylibs: bool,
    pub show_rpaths: bool,
    pub show_exports: bool,
    pub show_function_starts: bool,
    pub show_bindings: bool,
    pub show_stubs: bool,
    pub show_helpers: bool,
    pub name_filter: Option<String>,
    pub dylib_filter: Option<String>,
    pub source_filter: Option<ImportBindingSource>,
    pub binding_kind_filter: Option<ImportBindingKind>,
    pub stub_kind_filter: Option<StubKind>,
    pub export_kind_filter: Option<ExportKindFilter>,
    pub export_flag_filter: Option<ExportFlagName>,
    pub ordinal_filter: Option<u32>,
    pub sort: Option<DyldSortKey>,
}

impl DyldViewOptions {
    pub(crate) const fn all() -> Self {
        Self {
            show_dylibs: true,
            show_rpaths: true,
            show_exports: true,
            show_function_starts: true,
            show_bindings: true,
            show_stubs: true,
            show_helpers: true,
            name_filter: None,
            dylib_filter: None,
            source_filter: None,
            binding_kind_filter: None,
            stub_kind_filter: None,
            export_kind_filter: None,
            export_flag_filter: None,
            ordinal_filter: None,
            sort: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportKindFilter {
    Regular,
    Reexport,
    Resolver,
    StubAndResolver,
    WeakDefinition,
    Absolute,
    ThreadLocal,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DyldSortKey {
    Address,
    Name,
    Dylib,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjcViewOptions {
    pub detail: ObjcDetail,
    pub owner_filter: Option<String>,
    pub name_source_filter: Option<ObjcNameSource>,
    pub selector_source_filter: Option<ObjcSelectorSource>,
    pub category_source_filter: Option<ObjcCategorySourceFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjcCategorySourceFilter {
    RuntimeList,
    SymbolSynthesis,
    SymbolSynthesisWithLists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjcDetail {
    Summary,
    Classes,
    Protocols,
    Categories,
    Methods,
    Properties,
    Ivars,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisassemblyRenderOptions {
    pub include_annotations: bool,
    pub include_references: bool,
    pub include_values: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DisassemblyView<'a> {
    pub target: &'a str,
    pub start_address: u64,
    pub bytes_len: usize,
    pub decoded_bytes: usize,
    pub end_address: u64,
    pub instruction_count: usize,
    pub stop_reason: String,
    pub window_end: Option<u64>,
    pub instructions: &'a [DecodedInstruction],
}

#[derive(Debug, Clone)]
pub(crate) struct ErrorResponse {
    pub code: &'static str,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheCollectionMetadata {
    pub total: usize,
    pub returned: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheMemberView {
    pub name: String,
    pub role: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheInfoView {
    pub cache_path: String,
    pub cache_uuid: String,
    pub architecture: String,
    pub member_count: usize,
    pub image_count: usize,
    pub has_local_symbols: bool,
    pub members: Vec<CacheMemberView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheImageView {
    pub id: String,
    pub image_index: u64,
    pub install_name: String,
    pub basename: String,
    pub image_base_vmaddr: u64,
    pub member_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheExportView {
    pub name: String,
    pub cache_vmaddr: Option<u64>,
    pub kind: String,
    pub flags: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheLookupKind {
    ExactSymbol,
    NearestSymbol,
    MappingOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheLookupResultView {
    pub kind: CacheLookupKind,
    pub cache_vmaddr: u64,
    pub image_id: Option<String>,
    pub install_name: Option<String>,
    pub image_base_vmaddr: Option<u64>,
    pub image_offset: Option<u64>,
    pub member_file_offset: Option<u64>,
    pub symbol: Option<String>,
    pub symbol_address: Option<u64>,
    pub offset_from_symbol: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheSymbolicationMatchView {
    pub name: String,
    pub image_id: String,
    pub install_name: String,
    pub cache_vmaddr: u64,
    pub image_base_vmaddr: u64,
    pub image_offset: u64,
    pub member_file_offset: Option<u64>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapabilityReport {
    status: CapabilityStatus,
    reasons: Vec<CompatibilityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolStatus {
    detected: bool,
    usable: bool,
    path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HashToolStatus {
    sha256sum: ToolStatus,
    shasum: ToolStatus,
    openssl: ToolStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorTools {
    xcrun: ToolStatus,
    clang: ToolStatus,
    strip: ToolStatus,
    python3: ToolStatus,
    nm: ToolStatus,
    sdk_path_probe: ToolStatus,
    hash_tools: HashToolStatus,
    selected_hash_tool: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorContext {
    host_platform: HostPlatform,
    host_architecture: HostArchitecture,
    target_triple: Option<String>,
    tools: DoctorTools,
}

const DOCTOR_TEST_SCENARIO_ENV: &str = "DAMSEL_DOCTOR_TEST_SCENARIO";
const DOCTOR_TEST_TARGET_TRIPLE_ENV: &str = "DAMSEL_DOCTOR_TEST_TARGET_TRIPLE";

#[derive(Debug, Clone, PartialEq, Eq)]
struct DoctorReport {
    host_platform: HostPlatform,
    host_architecture: HostArchitecture,
    target_triple: Option<String>,
    overall_status: CapabilityStatus,
    macho_analysis: CapabilityReport,
    fixture_rebuild: CapabilityReport,
    fixture_drift_check: CapabilityReport,
    bench_compile: CapabilityReport,
    bench_runtime: CapabilityReport,
    benchmark: CapabilityReport,
    tools: DoctorTools,
    issues: Vec<CompatibilityIssue>,
}

impl DoctorReport {
    fn capability(&self, capability: CompatibilityCapability) -> &CapabilityReport {
        match capability {
            CompatibilityCapability::MachoAnalysis => &self.macho_analysis,
            CompatibilityCapability::FixtureRebuild => &self.fixture_rebuild,
            CompatibilityCapability::FixtureDriftCheck => &self.fixture_drift_check,
            CompatibilityCapability::Benchmark => &self.benchmark,
            CompatibilityCapability::BenchCompile => &self.bench_compile,
            CompatibilityCapability::BenchRuntime => &self.bench_runtime,
        }
    }

    fn capability_mut(&mut self, capability: CompatibilityCapability) -> &mut CapabilityReport {
        match capability {
            CompatibilityCapability::MachoAnalysis => &mut self.macho_analysis,
            CompatibilityCapability::FixtureRebuild => &mut self.fixture_rebuild,
            CompatibilityCapability::FixtureDriftCheck => &mut self.fixture_drift_check,
            CompatibilityCapability::Benchmark => &mut self.benchmark,
            CompatibilityCapability::BenchCompile => &mut self.bench_compile,
            CompatibilityCapability::BenchRuntime => &mut self.bench_runtime,
        }
    }
}

pub(crate) fn print_error(error: ErrorResponse, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            if let Some(details) = error.details {
                eprintln!("error [{}]: {} ({details})", error.code, error.message);
            } else {
                eprintln!("error [{}]: {}", error.code, error.message);
            }
        }
        OutputFormat::Json => {
            let dto = ErrorJsonDto { error: &error };
            emit_json_response_to_stderr("error", &dto, output);
        }
    }
}

pub(crate) fn render_disassembly_json(
    view: DisassemblyView<'_>,
    render_options: DisassemblyRenderOptions,
    pretty: bool,
) -> String {
    DisasmJsonDto {
        view,
        render_options,
    }
    .to_json_value()
    .render(pretty)
}

pub(crate) fn render_error_json(error: &ErrorResponse, pretty: bool) -> String {
    ErrorJsonDto { error }.to_json_value().render(pretty)
}

pub(crate) fn print_doctor(
    output: &OutputSettings,
    check: Option<DoctorCheckRequest>,
) -> DoctorCheckOutcome {
    let report = collect_doctor_report();
    match output.format {
        OutputFormat::Text => print_doctor_text(&report),
        OutputFormat::Json => {
            emit_json_response("doctor", &DoctorJsonDto { report: &report }, output)
        }
    }
    match check {
        Some(request) => evaluate_doctor_check(&report, request),
        None => DoctorCheckOutcome::Passed,
    }
}

pub(crate) fn print_cache_info(info: &CacheInfoView, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            println!("cache: {}", info.cache_path);
            println!("cache_uuid: {}", info.cache_uuid);
            println!("architecture: {}", info.architecture);
            println!("members: {}", info.member_count);
            println!("images: {}", info.image_count);
            println!("has_local_symbols: {}", info.has_local_symbols);
            println!("members:");
            for member in &info.members {
                println!(
                    "  - name={} role={} path={}",
                    member.name, member.role, member.path
                );
            }
        }
        OutputFormat::Json => emit_json_response("cache_info", &CacheInfoJsonDto { info }, output),
    }
}

pub(crate) fn print_cache_images(
    metadata: &CacheCollectionMetadata,
    images: &[CacheImageView],
    output: &OutputSettings,
) {
    match output.format {
        OutputFormat::Text => {
            println!(
                "images: returned={} total={} truncated={}",
                metadata.returned, metadata.total, metadata.truncated
            );
            for image in images {
                println!(
                    "  - id={} index={} base={:#x} install_name={} basename={} member={}",
                    image.id,
                    image.image_index,
                    image.image_base_vmaddr,
                    image.install_name,
                    image.basename,
                    image.member_name
                );
            }
        }
        OutputFormat::Json => emit_json_response(
            "cache_images",
            &CacheImagesJsonDto { metadata, images },
            output,
        ),
    }
}

pub(crate) fn print_cache_image(image: &CacheImageView, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            println!("image:");
            println!("  id={}", image.id);
            println!("  index={}", image.image_index);
            println!("  install_name={}", image.install_name);
            println!("  basename={}", image.basename);
            println!("  image_base_vmaddr={:#x}", image.image_base_vmaddr);
            println!("  member={}", image.member_name);
        }
        OutputFormat::Json => {
            emit_json_response("cache_image", &CacheImageJsonDto { image }, output)
        }
    }
}

pub(crate) fn print_cache_exports(
    image: &CacheImageView,
    metadata: &CacheCollectionMetadata,
    exports: &[CacheExportView],
    output: &OutputSettings,
) {
    match output.format {
        OutputFormat::Text => {
            println!(
                "exports: image={} returned={} total={} truncated={}",
                image.id, metadata.returned, metadata.total, metadata.truncated
            );
            for export in exports {
                println!(
                    "  - addr={} kind={} flags={} {}",
                    export
                        .cache_vmaddr
                        .map(|value| format!("{value:#x}"))
                        .unwrap_or_else(|| "-".to_string()),
                    export.kind,
                    export.flags,
                    export.name
                );
            }
        }
        OutputFormat::Json => emit_json_response(
            "cache_exports",
            &CacheExportsJsonDto {
                image,
                metadata,
                exports,
            },
            output,
        ),
    }
}

pub(crate) fn print_cache_lookup_address(result: &CacheLookupResultView, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            println!(
                "lookup_address: kind={} cache_vmaddr={:#x}",
                cache_lookup_kind_text(result.kind),
                result.cache_vmaddr
            );
            if let Some(image_id) = &result.image_id {
                println!("  image_id={image_id}");
            }
            if let Some(install_name) = &result.install_name {
                println!("  install_name={install_name}");
            }
            if let Some(image_base) = result.image_base_vmaddr {
                println!("  image_base_vmaddr={image_base:#x}");
            }
            if let Some(image_offset) = result.image_offset {
                println!("  image_offset={image_offset:#x}");
            }
            if let Some(file_offset) = result.member_file_offset {
                println!("  member_file_offset={file_offset:#x}");
            }
            if let Some(symbol) = &result.symbol {
                println!("  symbol={symbol}");
            }
            if let Some(symbol_address) = result.symbol_address {
                println!("  symbol_address={symbol_address:#x}");
            }
            if let Some(offset) = result.offset_from_symbol {
                println!("  offset_from_symbol={offset:#x}");
            }
        }
        OutputFormat::Json => emit_json_response(
            "cache_lookup_address",
            &CacheLookupAddressJsonDto { result },
            output,
        ),
    }
}

pub(crate) fn print_cache_resolve_symbol(
    metadata: &CacheCollectionMetadata,
    matches: &[CacheSymbolicationMatchView],
    output: &OutputSettings,
) {
    match output.format {
        OutputFormat::Text => {
            println!(
                "resolve_symbol: returned={} total={} truncated={}",
                metadata.returned, metadata.total, metadata.truncated
            );
            for entry in matches {
                println!(
                    "  - {} cache_vmaddr={:#x} image_id={} install_name={} source={} image_base_vmaddr={:#x} image_offset={:#x} member_file_offset={}",
                    entry.name,
                    entry.cache_vmaddr,
                    entry.image_id,
                    entry.install_name,
                    entry.source,
                    entry.image_base_vmaddr,
                    entry.image_offset,
                    entry
                        .member_file_offset
                        .map(|value| format!("{value:#x}"))
                        .unwrap_or_else(|| "-".to_string())
                );
            }
        }
        OutputFormat::Json => emit_json_response(
            "cache_resolve_symbol",
            &CacheResolveSymbolJsonDto { metadata, matches },
            output,
        ),
    }
}

pub(crate) fn print_info(image: &BinaryImage, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => print_info_text(image),
        OutputFormat::Json => emit_json_response("info", &InfoJsonDto { image }, output),
    }
}

pub(crate) fn print_sections(sections: &[Section], output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            for section in sections {
                println!(
                    "{:>#18x} {:>#8x} {:<8} {:<18} {}",
                    section.address,
                    section.size,
                    if section.executable { "exec" } else { "-" },
                    section.segment_name,
                    section.name
                );
            }
        }
        OutputFormat::Json => emit_json_response("sections", &SectionsJsonDto { sections }, output),
    }
}

pub(crate) fn print_symbols(symbols: &[Symbol], output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            for symbol in symbols {
                println!(
                    "{:>#18x} {:>#8x} {:<8} {:<6} {:<5} {}{}",
                    symbol.address,
                    symbol.size,
                    symbol.kind,
                    if symbol.defined { "def" } else { "undef" },
                    if symbol.global { "glob" } else { "loc" },
                    symbol.name,
                    symbol
                        .section
                        .as_ref()
                        .map(|section| format!(" [{section}]"))
                        .unwrap_or_default()
                );
            }
        }
        OutputFormat::Json => emit_json_response("symbols", &SymbolsJsonDto { symbols }, output),
    }
}

pub(crate) fn print_imports(imports: &[Import], output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            for import in imports {
                println!(
                    "{:>#18} {:<5} {:<5} {:<30} {}",
                    import
                        .address
                        .map(|address| format!("{address:#x}"))
                        .unwrap_or_else(|| "-".to_string()),
                    if import.is_lazy { "lazy" } else { "-" },
                    if import.is_weak { "weak" } else { "-" },
                    import.dylib,
                    import.name
                );
            }
        }
        OutputFormat::Json => emit_json_response("imports", &ImportsJsonDto { imports }, output),
    }
}

pub(crate) fn print_relocations(relocations: &[Relocation], output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            for relocation in relocations {
                println!(
                    "{:>#18x} {:<24} {:<3} {:<18} {:<18} {} addend={}",
                    relocation.address,
                    relocation.section,
                    relocation.size,
                    relocation.kind,
                    relocation.encoding,
                    relocation.target,
                    relocation.addend
                );
            }
        }
        OutputFormat::Json => {
            emit_json_response("relocs", &RelocationsJsonDto { relocations }, output)
        }
    }
}

pub(crate) fn print_objc(image: &BinaryImage, view: &ObjcViewOptions, output: &OutputSettings) {
    let objc = image.objc();
    let filtered = filter_objc_view(objc, view);
    match output.format {
        OutputFormat::Text => {
            if let Some(flags) = objc.image_info_flags {
                println!("image_info_flags: {flags:#x}");
            }
            println!(
                "summary: class_names={} selectors={} methods={} pointer_refs={} classes={} protocols={} categories={}",
                objc.class_names.len(),
                objc.selector_names.len(),
                objc.method_names.len(),
                objc.pointer_refs.len(),
                objc.classes.len(),
                objc.protocols.len(),
                objc.categories.len(),
            );
            if let Some(filters) = objc_active_filter_text(view) {
                println!("active_filters: {filters}");
            }
            if matches!(view.detail, ObjcDetail::All) {
                println!("classes:");
                for class_name in &objc.class_names {
                    println!("  {class_name}");
                }
                println!("selectors:");
                for selector in &objc.selector_names {
                    println!("  {selector}");
                }
                println!("methods:");
                for method in &objc.method_names {
                    println!("  {method}");
                }
            }
            if !filtered.classes.is_empty() && objc_detail_includes_classes(view.detail) {
                println!(
                    "class_records ({}/{}):",
                    filtered.classes.len(),
                    objc.classes.len()
                );
                for record in &filtered.classes {
                    println!(
                        "  {:#x} name={} source={:?} superclass={} metaclass={} ro={} methods={} class_methods={} properties={} ivars={} protocols={}",
                        record.class_pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.name_source,
                        record
                            .superclass_name
                            .clone()
                            .or_else(|| record
                                .superclass_pointer
                                .map(|value| format!("{value:#x}")))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .metaclass_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .ro_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record.methods.len(),
                        record.class_methods.len(),
                        record.properties.len(),
                        record.ivars.len(),
                        record.adopted_protocols.len(),
                    );
                }
            }
            if !filtered.protocols.is_empty() && objc_detail_includes_protocols(view.detail) {
                println!(
                    "protocol_records ({}/{}):",
                    filtered.protocols.len(),
                    objc.protocols.len()
                );
                for record in &filtered.protocols {
                    println!(
                        "  {:#x} {} source={:?} required_inst={} required_class={} optional_inst={} optional_class={} properties={}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.name_source,
                        record.required_instance_methods.len(),
                        record.required_class_methods.len(),
                        record.optional_instance_methods.len(),
                        record.optional_class_methods.len(),
                        record.properties.len(),
                    );
                }
            }
            if !filtered.categories.is_empty() && objc_detail_includes_categories(view.detail) {
                println!(
                    "category_records ({}/{}):",
                    filtered.categories.len(),
                    objc.categories.len()
                );
                for record in &filtered.categories {
                    println!(
                        "  {:#x} name={} name_source={:?} record_source={:?} class={} class_source={:?} class_ptr={} prop_list={} proto_list={} props_source={:?} protos_source={:?} methods={} class_methods={} properties={} protocols={}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.name_source,
                        record.record_source,
                        record.class_name.as_deref().unwrap_or("-"),
                        record.class_name_source,
                        record
                            .class_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .property_list_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .protocol_list_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record.properties_source,
                        record.protocols_source,
                        record.methods.len(),
                        record.class_methods.len(),
                        record.properties.len(),
                        record.adopted_protocols.len(),
                    );
                }
            }
            if !filtered.methods.is_empty() && objc_detail_includes_methods(view.detail) {
                println!(
                    "method_records ({}/{}):",
                    filtered.methods.len(),
                    objc_total_methods(objc)
                );
                for entry in &filtered.methods {
                    println!(
                        "  owner={} kind={} class_method={} selector={} selector_source={:?} impl={} types={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        objc_method_owner_kind_text(entry.record.owner_kind),
                        entry.record.is_class_method,
                        entry.record.selector.as_deref().unwrap_or("-"),
                        entry.record.selector_source,
                        entry
                            .record
                            .implementation
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        entry.record.type_encoding.as_deref().unwrap_or("-"),
                    );
                }
            }
            if !filtered.properties.is_empty() && objc_detail_includes_properties(view.detail) {
                println!(
                    "property_records ({}/{}):",
                    filtered.properties.len(),
                    objc_total_properties(objc)
                );
                for entry in &filtered.properties {
                    println!(
                        "  owner={} name={} source={:?} attrs={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        entry.record.name.as_deref().unwrap_or("-"),
                        entry.record.name_source,
                        entry.record.attributes.as_deref().unwrap_or("-"),
                    );
                }
            }
            if !filtered.ivars.is_empty() && objc_detail_includes_ivars(view.detail) {
                println!(
                    "ivar_records ({}/{}):",
                    filtered.ivars.len(),
                    objc_total_ivars(objc)
                );
                for entry in &filtered.ivars {
                    println!(
                        "  owner={} name={} source={:?} type={} offset={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        entry.record.name.as_deref().unwrap_or("-"),
                        entry.record.name_source,
                        entry.record.type_encoding.as_deref().unwrap_or("-"),
                        entry
                            .record
                            .offset
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                    );
                }
            }
            if !filtered.pointer_refs.is_empty() && matches!(view.detail, ObjcDetail::All) {
                println!(
                    "pointer_refs ({}/{}):",
                    filtered.pointer_refs.len(),
                    objc.pointer_refs.len()
                );
                for entry in &filtered.pointer_refs {
                    println!(
                        "  {:<10} table={:#x} raw={:#x} resolved_addr={} resolved_name={}",
                        objc_pointer_kind_text(entry.kind),
                        entry.table_address,
                        entry.raw_pointer,
                        entry
                            .resolved_address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        entry.resolved_name.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        OutputFormat::Json => emit_json_response(
            "objc",
            &ObjcJsonDto {
                image,
                view: view.clone(),
            },
            output,
        ),
    }
}

pub(crate) fn print_dyld(image: &BinaryImage, view: &DyldViewOptions, output: &OutputSettings) {
    let dyld = image.dyld();
    let filtered = filter_dyld_view(dyld, view);
    match output.format {
        OutputFormat::Text => {
            println!(
                "dyld: dylibs={} rpaths={} exports={} function_starts={} bindings={} stubs={} helpers={} rebases={} binds={} chained_fixups={}",
                dyld.imported_dylibs.len(),
                dyld.rpaths.len(),
                dyld.exported_symbols.len(),
                dyld.function_starts.len(),
                filtered.import_bindings.len(),
                filtered.stubs.len(),
                filtered.stub_helpers.len(),
                dyld.has_rebases,
                dyld.has_binds,
                dyld.has_chained_fixups
            );
            if let Some(filters) = dyld_active_filter_text(view) {
                println!("active_filters: {filters}");
            }
            if view.show_dylibs {
                println!("dylibs:");
                for dylib in &dyld.imported_dylibs {
                    println!("  {dylib}");
                }
            }
            if view.show_rpaths {
                println!("rpaths:");
                for rpath in &dyld.rpaths {
                    println!("  {rpath}");
                }
            }
            if view.show_exports {
                println!(
                    "exports ({}/{}):",
                    filtered.exports.len(),
                    dyld.exported_symbols.len()
                );
                for export in &filtered.exports {
                    println!(
                        "  {:>#18} raw={} flags={} kind={:?} reexport={} resolver={} {}",
                        export
                            .address
                            .map(|address| format!("{address:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        export.raw_flags,
                        export.flags,
                        export.kind,
                        export
                            .reexport_target
                            .as_ref()
                            .map(|(dylib, symbol)| format!(
                                "{}:{}",
                                dylib,
                                symbol.as_deref().unwrap_or("-")
                            ))
                            .unwrap_or_else(|| "-".to_string()),
                        export
                            .resolver_target
                            .map(|address| format!("{address:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        export.name
                    );
                }
            }
            if view.show_function_starts {
                println!("function_starts:");
                for start in &dyld.function_starts {
                    println!("  {start:#x}");
                }
            }
            if view.show_bindings {
                println!(
                    "import_bindings ({}/{}):",
                    filtered.import_bindings.len(),
                    dyld.import_bindings.len()
                );
                for binding in &filtered.import_bindings {
                    println!(
                        "  {:>#18} {:>#18} {:<30} {:<24} addend={} kind={:?} source={:?} ordinal={} symidx={} weak={}",
                        binding
                            .address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        binding
                            .offset
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        binding.dylib,
                        binding.name,
                        binding.addend,
                        binding.binding_kind,
                        binding.source,
                        binding
                            .ordinal
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        binding
                            .symbol_index
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        binding.is_weak
                    );
                }
            }
            if view.show_stubs {
                println!("stubs ({}/{}):", filtered.stubs.len(), dyld.stubs.len());
                for stub in &filtered.stubs {
                    println!(
                        "  {:#18x} section={} ptr_section={} ptr={} helper={} ordinal={} kind={:?} {}:{} source={:?}",
                        stub.stub_address,
                        stub.section.as_deref().unwrap_or("-"),
                        stub.pointer_section.as_deref().unwrap_or("-"),
                        stub.pointer_address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        stub.helper_address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        stub.binding_ordinal
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        stub.stub_kind,
                        stub.dylib.as_deref().unwrap_or("-"),
                        stub.name.as_deref().unwrap_or("-"),
                        stub.source
                    );
                }
            }
            if view.show_helpers {
                println!(
                    "stub_helpers ({}/{}):",
                    filtered.stub_helpers.len(),
                    dyld.stub_helpers.len()
                );
                for helper in &filtered.stub_helpers {
                    println!(
                        "  {:#18x} stub={} stub_section={} ptr_section={} ptr={} ordinal={} {}:{}",
                        helper.helper_address,
                        helper
                            .target_stub
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        helper.stub_section.as_deref().unwrap_or("-"),
                        helper.pointer_section.as_deref().unwrap_or("-"),
                        helper
                            .pointer_address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        helper
                            .binding_ordinal
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        helper.dylib.as_deref().unwrap_or("-"),
                        helper.name.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        OutputFormat::Json => emit_json_response(
            "dyld",
            &DyldJsonDto {
                image,
                view: view.clone(),
            },
            output,
        ),
    }
}

pub(crate) fn print_slices(image: &BinaryImage, output: &OutputSettings) {
    match output.format {
        OutputFormat::Text => {
            for descriptor in image.available_slices() {
                let marker = if descriptor.selected { "*" } else { "-" };
                println!(
                    "{marker} offset={:#x} size={:#x} universal={} subtype={:#x} arch={}",
                    descriptor.offset,
                    descriptor.size,
                    descriptor.is_universal,
                    descriptor.cpu_subtype,
                    descriptor.architecture
                );
            }
        }
        OutputFormat::Json => emit_json_response("slices", &SlicesJsonDto { image }, output),
    }
}

pub(crate) fn print_disassembly(
    view: DisassemblyView<'_>,
    render_options: DisassemblyRenderOptions,
    output: &OutputSettings,
) {
    match output.format {
        OutputFormat::Text => {
            if let Some(window_end) = view.window_end {
                println!(
                    "target: {} start={:#x} end={:#x} bytes={:#x} decoded={:#x} instructions={} stop={}",
                    view.target,
                    view.start_address,
                    window_end,
                    view.bytes_len,
                    view.decoded_bytes,
                    view.instruction_count,
                    view.stop_reason
                );
            } else {
                println!(
                    "target: {} start={:#x} end={:#x} bytes={:#x} decoded={:#x} instructions={} stop={}",
                    view.target,
                    view.start_address,
                    view.end_address,
                    view.bytes_len,
                    view.decoded_bytes,
                    view.instruction_count,
                    view.stop_reason
                );
            }
            for instruction in view.instructions {
                let rendered = instruction.render();
                let mut comments = Vec::new();
                if render_options.include_annotations {
                    comments.extend(instruction.annotations.iter().map(ToString::to_string));
                }
                if render_options.include_references {
                    comments.extend(
                        instruction
                            .references
                            .iter()
                            .map(reference_to_text)
                            .map(|value| format!("ref {value}")),
                    );
                }
                if render_options.include_values {
                    comments.extend(
                        instruction
                            .recovered_values
                            .iter()
                            .map(recovered_value_to_text)
                            .map(|value| format!("value {value}")),
                    );
                }
                if comments.is_empty() {
                    println!(
                        "{:>#18x}  {:08x}  {}",
                        instruction.address, instruction.opcode, rendered
                    );
                } else {
                    println!(
                        "{:>#18x}  {:08x}  {} ; {}",
                        instruction.address,
                        instruction.opcode,
                        rendered,
                        comments.join(" | ")
                    );
                }
            }
        }
        OutputFormat::Json => emit_json_response(
            "disasm",
            &DisasmJsonDto {
                view,
                render_options,
            },
            output,
        ),
    }
}

fn print_info_text(image: &BinaryImage) {
    let objc = image.objc();
    let dyld = image.dyld();
    println!("path: {}", image.path().display());
    println!("format: {:?}", image.format());
    println!("architecture: {}", image.architecture());
    println!("endianness: {:?}", image.endianness());
    if let Some(entry) = image.entry_point() {
        println!("entry: {entry:#x}");
    }
    if let Some(platform) = image.platform() {
        println!("platform: {platform}");
    }
    let selected_slice = image.selected_slice();
    println!(
        "slice: offset={:#x} size={:#x} universal={} subtype={:#x}",
        selected_slice.offset,
        selected_slice.size,
        selected_slice.is_universal,
        selected_slice.cpu_subtype
    );
    println!("available_slices: {}", image.available_slices().len());
    println!(
        "counts: segments={} sections={} symbols={} imports={} relocations={}",
        image.segments().len(),
        image.sections().len(),
        image.symbols().len(),
        image.imports().len(),
        image.relocations().len()
    );
    println!(
        "objc: class_names={} selectors={} methods={} pointer_refs={} classes={} protocols={} categories={}",
        objc.class_names.len(),
        objc.selector_names.len(),
        objc.method_names.len(),
        objc.pointer_refs.len(),
        objc.classes.len(),
        objc.protocols.len(),
        objc.categories.len()
    );
    println!(
        "dyld: dylibs={} rpaths={} exports={} function_starts={} bindings={} stubs={} helpers={} rebases={} binds={} chained_fixups={}",
        dyld.imported_dylibs.len(),
        dyld.rpaths.len(),
        dyld.exported_symbols.len(),
        dyld.function_starts.len(),
        dyld.import_bindings.len(),
        dyld.stubs.len(),
        dyld.stub_helpers.len(),
        dyld.has_rebases,
        dyld.has_binds,
        dyld.has_chained_fixups
    );
    if !dyld.imported_dylibs.is_empty() {
        println!("dylibs:");
        for dylib in &dyld.imported_dylibs {
            println!("  {dylib}");
        }
    }
}

fn cache_lookup_kind_text(kind: CacheLookupKind) -> &'static str {
    match kind {
        CacheLookupKind::ExactSymbol => "exact_symbol",
        CacheLookupKind::NearestSymbol => "nearest_symbol",
        CacheLookupKind::MappingOnly => "mapping_only",
    }
}

#[derive(Debug, Clone)]
struct FilteredDyldView<'a> {
    exports: Vec<&'a damsel_core::ExportRecord>,
    import_bindings: Vec<&'a ImportBindingRecord>,
    stubs: Vec<&'a StubEntry>,
    stub_helpers: Vec<&'a StubHelperEntry>,
}

#[derive(Debug, Clone)]
struct ObjcOwnedRecordView<'a, T> {
    owner_name: Option<String>,
    record: &'a T,
}

#[derive(Debug, Clone)]
struct FilteredObjcView<'a> {
    pointer_refs: Vec<&'a ObjcPointerRef>,
    classes: Vec<&'a ObjcClassRecord>,
    protocols: Vec<&'a ObjcProtocolRecord>,
    categories: Vec<&'a ObjcCategoryRecord>,
    methods: Vec<ObjcOwnedRecordView<'a, ObjcMethodRecord>>,
    properties: Vec<ObjcOwnedRecordView<'a, ObjcPropertyRecord>>,
    ivars: Vec<ObjcOwnedRecordView<'a, ObjcIvarRecord>>,
}

fn filter_dyld_view<'a>(
    dyld: &'a damsel_core::DyldMetadata,
    view: &DyldViewOptions,
) -> FilteredDyldView<'a> {
    let mut exports = dyld
        .exported_symbols
        .iter()
        .filter(|export| {
            view.name_filter
                .as_deref()
                .is_none_or(|needle| contains_case_insensitive(&export.name, needle))
                && view
                    .export_kind_filter
                    .is_none_or(|kind| export_matches_kind(export, kind))
                && view
                    .export_flag_filter
                    .is_none_or(|flag| export_matches_flag(export, flag))
        })
        .collect::<Vec<_>>();
    sort_exports(&mut exports, view.sort);

    let mut import_bindings = dyld
        .import_bindings
        .iter()
        .filter(|binding| {
            view.name_filter
                .as_deref()
                .is_none_or(|needle| contains_case_insensitive(&binding.name, needle))
                && view
                    .dylib_filter
                    .as_deref()
                    .is_none_or(|needle| contains_case_insensitive(&binding.dylib, needle))
                && view
                    .source_filter
                    .is_none_or(|source| binding.source == source)
                && view
                    .binding_kind_filter
                    .is_none_or(|kind| binding.binding_kind == kind)
                && view
                    .stub_kind_filter
                    .as_ref()
                    .is_none_or(|kind| binding_matches_stub_kind(binding.binding_kind, kind))
                && view
                    .ordinal_filter
                    .is_none_or(|ordinal| binding.ordinal == Some(ordinal))
        })
        .collect::<Vec<_>>();
    sort_bindings(&mut import_bindings, view.sort);

    let mut stubs = dyld
        .stubs
        .iter()
        .filter(|stub| {
            view.name_filter.as_deref().is_none_or(|needle| {
                stub.name
                    .as_deref()
                    .is_some_and(|value| contains_case_insensitive(value, needle))
            }) && view.dylib_filter.as_deref().is_none_or(|needle| {
                stub.dylib
                    .as_deref()
                    .is_some_and(|value| contains_case_insensitive(value, needle))
            }) && view
                .source_filter
                .is_none_or(|source| stub.source == source)
                && view
                    .binding_kind_filter
                    .is_none_or(|kind| stub_matches_binding_kind(&stub.stub_kind, kind))
                && view
                    .stub_kind_filter
                    .as_ref()
                    .is_none_or(|kind| &stub.stub_kind == kind)
                && view
                    .ordinal_filter
                    .is_none_or(|ordinal| stub.binding_ordinal == Some(ordinal))
        })
        .collect::<Vec<_>>();
    sort_stubs(&mut stubs, view.sort);

    let allowed_helper_addresses = stubs
        .iter()
        .filter_map(|stub| stub.helper_address)
        .collect::<std::collections::BTreeSet<_>>();
    let restrict_helpers_to_stubs = view.name_filter.is_some()
        || view.dylib_filter.is_some()
        || view.source_filter.is_some()
        || view.binding_kind_filter.is_some()
        || view.stub_kind_filter.is_some()
        || view.ordinal_filter.is_some();
    let mut stub_helpers = dyld
        .stub_helpers
        .iter()
        .filter(|helper| {
            view.name_filter.as_deref().is_none_or(|needle| {
                helper
                    .name
                    .as_deref()
                    .is_some_and(|value| contains_case_insensitive(value, needle))
            }) && view.dylib_filter.as_deref().is_none_or(|needle| {
                helper
                    .dylib
                    .as_deref()
                    .is_some_and(|value| contains_case_insensitive(value, needle))
            }) && view
                .ordinal_filter
                .is_none_or(|ordinal| helper.binding_ordinal == Some(ordinal))
                && (!restrict_helpers_to_stubs
                    || allowed_helper_addresses.contains(&helper.helper_address))
        })
        .collect::<Vec<_>>();
    sort_stub_helpers(&mut stub_helpers, view.sort);

    FilteredDyldView {
        exports,
        import_bindings,
        stubs,
        stub_helpers,
    }
}

fn filter_objc_view<'a>(
    objc: &'a damsel_core::ObjcMetadata,
    view: &ObjcViewOptions,
) -> FilteredObjcView<'a> {
    let classes = objc
        .classes
        .iter()
        .filter(|record| {
            objc_class_matches(
                record,
                view.owner_filter.as_deref(),
                view.name_source_filter,
            )
        })
        .collect::<Vec<_>>();
    let protocols = objc
        .protocols
        .iter()
        .filter(|record| {
            objc_protocol_matches(
                record,
                view.owner_filter.as_deref(),
                view.name_source_filter,
            )
        })
        .collect::<Vec<_>>();
    let categories = objc
        .categories
        .iter()
        .filter(|record| {
            objc_category_matches(
                record,
                view.owner_filter.as_deref(),
                view.name_source_filter,
                view.category_source_filter,
            )
        })
        .collect::<Vec<_>>();

    let mut methods = Vec::new();
    let mut properties = Vec::new();
    let mut ivars = Vec::new();

    for class_record in &classes {
        let owner_name = class_record.name.clone();
        methods.extend(
            class_record
                .methods
                .iter()
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
        methods.extend(
            class_record
                .class_methods
                .iter()
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
        properties.extend(
            class_record
                .properties
                .iter()
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
        ivars.extend(class_record.ivars.iter().map(|record| ObjcOwnedRecordView {
            owner_name: owner_name.clone(),
            record,
        }));
    }

    for protocol_record in &protocols {
        let owner_name = protocol_record.name.clone();
        methods.extend(
            protocol_record
                .required_instance_methods
                .iter()
                .chain(protocol_record.required_class_methods.iter())
                .chain(protocol_record.optional_instance_methods.iter())
                .chain(protocol_record.optional_class_methods.iter())
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
        properties.extend(
            protocol_record
                .properties
                .iter()
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
    }

    for category_record in &categories {
        let owner_name = category_record
            .name
            .clone()
            .or_else(|| category_record.class_name.clone());
        methods.extend(
            category_record
                .methods
                .iter()
                .chain(category_record.class_methods.iter())
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
        properties.extend(
            category_record
                .properties
                .iter()
                .map(|record| ObjcOwnedRecordView {
                    owner_name: owner_name.clone(),
                    record,
                }),
        );
    }

    if let Some(selector_source) = view.selector_source_filter {
        methods.retain(|entry| entry.record.selector_source == selector_source);
    }
    if let Some(name_source) = view.name_source_filter {
        properties.retain(|entry| entry.record.name_source == name_source);
        ivars.retain(|entry| entry.record.name_source == name_source);
    }

    let pointer_refs = if view.owner_filter.is_some() {
        let matched_names = classes
            .iter()
            .filter_map(|record| record.name.as_deref())
            .chain(
                categories
                    .iter()
                    .filter_map(|record| record.class_name.as_deref()),
            )
            .chain(protocols.iter().filter_map(|record| record.name.as_deref()))
            .collect::<std::collections::BTreeSet<_>>();
        objc.pointer_refs
            .iter()
            .filter(|entry| {
                entry
                    .resolved_name
                    .as_deref()
                    .is_some_and(|name| matched_names.iter().any(|candidate| candidate == &name))
            })
            .collect::<Vec<_>>()
    } else {
        objc.pointer_refs.iter().collect::<Vec<_>>()
    };

    FilteredObjcView {
        pointer_refs,
        classes,
        protocols,
        categories,
        methods,
        properties,
        ivars,
    }
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn binding_source_rank(source: ImportBindingSource) -> u8 {
    match source {
        ImportBindingSource::ChainedFixup => 0,
        ImportBindingSource::IndirectSymbol => 1,
        ImportBindingSource::Stub => 2,
        ImportBindingSource::Other => 3,
    }
}

fn sort_exports(exports: &mut Vec<&damsel_core::ExportRecord>, sort: Option<DyldSortKey>) {
    match sort.unwrap_or(DyldSortKey::Address) {
        DyldSortKey::Address => {
            exports.sort_by_key(|export| (export.address.unwrap_or_default(), export.name.clone()))
        }
        DyldSortKey::Name => exports.sort_by_key(|export| export.name.clone()),
        DyldSortKey::Dylib | DyldSortKey::Source => {
            exports.sort_by_key(|export| (export.raw_flags.clone(), export.name.clone()))
        }
    }
}

fn sort_bindings(bindings: &mut Vec<&ImportBindingRecord>, sort: Option<DyldSortKey>) {
    match sort.unwrap_or(DyldSortKey::Address) {
        DyldSortKey::Address => bindings.sort_by_key(|binding| {
            (
                binding.address.unwrap_or_default(),
                binding.offset.unwrap_or_default(),
                binding.dylib.clone(),
                binding.name.clone(),
            )
        }),
        DyldSortKey::Name => bindings.sort_by_key(|binding| {
            (
                binding.name.clone(),
                binding.dylib.clone(),
                binding.address.unwrap_or_default(),
            )
        }),
        DyldSortKey::Dylib => bindings.sort_by_key(|binding| {
            (
                binding.dylib.clone(),
                binding.name.clone(),
                binding.address.unwrap_or_default(),
            )
        }),
        DyldSortKey::Source => bindings.sort_by_key(|binding| {
            (
                binding_source_rank(binding.source),
                binding.dylib.clone(),
                binding.name.clone(),
                binding.address.unwrap_or_default(),
            )
        }),
    }
}

fn sort_stubs(stubs: &mut Vec<&StubEntry>, sort: Option<DyldSortKey>) {
    match sort.unwrap_or(DyldSortKey::Address) {
        DyldSortKey::Address => stubs.sort_by_key(|stub| stub.stub_address),
        DyldSortKey::Name => stubs.sort_by_key(|stub| {
            (
                stub.name.clone().unwrap_or_default(),
                stub.dylib.clone().unwrap_or_default(),
                stub.stub_address,
            )
        }),
        DyldSortKey::Dylib => stubs.sort_by_key(|stub| {
            (
                stub.dylib.clone().unwrap_or_default(),
                stub.name.clone().unwrap_or_default(),
                stub.stub_address,
            )
        }),
        DyldSortKey::Source => stubs.sort_by_key(|stub| {
            (
                binding_source_rank(stub.source),
                stub.dylib.clone().unwrap_or_default(),
                stub.name.clone().unwrap_or_default(),
                stub.stub_address,
            )
        }),
    }
}

fn sort_stub_helpers(helpers: &mut Vec<&StubHelperEntry>, sort: Option<DyldSortKey>) {
    match sort.unwrap_or(DyldSortKey::Address) {
        DyldSortKey::Address => helpers.sort_by_key(|helper| helper.helper_address),
        DyldSortKey::Name => helpers.sort_by_key(|helper| {
            (
                helper.name.clone().unwrap_or_default(),
                helper.dylib.clone().unwrap_or_default(),
                helper.helper_address,
            )
        }),
        DyldSortKey::Dylib => helpers.sort_by_key(|helper| {
            (
                helper.dylib.clone().unwrap_or_default(),
                helper.name.clone().unwrap_or_default(),
                helper.helper_address,
            )
        }),
        DyldSortKey::Source => helpers.sort_by_key(|helper| {
            (
                helper.binding_ordinal.unwrap_or_default(),
                helper.dylib.clone().unwrap_or_default(),
                helper.name.clone().unwrap_or_default(),
                helper.helper_address,
            )
        }),
    }
}

fn binding_matches_stub_kind(binding_kind: ImportBindingKind, stub_kind: &StubKind) -> bool {
    matches!(
        (binding_kind, stub_kind),
        (ImportBindingKind::Lazy, StubKind::Lazy) | (ImportBindingKind::NonLazy, StubKind::NonLazy)
    )
}

fn export_matches_kind(export: &damsel_core::ExportRecord, kind: ExportKindFilter) -> bool {
    matches!(
        (&export.kind, kind),
        (ExportKind::Regular, ExportKindFilter::Regular)
            | (ExportKind::Reexport { .. }, ExportKindFilter::Reexport)
            | (ExportKind::Resolver { .. }, ExportKindFilter::Resolver)
            | (
                ExportKind::StubAndResolver { .. },
                ExportKindFilter::StubAndResolver
            )
            | (ExportKind::WeakDefinition, ExportKindFilter::WeakDefinition)
            | (ExportKind::Absolute, ExportKindFilter::Absolute)
            | (ExportKind::ThreadLocal, ExportKindFilter::ThreadLocal)
            | (ExportKind::Unknown(_), ExportKindFilter::Unknown)
    )
}

fn export_matches_flag(export: &damsel_core::ExportRecord, flag: ExportFlagName) -> bool {
    match flag {
        ExportFlagName::WeakDefinition => export.flags.is_weak_definition,
        ExportFlagName::Reexport => export.flags.is_reexport,
        ExportFlagName::StubAndResolver => export.flags.is_stub_and_resolver,
        ExportFlagName::ThreadLocal => export.flags.is_thread_local,
        ExportFlagName::Absolute => export.flags.is_absolute,
    }
}

fn stub_matches_binding_kind(stub_kind: &StubKind, binding_kind: ImportBindingKind) -> bool {
    match binding_kind {
        ImportBindingKind::Lazy => matches!(stub_kind, StubKind::Lazy),
        ImportBindingKind::NonLazy => matches!(stub_kind, StubKind::NonLazy),
        ImportBindingKind::ChainedFixup => false,
    }
}

fn dyld_active_filter_text(view: &DyldViewOptions) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(name) = &view.name_filter {
        parts.push(format!("name={name}"));
    }
    if let Some(dylib) = &view.dylib_filter {
        parts.push(format!("dylib={dylib}"));
    }
    if let Some(source) = view.source_filter {
        parts.push(format!("source={source:?}"));
    }
    if let Some(binding_kind) = view.binding_kind_filter {
        parts.push(format!("binding_kind={binding_kind:?}"));
    }
    if let Some(stub_kind) = &view.stub_kind_filter {
        parts.push(format!("stub_kind={stub_kind:?}"));
    }
    if let Some(export_kind) = view.export_kind_filter {
        parts.push(format!("export_kind={export_kind:?}"));
    }
    if let Some(export_flag) = view.export_flag_filter {
        parts.push(format!("export_flag={export_flag:?}"));
    }
    if let Some(ordinal) = view.ordinal_filter {
        parts.push(format!("ordinal={ordinal}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn objc_active_filter_text(view: &ObjcViewOptions) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(owner) = &view.owner_filter {
        parts.push(format!("owner={owner}"));
    }
    if let Some(source) = view.name_source_filter {
        parts.push(format!("name_source={source:?}"));
    }
    if let Some(source) = view.selector_source_filter {
        parts.push(format!("selector_source={source:?}"));
    }
    if let Some(source) = view.category_source_filter {
        parts.push(format!("category_source={source:?}"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn objc_total_methods(objc: &damsel_core::ObjcMetadata) -> usize {
    let class_methods = objc
        .classes
        .iter()
        .map(|record| record.methods.len() + record.class_methods.len())
        .sum::<usize>();
    let protocol_methods = objc
        .protocols
        .iter()
        .map(|record| {
            record.required_instance_methods.len()
                + record.required_class_methods.len()
                + record.optional_instance_methods.len()
                + record.optional_class_methods.len()
        })
        .sum::<usize>();
    let category_methods = objc
        .categories
        .iter()
        .map(|record| record.methods.len() + record.class_methods.len())
        .sum::<usize>();
    class_methods + protocol_methods + category_methods
}

fn objc_total_properties(objc: &damsel_core::ObjcMetadata) -> usize {
    let class_properties = objc
        .classes
        .iter()
        .map(|record| record.properties.len())
        .sum::<usize>();
    let protocol_properties = objc
        .protocols
        .iter()
        .map(|record| record.properties.len())
        .sum::<usize>();
    let category_properties = objc
        .categories
        .iter()
        .map(|record| record.properties.len())
        .sum::<usize>();
    class_properties + protocol_properties + category_properties
}

fn objc_total_ivars(objc: &damsel_core::ObjcMetadata) -> usize {
    objc.classes.iter().map(|record| record.ivars.len()).sum()
}

fn objc_class_matches(
    record: &ObjcClassRecord,
    owner_filter: Option<&str>,
    name_source_filter: Option<ObjcNameSource>,
) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
            || record
                .superclass_name
                .as_deref()
                .is_some_and(|value| contains_case_insensitive(value, needle))
    }) && name_source_filter.is_none_or(|source| record.name_source == source)
}

fn objc_protocol_matches(
    record: &ObjcProtocolRecord,
    owner_filter: Option<&str>,
    name_source_filter: Option<ObjcNameSource>,
) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
    }) && name_source_filter.is_none_or(|source| record.name_source == source)
}

fn objc_category_matches(
    record: &ObjcCategoryRecord,
    owner_filter: Option<&str>,
    name_source_filter: Option<ObjcNameSource>,
    category_source_filter: Option<ObjcCategorySourceFilter>,
) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
            || record
                .class_name
                .as_deref()
                .is_some_and(|value| contains_case_insensitive(value, needle))
    }) && name_source_filter
        .is_none_or(|source| record.name_source == source || record.class_name_source == source)
        && category_source_filter.is_none_or(|source| match source {
            ObjcCategorySourceFilter::RuntimeList => {
                record.record_source == ObjcCategoryRecordSource::RuntimeList
            }
            ObjcCategorySourceFilter::SymbolSynthesis => {
                record.record_source == ObjcCategoryRecordSource::SymbolSynthesis
            }
            ObjcCategorySourceFilter::SymbolSynthesisWithLists => {
                record.record_source == ObjcCategoryRecordSource::SymbolSynthesis
                    && record.has_list_backing()
            }
        })
}

fn objc_method_owner_kind_text(kind: ObjcMethodOwnerKind) -> &'static str {
    match kind {
        ObjcMethodOwnerKind::Class => "class",
        ObjcMethodOwnerKind::Metaclass => "metaclass",
        ObjcMethodOwnerKind::Protocol => "protocol",
        ObjcMethodOwnerKind::Category => "category",
    }
}

fn objc_detail_includes_classes(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Classes | ObjcDetail::All)
}

fn objc_detail_includes_protocols(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Protocols | ObjcDetail::All)
}

fn objc_detail_includes_categories(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Categories | ObjcDetail::All)
}

fn objc_detail_includes_methods(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Methods | ObjcDetail::All)
}

fn objc_detail_includes_properties(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Properties | ObjcDetail::All)
}

fn objc_detail_includes_ivars(detail: ObjcDetail) -> bool {
    matches!(detail, ObjcDetail::Ivars | ObjcDetail::All)
}

trait JsonDto {
    fn to_json_value(&self) -> JsonValue;
}

struct JsonEnvelope<'a, T> {
    command: &'a str,
    data: &'a T,
}

impl<T: JsonDto> JsonDto for JsonEnvelope<'_, T> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("schema_version".to_string(), u64_num(1)),
            (
                "command".to_string(),
                JsonValue::String(self.command.to_string()),
            ),
            ("data".to_string(), self.data.to_json_value()),
        ])
    }
}

struct InfoJsonDto<'a> {
    image: &'a BinaryImage,
}

impl JsonDto for InfoJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let image = self.image;
        let objc = image.objc();
        let dyld = image.dyld();
        let selected_slice = image.selected_slice();
        JsonValue::Object(vec![
            (
                "path".to_string(),
                JsonValue::String(image.path().display().to_string()),
            ),
            (
                "format".to_string(),
                JsonValue::String(format!("{:?}", image.format())),
            ),
            (
                "architecture".to_string(),
                JsonValue::String(image.architecture().to_string()),
            ),
            (
                "endianness".to_string(),
                JsonValue::String(format!("{:?}", image.endianness())),
            ),
            ("entry_point".to_string(), opt_u64(image.entry_point())),
            (
                "platform".to_string(),
                image
                    .platform()
                    .map(|platform| JsonValue::String(platform.to_string()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "slice".to_string(),
                JsonValue::Object(vec![
                    ("offset".to_string(), u64_num(selected_slice.offset)),
                    ("size".to_string(), u64_num(selected_slice.size)),
                    (
                        "is_universal".to_string(),
                        JsonValue::Bool(selected_slice.is_universal),
                    ),
                    (
                        "cpu_subtype".to_string(),
                        u64_num(u64::from(selected_slice.cpu_subtype)),
                    ),
                ]),
            ),
            (
                "available_slices".to_string(),
                JsonValue::Array(
                    image
                        .available_slices()
                        .iter()
                        .map(slice_descriptor_json)
                        .collect(),
                ),
            ),
            (
                "counts".to_string(),
                JsonValue::Object(vec![
                    ("segments".to_string(), usize_num(image.segments().len())),
                    ("sections".to_string(), usize_num(image.sections().len())),
                    ("symbols".to_string(), usize_num(image.symbols().len())),
                    ("imports".to_string(), usize_num(image.imports().len())),
                    (
                        "relocations".to_string(),
                        usize_num(image.relocations().len()),
                    ),
                ]),
            ),
            (
                "objc".to_string(),
                JsonValue::Object(vec![
                    ("class_names".to_string(), usize_num(objc.class_names.len())),
                    (
                        "selectors".to_string(),
                        usize_num(objc.selector_names.len()),
                    ),
                    ("methods".to_string(), usize_num(objc.method_names.len())),
                    (
                        "pointer_refs".to_string(),
                        usize_num(objc.pointer_refs.len()),
                    ),
                    ("classes".to_string(), usize_num(objc.classes.len())),
                    ("protocols".to_string(), usize_num(objc.protocols.len())),
                    ("categories".to_string(), usize_num(objc.categories.len())),
                    (
                        "image_info_flags".to_string(),
                        objc.image_info_flags
                            .map(u64::from)
                            .map(u64_num)
                            .unwrap_or(JsonValue::Null),
                    ),
                ]),
            ),
            (
                "dyld".to_string(),
                JsonValue::Object(vec![
                    ("dylibs".to_string(), usize_num(dyld.imported_dylibs.len())),
                    ("rpaths".to_string(), usize_num(dyld.rpaths.len())),
                    (
                        "exports".to_string(),
                        usize_num(dyld.exported_symbols.len()),
                    ),
                    (
                        "function_starts".to_string(),
                        usize_num(dyld.function_starts.len()),
                    ),
                    (
                        "import_bindings".to_string(),
                        usize_num(dyld.import_bindings.len()),
                    ),
                    ("stubs".to_string(), usize_num(dyld.stubs.len())),
                    (
                        "stub_helpers".to_string(),
                        usize_num(dyld.stub_helpers.len()),
                    ),
                    ("has_rebases".to_string(), JsonValue::Bool(dyld.has_rebases)),
                    ("has_binds".to_string(), JsonValue::Bool(dyld.has_binds)),
                    (
                        "has_chained_fixups".to_string(),
                        JsonValue::Bool(dyld.has_chained_fixups),
                    ),
                ]),
            ),
        ])
    }
}

struct SectionsJsonDto<'a> {
    sections: &'a [Section],
}

impl JsonDto for SectionsJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Array(
            self.sections
                .iter()
                .map(|section| {
                    JsonValue::Object(vec![
                        (
                            "segment".to_string(),
                            JsonValue::String(section.segment_name.clone()),
                        ),
                        ("name".to_string(), JsonValue::String(section.name.clone())),
                        (
                            "full_name".to_string(),
                            JsonValue::String(section.full_name()),
                        ),
                        ("address".to_string(), u64_num(section.address)),
                        ("size".to_string(), u64_num(section.size)),
                        (
                            "file_offset".to_string(),
                            section.file_offset.map(u64_num).unwrap_or(JsonValue::Null),
                        ),
                        ("file_size".to_string(), u64_num(section.file_size)),
                        (
                            "kind".to_string(),
                            JsonValue::String(section.kind.to_string()),
                        ),
                        (
                            "executable".to_string(),
                            JsonValue::Bool(section.executable),
                        ),
                    ])
                })
                .collect(),
        )
    }
}

struct SymbolsJsonDto<'a> {
    symbols: &'a [Symbol],
}

impl JsonDto for SymbolsJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Array(
            self.symbols
                .iter()
                .map(|symbol| {
                    JsonValue::Object(vec![
                        ("name".to_string(), JsonValue::String(symbol.name.clone())),
                        ("address".to_string(), u64_num(symbol.address)),
                        ("size".to_string(), u64_num(symbol.size)),
                        (
                            "kind".to_string(),
                            JsonValue::String(symbol.kind.to_string()),
                        ),
                        ("defined".to_string(), JsonValue::Bool(symbol.defined)),
                        ("global".to_string(), JsonValue::Bool(symbol.global)),
                        ("weak".to_string(), JsonValue::Bool(symbol.weak)),
                        (
                            "section".to_string(),
                            symbol
                                .section
                                .as_ref()
                                .map(|value| JsonValue::String(value.clone()))
                                .unwrap_or(JsonValue::Null),
                        ),
                    ])
                })
                .collect(),
        )
    }
}

struct ImportsJsonDto<'a> {
    imports: &'a [Import],
}

impl JsonDto for ImportsJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Array(
            self.imports
                .iter()
                .map(|import| {
                    JsonValue::Object(vec![
                        ("name".to_string(), JsonValue::String(import.name.clone())),
                        ("dylib".to_string(), JsonValue::String(import.dylib.clone())),
                        (
                            "address".to_string(),
                            import.address.map(u64_num).unwrap_or(JsonValue::Null),
                        ),
                        (
                            "offset".to_string(),
                            import.offset.map(u64_num).unwrap_or(JsonValue::Null),
                        ),
                        ("addend".to_string(), i64_num(import.addend)),
                        ("is_lazy".to_string(), JsonValue::Bool(import.is_lazy)),
                        ("is_weak".to_string(), JsonValue::Bool(import.is_weak)),
                    ])
                })
                .collect(),
        )
    }
}

struct RelocationsJsonDto<'a> {
    relocations: &'a [Relocation],
}

impl JsonDto for RelocationsJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Array(
            self.relocations
                .iter()
                .map(|relocation| {
                    JsonValue::Object(vec![
                        (
                            "section".to_string(),
                            JsonValue::String(relocation.section.clone()),
                        ),
                        ("address".to_string(), u64_num(relocation.address)),
                        ("size".to_string(), u64_num(u64::from(relocation.size))),
                        (
                            "kind".to_string(),
                            JsonValue::String(relocation.kind.to_string()),
                        ),
                        (
                            "encoding".to_string(),
                            JsonValue::String(relocation.encoding.to_string()),
                        ),
                        (
                            "target".to_string(),
                            JsonValue::String(relocation.target.to_string()),
                        ),
                        ("addend".to_string(), i64_num(relocation.addend)),
                    ])
                })
                .collect(),
        )
    }
}

struct ObjcJsonDto<'a> {
    image: &'a BinaryImage,
    view: ObjcViewOptions,
}

impl JsonDto for ObjcJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let objc = self.image.objc();
        let filtered = filter_objc_view(objc, &self.view);
        JsonValue::Object(vec![
            (
                "requested_detail".to_string(),
                JsonValue::String(format!("{:?}", self.view.detail).to_ascii_lowercase()),
            ),
            (
                "owner_filter".to_string(),
                self.view
                    .owner_filter
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "name_source_filter".to_string(),
                self.view
                    .name_source_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "selector_source_filter".to_string(),
                self.view
                    .selector_source_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "category_source_filter".to_string(),
                self.view
                    .category_source_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "image_info_flags".to_string(),
                objc.image_info_flags
                    .map(u64::from)
                    .map(u64_num)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "class_names".to_string(),
                JsonValue::Array(
                    objc.class_names
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "selector_names".to_string(),
                JsonValue::Array(
                    objc.selector_names
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "method_names".to_string(),
                JsonValue::Array(
                    objc.method_names
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "pointer_refs".to_string(),
                if matches!(self.view.detail, ObjcDetail::All) {
                    JsonValue::Array(
                        filtered
                            .pointer_refs
                            .iter()
                            .map(|entry| {
                                JsonValue::Object(vec![
                                    (
                                        "kind".to_string(),
                                        JsonValue::String(
                                            objc_pointer_kind_text(entry.kind).to_string(),
                                        ),
                                    ),
                                    ("table_address".to_string(), u64_num(entry.table_address)),
                                    ("raw_pointer".to_string(), u64_num(entry.raw_pointer)),
                                    (
                                        "resolved_address".to_string(),
                                        entry
                                            .resolved_address
                                            .map(u64_num)
                                            .unwrap_or(JsonValue::Null),
                                    ),
                                    (
                                        "resolved_name".to_string(),
                                        entry
                                            .resolved_name
                                            .as_ref()
                                            .map(|value| JsonValue::String(value.clone()))
                                            .unwrap_or(JsonValue::Null),
                                    ),
                                ])
                            })
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "classes".to_string(),
                if objc_detail_includes_classes(self.view.detail) {
                    JsonValue::Array(
                        filtered
                            .classes
                            .iter()
                            .map(|record| objc_class_record_json(record))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "protocols".to_string(),
                if objc_detail_includes_protocols(self.view.detail) {
                    JsonValue::Array(
                        filtered
                            .protocols
                            .iter()
                            .map(|record| objc_protocol_record_json(record))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "categories".to_string(),
                if objc_detail_includes_categories(self.view.detail) {
                    JsonValue::Array(
                        filtered
                            .categories
                            .iter()
                            .map(|record| objc_category_record_json(record))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "methods".to_string(),
                if objc_detail_includes_methods(self.view.detail) {
                    JsonValue::Array(filtered.methods.iter().map(objc_method_view_json).collect())
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "properties".to_string(),
                if objc_detail_includes_properties(self.view.detail) {
                    JsonValue::Array(
                        filtered
                            .properties
                            .iter()
                            .map(objc_property_view_json)
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "ivars".to_string(),
                if objc_detail_includes_ivars(self.view.detail) {
                    JsonValue::Array(filtered.ivars.iter().map(objc_ivar_view_json).collect())
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
        ])
    }
}

struct DyldJsonDto<'a> {
    image: &'a BinaryImage,
    view: DyldViewOptions,
}

impl JsonDto for DyldJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let dyld = self.image.dyld();
        let filtered = filter_dyld_view(dyld, &self.view);
        JsonValue::Object(vec![
            ("has_rebases".to_string(), JsonValue::Bool(dyld.has_rebases)),
            ("has_binds".to_string(), JsonValue::Bool(dyld.has_binds)),
            (
                "has_chained_fixups".to_string(),
                JsonValue::Bool(dyld.has_chained_fixups),
            ),
            (
                "included_sections".to_string(),
                JsonValue::Array(included_dyld_sections(&self.view)),
            ),
            (
                "name_filter".to_string(),
                self.view
                    .name_filter
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "dylib_filter".to_string(),
                self.view
                    .dylib_filter
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "source_filter".to_string(),
                self.view
                    .source_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "binding_kind_filter".to_string(),
                self.view
                    .binding_kind_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "stub_kind_filter".to_string(),
                self.view
                    .stub_kind_filter
                    .as_ref()
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "export_kind_filter".to_string(),
                self.view
                    .export_kind_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "export_flag_filter".to_string(),
                self.view
                    .export_flag_filter
                    .map(|value| JsonValue::String(format!("{value:?}")))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "ordinal_filter".to_string(),
                self.view
                    .ordinal_filter
                    .map(|value| u64_num(u64::from(value)))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "dylibs".to_string(),
                if self.view.show_dylibs {
                    JsonValue::Array(
                        dyld.imported_dylibs
                            .iter()
                            .cloned()
                            .map(JsonValue::String)
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "rpaths".to_string(),
                if self.view.show_rpaths {
                    JsonValue::Array(dyld.rpaths.iter().cloned().map(JsonValue::String).collect())
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "exports".to_string(),
                if self.view.show_exports {
                    JsonValue::Array(
                        filtered
                            .exports
                            .iter()
                            .map(|export| export_json(export))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "function_starts".to_string(),
                if self.view.show_function_starts {
                    JsonValue::Array(dyld.function_starts.iter().copied().map(u64_num).collect())
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "import_bindings".to_string(),
                if self.view.show_bindings {
                    JsonValue::Array(
                        filtered
                            .import_bindings
                            .iter()
                            .map(|binding| import_binding_json(binding))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "stubs".to_string(),
                if self.view.show_stubs {
                    JsonValue::Array(
                        filtered
                            .stubs
                            .iter()
                            .map(|stub| stub_entry_json(stub))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "stub_helpers".to_string(),
                if self.view.show_helpers {
                    JsonValue::Array(
                        filtered
                            .stub_helpers
                            .iter()
                            .map(|helper| stub_helper_json(helper))
                            .collect(),
                    )
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
        ])
    }
}

struct SlicesJsonDto<'a> {
    image: &'a BinaryImage,
}

impl JsonDto for SlicesJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Array(
            self.image
                .available_slices()
                .iter()
                .map(slice_descriptor_json)
                .collect(),
        )
    }
}

struct CacheInfoJsonDto<'a> {
    info: &'a CacheInfoView,
}

impl JsonDto for CacheInfoJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![
            (
                "header".to_string(),
                JsonValue::Object(vec![
                    (
                        "cache_path".to_string(),
                        JsonValue::String(self.info.cache_path.clone()),
                    ),
                    (
                        "cache_uuid".to_string(),
                        JsonValue::String(self.info.cache_uuid.clone()),
                    ),
                    (
                        "architecture".to_string(),
                        JsonValue::String(self.info.architecture.clone()),
                    ),
                    (
                        "member_count".to_string(),
                        usize_num(self.info.member_count),
                    ),
                    ("image_count".to_string(), usize_num(self.info.image_count)),
                    (
                        "has_local_symbols".to_string(),
                        JsonValue::Bool(self.info.has_local_symbols),
                    ),
                ]),
            ),
            (
                "members".to_string(),
                JsonValue::Array(
                    self.info
                        .members
                        .iter()
                        .map(|member| {
                            JsonValue::Object(vec![
                                ("name".to_string(), JsonValue::String(member.name.clone())),
                                ("role".to_string(), JsonValue::String(member.role.clone())),
                                ("path".to_string(), JsonValue::String(member.path.clone())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

struct CacheImagesJsonDto<'a> {
    metadata: &'a CacheCollectionMetadata,
    images: &'a [CacheImageView],
}

impl JsonDto for CacheImagesJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![
            (
                "metadata".to_string(),
                collection_metadata_json(self.metadata),
            ),
            (
                "images".to_string(),
                JsonValue::Array(self.images.iter().map(cache_image_json).collect()),
            ),
        ])
    }
}

struct CacheImageJsonDto<'a> {
    image: &'a CacheImageView,
}

impl JsonDto for CacheImageJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![("image".to_string(), cache_image_json(self.image))])
    }
}

struct CacheExportsJsonDto<'a> {
    image: &'a CacheImageView,
    metadata: &'a CacheCollectionMetadata,
    exports: &'a [CacheExportView],
}

impl JsonDto for CacheExportsJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("image".to_string(), cache_image_json(self.image)),
            (
                "metadata".to_string(),
                collection_metadata_json(self.metadata),
            ),
            (
                "exports".to_string(),
                JsonValue::Array(self.exports.iter().map(cache_export_json).collect()),
            ),
        ])
    }
}

struct CacheLookupAddressJsonDto<'a> {
    result: &'a CacheLookupResultView,
}

impl JsonDto for CacheLookupAddressJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![(
            "result".to_string(),
            JsonValue::Object(vec![
                (
                    "kind".to_string(),
                    JsonValue::String(cache_lookup_kind_text(self.result.kind).to_string()),
                ),
                (
                    "cache_vmaddr".to_string(),
                    u64_num(self.result.cache_vmaddr),
                ),
                (
                    "image_id".to_string(),
                    self.result
                        .image_id
                        .as_ref()
                        .map(|value| JsonValue::String(value.clone()))
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "install_name".to_string(),
                    self.result
                        .install_name
                        .as_ref()
                        .map(|value| JsonValue::String(value.clone()))
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "image_base_vmaddr".to_string(),
                    self.result
                        .image_base_vmaddr
                        .map(u64_num)
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "image_offset".to_string(),
                    self.result
                        .image_offset
                        .map(u64_num)
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "member_file_offset".to_string(),
                    self.result
                        .member_file_offset
                        .map(u64_num)
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "symbol".to_string(),
                    self.result
                        .symbol
                        .as_ref()
                        .map(|value| JsonValue::String(value.clone()))
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "symbol_address".to_string(),
                    self.result
                        .symbol_address
                        .map(u64_num)
                        .unwrap_or(JsonValue::Null),
                ),
                (
                    "offset_from_symbol".to_string(),
                    self.result
                        .offset_from_symbol
                        .map(u64_num)
                        .unwrap_or(JsonValue::Null),
                ),
            ]),
        )])
    }
}

struct CacheResolveSymbolJsonDto<'a> {
    metadata: &'a CacheCollectionMetadata,
    matches: &'a [CacheSymbolicationMatchView],
}

impl JsonDto for CacheResolveSymbolJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(vec![
            (
                "metadata".to_string(),
                collection_metadata_json(self.metadata),
            ),
            (
                "matches".to_string(),
                JsonValue::Array(
                    self.matches
                        .iter()
                        .map(|entry| {
                            JsonValue::Object(vec![
                                ("name".to_string(), JsonValue::String(entry.name.clone())),
                                (
                                    "image_id".to_string(),
                                    JsonValue::String(entry.image_id.clone()),
                                ),
                                (
                                    "install_name".to_string(),
                                    JsonValue::String(entry.install_name.clone()),
                                ),
                                ("cache_vmaddr".to_string(), u64_num(entry.cache_vmaddr)),
                                (
                                    "image_base_vmaddr".to_string(),
                                    u64_num(entry.image_base_vmaddr),
                                ),
                                ("image_offset".to_string(), u64_num(entry.image_offset)),
                                (
                                    "member_file_offset".to_string(),
                                    entry
                                        .member_file_offset
                                        .map(u64_num)
                                        .unwrap_or(JsonValue::Null),
                                ),
                                (
                                    "source".to_string(),
                                    JsonValue::String(entry.source.clone()),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

fn cache_image_json(image: &CacheImageView) -> JsonValue {
    JsonValue::Object(vec![
        ("id".to_string(), JsonValue::String(image.id.clone())),
        ("image_index".to_string(), u64_num(image.image_index)),
        (
            "install_name".to_string(),
            JsonValue::String(image.install_name.clone()),
        ),
        (
            "basename".to_string(),
            JsonValue::String(image.basename.clone()),
        ),
        (
            "image_base_vmaddr".to_string(),
            u64_num(image.image_base_vmaddr),
        ),
        (
            "member_name".to_string(),
            JsonValue::String(image.member_name.clone()),
        ),
    ])
}

fn cache_export_json(export: &CacheExportView) -> JsonValue {
    JsonValue::Object(vec![
        ("name".to_string(), JsonValue::String(export.name.clone())),
        (
            "cache_vmaddr".to_string(),
            export.cache_vmaddr.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        ("kind".to_string(), JsonValue::String(export.kind.clone())),
        ("flags".to_string(), JsonValue::String(export.flags.clone())),
    ])
}

fn collection_metadata_json(metadata: &CacheCollectionMetadata) -> JsonValue {
    JsonValue::Object(vec![
        ("total".to_string(), usize_num(metadata.total)),
        ("returned".to_string(), usize_num(metadata.returned)),
        ("truncated".to_string(), JsonValue::Bool(metadata.truncated)),
    ])
}

fn slice_descriptor_json(descriptor: &SliceDescriptor) -> JsonValue {
    JsonValue::Object(vec![
        ("offset".to_string(), u64_num(descriptor.offset)),
        ("size".to_string(), u64_num(descriptor.size)),
        (
            "is_universal".to_string(),
            JsonValue::Bool(descriptor.is_universal),
        ),
        (
            "cpu_subtype".to_string(),
            u64_num(u64::from(descriptor.cpu_subtype)),
        ),
        (
            "architecture".to_string(),
            JsonValue::String(descriptor.architecture.to_string()),
        ),
        ("selected".to_string(), JsonValue::Bool(descriptor.selected)),
    ])
}

fn included_dyld_sections(view: &DyldViewOptions) -> Vec<JsonValue> {
    let mut sections = Vec::new();
    if view.show_dylibs {
        sections.push(JsonValue::String("dylibs".to_string()));
    }
    if view.show_rpaths {
        sections.push(JsonValue::String("rpaths".to_string()));
    }
    if view.show_exports {
        sections.push(JsonValue::String("exports".to_string()));
    }
    if view.show_function_starts {
        sections.push(JsonValue::String("function_starts".to_string()));
    }
    if view.show_bindings {
        sections.push(JsonValue::String("import_bindings".to_string()));
    }
    if view.show_stubs {
        sections.push(JsonValue::String("stubs".to_string()));
    }
    if view.show_helpers {
        sections.push(JsonValue::String("stub_helpers".to_string()));
    }
    sections
}

fn export_json(export: &damsel_core::ExportRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("name".to_string(), JsonValue::String(export.name.clone())),
        (
            "address".to_string(),
            export.address.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "raw_flags".to_string(),
            JsonValue::String(export.raw_flags.clone()),
        ),
        ("flags".to_string(), export_flags_json(&export.flags)),
        ("kind".to_string(), export_kind_json(&export.kind)),
        (
            "reexport_target".to_string(),
            export
                .reexport_target
                .as_ref()
                .map(|(dylib, symbol)| {
                    JsonValue::Object(vec![
                        ("dylib".to_string(), JsonValue::String(dylib.clone())),
                        (
                            "symbol".to_string(),
                            symbol
                                .as_ref()
                                .map(|value| JsonValue::String(value.clone()))
                                .unwrap_or(JsonValue::Null),
                        ),
                    ])
                })
                .unwrap_or(JsonValue::Null),
        ),
        (
            "resolver_target".to_string(),
            export
                .resolver_target
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn export_flags_json(flags: &damsel_core::ExportFlags) -> JsonValue {
    JsonValue::Object(vec![
        ("raw_bits".to_string(), u64_num(flags.raw_bits)),
        ("kind_bits".to_string(), u64_num(u64::from(flags.kind_bits))),
        (
            "is_weak_definition".to_string(),
            JsonValue::Bool(flags.is_weak_definition),
        ),
        (
            "is_reexport".to_string(),
            JsonValue::Bool(flags.is_reexport),
        ),
        (
            "is_stub_and_resolver".to_string(),
            JsonValue::Bool(flags.is_stub_and_resolver),
        ),
        (
            "is_thread_local".to_string(),
            JsonValue::Bool(flags.is_thread_local),
        ),
        (
            "is_absolute".to_string(),
            JsonValue::Bool(flags.is_absolute),
        ),
        ("unknown_bits".to_string(), u64_num(flags.unknown_bits)),
        (
            "flag_names".to_string(),
            JsonValue::Array(
                flags
                    .flag_names()
                    .map(|flag| JsonValue::String(format!("{flag:?}")))
                    .collect(),
            ),
        ),
    ])
}

fn export_kind_json(kind: &ExportKind) -> JsonValue {
    match kind {
        ExportKind::Regular => JsonValue::Object(vec![(
            "type".to_string(),
            JsonValue::String("regular".to_string()),
        )]),
        ExportKind::Reexport { dylib, symbol } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("reexport".to_string()),
            ),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            (
                "symbol".to_string(),
                symbol
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
        ]),
        ExportKind::Resolver { resolver_address } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("resolver".to_string()),
            ),
            (
                "resolver_address".to_string(),
                resolver_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
        ]),
        ExportKind::StubAndResolver {
            stub_address,
            resolver_address,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("stub_and_resolver".to_string()),
            ),
            (
                "stub_address".to_string(),
                stub_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "resolver_address".to_string(),
                resolver_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
        ]),
        ExportKind::WeakDefinition => JsonValue::Object(vec![(
            "type".to_string(),
            JsonValue::String("weak_definition".to_string()),
        )]),
        ExportKind::Absolute => JsonValue::Object(vec![(
            "type".to_string(),
            JsonValue::String("absolute".to_string()),
        )]),
        ExportKind::ThreadLocal => JsonValue::Object(vec![(
            "type".to_string(),
            JsonValue::String("thread_local".to_string()),
        )]),
        ExportKind::Unknown(value) => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("unknown".to_string())),
            ("value".to_string(), JsonValue::String(value.clone())),
        ]),
    }
}

fn import_binding_json(binding: &ImportBindingRecord) -> JsonValue {
    JsonValue::Object(vec![
        (
            "dylib".to_string(),
            JsonValue::String(binding.dylib.clone()),
        ),
        ("name".to_string(), JsonValue::String(binding.name.clone())),
        (
            "address".to_string(),
            binding.address.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "offset".to_string(),
            binding.offset.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        ("addend".to_string(), i64_num(binding.addend)),
        (
            "source".to_string(),
            JsonValue::String(format!("{:?}", binding.source)),
        ),
        (
            "ordinal".to_string(),
            binding
                .ordinal
                .map(|value| u64_num(u64::from(value)))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "symbol_index".to_string(),
            binding
                .symbol_index
                .map(|value| u64_num(u64::from(value)))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "binding_kind".to_string(),
            JsonValue::String(format!("{:?}", binding.binding_kind)),
        ),
        ("is_weak".to_string(), JsonValue::Bool(binding.is_weak)),
    ])
}

fn stub_entry_json(stub: &StubEntry) -> JsonValue {
    JsonValue::Object(vec![
        ("stub_address".to_string(), u64_num(stub.stub_address)),
        (
            "section".to_string(),
            stub.section
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "pointer_address".to_string(),
            stub.pointer_address.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "pointer_section".to_string(),
            stub.pointer_section
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "helper_address".to_string(),
            stub.helper_address.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "binding_ordinal".to_string(),
            stub.binding_ordinal
                .map(|value| u64_num(u64::from(value)))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "stub_kind".to_string(),
            JsonValue::String(format!("{:?}", stub.stub_kind)),
        ),
        (
            "dylib".to_string(),
            stub.dylib
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name".to_string(),
            stub.name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "source".to_string(),
            JsonValue::String(format!("{:?}", stub.source)),
        ),
    ])
}

fn stub_helper_json(helper: &StubHelperEntry) -> JsonValue {
    JsonValue::Object(vec![
        ("helper_address".to_string(), u64_num(helper.helper_address)),
        (
            "target_stub".to_string(),
            helper.target_stub.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "binding_ordinal".to_string(),
            helper
                .binding_ordinal
                .map(|value| u64_num(u64::from(value)))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "stub_section".to_string(),
            helper
                .stub_section
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "pointer_address".to_string(),
            helper
                .pointer_address
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "pointer_section".to_string(),
            helper
                .pointer_section
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "dylib".to_string(),
            helper
                .dylib
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name".to_string(),
            helper
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn objc_class_record_json(record: &ObjcClassRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("class_pointer".to_string(), u64_num(record.class_pointer)),
        (
            "name".to_string(),
            record
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name_source".to_string(),
            JsonValue::String(format!("{:?}", record.name_source)),
        ),
        (
            "superclass_pointer".to_string(),
            record
                .superclass_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "superclass_name".to_string(),
            record
                .superclass_name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "ro_pointer".to_string(),
            record.ro_pointer.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "metaclass_pointer".to_string(),
            record
                .metaclass_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "method_list_pointer".to_string(),
            record
                .method_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "property_list_pointer".to_string(),
            record
                .property_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "protocol_list_pointer".to_string(),
            record
                .protocol_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "ivar_list_pointer".to_string(),
            record
                .ivar_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "adopted_protocols".to_string(),
            JsonValue::Array(
                record
                    .adopted_protocols
                    .iter()
                    .cloned()
                    .map(JsonValue::String)
                    .collect(),
            ),
        ),
        (
            "methods".to_string(),
            JsonValue::Array(record.methods.iter().map(objc_method_record_json).collect()),
        ),
        (
            "class_methods".to_string(),
            JsonValue::Array(
                record
                    .class_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "properties".to_string(),
            JsonValue::Array(
                record
                    .properties
                    .iter()
                    .map(objc_property_record_json)
                    .collect(),
            ),
        ),
        (
            "ivars".to_string(),
            JsonValue::Array(record.ivars.iter().map(objc_ivar_record_json).collect()),
        ),
    ])
}

fn objc_protocol_record_json(record: &ObjcProtocolRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("pointer".to_string(), u64_num(record.pointer)),
        (
            "name".to_string(),
            record
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name_source".to_string(),
            JsonValue::String(format!("{:?}", record.name_source)),
        ),
        (
            "required_instance_methods".to_string(),
            JsonValue::Array(
                record
                    .required_instance_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "required_class_methods".to_string(),
            JsonValue::Array(
                record
                    .required_class_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "optional_instance_methods".to_string(),
            JsonValue::Array(
                record
                    .optional_instance_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "optional_class_methods".to_string(),
            JsonValue::Array(
                record
                    .optional_class_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "properties".to_string(),
            JsonValue::Array(
                record
                    .properties
                    .iter()
                    .map(objc_property_record_json)
                    .collect(),
            ),
        ),
    ])
}

fn objc_category_record_json(record: &ObjcCategoryRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("pointer".to_string(), u64_num(record.pointer)),
        (
            "name".to_string(),
            record
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "class_pointer".to_string(),
            record.class_pointer.map(u64_num).unwrap_or(JsonValue::Null),
        ),
        (
            "class_name".to_string(),
            record
                .class_name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name_source".to_string(),
            JsonValue::String(format!("{:?}", record.name_source)),
        ),
        (
            "class_name_source".to_string(),
            JsonValue::String(format!("{:?}", record.class_name_source)),
        ),
        (
            "property_list_pointer".to_string(),
            record
                .property_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "protocol_list_pointer".to_string(),
            record
                .protocol_list_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "properties_source".to_string(),
            JsonValue::String(format!("{:?}", record.properties_source)),
        ),
        (
            "protocols_source".to_string(),
            JsonValue::String(format!("{:?}", record.protocols_source)),
        ),
        (
            "record_source".to_string(),
            JsonValue::String(format!("{:?}", record.record_source)),
        ),
        (
            "adopted_protocols".to_string(),
            JsonValue::Array(
                record
                    .adopted_protocols
                    .iter()
                    .cloned()
                    .map(JsonValue::String)
                    .collect(),
            ),
        ),
        (
            "methods".to_string(),
            JsonValue::Array(record.methods.iter().map(objc_method_record_json).collect()),
        ),
        (
            "class_methods".to_string(),
            JsonValue::Array(
                record
                    .class_methods
                    .iter()
                    .map(objc_method_record_json)
                    .collect(),
            ),
        ),
        (
            "properties".to_string(),
            JsonValue::Array(
                record
                    .properties
                    .iter()
                    .map(objc_property_record_json)
                    .collect(),
            ),
        ),
    ])
}

fn objc_method_record_json(record: &ObjcMethodRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("owner_pointer".to_string(), u64_num(record.owner_pointer)),
        (
            "owner_kind".to_string(),
            JsonValue::String(
                match record.owner_kind {
                    ObjcMethodOwnerKind::Class => "class",
                    ObjcMethodOwnerKind::Metaclass => "metaclass",
                    ObjcMethodOwnerKind::Protocol => "protocol",
                    ObjcMethodOwnerKind::Category => "category",
                }
                .to_string(),
            ),
        ),
        (
            "is_class_method".to_string(),
            JsonValue::Bool(record.is_class_method),
        ),
        (
            "selector".to_string(),
            record
                .selector
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "selector_source".to_string(),
            JsonValue::String(format!("{:?}", record.selector_source)),
        ),
        (
            "implementation".to_string(),
            record
                .implementation
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "type_encoding".to_string(),
            record
                .type_encoding
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn objc_property_record_json(record: &ObjcPropertyRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("owner_pointer".to_string(), u64_num(record.owner_pointer)),
        (
            "name".to_string(),
            record
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name_source".to_string(),
            JsonValue::String(format!("{:?}", record.name_source)),
        ),
        (
            "attributes".to_string(),
            record
                .attributes
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn objc_ivar_record_json(record: &ObjcIvarRecord) -> JsonValue {
    JsonValue::Object(vec![
        ("owner_pointer".to_string(), u64_num(record.owner_pointer)),
        (
            "name".to_string(),
            record
                .name
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "name_source".to_string(),
            JsonValue::String(format!("{:?}", record.name_source)),
        ),
        (
            "type_encoding".to_string(),
            record
                .type_encoding
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "offset".to_string(),
            record.offset.map(u64_num).unwrap_or(JsonValue::Null),
        ),
    ])
}

fn objc_method_view_json(view: &ObjcOwnedRecordView<'_, ObjcMethodRecord>) -> JsonValue {
    let mut object = match objc_method_record_json(view.record) {
        JsonValue::Object(object) => object,
        _ => unreachable!(),
    };
    object.push((
        "owner_name".to_string(),
        view.owner_name
            .as_ref()
            .map(|value| JsonValue::String(value.clone()))
            .unwrap_or(JsonValue::Null),
    ));
    JsonValue::Object(object)
}

fn objc_property_view_json(view: &ObjcOwnedRecordView<'_, ObjcPropertyRecord>) -> JsonValue {
    let mut object = match objc_property_record_json(view.record) {
        JsonValue::Object(object) => object,
        _ => unreachable!(),
    };
    object.push((
        "owner_name".to_string(),
        view.owner_name
            .as_ref()
            .map(|value| JsonValue::String(value.clone()))
            .unwrap_or(JsonValue::Null),
    ));
    JsonValue::Object(object)
}

fn objc_ivar_view_json(view: &ObjcOwnedRecordView<'_, ObjcIvarRecord>) -> JsonValue {
    let mut object = match objc_ivar_record_json(view.record) {
        JsonValue::Object(object) => object,
        _ => unreachable!(),
    };
    object.push((
        "owner_name".to_string(),
        view.owner_name
            .as_ref()
            .map(|value| JsonValue::String(value.clone()))
            .unwrap_or(JsonValue::Null),
    ));
    JsonValue::Object(object)
}

fn objc_pointer_kind_text(kind: ObjcPointerKind) -> &'static str {
    match kind {
        ObjcPointerKind::SelRef => "selref",
        ObjcPointerKind::ClassRef => "classref",
        ObjcPointerKind::ClassList => "classlist",
    }
}

struct DisasmJsonDto<'a> {
    view: DisassemblyView<'a>,
    render_options: DisassemblyRenderOptions,
}

impl JsonDto for DisasmJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let instructions = self
            .view
            .instructions
            .iter()
            .map(|instruction| {
                JsonValue::Object(vec![
                    ("address".to_string(), u64_num(instruction.address)),
                    ("size".to_string(), u64_num(u64::from(instruction.size))),
                    ("opcode".to_string(), u64_num(u64::from(instruction.opcode))),
                    (
                        "mnemonic".to_string(),
                        JsonValue::String(instruction.mnemonic.clone()),
                    ),
                    (
                        "operands".to_string(),
                        JsonValue::Array(
                            instruction
                                .operands
                                .iter()
                                .map(ToString::to_string)
                                .map(JsonValue::String)
                                .collect(),
                        ),
                    ),
                    (
                        "rendered".to_string(),
                        JsonValue::String(instruction.render()),
                    ),
                    (
                        "references".to_string(),
                        if self.render_options.include_references {
                            JsonValue::Array(
                                instruction.references.iter().map(reference_json).collect(),
                            )
                        } else {
                            JsonValue::Array(Vec::new())
                        },
                    ),
                    (
                        "annotations".to_string(),
                        if self.render_options.include_annotations {
                            JsonValue::Array(
                                instruction
                                    .annotations
                                    .iter()
                                    .map(annotation_json)
                                    .collect(),
                            )
                        } else {
                            JsonValue::Array(Vec::new())
                        },
                    ),
                    (
                        "recovered_values".to_string(),
                        if self.render_options.include_values {
                            JsonValue::Array(
                                instruction
                                    .recovered_values
                                    .iter()
                                    .map(recovered_value_json)
                                    .collect(),
                            )
                        } else {
                            JsonValue::Array(Vec::new())
                        },
                    ),
                ])
            })
            .collect();

        JsonValue::Object(vec![
            (
                "target".to_string(),
                JsonValue::String(self.view.target.to_string()),
            ),
            (
                "start_address".to_string(),
                u64_num(self.view.start_address),
            ),
            ("end_address".to_string(), u64_num(self.view.end_address)),
            (
                "window_end".to_string(),
                self.view.window_end.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            ("bytes_len".to_string(), usize_num(self.view.bytes_len)),
            (
                "decoded_bytes".to_string(),
                usize_num(self.view.decoded_bytes),
            ),
            (
                "instruction_count".to_string(),
                usize_num(self.view.instruction_count),
            ),
            (
                "stop_reason".to_string(),
                JsonValue::String(self.view.stop_reason.clone()),
            ),
            ("instructions".to_string(), JsonValue::Array(instructions)),
        ])
    }
}

struct ErrorJsonDto<'a> {
    error: &'a ErrorResponse,
}

impl JsonDto for ErrorJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let error = self.error;
        JsonValue::Object(vec![
            (
                "code".to_string(),
                JsonValue::String(error.code.to_string()),
            ),
            (
                "message".to_string(),
                JsonValue::String(error.message.to_string()),
            ),
            (
                "details".to_string(),
                error
                    .details
                    .as_ref()
                    .map(|value| JsonValue::String(value.to_string()))
                    .unwrap_or(JsonValue::Null),
            ),
        ])
    }
}

struct DoctorJsonDto<'a> {
    report: &'a DoctorReport,
}

impl JsonDto for DoctorJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let report = self.report;
        JsonValue::Object(vec![
            (
                "host".to_string(),
                JsonValue::Object(vec![
                    (
                        "os".to_string(),
                        JsonValue::String(report.host_platform.to_string()),
                    ),
                    (
                        "architecture".to_string(),
                        JsonValue::String(report.host_architecture.to_string()),
                    ),
                    (
                        "target_triple".to_string(),
                        report
                            .target_triple
                            .as_ref()
                            .map(|value| JsonValue::String(value.clone()))
                            .unwrap_or(JsonValue::Null),
                    ),
                ]),
            ),
            (
                "overall_status".to_string(),
                JsonValue::String(report.overall_status.to_string()),
            ),
            (
                "capabilities".to_string(),
                JsonValue::Object(vec![
                    (
                        "macho_analysis".to_string(),
                        capability_report_json(&report.macho_analysis),
                    ),
                    (
                        "fixture_rebuild".to_string(),
                        capability_report_json(&report.fixture_rebuild),
                    ),
                    (
                        "fixture_drift_check".to_string(),
                        capability_report_json(&report.fixture_drift_check),
                    ),
                    (
                        "bench_compile".to_string(),
                        capability_report_json(&report.bench_compile),
                    ),
                    (
                        "bench_runtime".to_string(),
                        capability_report_json(&report.bench_runtime),
                    ),
                    (
                        "benchmark".to_string(),
                        capability_report_json(&report.benchmark),
                    ),
                ]),
            ),
            (
                "tools".to_string(),
                JsonValue::Object(vec![
                    ("xcrun".to_string(), tool_status_json(&report.tools.xcrun)),
                    ("strip".to_string(), tool_status_json(&report.tools.strip)),
                    ("clang".to_string(), tool_status_json(&report.tools.clang)),
                    (
                        "python3".to_string(),
                        tool_status_json(&report.tools.python3),
                    ),
                    ("nm".to_string(), tool_status_json(&report.tools.nm)),
                    (
                        "sdk_path_probe".to_string(),
                        tool_status_json(&report.tools.sdk_path_probe),
                    ),
                    (
                        "hash_tools".to_string(),
                        JsonValue::Object(vec![
                            (
                                "sha256sum".to_string(),
                                tool_status_json(&report.tools.hash_tools.sha256sum),
                            ),
                            (
                                "shasum".to_string(),
                                tool_status_json(&report.tools.hash_tools.shasum),
                            ),
                            (
                                "openssl".to_string(),
                                tool_status_json(&report.tools.hash_tools.openssl),
                            ),
                        ]),
                    ),
                    (
                        "selected_hash_tool".to_string(),
                        report
                            .tools
                            .selected_hash_tool
                            .as_ref()
                            .map(|value| JsonValue::String(value.clone()))
                            .unwrap_or(JsonValue::Null),
                    ),
                ]),
            ),
            (
                "issues".to_string(),
                JsonValue::Array(report.issues.iter().map(compatibility_issue_json).collect()),
            ),
        ])
    }
}

fn reference_to_text(reference: &Reference) -> String {
    match reference {
        Reference::Call { target } => format!("call {target:#x}"),
        Reference::Branch { target } => format!("branch {target:#x}"),
        Reference::IndirectCall { via } => format!("indirect-call via {via}"),
        Reference::IndirectBranch { via } => format!("indirect-branch via {via}"),
        Reference::Page { target } => format!("page {target:#x}"),
        Reference::Data { target } => format!("data {target:#x}"),
        Reference::Import {
            name,
            dylib,
            address,
        } => match address {
            Some(address) => format!("import {dylib}:{name} ({address:#x})"),
            None => format!("import {dylib}:{name}"),
        },
        Reference::ImportBinding {
            dylib,
            name,
            address,
            offset,
            addend,
            binding_kind,
            source,
            is_weak,
        } => format!(
            "import-binding {}:{} addr={} off={} addend={} kind={:?} source={:?} weak={}",
            dylib,
            name,
            address
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            offset
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            addend,
            binding_kind,
            source,
            is_weak
        ),
        Reference::Stub {
            stub_address,
            section: _,
            pointer_address,
            dylib,
            name,
            stub_kind,
            source,
            ..
        } => format!(
            "stub {stub_address:#x} ptr={} kind={stub_kind:?} {}:{} source={:?}",
            pointer_address
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            dylib.clone().unwrap_or_else(|| "-".to_string()),
            name.clone().unwrap_or_else(|| "-".to_string()),
            source
        ),
        Reference::StubHelper {
            helper_address,
            target_stub,
            stub_section,
            pointer_address,
            pointer_section,
            binding_ordinal,
            dylib,
            name,
            ..
        } => format!(
            "stub-helper {helper_address:#x} stub={} stub_section={} ptr={} ptr_section={} ordinal={} {}:{}",
            target_stub
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            stub_section.clone().unwrap_or_else(|| "-".to_string()),
            pointer_address
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            pointer_section.clone().unwrap_or_else(|| "-".to_string()),
            binding_ordinal
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            dylib.clone().unwrap_or_else(|| "-".to_string()),
            name.clone().unwrap_or_else(|| "-".to_string()),
        ),
        Reference::RelocationEvidence {
            address,
            kind,
            encoding,
            target,
            addend,
        } => format!(
            "reloc-evidence {address:#x} kind={kind} enc={encoding} target={target} addend={addend}"
        ),
    }
}

fn reference_json(reference: &Reference) -> JsonValue {
    match reference {
        Reference::Call { target } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("call".to_string())),
            ("target".to_string(), u64_num(*target)),
        ]),
        Reference::Branch { target } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("branch".to_string())),
            ("target".to_string(), u64_num(*target)),
        ]),
        Reference::IndirectCall { via } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("indirect_call".to_string()),
            ),
            ("via".to_string(), JsonValue::String(via.clone())),
        ]),
        Reference::IndirectBranch { via } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("indirect_branch".to_string()),
            ),
            ("via".to_string(), JsonValue::String(via.clone())),
        ]),
        Reference::Page { target } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("page".to_string())),
            ("target".to_string(), u64_num(*target)),
        ]),
        Reference::Data { target } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("data".to_string())),
            ("target".to_string(), u64_num(*target)),
        ]),
        Reference::Import {
            name,
            dylib,
            address,
        } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("import".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            (
                "address".to_string(),
                address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
        ]),
        Reference::ImportBinding {
            dylib,
            name,
            address,
            offset,
            addend,
            binding_kind,
            source,
            is_weak,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("import_binding".to_string()),
            ),
            ("name".to_string(), JsonValue::String(name.clone())),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            (
                "address".to_string(),
                address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "offset".to_string(),
                offset.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            ("addend".to_string(), i64_num(*addend)),
            (
                "binding_kind".to_string(),
                JsonValue::String(format!("{binding_kind:?}")),
            ),
            (
                "source".to_string(),
                JsonValue::String(format!("{source:?}")),
            ),
            ("is_weak".to_string(), JsonValue::Bool(*is_weak)),
        ]),
        Reference::Stub {
            stub_address,
            section,
            pointer_section,
            pointer_address,
            helper_address,
            binding_ordinal,
            stub_kind,
            dylib,
            name,
            source,
        } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("stub".to_string())),
            ("stub_address".to_string(), u64_num(*stub_address)),
            (
                "section".to_string(),
                section
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "pointer_section".to_string(),
                pointer_section
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "pointer_address".to_string(),
                pointer_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "helper_address".to_string(),
                helper_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "binding_ordinal".to_string(),
                binding_ordinal
                    .map(|value| u64_num(u64::from(value)))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "stub_kind".to_string(),
                JsonValue::String(format!("{stub_kind:?}")),
            ),
            (
                "dylib".to_string(),
                dylib
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "name".to_string(),
                name.as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "source".to_string(),
                JsonValue::String(format!("{source:?}")),
            ),
        ]),
        Reference::StubHelper {
            helper_address,
            target_stub,
            stub_section,
            pointer_address,
            pointer_section,
            binding_ordinal,
            dylib,
            name,
            ..
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("stub_helper".to_string()),
            ),
            ("helper_address".to_string(), u64_num(*helper_address)),
            (
                "target_stub".to_string(),
                target_stub.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "stub_section".to_string(),
                stub_section
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "pointer_address".to_string(),
                pointer_address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "pointer_section".to_string(),
                pointer_section
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "binding_ordinal".to_string(),
                binding_ordinal
                    .map(|value| u64_num(u64::from(value)))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "dylib".to_string(),
                dylib
                    .as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "name".to_string(),
                name.as_ref()
                    .map(|value| JsonValue::String(value.clone()))
                    .unwrap_or(JsonValue::Null),
            ),
        ]),
        Reference::RelocationEvidence {
            address,
            kind,
            encoding,
            target,
            addend,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("relocation_evidence".to_string()),
            ),
            ("address".to_string(), u64_num(*address)),
            ("kind".to_string(), JsonValue::String(kind.clone())),
            ("encoding".to_string(), JsonValue::String(encoding.clone())),
            ("target".to_string(), JsonValue::String(target.clone())),
            ("addend".to_string(), i64_num(*addend)),
        ]),
    }
}

fn annotation_json(annotation: &Annotation) -> JsonValue {
    match annotation {
        Annotation::Symbol(name) => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("symbol".to_string())),
            ("name".to_string(), JsonValue::String(name.clone())),
        ]),
        Annotation::TargetSymbol { address, name } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("target_symbol".to_string()),
            ),
            ("address".to_string(), u64_num(*address)),
            ("name".to_string(), JsonValue::String(name.clone())),
        ]),
        Annotation::Import { dylib, name } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("import".to_string())),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            ("name".to_string(), JsonValue::String(name.clone())),
        ]),
        Annotation::IndirectControlFlow { kind, via } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("indirect_control_flow".to_string()),
            ),
            ("kind".to_string(), JsonValue::String(kind.clone())),
            ("via".to_string(), JsonValue::String(via.clone())),
        ]),
        Annotation::Relocation {
            address,
            kind,
            encoding,
            target,
            addend,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("relocation".to_string()),
            ),
            ("address".to_string(), u64_num(*address)),
            ("kind".to_string(), JsonValue::String(kind.clone())),
            ("encoding".to_string(), JsonValue::String(encoding.clone())),
            ("target".to_string(), JsonValue::String(target.clone())),
            ("addend".to_string(), i64_num(*addend)),
        ]),
        Annotation::ImportBinding {
            dylib,
            name,
            address,
            offset,
            addend,
            binding_kind,
            source,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("import_binding".to_string()),
            ),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            ("name".to_string(), JsonValue::String(name.clone())),
            (
                "address".to_string(),
                address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "offset".to_string(),
                offset.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            ("addend".to_string(), i64_num(*addend)),
            (
                "binding_kind".to_string(),
                JsonValue::String(format!("{binding_kind:?}")),
            ),
            (
                "source".to_string(),
                JsonValue::String(format!("{source:?}")),
            ),
        ]),
        Annotation::ImportBindingEvidence {
            dylib,
            name,
            address,
            offset,
            addend,
            binding_kind,
            source,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("import_binding_evidence".to_string()),
            ),
            ("dylib".to_string(), JsonValue::String(dylib.clone())),
            ("name".to_string(), JsonValue::String(name.clone())),
            (
                "address".to_string(),
                address.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            (
                "offset".to_string(),
                offset.map(u64_num).unwrap_or(JsonValue::Null),
            ),
            ("addend".to_string(), i64_num(*addend)),
            (
                "binding_kind".to_string(),
                JsonValue::String(format!("{binding_kind:?}")),
            ),
            (
                "source".to_string(),
                JsonValue::String(format!("{source:?}")),
            ),
        ]),
        Annotation::JumpTableCandidate {
            base,
            index_register,
            element_size,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("jump_table_candidate".to_string()),
            ),
            ("base".to_string(), u64_num(*base)),
            (
                "index_register".to_string(),
                JsonValue::String(index_register.clone()),
            ),
            (
                "element_size".to_string(),
                u64_num(u64::from(*element_size)),
            ),
        ]),
        Annotation::RelocationEvidence {
            address,
            kind,
            encoding,
            target,
            addend,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("relocation_evidence".to_string()),
            ),
            ("address".to_string(), u64_num(*address)),
            ("kind".to_string(), JsonValue::String(kind.clone())),
            ("encoding".to_string(), JsonValue::String(encoding.clone())),
            ("target".to_string(), JsonValue::String(target.clone())),
            ("addend".to_string(), i64_num(*addend)),
        ]),
        Annotation::TableSlotResolved {
            table_base,
            slot_address,
            index_register,
            element_size,
            encoding,
            target,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("table_slot_resolved".to_string()),
            ),
            ("table_base".to_string(), u64_num(*table_base)),
            ("slot_address".to_string(), u64_num(*slot_address)),
            (
                "index_register".to_string(),
                JsonValue::String(index_register.clone()),
            ),
            (
                "element_size".to_string(),
                u64_num(u64::from(*element_size)),
            ),
            (
                "encoding".to_string(),
                JsonValue::String(encoding.to_string()),
            ),
            ("target".to_string(), u64_num(*target)),
        ]),
        Annotation::IndirectTargetResolved {
            via,
            target,
            reason,
        } => JsonValue::Object(vec![
            (
                "type".to_string(),
                JsonValue::String("indirect_target_resolved".to_string()),
            ),
            ("via".to_string(), JsonValue::String(via.clone())),
            ("target".to_string(), u64_num(*target)),
            ("reason".to_string(), JsonValue::String(reason.to_string())),
        ]),
        Annotation::Note(note) => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("note".to_string())),
            ("value".to_string(), JsonValue::String(note.clone())),
        ]),
    }
}

fn recovered_value_to_text(value: &RecoveredValue) -> String {
    format!(
        "{}={:#x} kind={:?} source={:?}",
        value.register, value.value, value.kind, value.source
    )
}

fn recovered_value_json(value: &RecoveredValue) -> JsonValue {
    JsonValue::Object(vec![
        (
            "register".to_string(),
            JsonValue::String(value.register.clone()),
        ),
        ("value".to_string(), u64_num(value.value)),
        (
            "kind".to_string(),
            JsonValue::String(format!("{:?}", value.kind)),
        ),
        (
            "source".to_string(),
            JsonValue::String(format!("{:?}", value.source)),
        ),
    ])
}

fn capability_report_json(report: &CapabilityReport) -> JsonValue {
    JsonValue::Object(vec![
        (
            "status".to_string(),
            JsonValue::String(report.status.to_string()),
        ),
        (
            "reasons".to_string(),
            JsonValue::Array(
                report
                    .reasons
                    .iter()
                    .map(compatibility_issue_json)
                    .collect(),
            ),
        ),
    ])
}

fn tool_status_json(status: &ToolStatus) -> JsonValue {
    JsonValue::Object(vec![
        ("detected".to_string(), JsonValue::Bool(status.detected)),
        ("usable".to_string(), JsonValue::Bool(status.usable)),
        (
            "path".to_string(),
            status
                .path
                .as_ref()
                .map(|value| JsonValue::String(value.clone()))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn compatibility_issue_json(issue: &CompatibilityIssue) -> JsonValue {
    JsonValue::Object(vec![
        (
            "code".to_string(),
            JsonValue::String(issue.code.to_string()),
        ),
        (
            "message".to_string(),
            JsonValue::String(issue.message.clone()),
        ),
    ])
}

fn print_doctor_text(report: &DoctorReport) {
    println!("host_os: {}", report.host_platform);
    println!("host_architecture: {}", report.host_architecture);
    println!(
        "target_triple: {}",
        report.target_triple.as_deref().unwrap_or("-")
    );
    println!("overall_status: {}", report.overall_status);
    println!("capabilities:");
    print_capability_line("macho_analysis", &report.macho_analysis);
    print_capability_line("fixture_rebuild", &report.fixture_rebuild);
    print_capability_line("fixture_drift_check", &report.fixture_drift_check);
    print_capability_line("bench_compile", &report.bench_compile);
    print_capability_line("bench_runtime", &report.bench_runtime);
    print_capability_line("benchmark", &report.benchmark);
    println!("tools:");
    print_tool_line("xcrun", &report.tools.xcrun);
    print_tool_line("strip", &report.tools.strip);
    print_tool_line("clang", &report.tools.clang);
    print_tool_line("python3", &report.tools.python3);
    print_tool_line("nm", &report.tools.nm);
    print_tool_line("sdk_path_probe", &report.tools.sdk_path_probe);
    print_tool_line("sha256sum", &report.tools.hash_tools.sha256sum);
    print_tool_line("shasum", &report.tools.hash_tools.shasum);
    print_tool_line("openssl", &report.tools.hash_tools.openssl);
    println!(
        "  selected_hash_tool: {}",
        report.tools.selected_hash_tool.as_deref().unwrap_or("none")
    );
    if report.issues.is_empty() {
        println!("issues: none");
    } else {
        println!("issues:");
        for issue in &report.issues {
            println!("  - {}: {}", issue.code, issue.message);
        }
    }
}

fn doctor_non_summary_capabilities() -> impl Iterator<Item = CompatibilityCapability> {
    CompatibilityPolicy::DOCTOR_CHECK_ALL.into_iter()
}

fn doctor_non_summary_reports<'a>(report: &'a DoctorReport) -> Vec<&'a CapabilityReport> {
    doctor_non_summary_capabilities()
        .map(|capability| report.capability(capability))
        .collect()
}

fn doctor_summary_reports<'a>(
    report: &'a DoctorReport,
    capability: CompatibilityCapability,
) -> Vec<&'a CapabilityReport> {
    CompatibilityPolicy::capability(capability)
        .summary_inputs
        .iter()
        .map(|input| report.capability(*input))
        .collect()
}

fn print_capability_line(name: &str, report: &CapabilityReport) {
    println!("  {name}: {}", report.status);
    for reason in &report.reasons {
        println!("    - {}: {}", reason.code, reason.message);
    }
}

fn print_tool_line(name: &str, status: &ToolStatus) {
    let detected = if status.detected { "yes" } else { "no" };
    let usable = if status.usable { "yes" } else { "no" };
    let path = status.path.as_deref().unwrap_or("-");
    println!("  {name}: detected={detected} usable={usable} path={path}");
}

fn collect_doctor_report() -> DoctorReport {
    let context = doctor_context_override_from_env().unwrap_or_else(default_doctor_context);
    evaluate_doctor_report(&context)
}

fn default_doctor_context() -> DoctorContext {
    DoctorContext {
        host_platform: HostPlatform::current(),
        host_architecture: HostArchitecture::current(),
        target_triple: option_env!("DAMSEL_TARGET_TRIPLE").map(ToString::to_string),
        tools: detect_doctor_tools(),
    }
}

fn doctor_context_override_from_env() -> Option<DoctorContext> {
    let scenario = env::var(DOCTOR_TEST_SCENARIO_ENV).ok()?;
    let (host_platform, host_architecture) = parse_doctor_test_scenario(&scenario)?;
    let target_triple = env::var(DOCTOR_TEST_TARGET_TRIPLE_ENV)
        .ok()
        .or_else(|| option_env!("DAMSEL_TARGET_TRIPLE").map(ToString::to_string));
    Some(DoctorContext {
        host_platform,
        host_architecture,
        target_triple,
        tools: detect_doctor_tools(),
    })
}

fn parse_doctor_test_scenario(value: &str) -> Option<(HostPlatform, HostArchitecture)> {
    match value {
        "linux-arm64" => Some((HostPlatform::Linux, HostArchitecture::Arm64)),
        "linux-x86_64" => Some((HostPlatform::Linux, HostArchitecture::X86_64)),
        "macos-arm64" => Some((HostPlatform::MacOS, HostArchitecture::Arm64)),
        "macos-x86_64" => Some((HostPlatform::MacOS, HostArchitecture::X86_64)),
        "windows-x86_64" => Some((HostPlatform::Windows, HostArchitecture::X86_64)),
        _ => {
            let (platform, architecture) = value.strip_prefix("unknown:")?.split_once(':')?;
            Some((
                HostPlatform::Unknown(platform.to_string()),
                HostArchitecture::Unknown(architecture.to_string()),
            ))
        }
    }
}

fn evaluate_doctor_report(context: &DoctorContext) -> DoctorReport {
    let mut report = DoctorReport {
        host_platform: context.host_platform.clone(),
        host_architecture: context.host_architecture.clone(),
        target_triple: context.target_triple.clone(),
        overall_status: CapabilityStatus::Supported,
        macho_analysis: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        fixture_rebuild: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        fixture_drift_check: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        bench_compile: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        bench_runtime: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        benchmark: CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
        tools: context.tools.clone(),
        issues: Vec::new(),
    };
    for capability in doctor_non_summary_capabilities() {
        *report.capability_mut(capability) = evaluate_primary_capability(context, capability);
    }
    report.benchmark = summarize_capability_reports(&doctor_summary_reports(
        &report,
        CompatibilityCapability::Benchmark,
    ));
    let non_summary_reports = doctor_non_summary_reports(&report);
    let overall_status = non_summary_reports
        .iter()
        .map(|report| report.status)
        .max_by_key(|status| capability_status_severity(*status))
        .unwrap_or(CapabilityStatus::Supported);
    let issues = unique_issues_from_reports(&non_summary_reports);
    report.issues = issues;
    report.overall_status = overall_status;
    report
}

fn evaluate_primary_capability(
    context: &DoctorContext,
    capability: CompatibilityCapability,
) -> CapabilityReport {
    let policy = CompatibilityPolicy::capability(capability);
    let host_status = CompatibilityPolicy::expected_status_for_host_rule(
        policy.host_rule,
        &context.host_platform,
        &context.host_architecture,
    );
    match capability {
        CompatibilityCapability::MachoAnalysis => report_for_host_rule(
            host_status,
            "host_not_ci_verified",
            "Mach-O analysis should still work, but this host pair is outside the current CI matrix.",
        ),
        CompatibilityCapability::FixtureRebuild => evaluate_fixture_rebuild_capability(context),
        CompatibilityCapability::FixtureDriftCheck => {
            evaluate_fixture_drift_check_capability(context, policy.required_tools_any)
        }
        CompatibilityCapability::BenchCompile => report_for_host_rule(
            host_status,
            "host_not_ci_verified_for_bench_compile",
            "Bench compile smoke is not CI-verified for this host pair.",
        ),
        CompatibilityCapability::BenchRuntime => report_for_host_rule(
            host_status,
            "throughput_smoke_linux_arm64_only",
            "Throughput smoke runs only on linux arm64; other hosts support benchmark compile smoke.",
        ),
        CompatibilityCapability::Benchmark => CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        },
    }
}

fn report_for_host_rule(
    status: CapabilityStatus,
    degraded_code: &'static str,
    degraded_message: &'static str,
) -> CapabilityReport {
    let reasons = if status == CapabilityStatus::SupportedWithDegradedFeatures {
        vec![compatibility_issue(degraded_code, degraded_message)]
    } else {
        Vec::new()
    };
    CapabilityReport { status, reasons }
}

fn evaluate_fixture_rebuild_capability(context: &DoctorContext) -> CapabilityReport {
    let policy = CompatibilityPolicy::capability(CompatibilityCapability::FixtureRebuild);
    let host_status = CompatibilityPolicy::expected_status_for_host_rule(
        policy.host_rule,
        &context.host_platform,
        &context.host_architecture,
    );
    if host_status == CapabilityStatus::Unsupported {
        return CapabilityReport {
            status: CapabilityStatus::Unsupported,
            reasons: vec![compatibility_issue(
                "fixture_rebuild_macos_only",
                "Fixture rebuild requires macOS and Xcode tooling.",
            )],
        };
    }

    let mut reasons = Vec::new();
    for requirement in policy.required_tools_all {
        push_tool_requirement_from_policy(&mut reasons, context, *requirement);
    }
    let status = if reasons.is_empty() {
        CapabilityStatus::Supported
    } else {
        CapabilityStatus::Unsupported
    };
    CapabilityReport { status, reasons }
}

fn evaluate_fixture_drift_check_capability(
    context: &DoctorContext,
    required_tools_any: &[CompatibilityToolRequirement],
) -> CapabilityReport {
    let any_usable = required_tools_any
        .iter()
        .any(|requirement| tool_requirement_usable(context, *requirement));
    if any_usable {
        CapabilityReport {
            status: CapabilityStatus::Supported,
            reasons: Vec::new(),
        }
    } else {
        CapabilityReport {
            status: CapabilityStatus::Unsupported,
            reasons: vec![compatibility_issue(
                "missing_usable_hash_tool",
                "No usable hash tool detected; fixture drift check expects sha256sum, shasum, or openssl.",
            )],
        }
    }
}

fn push_tool_requirement(
    reasons: &mut Vec<CompatibilityIssue>,
    status: &ToolStatus,
    tool_name: &'static str,
    missing_code: &'static str,
    unusable_code: &'static str,
) {
    if !status.detected {
        reasons.push(compatibility_issue(
            missing_code,
            format!("{tool_name} was not found on PATH."),
        ));
    } else if !status.usable {
        reasons.push(compatibility_issue(
            unusable_code,
            format!("{tool_name} was detected but did not pass usability probe."),
        ));
    }
}

fn push_tool_requirement_from_policy(
    reasons: &mut Vec<CompatibilityIssue>,
    context: &DoctorContext,
    requirement: CompatibilityToolRequirement,
) {
    match requirement {
        CompatibilityToolRequirement::Xcrun => push_tool_requirement(
            reasons,
            &context.tools.xcrun,
            "xcrun",
            "missing_xcrun",
            "unusable_xcrun",
        ),
        CompatibilityToolRequirement::XcrunSdkPathProbe => {
            if !context.tools.sdk_path_probe.usable {
                reasons.push(compatibility_issue(
                    "xcrun_sdk_path_probe_failed",
                    "xcrun --show-sdk-path failed.",
                ));
            }
        }
        CompatibilityToolRequirement::Clang => push_tool_requirement(
            reasons,
            &context.tools.clang,
            "clang",
            "missing_clang",
            "unusable_clang",
        ),
        CompatibilityToolRequirement::Strip => push_tool_requirement(
            reasons,
            &context.tools.strip,
            "strip",
            "missing_strip",
            "unusable_strip",
        ),
        CompatibilityToolRequirement::Python3 => push_tool_requirement(
            reasons,
            &context.tools.python3,
            "python3",
            "missing_python3",
            "unusable_python3",
        ),
        CompatibilityToolRequirement::Nm => push_tool_requirement(
            reasons,
            &context.tools.nm,
            "nm",
            "missing_nm",
            "unusable_nm",
        ),
        CompatibilityToolRequirement::Sha256sum
        | CompatibilityToolRequirement::Shasum
        | CompatibilityToolRequirement::Openssl => {}
    }
}

fn tool_requirement_usable(
    context: &DoctorContext,
    requirement: CompatibilityToolRequirement,
) -> bool {
    match requirement {
        CompatibilityToolRequirement::Xcrun => context.tools.xcrun.usable,
        CompatibilityToolRequirement::XcrunSdkPathProbe => context.tools.sdk_path_probe.usable,
        CompatibilityToolRequirement::Clang => context.tools.clang.usable,
        CompatibilityToolRequirement::Strip => context.tools.strip.usable,
        CompatibilityToolRequirement::Python3 => context.tools.python3.usable,
        CompatibilityToolRequirement::Nm => context.tools.nm.usable,
        CompatibilityToolRequirement::Sha256sum => context.tools.hash_tools.sha256sum.usable,
        CompatibilityToolRequirement::Shasum => context.tools.hash_tools.shasum.usable,
        CompatibilityToolRequirement::Openssl => context.tools.hash_tools.openssl.usable,
    }
}

fn summarize_capability_reports(reports: &[&CapabilityReport]) -> CapabilityReport {
    let status = reports
        .iter()
        .map(|report| report.status)
        .max_by_key(|status| capability_status_severity(*status))
        .unwrap_or(CapabilityStatus::Supported);
    CapabilityReport {
        status,
        reasons: unique_issues_from_reports(reports),
    }
}

fn unique_issues_from_reports(reports: &[&CapabilityReport]) -> Vec<CompatibilityIssue> {
    let mut issues = Vec::new();
    for report in reports {
        for issue in &report.reasons {
            if !issues.iter().any(|item: &CompatibilityIssue| {
                item.code == issue.code && item.message == issue.message
            }) {
                issues.push(issue.clone());
            }
        }
    }
    issues
}

fn detect_doctor_tools() -> DoctorTools {
    let xcrun = probe_executable_tool_with_probe("xcrun", &["--version"]);
    let clang = probe_tool_via_xcrun_or_path(&xcrun, "clang");
    let strip = probe_tool_via_xcrun_or_path(&xcrun, "strip");
    let python3 = probe_python3();
    let nm = probe_executable_tool_with_probe("nm", &["--version"]);
    let sdk_path_probe = probe_xcrun_sdk_path(&xcrun);
    let hash_tools = HashToolStatus {
        sha256sum: probe_hash_tool("sha256sum"),
        shasum: probe_hash_tool("shasum"),
        openssl: probe_hash_tool("openssl"),
    };
    let selected_hash_tool = select_hash_tool(&hash_tools).map(ToString::to_string);

    DoctorTools {
        xcrun,
        clang,
        strip,
        python3,
        nm,
        sdk_path_probe,
        hash_tools,
        selected_hash_tool,
    }
}

fn probe_executable_tool_with_probe(name: &str, probe_args: &[&str]) -> ToolStatus {
    let path = find_command_path(name).map(|value| value.to_string_lossy().to_string());
    let detected = path.is_some();
    let usable = path
        .as_deref()
        .is_some_and(|path| tool_probe_usable(name, path, probe_args));
    ToolStatus {
        detected,
        usable,
        path,
    }
}

fn probe_python3() -> ToolStatus {
    let path = find_command_path("python3").map(|value| value.to_string_lossy().to_string());
    let detected = path.is_some();
    let usable = path
        .as_deref()
        .is_some_and(|path| command_is_invocable(path, &["-c", "import sys; sys.exit(0)"]));
    ToolStatus {
        detected,
        usable,
        path,
    }
}

fn probe_tool_via_xcrun_or_path(xcrun: &ToolStatus, name: &str) -> ToolStatus {
    if xcrun.usable {
        if let Some(xcrun_path) = xcrun.path.as_deref() {
            if let Some(path) = probe_xcrun_find(xcrun_path, name) {
                return ToolStatus {
                    detected: true,
                    usable: tool_probe_usable(name, &path, &["--version"]),
                    path: Some(path),
                };
            }
        }
    }
    probe_executable_tool_with_probe(name, &["--version"])
}

fn probe_xcrun_find(xcrun_path: &str, tool: &str) -> Option<String> {
    let output = std::process::Command::new(xcrun_path)
        .args(["--find", tool])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if resolved.is_empty() {
        None
    } else {
        Some(resolved)
    }
}

fn probe_xcrun_sdk_path(xcrun: &ToolStatus) -> ToolStatus {
    if !xcrun.usable {
        return ToolStatus {
            detected: false,
            usable: false,
            path: None,
        };
    }
    let Some(xcrun_path) = xcrun.path.as_deref() else {
        return ToolStatus {
            detected: true,
            usable: false,
            path: None,
        };
    };
    let output = std::process::Command::new(xcrun_path)
        .args(["--show-sdk-path"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                ToolStatus {
                    detected: true,
                    usable: false,
                    path: None,
                }
            } else {
                let usable = Path::new(&path).exists();
                ToolStatus {
                    detected: true,
                    usable,
                    path: Some(path),
                }
            }
        }
        _ => ToolStatus {
            detected: true,
            usable: false,
            path: None,
        },
    }
}

fn probe_hash_tool(name: &str) -> ToolStatus {
    let path = find_command_path(name).map(|value| value.to_string_lossy().to_string());
    let detected = path.is_some();
    let usable = path
        .as_deref()
        .is_some_and(|path| is_executable_path(Path::new(path)) && run_hash_probe(path, name));
    ToolStatus {
        detected,
        usable,
        path,
    }
}

fn tool_probe_usable(name: &str, command: &str, probe_args: &[&str]) -> bool {
    if !is_executable_path(Path::new(command)) {
        return false;
    }
    if command_is_invocable(command, probe_args) {
        return true;
    }
    name == "strip" && command_can_start(command, probe_args)
}

fn command_is_invocable(command: &str, args: &[&str]) -> bool {
    std::process::Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn command_can_start(command: &str, args: &[&str]) -> bool {
    std::process::Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn is_executable_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        match fs::metadata(path) {
            Ok(metadata) => metadata.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn run_hash_probe(command: &str, name: &str) -> bool {
    let null_path = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let mut process = std::process::Command::new(command);
    match name {
        "sha256sum" => {
            process.arg(null_path);
        }
        "shasum" => {
            process.args(["-a", "256", null_path]);
        }
        "openssl" => {
            process.args(["dgst", "-sha256", null_path]);
        }
        _ => return false,
    }
    match process.output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn select_hash_tool(hash_tools: &HashToolStatus) -> Option<&'static str> {
    if hash_tools.sha256sum.usable {
        Some("sha256sum")
    } else if hash_tools.shasum.usable {
        Some("shasum")
    } else if hash_tools.openssl.usable {
        Some("openssl")
    } else {
        None
    }
}

fn find_command_path(name: &str) -> Option<std::path::PathBuf> {
    if name.is_empty() {
        return None;
    }
    if Path::new(name).components().count() > 1 {
        return Path::new(name)
            .is_file()
            .then(|| Path::new(name).to_path_buf());
    }
    let paths = env::var_os("PATH")?;
    #[cfg(windows)]
    let path_exts = env::var_os("PATHEXT")
        .map(|exts| {
            exts.to_string_lossy()
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![".EXE".to_string(), ".BAT".to_string(), ".CMD".to_string()]);
    for dir in env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            for ext in &path_exts {
                let suffix = ext.trim_start_matches('.');
                let with_ext = dir.join(format!("{name}.{suffix}"));
                if with_ext.is_file() {
                    return Some(with_ext);
                }
            }
        }
    }
    None
}

fn compatibility_issue(code: &'static str, message: impl Into<String>) -> CompatibilityIssue {
    CompatibilityIssue::new(code, message)
}

fn capability_status_severity(status: CapabilityStatus) -> u8 {
    match status {
        CapabilityStatus::Supported => 0,
        CapabilityStatus::SupportedWithDegradedFeatures => 1,
        CapabilityStatus::Unsupported => 2,
    }
}

fn evaluate_doctor_check(report: &DoctorReport, request: DoctorCheckRequest) -> DoctorCheckOutcome {
    let required_status = match request.require_status {
        DoctorRequireStatus::Supported => CapabilityStatus::Supported,
        DoctorRequireStatus::SupportedWithDegradedFeatures => {
            CapabilityStatus::SupportedWithDegradedFeatures
        }
    };
    let required_severity = capability_status_severity(required_status);
    let passes = match request.target {
        DoctorCheckTarget::All => doctor_non_summary_capabilities().all(|capability| {
            capability_status_severity(report.capability(capability).status) <= required_severity
        }),
        DoctorCheckTarget::MachoAnalysis => {
            capability_status_severity(report.macho_analysis.status) <= required_severity
        }
        DoctorCheckTarget::FixtureRebuild => {
            capability_status_severity(report.fixture_rebuild.status) <= required_severity
        }
        DoctorCheckTarget::FixtureDriftCheck => {
            capability_status_severity(report.fixture_drift_check.status) <= required_severity
        }
        DoctorCheckTarget::BenchCompile => {
            capability_status_severity(report.bench_compile.status) <= required_severity
        }
        DoctorCheckTarget::BenchRuntime => {
            capability_status_severity(report.bench_runtime.status) <= required_severity
        }
    };
    if passes {
        DoctorCheckOutcome::Passed
    } else {
        DoctorCheckOutcome::Failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing_tool() -> ToolStatus {
        ToolStatus {
            detected: false,
            usable: false,
            path: None,
        }
    }

    fn usable_tool(path: &str) -> ToolStatus {
        ToolStatus {
            detected: true,
            usable: true,
            path: Some(path.to_string()),
        }
    }

    fn doctor_tools(selected_hash_tool: Option<&str>) -> DoctorTools {
        let sha256sum = if selected_hash_tool == Some("sha256sum") {
            usable_tool("/usr/bin/sha256sum")
        } else {
            missing_tool()
        };
        let shasum = if selected_hash_tool == Some("shasum") {
            usable_tool("/usr/bin/shasum")
        } else {
            missing_tool()
        };
        let openssl = if selected_hash_tool == Some("openssl") {
            usable_tool("/usr/bin/openssl")
        } else {
            missing_tool()
        };
        DoctorTools {
            xcrun: usable_tool("/usr/bin/xcrun"),
            clang: usable_tool("/usr/bin/clang"),
            strip: usable_tool("/usr/bin/strip"),
            python3: usable_tool("/usr/bin/python3"),
            nm: usable_tool("/usr/bin/nm"),
            sdk_path_probe: usable_tool("/Applications/Xcode.app/SDKs/MacOSX.sdk"),
            hash_tools: HashToolStatus {
                sha256sum,
                shasum,
                openssl,
            },
            selected_hash_tool: selected_hash_tool.map(ToString::to_string),
        }
    }

    fn context(platform: HostPlatform, architecture: HostArchitecture) -> DoctorContext {
        DoctorContext {
            host_platform: platform,
            host_architecture: architecture,
            target_triple: Some("test-target".to_string()),
            tools: doctor_tools(Some("sha256sum")),
        }
    }

    #[test]
    fn evaluator_reports_full_support_on_linux_arm64() {
        let report = evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::Arm64));
        assert_eq!(report.macho_analysis.status, CapabilityStatus::Supported);
        assert_eq!(
            report.fixture_drift_check.status,
            CapabilityStatus::Supported
        );
        assert_eq!(report.bench_compile.status, CapabilityStatus::Supported);
        assert_eq!(report.bench_runtime.status, CapabilityStatus::Supported);
        assert_eq!(report.benchmark.status, CapabilityStatus::Supported);
    }

    #[test]
    fn evaluator_reports_degraded_runtime_on_linux_x86_64() {
        let report =
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::X86_64));
        assert_eq!(report.bench_compile.status, CapabilityStatus::Supported);
        assert_eq!(
            report.bench_runtime.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert_eq!(
            report.benchmark.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert!(
            report
                .bench_runtime
                .reasons
                .iter()
                .any(|issue| issue.code == "throughput_smoke_linux_arm64_only")
        );
    }

    #[test]
    fn evaluator_reports_graceful_degraded_host_on_windows() {
        let report =
            evaluate_doctor_report(&context(HostPlatform::Windows, HostArchitecture::X86_64));
        assert_eq!(
            report.macho_analysis.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert_eq!(
            report.bench_compile.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert_eq!(report.fixture_rebuild.status, CapabilityStatus::Unsupported);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "host_not_ci_verified")
        );
    }

    #[test]
    fn evaluator_reports_graceful_degraded_host_on_unknown_platform() {
        let report = evaluate_doctor_report(&context(
            HostPlatform::Unknown("solaris".to_string()),
            HostArchitecture::Unknown("sparc64".to_string()),
        ));
        assert_eq!(
            report.macho_analysis.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert_eq!(
            report.bench_compile.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert_eq!(
            report.bench_runtime.status,
            CapabilityStatus::SupportedWithDegradedFeatures
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "host_not_ci_verified")
        );
    }

    #[test]
    fn evaluator_requires_sdk_probe_for_fixture_rebuild() {
        let mut context = context(HostPlatform::MacOS, HostArchitecture::Arm64);
        context.tools.sdk_path_probe = missing_tool();
        let report = evaluate_doctor_report(&context);
        assert_eq!(report.fixture_rebuild.status, CapabilityStatus::Unsupported);
        assert!(
            report
                .fixture_rebuild
                .reasons
                .iter()
                .any(|issue| issue.code == "xcrun_sdk_path_probe_failed")
        );
    }

    #[test]
    fn evaluator_uses_openssl_only_hash_fallback() {
        let mut context = context(HostPlatform::Linux, HostArchitecture::Arm64);
        context.tools = doctor_tools(Some("openssl"));
        let report = evaluate_doctor_report(&context);
        assert_eq!(
            report.fixture_drift_check.status,
            CapabilityStatus::Supported
        );
        assert_eq!(report.tools.selected_hash_tool.as_deref(), Some("openssl"));
        assert!(report.tools.hash_tools.openssl.usable);
        assert!(!report.tools.hash_tools.sha256sum.usable);
    }

    #[test]
    fn doctor_check_targets_split_bench_capabilities() {
        let report =
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::X86_64));
        assert_eq!(
            evaluate_doctor_check(
                &report,
                DoctorCheckRequest {
                    target: DoctorCheckTarget::BenchCompile,
                    require_status: DoctorRequireStatus::Supported,
                }
            ),
            DoctorCheckOutcome::Passed
        );
        assert_eq!(
            evaluate_doctor_check(
                &report,
                DoctorCheckRequest {
                    target: DoctorCheckTarget::BenchRuntime,
                    require_status: DoctorRequireStatus::Supported,
                }
            ),
            DoctorCheckOutcome::Failed
        );
        assert_eq!(
            evaluate_doctor_check(
                &report,
                DoctorCheckRequest {
                    target: DoctorCheckTarget::BenchRuntime,
                    require_status: DoctorRequireStatus::SupportedWithDegradedFeatures,
                }
            ),
            DoctorCheckOutcome::Passed
        );
    }

    #[test]
    fn parse_doctor_test_scenario_supports_known_values() {
        assert_eq!(
            parse_doctor_test_scenario("linux-arm64"),
            Some((HostPlatform::Linux, HostArchitecture::Arm64))
        );
        assert_eq!(
            parse_doctor_test_scenario("linux-x86_64"),
            Some((HostPlatform::Linux, HostArchitecture::X86_64))
        );
        assert_eq!(
            parse_doctor_test_scenario("macos-arm64"),
            Some((HostPlatform::MacOS, HostArchitecture::Arm64))
        );
        assert_eq!(
            parse_doctor_test_scenario("windows-x86_64"),
            Some((HostPlatform::Windows, HostArchitecture::X86_64))
        );
    }

    #[test]
    fn parse_doctor_test_scenario_supports_unknown_tuple() {
        assert_eq!(
            parse_doctor_test_scenario("unknown:solaris:sparc64"),
            Some((
                HostPlatform::Unknown("solaris".to_string()),
                HostArchitecture::Unknown("sparc64".to_string()),
            ))
        );
    }

    #[test]
    fn parse_doctor_test_scenario_rejects_invalid_values() {
        assert_eq!(parse_doctor_test_scenario("unknown"), None);
        assert_eq!(parse_doctor_test_scenario("unknown:solaris"), None);
        assert_eq!(parse_doctor_test_scenario("linux"), None);
    }

    #[test]
    fn benchmark_summary_matches_split_max_severity() {
        for report in [
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::Arm64)),
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::X86_64)),
            evaluate_doctor_report(&context(HostPlatform::MacOS, HostArchitecture::Arm64)),
            evaluate_doctor_report(&context(HostPlatform::Windows, HostArchitecture::X86_64)),
        ] {
            let split_max = [report.bench_compile.status, report.bench_runtime.status]
                .iter()
                .max_by_key(|status| capability_status_severity(**status))
                .copied()
                .expect("split capability statuses");
            assert_eq!(report.benchmark.status, split_max);
        }
    }

    #[test]
    fn overall_status_matches_non_summary_max_severity() {
        for report in [
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::Arm64)),
            evaluate_doctor_report(&context(HostPlatform::Linux, HostArchitecture::X86_64)),
            evaluate_doctor_report(&context(HostPlatform::MacOS, HostArchitecture::Arm64)),
            evaluate_doctor_report(&context(HostPlatform::Windows, HostArchitecture::X86_64)),
        ] {
            let expected = doctor_non_summary_reports(&report)
                .iter()
                .map(|capability| capability.status)
                .max_by_key(|status| capability_status_severity(*status))
                .expect("non-summary statuses");
            assert_eq!(report.overall_status, expected);
        }
    }

    #[test]
    fn issues_are_deduped_union_of_non_summary_reasons() {
        let mut context = context(HostPlatform::MacOS, HostArchitecture::Arm64);
        context.tools.python3 = ToolStatus {
            detected: true,
            usable: false,
            path: Some("/usr/bin/python3".to_string()),
        };
        context.tools.nm = ToolStatus {
            detected: true,
            usable: false,
            path: Some("/usr/bin/nm".to_string()),
        };
        context.tools.sdk_path_probe = ToolStatus {
            detected: true,
            usable: false,
            path: None,
        };
        context.tools.selected_hash_tool = None;
        context.tools.hash_tools = HashToolStatus {
            sha256sum: missing_tool(),
            shasum: missing_tool(),
            openssl: missing_tool(),
        };
        let report = evaluate_doctor_report(&context);
        let expected = unique_issues_from_reports(&doctor_non_summary_reports(&report));
        assert_eq!(report.issues, expected);
    }
}

fn emit_json_response<T: JsonDto>(command: &str, dto: &T, output: &OutputSettings) {
    let envelope = JsonEnvelope { command, data: dto };
    println!("{}", envelope.to_json_value().render(output.pretty));
}

fn emit_json_response_to_stderr<T: JsonDto>(command: &str, dto: &T, output: &OutputSettings) {
    let envelope = JsonEnvelope { command, data: dto };
    let rendered = envelope.to_json_value().render(output.pretty);
    let _ = writeln!(io::stderr(), "{rendered}");
}

fn opt_u64(value: Option<u64>) -> JsonValue {
    value.map(u64_num).unwrap_or(JsonValue::Null)
}

fn usize_num(value: usize) -> JsonValue {
    JsonValue::Number(value.to_string())
}

fn u64_num(value: u64) -> JsonValue {
    JsonValue::Number(value.to_string())
}

fn i64_num(value: i64) -> JsonValue {
    JsonValue::Number(value.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn render(&self, pretty: bool) -> String {
        let mut out = String::new();
        self.write_to(&mut out, pretty, 0);
        out
    }

    fn write_to(&self, out: &mut String, pretty: bool, indent: usize) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Number(number) => out.push_str(number),
            Self::String(value) => write_json_string(out, value),
            Self::Array(values) => {
                out.push('[');
                if pretty && !values.is_empty() {
                    out.push('\n');
                }
                for (index, value) in values.iter().enumerate() {
                    if pretty {
                        push_indent(out, indent + 1);
                    }
                    value.write_to(out, pretty, indent + 1);
                    if index + 1 < values.len() {
                        out.push(',');
                    }
                    if pretty {
                        out.push('\n');
                    }
                }
                if pretty && !values.is_empty() {
                    push_indent(out, indent);
                }
                out.push(']');
            }
            Self::Object(fields) => {
                out.push('{');
                if pretty && !fields.is_empty() {
                    out.push('\n');
                }
                for (index, (key, value)) in fields.iter().enumerate() {
                    if pretty {
                        push_indent(out, indent + 1);
                    }
                    write_json_string(out, key);
                    out.push(':');
                    if pretty {
                        out.push(' ');
                    }
                    value.write_to(out, pretty, indent + 1);
                    if index + 1 < fields.len() {
                        out.push(',');
                    }
                    if pretty {
                        out.push('\n');
                    }
                }
                if pretty && !fields.is_empty() {
                    push_indent(out, indent);
                }
                out.push('}');
            }
        }
    }
}

fn push_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_json_string(out: &mut String, input: &str) {
    out.push('"');
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}
