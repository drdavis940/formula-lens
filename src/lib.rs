//! Parses spreadsheet formula strings into an inspectable AST, and
//! renders the result (or any parse error) either as plain text for a
//! human or as JSON for another program.
//!
//! ```
//! use formula_lens::parse_formula;
//!
//! let expr = parse_formula("=SUM(A1:A3, B1*2)").unwrap();
//! assert_eq!(expr.to_string(), "SUM(A1:A3, (B1 * 2))");
//! assert!(expr.to_json().starts_with("{\"type\":\"call\""));
//! ```

pub mod ast;
pub mod diagnostic;
pub mod eval;
pub mod lexer;
pub mod parser;

pub use ast::{CellRef, Expr};
pub use diagnostic::{Diagnostic, OutputMode};
pub use eval::{eval, EvalError, Grid, MapGrid};
pub use parser::ParseError;

/// Parses a formula string, wrapping any [`ParseError`] into a
/// [`Diagnostic`] that already knows how to render itself for either
/// output mode.
pub fn parse_formula(input: &str) -> Result<Expr, Diagnostic> {
    parser::parse(input).map_err(|e| Diagnostic::new(e.message, e.position))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_formula_reports_diagnostics_in_both_modes() {
        let err = parse_formula("=1 +").unwrap_err();
        assert!(err.render(OutputMode::Human).starts_with("error at position"));
        assert!(err.render(OutputMode::Json).starts_with("{\"error\":true"));
    }
}
