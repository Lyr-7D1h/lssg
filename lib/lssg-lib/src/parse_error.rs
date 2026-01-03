use std::{
    error, fmt,
    io::{self},
    num::TryFromIntError,
    string,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    Io,
    EndOfFile,
    InvalidInput,
    Unsupported,
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub message: String,
    pub context: String,
}

impl ParseError {
    pub fn new<S: Into<String>>(message: S, kind: ParseErrorKind) -> ParseError {
        ParseError {
            message: message.into(),
            kind,
            context: String::new(),
        }
    }

    pub fn invalid<S: Into<String>>(message: S) -> ParseError {
        Self::new(message, ParseErrorKind::InvalidInput)
    }

    pub fn unsupported<S: Into<String>>(message: S) -> ParseError {
        Self::new(message, ParseErrorKind::Unsupported)
    }

    pub fn eof<S: Into<String>>(message: S) -> ParseError {
        Self::new(message, ParseErrorKind::EndOfFile)
    }
}

impl error::Error for ParseError {}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error while parsing file {}. \n{}",
            self.message,
            self.context
        )
    }
}
impl From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::UnexpectedEof => Self::eof(error.to_string()),
            _ => Self::new(format!("Failed to read input: {error}"), ParseErrorKind::Io),
        }
    }
}
impl From<TryFromIntError> for ParseError {
    fn from(error: TryFromIntError) -> Self {
        Self::new(
            format!("Failed to read input: {error}"),
            ParseErrorKind::InvalidInput,
        )
    }
}
impl From<string::FromUtf8Error> for ParseError {
    fn from(value: string::FromUtf8Error) -> Self {
        Self::invalid(format!("Invalid utf-8 string found: '{value}'"))
    }
}
