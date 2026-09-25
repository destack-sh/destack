use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignments into discarded struct and tuple constructions.
    pub TEMPORARY_ASSIGNMENT {
        id: "temporary-assignment",
        summary: "Disallow assignments into discarded struct and tuple constructions",
        explanation: r#"
Assigning through a freshly constructed struct or tuple updates a value discarded at the end of the statement.
Instead, you SHOULD construct the intended value directly or bind it before assigning into it.
"#,
        example: {
            reported: r#"
struct Point {
    x: int32;
}

function discard(): void {
    Point { x: 1 }.x = 2;
}
"#,
            accepted: r#"
struct Point {
    x: int32;
}

function retain(): Point {
    let point = Point { x: 1 };
    point.x = 2;
    return point;
}
"#,
        },
        provenance: [Clippy("temporary_assignment")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report stored writes through discarded struct constructions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct and compound assignments
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        let Some(root) = module.stored_write_root(assignment.target)? else {
            continue;
        };
        if !matches!(
            view.get(root),
            dir::Expression::StructExpression { .. } | dir::Expression::TupleExpression { .. }
        ) {
            continue;
        }

        // report the complete discarded write
        let span = module.source_extent(expression.into_any())?;
        let diagnostic = lint
            .diagnostic("assignment updates a discarded value", span)
            .help("construct the intended value directly or bind it first");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a stored field write through a temporary struct.
    #[test]
    fn test_reports_struct_member_assignment() {
        TestSession::assert_example(&TEMPORARY_ASSIGNMENT);
    }

    /// Report a nested field write through a temporary struct.
    #[test]
    fn test_reports_nested_struct_member_assignment() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
struct Point {
    x: int32;
}

struct Wrapper {
    point: Point;
}

function discard(): void {
    Wrapper { point: Point { x: 1 } }.point.x += 1;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[temporary-assignment]: assignment updates a discarded value
  ──▶ main.tspp:10:5
   │
 8 │
 9 │ function discard(): void {
10 │     Wrapper { point: Point { x: 1 } }.point.x += 1;
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │ }
   │

 = help: construct the intended value directly or bind it first
"#,
        );
    }

    /// Report a stored element write through a temporary tuple.
    #[test]
    fn test_reports_tuple_element_assignment() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
function discard(): void {
    (1, 2)[0] = 3;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[temporary-assignment]: assignment updates a discarded value
 ──▶ main.tspp:2:5
  │
1 │ function discard(): void {
2 │     (1, 2)[0] = 3;
  │     ^^^^^^^^^^^^^
3 │ }
  │

 = help: construct the intended value directly or bind it first
"#,
        );
    }

    /// Accept a field write through a retained binding.
    #[test]
    fn test_accepts_binding_member() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
struct Point {
    x: int32;
}

function retain(): Point {
    let point = Point { x: 1 };
    point.x = 2;
    return point;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a write through a managed field of a temporary struct.
    #[test]
    fn test_accepts_managed_struct_field() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
class Cell {
    value: int32 = 0;
}

struct Wrapper {
    cell: Cell;
}

function update(cell: Cell): void {
    Wrapper { cell }.cell.value = 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a stored write through a managed value returned by a call.
    #[test]
    fn test_accepts_managed_call_receiver() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
class Cell {
    value: int32 = 0;
}

declare function current(): Cell;

function update(): void {
    current().value = 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an effectful setter call through a temporary receiver.
    #[test]
    fn test_accepts_temporary_setter_receiver() {
        let session = TestSession::dir(
            &TEMPORARY_ASSIGNMENT,
            r#"
class Cell {
    set value(next: int32) {}
}

function update(): void {
    new Cell().value = 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
