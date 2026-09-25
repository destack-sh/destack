use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer Clone.cloneFrom when replacing a cloneable value.
    pub PREFER_CLONE_FROM {
        id: "prefer-clone-from",
        summary: "Prefer Clone.cloneFrom when replacing a cloneable value",
        explanation: r#"
Assigning a newly cloned value discards storage that the destination may be able to reuse.
Instead, you SHOULD call `cloneFrom` so the destination can reuse its existing allocation.
"#,
        example: {
            reported: r#"
import { rc } from "tspp:memory";

function replace(target: &exclusive rc.Rc<int32>, source: &immutable rc.Rc<int32>): void {
    *target = source.clone();
}
"#,
            accepted: r#"
import { rc } from "tspp:memory";

function replace(target: &exclusive rc.Rc<int32>, source: &immutable rc.Rc<int32>): void {
    target.cloneFrom(source);
}
"#,
        },
        provenance: [Clippy("assigning_clones")],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report clone assignments that can reuse destination storage.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect plain assignments whose value is a canonical clone call
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        let Some(assignment) = module.place_assignment(expression) else {
            continue;
        };
        if assignment.operator != dir::AssignOperator::Assign {
            continue;
        }
        let Some(clone) = module.member_call(assignment.value) else {
            continue;
        };
        if clone.is_optional()
            || !clone.arguments.is_empty()
            || module.implemented_language_member(assignment.value)?
                != Some(dir::LanguageItem::Clone.member("clone"))
        {
            continue;
        }

        // require equal types without a coercion at the assignment
        let target_type = module.adjusted_type_id(assignment.target.into_any())?;
        let target_type = module.dir.strip_form(target_type)?;
        let source_type = module.adjusted_type_id(clone.receiver.into_any())?;
        let source_type = module.dir.strip_form(source_type)?;
        if !module.dir.types_match(target_type, source_type)?
            || module.satisfies_copy(source_type)?
            || module.place_access(assignment.target)? != Some(dir::Access::Exclusive)
        {
            continue;
        }

        // reject destinations that may overlap the clone source
        let destination = module.dereferenced_place(assignment.target);
        let Some(target) = module.access_resolution(destination) else {
            continue;
        };
        let Some(source) = module.access_resolution(clone.receiver) else {
            continue;
        };
        if target.path().starts_with(source.path()) || source.path().starts_with(target.path()) {
            continue;
        }

        // replace cloning assignment with Clone.cloneFrom
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("assignment replaces a value with its clone", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, assignment.target, clone.receiver)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one Clone.cloneFrom call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    source: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target_extent = module.source_extent(target.into_any())?;
    let source_extent = module.source_extent(source.into_any())?;
    if module.has_unretained_comment(extent, &[target_extent, source_extent])? {
        return Ok(None);
    }

    // call through the reference beneath an explicit assignment dereference
    let target = module.dereferenced_place(target);
    let target = module.expression_source(target, dir::OperatorPrecedence::Postfix)?;
    let source = module.expression_source(source, dir::OperatorPrecedence::Lowest)?;
    let replacement = format!("{target}.cloneFrom({source})");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("reuse destination storage with Clone.cloneFrom", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a self clone that would create overlapping borrows.
    #[test]
    fn test_accepts_self_clone() {
        let session = TestSession::dir(
            &PREFER_CLONE_FROM,
            r#"
import { rc } from "tspp:memory";

function retain(value: &exclusive rc.Rc<int32>): void {
    *value = value.clone();
}

function replaceAlias(target: &rc.Rc<int32>, source: &immutable rc.Rc<int32>): void {
    *target = source.clone();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace cloning between disjoint fields of one value.
    #[test]
    fn test_replaces_disjoint_field_clone() {
        let session = TestSession::dir(
            &PREFER_CLONE_FROM,
            r#"
import { rc } from "tspp:memory";

struct Pair {
    left: rc.Rc<int32>;
    right: rc.Rc<int32>;
}

function replace(pair: &exclusive Pair): void {
    pair.left = pair.right.clone();
}
"#,
        );

        session.assert_suggestions(
            r#"
import { rc } from "tspp:memory";

struct Pair {
    left: rc.Rc<int32>;
    right: rc.Rc<int32>;
}

function replace(pair: &exclusive Pair): void {
    pair.left.cloneFrom(pair.right);
}
"#,
        );
    }

    /// Leave Copy clone assignments to clone-on-copy.
    #[test]
    fn test_accepts_copy_clone_assignment() {
        let session = TestSession::dir(
            &PREFER_CLONE_FROM,
            r#"
function replace(target: &int32, source: int32): void {
    *target = source.clone();
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept the default implementation of a clone interface.
    #[test]
    fn test_accepts_default_clone_implementation() {
        let session = TestSession::dir(
            &PREFER_CLONE_FROM,
            r#"
newtype interface Duplicate {
    clone(&readonly this): ^this;

    cloneFrom(&this, source: &readonly this): void {
        *this = source.clone();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
