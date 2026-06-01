use destack_dir as dir;
use destack_workspace::{EmptyFunctionKind, LintSeverity};

use crate::rules::common::{
    CallableOwnerId, block_is_empty_without_comment, callable_owner_span,
    for_each_callable_signature,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty functions.
    ///
    /// Empty functions are often a sign of incomplete code. If intentional,
    /// add a comment explaining why the function is empty.
    #[lint(
        id = "no-empty-function",
        code = "LU013",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoEmptyFunction,
    "Disallow empty functions"
}

impl LintRule for NoEmptyFunction {
    fn meta(&self) -> &'static LintMeta {
        NoEmptyFunction::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect all callable signatures consistently
        for_each_callable_signature(ctx.dir.tree(), |owner_id, signature, body_expression_id| {
            let Some(body_expression_id) = body_expression_id else {
                return;
            };

            report_empty_function_body(ctx, meta, owner_id, signature, body_expression_id);
        });
    }
}

/// Report one empty function-like body from declarations or methods.
fn report_empty_function_body(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: CallableOwnerId,
    signature: &dir::FunctionSignature,
    body_expression_id: dir::LocalNodeId<dir::Expression>,
) {
    // require an empty uncommented block body
    let Some(block_id) = empty_body_block_id(ctx, body_expression_id) else {
        return;
    };

    // honor allowed empty function kinds
    let function_kind = empty_function_kind(owner_id, signature);
    if ctx
        .options()
        .correctness
        .no_empty_function_allow
        .contains(&function_kind)
    {
        return;
    }

    // skip disabled diagnostics
    let severity = match owner_id {
        CallableOwnerId::Declaration(declaration_id) => {
            ctx.get_effective_severity(meta, declaration_id)
        }
        CallableOwnerId::Member(member_id) => ctx.get_effective_severity(meta, member_id),
        CallableOwnerId::Property(property_id) => ctx.get_effective_severity(meta, property_id),
    };
    if !severity.is_enabled() {
        return;
    }

    // build the empty function diagnostic
    let mut diagnostic = LintReport::new(
        NO_EMPTY_FUNCTION.id,
        NO_EMPTY_FUNCTION.code,
        NO_EMPTY_FUNCTION.category,
        severity,
        "empty function",
        callable_owner_span(ctx.dir.tree(), owner_id),
    )
    .label("add implementation or a comment explaining why empty");

    // attach a safe comment insertion fix when enabled
    if ctx.compute_fixes
        && let Some(fix) = no_empty_function_fix(ctx, block_id)
    {
        diagnostic = diagnostic.fix(fix);
    }

    ctx.report(diagnostic);
}

/// Return one coarse function kind for option matching.
fn empty_function_kind(
    owner_id: CallableOwnerId,
    signature: &dir::FunctionSignature,
) -> EmptyFunctionKind {
    // getters, setters, and constructors are specialized method kinds first
    if let Some(role) = signature.role {
        return match role {
            dir::FunctionRole::Getter => EmptyFunctionKind::Getters,
            dir::FunctionRole::Setter => EmptyFunctionKind::Setters,
            dir::FunctionRole::Constructor => EmptyFunctionKind::Constructors,
            dir::FunctionRole::New | dir::FunctionRole::Call => {
                empty_non_accessor_function_kind(owner_id, signature)
            }
        };
    }

    empty_non_accessor_function_kind(owner_id, signature)
}

/// Return the non-accessor empty function kind for option matching.
fn empty_non_accessor_function_kind(
    owner_id: CallableOwnerId,
    signature: &dir::FunctionSignature,
) -> EmptyFunctionKind {
    // method override is a distinct opt in policy from ordinary methods
    if matches!(
        owner_id,
        CallableOwnerId::Member(_) | CallableOwnerId::Property(_)
    ) && signature.is_override
    {
        return EmptyFunctionKind::OverrideMethods;
    }

    // classify by callable owner and signature traits
    match owner_id {
        CallableOwnerId::Declaration(_) => match signature.form {
            dir::FunctionForm::Lambda => EmptyFunctionKind::ArrowFunctions,
            dir::FunctionForm::Function => {
                if signature.is_generator {
                    EmptyFunctionKind::GeneratorFunctions
                } else if signature.asynchrony == dir::Asynchrony::Async {
                    EmptyFunctionKind::AsyncFunctions
                } else {
                    EmptyFunctionKind::Functions
                }
            }
        },
        CallableOwnerId::Member(_) | CallableOwnerId::Property(_) => {
            if signature.is_generator {
                EmptyFunctionKind::GeneratorMethods
            } else if signature.asynchrony == dir::Asynchrony::Async {
                EmptyFunctionKind::AsyncMethods
            } else {
                EmptyFunctionKind::Methods
            }
        }
    }
}

/// Return one block id when a function body is empty and uncommented.
fn empty_body_block_id(
    ctx: &LintModuleContext<'_>,
    body_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::LocalNodeId<dir::Block>> {
    // require a block expression body
    let body_expression = ctx.dir.get(body_expression_id);
    let dir::Expression::Block(block_id) = body_expression else {
        return None;
    };

    // keep only empty uncommented blocks
    if block_is_empty_without_comment(ctx.dir.tree(), *block_id) {
        return Some(*block_id);
    }

    None
}

/// Build a safe fix that annotates an empty function block.
fn no_empty_function_fix(
    ctx: &LintModuleContext<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> Option<LintFix> {
    // replace the empty body with an explicit marker comment
    let block_span = ctx.dir.get_span(block_id);
    let edits = ctx
        .edit_builder()
        .replace(block_span, "{\n    // intentionally empty\n}")
        .into_edits();
    Some(LintFix::safe("Add intentional empty function comment").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_detects_empty_function.ds",
            "function foo() {}",
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_has_fix("no-empty-function");
    }

    #[test]
    fn test_detects_empty_arrow_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_detects_empty_arrow_function.ds",
            "const foo = () => {}",
        );
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_detects_empty_method() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_detects_empty_method.ds",
            r#"
class Foo {
    method() {}
}
"#,
        );
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_detects_empty_object_method() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_detects_empty_object_method.ds",
            r#"
const service = {
    handle() {}
};
"#,
        );
        test.result(result).assert_lint("no-empty-function");
    }

    #[test]
    fn test_allows_empty_constructor_when_configured() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction).with_options(|options| {
            options.correctness.no_empty_function_allow = vec![EmptyFunctionKind::Constructors];
        });
        let result = test.lint(
            "no_empty_function/test_allows_empty_constructor_when_configured.ds",
            r#"
class Service {
    constructor() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_empty_object_method_when_methods_are_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction).with_options(|options| {
            options.correctness.no_empty_function_allow = vec![EmptyFunctionKind::Methods];
        });
        let result = test.lint(
            "no_empty_function/test_allows_empty_object_method_when_methods_are_allowed.ds",
            r#"
const service = {
    handle() {}
};
"#,
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_body() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_allows_function_with_body.ds",
            "function foo() { return 1; }",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_with_comment() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_allows_function_with_comment.ds",
            "function foo() { /* intentionally empty */ }",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_allows_function_declaration_without_body() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_allows_function_declaration_without_body.ts",
            "declare function foo(): void;",
        );
        test.result(result).assert_no_lint("no-empty-function");
    }

    #[test]
    fn test_fix_adds_comment_to_empty_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_fix_adds_comment_to_empty_function.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_safe_fixed(
                r#"
function foo() {
    // intentionally empty
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_adds_comment_to_empty_arrow_function() {
        let test = TestProgram::for_rule_without_prelude(NoEmptyFunction);
        let result = test.lint(
            "no_empty_function/test_mutation_fix_adds_comment_to_empty_arrow_function.ds",
            r#"
const foo = () => {}
"#,
        );
        test.result(result)
            .assert_lint("no-empty-function")
            .assert_safe_fixed(
                r#"
const foo = () => {
    // intentionally empty
};
"#,
            );
    }
}
