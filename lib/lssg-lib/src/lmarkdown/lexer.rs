use std::{collections::HashMap, io::Read};

use crate::{
    char_reader::CharReader,
    lmarkdown::{block_token::read_block_tokens, inline_token::read_inline_tokens},
    parse_error::ParseError,
};

// official spec: https://spec.commonmark.org/0.30/
// https://github.com/markedjs/marked/blob/master/src/Lexer.ts
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// demo: https://marked.js.org/demo/
// demo: https://spec.commonmark.org/dingus/
/// A function to get the next markdown token using recursive decent.
/// Will first parse a block token (token for one or multiple lines) and then parse for any inline tokens when needed.
pub fn read_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {
    let mut block_tokens = read_block_tokens(reader)?;

    // parse text inside of block tokens to inline tokens
    for t in block_tokens.iter_mut() {
        block_token_lexer(t);
    }

    return Ok(block_tokens);
}

/// parse text inside of block tokens to inline tokens
fn block_token_lexer(block_token: &mut Token) -> Result<(), ParseError> {
    match block_token {
        // Html is special because it can contains any kind of token
        Token::Html { tokens, .. } => tokens.into_iter().for_each(|t| {
            if t.is_block_token() {
                block_token_lexer(t);
            }
        }),
        Token::BulletList { texts, items, .. } | Token::OrderedList { texts, items, .. } => {
            *items = texts
                .into_iter()
                .map(|text| {
                    let mut reader = CharReader::new(text.as_bytes());
                    read_inline_tokens(&mut reader)
                })
                .collect::<Result<Vec<Vec<Token>>, ParseError>>()?;
        }
        Token::Heading { text, tokens, .. }
        | Token::Paragraph { text, tokens, .. }
        | Token::BlockQuote { text, tokens, .. } => {
            let mut reader = CharReader::new(text.as_bytes());
            *tokens = read_inline_tokens(&mut reader)?;
        }
        _ => {}
    };

    return Ok(());
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Attributes {
        table: toml::map::Map<String, toml::Value>,
    },
    BulletList {
        texts: Vec<String>,
        items: Vec<Vec<Token>>,
    },
    OrderedList {
        texts: Vec<String>,
        items: Vec<Vec<Token>>,
    },
    Heading {
        text: String,
        tokens: Vec<Token>,
        /// 0-6
        depth: u8,
    },
    Html {
        tokens: Vec<Token>,
        tag: String,
        attributes: HashMap<String, String>,
    },
    Paragraph {
        text: String,
        tokens: Vec<Token>,
    },
    BlockQuote {
        text: String,
        tokens: Vec<Token>,
    },
    Code {
        info: Option<String>,
        text: String,
    },
    Bold {
        text: String,
    },
    Emphasis {
        text: String,
    },
    /// https://spec.commonmark.org/0.30/#images
    Image {
        /// alt, recommended to convert tokens to text
        tokens: Vec<Token>,
        src: String,
        title: Option<String>,
    },
    Link {
        /// The text portion of a link that contains Tokens
        tokens: Vec<Token>,
        href: String,
        title: Option<String>,
    },
    Text {
        text: String,
    },
    Comment {
        raw: String,
    },
    // https://spec.commonmark.org/0.30/#thematic-breaks
    ThematicBreak,
    HardBreak,
    /// Indicating of a space between paragraphs
    SoftBreak,
}

impl Token {
    pub fn get_tokens(&self) -> Option<Vec<&Token>> {
        match self {
            Token::Heading { tokens, .. }
            | Token::Paragraph { tokens, .. }
            | Token::Link { tokens, .. }
            | Token::Image { tokens, .. }
            | Token::Html { tokens, .. } => Some(tokens.iter().collect()),
            Token::BulletList { items, .. } | Token::OrderedList { items, .. } => {
                let tokens = items.iter().flatten().collect();
                Some(tokens)
            }
            _ => None,
        }
    }

    pub fn to_text(&self) -> Option<String> {
        if let Some(tokens) = self.get_tokens() {
            let mut result = String::new();
            for t in tokens {
                if let Some(text) = t.to_text() {
                    result.push_str(&text)
                }
            }
            return Some(result);
        }
        Some(
            match self {
                Token::Bold { text, .. } => text,
                Token::Text { text, .. } => text,
                Token::SoftBreak { .. } => " ",
                _ => return None,
            }
            .into(),
        )
    }

    pub fn is_block_token(&self) -> bool {
        match self {
            Token::Attributes { .. }
            | Token::BulletList { .. }
            | Token::OrderedList { .. }
            | Token::Heading { .. }
            | Token::Html { .. }
            | Token::Paragraph { .. }
            | Token::BlockQuote { .. }
            | Token::Code { .. } => true,
            _ => false,
        }
    }
}
