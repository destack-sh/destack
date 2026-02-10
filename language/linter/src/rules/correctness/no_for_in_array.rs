use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::is_array_type;
use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow iterating over arrays with for-in.
    ///
    /// `for-in` iterates over enumerable property names (strings), not values.
    /// This is almost never what you want for arrays. Use `for-of` instead.
    #[lint(
        id = "no-for-in-array",
        code = "LC017",
        category = Correctness,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub NoForInArray,
    "Disallow iterating over arrays with for-in"
}

impl LintRule for NoForInArray {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoForInArray::meta()
    }

    /// Check module DIR nodes for for-in on arrays.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = ForInArrayVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that flags for-in on arrays.
struct ForInArrayVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The well known Array symbol for this module.
    array_symbol: dir::GlobalSymbolId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> ForInArrayVisitor<'a, 'b> {
    /// Build a visitor for for-in array checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        Self {
            ctx,
            meta,
            array_symbol,
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

    /// Check a for-each expression for array iteration with for-in.
    fn check_for_in_array(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        binding: &dir::ForEachBinding,
        iterator_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // resolve the iterator type
        let Some(type_id) = self.ctx.expression_type_id(iterator_id) else {
            return;
        };

        // check if the iterator is an array type
        let is_array = is_array_type(self.ctx.types, type_id, Some(self.array_symbol));
        if !is_array {
            return;
        }

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report the diagnostic
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            NO_FOR_IN_ARRAY.id,
            NO_FOR_IN_ARRAY.code,
            NO_FOR_IN_ARRAY.category,
            severity,
            "do not use for-in with arrays",
            self.ctx.module.file_id,
            span,
        )
        .with_label("use for-of to iterate over array values");

        // compute fixes only when requested by the runner
        if self.ctx.include_fixes
            && let Some(fix) = self.for_in_array_fix(binding, iterator_id)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build an unsafe fix by rewriting `for (... in ...)` to `for (... of ...)`.
    fn for_in_array_fix(
        &self,
        binding: &dir::ForEachBinding,
        iterator_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<LintFix> {
        let binding_span = for_each_binding_span(self.ctx, binding);
        let iterator_span = self.ctx.get_span(iterator_id);
        if binding_span.end >= iterator_span.start {
            return None;
        }

        let keyword_span = destack_source::Span::new(
            self.ctx.module.file_id,
            binding_span.end,
            iterator_span.start,
        );
        let keyword_text = self.ctx.get_span_text(keyword_span);
        let replacement = replace_for_in_keyword(keyword_text)?;

        let edits = self
            .ctx
            .edit_builder()
            .replace(keyword_span, replacement)
            .into_edits();
        Some(LintFix::r#unsafe("Replace for-in with for-of").with_edits(edits))
    }
}

impl NodeVisitor for ForInArrayVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // check for-in expressions on arrays
        if let dir::Expression::ForEach {
            kind: dir::ForEachKind::In,
            binding,
            iterator,
            ..
        } = expression
        {
            self.check_for_in_array(id, binding, *iterator);
        }

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// Return one binding span for a for-each binding.
fn for_each_binding_span(
    ctx: &LintModuleDirContext<'_>,
    binding: &dir::ForEachBinding,
) -> destack_source::Span {
    match binding {
        dir::ForEachBinding::Pattern { pattern, .. }
        | dir::ForEachBinding::Using { pattern, .. } => ctx.get_span(*pattern),
    }
}

/// Replace one standalone `in` keyword with `of` in the provided text.
fn replace_for_in_keyword(text: &str) -> Option<String> {
    let mut match_range = None;
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        return None;
    }

    for index in 0..=bytes.len() - 2 {
        if &bytes[index..index + 2] != b"in" {
            continue;
        }

        let before = text[..index].chars().next_back();
        let after = text[index + 2..].chars().next();
        if before.is_some_and(is_identifier_char) || after.is_some_and(is_identifier_char) {
            continue;
        }

        if match_range.is_some() {
            return None;
        }
        match_range = Some((index, index + 2));
    }

    let (start, end) = match_range?;
    let mut replacement = String::with_capacity(text.len());
    replacement.push_str(&text[..start]);
    replacement.push_str("of");
    replacement.push_str(&text[end..]);
    Some(replacement)
}

/// Return true when one char is an identifier character.
fn is_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_' || character == '$'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_for_in_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_array.ds",
            r#"
let items = [1, 2, 3];
for (const key in items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-for-in-array");
    }

    #[test]
    fn test_fix_rewrites_for_in_array_to_for_of() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_fix_rewrites_for_in_array_to_for_of.ds",
            r#"
let items = [1, 2, 3];
for (const key in items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
for (const key of items) {
    let value = key;
}
"#,
            );
    }

    #[test]
    fn test_flags_for_in_typed_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_flags_for_in_typed_array.ds",
            r#"
let items: number[] = [1, 2, 3];
for (const key in items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_lint("no-for-in-array");
    }

    #[test]
    fn test_mutation_fix_rewrites_typed_for_in_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_mutation_fix_rewrites_typed_for_in_array.ds",
            r#"
let items: number[] = [1, 2, 3];
for (const key in items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_unsafe_fixed(
                r#"
let items: number[] = [1, 2, 3];
for (const key of items) {
    let value = key;
}
"#,
            );
    }

    #[test]
    fn test_fix_handles_inline_comments_around_in_keyword() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_fix_handles_inline_comments_around_in_keyword.ds",
            r#"
let items = [1, 2, 3];
for (const key/* left */in/* right */items) {
    let value = key;
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-for-in-array")
            .assert_unsafe_fixed(
                r#"
let items = [1, 2, 3];
for (const key of /* right */ items) {
    let value = key;
}
"#,
            );
    }

    #[test]
    fn test_allows_for_of_array() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_allows_for_of_array.ds",
            r#"
let items = [1, 2, 3];
for (const value of items) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-for-in-array");
    }

    #[test]
    fn test_allows_for_in_object() {
        let test = TestProgram::for_rule_without_prelude(NoForInArray);
        let result = test.lint_dir(
            "no_for_in_array/test_allows_for_in_object.ds",
            r#"
let obj = { a: 1, b: 2 };
for (const key in obj) {
}
"#,
        );
        test.check_clean();
        test.result(result).assert_no_lint("no-for-in-array");
    }
}
