use std::io::Read;

use virtual_dom::Html;

use crate::{char_reader::CharReader, parse_error::ParseError};

use super::{Token, html::html_comment, html::html_element, sanitize_text};

pub fn read_inline_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {
    let mut tokens = vec![];
    'outer: while let Some(c) = reader.peek_char(0)? {
        // html
        if c == '<' {
            // comment
            if let Some(Html::Comment { text: raw }) = html_comment(reader)? {
                tokens.push(Token::Comment { raw });
                continue;
            }

            // html
            if let Some((tag, attributes, content)) = html_element(reader)? {
                let content_tokens = if let Some(content) = content {
                    let content = sanitize_text(content);
                    read_inline_tokens(&mut CharReader::new(content.as_bytes()))?
                } else {
                    vec![]
                };

                tokens.push(Token::Html {
                    tag,
                    attributes,
                    tokens: content_tokens,
                });
                continue;
            }

            // https://spec.commonmark.org/0.30/#autolinks
            if let Some(link) = reader.peek_until_exclusive_from(1, |c| c == '>')? {
                let mut valid = false;
                for c in link.chars() {
                    match c {
                        '<' | '>' | ' ' => {
                            valid = false;
                            break;
                        }
                        // must contain scheme
                        ':' => {
                            valid = true;
                        }
                        _ => {}
                    }
                }
                if valid {
                    reader.consume(1)?;
                    let text = reader.consume_string(link.len())?;
                    reader.consume(1)?;
                    tokens.push(Token::Link {
                        tokens: vec![Token::Text { text }],
                        href: link,
                        title: None,
                    });
                    continue;
                }
            }
        }

        // https://spec.commonmark.org/0.30/#code-spans
        if c == '`' {
            let mut backtick_count = 1;
            while let Some('`') = reader.peek_char(backtick_count)? {
                backtick_count += 1;
            }

            let mut i = backtick_count;
            let mut count = 0;
            while let Some(c) = reader.peek_char(i)? {
                match c {
                    '`' => {
                        count += 1;

                        // skip if next is backtick
                        if let Ok(Some('`')) = reader.peek_char(i + 1) {
                            i += 1;
                            continue;
                        }

                        if count == backtick_count {
                            reader.consume(backtick_count)?;
                            let mut text = reader.consume_string(i + 1 - backtick_count * 2)?;
                            // remove leading and ending space if not only contained with spaces
                            if text.starts_with(" ")
                                && text.ends_with(" ")
                                && text.find(char::is_alphabetic).is_some()
                            {
                                text = text[1..text.len() - 1].to_string();
                            }
                            reader.consume(backtick_count)?;
                            tokens.push(Token::Code { text });
                            continue 'outer;
                        }
                    }
                    _ => count = 0,
                }
                i += 1;
            }
        }

        // https://spec.commonmark.org/0.30/#images
        if c == '!'
            && let Some('[') = reader.peek_char(1)?
            && let Some(raw_text) = reader.peek_until_inclusive_from(2, |c| c == ']')?
        {
            let href_start = 2 + raw_text.len();
            if let Some('(') = reader.peek_char(href_start)?
                && let Some(raw_href) =
                    reader.peek_until_inclusive_from(href_start + 1, |c| c == ')')?
            {
                reader.consume(2)?;
                let text = reader.consume_string(raw_text.len() - 1)?;
                reader.consume(2)?;
                let src = reader.consume_string(raw_href.len() - 1)?;
                let src = sanitize_text(src);

                // https://spec.commonmark.org/0.30/#link-title
                let title = if let Some(start_title) = src.find(" ") {
                    let title = &src[start_title..src.len()];

                    if ((title.starts_with("\"") && title.ends_with("\""))
                        || (title.starts_with("\'") && title.ends_with("\'")))
                        && title.len() >= 2
                    {
                        Some(title.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                reader.consume(1)?;
                let alt = read_inline_tokens(&mut CharReader::new(text.as_bytes()))?;
                tokens.push(Token::Image {
                    tokens: alt,
                    src,
                    title,
                });
                continue;
            }
        }

        // links: https://spec.commonmark.org/0.30/#links
        if c == '[' {
            let mut indent = 1;
            let mut i = 1;
            while let Ok(Some(c)) = reader.peek_char(i) {
                i += 1;
                match c {
                    '[' => {
                        if let Ok(Some('!')) = reader.peek_char(i - 1) {
                            continue;
                        }
                        indent += 1;
                    }
                    ']' => {
                        indent -= 1;
                        if indent == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if indent == 0
                && let Some('(') = reader.peek_char(i)?
                && let Some(raw_href) = reader.peek_until_inclusive_from(i + 1, |c| c == ')')?
            {
                reader.consume(1)?;
                let text = reader.consume_string(i - 2)?;
                reader.consume(2)?;
                let mut href = reader.consume_string(raw_href.len() - 1)?;
                reader.consume(1)?;
                let text = sanitize_text(text);
                let text = read_inline_tokens(&mut CharReader::new(text.as_bytes()))?;

                // https://spec.commonmark.org/0.30/#link-title
                let title = if let Some(start_title) = href.find(" ") {
                    let title = &href[start_title + 1..href.len()];

                    if ((title.starts_with("\"") && title.ends_with("\""))
                        || (title.starts_with("\'") && title.ends_with("\'")))
                        && title.len() >= 2
                    {
                        let title = title[1..title.len() - 1].to_string();
                        href = (&href[0..start_title]).into();
                        Some(title)
                    } else {
                        None
                    }
                } else {
                    None
                };

                tokens.push(Token::Link {
                    tokens: text,
                    href,
                    title,
                });
                continue;
            }
        }

        // emphasis: https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis
        if c == '*' {
            if let Some('*') = reader.peek_char(1)?
                && let Some(text) = reader.peek_until_match_inclusive_from(2, "**")?
            {
                reader.consume(2)?;
                let text = reader.consume_string(text.len() - 2)?;
                reader.consume(2)?;
                tokens.push(Token::Bold { text });
                continue;
            }
            if let Some(text) = reader.peek_until_inclusive_from(1, |c| c == '*')? {
                reader.consume(1)?;
                let text = reader.consume_string(text.len() - 1)?;
                reader.consume(1)?;
                tokens.push(Token::Emphasis { text });
                continue;
            }
        }

        // GFM autolinks extension - only at beginning of line, after whitespace, or after delimiters
        let can_autolink = tokens.is_empty()
            || matches!(
                tokens.last(),
                Some(Token::Text { text }) if text.ends_with(|c: char| c.is_whitespace() || matches!(c, '*' | '_' | '~' | '('))
            )
            || matches!(tokens.last(), Some(Token::SoftBreak | Token::HardBreak));
        if can_autolink {
            if let Some(autolink) = autolink(c, reader)? {
                tokens.push(autolink);
                continue;
            }
        }

        let c = reader.consume_char().unwrap().expect("has to be a char");

        // line breaks
        if c == '\n' {
            // https://spec.commonmark.org/0.30/#hard-line-break
            if let Some(Token::Text { text }) = tokens.last_mut() {
                if text.ends_with("\\") {
                    text.pop();
                    tokens.push(Token::HardBreak);
                    continue;
                }
                if text.ends_with("  ") {
                    *text = text.trim_end().to_string();
                    tokens.push(Token::HardBreak);
                    continue;
                }
            }

            // soft break: https://spec.commonmark.org/0.30/#soft-line-breaks
            // only add soft break if not last character
            if reader.peek_char(0)?.is_some() {
                tokens.push(Token::SoftBreak);
            }
            continue;
        }
        // push character to text
        if let Some(Token::Text { text }) = tokens.last_mut() {
            text.push(c)
        } else {
            tokens.push(Token::Text { text: c.into() })
        }
    }

    Ok(tokens)
}

fn strip_trailing_punctuation(url: &str) -> String {
    let mut result = url.to_string();

    loop {
        let mut changed = false;

        // Strip simple punctuation
        while result.ends_with(|c: char| matches!(c, '.' | ',' | '!' | '?' | ':' | ';')) {
            result.pop();
            changed = true;
        }

        // Strip unbalanced closing parens
        while result.ends_with(')') {
            let open_count = result.matches('(').count();
            let close_count = result.matches(')').count();

            if close_count > open_count {
                result.pop();
                changed = true;
            } else {
                break;
            }
        }

        if !changed {
            break;
        }
    }

    result
}

fn peek_until_whitespace_or_eof(reader: &mut CharReader<impl Read>) -> Result<String, ParseError> {
    if let Some(text) =
        reader.peek_until_exclusive_from(0, |c| c.is_whitespace() || c == '<' || c == '>')?
    {
        Ok(text)
    } else {
        // Reached EOF - get all remaining text
        let mut len = 0;
        while reader.peek_char(len)?.is_some() {
            len += 1;
        }
        reader.peek_string(len)
    }
}

fn consume_and_create_autolink(
    reader: &mut CharReader<impl Read>,
    text: String,
    href: String,
) -> Result<Token, ParseError> {
    reader.consume(text.len())?;
    Ok(Token::Autolink { href, text })
}

fn autolink(c: char, reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    // www. autolinks
    if c == 'w' && "ww." == reader.peek_string_from(1, 3)? {
        let full_text = peek_until_whitespace_or_eof(reader)?;

        if let Some(domain_part) = full_text.strip_prefix("www.") {
            let domain_end = domain_part
                .char_indices()
                .find(|(_, c)| !c.is_alphanumeric() && *c != '-' && *c != '_' && *c != '.')
                .map(|(i, _)| i)
                .unwrap_or(domain_part.len());

            let domain = &domain_part[..domain_end];

            if domain.contains('.') && domain.len() > 1 {
                let url = strip_trailing_punctuation(&full_text);
                return consume_and_create_autolink(reader, url.clone(), format!("http://{url}"))
                    .map(Some);
            }
        }
    }

    // scheme-based URLs (http://, https://, ftp://, etc.)
    if c.is_alphabetic() {
        let potential_url = peek_until_whitespace_or_eof(reader)?;

        if let Some(scheme_pos) = potential_url.find("://") {
            let scheme = &potential_url[..scheme_pos];

            if (2..=32).contains(&scheme.len())
                && scheme
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '+'))
            {
                let url = strip_trailing_punctuation(&potential_url);

                if url.len() > scheme_pos + 3 {
                    return consume_and_create_autolink(reader, url.clone(), url).map(Some);
                }
            }
        }
    }

    // Email autolinks
    if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
        let potential_email = peek_until_whitespace_or_eof(reader)?;

        if let Some(at_pos) = potential_email.find('@') {
            let (local, domain) = (&potential_email[..at_pos], &potential_email[at_pos + 1..]);

            if !local.is_empty()
                && domain.contains('.')
                && domain.len() > 2
                && local
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
                && domain
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-'))
            {
                let email = strip_trailing_punctuation(&potential_email);
                return consume_and_create_autolink(
                    reader,
                    email.clone(),
                    format!("mailto:{email}"),
                )
                .map(Some);
            }
        }
    }

    Ok(None)
}
