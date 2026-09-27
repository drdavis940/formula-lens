//! Evaluates a parsed formula against a grid of cell values.
//!
//! This crate doesn't dictate a storage format for a spreadsheet's
//! cells: implement [`Grid`] over whatever a caller already has, or use
//! [`MapGrid`] for tests and small scripts.

use std::collections::HashMap;
use std::fmt;

use crate::ast::{CellRef, Expr};

#[derive(Debug, Clone, PartialEq)]
pub struct EvalError {
    pub message: String,
}

impl EvalError {
    fn new(message: impl Into<String>) -> Self {
        EvalError { message: message.into() }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// A source of cell values to evaluate a formula against.
pub trait Grid {
    /// Returns the numeric value of a cell, or `None` if it's blank.
    fn get(&self, cell: &CellRef) -> Option<f64>;
}

/// An in-memory grid backed by a hash map.
#[derive(Debug, Clone, Default)]
pub struct MapGrid {
    cells: HashMap<CellRef, f64>,
}

impl MapGrid {
    pub fn new() -> Self {
        MapGrid { cells: HashMap::new() }
    }

    pub fn set(&mut self, cell: CellRef, value: f64) {
        self.cells.insert(cell, value);
    }
}

impl Grid for MapGrid {
    fn get(&self, cell: &CellRef) -> Option<f64> {
        self.cells.get(cell).copied()
    }
}

/// Evaluates an expression tree to a single number.
///
/// A blank cell evaluates to 0, matching how spreadsheets treat an empty
/// cell in an arithmetic context. A bare range (one not passed to an
/// aggregating function like `SUM`) has no single numeric value, so
/// evaluating one directly is an error.
pub fn eval(expr: &Expr, grid: &dyn Grid) -> Result<f64, EvalError> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Cell(c) => Ok(grid.get(c).unwrap_or(0.0)),
        Expr::Range(_, _) => Err(EvalError::new(
            "a cell range can only be used as a function argument, not evaluated directly",
        )),
        Expr::Neg(e) => Ok(-eval(e, grid)?),
        Expr::Binary { op, lhs, rhs } => {
            let l = eval(lhs, grid)?;
            let r = eval(rhs, grid)?;
            match op {
                '+' => Ok(l + r),
                '-' => Ok(l - r),
                '*' => Ok(l * r),
                '/' => {
                    if r == 0.0 {
                        Err(EvalError::new("division by zero"))
                    } else {
                        Ok(l / r)
                    }
                }
                '^' => Ok(l.powf(r)),
                other => Err(EvalError::new(format!("unknown operator '{}'", other))),
            }
        }
        Expr::Call { name, args } => eval_call(name, args, grid),
    }
}

fn eval_call(name: &str, args: &[Expr], grid: &dyn Grid) -> Result<f64, EvalError> {
    match name {
        "SUM" => {
            let mut total = 0.0;
            for arg in args {
                total += eval_summable(arg, grid)?;
            }
            Ok(total)
        }
        other => Err(EvalError::new(format!("unknown function '{}'", other))),
    }
}

/// Evaluates one call argument in a summing context: a range expands to
/// the sum of every cell inside it, anything else evaluates normally.
fn eval_summable(expr: &Expr, grid: &dyn Grid) -> Result<f64, EvalError> {
    match expr {
        Expr::Range(a, b) => {
            let mut total = 0.0;
            for row in a.row.min(b.row)..=a.row.max(b.row) {
                for col in a.col.min(b.col)..=a.col.max(b.col) {
                    total += grid.get(&CellRef::new(col, row)).unwrap_or(0.0);
                }
            }
            Ok(total)
        }
        other => eval(other, grid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn grid_with(pairs: &[(&str, f64)]) -> MapGrid {
        let mut grid = MapGrid::new();
        for (cell, value) in pairs {
            grid.set(CellRef::parse(cell).unwrap(), *value);
        }
        grid
    }

    #[test]
    fn evaluates_arithmetic_with_precedence() {
        let expr = parse("=1 + 2 * 3").unwrap();
        assert_eq!(eval(&expr, &MapGrid::new()).unwrap(), 7.0);
    }

    #[test]
    fn evaluates_cell_references_and_blanks_as_zero() {
        let grid = grid_with(&[("A1", 10.0)]);
        let expr = parse("=A1 + B1").unwrap();
        assert_eq!(eval(&expr, &grid).unwrap(), 10.0);
    }

    #[test]
    fn sums_a_range_and_extra_arguments() {
        let grid = grid_with(&[("A1", 1.0), ("A2", 2.0), ("A3", 3.0)]);
        let expr = parse("SUM(A1:A3, 4)").unwrap();
        assert_eq!(eval(&expr, &grid).unwrap(), 10.0);
    }

    #[test]
    fn rejects_division_by_zero() {
        let expr = parse("=1 / 0").unwrap();
        assert!(eval(&expr, &MapGrid::new()).is_err());
    }

    #[test]
    fn rejects_a_bare_range_outside_a_function() {
        let expr = parse("=A1:A3").unwrap();
        assert!(eval(&expr, &MapGrid::new()).is_err());
    }

    #[test]
    fn rejects_unknown_functions() {
        let expr = parse("=AVERAGE(A1:A3)").unwrap();
        assert!(eval(&expr, &MapGrid::new()).is_err());
    }
}
