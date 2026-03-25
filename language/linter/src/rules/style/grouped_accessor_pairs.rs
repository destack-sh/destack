use std::collections::HashMap;

use destack_ast::{self as ast, BindingAnchor, Declaration, FunctionMode, Key, Member};
use destack_workspace::{GroupedAccessorPairsOrder, LintSeverity};

use crate::rules::common::{expression_structural_signature, span_has_comment_trivia};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Require grouped accessor pairs in object literals and classes.
    ///
    /// Getters and setters for the same property should be defined adjacent to each other.
    /// This improves code readability and makes it easier to understand the property's behavior.
    ///
    /// ```
    /// // bad
    /// class Example {
    ///     get foo() { return this._foo }
    ///     bar = 1
    ///     set foo(v) { this._foo = v }  // setter far from getter
    /// }
    ///
    /// // good
    /// class Example {
    ///     get foo() { return this._foo }
    ///     set foo(v) { this._foo = v }  // getter and setter together
    ///     bar = 1
    /// }
    /// ```
    #[lint(
        id = "grouped-accessor-pairs",
        code = "LY013",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub GroupedAccessorPairs,
    "Require grouped accessor pairs"
}

impl LintRule for GroupedAccessorPairs {
    fn meta(&self) -> &'static crate::LintMeta {
        GroupedAccessorPairs::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let members = match declaration {
                Declaration::Class { members, .. } => members,
                Declaration::Struct { members, .. }
                | Declaration::Interface { members, .. }
                | Declaration::Extension { members, .. } => {
                    if !ctx.options.grouped_accessor_pairs_enforce_for_types {
                        continue;
                    }

                    members
                }
                _ => continue,
            };

            check_members_for_ungrouped_accessors(
                ctx,
                meta,
                members,
                ctx.options.grouped_accessor_pairs_order,
            );
        }

        // also check object expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::ObjectExpression { properties, .. } = expression else {
                continue;
            };

            check_properties_for_ungrouped_accessors(
                ctx,
                meta,
                properties,
                ctx.options.grouped_accessor_pairs_order,
            );
        }
    }
}

/// Member owner partition for accessor grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AccessorOwner {
    /// Instance member accessor.
    Instance,
    /// Static member accessor.
    Static,
}

/// Key identity for accessor pairing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AccessorKey {
    /// Named key identity.
    Name(ast::StringId),
    /// Private key identity.
    Private(ast::StringId),
    /// Computed key signature.
    Computed(Vec<u64>),
}

/// Accessor index buckets for one key.
#[derive(Debug, Default)]
struct AccessorIndices {
    /// Getter member indices.
    getters: Vec<usize>,
    /// Setter member indices.
    setters: Vec<usize>,
}

/// Return key identity for one AST key.
fn accessor_key(ctx: &LintAstContext<'_>, key: &Key) -> Option<AccessorKey> {
    match key {
        Key::Name(name) => Some(AccessorKey::Name(name.string())),
        Key::Private(name) => Some(AccessorKey::Private(*name)),
        Key::Expression(expression_id) => Some(AccessorKey::Computed(
            expression_structural_signature(ctx.tree, ctx.strings, *expression_id),
        )),
        Key::NamedExpression { key, .. } => Some(AccessorKey::Computed(
            expression_structural_signature(ctx.tree, ctx.strings, *key),
        )),
    }
}

/// Return owner partition for one member.
fn member_owner(member: &Member) -> AccessorOwner {
    let modifiers = match member {
        Member::Method { modifiers, .. } => modifiers,
        _ => return AccessorOwner::Instance,
    };

    if modifiers.as_ref().and_then(|modifier| modifier.anchor) == Some(BindingAnchor::Static) {
        return AccessorOwner::Static;
    }

    AccessorOwner::Instance
}

/// Check members in object-like declarations for ungrouped accessor pairs.
fn check_members_for_ungrouped_accessors(
    ctx: &mut LintAstContext<'_>,
    meta: &'static crate::LintMeta,
    members: &[ast::LocalNodeId<Member>],
    order: GroupedAccessorPairsOrder,
) {
    // collect getter and setter indices per owner partition and key
    let mut accessor_indices: HashMap<(AccessorOwner, AccessorKey), AccessorIndices> =
        HashMap::new();

    for (index, member_id) in members.iter().enumerate() {
        let member = ctx.tree.get(*member_id);
        let Member::Method {
            key: Some(key),
            signature,
            ..
        } = member
        else {
            continue;
        };

        let Some(mode) = signature.mode else {
            continue;
        };

        let Some(key) = accessor_key(ctx, key) else {
            continue;
        };

        let owner = member_owner(member);
        let entry = accessor_indices.entry((owner, key)).or_default();

        match mode {
            FunctionMode::Getter => entry.getters.push(index),
            FunctionMode::Setter => entry.setters.push(index),
            _ => {}
        }
    }

    // report one accessor pair only when exactly one getter and one setter exist
    let mut has_reported_reorder_fix = false;
    for ((_, key), indices) in accessor_indices {
        if indices.getters.len() != 1 || indices.setters.len() != 1 {
            continue;
        }

        let getter_idx = indices.getters[0];
        let setter_idx = indices.setters[0];

        let diff = getter_idx.abs_diff(setter_idx);
        let order_violation = accessor_order_violation(order, getter_idx, setter_idx);
        if diff != 1 || order_violation {
            let later_idx = getter_idx.max(setter_idx);
            let later_member_id = members[later_idx];
            let severity = ctx.get_effective_severity(meta, later_member_id);
            if !severity.is_enabled() {
                continue;
            }

            let accessor_name = match key {
                AccessorKey::Name(name) | AccessorKey::Private(name) => {
                    ctx.strings.get(name).as_ref().to_string()
                }
                AccessorKey::Computed(_) => "<computed>".to_string(),
            };

            let (message, label) = if diff != 1 {
                (
                    format!("getter and setter for `{accessor_name}` are not adjacent"),
                    "move to be adjacent to its counterpart",
                )
            } else {
                accessor_order_message(order, &accessor_name)
            };
            let mut diagnostic = LintDiagnostic::new(
                GROUPED_ACCESSOR_PAIRS.id,
                GROUPED_ACCESSOR_PAIRS.code,
                GROUPED_ACCESSOR_PAIRS.category,
                severity,
                message,
                ctx.module.file_id,
                ctx.tree.get_span(later_member_id),
            )
            .with_label(label);

            if ctx.compute_fixes && diff != 1 && !has_reported_reorder_fix {
                if let Some(fix) = grouped_member_accessor_fix(ctx, members, getter_idx, setter_idx)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }
                has_reported_reorder_fix = true;
            }

            ctx.report(diagnostic);
        }
    }
}

/// Check properties in object expressions for ungrouped accessor pairs.
fn check_properties_for_ungrouped_accessors(
    ctx: &mut LintAstContext<'_>,
    meta: &'static crate::LintMeta,
    properties: &[ast::LocalNodeId<ast::Property>],
    order: GroupedAccessorPairsOrder,
) {
    // collect getter and setter indices per key
    let mut accessor_indices: HashMap<AccessorKey, AccessorIndices> = HashMap::new();

    for (index, property_id) in properties.iter().enumerate() {
        let property = ctx.tree.get(*property_id);
        let ast::Property::Method {
            key: Some(key),
            signature,
            ..
        } = property
        else {
            continue;
        };

        let Some(mode) = signature.mode else {
            continue;
        };

        let Some(key) = accessor_key(ctx, key) else {
            continue;
        };

        let entry = accessor_indices.entry(key).or_default();

        match mode {
            FunctionMode::Getter => entry.getters.push(index),
            FunctionMode::Setter => entry.setters.push(index),
            _ => {}
        }
    }

    // report one accessor pair only when exactly one getter and one setter exist
    let mut has_reported_reorder_fix = false;
    for (key, indices) in accessor_indices {
        if indices.getters.len() != 1 || indices.setters.len() != 1 {
            continue;
        }

        let getter_idx = indices.getters[0];
        let setter_idx = indices.setters[0];

        let diff = getter_idx.abs_diff(setter_idx);
        let order_violation = accessor_order_violation(order, getter_idx, setter_idx);
        if diff != 1 || order_violation {
            let later_idx = getter_idx.max(setter_idx);
            let later_prop_id = properties[later_idx];
            let severity = ctx.get_effective_severity(meta, later_prop_id);
            if !severity.is_enabled() {
                continue;
            }

            let accessor_name = match key {
                AccessorKey::Name(name) | AccessorKey::Private(name) => {
                    ctx.strings.get(name).as_ref().to_string()
                }
                AccessorKey::Computed(_) => "<computed>".to_string(),
            };

            let (message, label) = if diff != 1 {
                (
                    format!("getter and setter for `{accessor_name}` are not adjacent"),
                    "move to be adjacent to its counterpart",
                )
            } else {
                accessor_order_message(order, &accessor_name)
            };
            let mut diagnostic = LintDiagnostic::new(
                GROUPED_ACCESSOR_PAIRS.id,
                GROUPED_ACCESSOR_PAIRS.code,
                GROUPED_ACCESSOR_PAIRS.category,
                severity,
                message,
                ctx.module.file_id,
                ctx.tree.get_span(later_prop_id),
            )
            .with_label(label);

            if ctx.compute_fixes && diff != 1 && !has_reported_reorder_fix {
                if let Some(fix) =
                    grouped_property_accessor_fix(ctx, properties, getter_idx, setter_idx)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }
                has_reported_reorder_fix = true;
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when one adjacent accessor pair violates the configured order.
fn accessor_order_violation(
    order: GroupedAccessorPairsOrder,
    getter_index: usize,
    setter_index: usize,
) -> bool {
    match order {
        GroupedAccessorPairsOrder::AnyOrder => false,
        GroupedAccessorPairsOrder::GetBeforeSet => getter_index > setter_index,
        GroupedAccessorPairsOrder::SetBeforeGet => setter_index > getter_index,
    }
}

/// Return the strict-order diagnostic message and label.
fn accessor_order_message(
    order: GroupedAccessorPairsOrder,
    accessor_name: &str,
) -> (String, &'static str) {
    match order {
        GroupedAccessorPairsOrder::AnyOrder => {
            unreachable!("order diagnostics require a strict accessor order")
        }
        GroupedAccessorPairsOrder::GetBeforeSet => (
            format!("getter for `{accessor_name}` should come before setter"),
            "place the getter before the setter",
        ),
        GroupedAccessorPairsOrder::SetBeforeGet => (
            format!("setter for `{accessor_name}` should come before getter"),
            "place the setter before the getter",
        ),
    }
}

/// Build an unsafe reorder fix for object-like member accessor pairs.
fn grouped_member_accessor_fix(
    ctx: &LintAstContext<'_>,
    members: &[ast::LocalNodeId<Member>],
    getter_index: usize,
    setter_index: usize,
) -> Option<LintFix> {
    if members.is_empty() {
        return None;
    }

    let first_member_span = ctx.tree.get_span(*members.first()?);
    let last_member_span = ctx.tree.get_span(*members.last()?);
    let full_span = destack_source::Span::new(
        first_member_span.file,
        first_member_span.start,
        last_member_span.end,
    );
    if span_has_comment_trivia(ctx.tree, full_span) {
        return None;
    }

    let reordered_indices = reorder_accessor_indices(members.len(), getter_index, setter_index)?;
    let replacement = reordered_indices
        .iter()
        .map(|member_index| {
            let member_id = members[*member_index];
            ctx.get_span_text(ctx.tree.get_span(member_id)).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let edits = ctx
        .edit_builder()
        .replace(full_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Group getter and setter accessors together").with_edits(edits))
}

/// Build an unsafe reorder fix for object property accessor pairs.
fn grouped_property_accessor_fix(
    ctx: &LintAstContext<'_>,
    properties: &[ast::LocalNodeId<ast::Property>],
    getter_index: usize,
    setter_index: usize,
) -> Option<LintFix> {
    if properties.is_empty() {
        return None;
    }

    let first_property_span = ctx.tree.get_span(*properties.first()?);
    let last_property_span = ctx.tree.get_span(*properties.last()?);
    let full_span = destack_source::Span::new(
        first_property_span.file,
        first_property_span.start,
        last_property_span.end,
    );
    if span_has_comment_trivia(ctx.tree, full_span) {
        return None;
    }

    let reordered_indices = reorder_accessor_indices(properties.len(), getter_index, setter_index)?;
    let replacement = reordered_indices
        .iter()
        .map(|property_index| {
            let property_id = properties[*property_index];
            ctx.get_span_text(ctx.tree.get_span(property_id))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    let edits = ctx
        .edit_builder()
        .replace(full_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Group getter and setter accessors together").with_edits(edits))
}

/// Return reordered indices with the later accessor moved next to the earlier one.
fn reorder_accessor_indices(
    item_count: usize,
    getter_index: usize,
    setter_index: usize,
) -> Option<Vec<usize>> {
    if getter_index >= item_count || setter_index >= item_count {
        return None;
    }

    let earlier_index = getter_index.min(setter_index);
    let later_index = getter_index.max(setter_index);
    if later_index.abs_diff(earlier_index) <= 1 {
        return None;
    }

    let mut reordered_indices = (0..item_count).collect::<Vec<_>>();
    let moved_index = reordered_indices.remove(later_index);
    reordered_indices.insert(earlier_index + 1, moved_index);
    Some(reordered_indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_adjacent_accessors_allowed() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_adjacent_accessors_allowed.ds",
            r#"
class Example {
    get foo() { return this._foo }
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_setter_then_getter_adjacent_allowed() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_setter_then_getter_adjacent_allowed.ds",
            r#"
class Example {
    set foo(v) { this._foo = v }
    get foo() { return this._foo }
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_reports_setter_then_getter_when_get_before_set_is_required() {
        let test =
            TestProgram::for_rule_without_prelude(GroupedAccessorPairs).with_options(|options| {
                options.grouped_accessor_pairs_order = GroupedAccessorPairsOrder::GetBeforeSet;
            });
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_reports_setter_then_getter_when_get_before_set_is_required.ds",
            r#"
class Example {
    set foo(v) { this._foo = v }
    get foo() { return this._foo }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_non_adjacent_accessors_detected() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_non_adjacent_accessors_detected.ds",
            r#"
class Example {
    get foo() { return this._foo }
    bar: int32 = 1
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result)
            .assert_lint("grouped-accessor-pairs")
            .assert_unsafe_fixed(
                r#"
class Example {
    get foo() {
        return this._foo;
    }
    set foo(v) {
        this._foo = v
    }
    bar: int32 = 1;
}
"#,
            );
    }

    #[test]
    fn test_multiple_fields_between_detected() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_multiple_fields_between_detected.ds",
            r#"
class Example {
    get foo() { return this._foo }
    bar: int32 = 1
    baz: string = ""
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_only_getter_allowed() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_only_getter_allowed.ds",
            r#"
class Example {
    get foo() { return this._foo }
    bar: int32 = 1
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_only_setter_allowed() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_only_setter_allowed.ds",
            r#"
class Example {
    set foo(v) { this._foo = v }
    bar: int32 = 1
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_object_literal_non_adjacent_detected() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_object_literal_non_adjacent_detected.ds",
            r#"
const obj = {
    get foo() { return this._foo },
    bar: 1,
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result)
            .assert_lint("grouped-accessor-pairs")
            .assert_unsafe_fixed(
                r#"
const obj = {
    get foo() {
        return this._foo;
    },
    set foo(v) {
        this._foo = v
    },
    bar: 1,
};
"#,
            );
    }

    #[test]
    fn test_object_literal_adjacent_allowed() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_object_literal_adjacent_allowed.ds",
            r#"
const obj = {
    get foo() { return this._foo },
    set foo(v) { this._foo = v },
    bar: 1
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_multiple_accessor_pairs_one_ungrouped() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_multiple_accessor_pairs_one_ungrouped.ds",
            r#"
class Example {
    get foo() { return this._foo }
    set foo(v) { this._foo = v }
    get bar() { return this._bar }
    baz: int32 = 1
    set bar(v) { this._bar = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_struct_accessors_non_adjacent_detected() {
        let test =
            TestProgram::for_rule_without_prelude(GroupedAccessorPairs).with_options(|options| {
                options.grouped_accessor_pairs_enforce_for_types = true;
            });
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_struct_accessors_non_adjacent_detected.ds",
            r#"
struct Example {
    get foo() { this._foo }
    bar: int32
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_ignores_struct_accessors_by_default() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_ignores_struct_accessors_by_default.ds",
            r#"
struct Example {
    get foo() { this._foo }
    bar: int32
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_checks_struct_accessors_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(GroupedAccessorPairs).with_options(|options| {
                options.grouped_accessor_pairs_enforce_for_types = true;
            });
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_checks_struct_accessors_when_enabled.ds",
            r#"
struct Example {
    get foo() { this._foo }
    bar: int32
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_ignores_cross_static_instance_pairs() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_ignores_cross_static_instance_pairs.ds",
            r#"
class Example {
    static get foo() { return 1 }
    set foo(value) { this._foo = value }
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_ignores_duplicate_getter_or_setter_pairs() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_ignores_duplicate_getter_or_setter_pairs.ds",
            r#"
const value = {
    get foo() { return 1 },
    x: 1,
    get foo() { return 2 },
    set foo(next) { sink(next) }
}
"#,
        );
        test.result(result).assert_no_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_no_fix_when_member_range_contains_comments() {
        let test = TestProgram::for_rule_without_prelude(GroupedAccessorPairs);
        let result = test.lint_ast(
            "grouped_accessor_pairs/test_no_fix_when_member_range_contains_comments.ds",
            r#"
class Example {
    get foo() { return this._foo }
    // keep bar grouped with docs
    bar: int32 = 1
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result)
            .assert_lint("grouped-accessor-pairs")
            .assert_has_no_fix("grouped-accessor-pairs");
    }
}
