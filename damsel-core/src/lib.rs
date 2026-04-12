mod decode;
mod model;

pub use decode::DecodeError;
pub use decode::decode_aarch64_v2;
pub use decode::decode_aarch64_with_limit;
pub use model::{
    Annotation, Architecture, BinaryFormat, BinaryImage, BinaryImageBuilder,
    BinaryImageValidationError, BinarySource, DecodedInstruction, DisassemblyLimit,
    DisassemblyOptions, DisassemblyRequest, DisassemblyRequestV2, DisassemblyResult,
    DisassemblyResultV2, DisassemblyStopReason, DisassemblyTarget, DyldMetadata, Endianness,
    ExportFlags, ExportKind, ExportRecord, ExportedSymbol, Import, ImportBindingKind,
    ImportBindingRecord, ImportBindingSource, ObjcCategoryRecord, ObjcClassRecord, ObjcIvarRecord,
    ObjcCategoryRecordSource, ObjcMetadata, ObjcMethodOwnerKind, ObjcMethodRecord, ObjcNameSource,
    ObjcPointerKind, ObjcPointerRef, ObjcPropertyRecord, ObjcProtocolRecord, ObjcSelectorSource,
    Operand, Platform, RecoveredValue, RecoveredValueKind, RecoveredValueSource, Reference, Relocation,
    RelocationEncodingId, RelocationKindId, RelocationTargetKind, Section, SectionKind, Segment,
    SliceDescriptor, SliceInfo, StubEntry, StubHelperEntry, StubKind, Symbol, SymbolKind,
};

pub fn decode_aarch64(
    bytes: &[u8],
    start_address: u64,
    max_instructions: Option<usize>,
) -> Result<Vec<DecodedInstruction>, DecodeError> {
    decode::decode_aarch64(bytes, start_address, max_instructions)
}
