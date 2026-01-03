use virtual_dom::{DomNode, DomNodeKind};

use crate::lmarkdown::Token;

impl From<DomNode> for Token {
    fn from(val: DomNode) -> Self {
        match &*val.kind() {
            DomNodeKind::Text { text } => Token::Text { text: text.clone() },
            DomNodeKind::Element { tag, attributes } => {
                // Recursively convert all children to tokens
                let tokens: Vec<Token> = val.children().map(|child| child.into()).collect();

                Token::Html {
                    tag: tag.clone(),
                    attributes: attributes.clone(),
                    tokens,
                }
            }
        }
    }
}
