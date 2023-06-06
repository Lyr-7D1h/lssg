use std::io::{Cursor, Read};

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

        // fill peek buffer
        (&mut self.inner)
            .take(buf.len() as u64)
            .read_to_end(&mut self.peek_buffer)?;
        // println!("{}", String::from_utf8(self.peek_buffer.clone()).unwrap());
        // read peek_buffer to buf
        let mut cursor = Cursor::new(&mut self.peek_buffer);
        let read = cursor.read(buf)?;
        return Ok(read);
    }

    /// Try to fill string with `length` bytes
    pub fn peek_string(&mut self, length: usize) -> Result<String, ParseError> {
        let mut buffer = vec![0; length];
        self.peek(&mut buffer)?;
        return Ok(String::from_utf8(buffer)
            .map_err(|_| ParseError::invalid("String contains invalid utf-8"))?);
    }

    /// Read a character
    pub fn peek_char(&mut self) -> Result<Option<char>, ParseError> {
        let mut buffer = [0; 1];
        let read = self.peek(&mut buffer)?;
        if read == 0 {
            return Ok(None);
        }
        return Ok(Some(buffer[0] as char));
    }

    /// Read a character, will error on eof
    pub fn peek_char_exact(&mut self) -> Result<char, ParseError> {
        let mut buffer = [0; 1];
        let read = self.peek(&mut buffer)?;
        if read == 0 {
            return Err(ParseError::eof("Reached eof when peeking char"));
        }
        return Ok(buffer[0] as char);
    }

    // pub fn peek_until(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
    //     let mut buffer = vec![0; 1];
    //     self.peek(&mut buffer)?;
    //     while op(buffer[buffer.len() - 1] as char) {
    //         buffer.resize(buffer.len() + 1, 0);
    //     }
    //     return Ok(String::from_utf8(buffer)?);
    // }

    pub fn read_string(&mut self, length: usize) -> Result<String, ParseError> {
        let mut buffer = vec![0; length];
        self.read_exact(&mut buffer)?;
        return Ok(String::from_utf8(buffer)
            .map_err(|_| ParseError::invalid("String contains invalid utf-8"))?);
    }

    pub fn read_char(&mut self) -> Result<Option<char>, ParseError> {
        let mut buffer = [0; 1];
        let read = self.read(&mut buffer)?;
        if read == 0 {
            return Ok(None);
        }
        return Ok(Some(buffer[0] as char));
    }
    pub fn read_char_exact(&mut self) -> Result<char, ParseError> {
        let mut buffer = [0; 1];
        self.read_exact(&mut buffer)?;
        return Ok(buffer[0] as char);
    }

    /// will read until eof or `op` is true
    pub fn read_until(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        let mut result = String::new();
        loop {
            let c = match self.peek_char()? {
                Some(c) => c,
                None => break,
            };
            if op(c) {
                break;
            }
            match self.read_char()? {
                Some(c) => result.push(c),
                None => break,
            }
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
    assert_eq!(reader.read_char_exact()?, ' ');

    assert_eq!(reader.peek_string(3)?, "is ".to_owned());
    assert_eq!(reader.peek_string(2)?, "is".to_owned());

    assert_eq!(reader.peek_char_exact()?, 'i');
    assert_eq!(reader.peek_string(2)?, "is".to_owned());
    assert_eq!(reader.read_string(10)?, "is a piece".to_owned());

    assert_eq!(reader.peek_char_exact()?, ' ');
    assert_eq!(reader.read_char_exact()?, ' ');
    assert_eq!(reader.read_string(7)?, "of text".to_owned());
    assert!(reader.read_char_exact().is_err());
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
