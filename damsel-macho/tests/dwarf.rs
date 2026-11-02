//! DWARF import tests (plan feature 7, slice 3): struct declarations
//! extracted from the -g fixture's debug info.

use damsel_macho::import_structs;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/bin")
        .join(name)
}

fn dsym_path(bundle: &str) -> std::path::PathBuf {
    fixture(bundle)
        .with_extension("dSYM")
        .join("Contents/Resources/DWARF")
        .join(bundle)
}

#[test]
fn extracts_struct_layouts_with_types_and_offsets() {
    // The Apple flow: linked executables drop __DWARF; dsymutil
    // bundles carry it. Import from the bundle's inner Mach-O.
    let structs = import_structs(&dsym_path("arm64-symbolized-dwarf")).expect("dwarf import");
    let by_name = |name: &str| {
        structs
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("missing {name}: {structs:#?}"))
    };

    let point = by_name("Point");
    assert_eq!(point.fields.len(), 2);
    assert_eq!(point.fields[0].field_name, "x");
    assert_eq!(point.fields[0].offset, 0);
    assert_eq!(point.fields[0].type_name, "int64_t");
    assert_eq!(point.fields[1].field_name, "y");
    assert_eq!(point.fields[1].offset, 8);

    let shape = by_name("Shape");
    assert_eq!(shape.fields.len(), 4);
    assert_eq!(shape.fields[0].field_name, "origin");
    assert_eq!(shape.fields[0].offset, 0);
    assert_eq!(shape.fields[0].type_name, "Point");
    assert_eq!(shape.fields[1].field_name, "sides");
    assert_eq!(shape.fields[1].offset, 16);
    assert_eq!(shape.fields[1].type_name, "uint32_t");
    assert_eq!(shape.fields[3].field_name, "label");
    assert_eq!(shape.fields[3].offset, 24);
    assert_eq!(shape.fields[3].type_name, "char *");
}

#[test]
fn binaries_without_dwarf_yield_nothing() {
    let structs = import_structs(&fixture("arm64-symbolized")).expect("parse ok");
    assert!(
        structs.is_empty(),
        "no debug info, no structs: {structs:#?}"
    );
}
