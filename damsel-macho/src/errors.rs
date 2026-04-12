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
