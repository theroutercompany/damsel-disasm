use thiserror::Error;

#[derive(Debug, Error)]
pub enum MachoError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("object parse error: {0}")]
    Object(#[from] object::Error),
    #[error("mach-o parse error: {0}")]
    Goblin(#[from] goblin::error::Error),
    #[error("decode error: {0}")]
    Decode(#[from] damsel_core::DecodeError),
    #[error("unsupported file kind: {0}")]
    UnsupportedFileKind(String),
    #[error("unsupported architecture: {0}")]
    UnsupportedArchitecture(String),
    #[error(
        "slice range is out of bounds (offset={offset:#x}, size={size:#x}, file_len={file_len:#x})"
    )]
    SliceOutOfBounds {
        offset: u64,
        size: u64,
        file_len: u64,
    },
    #[error("malformed fat binary: {0}")]
    MalformedFatBinary(String),
    #[error("malformed dyld chained-fixups payload: {0}")]
    MalformedDyldPayload(String),
    #[error(
        "linkedit payload range is out of bounds (offset={offset:#x}, size={size:#x}, file_len={file_len:#x})"
    )]
    LinkeditRangeOutOfBounds {
        offset: u64,
        size: u64,
        file_len: u64,
    },
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("section not found: {0}")]
    SectionNotFound(String),
    #[error("address {0:#x} is not mapped")]
    AddressNotMapped(u64),
    #[error("section `{0}` has no file-backed contents")]
    SectionHasNoFileData(String),
}

pub type Result<T> = std::result::Result<T, MachoError>;
