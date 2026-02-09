use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

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
    fn meta(&self) -> &'static LintMeta {
        NoThrowInResultFunction::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // intern "Result" name for type checking
        // #Cleanup: use proper Result symbol from standard library for-no-throw-in-result-function?
        // (though supporting any type that contains "Result" isn't actually that bad?)
        let result_name = ctx.program.strings.intern("Result");

        // collect functions with Result return type
        let functions: Vec<_> = ctx
            .tree
            .iter_nodes_of_type::<dir::Declaration>()
            .filter_map(|(decl_id, decl)| {
                if let dir::Declaration::Function {
                    signature,
                    body: Some(body_id),
                    ..
                } = decl
                {
                    // check if return type is Result
                    if is_result_return_type(ctx.tree, signature, result_name) {
                        return Some((decl_id, *body_id));
                    }
                }
                None
            })
            .collect();

        // check each Result-returning function for throws
        for (decl_id, body_id) in functions {
            let mut visitor = ThrowInResultVisitor::new(ctx, meta, decl_id);
            visitor.run(body_id);
        }
    }
}

/// Check if a function signature has a Result return type.
fn is_result_return_type(
    tree: &dir::NodeTree,
    signature: &dir::FunctionSignature,
    result_name: StringId,
) -> bool {
    // get the return type expression
    let Some(return_type_id) = signature.return_type else {
        return false;
    };

    // check if it's a reference to Result
    let return_type = tree.get(return_type_id);
    match return_type {
        // match Result<T, E> via reference with static arguments
        dir::Expression::LocalReference {
            path,
            static_arguments: Some(_),
            ..
        }
        | dir::Expression::ModuleReference {
            path,
            static_arguments: Some(_),
            ..
        }
        | dir::Expression::GlobalReference {
            path,
            static_arguments: Some(_),
            ..
        } => path.first_segment() == Some(result_name),
        // match bare Result (unlikely but possible)
        dir::Expression::LocalReference { path, .. }
        | dir::Expression::ModuleReference { path, .. }
        | dir::Expression::GlobalReference { path, .. } => {
            path.first_segment() == Some(result_name)
        }
        _ => false,
    }
}

/// Visitor that finds throw expressions in a function body.
struct ThrowInResultVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The declaration node id for severity checking.
    decl_id: dir::LocalNodeId<dir::Declaration>,
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
        decl_id: dir::LocalNodeId<dir::Declaration>,
    ) -> Self {
        Self {
            ctx,
            meta,
            decl_id,
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
        let severity = self.ctx.get_effective_severity(self.meta, self.decl_id);
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

impl NodeVisitor for ThrowInResultVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // track nested function/class declarations (which can contain functions)
        if let dir::Expression::Declaration { declaration } = expression {
            let decl = tree.get(*declaration);
            if matches!(decl, dir::Declaration::Function { .. }) {
                self.function_depth += 1;
            }
        }

        // only check throws in the top-level function, not nested functions
        if self.function_depth == 0 && matches!(expression, dir::Expression::Throw { .. }) {
            self.report(id);
        }

        // walk children
        walk_expression(self, tree, id, expression);

        // restore function depth
        if let dir::Expression::Declaration { declaration } = expression {
            let decl = tree.get(*declaration);
            if matches!(decl, dir::Declaration::Function { .. }) {
                self.function_depth -= 1;
            }
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
}
