use assert_cmd::Command;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    repo_root().join("fixtures/bin").join(name)
}

fn run_json_ok(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(
        output.status.success(),
        "stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn run_json_err(args: &[&str]) -> String {
    let output = Command::cargo_bin("damsel-cli")
        .expect("binary exists")
        .args(args)
        .output()
        .expect("command runs");
    assert!(
        !output.status.success(),
        "expected failure, stdout={:?} stderr={:?}",
        output.stdout,
        output.stderr
    );
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn parse_json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|error| panic!("invalid json: {error}\n{text}"))
}

fn sorted_object_keys(value: &Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .unwrap_or_else(|| panic!("expected object: {value}"))
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn assert_exact_object_keys(value: &Value, expected: &[&str]) {
    let mut expected_keys = expected
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    expected_keys.sort();
    assert_eq!(sorted_object_keys(value), expected_keys);
}

#[test]
fn info_json_contract() {
    let path = fixture("arm64-symbolized");
    let out = run_json_ok(&[
        "--format",
        "json",
        "info",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "info");
    assert_eq!(json["data"]["architecture"], "arm64");
    assert!(json["data"]["available_slices"].is_array());
    assert!(json["data"]["dyld"]["stub_helpers"].is_number());
}

#[test]
fn dyld_json_contract_exposes_bindings_and_stubs() {
    let path = fixture("import-rich");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "dyld");
    assert!(json["data"]["included_sections"].is_array());
    assert!(json["data"]["import_bindings"].is_array());
    assert!(json["data"]["stubs"].is_array());
    assert!(json["data"]["stub_helpers"].is_array());
    let exports = json["data"]["exports"].as_array().expect("exports array");
    if let Some(export) = exports.first() {
        assert_exact_object_keys(
            export,
            &[
                "name",
                "address",
                "raw_flags",
                "flags",
                "kind",
                "reexport_target",
                "resolver_target",
            ],
        );
        assert_exact_object_keys(
            &export["flags"],
            &[
                "raw_bits",
                "kind_bits",
                "is_weak_definition",
                "is_reexport",
                "is_stub_and_resolver",
                "is_thread_local",
                "is_absolute",
                "unknown_bits",
                "flag_names",
            ],
        );
    }
    assert_exact_object_keys(
        &json["data"],
        &[
            "has_rebases",
            "has_binds",
            "has_chained_fixups",
            "included_sections",
            "name_filter",
            "dylib_filter",
            "source_filter",
            "binding_kind_filter",
            "stub_kind_filter",
            "export_kind_filter",
            "export_flag_filter",
            "ordinal_filter",
            "dylibs",
            "rpaths",
            "exports",
            "function_starts",
            "import_bindings",
            "stubs",
            "stub_helpers",
        ],
    );
    let stubs = json["data"]["stubs"].as_array().expect("stub array");
    assert!(!stubs.is_empty(), "{out}");
    assert_eq!(stubs[0]["section"], "__TEXT:__stubs");
    assert!(stubs[0]["stub_kind"].is_string());
    assert_exact_object_keys(
        &stubs[0],
        &[
            "stub_address",
            "section",
            "pointer_address",
            "pointer_section",
            "helper_address",
            "binding_ordinal",
            "stub_kind",
            "dylib",
            "name",
            "source",
        ],
    );
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert_exact_object_keys(
        &bindings[0],
        &[
            "dylib",
            "name",
            "address",
            "offset",
            "addend",
            "source",
            "ordinal",
            "symbol_index",
            "binding_kind",
            "is_weak",
        ],
    );
}

#[test]
fn dyld_json_filters_bindings_by_name_and_source() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--name",
        "puts",
        "--source",
        "indirect-symbol",
    ]);
    let json = parse_json(&out);
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert!(bindings.iter().all(|binding| {
        binding["name"]
            .as_str()
            .is_some_and(|name| name.to_ascii_lowercase().contains("puts"))
            && binding["source"] == "IndirectSymbol"
            && binding["binding_kind"] == "Lazy"
    }));
}

#[test]
fn dyld_json_filters_by_kind_and_ordinal() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
        "--stubs",
        "--helpers",
        "--binding-kind",
        "lazy",
        "--stub-kind",
        "lazy",
        "--ordinal",
        "1",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["binding_kind_filter"], "Lazy");
    assert_eq!(json["data"]["stub_kind_filter"], "Lazy");
    assert_eq!(json["data"]["ordinal_filter"], 1);
    let bindings = json["data"]["import_bindings"]
        .as_array()
        .expect("binding array");
    assert!(!bindings.is_empty(), "{out}");
    assert!(
        bindings
            .iter()
            .all(|binding| binding["binding_kind"] == "Lazy" && binding["ordinal"] == 1)
    );
    let stubs = json["data"]["stubs"].as_array().expect("stub array");
    assert!(!stubs.is_empty(), "{out}");
    assert!(
        stubs
            .iter()
            .all(|stub| stub["stub_kind"] == "Lazy" && stub["binding_ordinal"] == 1)
    );
    let helpers = json["data"]["stub_helpers"]
        .as_array()
        .expect("helper array");
    assert!(!helpers.is_empty(), "{out}");
    assert!(helpers.iter().all(|helper| helper["binding_ordinal"] == 1));
}

#[test]
fn dyld_json_filters_exports_by_kind() {
    let path = fixture("export-kinds");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--exports",
        "--export-kind",
        "regular",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["export_kind_filter"], "Regular");
    let exports = json["data"]["exports"].as_array().expect("exports array");
    assert!(!exports.is_empty(), "{out}");
    assert!(
        exports
            .iter()
            .all(|export| export["kind"]["type"] == "regular")
    );
}

#[test]
fn dyld_json_filters_exports_by_flag() {
    let path = fixture("export-kinds");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--exports",
        "--export-flag",
        "absolute",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["export_flag_filter"], "Absolute");
    let exports = json["data"]["exports"].as_array().expect("exports array");
    assert!(!exports.is_empty(), "{out}");
    assert!(exports.iter().all(|export| export["flags"]["is_absolute"] == true));
    assert!(exports.iter().all(|export| {
        export["flags"]["flag_names"]
            .as_array()
            .is_some_and(|names| names.iter().any(|name| name == "Absolute"))
    }));
}

#[test]
fn dyld_json_exposes_stub_helpers_for_lazy_fixture() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    let helpers = json["data"]["stub_helpers"]
        .as_array()
        .expect("helper array");
    assert!(!helpers.is_empty(), "{out}");
    assert!(helpers[0]["target_stub"].is_number());
    assert!(helpers[0]["stub_section"].is_string() || helpers[0]["stub_section"].is_null());
    assert!(helpers[0]["pointer_section"].is_string() || helpers[0]["pointer_section"].is_null());
    assert!(helpers[0]["pointer_address"].is_number() || helpers[0]["pointer_address"].is_null());
    assert_exact_object_keys(
        &helpers[0],
        &[
            "helper_address",
            "target_stub",
            "binding_ordinal",
            "stub_section",
            "pointer_address",
            "pointer_section",
            "dylib",
            "name",
        ],
    );
}

#[test]
fn dyld_json_section_toggles_keep_key_stability() {
    let path = fixture("import-lazy");
    let out = run_json_ok(&[
        "--format",
        "json",
        "dyld",
        path.to_str().expect("utf8 path"),
        "--bindings",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "dyld");
    assert_eq!(
        json["data"]["included_sections"],
        Value::Array(vec![Value::String("import_bindings".to_string())])
    );
    assert!(
        json["data"]["import_bindings"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
    );
    assert_eq!(json["data"]["dylibs"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["rpaths"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["exports"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["function_starts"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["stubs"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["stub_helpers"], Value::Array(Vec::new()));
    assert_exact_object_keys(
        &json["data"],
        &[
            "has_rebases",
            "has_binds",
            "has_chained_fixups",
            "included_sections",
            "name_filter",
            "dylib_filter",
            "source_filter",
            "binding_kind_filter",
            "stub_kind_filter",
            "export_kind_filter",
            "export_flag_filter",
            "ordinal_filter",
            "dylibs",
            "rpaths",
            "exports",
            "function_starts",
            "import_bindings",
            "stubs",
            "stub_helpers",
        ],
    );
}

#[test]
fn objc_json_contract_exposes_structured_runtime_records() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "objc");
    assert!(json["data"]["class_names"].is_array());
    assert!(json["data"]["pointer_refs"].is_array());
    assert!(json["data"]["classes"].is_array());
    assert!(json["data"]["protocols"].is_array());
    assert!(json["data"]["categories"].is_array());
    assert_exact_object_keys(
        &json["data"],
        &[
            "requested_detail",
            "owner_filter",
            "name_source_filter",
            "selector_source_filter",
            "category_source_filter",
            "image_info_flags",
            "class_names",
            "selector_names",
            "method_names",
            "pointer_refs",
            "classes",
            "protocols",
            "categories",
            "methods",
            "properties",
            "ivars",
        ],
    );
    let classes = json["data"]["classes"].as_array().expect("class array");
    if let Some(class) = classes.first() {
        assert!(class["name_source"].is_string());
    }
    let categories = json["data"]["categories"]
        .as_array()
        .expect("category array");
    if let Some(category) = categories.first() {
        assert!(category["record_source"].is_string());
        assert!(category["property_list_pointer"].is_null() || category["property_list_pointer"].is_number());
        assert!(category["protocol_list_pointer"].is_null() || category["protocol_list_pointer"].is_number());
        assert!(category["properties_source"].is_string());
        assert!(category["protocols_source"].is_string());
    }
}

#[test]
fn objc_json_detail_and_owner_filters_are_structured() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
        "--owner",
        "GreetingProviding",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["requested_detail"], "methods");
    assert_eq!(json["data"]["owner_filter"], "GreetingProviding");
    let methods = json["data"]["methods"].as_array().expect("methods array");
    assert!(!methods.is_empty(), "{out}");
    assert!(methods.iter().all(|entry| {
        entry["owner_name"]
            .as_str()
            .is_some_and(|name| name.contains("GreetingProviding"))
    }));
}

#[test]
fn objc_json_provenance_filters_are_applied() {
    let path = fixture("objc-sample");
    let baseline = parse_json(&run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
    ]));
    let class_source = baseline["data"]["classes"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["name_source"].as_str())
        .unwrap_or("Unresolved");
    let selector_source = baseline["data"]["methods"]
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["selector_source"].as_str())
        .unwrap_or("Unresolved");
    let class_source_arg = match class_source {
        "Runtime" => "runtime",
        "PointerTable" => "pointer-table",
        "LegacyPool" => "legacy-pool",
        "Unresolved" => "unresolved",
        value => panic!("unexpected name_source: {value}"),
    };
    let selector_source_arg = match selector_source {
        "Direct" => "direct",
        "Relative" => "relative",
        "LegacyPool" => "legacy-pool",
        "Unresolved" => "unresolved",
        value => panic!("unexpected selector_source: {value}"),
    };

    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--name-source",
        class_source_arg,
        "--selector-source",
        selector_source_arg,
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["name_source_filter"], class_source);
    assert_eq!(json["data"]["selector_source_filter"], selector_source);
    let classes = json["data"]["classes"].as_array().expect("classes array");
    if !classes.is_empty() {
        assert!(
            classes
                .iter()
                .all(|entry| entry["name_source"] == class_source)
        );
    }
    let methods = json["data"]["methods"].as_array().expect("methods array");
    if !methods.is_empty() {
        assert!(
            methods
                .iter()
                .all(|entry| entry["selector_source"] == selector_source)
        );
    }
}

#[test]
fn objc_json_category_source_filter_is_applied() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--category-source",
        "symbol-synthesis",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["category_source_filter"], "SymbolSynthesis");
    let categories = json["data"]["categories"]
        .as_array()
        .expect("categories array");
    assert!(!categories.is_empty(), "{out}");
    assert!(
        categories
            .iter()
            .all(|category| category["record_source"] == "SymbolSynthesis")
    );
    assert!(categories.iter().any(|category| {
        category["protocol_list_pointer"].is_number() || category["property_list_pointer"].is_number()
    }));
}

#[test]
fn objc_json_category_source_with_lists_filter_is_applied() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "all",
        "--category-source",
        "symbol-synthesis-with-lists",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["category_source_filter"], "SymbolSynthesisWithLists");
    let categories = json["data"]["categories"]
        .as_array()
        .expect("categories array");
    assert!(!categories.is_empty(), "{out}");
    assert!(categories.iter().all(|category| {
        category["record_source"] == "SymbolSynthesis"
            && (category["protocol_list_pointer"].is_number()
                || category["property_list_pointer"].is_number())
    }));
}

#[test]
fn objc_json_detail_toggle_keeps_empty_sections_and_stable_keys() {
    let path = fixture("objc-sample");
    let out = run_json_ok(&[
        "--format",
        "json",
        "objc",
        path.to_str().expect("utf8 path"),
        "--detail",
        "methods",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["data"]["requested_detail"], "methods");
    assert!(
        json["data"]["methods"]
            .as_array()
            .is_some_and(|entries| !entries.is_empty())
    );
    assert_eq!(json["data"]["classes"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["protocols"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["categories"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["properties"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["ivars"], Value::Array(Vec::new()));
    assert_eq!(json["data"]["pointer_refs"], Value::Array(Vec::new()));
    assert_exact_object_keys(
        &json["data"],
        &[
            "requested_detail",
            "owner_filter",
            "name_source_filter",
            "selector_source_filter",
            "category_source_filter",
            "image_info_flags",
            "class_names",
            "selector_names",
            "method_names",
            "pointer_refs",
            "classes",
            "protocols",
            "categories",
            "methods",
            "properties",
            "ivars",
        ],
    );
}

#[test]
fn slices_json_contract_exposes_full_inventory() {
    let path = fixture("universal-hello");
    let out = run_json_ok(&[
        "--format",
        "json",
        "slices",
        path.to_str().expect("utf8 path"),
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "slices");
    let slices = json["data"].as_array().expect("slice array");
    assert!(slices.iter().any(|entry| entry["selected"] == true));
    assert!(slices.iter().any(|entry| entry["architecture"] == "arm64"));
    assert!(slices.iter().any(|entry| entry["architecture"] == "x86_64"));
}

#[test]
fn disasm_json_contract_has_window_and_analysis_fields() {
    let path = fixture("semantic-switch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--section",
        "__text",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    assert_eq!(json["command"], "disasm");
    assert!(json["data"]["window_end"].is_null() || json["data"]["window_end"].is_number());
    assert!(json["data"]["decoded_bytes"].is_number());
    assert!(json["data"]["stop_reason"].is_string());
    assert!(json["data"]["instruction_count"].is_number());
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(!instructions.is_empty(), "{out}");
    assert!(instructions[0]["references"].is_array());
    assert!(instructions[0]["annotations"].is_array());
    assert!(instructions[0]["recovered_values"].is_array());
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations
                    .iter()
                    .any(|annotation| annotation["type"] == "jump_table_candidate")
            })
    }));
    for annotation in instructions
        .iter()
        .flat_map(|instruction| instruction["annotations"].as_array().into_iter().flatten())
        .filter(|annotation| annotation["type"] == "table_slot_resolved")
    {
        assert!(annotation["table_base"].is_number());
        assert!(annotation["slot_address"].is_number());
        assert!(annotation["index_register"].is_string());
        assert!(annotation["element_size"].is_number());
        assert!(annotation["target"].is_number());
    }
    let helper_refs = instructions
        .iter()
        .flat_map(|instruction| instruction["references"].as_array().into_iter().flatten())
        .filter(|reference| reference["type"] == "stub_helper")
        .collect::<Vec<_>>();
    if let Some(reference) = helper_refs.first() {
        assert!(reference["helper_address"].is_number());
        assert!(reference["target_stub"].is_number() || reference["target_stub"].is_null());
        assert!(reference["pointer_address"].is_number() || reference["pointer_address"].is_null());
        assert!(reference["stub_section"].is_string() || reference["stub_section"].is_null());
        assert!(reference["pointer_section"].is_string() || reference["pointer_section"].is_null());
        assert_exact_object_keys(
            reference,
            &[
                "type",
                "helper_address",
                "target_stub",
                "stub_section",
                "pointer_address",
                "pointer_section",
                "binding_ordinal",
                "dylib",
                "name",
            ],
        );
    }
    assert_exact_object_keys(
        &json["data"],
        &[
            "target",
            "start_address",
            "end_address",
            "window_end",
            "bytes_len",
            "decoded_bytes",
            "instruction_count",
            "stop_reason",
            "instructions",
        ],
    );
    assert_exact_object_keys(
        &instructions[0],
        &[
            "address",
            "size",
            "opcode",
            "mnemonic",
            "operands",
            "rendered",
            "references",
            "annotations",
            "recovered_values",
        ],
    );
}

#[test]
fn disasm_json_exposes_typed_indirect_target_reasons() {
    let path = fixture("indirect-dispatch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_dispatch_second_slot",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "indirect_target_resolved"
                        && annotation["reason"] == "function-pointer"
                })
            })
    }));
    assert!(instructions.iter().any(|instruction| {
        instruction["annotations"]
            .as_array()
            .is_some_and(|annotations| {
                annotations.iter().any(|annotation| {
                    annotation["type"] == "table_slot_resolved"
                        && annotation["target"].is_number()
                })
            })
    }));
}

#[test]
fn disasm_json_exposes_export_address_recovered_values() {
    let path = fixture("indirect-dispatch");
    let out = run_json_ok(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_load_export_target",
        "--show-references",
        "--show-values",
    ]);
    let json = parse_json(&out);
    let instructions = json["data"]["instructions"]
        .as_array()
        .expect("instruction array");
    assert!(instructions.iter().any(|instruction| {
        instruction["recovered_values"]
            .as_array()
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value["kind"] == "ExportAddress")
            })
    }));
}

#[test]
fn error_json_envelope_for_invalid_args() {
    let path = fixture("arm64-symbolized");
    let err = run_json_err(&[
        "--format",
        "json",
        "disasm",
        path.to_str().expect("utf8 path"),
        "--symbol",
        "_main",
        "--count",
        "8",
        "--limit",
        "4",
    ]);
    let json = parse_json(&err);
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "error");
    assert_eq!(json["data"]["code"], "invalid_args");
    assert_eq!(
        json["data"]["message"],
        "`--count` and `--limit` cannot differ when both are provided"
    );
}
