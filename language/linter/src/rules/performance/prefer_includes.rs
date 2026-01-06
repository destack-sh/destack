use destack_base::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, flip_binary_operator, is_array_type, is_string_type};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `includes()` over `indexOf()` comparisons.
    ///
    /// `includes()` communicates intent more clearly and avoids comparisons
    /// against sentinel values.
    #[lint(
        id = "prefer-includes",
        code = "LP019",
        category = Performance,
        level = Dir,
        requires_all = [
            RequireWellKnownSymbol(WellKnownSymbol::Array),
            RequireWellKnownSymbol(WellKnownSymbol::String),
        ],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferIncludes,
    "Prefer includes() over indexOf() comparisons"
}

impl LintRule for PreferIncludes {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferIncludes::meta()
    }

    /// Check module DIR nodes for indexOf comparisons that should use includes().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferIncludesVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// The kind of indexOf comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncludesCheck {
    /// The comparison checks that a match exists.
    AnyMatch,
    /// The comparison checks that no match exists.
    NoMatch,
}

/// Node visitor that flags prefer-includes patterns.
struct PreferIncludesVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The well known String symbol for this module.
    string_symbol: dir::GlobalSymbolId,
    /// The string id for the indexOf method name.
    index_of_name: StringId,
    /// The string id for the lastIndexOf method name.
    last_index_of_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferIncludesVisitor<'a, 'b> {
    /// Build a visitor for prefer-includes checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let string_symbol = ctx.well_known_symbol(WellKnownSymbol::String);
        let index_of_name = ctx.program.strings.intern("indexOf");
        let last_index_of_name = ctx.program.strings.intern("lastIndexOf");

        Self {
            ctx,
            meta,
            array_symbol,
            string_symbol,
            index_of_name,
            last_index_of_name,
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

    /// Check comparison expressions for prefer-includes patterns.
    fn check_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // match comparisons with the constant on the right
        if let Some(check) = self.match_comparison(left, operator, right, false) {
            self.report_match(expression_id, check);
            return;
        }

        // match comparisons with the constant on the left
        if let Some(check) = self.match_comparison(right, operator, left, true) {
            self.report_match(expression_id, check);
        }
    }

    /// Match a comparison with a candidate expression and constant.
    fn match_comparison(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        constant_id: dir::LocalNodeId<dir::Expression>,
        flipped: bool,
    ) -> Option<IncludesCheck> {
        // resolve constant comparisons
        let constant_value = self.ctx.const_value(constant_id)?;
        let constant = const_i64(&constant_value)?;

        // normalize operators when constants are on the left
        let operator = if flipped {
            flip_binary_operator(operator)?
        } else {
            operator
        };

        // check for indexOf and lastIndexOf comparisons
        if self.is_index_of_call(candidate_id) {
            return check_index_of_comparison(operator, constant);
        }

        None
    }

    /// Report a prefer-includes match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        check: IncludesCheck,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic text
        let label = match check {
            IncludesCheck::AnyMatch => "use includes() to check for a match",
            IncludesCheck::NoMatch => "use !includes() to check for no matches",
        };

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                PREFER_INCLUDES.id,
                PREFER_INCLUDES.code,
                PREFER_INCLUDES.category,
                severity,
                "prefer includes() over indexOf() comparison",
                self.ctx.module.file_id,
                span,
            )
            .with_label(label),
        );
    }

    /// Return true when the expression is an indexOf or lastIndexOf call.
    fn is_index_of_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // match call expression
        let expression = self.ctx.tree.get(expression_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            return false;
        };
        if dynamic_arguments.is_empty() {
            return false;
        }

        // match member access for indexOf or lastIndexOf
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        let is_index = *name == self.index_of_name || *name == self.last_index_of_name;
        if !is_index {
            return false;
        }

        self.is_supported_receiver(*left)
    }

    /// Return true when the receiver expression is an array or string type.
    fn is_supported_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        let is_array = is_array_type(self.ctx.types, type_id, Some(self.array_symbol));
        if is_array {
            return true;
        }

        is_string_type(self.ctx.types, type_id, Some(self.string_symbol))
    }
}

impl NodeVisitor for PreferIncludesVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check comparison expressions
        if let dir::Expression::Binary {
            left,
            operator,
            right,
        } = expression
        {
            self.check_binary(id, *operator, *left, *right);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Check indexOf comparisons against constants.
fn check_index_of_comparison(
    operator: dir::BinaryOperator,
    constant: i64,
) -> Option<IncludesCheck> {
    match operator {
        dir::BinaryOperator::GreaterThan if constant == -1 => Some(IncludesCheck::AnyMatch),
        dir::BinaryOperator::GreaterThanOrEqual if constant == 0 => Some(IncludesCheck::AnyMatch),
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict if constant == -1 => {
            Some(IncludesCheck::AnyMatch)
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict if constant == -1 => {
            Some(IncludesCheck::NoMatch)
        }
        dir::BinaryOperator::LessThan if constant == 0 => Some(IncludesCheck::NoMatch),
        dir::BinaryOperator::LessThanOrEqual if constant == -1 => Some(IncludesCheck::NoMatch),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report indexOf comparisons against -1.
    #[test]
    fn test_flags_index_of_not_found_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let has = items.indexOf(2) !== -1;
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Report string indexOf comparisons against -1.
    #[test]
    fn test_flags_string_index_of_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "test.ds",
            r#"
let text = "hello";
let has = text.indexOf("lo") != -1;
"#,
        );
        test.result(result).assert_lint("prefer-includes");
    }

    /// Allow indexOf comparisons that are not includes checks.
    #[test]
    fn test_allows_index_of_zero_check() {
        let test = TestProgram::for_rule_without_prelude(PreferIncludes);
        let result = test.lint_dir(
            "test.ds",
            r#"
let items = [1, 2, 3];
let first = items.indexOf(2) === 0;
"#,
        );
        test.result(result).assert_no_lint("prefer-includes");
    }
}
