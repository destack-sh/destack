use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer take over moving a value and assigning its default.
    pub MANUAL_TAKE {
        id: "manual-take",
        summary: "Prefer take over moving a value and assigning its default",
        explanation: r#"
Moving a place into a temporary and then assigning its default value manually takes the place's value.
Instead, you SHOULD call `take` with an exclusive borrow of the place.
"#,
        example: {
            reported: r#"
function remove<T: Default>(initial: T): T {
    let value = initial;
    const previous = value;
    value = T.default();
    return previous;
}
"#,
            accepted: r#"
import { take } from "tspp:memory";

function remove<T: Default>(initial: T): T {
    let value = initial;
    return take(&exclusive value);
}
"#,
        },
        provenance: [Clippy("manual_take")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report adjacent moves and default assignments that manually take a place.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect adjacent statements within each block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let mut expressions = block.iter_expressions();
        let Some(mut declaration) = expressions.next() else {
            continue;
        };

        // match one direct temporary followed by a default assignment
        for assignment in expressions {
            if let Some(place) = taken_place(module, declaration, assignment)? {
                let first = module.statement_span(declaration)?;
                let second = module.statement_span(assignment)?;
                let span = first.merge(second);
                let place = module.source_extent(place.into_any())?;
                let place = module.source(place)?;
                let diagnostic = lint
                    .diagnostic("place is moved and then assigned its default", span)
                    .help(format!("use `take(&exclusive {place})`"));
                output.report(diagnostic);
            }

            declaration = assignment;
        }
    }

    Ok(output)
}

/// Return the place moved and defaulted by two adjacent statements.
fn taken_place(
    module: &DirModule<'_>,
    declaration: dir::LocalNodeId<dir::Expression>,
    assignment: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    // select a direct initialized temporary
    let Some((_, declarator)) = module.binding_declarator(declaration) else {
        return Ok(None);
    };
    let Some(place) = declarator.value else {
        return Ok(None);
    };

    // require a plain assignment back into the same place
    let Some(assignment) = module.place_assignment(assignment) else {
        return Ok(None);
    };
    if assignment.operator != dir::AssignOperator::Assign
        || !module.is_same_computation(place, assignment.target)?
    {
        return Ok(None);
    }

    // require the temporary to receive the unadjusted place value
    let place_type = module.node_type_id(place.into_any())?;
    let place_type = module.dir.strip_form(place_type)?;
    let temporary_type = module.node_type_id(declarator.pattern.into_any())?;
    let temporary_type = module.dir.strip_form(temporary_type)?;
    if place_type != temporary_type
        || module
            .coercions
            .coercion(place.into_global_any(module.id))
            .is_some()
    {
        return Ok(None);
    }

    // require the canonical zero-argument default operation
    if module.language_member(assignment.value)?
        != Some(dir::LanguageItem::Default.member("default"))
    {
        return Ok(None);
    }
    let dir::Expression::Call { arguments, .. } = module.view().get(assignment.value) else {
        return Ok(None);
    };
    let default_type = module.node_type_id(assignment.value.into_any())?;
    let default_type = module.dir.strip_form(default_type)?;
    if place_type != default_type {
        return Ok(None);
    }

    Ok(arguments.is_empty().then_some(place))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a value moved before the same place is defaulted.
    #[test]
    fn test_reports_manual_take() {
        let session = TestSession::dir(
            &MANUAL_TAKE,
            r#"
function remove<T: Default>(initial: T): T {
    let value = initial;
    const previous = value;
    value = T.default();
    return previous;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-take]: place is moved and then assigned its default
 ──▶ main.tspp:3:5
  │
1 │ function remove<T: Default>(initial: T): T {
2 │     let value = initial;
3 │     const previous = value;
  │     ^^^^^^^^^^^^^^^^^^^^^^^
4 │     value = T.default();
  │     ^^^^^^^^^^^^^^^^^^^^
5 │     return previous;
6 │ }
  │

 = help: use `take(&exclusive value)`
"#,
        );
    }

    /// Accept default assignment to a different place.
    #[test]
    fn test_accepts_different_place() {
        let session = TestSession::dir(
            &MANUAL_TAKE,
            r#"
function remove<T: Default>(first: T, second: T): T {
    let left = first;
    let right = second;
    const previous = left;
    right = T.default();
    return previous;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept assignment of a nondefault value.
    #[test]
    fn test_accepts_nondefault_assignment() {
        let session = TestSession::dir(
            &MANUAL_TAKE,
            r#"
function replace<T>(first: T, second: T): T {
    let value = first;
    const previous = value;
    value = second;
    return previous;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
