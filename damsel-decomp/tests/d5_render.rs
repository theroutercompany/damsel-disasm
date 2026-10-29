//! Pseudocode renderer contract tests over the real `_main` lift.

mod support;

use damsel_core::{DisassemblyLimit, DisassemblyOptions, DisassemblyRequestV2, DisassemblyTarget};
use damsel_decomp::{lift_arm64, render_pseudocode};

fn render_main() -> damsel_decomp::Pseudocode {
    let image = damsel_macho::load(support::fixture("arm64-symbolized")).unwrap();
    let request = DisassemblyRequestV2 {
        target: DisassemblyTarget::Symbol("_main".to_string()),
        range: None,
        limit: DisassemblyLimit::Unlimited,
        options: DisassemblyOptions {
            include_annotations: false,
            include_value_flow: false,
        },
    };
    let result = damsel_macho::disassemble_v2(&image, &request).unwrap();
    let function = lift_arm64(&result.instructions);
    render_pseudocode(&function, Some("_main"))
}

#[test]
fn main_renders_with_calls_frame_and_honest_unknowns() {
    let pseudocode = render_main();
    let text: Vec<&str> = pseudocode.lines.iter().map(|l| l.text.as_str()).collect();

    // Signature and return.
    assert_eq!(text[0], "fn _main() {");
    assert!(text.iter().any(|l| *l == "return;"));
    assert_eq!(*text.last().unwrap(), "}");

    // Frame allocation from the real prologue.
    assert!(text.iter().any(|l| l.contains("sp = (sp - 0x20)")), "got: {text:?}");

    // Both calls render (stub 0x4f4 and _exported_sum 0x460).
    assert!(text.iter().any(|l| l.contains("sub_1000004f4(")));
    assert!(text.iter().any(|l| l.contains("sub_100000460(")));

    // cset has no v1 semantics → visible unknown comment, not silence.
    assert!(text.iter().any(|l| l.contains("/* unknown: cset */")));

    // Every non-brace line carries source addresses (provenance rule).
    for (index, line) in pseudocode.lines.iter().enumerate() {
        if index == 0 || index + 1 == pseudocode.lines.len() {
            continue;
        }
        assert!(
            !line.source_addresses.is_empty(),
            "line {index} ({}) lost provenance",
            line.text
        );
    }

    // Deterministic serialization.
    let a = serde_json::to_string(&pseudocode).unwrap();
    let b = serde_json::to_string(&render_main()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn empty_function_renders_shell() {
    let function = lift_arm64(&[]);
    let pseudocode = render_pseudocode(&function, None);
    assert_eq!(pseudocode.lines.len(), 2); // fn … { }
    assert!(pseudocode.lines[0].text.starts_with("fn sub_"));
}
