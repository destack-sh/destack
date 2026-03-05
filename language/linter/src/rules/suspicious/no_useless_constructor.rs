use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_path_segments, expression_unwrap_parenthesized_syntax};
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary constructors.
    ///
    /// An empty constructor is unnecessary and can be removed.
    #[lint(
        id = "no-useless-constructor",
        code = "LU038",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessConstructor,
    "Disallow useless constructors"
}

impl LintRule for NoUselessConstructor {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessConstructor::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(node_id);
            let ast::Member::Method {
                modifiers,
                key,
                signature,
                body: Some(body_id),
                ..
            } = member
            else {
                continue;
            };

            // check if this is a constructor (no key, mode is Constructor)
            if key.is_some() {
                continue;
            }

            // check if mode is constructor
            let Some(ast::FunctionMode::Constructor) = signature.mode else {
                continue;
            };

            // skip protected or private constructors: they carry access semantics
            if modifiers
                .and_then(|modifier| modifier.visibility)
                .is_some_and(|visibility| {
                    matches!(
                        visibility,
                        ast::Visibility::Private | ast::Visibility::Protected
                    )
                })
            {
                continue;
            }

            // check source parity: constructors are useless when they are empty
            // or only pass parameters through to `super(...)`
            let is_useless_constructor = (is_empty_body(ctx, *body_id)
                && signature.dynamic_parameters.is_empty())
                || is_redundant_super_passthrough_constructor(
                    ctx,
                    signature.dynamic_parameters.as_slice(),
                    *body_id,
                );
            if is_useless_constructor {
                let severity = ctx.get_effective_severity(meta, *body_id);
                if !severity.is_enabled() {
                    continue;
                }

                // attach a safe fix only for constructors without explicit modifiers
                let mut diagnostic = LintDiagnostic::new(
                    NO_USELESS_CONSTRUCTOR.id,
                    NO_USELESS_CONSTRUCTOR.code,
                    NO_USELESS_CONSTRUCTOR.category,
                    severity,
                    "useless constructor",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("empty constructor can be removed");

                let member_span = ctx.tree.get_span(node_id);
                let member_text = ctx.get_span_text(member_span);
                if modifiers.is_none() && !contains_comment_token(&member_text) {
                    let member_span = ctx.tree.get_span(node_id);
                    let edits = ctx.edit_builder().delete(member_span).into_edits();
                    let fix =
                        LintFix::safe("Remove useless constructor declaration").with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when a constructor only forwards parameters to one `super(...)` call.
fn is_redundant_super_passthrough_constructor(
    ctx: &LintModuleAstContext<'_>,
    parameter_ids: &[ast::LocalNodeId<ast::Parameter>],
    body_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // resolve the single body expression
    let body_expression = ctx.tree.get(body_id);
    let ast::Expression::Block(block_id) = body_expression else {
        return false;
    };
    let block = ctx.tree.get(*block_id);
    if block.expressions.len() != 1 {
        return false;
    }

    // unwrap statement wrappers
    let mut expression_id = block.expressions[0];
    if let ast::Expression::Statement(inner_id) = ctx.tree.get(expression_id) {
        expression_id = *inner_id;
    }

    // require one `super(...)` call
    let expression = ctx.tree.get(expression_id);
    let ast::Expression::Call {
        left,
        dynamic_arguments,
        ..
    } = expression
    else {
        return false;
    };
    let left_id = expression_unwrap_parenthesized_syntax(ctx.tree, *left);
    let left_expression = ctx.tree.get(left_id);
    let is_super_callee = matches!(left_expression, ast::Expression::Super)
        || expression_path_segments(ctx.tree, left_id).is_some_and(|path_segments| {
            path_segments.len() == 1 && ctx.strings.get(path_segments[0]).as_ref() == "super"
        });
    if !is_super_callee {
        return false;
    }

    // require one argument per parameter in source order
    if parameter_ids.len() != dynamic_arguments.len() {
        return false;
    }
    parameter_ids
        .iter()
        .zip(dynamic_arguments.iter())
        .all(|(parameter_id, argument_id)| {
            match (
                constructor_parameter_name(ctx, *parameter_id),
                constructor_argument_name(ctx, *argument_id),
            ) {
                (
                    Some((parameter_name, parameter_is_variadic)),
                    Some((argument_name, argument_is_spread)),
                ) => parameter_name == argument_name && parameter_is_variadic == argument_is_spread,
                _ => false,
            }
        })
}

/// Return parameter name and variadic flag for simple constructor parameters.
fn constructor_parameter_name(
    ctx: &LintModuleAstContext<'_>,
    parameter_id: ast::LocalNodeId<ast::Parameter>,
) -> Option<(ast::StringId, bool)> {
    let parameter = ctx.tree.get(parameter_id);
    match parameter {
        ast::Parameter::Named { name, default, .. } if default.is_none() => Some((*name, false)),
        ast::Parameter::VariadicNamed { name, .. } => Some((*name, true)),
        _ => None,
    }
}

/// Return argument name and spread flag for simple constructor arguments.
fn constructor_argument_name(
    ctx: &LintModuleAstContext<'_>,
    argument_id: ast::LocalNodeId<ast::Argument>,
) -> Option<(ast::StringId, bool)> {
    let argument = ctx.tree.get(argument_id);
    let (value_id, is_spread) = match argument {
        ast::Argument::Positional { value, .. } => (*value, false),
        ast::Argument::Spread { value, .. } => (*value, true),
        _ => return None,
    };

    let Some(path_segments) = expression_path_segments(ctx.tree, value_id) else {
        return None;
    };
    if path_segments.len() != 1 {
        return None;
    }

    Some((path_segments[0], is_spread))
}

/// Return true when one constructor text may contain comments.
fn contains_comment_token(text: &str) -> bool {
    text.contains("//") || text.contains("/*")
}

/// Check if a body is empty.
fn is_empty_body(
    ctx: &LintModuleAstContext<'_>,
    body_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let body = ctx.tree.get(body_id);
    match body {
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            block.expressions.is_empty()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_detects_empty_constructor.ds",
            r#"
class Foo {
    constructor() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-constructor")
            .assert_has_fix("no-useless-constructor");
    }

    #[test]
    fn test_fix_removes_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_fix_removes_empty_constructor.ds",
            r#"
class Foo {
    constructor() {}
    value() {
        return 1
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-constructor")
            .assert_has_fix("no-useless-constructor")
            .assert_safe_fixed(
                r#"
class Foo {

    value() {
        return 1;
    }
}
"#,
            );
    }

    #[test]
    fn test_allows_constructor_with_initialization() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_allows_constructor_with_initialization.ds",
            r#"
class Foo {
    constructor() {
        this.x = 1
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_constructor_with_params() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_allows_constructor_with_params.ds",
            r#"
class Foo {
    constructor(x: int32) {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_private_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_allows_private_empty_constructor.ds",
            r#"
class Foo {
    private constructor() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_protected_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_allows_protected_empty_constructor.ds",
            r#"
class Foo {
    protected constructor() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    #[test]
    fn test_reports_public_empty_constructor_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_reports_public_empty_constructor_without_fix.ds",
            r#"
class Foo {
    public constructor() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-constructor")
            .assert_has_no_fix("no-useless-constructor");
    }

    #[test]
    fn test_reports_commented_empty_constructor_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_reports_commented_empty_constructor_without_fix.ds",
            r#"
class Foo {
    constructor() {
        // keep for docs
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-constructor")
            .assert_has_no_fix("no-useless-constructor");
    }

    #[test]
    fn test_detects_redundant_super_passthrough_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_detects_redundant_super_passthrough_constructor.ds",
            r#"
class Foo extends Base {
    constructor(x: int32, y: string) {
        super(x, y);
    }
}
"#,
        );
        test.result(result).assert_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_super_constructor_when_arguments_change() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_ast(
            "no_useless_constructor/test_allows_super_constructor_when_arguments_change.ds",
            r#"
class Foo extends Base {
    constructor(x: int32) {
        super(x + 1);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }
}
