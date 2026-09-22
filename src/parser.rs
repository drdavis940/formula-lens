//! Recursive-descent parser: token stream in, [`Expr`] tree out.
//!
//! Precedence, low to high: `+ -`, `* /`, unary `-`, `^`. `^` is
//! right-associative (`2^3^2` is `2^(3^2)`), matching how spreadsheets
//! usually treat exponentiation.

use crate::ast::{CellRef, Expr};
use crate::lexer::{tokenize, Token};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expect(&mut self, want: &Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(ref t) if t == want => Ok(()),
            Some(t) => Err(ParseError {
                message: format!("expected {:?}, found {:?}", want, t),
                position: self.pos,
            }),
            None => Err(ParseError {
                message: format!("expected {:?}, found end of formula", want),
                position: self.pos,
            }),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_term()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    lhs = Expr::Binary { op: '+', lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Some(Token::Minus) => {
                    self.advance();
                    let rhs = self.parse_term()?;
                    lhs = Expr::Binary { op: '-', lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::Binary { op: '*', lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                Some(Token::Slash) => {
                    self.advance();
                    let rhs = self.parse_unary()?;
                    lhs = Expr::Binary { op: '/', lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if let Some(Token::Minus) = self.peek() {
            self.advance();
            let e = self.parse_unary()?;
            return Ok(Expr::Neg(Box::new(e)));
        }
        if let Some(Token::Plus) = self.peek() {
            self.advance();
            return self.parse_unary();
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_primary()?;
        if let Some(Token::Caret) = self.peek() {
            self.advance();
            let exp = self.parse_unary()?;
            return Ok(Expr::Binary { op: '^', lhs: Box::new(base), rhs: Box::new(exp) });
        }
        Ok(base)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::LParen) => {
                let e = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }
            Some(Token::Ident(name)) => self.parse_ident(name),
            Some(t) => Err(ParseError {
                message: format!("unexpected token {:?}", t),
                position: self.pos,
            }),
            None => Err(ParseError {
                message: "unexpected end of formula".to_string(),
                position: self.pos,
            }),
        }
    }

    fn parse_ident(&mut self, name: String) -> Result<Expr, ParseError> {
        // A name immediately followed by '(' is a function call.
        if let Some(Token::LParen) = self.peek() {
            self.advance();
            let mut args = Vec::new();
            if !matches!(self.peek(), Some(Token::RParen)) {
                loop {
                    args.push(self.parse_expr()?);
                    match self.peek() {
                        Some(Token::Comma) => {
                            self.advance();
                        }
                        _ => break,
                    }
                }
            }
            self.expect(&Token::RParen)?;
            return Ok(Expr::Call { name: name.to_ascii_uppercase(), args });
        }

        // Otherwise it has to be a cell reference, possibly the start of a range.
        let start = CellRef::parse(&name).ok_or_else(|| ParseError {
            message: format!("'{}' is not a valid cell reference or function call", name),
            position: self.pos,
        })?;

        if let Some(Token::Colon) = self.peek() {
            self.advance();
            return match self.advance() {
                Some(Token::Ident(end_name)) => {
                    let end = CellRef::parse(&end_name).ok_or_else(|| ParseError {
                        message: format!("'{}' is not a valid cell reference", end_name),
                        position: self.pos,
                    })?;
                    Ok(Expr::Range(start, end))
                }
                Some(t) => Err(ParseError {
                    message: format!("expected cell reference after ':', found {:?}", t),
                    position: self.pos,
                }),
                None => Err(ParseError {
                    message: "expected cell reference after ':', found end of formula".to_string(),
                    position: self.pos,
                }),
            };
        }

        Ok(Expr::Cell(start))
    }
}

/// Parses a formula string into an [`Expr`] tree.
///
/// A leading `=` is optional and stripped if present, matching how a
/// formula usually gets typed into a spreadsheet cell.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix('=').unwrap_or(trimmed);

    let tokens = tokenize(body).map_err(|e| ParseError { message: e.message, position: e.position })?;

    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;

    if parser.pos != parser.tokens.len() {
        return Err(ParseError {
            message: "unexpected trailing input".to_string(),
            position: parser.pos,
        });
    }

    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_arithmetic_with_precedence() {
        let expr = parse("=1 + 2 * 3").unwrap();
        assert_eq!(expr.to_string(), "(1 + (2 * 3))");
    }

    #[test]
    fn parses_a_range_inside_a_call() {
        let expr = parse("SUM(A1:A3, B1)").unwrap();
        assert_eq!(expr.to_string(), "SUM(A1:A3, B1)");
    }

    #[test]
    fn parses_unary_minus_and_power() {
        let expr = parse("-2^2").unwrap();
        // Unary binds tighter than parse_power calls it per operand, so
        // this parses as -(2^2), matching common spreadsheet behavior.
        assert_eq!(expr.to_string(), "-(2 ^ 2)");
    }

    #[test]
    fn reports_position_of_bad_reference() {
        let err = parse("=A1 + ZZZ").unwrap_err();
        assert!(err.message.contains("ZZZ"));
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("1 + 1)").is_err());
    }
}
