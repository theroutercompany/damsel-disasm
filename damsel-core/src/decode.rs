use crate::{Annotation, DecodedInstruction, Operand, Reference};
use capstone::arch;
use capstone::arch::ArchDetail;
use capstone::arch::arm64::{Arm64Operand, Arm64OperandType};
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
            let operands = arm64_detail
                .operands()
                .map(|operand| map_operand(&cs, &mnemonic, arm64_detail.writeback(), &operand))
                .collect::<Vec<_>>();
            let mut references = derive_references(&mnemonic, arm64_detail.operands().collect());
            let mut annotations = Vec::new();

            if insn.address() == start_address {
                annotations.push(Annotation::Note("range start".to_string()));
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
                references,
                annotations,
            });
        }

        Ok(instructions)
    }
}

fn derive_references(mnemonic: &str, operands: Vec<Arm64Operand>) -> Vec<Reference> {
    let mut references = Vec::new();
    let lower = mnemonic.to_ascii_lowercase();

    for operand in operands {
        if let Arm64OperandType::Imm(value) = operand.op_type {
            if value >= 0 {
                let target = value as u64;
                let reference = if lower == "bl" {
                    Reference::Call { target }
                } else if lower == "adr" || lower == "adrp" {
                    Reference::Page { target }
                } else if lower.starts_with('b')
                    || lower.starts_with("cb")
                    || lower.starts_with("tb")
                {
                    Reference::Branch { target }
                } else {
                    Reference::Data { target }
                };
                references.push(reference);
            }
        }
    }

    references
}

fn map_operand(cs: &Capstone, mnemonic: &str, writeback: bool, operand: &Arm64Operand) -> Operand {
    let lower = mnemonic.to_ascii_lowercase();
    match operand.op_type {
        Arm64OperandType::Reg(register) => Operand::Register(
            cs.reg_name(register)
                .unwrap_or_else(|| format!("{register:?}")),
        ),
        Arm64OperandType::Imm(value) => {
            if (lower.starts_with('b') || lower == "adr" || lower == "adrp") && value >= 0 {
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
