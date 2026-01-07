use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{is_array_type, is_string_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow spread syntax in contexts where it's incorrect.
    ///
    /// Spreading non-iterable values or using spread in incorrect contexts
    /// can lead to runtime errors or unexpected behavior. This rule flags
    /// common misuses of spread syntax.
    #[lint(
        id = "no-misused-spread",
        code = "LC033",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [RequireWellKnownSymbol(WellKnownSymbol::Array), RequireWellKnownSymbol(WellKnownSymbol::String)],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoMisusedSpread,
    "Disallow spread in incorrect contexts"
}

impl LintRule for NoMisusedSpread {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoMisusedSpread::meta()
    }

    /// Check module DIR nodes for misused spread syntax.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let array_symbol = ctx.get_well_known_symbol(WellKnownSymbol::Array);
        let string_symbol = ctx.get_well_known_symbol(WellKnownSymbol::String);
        let mut visitor = MisusedSpreadVisitor::new(ctx, meta, array_symbol, string_symbol);
        visitor.run();
    }
}

/// Visitor that flags misused spread syntax.
struct MisusedSpreadVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Array symbol.
    array_symbol: Option<dir::GlobalSymbolId>,
    /// The String symbol.
    string_symbol: Option<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> MisusedSpreadVisitor<'a, 'b> {
    /// Build a visitor for misused spread checks.
    fn new(
        ctx: &'a mut LintModuleDirContext<'b>,
        meta: &'a LintMeta,
        array_symbol: Option<dir::GlobalSymbolId>,
        string_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        Self {
            ctx,
            meta,
            array_symbol,
            string_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check if a type is iterable (array or string).
    fn is_iterable_type(&self, type_id: dir::LocalTypeId) -> bool {
        is_array_type(self.ctx.types, type_id, self.array_symbol)
            || is_string_type(self.ctx.types, type_id, self.string_symbol)
    }

    /// Check if a type is spreadable in an object context.
    fn is_object_spreadable(&self, type_id: dir::LocalTypeId) -> bool {
        let ty = self.ctx.types.get_type(type_id);

        match ty {
            // objects can be spread
            dir::Type::Object { .. } => true,
            // references might be objects
            dir::Type::Reference { .. } => true,
            // follow wrappers
            dir::Type::Value { value } => self.is_object_spreadable(*value),
            dir::Type::Mutable { right, .. }
            | dir::Type::ValueOf { right, .. }
            | dir::Type::ReferenceOf { right, .. } => self.is_object_spreadable(*right),
            // unions are spreadable if all elements are
            dir::Type::Union { elements } => elements.iter().all(|e| self.is_object_spreadable(*e)),
            // primitives (except null/undefined) are not directly spreadable
            dir::Type::TypeLiteral { value } => !matches!(
                value,
                dir::TypeLiteral::Null | dir::TypeLiteral::Undefined | dir::TypeLiteral::Never
            ),
            _ => false,
        }
    }

    /// Check a spread in an array context.
    fn check_array_spread(
        &mut self,
        spread_id: dir::LocalNodeId<dir::Argument>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // get the spread value type
        let Some(type_id) = self.ctx.expression_type_id(value_id) else {
            return;
        };

        // check if the type is iterable
        if self.is_iterable_type(type_id) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, spread_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(spread_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_MISUSED_SPREAD.id,
                NO_MISUSED_SPREAD.code,
                NO_MISUSED_SPREAD.category,
                severity,
                "spreading non-iterable value in array",
                self.ctx.module.file_id,
                span,
            )
            .with_label("this value is not iterable"),
        );
    }

    /// Check a spread in an object context.
    fn check_object_spread(
        &mut self,
        spread_id: dir::LocalNodeId<dir::Property>,
        value_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // get the spread value type
        let Some(type_id) = self.ctx.expression_type_id(value_id) else {
            return;
        };

        // check if the type is spreadable in object context
        if self.is_object_spreadable(type_id) {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, spread_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(spread_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_MISUSED_SPREAD.id,
                NO_MISUSED_SPREAD.code,
                NO_MISUSED_SPREAD.category,
                severity,
                "spreading null or undefined in object",
                self.ctx.module.file_id,
                span,
            )
            .with_label("this value cannot be spread"),
        );
    }
}

impl NodeVisitor for MisusedSpreadVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check array expressions for spread
        if let dir::Expression::ArrayExpression { elements } = expression {
            for element_id in elements {
                let element = tree.get(*element_id);
                if let dir::Argument::Spread { value, .. } = element {
                    self.check_array_spread(*element_id, *value);
                }
            }
        }

        // check object expressions for spread
        if let dir::Expression::ObjectExpression { properties, .. } = expression {
            for prop_id in properties {
                let prop = tree.get(*prop_id);
                if let dir::Property::Spread { value, .. } = prop {
                    self.check_object_spread(*prop_id, *value);
                }
            }
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag spreading non-iterable in array.
    #[test]
    fn test_flags_non_iterable_array_spread() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedSpread);
        let result = test.lint_dir(
            "test.ds",
            r#"
let obj = { x: 1 };
let arr = [...obj];
"#,
        );
        test.result(result).assert_lint("no-misused-spread");
    }

    /// Allow spreading array in array.
    #[test]
    fn test_allows_array_spread() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedSpread);
        let result = test.lint_dir(
            "test.ds",
            r#"
let arr1 = [1, 2, 3];
let arr2 = [...arr1, 4, 5];
"#,
        );
        test.result(result).assert_no_lint("no-misused-spread");
    }

    /// Allow spreading string in array.
    #[test]
    fn test_allows_string_spread() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedSpread);
        let result = test.lint_dir(
            "test.ds",
            r#"
let str = "hello";
let chars = [...str];
"#,
        );
        test.result(result).assert_no_lint("no-misused-spread");
    }

    /// Allow spreading object in object.
    #[test]
    fn test_allows_object_spread() {
        let test = TestProgram::for_rule_with_prelude(NoMisusedSpread);
        let result = test.lint_dir(
            "test.ds",
            r#"
let obj1 = { x: 1 };
let obj2 = { ...obj1, y: 2 };
"#,
        );
        test.result(result).assert_no_lint("no-misused-spread");
    }
}
