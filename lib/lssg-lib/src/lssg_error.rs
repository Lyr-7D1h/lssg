use core::fmt;
use std::io;

use zip::result::ZipError;

use crate::parse_error::ParseError;

#[derive(Debug)]
pub enum LssgErrorKind {
    ParseError,
    /// Render error
    Render,
    Request,
    /// Error with the sitetree
    SiteTree,
    Io,
}

#[derive(Debug)]
pub struct LssgError {
    message: String,
    context: Option<String>,
    #[allow(dead_code)]
    kind: LssgErrorKind,
}
impl LssgError {
    pub fn new<S: Into<String>>(message: S, kind: LssgErrorKind) -> LssgError {
        LssgError {
            message: message.into(),
            kind,
            context: None,
        }
    }

    pub fn sitetree<S: Into<String>>(message: S) -> LssgError {
        Self::new(message, LssgErrorKind::SiteTree)
    }

    pub fn render<S: Into<String>>(message: S) -> LssgError {
        Self::new(message, LssgErrorKind::Render)
    }

    pub fn io<S: Into<String>>(message: S) -> LssgError {
        Self::new(message, LssgErrorKind::Io)
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}
impl From<ParseError> for LssgError {
    fn from(error: ParseError) -> Self {
        Self::new(&error.to_string(), LssgErrorKind::ParseError)
    }
}
impl From<io::Error> for LssgError {
    fn from(error: io::Error) -> Self {
        Self::new(&error.to_string(), LssgErrorKind::Io)
    }
}
impl From<reqwest::Error> for LssgError {
    fn from(error: reqwest::Error) -> Self {
        Self::new(&error.to_string(), LssgErrorKind::Request)
    }
}
impl From<ZipError> for LssgError {
    fn from(error: ZipError) -> Self {
        Self::new(&error.to_string(), LssgErrorKind::Io)
    }
}
impl std::error::Error for LssgError {}

impl fmt::Display for LssgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let context = if let Some(context) = &self.context {
            format!("with context '{context}'")
        } else {
            "".into()
        };
        write!(
            f,
            "Error when generating static files: '{}' {context}",
            self.message
        )
    }
}
