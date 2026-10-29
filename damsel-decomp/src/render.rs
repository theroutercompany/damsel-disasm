//! D5 pseudocode renderer v1 (Wave 6; strategy §Pseudocode Renderer).
//!
//! Line model: every line carries the addresses it was derived from
//! (v1 granularity: block start; statement-level sites where the IR
//! records them). Control flow renders in honest goto form until D4
//! structuring lands; `Unknown` statements render as visible comments
//! — never fabricated arithmetic.

use crate::ir::{IrCallTarget, IrExpr, IrFunction, IrStatement, IrTerminator, IrValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PseudocodeLine {
    pub indent: usize,
    pub text: String,
    /// Addresses this line derives from (v1: block start + statement
    /// sites where recorded).
    pub source_addresses: Vec<u64>,
    /// Warnings (unknown semantics, unresolved targets).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pseudocode {
    pub function: String,
    pub name: Option<String>,
    pub lines: Vec<PseudocodeLine>,
}

pub fn render_pseudocode(function: &IrFunction, name: Option<&str>) -> Pseudocode {
    let diamonds = crate::structure::find_diamonds(function);
    render_pseudocode_with_loops(function, name, &diamonds)
}

pub fn render_pseudocode_with_loops(
    function: &IrFunction,
    name: Option<&str>,
    diamonds: &[crate::structure::Diamond],
) -> Pseudocode {
    let loops = crate::structure::find_loops(function);
    let natural = crate::structure::find_natural_loops(function);
    let mut pseudocode = render_pseudocode_impl(function, name, diamonds);

    // do-while: a natural loop whose back edge is the header's own
    // CondBranch renders `} while (cond);`. Other back edges get their
    // specific goto annotated in place.
    for line in &mut pseudocode.lines {
        let Some((taken, _fall)) = parse_goto_branch(&line.text) else {
            // Bare `goto sub_X;` lines (Branch terminators).
            if let Some(address) = parse_goto_target(&line.text) {
                if loops.iter().any(|l| l.to_address == address) {
                    line.text = line
                        .text
                        .replace(&format!("goto sub_{address:x};"), &format!("goto sub_{address:x}; /* loop back */"));
                }
            }
            continue;
        };
        if let Some(natural_loop) = natural
            .iter()
            .find(|l| l.back_edge_from == l.header_block && l.to_address == taken)
        {
            if let Some(condition) = &natural_loop.condition {
                let text = crate::structure::render_condition_text(condition);
                line.text = format!("}} while ({text});");
                continue;
            }
        }
        if loops.iter().any(|l| l.to_address == taken) {
            line.text = line.text.replace(
                &format!("goto sub_{taken:x};"),
                &format!("goto sub_{taken:x}; /* loop back */"),
            );
        }
    }
    pseudocode
}

fn parse_goto_branch(text: &str) -> Option<(u64, u64)> {
    // "if (…) { goto sub_A; } else { goto sub_B; }" → (A, B).
    // Only the hex run immediately after each "sub_" counts.
    let mut targets = Vec::new();
    for part in text.split("goto sub_").skip(1) {
        let hex: String = part
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        if let Ok(address) = u64::from_str_radix(&hex, 16) {
            targets.push(address);
        }
    }
    match (targets.first().copied(), targets.get(1).copied()) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
}

fn parse_goto_target(text: &str) -> Option<u64> {
    let rest = text.strip_prefix("goto sub_")?;
    let hex = rest.strip_suffix(";")?;
    u64::from_str_radix(hex, 16).ok()
}

fn render_pseudocode_impl(
    function: &IrFunction,
    name: Option<&str>,
    diamonds: &[crate::structure::Diamond],
) -> Pseudocode {
    let mut lines = vec![PseudocodeLine {
        indent: 0,
        text: format!("fn {}() {{", name.unwrap_or(&function.function_id_placeholder())),
        source_addresses: vec![function.entry_address],
        warnings: Vec::new(),
    }];

    let mut rendered_blocks: std::collections::BTreeSet<usize> = Default::default();
    // Diamond arms render inside their cond block; skip standalone.
    for diamond in diamonds {
        rendered_blocks.insert(diamond.then_block);
        rendered_blocks.insert(diamond.else_block);
    }

    for block in &function.blocks {
        // Diamond cond block: structured if/else with arm bodies.
        if let Some(diamond) = diamonds.iter().find(|d| d.cond_block == block.id) {
            let condition_text = crate::structure::render_condition_text(&diamond.condition);
            lines.push(PseudocodeLine {
                indent: 1,
                text: format!("if ({condition_text}) {{"),
                source_addresses: vec![block.start_address],
                warnings: Vec::new(),
            });
            for arm_id in [diamond.then_block, diamond.else_block] {
                let is_else = arm_id == diamond.else_block;
                if is_else {
                    lines.push(PseudocodeLine {
                        indent: 1,
                        text: "} else {".to_string(),
                        source_addresses: vec![block.start_address],
                        warnings: Vec::new(),
                    });
                }
                if let Some(arm) = function.blocks.iter().find(|b| b.id == arm_id) {
                    for statement in &arm.statements {
                        let (text, warnings) = render_statement(statement);
                        let mut source = vec![arm.start_address];
                        match statement {
                            IrStatement::Call { site, .. } | IrStatement::Unknown { site, .. } => {
                                source.push(*site)
                            }
                            _ => {}
                        }
                        lines.push(PseudocodeLine {
                            indent: 2,
                            text,
                            source_addresses: source,
                            warnings,
                        });
                    }
                }
            }
            lines.push(PseudocodeLine {
                indent: 1,
                text: "}".to_string(),
                source_addresses: vec![block.start_address],
                warnings: Vec::new(),
            });
            // The cond block's own statements rendered below continue
            // through the normal path; skip re-rendering its goto line.
            for statement in &block.statements {
                let (text, warnings) = render_statement(statement);
                lines.push(PseudocodeLine {
                    indent: 1,
                    text,
                    source_addresses: vec![block.start_address],
                    warnings,
                });
            }
            continue;
        }
        if rendered_blocks.contains(&block.id) {
            continue;
        }
        for statement in &block.statements {
            let (text, warnings) = render_statement(statement);
            let mut source = vec![block.start_address];
            match statement {
                IrStatement::Call { site, .. } | IrStatement::Unknown { site, .. } => {
                    source.push(*site)
                }
                _ => {}
            }
            lines.push(PseudocodeLine {
                indent: 1,
                text,
                source_addresses: source,
                warnings,
            });
        }
        let (text, warnings) = render_terminator(&block.terminator);
        if !text.is_empty() {
            lines.push(PseudocodeLine {
                indent: 1,
                text,
                source_addresses: vec![block.start_address],
                warnings,
            });
        }
    }

    lines.push(PseudocodeLine {
        indent: 0,
        text: "}".to_string(),
        source_addresses: Vec::new(),
        warnings: Vec::new(),
    });

    Pseudocode {
        function: format!("{:#x}", function.entry_address),
        name: name.map(str::to_string),
        lines,
    }
}

impl IrFunction {
    fn function_id_placeholder(&self) -> String {
        format!("sub_{:x}", self.entry_address)
    }
}

fn render_statement(statement: &IrStatement) -> (String, Vec<String>) {
    match statement {
        IrStatement::Assign { destination, expression } => {
            (format!("{destination} = {};", render_expr(expression)), Vec::new())
        }
        IrStatement::Load { destination, address } => {
            (format!("{destination} = *({});", render_expr(address)), Vec::new())
        }
        IrStatement::Store { address, value } => {
            (format!("*({}) = {};", render_expr(address), render_expr(value)), Vec::new())
        }
        IrStatement::Call { target, .. } => match target {
            IrCallTarget::Function(address) => {
                (format!("sub_{address:x}();"), Vec::new())
            }
            IrCallTarget::Unknown(via) => (
                format!("call_unknown({via});"),
                vec!["unresolved target".to_string()],
            ),
        },
        IrStatement::Unknown { mnemonic, .. } => (
            format!("/* unknown: {mnemonic} */"),
            vec![format!("no v1 semantics for `{mnemonic}`")],
        ),
    }
}

fn render_terminator(terminator: &IrTerminator) -> (String, Vec<String>) {
    match terminator {
        IrTerminator::Return => ("return;".to_string(), Vec::new()),
        IrTerminator::Branch(target) => (format!("goto sub_{target:x};"), Vec::new()),
        IrTerminator::CondBranch { condition, taken, fallthrough } => (
            format!(
                "if ({}) {{ goto sub_{taken:x}; }} else {{ goto sub_{fallthrough:x}; }}",
                render_condition(condition)
            ),
            Vec::new(),
        ),
        IrTerminator::Trap => ("trap;".to_string(), Vec::new()),
        IrTerminator::WindowEnd => (String::new(), Vec::new()),
        IrTerminator::Unknown { mnemonic } => (
            format!("/* terminator unknown: {mnemonic} */"),
            vec!["unresolved terminator".to_string()],
        ),
    }
}

fn render_condition(condition: &crate::ir::IrCondition) -> String {
    match condition {
        crate::ir::IrCondition::Raw(raw) => format!("flags:{raw}"),
    }
}

fn render_expr(expression: &IrExpr) -> String {
    match expression {
        IrExpr::Value(IrValue::Const(value)) => format!("{value:#x}"),
        IrExpr::Value(IrValue::Register(register)) => register.clone(),
        IrExpr::Value(IrValue::Unknown(what)) => format!("unknown({what})"),
        IrExpr::BinOp { op, lhs, rhs } => {
            let op = match op {
                crate::ir::IrBinOp::Add => "+",
                crate::ir::IrBinOp::Sub => "-",
                crate::ir::IrBinOp::And => "&",
                crate::ir::IrBinOp::Or => "|",
                crate::ir::IrBinOp::Xor => "^",
                crate::ir::IrBinOp::Mul => "*",
                crate::ir::IrBinOp::Shl => "<<",
                crate::ir::IrBinOp::Shr => ">>",
            };
            format!("({} {} {})", render_expr(lhs), op, render_expr(rhs))
        }
        IrExpr::AddrOf { base, displacement } => {
            if *displacement == 0 {
                format!("&{base}[0]")
            } else if *displacement < 0 {
                format!("&{base}[-{:#x}]", displacement.unsigned_abs())
            } else {
                format!("&{base}[{displacement:#x}]")
            }
        }
        IrExpr::SegmentAddr { segment, displacement } => {
            format!("&{segment}[{displacement:#x}]")
        }
        IrExpr::IndexedAddr { base, index, scale, displacement } => {
            let scaled = if *scale == 1 {
                index.clone()
            } else {
                format!("{index} * {scale:#x}")
            };
            match displacement {
                0 => format!("&{base}[{scaled}]"),
                d if *d < 0 => format!("&{base}[{scaled} - {:#x}]", d.unsigned_abs()),
                d => format!("&{base}[{scaled} + {d:#x}]"),
            }
        }
    }
}
