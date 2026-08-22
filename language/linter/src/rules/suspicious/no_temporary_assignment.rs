use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assigning into values that are immediately discarded.
    pub NO_TEMPORARY_ASSIGNMENT {
        id: "no-temporary-assignment",
        summary: "Disallow assigning into values that are immediately discarded",
        explanation: r#"
Writing a stored member or element of a temporary value discards the mutation with that value.
Instead, you SHOULD bind the value before assigning into it or use the resulting value directly.
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
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report stored writes through temporary receivers.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect direct assignments and compound assignments
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        let Some(receiver) = module.stored_write_receiver(assignment.target)? else {
            continue;
        };

        // report receivers without stable checked storage
        if module.access_resolution(receiver).is_none() {
            let span = module.source_extent(assignment.target.into_any())?;
            let diagnostic = lint
                .diagnostic("assignment mutates a temporary value", span)
                .help("bind the value before assigning into it");
            output.report(diagnostic);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a field write through a temporary struct.
    #[test]
    fn test_reports_temporary_member() {
        TestSession::assert_example(&NO_TEMPORARY_ASSIGNMENT);
    }

    /// Report an element write through a temporary Array.
    #[test]
    fn test_reports_temporary_element() {
        let session = TestSession::dir(
            &NO_TEMPORARY_ASSIGNMENT,
            r#"
function discard(): void {
    values()[0] = 4;
}

function values(): int32[] {
    return [1, 2, 3];
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[no-temporary-assignment]: assignment mutates a temporary value
 ──▶ main.ds:2:5
  │
1 │ function discard(): void {
2 │     values()[0] = 4;
  │     ^^^^^^^^^^^
3 │ }
4 │
5 │ function values(): int32[] {
6 │     return [1, 2, 3];
7 │ }
  │
  = help: bind the value before assigning into it
"#,
        );
    }

    /// Accept a field write through a retained binding.
    #[test]
    fn test_accepts_binding_member() {
        let session = TestSession::dir(
            &NO_TEMPORARY_ASSIGNMENT,
            NO_TEMPORARY_ASSIGNMENT.example.accepted.source(),
        );

        session.assert_no_diagnostics();
    }

    /// Accept a setter call whose receiver retains the effect.
    #[test]
    fn test_accepts_setter() {
        let session = TestSession::dir(
            &NO_TEMPORARY_ASSIGNMENT,
            r#"
class Cell {
    set value(next: int32) {}
}

function assign(cell: Cell): void {
    cell.value = 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
