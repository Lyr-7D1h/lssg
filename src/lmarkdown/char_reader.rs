use std::io::{self, Cursor, Read};

use super::parse_error::ParseError;

/// Character Reader with peeking functionality
pub struct CharReader<R> {
    inner: R,
    peek_buffer: Vec<u8>,
}

impl<R: Read> CharReader<R> {
    pub fn new(input: R) -> CharReader<R> {
        CharReader {
            inner: input,
            peek_buffer: vec![],
        }
    }

    /// Will try to fill the buffer until it is filled or eof is reached
    pub fn peek(&mut self, buf: &mut [u8]) -> Result<usize, ParseError> {
        // if buffer is already contained within peek buffer return it
        if self.peek_buffer.len() >= buf.len() {
            let mut cursor = Cursor::new(&mut self.peek_buffer);
            cursor.read(buf)?;
        }

        let read = (&mut self.inner)
            .take(buf.len() as u64)
            .read_to_end(&mut self.peek_buffer)?;
        let mut cursor = Cursor::new(&mut self.peek_buffer);
        cursor.read(buf)?;
        return Ok(read);
    }

    pub fn peek_string(&mut self, length: usize) -> Result<String, ParseError> {
        let mut buffer = vec![0; length];
        self.peek(&mut buffer)?;
        return Ok(String::from_utf8(buffer)
            .map_err(|_| ParseError::invalid("String contains invalid utf-8"))?);
    }

    pub fn peek_char(&mut self) -> Result<char, ParseError> {
        let mut buffer = [0; 1];
        self.peek(&mut buffer)?;
        return Ok(buffer[0] as char);
    }

    pub fn peek_until(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        let mut buffer = vec![0; 1];
        self.peek(&mut buffer)?;
        while op(buffer[buffer.len() - 1] as char) {
            buffer.resize(buffer.len() + 1, 0);
        }
        return Ok(String::from_utf8(buffer)?);
    }

    pub fn read_string(&mut self, length: usize) -> Result<String, ParseError> {
        let mut buffer = vec![0; length];
        self.read_exact(&mut buffer)?;
        return Ok(String::from_utf8(buffer)
            .map_err(|_| ParseError::invalid("String contains invalid utf-8"))?);
    }

    pub fn read_char(&mut self) -> Result<char, ParseError> {
        let mut buffer = [0; 1];
        self.read_exact(&mut buffer)?;
        return Ok(buffer[0] as char);
    }

    /// will read until eof or `op` is true
    pub fn read_until(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        let mut result = String::new();
        loop {
            let c = match self.read_char() {
                Ok(c) => c,
                Err(e) => match e.kind {
                    super::parse_error::ParseErrorKind::EndOfFile => break,
                    _ => return Err(e),
                },
            };
            if op(c) {
                break;
            }
            result.push(c);
        }
        return Ok(result);
    }
}

impl<R: Read> Read for CharReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.peek_buffer.is_empty() {
            return self.inner.read(buf);
        }

        let amount_from_peek = self.peek_buffer.len().min(buf.len());
        // NOTE: probably not very efficient
        let new_peek_buffer = self.peek_buffer.split_off(amount_from_peek);
        let mut read = Cursor::new(&mut self.peek_buffer).read(buf)?;
        self.peek_buffer = new_peek_buffer;

        if let Some(amount_from_inner) = buf.len().checked_sub(amount_from_peek) {
            let mut inner_buffer = vec![0; amount_from_inner];
            read += self.inner.read(&mut inner_buffer)?;
            for i in 0..amount_from_inner {
                buf[amount_from_peek + i] = inner_buffer[i]
            }
        }

        return Ok(read);
    }
}

#[test]
fn test_peek() -> Result<(), ParseError> {
    let mut reader = CharReader::new("This is a piece of text".as_bytes());
    assert_eq!(reader.peek_string(4)?, "This".to_owned());
    assert_eq!(reader.read_string(4)?, "This".to_owned());
    assert_eq!(reader.read_char()?, ' ');

    assert_eq!(reader.peek_string(3)?, "is ".to_owned());
    assert_eq!(reader.peek_string(2)?, "is".to_owned());

    assert_eq!(reader.peek_char()?, 'i');
    assert_eq!(reader.peek_string(2)?, "is".to_owned());
    assert_eq!(reader.read_string(10)?, "is a piece".to_owned());

    assert_eq!(reader.peek_char()?, ' ');
    assert_eq!(reader.read_char()?, ' ');
    assert_eq!(reader.read_string(7)?, "of text".to_owned());
    assert!(reader.read_char().is_err());
    Ok(())
}

#[test]
fn test_newline() -> Result<(), ParseError> {
    let mut reader = CharReader::new(
        "This is a
Very important test"
            .as_bytes(),
    );
    assert_eq!(reader.peek_string(11)?, "This is a\nV".to_owned());
    Ok(())
}
