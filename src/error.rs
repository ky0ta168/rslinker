use std::fmt;

#[derive(Debug)]
pub enum LinkerError {
    Io(std::io::Error),
    InvalidFormat(String),
    UndefinedSymbol(String),
    DuplicateSymbol(String),
    EntryPointNotFound(String),
    InvalidAlignment(String),
    InvalidArgument(String),
}

impl fmt::Display for LinkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::InvalidFormat(s) => write!(f, "Invalid format: {s}"),
            Self::UndefinedSymbol(s) => write!(f, "Undefined symbol: {s}"),
            Self::DuplicateSymbol(s) => write!(f, "Duplicate symbol: {s}"),
            Self::EntryPointNotFound(s) => write!(f, "Entry point not found: {s}"),
            Self::InvalidAlignment(s) => write!(f, "Invalid alignment: {s}"),
            Self::InvalidArgument(s) => write!(f, "Invalid argument: {s}"),
        }
    }
}

impl std::error::Error for LinkerError {}

impl From<std::io::Error> for LinkerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, LinkerError>;
