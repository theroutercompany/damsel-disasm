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
    #[error("unsupported input kind: {0}")]
    UnsupportedInputKind(String),
    #[error("unsupported file kind: {0}")]
    UnsupportedFileKind(String),
    #[error("unsupported architecture: {0}")]
    UnsupportedArchitecture(String),
    #[error("unsupported shared cache architecture: {0}")]
    UnsupportedSharedCacheArchitecture(String),
    #[error(
        "unsupported thin architecture (cputype={cputype:#x}, subtype={cpusubtype:#x}); only arm64/arm64e are supported"
    )]
    UnsupportedThinArchitecture { cputype: u32, cpusubtype: u32 },
    #[error("universal binary does not contain an arm64/arm64e slice")]
    MissingArm64SliceInUniversal,
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
    #[error("malformed shared cache: {0}")]
    MalformedSharedCache(String),
    #[error("incomplete shared cache set: {0}")]
    IncompleteSharedCacheSet(String),
    #[error("cache image not found: {0}")]
    CacheImageNotFound(String),
    #[error("cache image is ambiguous: {0}")]
    CacheImageAmbiguous(String),
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
