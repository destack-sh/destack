use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    declaration_has_extends_heritage, expression_target_symbol, expression_unwrap_statement,
    source_text_contains_comment_token,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary constructors.
    ///
    /// An empty constructor is unnecessary and can be removed.
    #[lint(
        id = "no-useless-constructor",
        code = "LU038",
        category = Suspicious,
        level = Dir,
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUselessConstructor::meta()
    }

    /// Check module DIR members for redundant constructors.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect class constructors with bodies
        for member_id in ctx.dir.iter_node_ids_of_type::<dir::Member>() {
            let member = ctx.dir.get(member_id);
            let dir::Member::Method {
                visibility,
                signature,
                body: Some(body_expression_id),
                ..
            } = member
            else {
                continue;
            };

            // keep non-constructors out of this rule
            if signature.role != Some(dir::FunctionRole::Constructor) {
                continue;
            }

            // keep constructors with useful accessibility
            if constructor_has_useful_accessibility(ctx, member_id, *visibility) {
                continue;
            }

            // keep parameter property style constructors out of this rule
            if constructor_parameters_have_modifiers(ctx, signature.parameters.as_slice()) {
                continue;
            }

            // report only empty constructors without parameters or direct super passthroughs
            let is_useless_constructor = (constructor_body_is_empty(ctx, *body_expression_id)
                && signature.parameters.is_empty())
                || is_redundant_super_passthrough_constructor(
                    ctx,
                    signature.parameters.as_slice(),
                    *body_expression_id,
                );
            if !is_useless_constructor {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, member_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintReport::new(
                NO_USELESS_CONSTRUCTOR.id,
                NO_USELESS_CONSTRUCTOR.code,
                NO_USELESS_CONSTRUCTOR.category,
                severity,
                "useless constructor",
                ctx.get_span(member_id),
            )
            .label("remove this constructor");

            // keep fixes out of explicit modifier and comment carrying constructors
            let member_span = ctx.get_span(member_id);
            let member_text = ctx.get_span_text(member_span);
            if visibility.is_none() && !source_text_contains_comment_token(member_text) {
                let edits = ctx.edit_builder().delete(member_span).into_edits();
                let fix =
                    LintFix::suggestion("Remove useless constructor declaration").with_edits(edits);
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one constructor accessibility makes the constructor useful.
fn constructor_has_useful_accessibility(
    ctx: &LintModuleContext<'_>,
    member_id: dir::LocalNodeId<dir::Member>,
    visibility: Option<dir::Visibility>,
) -> bool {
    // keep private and protected constructors always
    if matches!(
        visibility,
        Some(dir::Visibility::Private | dir::Visibility::Protected)
    ) {
        return true;
    }

    // keep public constructors on subclasses
    if visibility == Some(dir::Visibility::Public) {
        return member_parent_class_has_super_class(ctx, member_id);
    }

    false
}

/// Return true when the enclosing class extends something.
fn member_parent_class_has_super_class(
    ctx: &LintModuleContext<'_>,
    member_id: dir::LocalNodeId<dir::Member>,
) -> bool {
    let Some(parent_id) = ctx.dir.get_parent(member_id.id) else {
        return false;
    };
    if parent_id.ty != dir::NodeType::Declaration {
        return false;
    }

    let declaration_id = parent_id.into_typed::<dir::Declaration>();
    let declaration = ctx.dir.get(declaration_id);
    if !matches!(declaration, dir::Declaration::Class(_)) {
        return false;
    }

    declaration_has_extends_heritage(declaration)
}

/// Return true when one constructor body is empty.
fn constructor_body_is_empty(
    ctx: &LintModuleContext<'_>,
    body_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let body_expression = ctx.dir.get(body_expression_id);
    let dir::Expression::Block(block) = body_expression else {
        return false;
    };
    let block = ctx.dir.get(*block);

    block.is_empty()
}

/// Return true when one constructor uses parameter modifiers.
fn constructor_parameters_have_modifiers(
    ctx: &LintModuleContext<'_>,
    parameter_ids: &[dir::LocalNodeId<dir::Parameter>],
) -> bool {
    parameter_ids.iter().copied().any(|parameter_id| {
        let parameter = ctx.dir.get(parameter_id);
        matches!(
            parameter,
            dir::Parameter::Named {
                visibility: Some(_),
                ..
            } | dir::Parameter::Named {
                is_readonly: true,
                ..
            } | dir::Parameter::VariadicNamed {
                visibility: Some(_),
                ..
            } | dir::Parameter::VariadicNamed {
                is_readonly: true,
                ..
            }
        )
    })
}

/// Return true when a constructor only forwards parameters to one `super(...)` call.
fn is_redundant_super_passthrough_constructor(
    ctx: &LintModuleContext<'_>,
    parameter_ids: &[dir::LocalNodeId<dir::Parameter>],
    body_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // require one block body with exactly one expression
    let body_expression = ctx.dir.get(body_expression_id);
    let dir::Expression::Block(block) = body_expression else {
        return false;
    };
    let block = ctx.dir.get(*block);
    if block.len() != 1 {
        return false;
    }

    // unwrap statement form to the effective expression
    let expression_id =
        expression_unwrap_statement(ctx.dir.tree(), block.first_expression().unwrap());
    let expression = ctx.dir.get(expression_id);
    let dir::Expression::Call {
        position: _,
        left,
        generic_arguments,
        arguments,
    } = expression
    else {
        return false;
    };
    if !generic_arguments.is_empty() {
        return false;
    }

    // require a direct `super(...)` call
    if !matches!(ctx.dir.get(*left), dir::Expression::Super) {
        return false;
    }

    // require one positional or spread argument per parameter in source order
    if parameter_ids.len() != arguments.len() {
        return false;
    }

    parameter_ids
        .iter()
        .zip(arguments.iter())
        .all(|(parameter_id, argument_id)| {
            match (
                constructor_parameter_binding(ctx, *parameter_id),
                constructor_argument_binding(ctx, *argument_id),
            ) {
                (
                    Some((parameter_symbol, parameter_is_variadic)),
                    Some((argument_symbol, argument_is_spread)),
                ) => {
                    argument_symbol == parameter_symbol.into_global(ctx.module_id())
                        && parameter_is_variadic == argument_is_spread
                }
                _ => false,
            }
        })
}

/// Return the simple parameter symbol and variadic flag for one constructor parameter.
fn constructor_parameter_binding(
    ctx: &LintModuleContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<(dir::LocalSymbolId, bool)> {
    let parameter = ctx.dir.get(parameter_id);
    match parameter {
        dir::Parameter::Named { default, .. } if default.is_none() => ctx
            .local_symbol_for_node(parameter_id)
            .map(|symbol| (symbol, false)),
        dir::Parameter::VariadicNamed { .. } => ctx
            .local_symbol_for_node(parameter_id)
            .map(|symbol| (symbol, true)),
        _ => None,
    }
}

/// Return the simple argument symbol and spread flag for one constructor argument.
fn constructor_argument_binding(
    ctx: &LintModuleContext<'_>,
    argument_id: dir::LocalNodeId<dir::Argument>,
) -> Option<(dir::GlobalSymbolId, bool)> {
    let argument = ctx.dir.get(argument_id);
    let (value_expression_id, is_spread) = match argument {
        dir::Argument::Positional { value } => (*value, false),
        dir::Argument::Spread { value, .. } => (*value, true),
        dir::Argument::Named { .. } | dir::Argument::Labeled { .. } | dir::Argument::Error => {
            return None;
        }
    };

    let value_symbol = expression_target_symbol(ctx, value_expression_id)?;
    Some((value_symbol, is_spread))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag empty constructors.
    #[test]
    fn test_detects_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
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

    /// Remove empty constructors when no modifier or comment blocks the edit.
    #[test]
    fn test_fix_removes_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
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
            .assert_suggested_fixed(
                r#"
class Foo {

    value() {
        return 1;
    }
}
"#,
            );
    }

    /// Allow constructors that initialize state.
    #[test]
    fn test_allows_constructor_with_initialization() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
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

    /// Allow empty constructors that still declare parameters.
    #[test]
    fn test_allows_constructor_with_params() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_constructor_with_params.ds",
            r#"
class Foo {
    constructor(x: int32) {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Allow private empty constructors.
    #[test]
    fn test_allows_private_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_private_empty_constructor.ds",
            r#"
class Foo {
    private constructor() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Allow protected empty constructors.
    #[test]
    fn test_allows_protected_empty_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_protected_empty_constructor.ds",
            r#"
class Foo {
    protected constructor() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Report public constructors without fixes when modifiers are present.
    #[test]
    fn test_reports_public_empty_constructor_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
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

    /// Allow public constructors on subclasses.
    #[test]
    fn test_allows_public_empty_constructor_on_subclass() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_public_empty_constructor_on_subclass.ds",
            r#"
class Foo extends Base {
    public constructor() {
        super()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Keep comment carrying constructors out of autofix.
    #[test]
    fn test_reports_commented_empty_constructor_without_fix() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
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

    /// Flag redundant super passthrough constructors.
    #[test]
    fn test_detects_redundant_super_passthrough_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_detects_redundant_super_passthrough_constructor.ds",
            r#"
class Base {}

class Foo extends Base {
    constructor(x: int32, y: string) {
        super(x, y);
    }
}
"#,
        );
        test.result(result).assert_lint("no-useless-constructor");
    }

    /// Allow parameter property style constructors.
    #[test]
    fn test_allows_constructor_with_parameter_modifiers() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_constructor_with_parameter_modifiers.ds",
            r#"
class Base {}

class Foo extends Base {
    constructor(public value: int32) {
        super(value);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Allow readonly parameter property style constructors.
    #[test]
    fn test_allows_constructor_with_readonly_parameter_property() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_constructor_with_readonly_parameter_property.ds",
            r#"
class Base {}

class Foo extends Base {
    constructor(readonly public value: int32) {
        super(value);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    /// Suggest removal instead of claiming a safe fix.
    #[test]
    fn test_reports_redundant_constructor_with_suggestion() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_reports_redundant_constructor_with_suggestion.ds",
            r#"
class Foo {
    constructor() {}
}
"#,
        );
        test.result(result)
            .assert_lint("no-useless-constructor")
            .assert_has_fix("no-useless-constructor")
            .assert_suggested_fixed(
                r#"
class Foo {}
"#,
            );
    }

    /// Allow super constructors when arguments change.
    #[test]
    fn test_allows_super_constructor_when_arguments_change() {
        let test = TestProgram::for_rule_without_prelude(NoUselessConstructor);
        let result = test.lint_dir(
            "no_useless_constructor/test_allows_super_constructor_when_arguments_change.ds",
            r#"
class Base {}

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
