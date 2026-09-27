# formula-lens

A small Rust library that parses spreadsheet formula strings (`=SUM(A1:A3, B1*2)`)
into an inspectable syntax tree, and renders the result — or a parse error — either
as plain text for a human or as JSON for another program to consume.

## Why

Anything that touches spreadsheet exports — a CSV/XLSX importer, a linter that
flags formulas referencing a deleted sheet, a migration script rewriting cell
references after inserting a column — needs to know what's inside a formula
string. Pulling in a full spreadsheet engine (recalculation, cell grid, cycle
detection) is overkill just to answer "what cells does this formula touch" or
"is this formula even syntactically valid." This library does only the parsing
and inspection part.

The library has no notion of a workbook; a caller supplies cell values by
implementing the [`Grid`](src/eval.rs) trait over whatever storage it
already has, or by using the bundled `MapGrid` for quick scripts and tests.

## Usage

```rust
use formula_lens::{parse_formula, OutputMode};

fn main() {
    match parse_formula("=SUM(A1:A3, B1*2)") {
        Ok(expr) => {
            // Human-readable, spreadsheet-ish text.
            println!("{}", expr);
            // SUM(A1:A3, (B1 * 2))

            // Machine-readable JSON, for a --json flag or a pipeline.
            println!("{}", expr.to_json());
            // {"type":"call","name":"SUM","args":[...]}
        }
        Err(diagnostic) => {
            eprintln!("{}", diagnostic.render(OutputMode::Human));
            eprintln!("{}", diagnostic.render(OutputMode::Json));
        }
    }
}
```

A caller building a CLI on top of this library can map its own `--json` flag
straight onto the mode:

```rust
use formula_lens::{parse_formula, OutputMode};

fn report(formula: &str, json_flag: bool) -> String {
    let mode = OutputMode::from_json_flag(json_flag);
    match parse_formula(formula) {
        Ok(expr) if mode == OutputMode::Json => expr.to_json(),
        Ok(expr) => expr.to_string(),
        Err(diagnostic) => diagnostic.render(mode),
    }
}
```

## Evaluating against a grid

Once parsed, an expression can be evaluated against a grid of cell values:

```rust
use formula_lens::{parse_formula, eval, CellRef, MapGrid};

let mut grid = MapGrid::new();
grid.set(CellRef::parse("A1").unwrap(), 1.0);
grid.set(CellRef::parse("A2").unwrap(), 2.0);
grid.set(CellRef::parse("A3").unwrap(), 3.0);

let expr = parse_formula("=SUM(A1:A3, 4)").unwrap();
assert_eq!(eval(&expr, &grid).unwrap(), 10.0);
```

`SUM` is the only built-in function the evaluator knows about today; an
unrecognized cell is treated as blank (0), and calling an unsupported
function or evaluating a bare range outside of `SUM` returns an
`EvalError`.

## What's parsed today

- Numbers: `1`, `2.5`
- Cell references: `A1`, `AB12`
- Ranges: `A1:A3`
- Arithmetic: `+ - * /` with standard precedence, unary minus, and `^`
  (right-associative)
- Function calls with comma-separated arguments: `SUM(A1:A3, B1, 2)`
- Parentheses for grouping

Not yet handled: string and boolean literals, `$` absolute references,
sheet-qualified references (`Sheet2!A1`), and comparison operators. See
the module docs in `src/` for the current grammar.

## Status

Early. The parser, AST, and a basic evaluator (`+ - * / ^` and `SUM`) are
solid enough to build on. See the crate root doc comment (`src/lib.rs`)
for the canonical example of the public API.

## License

MIT, see `LICENSE`.
