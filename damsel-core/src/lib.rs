mod decode;
mod model;

pub use decode::DecodeError;
pub use model::{
    Annotation, Architecture, BinaryFormat, BinaryImage, DecodedInstruction, DisassemblyRequest,
    DisassemblyResult, DisassemblyTarget, DyldMetadata, Endianness, ExportedSymbol, Import,
    ObjcMetadata, Operand, Reference, Relocation, Section, Segment, SliceInfo, Symbol, SymbolKind,
};

pub fn decode_aarch64(
    bytes: &[u8],
    start_address: u64,
    max_instructions: Option<usize>,
) -> Result<Vec<DecodedInstruction>, DecodeError> {
    decode::decode_aarch64(bytes, start_address, max_instructions)
}
