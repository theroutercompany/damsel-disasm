use crate::{Annotation, DecodedInstruction, Operand, Reference};
use capstone::arch;
use capstone::arch::arm64::{Arm64Operand, Arm64OperandType};
use capstone::arch::ArchDetail;
use capstone::prelude::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("unsupported architecture for decoder")]
    UnsupportedArchitecture,
    #[error("invalid instruction stream: {0}")]
    InvalidInstruction(String),
}

trait InstructionDecoder {
    fn decode(
        &self,
        bytes: &[u8],
        start_address: u64,
        max_instructions: Option<usize>,
    ) -> Result<Vec<DecodedInstruction>, DecodeError>;
}

pub(crate) fn decode_aarch64(
    bytes: &[u8],
    start_address: u64,
    max_instructions: Option<usize>,
) -> Result<Vec<DecodedInstruction>, DecodeError> {
    Bad64Decoder.decode(bytes, start_address, max_instructions)
}

struct Bad64Decoder;

impl InstructionDecoder for Bad64Decoder {
    fn decode(
        &self,
        bytes: &[u8],
        start_address: u64,
        max_instructions: Option<usize>,
    ) -> Result<Vec<DecodedInstruction>, DecodeError> {
        let cs = Capstone::new()
            .arm64()
            .mode(arch::arm64::ArchMode::Arm)
            .detail(true)
            .build()
            .map_err(|err| DecodeError::InvalidInstruction(err.to_string()))?;
        let insns = cs
            .disasm_all(bytes, start_address)
            .map_err(|err| DecodeError::InvalidInstruction(err.to_string()))?;
        let limit = max_instructions.unwrap_or(usize::MAX);
        let mut instructions = Vec::new();

        for insn in insns.iter().take(limit) {
            let mnemonic = insn.mnemonic().unwrap_or("unknown").to_string();
            let detail = cs
                .insn_detail(insn)
                .map_err(|err| DecodeError::InvalidInstruction(err.to_string()))?;
            let ArchDetail::Arm64Detail(arm64_detail) = detail.arch_detail();
            let arm64_operands = arm64_detail.operands().collect::<Vec<_>>();
            let operands = arm64_detail
                .operands()
                .map(|operand| map_operand(&cs, &mnemonic, arm64_detail.writeback(), &operand))
                .collect::<Vec<_>>();
            let derivation = derive_references(&cs, &detail, &mnemonic, &arm64_operands);
            let mut references = derivation.references;
            let mut annotations = Vec::new();

            if insn.address() == start_address {
                annotations.push(Annotation::Note("range start".to_string()));
            }
            for note in derivation.notes {
                annotations.push(Annotation::Note(note));
            }

            if references.is_empty() {
                references.shrink_to_fit();
            }

            instructions.push(DecodedInstruction {
                address: insn.address(),
                size: insn.bytes().len().try_into().unwrap_or(4),
                opcode: opcode_from_bytes(insn.bytes()),
                mnemonic,
                operands,
                recovered_values: Vec::new(),
                references,
                annotations,
            });
        }

        Ok(instructions)
    }
}

struct ReferenceDerivation {
    references: Vec<Reference>,
    notes: Vec<String>,
}

fn derive_references(
    cs: &Capstone,
    detail: &InsnDetail<'_>,
    mnemonic: &str,
    operands: &[Arm64Operand],
) -> ReferenceDerivation {
    let mut references = Vec::new();
    let mut notes = Vec::new();
    let lower = mnemonic.to_ascii_lowercase();
    let is_call_group = has_group(cs, detail, "call");
    let is_jump_group = has_group(cs, detail, "jump");
    let is_page_materialization = lower == "adr" || lower == "adrp";
    let is_call_like = is_call_group || is_call_mnemonic(&lower);
    let is_branch_like = is_jump_group || is_relative_branch(&lower);
    let is_control_flow = is_call_like || is_branch_like;

    for operand in operands {
        match operand.op_type {
            Arm64OperandType::Imm(value) if value >= 0 => {
                let target = value as u64;
                let reference = if is_page_materialization {
                    Reference::Page { target }
                } else if is_call_like {
                    Reference::Call { target }
                } else if is_branch_like {
                    Reference::Branch { target }
                } else {
                    Reference::Data { target }
                };
                push_reference(&mut references, reference);
            }
            Arm64OperandType::Reg(register) if is_control_flow => {
                if is_indirect_control(&lower) {
                    let register = cs
                        .reg_name(register)
                        .unwrap_or_else(|| format!("{register:?}"));
                    let reference = if is_call_like {
                        Reference::IndirectCall {
                            via: register.clone(),
                        }
                    } else {
                        Reference::IndirectBranch {
                            via: register.clone(),
                        }
                    };
                    push_reference(&mut references, reference);
                    let kind = if is_call_like {
                        "indirect call"
                    } else {
                        "indirect branch"
                    };
                    push_note(&mut notes, format!("{kind} via {register}"));
                }
            }
            Arm64OperandType::Mem(mem) if is_control_flow => {
                if is_indirect_control(&lower) {
                    let base = cs
                        .reg_name(mem.base())
                        .unwrap_or_else(|| format!("{:?}", mem.base()));
                    let reference = if is_call_like {
                        Reference::IndirectCall { via: base.clone() }
                    } else {
                        Reference::IndirectBranch { via: base.clone() }
                    };
                    push_reference(&mut references, reference);
                    let kind = if is_call_like {
                        "indirect call"
                    } else {
                        "indirect branch"
                    };
                    push_note(&mut notes, format!("{kind} via [{base}]"));
                }
            }
            _ => {}
        }
    }

    if references.is_empty() && is_control_flow && !is_return_like(&lower) {
        push_note(
            &mut notes,
            format!("control-flow target unresolved ({mnemonic})"),
        );
    }

    ReferenceDerivation { references, notes }
}

fn has_group(cs: &Capstone, detail: &InsnDetail<'_>, expected: &str) -> bool {
    detail.groups().iter().any(|group| {
        cs.group_name(*group)
            .as_deref()
            .map(|name| name.eq_ignore_ascii_case(expected))
            .unwrap_or(false)
    })
}

fn push_reference(target: &mut Vec<Reference>, reference: Reference) {
    if !target.contains(&reference) {
        target.push(reference);
    }
}

fn push_note(target: &mut Vec<String>, note: String) {
    if !target.iter().any(|existing| existing == &note) {
        target.push(note);
    }
}

fn is_relative_branch(mnemonic: &str) -> bool {
    mnemonic == "b"
        || mnemonic.starts_with("b.")
        || mnemonic == "cbz"
        || mnemonic == "cbnz"
        || mnemonic == "tbz"
        || mnemonic == "tbnz"
}

fn is_call_mnemonic(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "bl" | "blr" | "blraa" | "blraaz" | "blrab" | "blrabz"
    )
}

fn is_indirect_control(mnemonic: &str) -> bool {
    matches!(
        mnemonic,
        "blr"
            | "br"
            | "blraa"
            | "blraaz"
            | "blrab"
            | "blrabz"
            | "braa"
            | "braaz"
            | "brab"
            | "brabz"
    )
}

fn is_return_like(mnemonic: &str) -> bool {
    matches!(mnemonic, "ret" | "eret" | "drps" | "retaa" | "retab")
}

fn map_operand(cs: &Capstone, mnemonic: &str, writeback: bool, operand: &Arm64Operand) -> Operand {
    let lower = mnemonic.to_ascii_lowercase();
    match operand.op_type {
        Arm64OperandType::Reg(register) => Operand::Register(
            cs.reg_name(register)
                .unwrap_or_else(|| format!("{register:?}")),
        ),
        Arm64OperandType::Imm(value) => {
            if (is_relative_branch(&lower) || lower == "adr" || lower == "adrp") && value >= 0 {
                Operand::Label(value as u64)
            } else if value < 0 {
                Operand::ImmediateSigned(value)
            } else {
                Operand::ImmediateUnsigned(value as u64)
            }
        }
        Arm64OperandType::Mem(mem) => Operand::Memory {
            base: cs
                .reg_name(mem.base())
                .unwrap_or_else(|| format!("{:?}", mem.base())),
            index: cs.reg_name(mem.index()),
            displacement: i64::from(mem.disp()),
            writeback: writeback.then(|| "!".to_string()),
        },
        Arm64OperandType::Fp(value) => Operand::Other(format!("#{value}")),
        Arm64OperandType::Cimm(value) => Operand::ImmediateSigned(value),
        Arm64OperandType::RegMrs(sysreg) | Arm64OperandType::RegMsr(sysreg) => {
            Operand::SystemRegister(format!("{sysreg:?}"))
        }
        Arm64OperandType::Pstate(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::Sys(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::Prefetch(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::Barrier(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::SVCR(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::SMEIndex(value) => Operand::Other(format!("{value:?}")),
        Arm64OperandType::Invalid => Operand::Other("<invalid>".to_string()),
    }
}

fn opcode_from_bytes(bytes: &[u8]) -> u32 {
    let mut opcode = [0u8; 4];
    for (index, byte) in bytes.iter().copied().enumerate().take(4) {
        opcode[index] = byte;
    }
    u32::from_le_bytes(opcode)
}
