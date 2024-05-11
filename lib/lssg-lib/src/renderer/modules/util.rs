use crate::lmarkdown::Token;

pub fn tokens_to_text(tokens: &Vec<Token>) -> String {
    let mut result = String::new();
    for t in tokens {
        if let Some(text) = t.to_text() {
            result.push_str(&text)
        }
    }
    return result;
}
