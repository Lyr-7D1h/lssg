use std::io::{Cursor, Read};

use super::parse_error::{ParseError, ParseErrorKind};

pub trait Readable: Read {}
impl<T: Read + AsRef<[u8]>> Readable for T {}

/// A little endian binary reader
pub struct CharReader<N: Readable> {
    cursor: Cursor<N>,
}

impl<N: Readable> CharReader<N> {
    pub fn new(bits: N) -> CharReader<N> {
        CharReader {
            cursor: Cursor::new(bits),
        }
    }

    /// End Of File, returns true if not more bytes can be read
    pub fn eof(&mut self) -> Result<bool, ParseError> {
        let pos = self.position();
        match self.read_char() {
            Ok(_) => {
                self.set_position(pos);
                Ok(false)
            }
            Err(e) => {
                // if an io error occurs we assume no more bytes can be read, aka eof
                if e.kind == ParseErrorKind::Io {
                    self.set_position(pos);
                    return Ok(true);
                }
                return Err(e);
            }
        }
    }

    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    pub fn set_position(&mut self, position: u64) {
        self.cursor.set_position(position);
        // self.cursor.seek(SeekFrom::Current(position)).unwrap();
    }

    pub fn peek_string(&mut self, length: usize) -> Option<String> {
        let pos = self.position();
        let result = self.read_string(length).ok();
        self.set_position(pos);
        return result;
    }

    pub fn read_until(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        let mut result = String::new();
        let mut c = self.read_char()?;
        while op(c) {
            result.push(c);
            c = self.read_char()?;
        }
        return Ok(result);
    }

    pub fn read_string(&mut self, length: usize) -> Result<String, ParseError> {
        let mut buffer = vec![0; length];
        self.cursor.read_exact(&mut buffer)?;
        return Ok(String::from_utf8(buffer)
            .map_err(|_| ParseError::invalid("String contains invalid utf-8"))?);
    }

    pub fn peek_char(&mut self) -> Option<char> {
        let pos = self.position();
        let result = self.read_char().ok();
        self.set_position(pos);
        return result;
    }

    pub fn read_char(&mut self) -> Result<char, ParseError> {
        let mut buffer = [0; 4];
        self.cursor.read_exact(&mut buffer)?;
        return Ok(char::from_u32(u32::from_le_bytes(buffer))
            .ok_or(ParseError::invalid("Invalid character"))?);
    }
}
