//! Turns a formula's text into a flat stream of tokens for the parser.
//!
//! Kept deliberately dumb: it knows nothing about spreadsheet grammar
//! (precedence, function arity, ranges), just character classes.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    /// Any run of letters/digits/underscore starting with a letter or
    /// underscore. Could be a cell reference (`A1`) or a function name
    /// (`SUM`) — the parser decides which based on context.
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    Colon,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '^' => {
                tokens.push(Token::Caret);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                i += 1;
            }
            '0'..='9' | '.' => {
                let start = i;
                let mut seen_dot = c == '.';
                i += 1;
                while i < chars.len() {
                    let d = chars[i];
                    if d.is_ascii_digit() {
                        i += 1;
                    } else if d == '.' && !seen_dot {
                        seen_dot = true;
                        i += 1;
                    } else {
                        break;
                    }
                }
                let text: String = chars[start..i].iter().collect();
                match text.parse::<f64>() {
                    Ok(n) => tokens.push(Token::Number(n)),
                    Err(_) => {
                        return Err(LexError {
                            message: format!("invalid number literal '{}'", text),
                            position: start,
                        })
                    }
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(text));
            }
            other => {
                return Err(LexError {
                    message: format!("unexpected character '{}'", other),
                    position: i,
                });
            }
        }
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_a_simple_call() {
        let tokens = tokenize("SUM(A1:A3, 2.5)").unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("SUM".to_string()),
                Token::LParen,
                Token::Ident("A1".to_string()),
                Token::Colon,
                Token::Ident("A3".to_string()),
                Token::Comma,
                Token::Number(2.5),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn rejects_unknown_characters() {
        let err = tokenize("A1 & B1").unwrap_err();
        assert_eq!(err.position, 3);
    }
}
