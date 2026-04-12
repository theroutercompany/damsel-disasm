use damsel_core::{
    Annotation, BinaryImage, DecodedInstruction, ExportKind, Import, ImportBindingRecord,
    ImportBindingSource, ObjcCategoryRecord, ObjcClassRecord, ObjcIvarRecord, ObjcMethodRecord,
    ObjcMethodOwnerKind, ObjcPointerKind, ObjcPointerRef, ObjcPropertyRecord, ObjcProtocolRecord,
    RecoveredValue, Reference, Relocation, Section, SliceDescriptor, StubEntry, StubHelperEntry,
    Symbol,
};
use std::fmt::Write as _;
use std::io::{self, Write as _};

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
            sort: None,
        }
    }
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
                println!("class_records:");
                for record in &filtered.classes {
                    println!(
                        "  {:#x} name={} source={:?} superclass={} metaclass={} ro={} methods={} class_methods={} properties={} ivars={} protocols={}",
                        record.class_pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.name_source,
                        record
                            .superclass_name
                            .clone()
                            .or_else(|| record.superclass_pointer.map(|value| format!("{value:#x}")))
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
                println!("protocol_records:");
                for record in &filtered.protocols {
                    println!(
                        "  {:#x} {} required_inst={} required_class={} optional_inst={} optional_class={} properties={}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.required_instance_methods.len(),
                        record.required_class_methods.len(),
                        record.optional_instance_methods.len(),
                        record.optional_class_methods.len(),
                        record.properties.len(),
                    );
                }
            }
            if !filtered.categories.is_empty() && objc_detail_includes_categories(view.detail) {
                println!("category_records:");
                for record in &filtered.categories {
                    println!(
                        "  {:#x} name={} class={} class_ptr={} methods={} class_methods={} properties={} protocols={}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.class_name.as_deref().unwrap_or("-"),
                        record
                            .class_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record.methods.len(),
                        record.class_methods.len(),
                        record.properties.len(),
                        record.adopted_protocols.len(),
                    );
                }
            }
            if !filtered.methods.is_empty() && objc_detail_includes_methods(view.detail) {
                println!("method_records:");
                for entry in &filtered.methods {
                    println!(
                        "  owner={} kind={} class_method={} selector={} selector_source={:?} impl={} types={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        objc_method_owner_kind_text(entry.record.owner_kind),
                        entry.record.is_class_method,
                        entry.record.selector.as_deref().unwrap_or("-"),
                        entry.record.selector_source,
                        entry.record
                            .implementation
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        entry.record.type_encoding.as_deref().unwrap_or("-"),
                    );
                }
            }
            if !filtered.properties.is_empty() && objc_detail_includes_properties(view.detail) {
                println!("property_records:");
                for entry in &filtered.properties {
                    println!(
                        "  owner={} name={} attrs={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        entry.record.name.as_deref().unwrap_or("-"),
                        entry.record.attributes.as_deref().unwrap_or("-"),
                    );
                }
            }
            if !filtered.ivars.is_empty() && objc_detail_includes_ivars(view.detail) {
                println!("ivar_records:");
                for entry in &filtered.ivars {
                    println!(
                        "  owner={} name={} type={} offset={}",
                        entry.owner_name.as_deref().unwrap_or("-"),
                        entry.record.name.as_deref().unwrap_or("-"),
                        entry.record.type_encoding.as_deref().unwrap_or("-"),
                        entry.record
                            .offset
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                    );
                }
            }
            if !filtered.pointer_refs.is_empty() && matches!(view.detail, ObjcDetail::All) {
                println!("pointer_refs:");
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
                println!("exports:");
                for export in &filtered.exports {
                    println!(
                        "  {:>#18} {:<24} {}",
                        export
                            .address
                            .map(|address| format!("{address:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        export.flags,
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
                println!("import_bindings:");
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
                println!("stubs:");
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
                println!("stub_helpers:");
                for helper in &filtered.stub_helpers {
                    println!(
                        "  {:#18x} stub={} ordinal={} {}:{}",
                        helper.helper_address,
                        helper
                            .target_stub
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
                && view.source_filter.is_none_or(|source| binding.source == source)
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
            }) && view.source_filter.is_none_or(|source| stub.source == source)
        })
        .collect::<Vec<_>>();
    sort_stubs(&mut stubs, view.sort);

    let allowed_helper_addresses = stubs
        .iter()
        .filter_map(|stub| stub.helper_address)
        .collect::<std::collections::BTreeSet<_>>();
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
            }) && (view.name_filter.is_none()
                && view.dylib_filter.is_none()
                && view.source_filter.is_none()
                || allowed_helper_addresses.is_empty()
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
        .filter(|record| objc_class_matches(record, view.owner_filter.as_deref()))
        .collect::<Vec<_>>();
    let protocols = objc
        .protocols
        .iter()
        .filter(|record| objc_protocol_matches(record, view.owner_filter.as_deref()))
        .collect::<Vec<_>>();
    let categories = objc
        .categories
        .iter()
        .filter(|record| objc_category_matches(record, view.owner_filter.as_deref()))
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

    let pointer_refs = if view.owner_filter.is_some() {
        let matched_names = classes
            .iter()
            .filter_map(|record| record.name.as_deref())
            .chain(categories.iter().filter_map(|record| record.class_name.as_deref()))
            .chain(protocols.iter().filter_map(|record| record.name.as_deref()))
            .collect::<std::collections::BTreeSet<_>>();
        objc.pointer_refs
            .iter()
            .filter(|entry| {
                entry.resolved_name
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
    haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
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
        DyldSortKey::Address => exports.sort_by_key(|export| {
            (export.address.unwrap_or_default(), export.name.clone())
        }),
        DyldSortKey::Name => exports.sort_by_key(|export| export.name.clone()),
        DyldSortKey::Dylib | DyldSortKey::Source => {
            exports.sort_by_key(|export| (export.flags.clone(), export.name.clone()))
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
            (binding.name.clone(), binding.dylib.clone(), binding.address.unwrap_or_default())
        }),
        DyldSortKey::Dylib => bindings.sort_by_key(|binding| {
            (binding.dylib.clone(), binding.name.clone(), binding.address.unwrap_or_default())
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

fn objc_class_matches(record: &ObjcClassRecord, owner_filter: Option<&str>) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
            || record
                .superclass_name
                .as_deref()
                .is_some_and(|value| contains_case_insensitive(value, needle))
    })
}

fn objc_protocol_matches(record: &ObjcProtocolRecord, owner_filter: Option<&str>) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
    })
}

fn objc_category_matches(record: &ObjcCategoryRecord, owner_filter: Option<&str>) -> bool {
    owner_filter.is_none_or(|needle| {
        record
            .name
            .as_deref()
            .is_some_and(|value| contains_case_insensitive(value, needle))
            || record
                .class_name
                .as_deref()
                .is_some_and(|value| contains_case_insensitive(value, needle))
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
                    (
                        "class_names".to_string(),
                        usize_num(objc.class_names.len()),
                    ),
                    (
                        "selectors".to_string(),
                        usize_num(objc.selector_names.len()),
                    ),
                    (
                        "methods".to_string(),
                        usize_num(objc.method_names.len()),
                    ),
                    ("pointer_refs".to_string(), usize_num(objc.pointer_refs.len())),
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
                    (
                        "dylibs".to_string(),
                        usize_num(dyld.imported_dylibs.len()),
                    ),
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
                    ("stub_helpers".to_string(), usize_num(dyld.stub_helpers.len())),
                    (
                        "has_rebases".to_string(),
                        JsonValue::Bool(dyld.has_rebases),
                    ),
                    (
                        "has_binds".to_string(),
                        JsonValue::Bool(dyld.has_binds),
                    ),
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
                "image_info_flags".to_string(),
                objc.image_info_flags
                    .map(u64::from)
                    .map(u64_num)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "class_names".to_string(),
                JsonValue::Array(
                    objc
                        .class_names
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "selector_names".to_string(),
                JsonValue::Array(
                    objc
                        .selector_names
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ),
            (
                "method_names".to_string(),
                JsonValue::Array(
                    objc
                        .method_names
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
                                        JsonValue::String(objc_pointer_kind_text(entry.kind).to_string()),
                                    ),
                                    ("table_address".to_string(), u64_num(entry.table_address)),
                                    ("raw_pointer".to_string(), u64_num(entry.raw_pointer)),
                                    (
                                        "resolved_address".to_string(),
                                        entry.resolved_address.map(u64_num).unwrap_or(JsonValue::Null),
                                    ),
                                    (
                                        "resolved_name".to_string(),
                                        entry.resolved_name
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
                    JsonValue::Array(
                        filtered
                            .methods
                            .iter()
                            .map(objc_method_view_json)
                            .collect(),
                    )
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
                    JsonValue::Array(
                        dyld.rpaths
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
                "exports".to_string(),
                if self.view.show_exports {
                    JsonValue::Array(filtered.exports.iter().map(|export| export_json(export)).collect())
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
                    JsonValue::Array(filtered.import_bindings.iter().map(|binding| import_binding_json(binding)).collect())
                } else {
                    JsonValue::Array(Vec::new())
                },
            ),
            (
                "stubs".to_string(),
                if self.view.show_stubs {
                    JsonValue::Array(filtered.stubs.iter().map(|stub| stub_entry_json(stub)).collect())
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
        ("flags".to_string(), JsonValue::String(export.flags.to_string())),
        ("kind".to_string(), export_kind_json(&export.kind)),
    ])
}

fn export_kind_json(kind: &ExportKind) -> JsonValue {
    match kind {
        ExportKind::Regular => JsonValue::Object(vec![(
            "type".to_string(),
            JsonValue::String("regular".to_string()),
        )]),
        ExportKind::Reexport { dylib, symbol } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("reexport".to_string())),
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
            ("type".to_string(), JsonValue::String("resolver".to_string())),
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
        ("dylib".to_string(), JsonValue::String(binding.dylib.clone())),
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
            binding.ordinal.map(|value| u64_num(u64::from(value))).unwrap_or(JsonValue::Null),
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
        (
            "helper_address".to_string(),
            u64_num(helper.helper_address),
        ),
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
            record
                .class_pointer
                .map(u64_num)
                .unwrap_or(JsonValue::Null),
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
            JsonValue::String(match record.owner_kind {
                ObjcMethodOwnerKind::Class => "class",
                ObjcMethodOwnerKind::Metaclass => "metaclass",
                ObjcMethodOwnerKind::Protocol => "protocol",
                ObjcMethodOwnerKind::Category => "category",
            }
            .to_string()),
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
                                instruction
                                    .references
                                    .iter()
                                    .map(reference_json)
                                    .collect(),
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
            ("type".to_string(), JsonValue::String("indirect_call".to_string())),
            ("via".to_string(), JsonValue::String(via.clone())),
        ]),
        Reference::IndirectBranch { via } => JsonValue::Object(vec![
            ("type".to_string(), JsonValue::String("indirect_branch".to_string())),
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
        Reference::Import { name, dylib, address } => JsonValue::Object(vec![
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
            ("type".to_string(), JsonValue::String("relocation".to_string())),
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
            ("element_size".to_string(), u64_num(u64::from(*element_size))),
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
        ("register".to_string(), JsonValue::String(value.register.clone())),
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
