use std::fmt::{ Debug, Display };

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    UserError { message: String },
}

use Error::*;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Io(io) => Display::fmt(io, f),
            UserError { message } => write!(f, "{}", message)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Io(io) => Some(io),
            UserError {..} => None,
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
