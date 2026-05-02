use crate::LintMeta;
use std::collections::HashMap;

use destack_ast::{self as ast, Declaration, FunctionMode, Key, Member, Property, TypeMember};
use destack_source::Span;
use destack_workspace::{GroupedAccessorPairsOrder, LintSeverity};

use crate::rules::common::{expression_signature_for_tree, span_has_comment};
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

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
    fn meta(&self) -> &'static LintMeta {
        GroupedAccessorPairs::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);

            // declaration members
            match declaration {
                Declaration::Class(declaration) => {
                    check_ungrouped_accessors(
                        ctx,
                        meta,
                        &declaration.members,
                        collect_accessor_slots(ctx, &declaration.members),
                        ctx.options.style.grouped_accessor_pairs_order,
                        true,
                    );
                }
                Declaration::Struct(declaration) => {
                    if !ctx.options.style.grouped_accessor_pairs_enforce_for_types {
                        continue;
                    }

                    check_ungrouped_accessors(
                        ctx,
                        meta,
                        &declaration.members,
                        collect_accessor_slots(ctx, &declaration.members),
                        ctx.options.style.grouped_accessor_pairs_order,
                        true,
                    );
                }
                Declaration::Extension(declaration) => {
                    if !ctx.options.style.grouped_accessor_pairs_enforce_for_types {
                        continue;
                    }

                    check_ungrouped_accessors(
                        ctx,
                        meta,
                        &declaration.members,
                        collect_accessor_slots(ctx, &declaration.members),
                        ctx.options.style.grouped_accessor_pairs_order,
                        true,
                    );
                }
                Declaration::Interface(declaration) => {
                    if !ctx.options.style.grouped_accessor_pairs_enforce_for_types {
                        continue;
                    }

                    check_ungrouped_accessors(
                        ctx,
                        meta,
                        &declaration.members,
                        collect_accessor_slots(ctx, &declaration.members),
                        ctx.options.style.grouped_accessor_pairs_order,
                        false,
                    );
                }
                _ => {}
            }
        }

        // also check object expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::ObjectExpression { properties, .. } = expression else {
                continue;
            };

            check_ungrouped_accessors(
                ctx,
                meta,
                properties,
                collect_accessor_slots(ctx, properties),
                ctx.options.style.grouped_accessor_pairs_order,
                true,
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

/// One accessor candidate in an ordered item list.
#[derive(Debug, Clone)]
struct AccessorSlot {
    /// The item index within its owner list.
    index: usize,
    /// The accessor owner partition.
    owner: AccessorOwner,
    /// The accessor key identity.
    key: AccessorKey,
    /// The accessor mode.
    mode: FunctionMode,
}

/// One item that can contribute an accessor pair slot.
trait AccessorItem {
    /// Get the accessor key when one exists.
    fn key(&self) -> Option<&Key>;

    /// Get the accessor signature when one exists.
    fn signature(&self) -> Option<&ast::FunctionSignature>;

    /// Get the accessor owner partition.
    fn owner(&self) -> AccessorOwner {
        AccessorOwner::Instance
    }
}

impl AccessorItem for Member {
    fn key(&self) -> Option<&Key> {
        self.key()
    }

    fn signature(&self) -> Option<&ast::FunctionSignature> {
        self.signature()
    }

    fn owner(&self) -> AccessorOwner {
        if self.is_static() {
            return AccessorOwner::Static;
        }

        AccessorOwner::Instance
    }
}

impl AccessorItem for TypeMember {
    fn key(&self) -> Option<&Key> {
        self.key()
    }

    fn signature(&self) -> Option<&ast::FunctionSignature> {
        self.signature()
    }
}

impl AccessorItem for Property {
    fn key(&self) -> Option<&Key> {
        self.key()
    }

    fn signature(&self) -> Option<&ast::FunctionSignature> {
        self.signature()
    }
}

/// Return key identity for one AST key.
fn accessor_key(ctx: &LintAstContext<'_>, key: &Key) -> Option<AccessorKey> {
    match key {
        Key::Name(name) => Some(AccessorKey::Name(name.string())),
        Key::Private(name) => Some(AccessorKey::Private(*name)),
        Key::Expression(expression_id) => Some(AccessorKey::Computed(
            expression_signature_for_tree(ctx.tree, ctx.strings, *expression_id),
        )),
    }
}

/// Collect accessor slots from one ordered item list.
fn collect_accessor_slots<T>(
    ctx: &LintAstContext<'_>,
    items: &[ast::LocalNodeId<T>],
) -> Vec<AccessorSlot>
where
    T: AccessorItem + ast::Node + Clone,
    ast::Tree: ast::TreeImpl<T>,
{
    let mut slots = Vec::new();

    // accessor items
    for (index, item_id) in items.iter().enumerate() {
        let item = ctx.tree.get(*item_id);

        let Some(signature) = item.signature() else {
            continue;
        };

        let Some(mode) = signature.mode else {
            continue;
        };

        let Some(key) = item.key().and_then(|key| accessor_key(ctx, key)) else {
            continue;
        };

        slots.push(AccessorSlot {
            index,
            owner: item.owner(),
            key,
            mode,
        });
    }

    slots
}

/// Check one ordered accessor list for ungrouped getter and setter pairs.
fn check_ungrouped_accessors<T>(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    items: &[ast::LocalNodeId<T>],
    slots: Vec<AccessorSlot>,
    order: GroupedAccessorPairsOrder,
    allow_fix: bool,
) where
    T: ast::Node + Clone,
{
    // collect getter and setter indices per owner and key
    let mut accessor_indices: HashMap<(AccessorOwner, AccessorKey), AccessorIndices> =
        HashMap::new();

    for slot in &slots {
        let entry = accessor_indices
            .entry((slot.owner, slot.key.clone()))
            .or_default();

        match slot.mode {
            FunctionMode::Getter => entry.getters.push(slot.index),
            FunctionMode::Setter => entry.setters.push(slot.index),
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
            let later_item_id = items[later_idx];
            let severity = ctx.get_effective_severity(meta, later_item_id);
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
            let mut diagnostic = LintReport::new(
                GROUPED_ACCESSOR_PAIRS.id,
                GROUPED_ACCESSOR_PAIRS.code,
                GROUPED_ACCESSOR_PAIRS.category,
                severity,
                message,
                ctx.tree.get_span(later_item_id),
            )
            .label(label);

            if ctx.compute_fixes && allow_fix && diff != 1 && !has_reported_reorder_fix {
                if let Some(fix) = grouped_accessor_fix(ctx, items, getter_idx, setter_idx) {
                    diagnostic = diagnostic.fix(fix);
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

/// Build an unsafe reorder fix for one ordered accessor list.
fn grouped_accessor_fix<T>(
    ctx: &LintAstContext<'_>,
    items: &[ast::LocalNodeId<T>],
    getter_index: usize,
    setter_index: usize,
) -> Option<LintFix>
where
    T: ast::Node + Clone,
{
    if items.is_empty() {
        return None;
    }

    let first_item_id = *items.first()?;
    let last_item_id = *items.last()?;
    let first_item_span = ctx.tree.get_span(first_item_id);
    let last_item_span = ctx.tree.get_span(last_item_id);
    let full_span = Span::new(
        first_item_span.file,
        first_item_span.start,
        last_item_span.end,
    );

    if span_has_comment(ctx.tree, full_span) {
        return None;
    }

    let reordered_indices = reorder_accessor_indices(items.len(), getter_index, setter_index)?;
    let replacement = reordered_indices
        .iter()
        .map(|item_index| {
            let item_id = items[*item_index];
            ctx.get_span_text(ctx.tree.get_span(item_id)).to_string()
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
                options.style.grouped_accessor_pairs_order =
                    GroupedAccessorPairsOrder::GetBeforeSet;
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
                options.style.grouped_accessor_pairs_enforce_for_types = true;
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
                options.style.grouped_accessor_pairs_enforce_for_types = true;
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
