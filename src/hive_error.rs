use std::{error::Error, ffi::NulError, fmt::Display};

#[derive(Debug)]
pub enum HiveError {
    /// Propagated from std::ffi::NulError
    NulError(String),
    /// Unexpected system error occured, but all resources were cleaned up properly.
    CleanSystemError(InnerError),
    /// Unexpected system error occured, and memory cleanup may not have executed properly.
    /// Upon receiving this error, manual intervention might be necessary.
    DirtySystemError(InnerError),
}

#[derive(Debug)]
pub struct InnerError {
    os_error: Box<dyn Error>,
    message: String,
}

impl Display for HiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            HiveError::NulError(msg) => write!(f, "{msg}"),
            HiveError::CleanSystemError(err) => {
                write!(f, "{}. OS Error: {}", err.message, err.os_error)
            }
            HiveError::DirtySystemError(err) => {
                write!(f, "{}. OS Error: {}", err.message, err.os_error)
            }
        };
        msg
    }
}

impl From<NulError> for HiveError {
    fn from(_: NulError) -> Self {
        Self::NulError(String::from("std::ffi::NulError"))
    }
}

impl HiveError {
    pub(super) fn new_clean(message: impl ToString) -> Self {
        let inner = InnerError {
            os_error: Box::new(std::io::Error::last_os_error()),
            message: message.to_string(),
        };
        Self::CleanSystemError(inner)
    }
    pub(super) fn new_dirty(message: impl ToString) -> Self {
        let inner = InnerError {
            os_error: Box::new(std::io::Error::last_os_error()),
            message: message.to_string(),
        };
        Self::DirtySystemError(inner)
    }
}

impl Error for HiveError {}
