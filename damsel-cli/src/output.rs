use damsel_core::{
    BinaryImage, DecodedInstruction, Import, ImportBindingRecord, ObjcCategoryRecord,
    ObjcClassRecord, ObjcPointerKind, ObjcProtocolRecord, Reference, Relocation, Section,
    SliceDescriptor, StubEntry, Symbol,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DyldViewOptions {
    pub show_dylibs: bool,
    pub show_rpaths: bool,
    pub show_exports: bool,
    pub show_function_starts: bool,
    pub show_bindings: bool,
    pub show_stubs: bool,
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DisassemblyRenderOptions {
    pub include_annotations: bool,
    pub include_references: bool,
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

pub(crate) fn print_objc(image: &BinaryImage, output: &OutputSettings) {
    let objc = image.objc();
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
            if !objc.classes.is_empty() {
                println!("class_records:");
                for record in &objc.classes {
                    println!(
                        "  {:#x} name={} superclass={} ro={} methods={} properties={} protocols={}",
                        record.class_pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record
                            .superclass_name
                            .clone()
                            .or_else(|| record.superclass_pointer.map(|value| format!("{value:#x}")))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .ro_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        record
                            .method_list_pointer
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
                    );
                }
            }
            if !objc.protocols.is_empty() {
                println!("protocol_records:");
                for record in &objc.protocols {
                    println!(
                        "  {:#x} {}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                    );
                }
            }
            if !objc.categories.is_empty() {
                println!("category_records:");
                for record in &objc.categories {
                    println!(
                        "  {:#x} name={} class={} class_ptr={}",
                        record.pointer,
                        record.name.as_deref().unwrap_or("-"),
                        record.class_name.as_deref().unwrap_or("-"),
                        record
                            .class_pointer
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                    );
                }
            }
            if !objc.pointer_refs.is_empty() {
                println!("pointer_refs:");
                for entry in &objc.pointer_refs {
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
        OutputFormat::Json => emit_json_response("objc", &ObjcJsonDto { image }, output),
    }
}

pub(crate) fn print_dyld(image: &BinaryImage, view: DyldViewOptions, output: &OutputSettings) {
    let dyld = image.dyld();
    match output.format {
        OutputFormat::Text => {
            println!(
                "dyld: dylibs={} rpaths={} exports={} function_starts={} bindings={} stubs={} rebases={} binds={} chained_fixups={}",
                dyld.imported_dylibs.len(),
                dyld.rpaths.len(),
                dyld.exported_symbols.len(),
                dyld.function_starts.len(),
                dyld.import_bindings.len(),
                dyld.stubs.len(),
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
                for export in &dyld.exported_symbols {
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
                for binding in &dyld.import_bindings {
                    println!(
                        "  {:>#18} {:>#18} {:<30} {:<24} addend={} source={:?} weak={}",
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
                        binding.source,
                        binding.is_weak
                    );
                }
            }
            if view.show_stubs {
                println!("stubs:");
                for stub in &dyld.stubs {
                    println!(
                        "  {:#18x} section={} ptr={} {}:{} source={:?}",
                        stub.stub_address,
                        stub.section.as_deref().unwrap_or("-"),
                        stub.pointer_address
                            .map(|value| format!("{value:#x}"))
                            .unwrap_or_else(|| "-".to_string()),
                        stub.dylib.as_deref().unwrap_or("-"),
                        stub.name.as_deref().unwrap_or("-"),
                        stub.source
                    );
                }
            }
        }
        OutputFormat::Json => emit_json_response("dyld", &DyldJsonDto { image, view }, output),
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
        "dyld: dylibs={} rpaths={} exports={} function_starts={} bindings={} stubs={} rebases={} binds={} chained_fixups={}",
        dyld.imported_dylibs.len(),
        dyld.rpaths.len(),
        dyld.exported_symbols.len(),
        dyld.function_starts.len(),
        dyld.import_bindings.len(),
        dyld.stubs.len(),
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
}

impl JsonDto for ObjcJsonDto<'_> {
    fn to_json_value(&self) -> JsonValue {
        let objc = self.image.objc();
        JsonValue::Object(vec![
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
                JsonValue::Array(
                    objc.pointer_refs
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
                ),
            ),
            (
                "classes".to_string(),
                JsonValue::Array(objc.classes.iter().map(objc_class_record_json).collect()),
            ),
            (
                "protocols".to_string(),
                JsonValue::Array(
                    objc.protocols
                        .iter()
                        .map(objc_protocol_record_json)
                        .collect(),
                ),
            ),
            (
                "categories".to_string(),
                JsonValue::Array(
                    objc.categories
                        .iter()
                        .map(objc_category_record_json)
                        .collect(),
                ),
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
        let view = self.view;
        let mut fields = vec![
            ("has_rebases".to_string(), JsonValue::Bool(dyld.has_rebases)),
            ("has_binds".to_string(), JsonValue::Bool(dyld.has_binds)),
            (
                "has_chained_fixups".to_string(),
                JsonValue::Bool(dyld.has_chained_fixups),
            ),
        ];
        if view.show_dylibs {
            fields.push((
                "dylibs".to_string(),
                JsonValue::Array(
                    dyld
                        .imported_dylibs
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ));
        }
        if view.show_rpaths {
            fields.push((
                "rpaths".to_string(),
                JsonValue::Array(
                    dyld
                        .rpaths
                        .iter()
                        .cloned()
                        .map(JsonValue::String)
                        .collect(),
                ),
            ));
        }
        if view.show_exports {
            fields.push((
                "exports".to_string(),
                JsonValue::Array(
                    dyld
                        .exported_symbols
                        .iter()
                        .map(|export| {
                            JsonValue::Object(vec![
                                ("name".to_string(), JsonValue::String(export.name.clone())),
                                (
                                    "address".to_string(),
                                    export.address.map(u64_num).unwrap_or(JsonValue::Null),
                                ),
                                (
                                    "flags".to_string(),
                                    JsonValue::String(export.flags.to_string()),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        if view.show_function_starts {
            fields.push((
                "function_starts".to_string(),
                JsonValue::Array(
                    dyld
                        .function_starts
                        .iter()
                        .copied()
                        .map(u64_num)
                        .collect(),
                ),
            ));
        }
        if view.show_bindings {
            fields.push((
                "import_bindings".to_string(),
                JsonValue::Array(dyld.import_bindings.iter().map(import_binding_json).collect()),
            ));
        }
        if view.show_stubs {
            fields.push((
                "stubs".to_string(),
                JsonValue::Array(dyld.stubs.iter().map(stub_entry_json).collect()),
            ));
        }
        JsonValue::Object(fields)
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
    ])
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
                                    .map(reference_to_text)
                                    .map(JsonValue::String)
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
                                    .map(ToString::to_string)
                                    .map(JsonValue::String)
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
            source,
            is_weak,
        } => format!(
            "import-binding {}:{} addr={} off={} addend={} source={:?} weak={}",
            dylib,
            name,
            address
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            offset
                .map(|value| format!("{value:#x}"))
                .unwrap_or_else(|| "-".to_string()),
            addend,
            source,
            is_weak
        ),
        Reference::Stub {
            stub_address,
            section: _,
            pointer_address,
            dylib,
            name,
            source,
        } => format!(
            "stub {stub_address:#x} ptr={} {}:{} source={:?}",
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
