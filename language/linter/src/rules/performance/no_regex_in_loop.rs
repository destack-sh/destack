use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireLibSymbol;
use crate::rules::common::{
    expression_enters_nested_declaration_scope, expression_is_symbol_or_global_qualified_member,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `RegExp(...)` construction inside loops.
    ///
    /// Creating a RegExp object inside a loop causes unnecessary allocations
    /// on each iteration. Move the regex outside the loop or use a regex literal.
    #[lint(
        id = "no-regex-in-loop",
        code = "LP009",
        category = Performance,
        level = Dir,
        requires_all = [RequireLibSymbol("RegExp", &[])],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoRegexInLoop,
    "Disallow RegExp() construction inside loops"
}

impl LintRule for NoRegexInLoop {
    fn meta(&self) -> &'static LintMeta {
        NoRegexInLoop::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = NoRegexInLoopVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Visitor that flags RegExp construction inside loops.
struct NoRegexInLoopVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The RegExp symbol for this module.
    regexp_symbol: dir::GlobalSymbolId,
    /// The RegExp member name.
    regexp_name: StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// Whether the current traversal is inside a loop.
    is_in_loop: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoRegexInLoopVisitor<'a, 'b> {
    /// Build a visitor for no-regex-in-loop checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let regexp_name = ctx.string_id("RegExp");
        let regexp_symbol = ctx.declared_library_symbol(regexp_name);
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            regexp_symbol,
            regexp_name,
            global_qualifiers,
            is_in_loop: false,
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

    /// Check one constructor expression for RegExp construction inside a loop.
    fn check_constructor(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        callee_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // only check inside loops
        if !self.is_in_loop {
            return;
        }

        // check if this is RegExp construction
        if !expression_is_symbol_or_global_qualified_member(
            self.ctx.tree,
            callee_id,
            self.regexp_symbol,
            &self.global_qualifiers,
            self.regexp_name,
        ) {
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
            LintReport::new(
                NO_REGEX_IN_LOOP.id,
                NO_REGEX_IN_LOOP.code,
                NO_REGEX_IN_LOOP.category,
                severity,
                "RegExp construction inside loop causes unnecessary allocations",
                span,
            )
            .label("move regex outside loop or use a regex literal"),
        );
    }

    /// Visit a loop expression with loop context.
    fn visit_loop(
        &mut self,
        tree: &dir::Tree,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit loop condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit loop body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for each loop expression with loop context.
    fn visit_for_each(
        &mut self,
        tree: &dir::Tree,
        binding: &dir::ForEachBinding,
        iterator: dir::LocalNodeId<dir::Expression>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit binding inside loop context
        self.is_in_loop = true;
        match binding {
            dir::ForEachBinding::Pattern { pattern, .. } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
            dir::ForEachBinding::Using {
                asynchrony: _,
                pattern,
            } => {
                let pattern_node = tree.get(*pattern);
                self.visit_pattern(tree, *pattern, pattern_node);
            }
        }

        // visit iterator outside loop context
        self.is_in_loop = was_in_loop;
        let iterator_expression = tree.get(iterator);
        self.visit_expression(tree, iterator, iterator_expression);

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }

    /// Visit a for loop expression with loop context.
    fn visit_for(
        &mut self,
        tree: &dir::Tree,
        initialization: Option<dir::LocalNodeId<dir::Expression>>,
        condition: Option<dir::LocalNodeId<dir::Expression>>,
        increment: Option<dir::LocalNodeId<dir::Expression>>,
        body: dir::LocalNodeId<dir::Block>,
    ) {
        // capture prior loop state
        let was_in_loop = self.is_in_loop;

        // visit initialization outside loop context
        if let Some(initialization_id) = initialization {
            self.is_in_loop = was_in_loop;
            let initialization_expression = tree.get(initialization_id);
            self.visit_expression(tree, initialization_id, initialization_expression);
        }

        // visit condition inside loop context
        if let Some(condition_id) = condition {
            self.is_in_loop = true;
            let condition_expression = tree.get(condition_id);
            self.visit_expression(tree, condition_id, condition_expression);
        }

        // visit increment inside loop context
        if let Some(increment_id) = increment {
            self.is_in_loop = true;
            let increment_expression = tree.get(increment_id);
            self.visit_expression(tree, increment_id, increment_expression);
        }

        // visit body inside loop context
        self.is_in_loop = true;
        let body_block = tree.get(body);
        self.visit_block(tree, body, body_block);

        // restore prior loop state
        self.is_in_loop = was_in_loop;
    }
}

impl NodeVisitor for NoRegexInLoopVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // avoid leaking loop context into nested declarations
        if self.is_in_loop && expression_enters_nested_declaration_scope(tree, expression) {
            return;
        }

        // check constructor calls
        if let dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } = expression {
            self.check_constructor(id, *left);
        }

        // handle loop expressions with custom traversal
        match expression {
            dir::Expression::Loop {
                condition, body, ..
            } => {
                self.visit_loop(tree, *condition, *body);
                return;
            }
            dir::Expression::ForEach {
                binding,
                iterator,
                body,
                ..
            } => {
                self.visit_for_each(tree, binding, *iterator, *body);
                return;
            }
            dir::Expression::For {
                initialization,
                condition,
                increment,
                body,
                ..
            } => {
                self.visit_for(tree, *initialization, *condition, *increment, *body);
                return;
            }
            _ => {}
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag RegExp(...) inside for-of loop.
    #[test]
    fn test_flags_regexp_call_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_regexp_call_in_for_of.ds",
            r#"
let patterns = ["a", "b", "c"];
for (const p of patterns) {
    let re = RegExp(p);
    console.log(re.test("abc"));
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Flag new RegExp inside for-of loop.
    #[test]
    fn test_flags_regexp_in_for_of() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_regexp_in_for_of.ds",
            r#"
let patterns = ["a", "b", "c"];
for (const p of patterns) {
    let re = new RegExp(p);
    console.log(re.test("abc"));
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Flag new RegExp inside for loop.
    #[test]
    fn test_flags_regexp_in_for() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_regexp_in_for.ds",
            r#"
let patterns = ["a", "b", "c"];
for (let i = 0; i < patterns.length; i += 1) {
    let re = new RegExp(patterns[i]);
    console.log(re.test("abc"));
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Flag new RegExp inside while loop.
    #[test]
    fn test_flags_regexp_in_while() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_regexp_in_while.ds",
            r#"
let i = 0;
while (i < 10) {
    let re = new RegExp("test" + i);
    i += 1;
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Flag global qualified RegExp calls inside loops.
    #[test]
    fn test_flags_global_regexp_call_in_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_global_regexp_call_in_loop.ds",
            r#"
for (const pattern of ["a", "b"]) {
    let re = globalThis.RegExp(pattern);
    console.log(re.test("abc"));
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Flag computed global qualified RegExp calls inside loops.
    #[test]
    fn test_flags_computed_global_regexp_call_in_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_flags_computed_global_regexp_call_in_loop.ds",
            r#"
for (const pattern of ["a", "b"]) {
    let re = globalThis["RegExp"](pattern);
    console.log(re.test("abc"));
}
"#,
        );
        test.result(result).assert_lint("no-regex-in-loop");
    }

    /// Allow new RegExp outside loops.
    #[test]
    fn test_allows_regexp_outside_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_allows_regexp_outside_loop.ds",
            r#"
let re = new RegExp("test");
for (const item of [1, 2, 3]) {
    console.log(re.test(item));
}
"#,
        );
        test.result(result).assert_no_lint("no-regex-in-loop");
    }

    /// Allow regex literals inside loops.
    #[test]
    fn test_allows_regex_literal_in_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_allows_regex_literal_in_loop.ds",
            r#"
for (const item of ["a", "b", "c"]) {
    if (/test/.test(item)) {
        console.log(item);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-regex-in-loop");
    }

    /// Allow RegExp outside loops.
    #[test]
    fn test_allows_regexp_call_outside_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_allows_regexp_call_outside_loop.ds",
            r#"
let re = RegExp("test");
for (const item of [1, 2, 3]) {
    console.log(re.test(item));
}
"#,
        );
        test.result(result).assert_no_lint("no-regex-in-loop");
    }

    /// Allow RegExp construction inside functions declared within loops.
    #[test]
    fn test_allows_nested_function_inside_loop() {
        let test = TestProgram::for_rule_with_prelude(NoRegexInLoop);
        let result = test.lint_dir(
            "no_regex_in_loop/test_allows_nested_function_inside_loop.ds",
            r#"
for (const pattern of ["a", "b"]) {
    function build() {
        return new RegExp(pattern);
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-regex-in-loop");
    }
}
