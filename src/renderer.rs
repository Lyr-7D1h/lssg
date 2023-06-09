use std::{error::Error, fmt::Display};

use crate::parser::lexer::Token;

#[derive(Debug, Clone)]
pub enum Rel {
    Stylesheet,
}
impl TryFrom<String> for Rel {
    type Error = Box<dyn Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match &value[..] {
            "stylesheet" => Ok(Rel::Stylesheet),
            _ => Err(format!("Invalid rel value given {value}").into()),
        }
    }
}
impl Display for Rel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rel::Stylesheet => f.write_str("stylesheet"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HtmlLink {
    pub rel: Rel,
    pub href: String,
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct HtmlDocumentRenderOptions {
    pub links: Vec<HtmlLink>,
    pub title: String,
    pub meta: Vec<Meta>,
    pub language: String,
}

pub struct HtmlDocument {}

impl HtmlDocument {
    fn render_body_content(tokens: Vec<Token>, options: &mut HtmlDocumentRenderOptions) -> String {
        let mut body_content = String::new();
        for t in tokens.into_iter() {
            let html = match t {
                Token::Heading {
                    depth,
                    text,
                    tokens,
                } => format!(
                    "<h{depth}>{}</h{depth}>",
                    Self::render_body_content(tokens, options)
                ),
                Token::Paragraph { tokens } => {
                    format!("<p>{}</p>", Self::render_body_content(tokens, options))
                }
                Token::Bold { text } => format!("<b>{text}</b>"),
                Token::Italic { text } => format!("<i>{text}</i>"),
                Token::Code { language, code } => format!("<cod>{code}</code>"),
                Token::Space { raw } => format!("<br />"),
                Token::Link { text, href } => format!("<a href={href}>{text}</a>"),
                Token::Text { text } => text,
                Token::HtmlLink { rel, text, href } => {
                    if let Ok(rel) = Rel::try_from(rel) {
                        options.links.push(HtmlLink { rel, href })
                    }
                    String::new()
                }
                Token::EOF => continue,
            };
            body_content.push_str(&html);
        }
        body_content
    }

    pub fn render(tokens: Vec<Token>, mut options: HtmlDocumentRenderOptions) -> String {
        println!("{tokens:?}");
        let body_content = Self::render_body_content(tokens, &mut options);
        let body = format!("<body>{body_content}</body>");

        let links = options
            .links
            .iter()
            .map(|l| format!("<link rel={} href={}>", l.rel.to_string(), l.href))
            .reduce(|a, l| a + &l)
            .unwrap_or(String::new());
        let title = format!("<title>{}</title>", options.title);
        let head = format!("<head>{title}{links}</head>");

        let lang = options.language;
        return format!(r#"<!DOCTYPE html><html lang="{lang}">{head}{body}</html>"#);
    }
}
