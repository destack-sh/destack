use std::collections::{HashMap, HashSet};

use destack_dir::{self as dir, Member};
use destack_workspace::LintSeverity;

use crate::rules::common::{collect_assigned_symbol_usage, expression_unwrap_parenthesized};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer `readonly` for private fields that are never mutated.
    ///
    /// This rule intentionally focuses on private fields because those writes are
    /// fully observable inside the current module and avoid public API false positives.
    #[lint(
        id = "prefer-readonly",
        code = "LY078",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub PreferReadonly,
    "Prefer readonly for non-mutated private fields"
}

impl LintRule for PreferReadonly {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        PreferReadonly::meta()
    }

    /// Check module DIR nodes for non-mutated private fields.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let candidate_fields = collect_private_mutable_field_candidates(ctx);
        if candidate_fields.is_empty() {
            return;
        }

        let mutated_fields = collect_mutated_candidate_fields(ctx, &candidate_fields);

        // report each candidate field that was never mutated
        for (symbol_id, member_id) in candidate_fields {
            if mutated_fields.contains(&symbol_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, member_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.get_span(member_id);
            ctx.report(
                LintReport::new(
                    PREFER_READONLY.id,
                    PREFER_READONLY.code,
                    PREFER_READONLY.category,
                    severity,
                    "private field is never mutated",
                    span,
                )
                .label("mark this field as `readonly`"),
            );
        }
    }
}

/// Collect mutable private field candidates keyed by their symbol id.
fn collect_private_mutable_field_candidates(
    ctx: &LintModuleContext<'_>,
) -> HashMap<dir::GlobalSymbolId, dir::LocalNodeId<Member>> {
    let mut candidates = HashMap::new();

    // inspect class and struct member fields
    for (_declaration_id, declaration) in ctx.dir.iter_nodes_of_type::<dir::Declaration>() {
        let members = match declaration {
            dir::Declaration::Class(declaration) => &declaration.members,
            dir::Declaration::Struct(declaration) => &declaration.members,
            _ => continue,
        };

        for member_id in members {
            let member = ctx.dir.get(*member_id);
            let Member::Field {
                key,
                visibility,
                is_readonly,
                ..
            } = member
            else {
                continue;
            };

            if !field_is_private(*visibility, key) {
                continue;
            }
            if *is_readonly {
                continue;
            }

            let Some(global_symbol_id) = ctx.symbol_for_node(*member_id) else {
                continue;
            };
            candidates.insert(global_symbol_id, *member_id);
        }
    }

    candidates
}

/// Return true when one member field is private.
fn field_is_private(visibility: Option<dir::Visibility>, key: &dir::Key) -> bool {
    let is_private_by_modifier = visibility == Some(dir::Visibility::Private);
    let is_private_by_key = matches!(key, dir::Key::Private(_));

    is_private_by_modifier || is_private_by_key
}

/// Collect candidate field symbols that are mutated outside constructor initialization.
fn collect_mutated_candidate_fields(
    ctx: &LintModuleContext<'_>,
    candidates: &HashMap<dir::GlobalSymbolId, dir::LocalNodeId<Member>>,
) -> HashSet<dir::GlobalSymbolId> {
    let assigned_symbols = collect_assigned_symbol_usage(
        ctx.module_id(),
        ctx.dir.tree(),
        ctx.types,
        |assignment_expression_id, assigned_expression_id| {
            !assignment_is_constructor_self_initialization(
                ctx.dir.tree(),
                assignment_expression_id,
                assigned_expression_id,
            )
        },
    );

    assigned_symbols
        .into_iter()
        .filter(|symbol_id| candidates.contains_key(symbol_id))
        .collect()
}

/// Return true when one assignment is constructor initialization of a `this` field.
fn assignment_is_constructor_self_initialization(
    tree: &dir::Tree,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
    assigned_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    if !assignment_is_inside_constructor_method(tree, assignment_expression_id) {
        return false;
    }

    assigned_expression_is_self_field_reference(tree, assigned_expression_id)
}

/// Return true when one assignment expression is enclosed in a constructor method.
fn assignment_is_inside_constructor_method(
    tree: &dir::Tree,
    assignment_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current_node_id = assignment_expression_id.id;

    // walk parent nodes until the nearest member or declaration
    while let Some(parent_id) = tree.get_parent(current_node_id) {
        if parent_id.ty == dir::NodeType::Member {
            let member_id = parent_id.into_typed::<Member>();
            let member = tree.get(member_id);
            let Member::Method { signature, .. } = member else {
                return false;
            };

            return signature.role == Some(dir::FunctionRole::Constructor);
        }
        if parent_id.ty == dir::NodeType::Declaration {
            return false;
        }

        current_node_id = parent_id.id;
    }

    false
}

/// Return true when one assignment target references a field on `this`.
fn assigned_expression_is_self_field_reference(
    tree: &dir::Tree,
    assigned_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let assigned_expression_id = expression_unwrap_parenthesized(tree, assigned_expression_id);
    let assigned_expression = tree.get(assigned_expression_id);
    let receiver_expression_id = match assigned_expression {
        dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => *left,
        _ => return false,
    };

    expression_is_this_reference(tree, receiver_expression_id)
}

/// Return true when one expression resolves to a `this` reference.
fn expression_is_this_reference(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression_id = expression_unwrap_parenthesized(tree, expression_id);
    let expression = tree.get(expression_id);
    match expression {
        dir::Expression::This => true,
        dir::Expression::As { expression, .. } | dir::Expression::Satisfies { expression, .. } => {
            expression_is_this_reference(tree, *expression)
        }
        dir::Expression::MoveOf { right, .. } | dir::Expression::BorrowOf { right, .. } => {
            expression_is_this_reference(tree, *right)
        }
        dir::Expression::Maybe { left, .. } | dir::Expression::Must { left, .. } => {
            expression_is_this_reference(tree, *left)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag private fields that are never mutated.
    #[test]
    fn test_flags_non_mutated_private_field() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_flags_non_mutated_private_field.ds",
            r#"
class Counter {
    private count: int32 = 0;

    read(): int32 {
        return this.count;
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-readonly");
    }

    /// Allow private fields that are mutated after initialization.
    #[test]
    fn test_allows_mutated_private_field() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_mutated_private_field.ds",
            r#"
class Counter {
    private count: int32 = 0;

    increment(): void {
        this.count += 1;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Flag private fields written only in constructors.
    #[test]
    fn test_flags_private_field_assigned_in_constructor_only() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_flags_private_field_assigned_in_constructor_only.ds",
            r#"
class Counter {
    private count: int32 = 0;

    constructor(value: int32) {
        this.count = value;
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-readonly");
    }

    /// Allow fields that are already readonly.
    #[test]
    fn test_allows_existing_readonly_private_field() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_existing_readonly_private_field.ds",
            r#"
class Counter {
    private readonly count: int32 = 0;

    read(): int32 {
        return this.count;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Ignore public fields because writes may happen outside the module.
    #[test]
    fn test_ignores_public_field() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_ignores_public_field.ds",
            r#"
class Counter {
    count: int32 = 0;
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Flag private `#` fields that are never mutated.
    #[test]
    fn test_flags_non_mutated_private_hash_field() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_flags_non_mutated_private_hash_field.ds",
            r#"
class Counter {
    #count: int32 = 0;

    read(): int32 {
        return this.#count;
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-readonly");
    }

    /// Allow private `#` fields mutated with increment operators.
    #[test]
    fn test_allows_mutated_private_hash_field_with_increment() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_mutated_private_hash_field_with_increment.ds",
            r#"
class Counter {
    #count: int32 = 0;

    increment(): void {
        this.#count++;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Allow private fields initialized in constructor and mutated later.
    #[test]
    fn test_allows_constructor_then_method_mutation() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_constructor_then_method_mutation.ds",
            r#"
class Counter {
    private count: int32 = 0;

    constructor(start: int32) {
        this.count = start;
    }

    decrement(): void {
        this.count--;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Allow private fields mutated from nested functions inside constructors.
    #[test]
    fn test_allows_nested_function_mutation_inside_constructor() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_nested_function_mutation_inside_constructor.ds",
            r#"
class Counter {
    private count: int32 = 0;

    constructor(start: int32) {
        function bump(counter: Counter): void {
            counter.count += 1;
        }

        this.count = start;
        bump(this);
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }

    /// Allow fields mutated through other instances inside constructors.
    #[test]
    fn test_allows_constructor_assignment_to_other_instance() {
        let test = TestProgram::for_rule_without_prelude(PreferReadonly);
        let result = test.lint_dir(
            "prefer_readonly/test_allows_constructor_assignment_to_other_instance.ds",
            r#"
class Counter {
    private count: int32 = 0;

    constructor(other: Counter) {
        other.count = 1;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-readonly");
    }
}
