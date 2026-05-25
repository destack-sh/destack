use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_discarded_call_like_value, expression_has_symbol_decorator,
    expression_is_standalone_statement, expression_unwrap_parenthesized,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow ignoring return values from `@mustUse` APIs.
    ///
    /// Functions marked as `@mustUse` signal that dropping the return value
    /// is almost always a bug.
    #[lint(
        id = "unused-must-use",
        code = "LC042",
        category = Correctness,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub UnusedMustUse,
    "Disallow ignoring return values of @mustUse functions"
}

impl LintRule for UnusedMustUse {
    fn meta(&self) -> &'static LintMeta {
        UnusedMustUse::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let mut visitor = UnusedMustUseVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor for ignored must-use return checks.
struct UnusedMustUseVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> UnusedMustUseVisitor<'a, 'b> {
    /// Build a visitor for ignored must-use return checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.dir.tree();

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one statement for ignored must-use results.
    fn check_statement(
        &mut self,
        statement_id: dir::LocalNodeId<dir::Expression>,
        statement_expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some((expression_id, fix_expression_id)) =
            expression_discarded_call_like_value(self.ctx.dir.tree(), statement_expression_id)
        else {
            return;
        };
        let expression = self.ctx.dir.get(expression_id);

        let has_must_use = expression_has_must_use(self.ctx, expression_id)
            || call_like_callee_has_must_use(self.ctx, expression);
        if !has_must_use {
            return;
        }

        let severity = self.ctx.get_effective_severity(self.meta, statement_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(statement_id);
        let mut diagnostic = LintReport::new(
            UNUSED_MUST_USE.id,
            UNUSED_MUST_USE.code,
            UNUSED_MUST_USE.category,
            severity,
            "ignored return value from @mustUse API",
            span,
        )
        .label("use, return, or explicitly handle this result");

        // compute fixes only when requested by the runner
        if self.ctx.compute_fixes
            && let Some(fix) = unused_must_use_fix(self.ctx, fix_expression_id)
        {
            diagnostic = diagnostic.fix(fix);
        }

        self.ctx.report(diagnostic);
    }
}

/// Return true when the callee of one call-like expression is decorated with `@mustUse`.
fn call_like_callee_has_must_use(
    ctx: &LintModuleContext<'_>,
    expression: &dir::Expression,
) -> bool {
    let callee_id = match expression {
        dir::Expression::Call { left, .. } => *left,
        _ => return false,
    };

    expression_has_must_use(ctx, callee_id)
}

/// Return true when an expression candidate symbol is decorated with `@mustUse`.
fn expression_has_must_use(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(must_use_symbol) = ctx.get_language_item(dir::LanguageItem::MustUse) else {
        return false;
    };

    expression_has_symbol_decorator(
        ctx.artifacts.as_ref(),
        ctx.profile_id,
        ctx.module_id(),
        ctx.dir.tree(),
        ctx.strings,
        &ctx.symbols,
        ctx.types,
        ctx.resolutions,
        expression_id,
        must_use_symbol,
    )
}

/// Build a safe fix that explicitly discards the ignored result.
fn unused_must_use_fix(
    ctx: &LintModuleContext<'_>,
    statement_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // preserve the full replacement span, but normalize away wrapper parentheses
    let replacement_span = ctx.get_span(statement_expression_id);
    let normalized_id = expression_unwrap_parenthesized(ctx.dir.tree(), statement_expression_id);
    let normalized_span = ctx.get_span(normalized_id);
    let expression_text = ctx.get_span_text(normalized_span);
    if expression_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("void {expression_text}");
    let edits = ctx
        .edit_builder()
        .replace(replacement_span, replacement)
        .into_edits();
    Some(LintFix::safe("Explicitly discard the @mustUse result").with_edits(edits))
}

impl NodeVisitor for UnusedMustUseVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if expression_is_standalone_statement(tree, id)
            || matches!(expression, dir::Expression::Block(..))
        {
            self.check_statement(id, id);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag ignored @mustUse function calls.
    #[test]
    fn test_flags_ignored_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_flags_ignored_must_use_call.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

parse()
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_has_fix("unused-must-use");
    }

    /// Allow @mustUse return values when assigned.
    #[test]
    fn test_allows_used_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_allows_used_must_use_call.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

let value = parse()
"#,
        );
        test.result(result).assert_no_lint("unused-must-use");
    }

    /// Flag ignored @mustUse constructor calls.
    #[test]
    fn test_flags_ignored_must_use_constructor() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_flags_ignored_must_use_constructor.ds",
            r#"
@mustUse
class Token {
    value: int32

    constructor(value: int32) {
        this.value = value
    }
}

new Token(1)
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_has_fix("unused-must-use");
    }

    /// Allow non must-use calls.
    #[test]
    fn test_allows_non_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_allows_non_must_use_call.ds",
            r#"
function ping(): void {
}

ping()
"#,
        );
        test.result(result).assert_no_lint("unused-must-use");
    }

    /// Flag ignored awaited must-use calls.
    #[test]
    fn test_flags_ignored_awaited_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_flags_ignored_awaited_must_use_call.ds",
            r#"
@mustUse
function parse(): Promise<Result<int32, string>> {
    return Promise.resolve(Result.ok(1))
}

async function run(): Promise<void> {
    await parse()
}
"#,
        );
        test.result(result).assert_lint("unused-must-use");
    }

    /// Safely prefix ignored must-use calls with `void`.
    #[test]
    fn test_fix_prefixes_ignored_must_use_call_with_void() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_fix_prefixes_ignored_must_use_call_with_void.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

parse()
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_safe_fixed(
                r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1);
}

void parse();
"#,
            );
    }

    /// Safely prefix ignored must-use constructor calls with `void`.
    #[test]
    fn test_fix_prefixes_ignored_must_use_constructor_with_void() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_fix_prefixes_ignored_must_use_constructor_with_void.ds",
            r#"
@mustUse
class Token {
    value: int32

    constructor(value: int32) {
        this.value = value
    }
}

new Token(1)
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_safe_fixed(
                r#"
@mustUse
class Token {
    value: int32;

    constructor(value: int32) {
        this.value = value
    }
}

void new Token(1);
"#,
            );
    }

    /// Safely prefix ignored awaited must-use calls with `void`.
    #[test]
    fn test_fix_prefixes_ignored_awaited_must_use_call_with_void() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_fix_prefixes_ignored_awaited_must_use_call_with_void.ds",
            r#"
@mustUse
function parse(): Promise<Result<int32, string>> {
    return Promise.resolve(Result.ok(1))
}

async function run(): Promise<void> {
    await parse()
}
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_safe_fixed(
                r#"
@mustUse
function parse(): Promise<Result<int32, string>> {
    return Promise.resolve(Result.ok(1));
}

async function run(): Promise<void> {
    void await parse()
}
"#,
            );
    }

    /// Mutation: fix parenthesized ignored must-use calls.
    #[test]
    fn test_mutation_fix_parenthesized_ignored_must_use_call() {
        let test = TestProgram::for_rule_with_prelude(UnusedMustUse);
        let result = test.lint_dir(
            "unused_must_use/test_mutation_fix_parenthesized_ignored_must_use_call.ds",
            r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1)
}

{
    (parse())
}
"#,
        );
        test.result(result)
            .assert_lint("unused-must-use")
            .assert_safe_fixed(
                r#"
@mustUse
function parse(): Result<int32, string> {
    return Result.ok(1);
}

{
    void parse()
}
"#,
            );
    }
}
