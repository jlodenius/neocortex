use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum CortexError {
    /// Unexpected system error occured, but all resources were cleaned up properly.
    CleanSystem(InnerError),
    /// Unexpected system error occured, and memory cleanup may not have executed properly.
    /// Upon receiving this error, manual intervention might be necessary.
    DirtySystem(InnerError),
}

#[derive(Debug)]
pub struct InnerError {
    os_error: Box<dyn Error>,
    message: String,
}

impl Display for CortexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CortexError::CleanSystem(err) => {
                write!(f, "{}. OS Error: {}", err.message, err.os_error)
            }
            CortexError::DirtySystem(err) => {
                write!(f, "{}. OS Error: {}", err.message, err.os_error)
            }
        }
    }
}

impl CortexError {
    fn new_inner_error(message: impl ToString) -> InnerError {
        InnerError {
            os_error: Box::new(std::io::Error::last_os_error()),
            message: message.to_string(),
        }
    }
    pub(super) fn new_clean(message: impl ToString) -> Self {
        let inner = Self::new_inner_error(message);
        Self::CleanSystem(inner)
    }
    pub(super) fn new_dirty(message: impl ToString) -> Self {
        let inner = Self::new_inner_error(message);
        Self::DirtySystem(inner)
    }
}

impl Error for CortexError {}
