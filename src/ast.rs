//! The tree a formula parses into, plus the two ways to render it: as
//! spreadsheet-ish text ([`std::fmt::Display`]) and as JSON ([`Expr::to_json`]).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct CellRef {
    /// 1-based column index: A = 1, B = 2, ..., Z = 26, AA = 27.
    pub col: u32,
    /// 1-based row index, as typed in the formula.
    pub row: u32,
}

impl CellRef {
    pub fn new(col: u32, row: u32) -> Self {
        CellRef { col, row }
    }

    /// Parses a bare reference like `A1` or `AB12`. Returns `None` for
    /// anything that isn't letters-then-digits with no leftovers.
    pub fn parse(text: &str) -> Option<CellRef> {
        let upper = text.to_ascii_uppercase();
        let letters_end = upper.find(|c: char| !c.is_ascii_alphabetic())?;
        if letters_end == 0 {
            return None;
        }
        let (letters, digits) = upper.split_at(letters_end);
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let col = letters_to_col(letters)?;
        let row: u32 = digits.parse().ok()?;
        if row == 0 {
            return None;
        }
        Some(CellRef { col, row })
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", col_to_letters(self.col), self.row)
    }
}

fn letters_to_col(letters: &str) -> Option<u32> {
    let mut col: u32 = 0;
    for c in letters.chars() {
        if !c.is_ascii_alphabetic() {
            return None;
        }
        let digit = (c as u32) - ('A' as u32) + 1;
        col = col.checked_mul(26)?.checked_add(digit)?;
    }
    if col == 0 {
        None
    } else {
        Some(col)
    }
}

fn col_to_letters(mut col: u32) -> String {
    let mut letters = Vec::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        letters.push((b'A' + rem as u8) as char);
        col = (col - 1) / 26;
    }
    letters.iter().rev().collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Cell(CellRef),
    Range(CellRef, CellRef),
    Neg(Box<Expr>),
    Binary {
        op: char,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::Cell(c) => write!(f, "{}", c),
            Expr::Range(a, b) => write!(f, "{}:{}", a, b),
            Expr::Neg(e) => write!(f, "-{}", e),
            Expr::Binary { op, lhs, rhs } => write!(f, "({} {} {})", lhs, op, rhs),
            Expr::Call { name, args } => {
                write!(f, "{}(", name)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Expr {
    /// Renders the tree as JSON. Hand-rolled rather than pulled from a
    /// crate, since the whole point of this library is zero dependencies.
    pub fn to_json(&self) -> String {
        match self {
            Expr::Number(n) => format!("{{\"type\":\"number\",\"value\":{}}}", n),
            Expr::Cell(c) => format!(
                "{{\"type\":\"cell\",\"col\":{},\"row\":{},\"ref\":\"{}\"}}",
                c.col, c.row, c
            ),
            Expr::Range(a, b) => {
                format!("{{\"type\":\"range\",\"from\":\"{}\",\"to\":\"{}\"}}", a, b)
            }
            Expr::Neg(e) => format!("{{\"type\":\"neg\",\"expr\":{}}}", e.to_json()),
            Expr::Binary { op, lhs, rhs } => format!(
                "{{\"type\":\"binary\",\"op\":\"{}\",\"lhs\":{},\"rhs\":{}}}",
                op,
                lhs.to_json(),
                rhs.to_json()
            ),
            Expr::Call { name, args } => {
                let args_json: Vec<String> = args.iter().map(|a| a.to_json()).collect();
                format!(
                    "{{\"type\":\"call\",\"name\":\"{}\",\"args\":[{}]}}",
                    name,
                    args_json.join(",")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_column_letters() {
        assert_eq!(col_to_letters(1), "A");
        assert_eq!(col_to_letters(26), "Z");
        assert_eq!(col_to_letters(27), "AA");
        assert_eq!(letters_to_col("A"), Some(1));
        assert_eq!(letters_to_col("Z"), Some(26));
        assert_eq!(letters_to_col("AA"), Some(27));
    }

    #[test]
    fn parses_and_displays_cell_refs() {
        let cell = CellRef::parse("b12").unwrap();
        assert_eq!(cell, CellRef::new(2, 12));
        assert_eq!(cell.to_string(), "B12");
    }

    #[test]
    fn rejects_malformed_refs() {
        assert!(CellRef::parse("1A").is_none());
        assert!(CellRef::parse("A").is_none());
        assert!(CellRef::parse("A0").is_none());
        assert!(CellRef::parse("A1B").is_none());
    }
}
