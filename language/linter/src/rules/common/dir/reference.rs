use destack_core::StringId;
use destack_dir as dir;

use crate::LintModuleContext;

use super::{assign_pattern_target_expression, expression_unwrap_transparent};

/// The base of a reference path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceBase {
    /// A symbol backed reference.
    Symbol(dir::GlobalSymbolId),
    /// A `this` reference.
    This,
    /// A `super` reference.
    Super,
}

/// A reference path from a base symbol to member names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePath {
    /// The base of the path.
    pub base: ReferenceBase,
    /// The member names from the base expression.
    pub members: Vec<StringId>,
}

/// Resolve the target symbol for a reference expression.
pub fn expression_target_symbol(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    ctx.expression_target_symbol(expression_id)
}

/// Resolve the target symbol for one assignment pattern.
pub fn assign_pattern_target_symbol(
    ctx: &LintModuleContext<'_>,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
) -> Option<dir::GlobalSymbolId> {
    let expression_id = assign_pattern_target_expression(ctx.dir.tree(), assign_pattern_id)?;

    expression_target_symbol(ctx, expression_id)
}

/// Return true when one expression is exactly `new.target`.
pub fn expression_is_new_target(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    new_name: StringId,
    target_name: StringId,
) -> bool {
    // normalize wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct `new.target` path forms first
    if let Some(path) = expression_path_without_generic_arguments(expression) {
        return path.segments.len() >= 2
            && path.segments[0] == new_name
            && path.segments[1] == target_name;
    }

    // then match split member forms like `new.target.extra`
    let dir::Expression::Member { left, name } = expression else {
        return false;
    };
    if *name != Some(target_name) {
        return false;
    }

    let left_id = expression_unwrap_transparent(tree, *left);
    let left_expression = tree.get(left_id);
    let Some(path) = expression_path_without_generic_arguments(left_expression) else {
        return false;
    };

    path.segments.len() == 1 && path.segments[0] == new_name
}

/// Resolve a reference path for member expressions.
pub fn expression_reference_path(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<ReferencePath> {
    // collect member names walking left
    let mut members = Vec::new();
    let base = expression_reference_path_base(ctx, expression_id, &mut members)?;

    // normalize member order
    members.reverse();

    Some(ReferencePath { base, members })
}

/// Resolve a reference path for one assignment pattern.
pub fn assign_pattern_reference_path(
    ctx: &LintModuleContext<'_>,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
) -> Option<ReferencePath> {
    let expression_id = assign_pattern_target_expression(ctx.dir.tree(), assign_pattern_id)?;

    expression_reference_path(ctx, expression_id)
}

/// Return true when two expressions have equivalent source form.
pub fn expressions_have_equivalent_source_form(
    ctx: &LintModuleContext<'_>,
    left_id: dir::LocalNodeId<dir::Expression>,
    right_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    // normalize transparent wrappers before comparisons
    let left_id = expression_unwrap_transparent(ctx.dir.tree(), left_id);
    let right_id = expression_unwrap_transparent(ctx.dir.tree(), right_id);

    // compare canonicalized reference paths first
    let left_path = expression_reference_path(ctx, left_id);
    let right_path = expression_reference_path(ctx, right_id);
    if left_path.is_some() || right_path.is_some() {
        return left_path == right_path;
    }

    // compare source text with spacing removed
    let left_text = ctx.get_span_text(ctx.get_span(left_id));
    let right_text = ctx.get_span_text(ctx.get_span(right_id));
    normalize_expression_source_text(left_text) == normalize_expression_source_text(right_text)
}

/// Return true when one assignment pattern has the same source form as one expression.
pub fn assign_pattern_has_equivalent_source_form(
    ctx: &LintModuleContext<'_>,
    assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let Some(assign_expression_id) =
        assign_pattern_target_expression(ctx.dir.tree(), assign_pattern_id)
    else {
        return false;
    };

    expressions_have_equivalent_source_form(ctx, assign_expression_id, expression_id)
}

/// Normalize expression source text for token style equality checks.
fn normalize_expression_source_text(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Return true when one expression resolves to a symbol.
pub fn expression_is_symbol(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    symbol_id: dir::GlobalSymbolId,
) -> bool {
    expression_target_symbol(ctx, expression_id) == Some(symbol_id)
}

/// Return true when one expression resolves to any symbol in `symbols`.
pub fn expression_is_any_symbol(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    symbols: &[dir::GlobalSymbolId],
) -> bool {
    expression_target_symbol(ctx, expression_id)
        .is_some_and(|symbol_id| symbols.contains(&symbol_id))
}

/// Return one static string literal value from an expression.
pub fn expression_static_string_literal(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<StringId> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct string literals
    if let dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(value)) = expression {
        return Some(*value);
    }

    // match template literals without interpolations
    let dir::Expression::TemplateExpression { value } = expression else {
        return None;
    };
    let dir::TemplateLiteral::String { string } = value else {
        return None;
    };

    Some(*string)
}

/// Return one static regex literal pair as `(pattern, flags)`.
pub fn expression_regex_literal(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(StringId, Option<StringId>)> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match direct regex literals
    let dir::Expression::ScalarLiteral(dir::ScalarLiteral::RegexString { content, flags }) =
        expression
    else {
        return None;
    };

    Some((*content, *flags))
}

/// Return one positional argument value by index.
pub fn positional_argument_value(
    tree: &dir::Tree,
    arguments: &[dir::LocalNodeId<dir::Argument>],
    index: usize,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    let argument_id = *arguments.get(index)?;
    let argument = tree.get(argument_id);
    let dir::Argument::Positional { value, .. } = argument else {
        return None;
    };

    Some(*value)
}

/// Return one static property access pair as `(left, property_name)`.
pub fn expression_static_property_access(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(dir::LocalNodeId<dir::Expression>, StringId)> {
    // normalize transparent wrappers first
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);

    // match dot member access
    if let dir::Expression::Member { left, name } = expression {
        let name = (*name)?;

        return Some((*left, name));
    }

    // match bracket member access with static string keys
    let dir::Expression::Index { left, index, .. } = expression else {
        return None;
    };
    let index_id = index.as_ref().copied()?;
    let property_name = expression_static_string_literal(tree, index_id)?;

    Some((*left, property_name))
}

/// Return one static property name from a member-like or path-like expression.
pub fn expression_static_property_name(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<StringId> {
    // prefer direct property access forms first
    if let Some((_, property_name)) = expression_static_property_access(tree, expression_id) {
        return Some(property_name);
    }

    // then accept terminal path segments on path-like references
    let expression_id = expression_unwrap_transparent(tree, expression_id);
    let expression = tree.get(expression_id);
    expression_path_last_segment(expression)
}

/// Promise rejection callback arity policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromiseCallbackArity {
    /// Require the exact callback arity.
    Exact,
    /// Allow extra trailing arguments after the callback slot.
    Minimum,
}

/// Promise rejection callback info for `catch` and `then`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromiseRejectionCallback {
    /// The receiver expression when the callee exposes one.
    pub receiver_expression_id: Option<dir::LocalNodeId<dir::Expression>>,
    /// The callback argument index.
    pub callback_argument_index: usize,
    /// The callback kind label used in diagnostics and tests.
    pub callback_kind: &'static str,
}

/// Resolve Promise rejection callback info for `catch` and `then`.
pub fn promise_rejection_callback(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    argument_count: usize,
    catch_name: StringId,
    then_name: StringId,
    arity: PromiseCallbackArity,
) -> Option<PromiseRejectionCallback> {
    // keep call targets with one supported method name
    let method_name = expression_static_property_name(tree, expression_id)?;
    let receiver_expression_id =
        expression_static_property_access(tree, expression_id).map(|(receiver_id, _)| receiver_id);

    // catch(handler): first callback argument is rejection handler
    if method_name == catch_name {
        let is_valid_arity = if arity == PromiseCallbackArity::Exact {
            argument_count == 1
        } else {
            argument_count >= 1
        };
        return is_valid_arity.then_some(PromiseRejectionCallback {
            receiver_expression_id,
            callback_argument_index: 0,
            callback_kind: "catch",
        });
    }

    // then(onFulfilled, onRejected): second callback argument is rejection handler
    if method_name == then_name {
        let is_valid_arity = if arity == PromiseCallbackArity::Exact {
            argument_count == 2
        } else {
            argument_count >= 2
        };
        return is_valid_arity.then_some(PromiseRejectionCallback {
            receiver_expression_id,
            callback_argument_index: 1,
            callback_kind: "then rejection",
        });
    }

    None
}
/// Return receiver text for one member expression.
pub fn member_receiver_text(
    ctx: &LintModuleContext<'_>,
    left_expression_id: dir::LocalNodeId<dir::Expression>,
    member_text: &str,
    member_name: StringId,
    is_private: bool,
) -> Option<String> {
    let member_name = ctx.strings.get(member_name);
    let suffix = if is_private {
        format!(".#{member_name}")
    } else {
        format!(".{member_name}")
    };

    // prefer parsing from full member text for best source fidelity
    if let Some(receiver) = member_text.strip_suffix(&suffix) {
        let receiver = receiver.trim();
        if !receiver.is_empty() {
            return Some(receiver.to_string());
        }
    }

    // fall back to left expression span text
    let left_span = ctx.get_span(left_expression_id);
    let left_text = ctx.get_span_text(left_span).trim().to_string();
    if left_text.is_empty() {
        return None;
    }

    Some(left_text)
}

/// Return true when this expression is used as receiver helper target.
pub fn parent_is_receiver_helper(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
    bind_name: StringId,
    call_name: StringId,
    apply_name: StringId,
) -> bool {
    let Some(parent) = tree.get_parent(expression_id.id) else {
        return false;
    };
    if parent.ty != dir::NodeType::Expression {
        return false;
    }

    // inspect the immediate parent expression for helper member access
    let parent_id = parent.into_typed::<dir::Expression>();
    let parent_expression = tree.get(parent_id);
    matches!(
        parent_expression,
        dir::Expression::Member {
            left,
            name,
        }
            | dir::Expression::PrivateMember {
                left,
                name,
            }
            if *left == expression_id
                && (*name == Some(bind_name)
                    || *name == Some(call_name)
                    || *name == Some(apply_name))
    )
}

/// Return true when a call-like invocation safely binds method receivers.
pub fn call_like_invocation_is_receiver_bound(
    tree: &dir::Tree,
    call_like_id: dir::LocalNodeId<dir::Expression>,
    bind_name: StringId,
    call_name: StringId,
    apply_name: StringId,
) -> bool {
    let expression = tree.get(call_like_id);
    let (callee_id, arguments) = match expression {
        dir::Expression::Call {
            left, arguments, ..
        } => (*left, arguments.as_slice()),
        _ => return false,
    };

    // inspect callee helper usage for bind, call, and apply
    let callee = tree.get(callee_id);
    let uses_receiver_helper = matches!(
        callee,
        dir::Expression::Member {
            left: _,
            name,
        }
            | dir::Expression::PrivateMember {
                left: _,
                name,
            }
            if *name == Some(bind_name)
                || *name == Some(call_name)
                || *name == Some(apply_name)
    );

    // non helper callees are safe by construction
    if !uses_receiver_helper {
        return true;
    }

    !arguments.is_empty()
}
/// Get the base of a reference path.
fn expression_reference_path_base(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    members: &mut Vec<StringId>,
) -> Option<ReferenceBase> {
    // inspect the expression node
    let expression = ctx.dir.get(expression_id);

    // match the base or member steps
    match expression {
        dir::Expression::Parenthesized { expression } => {
            expression_reference_path_base(ctx, *expression, members)
        }
        dir::Expression::Member { left, name } => {
            let name = (*name)?;

            members.push(name);
            expression_reference_path_base(ctx, *left, members)
        }
        dir::Expression::This => Some(ReferenceBase::This),
        dir::Expression::Super => Some(ReferenceBase::Super),
        _ => expression_target_symbol(ctx, expression_id).map(ReferenceBase::Symbol),
    }
}

/// Info about a method call expression (receiver.method(...)).
#[derive(Debug, Clone, Copy)]
pub struct MethodCallInfo<'a> {
    /// The call expression id.
    pub call_id: dir::LocalNodeId<dir::Expression>,
    /// The callee member expression id.
    pub callee_id: dir::LocalNodeId<dir::Expression>,
    /// The receiver expression id.
    pub receiver_id: dir::LocalNodeId<dir::Expression>,
    /// The method name.
    pub method_name: StringId,
    /// Generic arguments on the call expression.
    pub generic_arguments: &'a [dir::LocalNodeId<dir::GenericArgument>],
    /// Dynamic arguments on the call expression.
    pub arguments: &'a [dir::LocalNodeId<dir::Argument>],
}

/// Info about one call-like expression (`call(...)` or `new call(...)`).
#[derive(Debug, Clone, Copy)]
pub struct CallLikeExpressionInfo<'a> {
    /// The call target expression.
    pub left: dir::LocalNodeId<dir::Expression>,
    /// Generic arguments.
    pub generic_arguments: &'a [dir::LocalNodeId<dir::GenericArgument>],
    /// Dynamic arguments.
    pub arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// Whether this expression is `new`.
    pub is_new: bool,
}

/// Match one call expression and extract call target and arguments.
pub fn expression_call_like(expression: &dir::Expression) -> Option<CallLikeExpressionInfo<'_>> {
    match expression {
        dir::Expression::Call {
            position: _,
            left,
            generic_arguments,
            arguments,
        } => Some(CallLikeExpressionInfo {
            left: *left,
            generic_arguments: generic_arguments.as_slice(),
            arguments,
            is_new: false,
        }),
        _ => None,
    }
}

/// Match a method call expression and extract its parts.
pub fn expression_method_call(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<MethodCallInfo<'_>> {
    // match call expression
    let expression = tree.get(expression_id);
    let dir::Expression::Call {
        position: _,
        left,
        generic_arguments,
        arguments,
    } = expression
    else {
        return None;
    };

    // match member access for the callee
    let callee_id = *left;
    let callee = tree.get(callee_id);
    let dir::Expression::Member { left, name } = callee else {
        return None;
    };
    let name = (*name)?;

    Some(MethodCallInfo {
        call_id: expression_id,
        callee_id,
        receiver_id: *left,
        method_name: name,
        generic_arguments: generic_arguments.as_slice(),
        arguments,
    })
}

/// Return one path when the expression is a path-like reference without generic arguments.
fn expression_path_without_generic_arguments(expression: &dir::Expression) -> Option<&dir::Path> {
    match expression {
        dir::Expression::QualifiedReference {
            path,
            generic_arguments,
        } if generic_arguments.is_empty() => Some(path),
        _ => None,
    }
}

/// Return the last path segment for one path-like expression.
fn expression_path_last_segment(expression: &dir::Expression) -> Option<StringId> {
    let path = expression_path_without_generic_arguments(expression)?;

    path.last_segment()
}
