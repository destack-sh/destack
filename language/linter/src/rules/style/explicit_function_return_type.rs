use crate::LintMeta;
use destack_ast::{
    self as ast, Declaration, FunctionKind, FunctionMode, Key, Member, Name, NodeVisitor,
    NodeVisitorOptions, Property, walk_expression,
};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Require explicit return type annotations on functions.
    ///
    /// Explicit return types improve code readability and catch errors early.
    /// They also provide better IDE support and documentation.
    #[lint(
        id = "explicit-function-return-type",
        code = "LY011",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub ExplicitFunctionReturnType,
    "Require explicit function return types"
}

impl LintRule for ExplicitFunctionReturnType {
    fn meta(&self) -> &'static LintMeta {
        ExplicitFunctionReturnType::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let constructor_name = ctx.repository.strings.intern("constructor");

        // declaration functions
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let Declaration::Function(declaration) = declaration else {
                continue;
            };

            if !signature_requires_explicit_return_type(
                ctx,
                &declaration.signature,
                None,
                constructor_name,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            report_missing_return_type(
                ctx,
                severity,
                ctx.tree.get_span(node_id),
                declaration.body,
                "function is missing explicit return type",
            );
        }

        // class and interface methods
        for node_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(node_id);
            let Member::Method {
                key,
                signature,
                body,
                ..
            } = member
            else {
                continue;
            };

            if !signature_requires_explicit_return_type(
                ctx,
                signature,
                key.as_ref(),
                constructor_name,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            report_missing_return_type(
                ctx,
                severity,
                ctx.tree.get_span(node_id),
                *body,
                "method is missing explicit return type",
            );
        }

        // object literal methods
        for node_id in ctx.tree.iter_nodes::<ast::Property>() {
            let property = ctx.tree.get(node_id);
            let Property::Method {
                key,
                signature,
                body,
                ..
            } = property
            else {
                continue;
            };

            if !signature_requires_explicit_return_type(
                ctx,
                signature,
                key.as_ref(),
                constructor_name,
            ) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            report_missing_return_type(
                ctx,
                severity,
                ctx.tree.get_span(node_id),
                *body,
                "method is missing explicit return type",
            );
        }
    }
}

/// Return true when one signature should report a missing return type.
fn signature_requires_explicit_return_type(
    ctx: &LintAstContext<'_>,
    signature: &ast::FunctionSignature,
    key: Option<&Key>,
    constructor_name: ast::StringId,
) -> bool {
    if signature.kind == FunctionKind::Lambda {
        return false;
    }

    if signature.return_type.is_some() {
        return false;
    }

    if signature.mode == Some(FunctionMode::Setter)
        || signature.mode == Some(FunctionMode::Constructor)
    {
        return false;
    }

    !method_key_is_constructor(ctx, key, constructor_name)
}

/// Return true when one method key is the constructor name.
fn method_key_is_constructor(
    _ctx: &LintAstContext<'_>,
    key: Option<&Key>,
    constructor_name: ast::StringId,
) -> bool {
    let Some(key) = key else {
        return false;
    };

    match key {
        Key::Name(Name::Identifier(name))
        | Key::Name(Name::String(name))
        | Key::Name(Name::Number(name))
        | Key::Private(name) => *name == constructor_name,
        Key::Expression(_) => false,
    }
}

/// Report one missing return type diagnostic with one conservative fix.
fn report_missing_return_type(
    ctx: &mut LintAstContext<'_>,
    severity: LintSeverity,
    function_span: Span,
    body_expression_id: Option<ast::LocalNodeId<ast::Expression>>,
    message: &str,
) {
    let mut diagnostic = LintDiagnostic::new(
        EXPLICIT_FUNCTION_RETURN_TYPE.id,
        EXPLICIT_FUNCTION_RETURN_TYPE.code,
        EXPLICIT_FUNCTION_RETURN_TYPE.category,
        severity,
        message,
        ctx.module.file_id,
        function_span,
    )
    .with_label("add return type annotation");

    if ctx.compute_fixes
        && let Some(body_expression_id) = body_expression_id
        && let Some(fix) = explicit_function_return_type_fix(ctx, function_span, body_expression_id)
    {
        diagnostic = diagnostic.with_fix(fix);
    }

    ctx.report(diagnostic);
}

/// Build a safe fix by inserting a `: void` return annotation.
fn explicit_function_return_type_fix(
    ctx: &LintAstContext<'_>,
    function_span: Span,
    body_expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    if contains_return_expression(ctx, body_expression_id) {
        return None;
    }

    let body_span = ctx.tree.get_span(body_expression_id);
    if body_span.start <= function_span.start || body_span.start >= function_span.end {
        return None;
    }

    let edits = ctx
        .edit_builder()
        .insert(body_span.start, ": void ")
        .into_edits();
    Some(LintFix::safe("Add explicit `void` return type").with_edits(edits))
}

/// Return true when an expression contains a return in this function body.
fn contains_return_expression(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    let mut visitor = ReturnDetectorVisitor::default();
    visitor.visit_expression(ctx.tree, expression_id, expression);
    visitor.has_return
}

/// Detect one return expression while skipping nested function bodies.
#[derive(Default)]
struct ReturnDetectorVisitor {
    /// Whether one return expression has been found.
    has_return: bool,
    /// Visitor options.
    options: NodeVisitorOptions,
}

impl NodeVisitor for ReturnDetectorVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &ast::Tree,
        id: ast::LocalNodeId<ast::Expression>,
        expression: &ast::Expression,
    ) {
        if self.has_return {
            return;
        }

        if matches!(expression, ast::Expression::Return { .. }) {
            self.has_return = true;
            return;
        }

        if let ast::Expression::Declaration(declaration) = expression {
            let declaration = tree.get(*declaration);
            if matches!(declaration, ast::Declaration::Function(_)) {
                return;
            }
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_missing_return_type() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_detects_missing_return_type.ts",
            r#"
function foo() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_explicit_return_type() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_allows_explicit_return_type.ts",
            r#"
function foo(): number {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_void_return_type() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_allows_void_return_type.ts",
            r#"
function foo(): void {
    console.log("hello");
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_arrow_functions() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_allows_arrow_functions.ts",
            r#"
const foo = () => 42;
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_detects_missing_return_type_on_class_method() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_detects_missing_return_type_on_class_method.ts",
            r#"
class Service {
    run() {
        return 1;
    }
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_constructor_without_return_type() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_allows_constructor_without_return_type.ts",
            r#"
class Service {
    constructor() {
        this.ready = true;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_setter_without_return_type() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_allows_setter_without_return_type.ts",
            r#"
class Service {
    set value(next: int32) {
        this._value = next;
    }
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_fix_adds_void_return_type_for_non_returning_method() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_fix_adds_void_return_type_for_non_returning_method.ts",
            r#"
class Logger {
    write(message: string) {
        sink(message);
    }
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type")
            .assert_safe_fixed(
                r#"
class Logger {
    write(message: string): void {
        sink(message)
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_adds_void_return_type_for_non_returning_function() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_fix_adds_void_return_type_for_non_returning_function.ts",
            r#"
function logMessage(message: string) {
    console.log(message);
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type")
            .assert_safe_fixed(
                r#"
function logMessage(message: string): void {
    console.log(message);
}
"#,
            );
    }

    #[test]
    fn test_no_fix_when_function_returns_value() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_no_fix_when_function_returns_value.ts",
            r#"
function toNumber(value: string) {
    return parseInt(value, 10);
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type")
            .assert_has_no_fix("explicit-function-return-type");
    }

    #[test]
    fn test_fix_ignores_return_inside_nested_function_body() {
        let test = TestProgram::for_rule_without_prelude(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "explicit_function_return_type/test_fix_ignores_return_inside_nested_function_body.ts",
            r#"
function run(message: string) {
    const format = (): string => {
        return `[${message}]`;
    };

    console.log(format());
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type")
            .assert_safe_fixed(
                r#"
function run(message: string): void {
    const format = (): string => {
        return `[${message}]`;
    };

    console.log(format());
}
"#,
            );
    }
}
