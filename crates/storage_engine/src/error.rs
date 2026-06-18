use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum EngineError {
    Io(std::io::Error),

    InvalidPageSize { expected: usize, actual: usize },

    KeyNotFound,

    ClockWentBackwards,
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::InvalidPageSize { expected, actual } => {
                write!(f, "invalid page size: expected {expected}, got {actual}")
            }
            Self::KeyNotFound => write!(f, "key was not found"),
            Self::ClockWentBackwards => write!(f, "system clock went backwards"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub type Result<T> = std::result::Result<T, EngineError>;
