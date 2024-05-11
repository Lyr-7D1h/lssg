// use std::{collections::HashMap, io::Read};
//
// use virtual_dom::Html;
//
// use crate::{char_reader::CharReader, parse_error::ParseError};
//
// use super::{read_inline_tokens, read_tokens, sanitize_text, Token};
//
// /// from virtual_dom::html
// pub fn html_attributes(start_tag_content: &str) -> Result<HashMap<String, String>, ParseError> {
//     let chars: Vec<char> = start_tag_content.chars().collect();
//     let mut attributes = HashMap::new();
//     let mut key = String::new();
//     let mut value = String::new();
//     let mut in_value = false;
//     let mut i = 0;
//     while i < chars.len() {
//         match chars[i] {
//             ' ' if in_value == false => {
//                 if key.len() > 0 {
//                     attributes.insert(key, value);
//                     key = String::new();
//                     value = String::new();
//                     in_value = false;
//                 }
//             }
//             '=' => match chars.get(i + 1) {
//                 Some('"') | Some('\'') => {
//                     i += 1;
//                     in_value = true
//                 }
//                 _ => {}
//             },
//             '\'' | '"' if in_value => in_value = false,
//             c => {
//                 if in_value {
//                     value.push(c)
//                 } else {
//                     key.push(c)
//                 }
//             }
//         }
//         i += 1;
//     }
//     if key.len() > 0 {
//         attributes.insert(key, value);
//     }
//
//     Ok(attributes)
// }
//
// struct Tokenizer<R> {
//     reader: CharReader<R>,
// }
//
// impl<R: Read> Tokenizer<R> {
//     pub fn new(reader: &mut CharReader<R>) -> Tokenizer<R> {
//         Tokenizer { reader }
//     }
//
//     /// from virtual_dom::html
//     fn html_element(self) -> Result<Option<(String, HashMap<String, String>, String)>, ParseError> {
//         if let Some('<') = self.reader.peek_char(0)? {
//             if let Some(start_tag) = self.reader.peek_until_exclusive_from(1, |c| c == '>')? {
//                 // get html tag
//                 let mut tag = String::new();
//                 for c in start_tag.chars() {
//                     match c {
//                         ' ' => break,
//                         '\n' => break,
//                         _ => tag.push(c),
//                     }
//                 }
//
//                 let end_tag = format!("</{tag}>");
//                 if let Some(html_block) = self
//                     .reader
//                     .peek_until_match_exclusive_from(2 + start_tag.len(), &end_tag)?
//                 {
//                     // <{start_tag}>
//                     self.reader.consume(start_tag.len() + 2)?;
//
//                     let attributes = html_attributes(&start_tag[tag.len()..start_tag.len()])?;
//
//                     let content = self.reader.consume_string(html_block.len())?;
//                     self.reader.consume(end_tag.len())?;
//
//                     return Ok(Some((tag, attributes, content)));
//                 }
//             }
//         }
//         Ok(None)
//     }
//
//     /// from virtual_dom::html
//     fn html_comment(self) -> Result<Option<Html>, ParseError> {
//         if "<!--" == self.reader.peek_string(4)? {
//             if let Some(text) = self.reader.peek_until_match_exclusive_from(4, "-->")? {
//                 self.reader.consume(4)?; // skip start
//                 let text = self.reader.consume_string(text.len())?;
//                 self.reader.consume(3)?; // skip end
//                 return Ok(Some(Html::Comment { text }));
//             }
//         }
//
//         Ok(None)
//     }
//
//     /// Allow for certain block tokens inside html
//     pub fn read_inline_html_tokens(
//         reader: &mut CharReader<impl Read>,
//     ) -> Result<Vec<Token>, ParseError> {
//         let mut tokens = vec![];
//
//         while let Some(_) = self.reader.peek_char(0)? {
//             if let Some(heading) = heading(reader)? {
//                 tokens.push(heading)
//             }
//             let text = self.reader.consume_until_match_inclusive("\n\n")?;
//             let text = sanitize_text(text);
//             tokens.append(&mut read_inline_tokens(&mut CharReader::new(
//                 text.as_bytes(),
//             ))?);
//         }
//
//         Ok(tokens)
//     }
//
//     /// https://spec.commonmark.org/0.30/#indented-code-blocks
//     pub fn indented_code(
//         reader: &mut CharReader<impl Read>,
//         tokens: &Vec<Token>,
//         blank_line: bool,
//     ) -> Result<Option<Token>, ParseError> {
//         // can't interupt a paragraph if there wasn't a blank line
//         if let Some(Token::Paragraph { .. }) = tokens.last() {
//             if !blank_line {
//                 return Ok(None);
//             }
//         }
//
//         let mut text = String::new();
//         while "    " == self.reader.peek_string(4)? {
//             let line = self.reader.consume_until_inclusive(|c| c == '\n')?;
//             text.push_str(&line[4..line.len()]);
//         }
//         if text.len() == 0 {
//             return Ok(None);
//         }
//
//         return Ok(Some(Token::Code { info: None, text }));
//     }
//
//     /// https://spec.commonmark.org/0.30/#fenced-code-blocks
//     pub fn fenced_code(self) -> Result<Option<Token>, ParseError> {
//         if let Some(indent) = detect_char_with_ident(reader, |c| c == '~' || c == '`')? {
//             let fence_type = self.reader.peek_char(indent)?.unwrap();
//             let mut count_backticks = 1;
//             while let Some(c) = self.reader.peek_char(indent + count_backticks)? {
//                 if c != fence_type {
//                     break;
//                 }
//                 count_backticks += 1;
//             }
//
//             // must start with more than 3 of same fence_type
//             if !(count_backticks >= 3) {
//                 return Ok(None);
//             }
//
//             let info = self.reader.consume_until_inclusive(|c| c == '\n')?;
//             let info = &info[indent + count_backticks..info.len()];
//             let info = sanitize_text(info.to_string());
//
//             let mut text = String::new();
//             // add all content
//             'outer: loop {
//                 let line = self.reader.consume_until_inclusive(|c| c == '\n')?;
//                 if line.len() == 0 {
//                     break;
//                 }
//
//                 let chars: Vec<char> = line.chars().collect();
//                 // check if closing
//                 for i in 0..4 {
//                     match chars.get(i) {
//                         Some(c) if *c == fence_type => {
//                             // continue if all characters are not same as opening fence
//                             for j in i..i + count_backticks {
//                                 if let Some(c) = chars.get(j) {
//                                     if *c != fence_type {
//                                         break;
//                                     }
//                                 }
//                             }
//                             break 'outer;
//                         }
//                         Some(' ') => {}
//                         Some(_) | None => break,
//                     }
//                 }
//
//                 let mut pos = 0;
//                 for i in 0..indent {
//                     if chars[i] == ' ' {
//                         pos += 1;
//                     }
//                     break;
//                 }
//                 text += &line[pos..line.len()];
//             }
//
//             return Ok(Some(Token::Code {
//                 info: Some(info),
//                 text,
//             }));
//         }
//
//         return Ok(None);
//     }
//
//     /// https://spec.commonmark.org/0.30/#setext-heading
//     pub fn setext_heading(&mut self, tokens: &mut Vec<Token>) -> Result<Option<Token>, ParseError> {
//         if let Some(Token::Paragraph {
//             tokens: ptokens, ..
//         }) = tokens.last()
//         {
//             if let Some(pos) = detect_char_with_ident(reader, |c| c == '=')? {
//                 let line = self.reader.peek_line_from(pos)?;
//                 if line.len() >= 3 {
//                     for c in line.chars() {
//                         if c != '=' {
//                             return Ok(None);
//                         }
//                     }
//                     self.reader.consume_string(pos + line.len())?;
//                     let heading = Token::Heading {
//                         tokens: ptokens.clone(),
//                         depth: 1,
//                     };
//                     tokens.pop(); // remove paragraph
//                     return Ok(Some(heading));
//                 }
//             } else if let Some(pos) = detect_char_with_ident(reader, |c| c == '-')? {
//                 let line = self.reader.peek_line_from(pos)?;
//                 if line.len() >= 3 {
//                     for c in line.chars() {
//                         if c != '-' {
//                             return Ok(None);
//                         }
//                     }
//                     self.reader.consume_string(pos + line.len())?;
//                     let heading = Token::Heading {
//                         tokens: ptokens.clone(),
//                         depth: 2,
//                     };
//                     tokens.pop(); // remove paragraph
//                     return Ok(Some(heading));
//                 }
//             }
//         }
//         Ok(None)
//     }
//
//     pub fn thematic_break(&mut self) -> Result<Option<Token>, ParseError> {
//         if let Some(pos) = self.detect_char_with_ident(|c| c == '*' || c == '-' || c == '_')? {
//             let line = self.reader.peek_line_from(pos)?;
//             if let Some(pattern) = line.replace(" ", "").get(0..3) {
//                 if pattern == "***" || pattern == "---" || pattern == "___" {
//                     self.reader.consume_string(pos + line.len())?;
//                     return Ok(Some(Token::ThematicBreak));
//                 }
//             }
//         }
//         return Ok(None);
//     }
//
//     fn list_item_tokens(&mut self, ident: usize) -> Result<Vec<Token>, ParseError> {
//         // read the first line
//         let line = self.reader.consume_until_inclusive(|c| c == '\n')?;
//         let mut item_content = line[ident..line.len()].to_string();
//         loop {
//             let line = self.reader.peek_line()?;
//
//             if line.is_empty() {
//                 let line = self.reader.consume_string(line.len() + 1)?;
//                 // end
//                 if line.len() == 0 {
//                     break;
//                 }
//                 item_content.push_str(&line);
//             } else if line.starts_with(&" ".repeat(ident)) {
//                 let line = self.reader.consume_string(line.len() + 1)?;
//                 item_content.push_str(&line[ident..line.len()]);
//             } else {
//                 break;
//             }
//         }
//         let mut reader = CharReader::<&[u8]>::from_string(&item_content);
//         return read_tokens(&mut reader);
//     }
//     // TODO implement all specs (check for same usage of bullet enc.)
//     /// https://spec.commonmark.org/0.30/#list-items
//     pub fn bullet_list(self) -> Result<Option<Token>, ParseError> {
//         let mut items = vec![];
//
//         while let Some(pos) = self.detect_char_with_ident(|c| c == '-' || c == '+' || c == '*')? {
//             // by default n=1
//             let mut n = 0;
//             for offset in 1..5 {
//                 match self.reader.peek_char(pos + offset)? {
//                     Some(' ') => {}
//                     Some(_) => {
//                         n = offset - 1;
//                         break;
//                     }
//                     None => return Ok(None),
//                 }
//             }
//             // must have atleast one whitespace
//             if n == 0 {
//                 return Ok(None);
//             }
//
//             let ident = pos + n;
//
//             let tokens = self.list_item_tokens(ident)?;
//             items.push(tokens)
//         }
//
//         if items.len() == 0 {
//             return Ok(None);
//         }
//
//         return Ok(Some(Token::BulletList { items }));
//     }
//     // TODO implement all specs (check for same usage of bullet enc.)
//     /// https://spec.commonmark.org/0.30/#list-items
//     pub fn ordered_list(&self) -> Result<Option<Token>, ParseError> {
//         let mut items = vec![];
//         while let Some(mut pos) = self.detect_char_with_ident(|c| c.is_ascii_digit())? {
//             for i in 1..10 {
//                 // not more than 9 digits allowed
//                 if i == 10 {
//                     return Ok(None);
//                 }
//
//                 match self.reader.peek_char(pos + i)? {
//                     Some(c) if c.is_ascii_digit() => {}
//                     Some('.') | Some(')') => {
//                         pos = i + pos;
//                         break;
//                     }
//                     Some(_) | None => return Ok(None),
//                 }
//             }
//
//             // by default n=1
//             let mut n = 1;
//             for i in 1..5 {
//                 match self.reader.peek_char(pos + i)? {
//                     Some(' ') => {}
//                     Some(_) => {
//                         n = i;
//                         break;
//                     }
//                     None => return Ok(None),
//                 }
//             }
//
//             let ident = pos + n;
//
//             let tokens = list_item_tokens(reader, ident)?;
//             items.push(tokens)
//         }
//
//         if items.len() == 0 {
//             return Ok(None);
//         }
//
//         return Ok(Some(Token::OrderedList { items }));
//     }
//
//     /// ignore up to 4 space idententations returns at which position the match begins
//     fn detect_char_with_ident(&self, op: fn(c: char) -> bool) -> Result<Option<usize>, ParseError> {
//         for i in 0..4 {
//             match self.reader.peek_char(i)? {
//                 Some(c) if op(c) => return Ok(Some(i)),
//                 Some(' ') => {}
//                 Some(_) | None => return Ok(None),
//             }
//         }
//         return Ok(None);
//     }
//
//     /// Heading (#*{depth} {text})
//     pub fn heading(&self) -> Result<Option<Token>, ParseError> {
//         if let Some(pos) = self.detect_char_with_ident(|c| c == '#')? {
//             let chars: Vec<char> = self.reader.peek_string_from(pos, 7)?.chars().collect();
//             let mut depth: u8 = 0;
//             for c in chars {
//                 match c {
//                     ' ' => break,
//                     '#' => depth += 1,
//                     _ => return Ok(None),
//                 }
//             }
//             let text: String = sanitize_text(
//                 self.reader
//                     .consume_until_inclusive(|c| c == '\n')?
//                     .chars()
//                     .skip(depth as usize + 1)
//                     .collect(),
//             );
//             let tokens = read_inline_tokens(&mut CharReader::new(text.as_bytes()))?;
//
//             Ok(Some(Token::Heading { depth, tokens }))
//         } else {
//             Ok(None)
//         }
//     }
//
//     // https://spec.commonmark.org/0.30/#block-quotes
//     pub fn blockquote(self) -> Result<Option<Token>, ParseError> {
//         let mut lines = vec![];
//         'outer: loop {
//             for i in 0..3 {
//                 match self.reader.peek_char(i)? {
//                     Some('>') => {
//                         let line = self.reader.consume_until_inclusive(|c| c == '\n')?;
//                         let text = line[i + 1..line.len()].trim_start().to_string();
//                         lines.push(text);
//                         continue 'outer;
//                     }
//                     Some(' ') => {}
//                     Some(_) | None => break 'outer,
//                 }
//             }
//             break;
//         }
//
//         if lines.len() == 0 {
//             return Ok(None);
//         }
//
//         let content = lines.join("\n");
//
//         let mut reader: CharReader<&[u8]> = CharReader::<&[u8]>::from_string(&content);
//         reader.set_has_read(true); // prevents attributes
//         let tokens = read_tokens(&mut reader)?;
//         return Ok(Some(Token::BlockQuote { tokens }));
//     }
// }
//
