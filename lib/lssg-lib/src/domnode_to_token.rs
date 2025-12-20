use virtual_dom::{DomNode, DomNodeKind};

use crate::lmarkdown::Token;

impl Into<Token> for DomNode {
    fn into(self) -> Token {
        match &*self.kind() {
            DomNodeKind::Text { text } => Token::Text { text: text.clone() },
            DomNodeKind::Element { tag, attributes } => {
                // Recursively convert all children to tokens
                let tokens: Vec<Token> = self.children().map(|child| child.into()).collect();

                Token::Html {
                    tag: tag.clone(),
                    attributes: attributes.clone(),
                    tokens,
                }
            }
        }
    }
}
