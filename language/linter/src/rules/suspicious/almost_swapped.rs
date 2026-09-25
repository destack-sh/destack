use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow assignments that overwrite a value before swapping it.
    pub ALMOST_SWAPPED {
        id: "almost-swapped",
        summary: "Disallow assignments that overwrite a value before swapping it",
        explanation: r#"
`left = right; right = left` assigns the original right value to both places because the first assignment overwrites the original left value.
Instead, you SHOULD assign the reversed tuple to both places in parallel.
"#,
        example: {
            reported: r#"
function exchange(pair: { left: int32; right: int32 }): void {
    pair.left = pair.right;
    pair.right = pair.left;
}
"#,
            accepted: r#"
function exchange(pair: { left: int32; right: int32 }): void {
    (pair.left, pair.right) = (pair.right, pair.left);
}
"#,
        },
        provenance: [Clippy("almost_swapped")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One direct assignment between stable places.
#[derive(Clone, Copy)]
struct PlaceTransfer<'a> {
    /// The written place.
    target: &'a dir::AccessResolution,
    /// The read place.
    value: &'a dir::AccessResolution,
}

impl<'a> PlaceTransfer<'a> {
    /// Select one plain assignment between stable places.
    fn select(
        module: &'a DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<Self> {
        let assignment = module.place_assignment(expression)?;
        if assignment.operator != dir::AssignOperator::Assign {
            return None;
        }
        let target = module.access_resolution(assignment.target)?;
        let value = module.access_resolution(assignment.value)?;

        Some(Self { target, value })
    }

    /// Return whether the next assignment reverses this assignment.
    fn is_reversed_by(self, next: Self) -> bool {
        self.target != self.value && self.target == next.value && self.value == next.target
    }
}

/// Report adjacent assignments that cannot exchange their values.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect adjacent statements within each block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let mut expressions = block.iter_expressions();
        let Some(mut first) = expressions.next() else {
            continue;
        };

        // compare each adjacent pair without allocating block storage
        for second in expressions {
            let first_assignment = PlaceTransfer::select(module, first);
            let second_assignment = PlaceTransfer::select(module, second);
            if first_assignment
                .zip(second_assignment)
                .is_some_and(|(first, second)| first.is_reversed_by(second))
            {
                let first_span = module.span(first.into_any())?;
                let second_span = module.span(second.into_any())?;
                let span = first_span.merge(second_span);
                let diagnostic = lint
                    .diagnostic("assignments overwrite a value instead of swapping", span)
                    .help("assign the reversed tuple to both places in parallel");
                output.report(diagnostic);
            }

            first = second;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report adjacent reversed stored field assignments.
    #[test]
    fn test_reports_reversed_field_assignments() {
        let session = TestSession::dir(
            &ALMOST_SWAPPED,
            r#"
struct Pair {
    left: int32;
    right: int32;
}
function exchange(pair: Pair): void {
    pair.left = pair.right;
    pair.right = pair.left;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[almost-swapped]: assignments overwrite a value instead of swapping
 ──▶ main.tspp:6:5
  │
4 │ }
5 │ function exchange(pair: Pair): void {
6 │     pair.left = pair.right;
  │     ^^^^^^^^^^^^^^^^^^^^^^^
7 │     pair.right = pair.left;
  │     ^^^^^^^^^^^^^^^^^^^^^^
8 │ }
  │

 = help: assign the reversed tuple to both places in parallel
"#,
        );
    }

    /// Report adjacent reversed binding assignments.
    #[test]
    fn test_reports_reversed_binding_assignments() {
        let session = TestSession::dir(
            &ALMOST_SWAPPED,
            r#"
function exchange(left: int32, right: int32): void {
    let first = left;
    let second = right;
    first = second;
    second = first;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[almost-swapped]: assignments overwrite a value instead of swapping
 ──▶ main.tspp:4:5
  │
2 │     let first = left;
3 │     let second = right;
4 │     first = second;
  │     ^^^^^^^^^^^^^^^
5 │     second = first;
  │     ^^^^^^^^^^^^^^
6 │ }
  │

 = help: assign the reversed tuple to both places in parallel
"#,
        );
    }

    /// Accept an exchange through a temporary value.
    #[test]
    fn test_accepts_temporary_exchange() {
        let session = TestSession::dir(
            &ALMOST_SWAPPED,
            r#"
function exchange(pair: { left: int32; right: int32 }): void {
    const previous = pair.left;
    pair.left = pair.right;
    pair.right = previous;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept reversed assignments separated by another statement.
    #[test]
    fn test_accepts_nonadjacent_assignments() {
        let session = TestSession::dir(
            &ALMOST_SWAPPED,
            r#"
function update(pair: { left: int32; right: int32 }): void {
    pair.left = pair.right;
    debugger;
    pair.right = pair.left;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
