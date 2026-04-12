use damsel_core::BinaryImage;

pub(crate) fn print_info(image: &BinaryImage) {
    println!("path: {}", image.path.display());
    println!("format: {:?}", image.format);
    println!("architecture: {}", image.architecture);
    println!("endianness: {:?}", image.endianness);
    if let Some(entry) = image.entry_point {
        println!("entry: {entry:#x}");
    }
    if let Some(platform) = &image.platform {
        println!("platform: {platform}");
    }
    println!(
        "slice: offset={:#x} size={:#x} universal={} subtype={:#x}",
        image.slice.offset, image.slice.size, image.slice.is_universal, image.slice.cpu_subtype
    );
    println!(
        "counts: segments={} sections={} symbols={} imports={} relocations={}",
        image.segments.len(),
        image.sections.len(),
        image.symbols.len(),
        image.imports.len(),
        image.relocations.len()
    );
    println!(
        "objc: classes={} selectors={} methods={}",
        image.objc.class_names.len(),
        image.objc.selector_names.len(),
        image.objc.method_names.len()
    );
    println!(
        "dyld: dylibs={} rpaths={} exports={} function_starts={} rebases={} binds={} chained_fixups={}",
        image.dyld.imported_dylibs.len(),
        image.dyld.rpaths.len(),
        image.dyld.exported_symbols.len(),
        image.dyld.function_starts.len(),
        image.dyld.has_rebases,
        image.dyld.has_binds,
        image.dyld.has_chained_fixups
    );
    if !image.dyld.imported_dylibs.is_empty() {
        println!("dylibs:");
        for dylib in &image.dyld.imported_dylibs {
            println!("  {dylib}");
        }
    }
}

pub(crate) fn print_sections(image: &BinaryImage) {
    for section in &image.sections {
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

pub(crate) fn print_symbols(image: &BinaryImage) {
    for symbol in &image.symbols {
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

pub(crate) fn print_imports(image: &BinaryImage) {
    for import in &image.imports {
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

pub(crate) fn print_objc(image: &BinaryImage) {
    if let Some(flags) = image.objc.image_info_flags {
        println!("image_info_flags: {flags:#x}");
    }
    println!("classes:");
    for class_name in &image.objc.class_names {
        println!("  {class_name}");
    }
    println!("selectors:");
    for selector in &image.objc.selector_names {
        println!("  {selector}");
    }
    println!("methods:");
    for method in &image.objc.method_names {
        println!("  {method}");
    }
}

pub(crate) fn print_disassembly(result: &damsel_core::DisassemblyResult) {
    println!(
        "target: {} start={:#x} bytes={:#x}",
        result.target, result.start_address, result.bytes_len
    );
    for instruction in &result.instructions {
        let rendered = instruction.render();
        let annotations = instruction
            .annotations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if annotations.is_empty() {
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
                annotations.join(" | ")
            );
        }
    }
}
