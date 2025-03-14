use std::io::{self, BufRead, BufReader, Read};

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

    pub fn from_string(input: &String) -> CharReader<&[u8]> {
        CharReader {
            reader: BufReader::<&[u8]>::new(&[]),
            buffer: input.chars().collect(),
            has_read: false,
        }
    }

    pub fn has_read(&self) -> bool {
        self.has_read
    }

    pub fn set_has_read(&mut self, has_read: bool) {
        self.has_read = has_read
    }

    /// Will try to fill the buffer until it is filled or eof is reached
    fn try_fill(&mut self, min: usize) -> Result<(), io::Error> {
        if min > self.buffer.len() {
            let mut bytes = vec![];
            while 0 != self.reader.read_until(b'\n', &mut bytes)? && min > self.buffer.len() {}
            // println!("B {bytes:?}");
            self.buffer.extend(
                String::from_utf8(bytes)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, format!("Failed to parse utf-8: {e}"))
                    })?
                    .chars(),
            );
        }
        Ok(())
    }

    /// Read a character. `pos` is 0 indexed
    pub fn peek_char(&mut self, pos: usize) -> Result<Option<char>, io::Error> {
        self.try_fill(pos + 1)?;
        return Ok(self.buffer.get(pos).copied());
    }

    pub fn peek_string(&mut self, length: usize) -> Result<String, io::Error> {
        return self.peek_string_from(0, length);
    }

    // TODO(perf): return a &str[], a slice of the characters in buf. Currently not possible
    // because rust stores chars as 4 bytes meaning `a` looks like 0x6100, you can't have multiple
    // zero bytes in utf-8 strings so needs to be converted. Possible fix by implementing a utf-8
    // reader storing only bytes and iterating over it.
    //
    /// Try to fill string with `length` bytes
    pub fn peek_string_from(&mut self, pos: usize, length: usize) -> Result<String, io::Error> {
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

    /// peek until \n or eof is reached
    pub fn peek_line(&mut self) -> Result<String, io::Error> {
        return self.peek_line_from(0);
    }

    /// peek until \n or eof is reached
    pub fn peek_line_from(&mut self, pos: usize) -> Result<String, io::Error> {
        let mut i = pos;
        let mut result = String::new();
        while let Some(c) = self.peek_char(i)? {
            if c == '\n' {
                break;
            }
            result.push(c);
            i += 1;
        }
        return Ok(result);
    }
    /// returns None if EOF is reached, to prevent false positives
    pub fn peek_until_exclusive_from(
        &mut self,
        pos: usize,
        op: fn(char) -> bool,
    ) -> Result<Option<String>, io::Error> {
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

        let string = self.peek_string_from(pos, i - pos)?;
        return Ok(Some(string));
    }

    /// returns None if EOF is reached, to prevent false positives
    pub fn peek_until_inclusive(
        &mut self,
        op: fn(char) -> bool,
    ) -> Result<Option<String>, io::Error> {
        return self.peek_until_inclusive_from(0, op);
    }

    /// returns None if EOF is reached, to prevent false positives
    pub fn peek_until_inclusive_from(
        &mut self,
        pos: usize,
        op: fn(char) -> bool,
    ) -> Result<Option<String>, io::Error> {
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

    pub fn peek_until_match_exclusive_from(
        &mut self,
        pos: usize,
        pattern: &str,
    ) -> Result<Option<String>, io::Error> {
        let chars: Vec<char> = pattern.chars().collect();

        let mut i = pos;
        'outer: loop {
            // check if match
            for (mi, mc) in chars.iter().enumerate() {
                let c = match self.peek_char(i + mi)? {
                    Some(c) => c,
                    None => return Ok(None), // eof
                };
                // no match break
                if c != *mc {
                    i += 1;
                    continue 'outer;
                }
            }
            // all characters matched
            break;
        }

        let string = self.peek_string_from(pos, i - pos)?;
        return Ok(Some(string));
    }

    /// Peek until matches or return None when not found
    pub fn peek_until_match_inclusive(
        &mut self,
        pattern: &str,
    ) -> Result<Option<String>, io::Error> {
        return self.peek_until_match_inclusive_from(0, pattern);
    }

    pub fn peek_until_match_inclusive_from(
        &mut self,
        pos: usize,
        pattern: &str,
    ) -> Result<Option<String>, io::Error> {
        let chars: Vec<char> = pattern.chars().collect();

        let mut i = pos;
        'outer: loop {
            // check if match
            for (mi, mc) in chars.iter().enumerate() {
                let c = match self.peek_char(i + mi)? {
                    Some(c) => c,
                    None => return Ok(None), // eof
                };
                // no match break
                if c != *mc {
                    i += 1;
                    continue 'outer;
                }
            }
            // all characters matched
            i += chars.len();
            break;
        }

        let string = self.peek_string_from(pos, i - pos)?;
        return Ok(Some(string));
    }

    pub fn consume(&mut self, length: usize) -> Result<Option<()>, io::Error> {
        self.has_read = true;
        self.try_fill(length)?;
        if self.buffer.len() == 0 {
            return Ok(None);
        }
        self.buffer.drain(0..length);
        Ok(Some(()))
    }

    pub fn consume_char(&mut self) -> Result<Option<char>, io::Error> {
        self.has_read = true;
        self.try_fill(1)?;
        if self.buffer.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(self.buffer.drain(0..1).collect::<Vec<char>>()[0]))
        }
    }

    /// Read {length} bytes returning a smaller string on EOF
    pub fn consume_string(&mut self, length: usize) -> Result<String, io::Error> {
        self.has_read = true;
        self.try_fill(length)?;
        return Ok(self
            .buffer
            .drain(0..length.min(self.buffer.len()))
            .collect());
    }

    /// Will read until eof or `op` is true including the true match
    pub fn consume_until_inclusive(&mut self, op: fn(char) -> bool) -> Result<String, io::Error> {
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
    pub fn consume_until_exclusive(&mut self, op: fn(char) -> bool) -> Result<String, io::Error> {
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

    /// stop consuming by pattern, if eof returns whatever is captured
    pub fn consume_until_match_inclusive(&mut self, pattern: &str) -> Result<String, io::Error> {
        let mut result = self.consume_string(pattern.len())?;
        if result.len() < pattern.len() {
            return Ok(result);
        }
        loop {
            if &result[result.len() - pattern.len()..] == pattern {
                break;
            }
            match self.consume_char()? {
                Some(c) => result.push(c),
                None => break,
            };
        }
        return Ok(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propegation() -> Result<(), io::Error> {
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
    fn test_newline() -> Result<(), io::Error> {
        let mut reader = CharReader::new(
            "This is a
Very important test"
                .as_bytes(),
        );
        assert_eq!(reader.peek_string(11)?, "This is a\nV".to_owned());
        assert_eq!(reader.consume_string(11)?, "This is a\nV".to_owned());
        Ok(())
    }

    #[test]
    fn test_peek_until_match_inclusive() {
        let input = "<!---->";
        let mut reader = CharReader::new(input.as_bytes());
        assert_eq!(
            reader.peek_until_match_inclusive("-->").unwrap(),
            Some(input.to_owned())
        );
        assert_eq!(
            reader
                .peek_until_match_exclusive_from(0, "-->")
                .unwrap()
                .unwrap(),
            "<!--".to_string()
        );
        assert_eq!(
            reader
                .peek_until_match_exclusive_from(4, "-->")
                .unwrap()
                .unwrap(),
            "".to_string()
        );

        let input = "   **";
        let mut reader = CharReader::new(input.as_bytes());
        assert_eq!(
            "*".to_string(),
            reader
                .peek_until_match_inclusive_from(3, "*")
                .unwrap()
                .unwrap(),
        );
    }

    #[test]
    fn test_consume_until_match_inclusive() {
        let input = "<!---->";
        let mut reader = CharReader::new(input.as_bytes());
        assert_eq!(
            reader.consume_until_match_inclusive("-->").unwrap(),
            input.to_owned()
        );
        assert_eq!(reader.consume(1).unwrap(), None);

        let input = "some string";
        let mut reader = CharReader::new(input.as_bytes());
        assert_eq!(
            reader.consume_until_match_inclusive("str").unwrap(),
            "some str".to_string()
        );
        assert_eq!(reader.consume_string(8).unwrap(), "ing".to_string());
    }
}

