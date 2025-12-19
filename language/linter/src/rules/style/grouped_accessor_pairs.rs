use std::collections::HashMap;

use destack_ast::{self as ast, Declaration, FunctionMode, Key, Member, Name};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        code = "LY050",
        category = Style,
        level = Ast,
        fixable = No,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let members = match declaration {
                Declaration::Class { members, .. }
                | Declaration::Struct { members, .. }
                | Declaration::Interface { members, .. }
                | Declaration::Extension { members, .. } => members,
                _ => continue,
            };

            check_members_for_ungrouped_accessors(ctx, meta, members);
        }

        // also check object expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let ast::Expression::ObjectExpression { properties, .. } = expression else {
                continue;
            };

            check_properties_for_ungrouped_accessors(ctx, meta, properties);
        }
    }
}

/// Check members in class-like declarations for ungrouped accessor pairs.
fn check_members_for_ungrouped_accessors(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    members: &[ast::LocalNodeId<Member>],
) {
    // map property name to (getter_index, setter_index)
    let mut accessor_indices: HashMap<ast::StringId, (Option<usize>, Option<usize>)> =
        HashMap::new();
    for (index, member_id) in members.iter().enumerate() {
        // find the method member
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

        let name_id = match key {
            Key::Name(Name::Identifier(id)) | Key::Name(Name::String(id)) => *id,
            _ => continue,
        };
        match mode {
            FunctionMode::Getter => {
                accessor_indices.entry(name_id).or_insert((None, None)).0 = Some(index);
            }
            FunctionMode::Setter => {
                accessor_indices.entry(name_id).or_insert((None, None)).1 = Some(index);
            }
            _ => {}
        }
    }

    // check for non-adjacent pairs
    for (name_id, (getter_idx, setter_idx)) in accessor_indices {
        let (Some(getter_idx), Some(setter_idx)) = (getter_idx, setter_idx) else {
            continue;
        };

        // check if they are adjacent (difference of 1)
        let diff = getter_idx.abs_diff(setter_idx);
        if diff != 1 {
            let later_idx = getter_idx.max(setter_idx);
            let later_member_id = members[later_idx];
            let severity = ctx.get_effective_severity(meta, later_member_id);
            if !severity.is_enabled() {
                continue;
            }
            let name = ctx.strings.get(name_id);

            ctx.report(
                LintDiagnostic::new(
                    GROUPED_ACCESSOR_PAIRS.id,
                    GROUPED_ACCESSOR_PAIRS.code,
                    GROUPED_ACCESSOR_PAIRS.category,
                    severity,
                    format!("getter and setter for `{}` are not adjacent", name.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(later_member_id),
                )
                .with_label("move to be adjacent to its counterpart"),
            );
        }
    }
}

/// Check properties in object expressions for ungrouped accessor pairs.
fn check_properties_for_ungrouped_accessors(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    properties: &[ast::LocalNodeId<ast::Property>],
) {
    // map property name to (getter_index, setter_index)
    let mut accessor_indices: HashMap<ast::StringId, (Option<usize>, Option<usize>)> =
        HashMap::new();

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

        let name_id = match key {
            Key::Name(Name::Identifier(id)) | Key::Name(Name::String(id)) => *id,
            _ => continue,
        };

        match mode {
            FunctionMode::Getter => {
                accessor_indices.entry(name_id).or_insert((None, None)).0 = Some(index);
            }
            FunctionMode::Setter => {
                accessor_indices.entry(name_id).or_insert((None, None)).1 = Some(index);
            }
            _ => {}
        }
    }

    // check for non-adjacent pairs
    for (name_id, (getter_idx, setter_idx)) in accessor_indices {
        let (Some(getter_idx), Some(setter_idx)) = (getter_idx, setter_idx) else {
            continue;
        };

        // check if they are adjacent (difference of 1)
        let diff = getter_idx.abs_diff(setter_idx);
        if diff != 1 {
            let later_idx = getter_idx.max(setter_idx);
            let later_prop_id = properties[later_idx];
            let severity = ctx.get_effective_severity(meta, later_prop_id);
            if !severity.is_enabled() {
                continue;
            }
            let name = ctx.strings.get(name_id);

            ctx.report(
                LintDiagnostic::new(
                    GROUPED_ACCESSOR_PAIRS.id,
                    GROUPED_ACCESSOR_PAIRS.code,
                    GROUPED_ACCESSOR_PAIRS.category,
                    severity,
                    format!("getter and setter for `{}` are not adjacent", name.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(later_prop_id),
                )
                .with_label("move to be adjacent to its counterpart"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_adjacent_accessors_allowed() {
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
    fn test_non_adjacent_accessors_detected() {
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Example {
    get foo() { return this._foo }
    bar: int32 = 1
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_multiple_fields_between_detected() {
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
            r#"
const obj = {
    get foo() { return this._foo },
    bar: 1,
    set foo(v) { this._foo = v }
}
"#,
        );
        test.result(result).assert_lint("grouped-accessor-pairs");
    }

    #[test]
    fn test_object_literal_adjacent_allowed() {
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(GroupedAccessorPairs);
        let result = test.lint_ast(
            "test.ds",
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
}
