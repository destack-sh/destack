use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_enters_nested_declaration_scope,
    function_signature_return_type_contains_reference_segment,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `throw` in functions returning `Result`.
    ///
    /// Functions that return `Result<T, E>` should use `Result.err()` instead
    /// of throwing. The Result type is designed for recoverable errors.
    #[lint(
        id = "no-throw-in-result-function",
        code = "LC028",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoThrowInResultFunction,
    "Disallow throw in Result-returning functions"
}

impl LintRule for NoThrowInResultFunction {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoThrowInResultFunction::meta()
    }

    /// Check module DIR nodes for throws in Result-returning callables.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // intern "Result" name for return type expression checks
        let result_name = ctx.repository.strings.intern("Result");

        // collect declaration functions with Result return type
        let mut callables: Vec<_> = ctx
            .tree
            .iter_nodes_of_type::<dir::Declaration>()
            .filter_map(|(decl_id, decl)| {
                // keep function declarations with executable bodies
                if let dir::Declaration::Function(declaration) = decl {
                    let body_id = declaration.body?;

                    // check if return type is Result or wraps Result in static arguments
                    if function_signature_return_type_contains_reference_segment(
                        ctx.tree,
                        &declaration.signature,
                        result_name,
                    ) {
                        return Some((CallableOwner::Declaration(decl_id), body_id));
                    }
                }
                None
            })
            .collect();

        // collect member methods with Result return type
        for (member_id, member) in ctx.tree.iter_nodes_of_type::<dir::Member>() {
            let dir::Member::Method {
                signature,
                body: Some(body_id),
                ..
            } = member
            else {
                continue;
            };

            // enforce this lint guard
            if !function_signature_return_type_contains_reference_segment(
                ctx.tree,
                signature,
                result_name,
            ) {
                continue;
            }

            callables.push((CallableOwner::Member(member_id), *body_id));
        }

        // check each Result returning callable for throws
        for (owner, body_id) in callables {
            let mut visitor = ThrowInResultVisitor::new(ctx, meta, owner);
            visitor.run(body_id);
        }
    }
}

/// Visitor that finds throw expressions in a function body.
struct ThrowInResultVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The callable owner node for severity checking.
    owner: CallableOwner,
    /// Current depth in nested functions (to avoid checking nested functions).
    function_depth: usize,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ThrowInResultVisitor<'a, 'b> {
    /// Build a new visitor.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        owner: CallableOwner,
    ) -> Self {
        Self {
            ctx,
            meta,
            owner,
            function_depth: 0,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the function body.
    fn run(&mut self, body_id: dir::LocalNodeId<dir::Expression>) {
        let tree = self.ctx.tree;
        let body = tree.get(body_id);
        self.visit_expression(tree, body_id, body);
    }

    /// Report a throw in Result function.
    fn report(&mut self, throw_id: dir::LocalNodeId<dir::Expression>) {
        // check effective severity
        let severity = match self.owner {
            CallableOwner::Declaration(declaration_id) => {
                self.ctx.get_effective_severity(self.meta, declaration_id)
            }
            CallableOwner::Member(member_id) => {
                self.ctx.get_effective_severity(self.meta, member_id)
            }
        };

        // skip disabled diagnostics
        if !severity.is_enabled() {
            return;
        }

        // report
        let span = self.ctx.get_span(throw_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_THROW_IN_RESULT_FUNCTION.id,
                NO_THROW_IN_RESULT_FUNCTION.code,
                NO_THROW_IN_RESULT_FUNCTION.category,
                severity,
                "throw in Result-returning function",
                self.ctx.module.file_id,
                span,
            )
            .with_label("use Result.err() instead of throw"),
        );
    }
}

/// One callable owner node for severity lookup.
#[derive(Clone, Copy)]
enum CallableOwner {
    /// One declaration function.
    Declaration(dir::LocalNodeId<dir::Declaration>),
    /// One member method.
    Member(dir::LocalNodeId<dir::Member>),
}

impl NodeVisitor for ThrowInResultVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // track nested declaration scopes, their control flow is local
        let enters_nested_scope = expression_enters_nested_declaration_scope(tree, expression);

        // enter nested declaration scope depth
        if enters_nested_scope {
            self.function_depth += 1;
        }

        // only check throws in the top level function, not nested functions
        if self.function_depth == 0 && matches!(expression, dir::Expression::Throw { .. }) {
            self.report(id);
        }

        // walk children
        walk_expression(self, tree, id, expression);

        // leave nested declaration scope depth
        if enters_nested_scope {
            self.function_depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag throw in Result-returning function.
    #[test]
    fn test_flags_throw_in_result_function() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_flags_throw_in_result_function.ds",
            r#"
function parse(input: string): Result<number, string> {
    if (input == "") {
        throw "empty input";
    }
    Result.ok(42)
}
"#,
        );
        test.result(result)
            .assert_lint("no-throw-in-result-function");
    }

    /// Allow Result.err in Result-returning function.
    #[test]
    fn test_allows_result_err() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_allows_result_err.ds",
            r#"
function parse(input: string): Result<number, string> {
    if (input == "") {
        return Result.err("empty input");
    }
    Result.ok(42)
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-throw-in-result-function");
    }

    /// Allow throw in non-Result function.
    #[test]
    fn test_allows_throw_in_void_function() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_allows_throw_in_void_function.ds",
            r#"
function fail(msg: string) {
    throw msg;
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-throw-in-result-function");
    }

    /// Allow throw in nested function inside Result function.
    #[test]
    fn test_allows_throw_in_nested_function() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_allows_throw_in_nested_function.ds",
            r#"
function outer(): Result<number, string> {
    const inner = () => {
        throw "nested throw is ok";
    };
    Result.ok(42)
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-throw-in-result-function");
    }

    /// Flag throw when using a qualified Result type path.
    #[test]
    fn test_flags_throw_in_qualified_result_function() {
        let test = TestProgram::for_rule_without_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_flags_throw_in_qualified_result_function.ds",
            r#"
namespace core {
    export type Result<T, E> = T | E;
}

function parse(input: string): core.Result<number, string> {
    throw "bad";
}
"#,
        );
        test.result(result)
            .assert_lint("no-throw-in-result-function");
    }

    /// Flag throw in Result returning class methods.
    #[test]
    fn test_flags_throw_in_result_method() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_flags_throw_in_result_method.ds",
            r#"
class Parser {
    parse(): Result<number, string> {
        throw "bad";
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-throw-in-result-function");
    }

    /// Flag throw in Promise wrapped Result return type.
    #[test]
    fn test_flags_throw_in_promise_wrapped_result_return_type() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_flags_throw_in_promise_wrapped_result_return_type.ds",
            r#"
async function parse(): Promise<Result<number, string>> {
    throw "bad";
}
"#,
        );
        test.result(result)
            .assert_lint("no-throw-in-result-function");
    }

    /// Allow throw in nested declarations inside Result functions.
    #[test]
    fn test_allows_throw_in_nested_class_method() {
        let test = TestProgram::for_rule_with_prelude(NoThrowInResultFunction);
        let result = test.lint_dir(
            "no_throw_in_result_function/test_allows_throw_in_nested_class_method.ds",
            r#"
function parse(): Result<number, string> {
    class Nested {
        run() {
            throw "nested throw is ok";
        }
    }

    return Result.ok(1);
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-throw-in-result-function");
    }
}
