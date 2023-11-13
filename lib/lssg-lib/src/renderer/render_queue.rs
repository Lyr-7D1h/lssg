use std::collections::VecDeque;

use crate::lmarkdown::Token;

pub struct RenderQueue {
    tokens: VecDeque<(Token, usize)>,
}

impl RenderQueue {
    pub fn pop_front(&mut self) -> Option<(Token, usize)> {
        self.tokens.pop_front()
    }

    pub fn from_tokens(tokens: Vec<Token>, parent_id: usize) -> Self {
        Self {
            tokens: VecDeque::from(
                tokens
                    .into_iter()
                    .map(|t| (t, parent_id))
                    .collect::<Vec<(Token, usize)>>(),
            ),
        }
    }

    pub fn push_tokens_front(&mut self, tokens: &Vec<Token>, parent_id: usize) {
        self.tokens
            .extend(tokens.clone().into_iter().map(|t| (t, parent_id)).rev());
    }
}
