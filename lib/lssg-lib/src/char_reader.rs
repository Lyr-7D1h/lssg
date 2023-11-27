use std::{
    io::{BufRead, BufReader, Cursor, Read},
    mem::transmute,
    str::Chars,
};

use super::parse_error::ParseError;

/// Character Reader with peeking functionality
/// It buffers lines internally. So if you parse a stream with that never ends with \n it will all
/// be put into memory
pub struct CharReader<R> {
    reader: BufReader<R>,
    buffer: Vec<char>,
    has_read: bool,
}

impl<R: Read> CharReader<R> {
    pub fn new(input: R) -> CharReader<R> {
        let reader = BufReader::new(input);
        CharReader {
            reader,
            buffer: vec![],
            has_read: false,
        }
    }

    pub fn from_string<'n>(input: &String) -> CharReader<&'n [u8]> {
        CharReader {
            reader: BufReader::new(&[]),
            buffer: input.chars().collect(),
            has_read: false,
        }
    }

    pub fn has_read(&self) -> bool {
        self.has_read
    }

    /// Will try to fill the buffer until it is filled or eof is reached
    fn try_fill(&mut self, min: usize) -> Result<(), ParseError> {
        if min > self.buffer.len() {
            let mut bytes = vec![];
            while 0 != self.reader.read_until(b'\n', &mut bytes)? && min > self.buffer.len() {}
            // println!("B {bytes:?}");
            self.buffer.extend(String::from_utf8(bytes)?.chars());
        }
        Ok(())
    }

    /// Read a character. `pos` is 0 indexed
    pub fn peek_char(&mut self, pos: usize) -> Result<Option<char>, ParseError> {
        self.try_fill(pos + 1)?;
        return Ok(self.buffer.get(pos).copied());
    }

    pub fn peek_string(&mut self, length: usize) -> Result<String, ParseError> {
        return self.peek_string_from(0, length);
    }

    // TODO(perf): return a &str[], a slice of the characters in buf. Currently not possible
    // because rust stores chars as 4 bytes meaning `a` looks like 0x6100, you can't have multiple
    // zero bytes in utf-8 strings so needs to be converted. Possible fix by implementing a utf-8
    // reader storing only bytes and iterating over it.
    //
    /// Try to fill string with `length` bytes
    pub fn peek_string_from(&mut self, pos: usize, length: usize) -> Result<String, ParseError> {
        self.try_fill(pos + length)?;
        let stop = (pos + length).min(self.buffer.len());
        let chars = &self.buffer[pos..stop];

        // have to convert characters to utf-8 because by default each char has 4 bytes.
        let mut bytes: Vec<u8> = Vec::with_capacity(chars.len() * 4);
        for &c in chars {
            bytes.extend(c.encode_utf8(&mut [0; 4]).bytes());
        }
        let string = unsafe { String::from_utf8_unchecked(bytes) };
        return Ok(string);
    }

    // TODO should return usize?
    pub fn peek_until(&mut self, op: fn(char) -> bool) -> Result<Option<String>, ParseError> {
        return self.peek_until_from(0, op);
    }

    pub fn peek_until_from(
        &mut self,
        pos: usize,
        op: fn(char) -> bool,
    ) -> Result<Option<String>, ParseError> {
        let mut i = pos;
        loop {
            match self.peek_char(i)? {
                Some(c) => {
                    if op(c) {
                        break;
                    }
                }
                None => return Ok(None),
            }
            i += 1;
        }

        let string = self.peek_string_from(pos, i - pos + 1)?;
        return Ok(Some(string));
    }

    /// Peek until matches or return None when not found
    pub fn peek_until_match_inclusive(
        &mut self,
        pattern: &str,
    ) -> Result<Option<String>, ParseError> {
        let chars: Vec<char> = pattern.chars().collect();

        let mut char_i = 0;
        let mut i = 0;
        loop {
            let c = match self.peek_char(i)? {
                Some(c) => c,
                None => return Ok(None), // eof
            };

            // iterate where we left off
            if chars[char_i] == c {
                char_i += 1;
                if char_i == chars.len() {
                    break;
                }
            } else {
                char_i = 0;
            }
            i += 1;
        }

        let string = self.peek_string(i + 1)?;
        return Ok(Some(string));
    }

    pub fn consume(&mut self, length: usize) -> Result<Option<()>, ParseError> {
        self.has_read = true;
        self.try_fill(length)?;
        if self.buffer.len() == 0 {
            return Ok(None);
        }
        self.buffer.drain(0..length);
        Ok(Some(()))
    }

    pub fn consume_char(&mut self) -> Result<Option<char>, ParseError> {
        self.has_read = true;
        self.try_fill(1)?;
        if self.buffer.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(self.buffer.drain(0..1).collect::<Vec<char>>()[0]))
        }
    }

    /// Read {length} bytes returning a smaller string on EOF
    pub fn consume_string(&mut self, length: usize) -> Result<String, ParseError> {
        self.has_read = true;
        self.try_fill(length)?;
        return Ok(self
            .buffer
            .drain(0..length.min(self.buffer.len()))
            .collect());
    }

    /// Will read until eof or `op` is true including the true match
    pub fn consume_until_inclusive(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        self.has_read = true;
        let mut result = String::new();
        loop {
            match self.consume_char()? {
                Some(c) => {
                    result.push(c);
                    if op(c) {
                        break;
                    }
                }
                None => {
                    break;
                }
            };
        }
        return Ok(result);
    }

    /// will read until eof or `op` is true excluding the character that matched
    pub fn consume_until_exclusive(&mut self, op: fn(char) -> bool) -> Result<String, ParseError> {
        self.has_read = true;
        let mut i = 0;
        loop {
            match self.peek_char(i)? {
                Some(c) => {
                    if op(c) {
                        break;
                    }
                }
                None => break,
            };
            i += 1;
        }
        return self.consume_string(i);
    }

    pub fn consume_until_match_inclusive(&mut self, pattern: &str) -> Result<String, ParseError> {
        self.has_read = true;
        // TODO refactor
        let chars: Vec<char> = pattern.chars().collect();
        let mut char_i = 0;

        let mut result = String::new();
        loop {
            let c = match self.consume_char()? {
                Some(c) => c,
                None => break,
            };
            result.push(c);
            if c == chars[char_i] {
                char_i += 1;
                if char_i == chars.len() {
                    break;
                }
            } else {
                char_i = 0;
            }
        }
        return Ok(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propegation() -> Result<(), ParseError> {
        let mut reader = CharReader::new("This is a piece of text".as_bytes());
        assert_eq!(reader.peek_string(4)?, "This".to_owned());
        assert_eq!(reader.peek_char(3)?, Some('s'));

        assert_eq!(reader.consume_string(5)?, "This ".to_owned());

        assert_eq!(reader.peek_string(3)?, "is ".to_owned());
        assert_eq!(reader.peek_string(2)?, "is".to_owned());

        assert_eq!(reader.consume_string(11)?, "is a piece ".to_owned());
        assert_eq!(reader.peek_string(3)?, "of ".to_owned());
        assert_eq!(reader.peek_char(1)?, Some('f'));
        assert_eq!(reader.consume_char()?, Some('o'));
        assert_eq!(reader.peek_char(1)?, Some(' '));

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
        assert_eq!(reader.consume_string(11)?, "This is a\nV".to_owned());
        Ok(())
    }
}
