use super::super::{DestackFormatContext, Expression, LocalNodeId};
use super::member::{
    member_object_prefers_new_callee_parentheses, should_unwrap_parenthesized_member_object,
    should_unwrap_parenthesized_new_member_callee,
};
use super::type_drop::should_drop_type_binary_left_parentheses;
use super::wrapper::should_drop_parenthesized_expression_wrapper;

/// Parenthesized unwrap policy for expression contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParenthesizedUnwrapPolicy {
    /// Unwrap when the value is used as a member object.
    MemberObject,
    /// Unwrap when the value is a `new` callee wrapper.
    NewMemberCallee,
}

/// Parenthesized drop policy for type contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ParenthesizedDropPolicy {
    /// Drop wrappers in generic expression contexts.
    ExpressionWrapper,
    /// Drop wrappers around type-binary left operands.
    TypeBinaryLeft { node_id: LocalNodeId<Expression> },
}

/// Decide whether a parenthesized expression should unwrap under a policy.
pub(crate) fn parenthesized_should_unwrap(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    policy: ParenthesizedUnwrapPolicy,
) -> bool {
    match policy {
        ParenthesizedUnwrapPolicy::MemberObject => should_unwrap_parenthesized_member_object(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedUnwrapPolicy::NewMemberCallee => {
            should_unwrap_parenthesized_new_member_callee(
                context,
                parenthesized_id,
                inner_expression_id,
            )
        }
    }
}

/// Decide whether a parenthesized expression should drop wrappers under a policy.
pub(crate) fn parenthesized_should_drop(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    policy: ParenthesizedDropPolicy,
) -> bool {
    match policy {
        ParenthesizedDropPolicy::ExpressionWrapper => should_drop_parenthesized_expression_wrapper(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedDropPolicy::TypeBinaryLeft { node_id } => {
            should_drop_type_binary_left_parentheses(
                context,
                node_id,
                parenthesized_id,
                inner_expression_id,
            )
        }
    }
}

/// Return whether `new` callee formatting should keep member-object parentheses.
pub(crate) fn parenthesized_prefers_new_member_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    member_object_prefers_new_callee_parentheses(context, object_id)
}
