use destack_dir::{self as dir, LanguageItem, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_repository::LintSeverity;

use crate::LintRequirement::RequireLanguageItem;
use crate::rules::common::expression_target_symbol;
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require symbol descriptions.
    ///
    /// Symbol descriptions are useful for debugging and introspection.
    /// Passing a description makes it easier to identify the symbol.
    #[lint(
        id = "symbol-description",
        code = "LY065",
        category = Style,
        level = Dir,
        requires_all = [RequireLanguageItem(LanguageItem::Symbol)],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub SymbolDescription,
    "Require symbol descriptions"
}

impl LintRule for SymbolDescription {
    fn meta(&self) -> &'static LintMeta {
        SymbolDescription::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = SymbolDescriptionVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags Symbol() calls without descriptions.
struct SymbolDescriptionVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The Symbol constructor symbol for this module profile.
    symbol_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> SymbolDescriptionVisitor<'a, 'b> {
    /// Build a visitor for symbol description checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let symbol_symbol = ctx.language_item(LanguageItem::Symbol);
        Self {
            ctx,
            meta,
            symbol_symbol,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check a Symbol call for missing description.
    fn check_symbol_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) {
        let expression = self.ctx.dir.get(expression_id);

        // match call expressions only (Symbol() is always called, never new)
        let dir::Expression::Call {
            left, arguments, ..
        } = expression
        else {
            return;
        };

        // check if this is a Symbol call
        let Some(target_symbol) = expression_target_symbol(self.ctx, *left) else {
            return;
        };
        if target_symbol != self.symbol_symbol {
            return;
        }

        // if there are arguments, the description is provided
        if !arguments.is_empty() {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintReport::new(
            SYMBOL_DESCRIPTION.id,
            SYMBOL_DESCRIPTION.code,
            SYMBOL_DESCRIPTION.category,
            severity,
            "Symbol() called without description",
            span,
        )
        .label("add a description string to Symbol()");

        if let Some(fix) = symbol_description_fix(self.ctx, *left, span) {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

impl NodeVisitor for SymbolDescriptionVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.check_symbol_call(id);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Build a fix that inserts a fallback description for Symbol calls.
fn symbol_description_fix(
    ctx: &LintModuleContext<'_>,
    callee_id: dir::LocalNodeId<dir::Expression>,
    call_span: destack_source::Span,
) -> Option<LintFix> {
    let callee_text = ctx.get_span_text(ctx.get_span(callee_id));
    let replacement = format!("{callee_text}(\"symbol\")");
    let edits = ctx
        .edit_builder()
        .replace(call_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add Symbol description").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag Symbol() without description.
    #[test]
    fn test_flags_symbol_without_description() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_flags_symbol_without_description.ds",
            r#"
let sym = Symbol();
"#,
        );
        test.result(result)
            .assert_lint("symbol-description")
            .assert_has_fix("symbol-description");
    }

    /// Allow Symbol() with description.
    #[test]
    fn test_allows_symbol_with_description() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_allows_symbol_with_description.ds",
            r#"
let sym = Symbol("mySymbol");
"#,
        );
        test.result(result).assert_no_lint("symbol-description");
    }

    /// Allow Symbol.for() calls.
    #[test]
    fn test_allows_symbol_for() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_allows_symbol_for.ds",
            r#"
let sym = Symbol.for("mySymbol");
"#,
        );
        test.result(result).assert_no_lint("symbol-description");
    }

    /// Allow shadowed Symbol.
    #[test]
    fn test_allows_shadowed_symbol() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_allows_shadowed_symbol.ds",
            r#"
let Symbol = (): number => 42;
let value = Symbol();
"#,
        );
        test.result(result).assert_no_lint("symbol-description");
    }

    /// Add description to Symbol() calls without arguments.
    #[test]
    fn test_fix_symbol_without_description() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_fix_symbol_without_description.ds",
            r#"
let sym = Symbol()
"#,
        );
        test.result(result)
            .assert_lint("symbol-description")
            .assert_safe_fixed(
                r#"
let sym = Symbol("symbol");
"#,
            );
    }

    /// Fix multiple Symbol() calls in the same module.
    #[test]
    fn test_mutation_fix_multiple_symbol_calls() {
        let test = TestProgram::for_rule_with_prelude(SymbolDescription);
        let result = test.lint_dir(
            "symbol_description/test_mutation_fix_multiple_symbol_calls.ds",
            r#"
let first = Symbol()
let second = Symbol()
"#,
        );
        test.result(result)
            .assert_lint_count("symbol-description", 2)
            .assert_safe_fixed(
                r#"
let first = Symbol("symbol");
let second = Symbol("symbol");
"#,
            );
    }
}
