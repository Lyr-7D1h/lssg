use core::fmt;
use std::{fmt::Display, io};

use crate::parser::parse_error::ParseError;

#[derive(Debug)]
pub enum LssgErrorKind {
    ParseError,
    Regex,
    /// Render error
    Render,
    /// Error with the sitemap
    Sitemap,
    Io,
}

#[derive(Debug)]
pub struct LssgError {
    message: String,
    context: Option<String>,
    kind: LssgErrorKind,
}
impl LssgError {
    pub fn new(message: &str, kind: LssgErrorKind) -> LssgError {
        LssgError {
            message: message.to_string(),
            kind,
            context: None,
        }
    }

    pub fn sitemap(message: &str) -> LssgError {
        Self::new(message, LssgErrorKind::Sitemap)
    }
    pub fn render(message: &str) -> LssgError {
        Self::new(message, LssgErrorKind::Render)
    }

    pub fn io(message: &str) -> LssgError {
        Self::new(message, LssgErrorKind::Io)
    }

    pub fn with_context(&mut self, context: String) {
        self.context = Some(context);
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
impl From<regex::Error> for LssgError {
    fn from(error: regex::Error) -> Self {
        Self::new(&error.to_string(), LssgErrorKind::Regex)
    }
}

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
