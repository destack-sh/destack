use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_target_symbol;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow using the Array constructor.
    ///
    /// Array constructors are confusing because single argument calls
    /// create sparse arrays instead of arrays with values.
    #[lint(
        id = "no-array-constructor",
        code = "LC005",
        category = Correctness,
        level = Dir,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoArrayConstructor,
    "Disallow Array constructor usage"
}

impl LintRule for NoArrayConstructor {
    /// Return lint metadata.
    fn meta(&self) -> &'static crate::LintMeta {
        NoArrayConstructor::meta()
    }

    /// Check module DIR nodes for Array constructor calls.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        // resolve lint metadata
        let meta = self.meta();

        // walk the module for array constructor calls
        let mut visitor = ArrayConstructorVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags Array constructor calls.
struct ArrayConstructorVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ArrayConstructorVisitor<'a, 'b> {
    /// Build a visitor for Array constructor checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        // resolve the well known Array symbol for this module
        let array_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Array);

        // prepare visitor state
        Self {
            ctx,
            meta,
            array_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when no Array symbol is available
        if self.array_symbol.is_none() {
            return;
        }

        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a call or constructor for Array usage.
    fn check_array_constructor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        kind: &'static str,
    ) {
        // ignore non array references
        let Some(array_symbol) = self.array_symbol else {
            return;
        };
        let Some(target_symbol) = expression_target_symbol(self.ctx.tree, left) else {
            return;
        };
        if target_symbol != array_symbol {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_ARRAY_CONSTRUCTOR.id,
                NO_ARRAY_CONSTRUCTOR.code,
                NO_ARRAY_CONSTRUCTOR.category,
                severity,
                "avoid using the Array constructor",
                self.ctx.module.file_id,
                span,
            )
            .with_label(format!("replace this {kind} with an array literal")),
        );
    }
}

impl NodeVisitor for ArrayConstructorVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for array constructor calls
        if let Some((kind, left)) = array_constructor_reference(expression) {
            self.check_array_constructor(id, left, kind);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Identify Array constructor call forms.
fn array_constructor_reference(
    expression: &dir::Expression,
) -> Option<(&'static str, dir::LocalNodeId<dir::Expression>)> {
    // match call and constructor expressions
    match expression {
        dir::Expression::Call { left, .. } => Some(("call", *left)),
        dir::Expression::New { left, .. } => Some(("constructor", *left)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LintLevel;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_array_constructor_call() {
        let test = TestProgram::for_rule_with_builtins(NoArrayConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let items = Array(1, 2);
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-array-constructor");
    }

    #[test]
    fn test_flags_array_constructor_new() {
        let test = TestProgram::for_rule_with_builtins(NoArrayConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let items = new Array(1);
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_lint("no-array-constructor");
    }

    #[test]
    fn test_allows_array_literal() {
        let test = TestProgram::for_rule_with_builtins(NoArrayConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let items = [1, 2];
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-array-constructor");
    }

    #[test]
    fn test_allows_shadowed_array() {
        let test = TestProgram::for_rule_with_builtins(NoArrayConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let Array = (value: number): number => value;
let item = Array(1);
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-array-constructor");
    }

    #[test]
    fn test_allows_shadowed_array_in_function() {
        let test = TestProgram::for_rule_with_builtins(NoArrayConstructor);
        let result = test.lint(
            "test.ds",
            r#"
let build = (Array: (value: number) => number): number => {
    return Array(1);
};
"#,
            LintLevel::Dir,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-array-constructor");
    }
}
