use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptingError {
    pub(crate) message: String,
    pub(crate) source: String,
}

/// The crate's error type.
///
/// It is currently very primitive, and implements [`From`] for both
/// [io::Error](std::io::Error) and [`String`] to allow for some plugin
/// custom error reporting.
///
/// # User Message
/// If your plugin needs to report some user error, you can use [`Error::UserError`]
/// to show them a message, and hopefully let them know what went wrong.
///
/// Line location is planned in the future to improve diagnostics.
#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    #[serde(skip)] // never happens in testing
    /// An error that happened while trying to `emit` code.
    Io(std::io::Error),
    /// User Error with a friendly message to inform what went wrong.
    UserError { message: String },
    /// User-Facing error that happened during script execution.
    ScriptingError(Vec<ScriptingError>)
}

use Error::*;

impl From<ScriptingError> for Error {
    fn from(value: ScriptingError) -> Self {
        Self::ScriptingError(vec![value])
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UserError { message: a }, UserError { message: b }) => a == b,
            _ => false,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Io(io) => Display::fmt(io, f),
            UserError { message } => write!(f, "{}", message),
            ScriptingError (messages) => {
                write!(f, "Many errors:\n")?;
                for msg in messages {
                    writeln!(f, "`{}` in [{}]", msg.message, msg.source)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Io(io) => Some(io),
            UserError { .. } | &ScriptingError (_) => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Io(value)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        UserError { message: value }
    }
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        UserError {
            message: value.into(),
        }
    }
}
