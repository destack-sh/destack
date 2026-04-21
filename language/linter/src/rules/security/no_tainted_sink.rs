use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    TaintAnalysis, TaintCache, TaintLabels, assign_pattern_target_expression,
    expression_sink_taint_labels,
};
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow flows of tainted values into security-sensitive sinks.
    ///
    /// This rule is decorator-driven:
    /// - mark sources with `@taint` or `@taint("label")`
    /// - mark sinks with `@sink` or `@sink("label")`
    /// - optionally mark sanitizers with `@sanitizer` or `@sanitizer("label")`
    /// Use this rule for project specific sink coverage.
    /// Pair it with dedicated rules like `no-open-redirect` for well known platform sink families.
    #[lint(
        id = "no-tainted-sink",
        code = "LS010",
        category = Security,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoTaintedSink,
    "Disallow passing tainted values into sink APIs"
}

impl LintRule for NoTaintedSink {
    fn meta(&self) -> &'static LintMeta {
        NoTaintedSink::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoTaintedSinkVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor for tainted sink flows.
struct NoTaintedSinkVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// Cached taint analysis state.
    taint_cache: TaintCache,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoTaintedSinkVisitor<'a, 'b> {
    /// Build a visitor for tainted sink flows.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        Self {
            ctx,
            meta,
            taint_cache: TaintCache::default(),
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the module roots.
    fn run(&mut self) {
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one call expression for tainted sink arguments.
    fn check_call_like(
        &mut self,
        callee_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) {
        let callee = self.ctx.tree.get(callee_id);
        let sink_labels = expression_sink_taint_labels(
            self.ctx.repository.as_ref(),
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            callee_id,
            callee,
        );
        if sink_labels.is_empty() {
            return;
        }

        for argument_id in arguments.iter().copied() {
            let argument = self.ctx.tree.get(argument_id);
            let source_id = argument.value();
            let source_labels = self.expression_taint_labels(source_id);
            if !source_labels.matches_sink(&sink_labels, self.ctx.repository.as_ref()) {
                continue;
            }

            self.report(source_id, "tainted value passed to sink");
        }
    }

    /// Check one assignment for tainted values flowing into sink targets.
    fn check_assign(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        let left_expression = self.ctx.tree.get(left);
        let sink_labels = expression_sink_taint_labels(
            self.ctx.repository.as_ref(),
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.symbols,
            self.ctx.types,
            left,
            left_expression,
        );
        if sink_labels.is_empty() {
            return;
        }

        let source_labels = self.expression_taint_labels(right);
        if !source_labels.matches_sink(&sink_labels, self.ctx.repository.as_ref()) {
            return;
        }

        self.report(right, "tainted value assigned to sink");
    }

    /// Resolve taint labels for one expression.
    fn expression_taint_labels(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> TaintLabels {
        let mut taint = TaintAnalysis::new(
            self.ctx.repository.as_ref(),
            self.ctx.revision,
            self.ctx.profile_id,
            self.ctx.module_id(),
            self.ctx.tree,
            self.ctx.symbols,
            self.ctx.types,
            &mut self.taint_cache,
            false,
        );
        taint.expression_taint_labels(expression_id)
    }

    /// Report one tainted sink diagnostic.
    fn report(&mut self, expression_id: dir::LocalNodeId<dir::Expression>, message: &str) {
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintDiagnostic::new(
                NO_TAINTED_SINK.id,
                NO_TAINTED_SINK.code,
                NO_TAINTED_SINK.category,
                severity,
                message,
                self.ctx.module.file_id,
                span,
            )
            .with_label("tainted data reaches a security sink"),
        );
    }
}

impl NodeVisitor for NoTaintedSinkVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let dir::Expression::Assign { left, right } = expression {
            let Some(left_expression_id) = assign_pattern_target_expression(self.ctx.tree, *left)
            else {
                return;
            };
            self.check_assign(left_expression_id, *right);
        }

        if let dir::Expression::Call {
            left, arguments, ..
        } = expression
        {
            self.check_call_like(*left, arguments);
        }

        if let dir::Expression::New {
            left, arguments, ..
        } = expression
        {
            self.check_call_like(*left, arguments);
        }

        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag tainted values passed into unlabeled sinks.
    #[test]
    fn test_flags_tainted_argument_for_unlabeled_sink() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_tainted_argument_for_unlabeled_sink.ds",
            r#"
@taint("url")
function readUrl(): string {
    return "/next"
}

@sink
function sendToSink(value: string): void {
}

sendToSink(readUrl())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Flag tainted values passed into matching labeled sinks.
    #[test]
    fn test_flags_tainted_argument_for_matching_labeled_sink() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_tainted_argument_for_matching_labeled_sink.ds",
            r#"
@taint("url")
function readUrl(): string {
    return "/next"
}

@sink("url")
function redirect(target: string): void {
}

redirect(readUrl())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Allow tainted values when labels do not match sink requirements.
    #[test]
    fn test_allows_label_mismatch() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_label_mismatch.ds",
            r#"
@taint("sql")
function readQuery(): string {
    return "select 1"
}

@sink("url")
function redirect(target: string): void {
}

redirect(readQuery())
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Flag assignments into sink fields.
    #[test]
    fn test_flags_tainted_assignment_into_sink_field() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_tainted_assignment_into_sink_field.ds",
            r#"
@taint("url")
function readUrl(): string {
    return "/next"
}

class RedirectState {
    @sink("url")
    next: string

    constructor() {
        this.next = "/safe"
    }
}

let state = new RedirectState()
state.next = readUrl()
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Allow plain non-tainted values for sinks.
    #[test]
    fn test_allows_non_tainted_value_for_sink() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_non_tainted_value_for_sink.ds",
            r#"
@sink
function sendToSink(value: string): void {
}

sendToSink("/safe")
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Allow values sanitized by a matching label sanitizer.
    #[test]
    fn test_allows_matching_label_sanitizer() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_matching_label_sanitizer.ds",
            r#"
@taint("url")
function readUrl(): string {
    return "/next"
}

@sanitizer("url")
function sanitizeUrl(input: string): string {
    return "/safe"
}

@sink("url")
function redirect(target: string): void {
}

redirect(sanitizeUrl(readUrl()))
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Flag taint propagated through local bindings.
    #[test]
    fn test_flags_taint_through_local_binding() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_taint_through_local_binding.ds",
            r#"
@taint("url")
function readUrl(): string {
    return "/next"
}

@sink
function sendToSink(value: string): void {
}

let captured = readUrl()
sendToSink(captured)
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Flag taint when sink uses a wildcard label prefix.
    #[test]
    fn test_flags_matching_wildcard_sink_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_matching_wildcard_sink_label.ds",
            r#"
@taint("sql.query")
function readQuery(): string {
    return "select 1"
}

@sink("sql.*")
function executeSQL(query: string): void {
}

executeSQL(readQuery())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Allow taint when wildcard sink prefix does not match by segment.
    #[test]
    fn test_allows_non_matching_wildcard_sink_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_non_matching_wildcard_sink_label.ds",
            r#"
@taint("sqlx.query")
function readQuery(): string {
    return "select 1"
}

@sink("sql.*")
function executeSQL(query: string): void {
}

executeSQL(readQuery())
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Flag taint for multi-label source decorators.
    #[test]
    fn test_flags_multi_argument_taint_decorator() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_multi_argument_taint_decorator.ds",
            r#"
@taint("url", "redirect")
function readTarget(): string {
    return "/next"
}

@sink("redirect")
function redirect(target: string): void {
}

redirect(readTarget())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Allow flows sanitized by wildcard sanitizer labels.
    #[test]
    fn test_allows_wildcard_sanitizer_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_wildcard_sanitizer_label.ds",
            r#"
@taint("url.redirect")
function readTarget(): string {
    return "/next"
}

@sanitizer("url.*")
function normalizeURL(value: string): string {
    return "/safe"
}

@sink("url.redirect")
function redirect(target: string): void {
}

redirect(normalizeURL(readTarget()))
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Allow flows sanitized by global wildcard sanitizer label.
    #[test]
    fn test_allows_global_wildcard_sanitizer_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_global_wildcard_sanitizer_label.ds",
            r#"
@taint("sql.query")
function readQuery(): string {
    return "select 1"
}

@sanitizer("*")
function sanitizeAny(value: string): string {
    return "safe"
}

@sink("sql.query")
function executeSQL(query: string): void {
}

executeSQL(sanitizeAny(readQuery()))
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Flag taint for suffix glob sink patterns.
    #[test]
    fn test_flags_suffix_glob_sink_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_suffix_glob_sink_label.ds",
            r#"
@taint("cmd.exec")
function readCommand(): string {
    return "whoami"
}

@sink("*exec")
function runCommand(command: string): void {
}

runCommand(readCommand())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Flag taint for middle glob sink patterns.
    #[test]
    fn test_flags_middle_glob_sink_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_middle_glob_sink_label.ds",
            r#"
@taint("sql.query.raw")
function readQuery(): string {
    return "select 1"
}

@sink("sql.*.raw")
function executeSQL(query: string): void {
}

executeSQL(readQuery())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Flag taint for multi-argument sink labels.
    #[test]
    fn test_flags_multi_argument_sink_labels() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_flags_multi_argument_sink_labels.ds",
            r#"
@taint("sql.query")
function readQuery(): string {
    return "select 1"
}

@sink("url.*", "sql.*")
function executeAny(value: string): void {
}

executeAny(readQuery())
"#,
        );
        test.result(result).assert_lint("no-tainted-sink");
    }

    /// Allow taint when a glob sink pattern does not match.
    #[test]
    fn test_allows_non_matching_middle_glob_sink_label() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_non_matching_middle_glob_sink_label.ds",
            r#"
@taint("sql.query.raw")
function readQuery(): string {
    return "select 1"
}

@sink("sql.*.safe")
function executeSQL(query: string): void {
}

executeSQL(readQuery())
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }

    /// Allow taint removed by multi-argument sanitizer labels.
    #[test]
    fn test_allows_multi_argument_sanitizer_labels() {
        let test = TestProgram::for_rule_with_prelude(NoTaintedSink);
        let result = test.lint_dir(
            "no_tainted_sink/test_allows_multi_argument_sanitizer_labels.ds",
            r#"
@taint("cmd.exec")
function readCommand(): string {
    return "whoami"
}

@sanitizer("url.*", "cmd.*")
function sanitizeValue(value: string): string {
    return "safe"
}

@sink("*exec")
function runCommand(command: string): void {
}

runCommand(sanitizeValue(readCommand()))
"#,
        );
        test.result(result).assert_no_lint("no-tainted-sink");
    }
}
