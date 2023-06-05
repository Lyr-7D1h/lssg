use crate::parser::lexer::Token;

pub trait Renderer {
    fn render(tokens: Vec<Token>) -> String;
}

pub struct DefaultHtmlRenderer {}
impl Renderer for DefaultHtmlRenderer {
    fn render(tokens: Vec<Token>) -> String {
        let mut output = String::new();
        for t in tokens.into_iter() {
            let html = match t {
                Token::Heading {
                    depth,
                    text,
                    tokens,
                } => format!("<h{depth}>{}</h{depth}>", Self::render(tokens)),
                Token::Paragraph { tokens } => format!("<p>{}</p>", Self::render(tokens)),
                Token::Bold { text } => format!("<b>{text}</b>"),
                Token::Italic { text } => format!("<i>{text}</i>"),
                Token::Code { language, code } => todo!(),
                Token::Space { raw } => format!("<br />"),
                Token::Link { text, href } => format!("<a href={href}>{text}</a>"),
                Token::Text { text } => text,
                Token::EOF => continue,
            };
            output.push_str(&html);
        }
        return output;
    }
}
