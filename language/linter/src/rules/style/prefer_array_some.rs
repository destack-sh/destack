use destack_core::StringId;
use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, WellKnownSymbol, walk_expression};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::LintRequirement::RequireWellKnownSymbol;
use crate::rules::common::{const_i64, flip_binary_operator, is_array_type, member_receiver_text};
use crate::{
    ConstValue, LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint,
};

declare_lint! {
    /// Prefer `some()` over `filter().length` or `findIndex()` comparisons.
    ///
    /// `some()` short-circuits and directly expresses intent when checking if
    /// any element matches a predicate.
    #[lint(
        id = "prefer-array-some",
        code = "LY032",
        category = Style,
        level = Dir,
        requires_all = [RequireWellKnownSymbol(WellKnownSymbol::Array)],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferArraySome,
    "Prefer array.some() over filter().length or findIndex() comparisons"
}

impl LintRule for PreferArraySome {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferArraySome::meta()
    }

    /// Check module DIR nodes for array comparisons that should use some().
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();
        let mut visitor = PreferArraySomeVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Check kinds for prefer-array-some comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArraySomeKind {
    /// `array.filter(...).length` comparison.
    FilterLength,
    /// `array.findIndex(...)` comparison.
    FindIndex,
    /// `array.find(...)` comparison against nullish values.
    Find,
}

/// Expected match direction for prefer-array-some checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArraySomeCheck {
    /// The comparison checks that a match exists.
    AnyMatch,
    /// The comparison checks that no match exists.
    NoMatch,
}

/// Match info for array some comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ArraySomeMatch {
    /// The comparison kind.
    kind: ArraySomeKind,
    /// The comparison intent.
    check: ArraySomeCheck,
    /// The candidate expression under comparison.
    candidate_expression_id: dir::LocalNodeId<dir::Expression>,
}

/// Node visitor that flags prefer-array-some patterns.
struct PreferArraySomeVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The array symbol for this module profile.
    array_symbol: dir::GlobalSymbolId,
    /// The string id for the filter method name.
    filter_name: StringId,
    /// The string id for the findIndex method name.
    find_index_name: StringId,
    /// The string id for the findLastIndex method name.
    find_last_index_name: StringId,
    /// The string id for the find method name.
    find_name: StringId,
    /// The string id for the findLast method name.
    find_last_name: StringId,
    /// The string id for the some method name.
    some_name: StringId,
    /// The string id for the length property name.
    length_name: StringId,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> PreferArraySomeVisitor<'a, 'b> {
    /// Build a visitor for prefer-array-some checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let array_symbol = ctx.well_known_symbol(WellKnownSymbol::Array);
        let filter_name = ctx.repository.strings.intern("filter");
        let find_index_name = ctx.repository.strings.intern("findIndex");
        let find_last_index_name = ctx.repository.strings.intern("findLastIndex");
        let find_name = ctx.repository.strings.intern("find");
        let find_last_name = ctx.repository.strings.intern("findLast");
        let some_name = ctx.repository.strings.intern("some");
        let length_name = ctx.repository.strings.intern("length");

        Self {
            ctx,
            meta,
            array_symbol,
            filter_name,
            find_index_name,
            find_last_index_name,
            find_name,
            find_last_name,
            some_name,
            length_name,
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

    /// Check comparison expressions for prefer-array-some patterns.
    fn check_binary(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        // match comparisons with the constant on the right
        if let Some(match_info) = self.match_comparison(left, operator, right, false) {
            self.report_match(expression_id, match_info);
            return;
        }

        // match comparisons with the constant on the left
        if let Some(match_info) = self.match_comparison(right, operator, left, true) {
            self.report_match(expression_id, match_info);
        }
    }

    /// Match a comparison with a candidate expression and constant.
    fn match_comparison(
        &mut self,
        candidate_id: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        constant_id: dir::LocalNodeId<dir::Expression>,
        flipped: bool,
    ) -> Option<ArraySomeMatch> {
        // resolve constant comparisons
        let constant_value = self.ctx.const_value(constant_id)?;

        // normalize operators when constants are on the left
        let operator = if flipped {
            flip_binary_operator(operator)?
        } else {
            operator
        };

        // check for filter length comparisons
        if self.is_filter_length(candidate_id) {
            let constant = const_i64(&constant_value)?;
            let check = check_filter_length_comparison(operator, constant)?;
            return Some(ArraySomeMatch {
                kind: ArraySomeKind::FilterLength,
                check,
                candidate_expression_id: candidate_id,
            });
        }

        // check for findIndex and findLastIndex comparisons
        if self.is_find_index_like_call(candidate_id) {
            let constant = const_i64(&constant_value)?;
            let check = check_find_index_comparison(operator, constant)?;
            return Some(ArraySomeMatch {
                kind: ArraySomeKind::FindIndex,
                check,
                candidate_expression_id: candidate_id,
            });
        }

        // check for find and findLast comparisons against nullish values
        if self.is_find_like_call(candidate_id) {
            let check = check_find_nullish_comparison(operator, &constant_value)?;
            return Some(ArraySomeMatch {
                kind: ArraySomeKind::Find,
                check,
                candidate_expression_id: candidate_id,
            });
        }

        None
    }

    /// Report a prefer-array-some match.
    fn report_match(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        match_info: ArraySomeMatch,
    ) {
        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // build diagnostic text
        let target = match match_info.kind {
            ArraySomeKind::FilterLength => "filter().length",
            ArraySomeKind::FindIndex => "findIndex()",
            ArraySomeKind::Find => "find()",
        };
        let label = match match_info.check {
            ArraySomeCheck::AnyMatch => "use array.some(...) to check for any match",
            ArraySomeCheck::NoMatch => "use !array.some(...) to check for no matches",
        };

        // build diagnostic and attach fix for findIndex rewrites
        let span = self.ctx.get_span(expression_id);
        let mut diagnostic = LintDiagnostic::new(
            PREFER_ARRAY_SOME.id,
            PREFER_ARRAY_SOME.code,
            PREFER_ARRAY_SOME.category,
            severity,
            format!("prefer some() over {target} comparison"),
            self.ctx.module.file_id,
            span,
        )
        .with_label(label);
        if matches!(
            match_info.kind,
            ArraySomeKind::FindIndex | ArraySomeKind::Find
        ) && let Some(fix) = self.find_call_comparison_fix(expression_id, match_info)
        {
            diagnostic = diagnostic.with_fix(fix);
        }

        self.ctx.report(diagnostic);
    }

    /// Build a safe fix from one find style comparison to some.
    fn find_call_comparison_fix(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        match_info: ArraySomeMatch,
    ) -> Option<LintFix> {
        let candidate_expression = self.ctx.tree.get(match_info.candidate_expression_id);
        let dir::Expression::Call {
            left,
            generic_arguments,
            dynamic_arguments,
        } = candidate_expression
        else {
            return None;
        };
        if !generic_arguments.is_empty() {
            return None;
        }
        if dynamic_arguments.is_empty() || dynamic_arguments.len() > 2 {
            return None;
        }

        // require positional callback and optional positional this-arg
        for argument_id in dynamic_arguments {
            let argument = self.ctx.tree.get(*argument_id);
            if !matches!(argument, dir::Argument::Positional { .. }) {
                return None;
            }
        }

        // resolve `.findIndex` member expression
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member {
            left: receiver_expression_id,
            name,
            generic_arguments,
            ..
        } = member_expression
        else {
            return None;
        };
        match match_info.kind {
            ArraySomeKind::FindIndex => {
                if *name != Some(self.find_index_name) && *name != Some(self.find_last_index_name) {
                    return None;
                }
            }
            ArraySomeKind::Find => {
                if *name != Some(self.find_name) && *name != Some(self.find_last_name) {
                    return None;
                }
            }
            ArraySomeKind::FilterLength => return None,
        }
        if !generic_arguments.is_empty() {
            return None;
        }

        // derive receiver text from member expression text
        let member_span = self.ctx.get_span(*left);
        let member_text = self.ctx.get_span_text(member_span);
        let receiver_text = member_receiver_text(
            self.ctx,
            *receiver_expression_id,
            member_text,
            (*name)?,
            false,
        )?;

        // preserve callback and optional this-arg source range
        let first_argument_id = *dynamic_arguments.first()?;
        let last_argument_id = *dynamic_arguments.last()?;
        let first_span = self.ctx.get_span(first_argument_id);
        let last_span = self.ctx.get_span(last_argument_id);
        let arguments_span = Span::new(first_span.file, first_span.start, last_span.end);
        let arguments_text = self.ctx.get_span_text(arguments_span);

        let some_name = self.ctx.repository.strings.get(self.some_name);
        let some_call = format!("{receiver_text}.{}({arguments_text})", some_name.as_ref());
        let replacement = match match_info.check {
            ArraySomeCheck::AnyMatch => some_call,
            ArraySomeCheck::NoMatch => format!("!{some_call}"),
        };

        // replace the full comparison expression
        let expression_span = self.ctx.get_span(expression_id);
        let edits = self
            .ctx
            .edit_builder()
            .replace(expression_span, replacement)
            .into_edits();

        Some(LintFix::safe("Replace find style comparison with some()").with_edits(edits))
    }

    /// Return true when the expression is a filter().length chain on an array.
    fn is_filter_length(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);

        // match `.length` member access
        let dir::Expression::Member { left, name, .. } = expression else {
            return false;
        };
        if *name != Some(self.length_name) {
            return false;
        }

        // match call expression on the left
        let call_id = *left;
        let call_expression = self.ctx.tree.get(call_id);
        let dir::Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = call_expression
        else {
            return false;
        };
        if dynamic_arguments.is_empty() {
            return false;
        }

        // match `.filter(...)` call
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != Some(self.filter_name) {
            return false;
        }

        self.is_array_receiver(*left)
    }

    /// Return true when the expression is an array findIndex or findLastIndex call.
    fn is_find_index_like_call(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let expression = self.ctx.tree.get(expression_id);

        // match call expression
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

        // match `.findIndex(...)` call
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != Some(self.find_index_name) && *name != Some(self.find_last_index_name) {
            return false;
        }

        self.is_array_receiver(*left)
    }

    /// Return true when the expression is an array find or findLast call.
    fn is_find_like_call(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        let expression = self.ctx.tree.get(expression_id);

        // match call expression
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

        // match `.find(...)` call
        let member_expression = self.ctx.tree.get(*left);
        let dir::Expression::Member { left, name, .. } = member_expression else {
            return false;
        };
        if *name != Some(self.find_name) && *name != Some(self.find_last_name) {
            return false;
        }

        self.is_array_receiver(*left)
    }

    /// Return true when the receiver expression is an array type.
    fn is_array_receiver(&mut self, expression_id: dir::LocalNodeId<dir::Expression>) -> bool {
        // resolve the receiver type
        let Some(type_id) = self.ctx.expression_type_id(expression_id) else {
            return false;
        };

        is_array_type(self.ctx.types, type_id, Some(self.array_symbol))
    }
}

impl NodeVisitor for PreferArraySomeVisitor<'_, '_> {
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

/// Check filter().length comparisons against constants.
fn check_filter_length_comparison(
    operator: dir::BinaryOperator,
    constant: i64,
) -> Option<ArraySomeCheck> {
    match operator {
        dir::BinaryOperator::GreaterThan if constant == 0 => Some(ArraySomeCheck::AnyMatch),
        dir::BinaryOperator::GreaterThanOrEqual if constant == 1 => Some(ArraySomeCheck::AnyMatch),
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict if constant == 0 => {
            Some(ArraySomeCheck::AnyMatch)
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict if constant == 0 => {
            Some(ArraySomeCheck::NoMatch)
        }
        dir::BinaryOperator::LessThan if constant == 1 => Some(ArraySomeCheck::NoMatch),
        dir::BinaryOperator::LessThanOrEqual if constant == 0 => Some(ArraySomeCheck::NoMatch),
        _ => None,
    }
}

/// Check findIndex() comparisons against constants.
fn check_find_index_comparison(
    operator: dir::BinaryOperator,
    constant: i64,
) -> Option<ArraySomeCheck> {
    match operator {
        dir::BinaryOperator::GreaterThan if constant == -1 => Some(ArraySomeCheck::AnyMatch),
        dir::BinaryOperator::GreaterThanOrEqual if constant == 0 => Some(ArraySomeCheck::AnyMatch),
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict if constant == -1 => {
            Some(ArraySomeCheck::AnyMatch)
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict if constant == -1 => {
            Some(ArraySomeCheck::NoMatch)
        }
        dir::BinaryOperator::LessThan if constant == 0 => Some(ArraySomeCheck::NoMatch),
        dir::BinaryOperator::LessThanOrEqual if constant == -1 => Some(ArraySomeCheck::NoMatch),
        _ => None,
    }
}

/// Check find and findLast comparisons against nullish values.
fn check_find_nullish_comparison(
    operator: dir::BinaryOperator,
    constant: &ConstValue,
) -> Option<ArraySomeCheck> {
    if !matches!(constant, ConstValue::Null | ConstValue::Undefined) {
        return None;
    }

    match operator {
        dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
            Some(ArraySomeCheck::AnyMatch)
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => {
            Some(ArraySomeCheck::NoMatch)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Report filter length comparisons against zero.
    #[test]
    fn test_flags_filter_length_non_empty_check() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_flags_filter_length_non_empty_check.ds",
            r#"
let items = [1, 2, 3];
let has = items.filter(item => item > 1).length > 0;
"#,
        );
        test.result(result).assert_lint("prefer-array-some");
    }

    /// Report filter length comparisons that check for emptiness.
    #[test]
    fn test_flags_filter_length_empty_check() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_flags_filter_length_empty_check.ds",
            r#"
let items = [1, 2, 3];
let empty = items.filter(item => item > 1).length == 0;
"#,
        );
        test.result(result).assert_lint("prefer-array-some");
    }

    /// Report findIndex comparisons against -1.
    #[test]
    fn test_flags_find_index_comparison() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_flags_find_index_comparison.ds",
            r#"
let items = [1, 2, 3];
let has = items.findIndex(item => item > 1) !== -1;
"#,
        );
        test.result(result).assert_lint("prefer-array-some");
    }

    /// Safely rewrite findIndex any-match checks.
    #[test]
    fn test_fix_find_index_any_match() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_index_any_match.ds",
            r#"
let items = [1, 2, 3];
let has = items.findIndex(item => item > 1) !== -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let has = items.some((item) => item > 1);
"#,
            );
    }

    /// Safely rewrite findIndex no-match checks.
    #[test]
    fn test_fix_find_index_no_match() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_index_no_match.ds",
            r#"
let items = [1, 2, 3];
let missing = items.findIndex(item => item > 5) === -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let missing = !items.some((item) => item > 5);
"#,
            );
    }

    /// Safely rewrite findLastIndex any-match checks.
    #[test]
    fn test_fix_find_last_index_any_match() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_last_index_any_match.ds",
            r#"
let items = [1, 2, 3];
let has = items.findLastIndex(item => item > 1) !== -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let has = items.some((item) => item > 1);
"#,
            );
    }

    /// Safely rewrite findLastIndex no-match checks.
    #[test]
    fn test_fix_find_last_index_no_match() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_last_index_no_match.ds",
            r#"
let items = [1, 2, 3];
let missing = items.findLastIndex(item => item > 5) == -1;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let missing = !items.some((item) => item > 5);
"#,
            );
    }

    /// Safely rewrite find nullish any-match checks.
    #[test]
    fn test_fix_find_not_null_check() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_not_null_check.ds",
            r#"
let items = [1, 2, 3];
let has = items.find(item => item > 1) != null;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let has = items.some((item) => item > 1);
"#,
            );
    }

    /// Safely rewrite find undefined no-match checks.
    #[test]
    fn test_fix_find_equal_undefined_check() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_fix_find_equal_undefined_check.ds",
            r#"
let items = [1, 2, 3];
let missing = items.find(item => item > 5) === undefined;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_fix("prefer-array-some")
            .assert_safe_fixed(
                r#"
let items = [1, 2, 3];
let missing = !items.some((item) => item > 5);
"#,
            );
    }

    /// Keep filter-length diagnostics without fix because some short-circuits.
    #[test]
    fn test_no_fix_filter_length_pattern() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_no_fix_filter_length_pattern.ds",
            r#"
let items = [1, 2, 3];
let has = items.filter(item => item > 1).length > 0;
"#,
        );
        test.result(result)
            .assert_lint("prefer-array-some")
            .assert_has_no_fix("prefer-array-some");
    }

    /// Allow filter length comparisons that are not simple existence checks.
    #[test]
    fn test_allows_filter_length_thresholds() {
        let test = TestProgram::for_rule_with_prelude(PreferArraySome);
        let result = test.lint_dir(
            "prefer_array_some/test_allows_filter_length_thresholds.ds",
            r#"
let items = [1, 2, 3];
let many = items.filter(item => item > 1).length > 1;
"#,
        );
        test.result(result).assert_no_lint("prefer-array-some");
    }
}
