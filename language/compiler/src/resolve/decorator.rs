use destack_dir::{Annotation, Argument, Expression, LocalNodeId, NodeTree, StringId};

use crate::Compiler;

/// The parsed shape of a decorator expression.
pub(crate) struct DecoratorCall<'a> {
    /// The decorator callee expression.
    pub callee: LocalNodeId<Expression>,
    /// The decorator arguments when the expression is a call.
    pub arguments: Option<&'a [LocalNodeId<Argument>]>,
}

impl Compiler {
    /// Unwrap parenthesized decorator expressions.
    pub(crate) fn unwrap_decorator_expression(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;
        loop {
            let expression = tree.get(current);
            let Expression::Parenthesized { expression } = expression else {
                return current;
            };
            current = *expression;
        }
    }

    /// Resolve the decorator callee and arguments from a decorator expression.
    pub(crate) fn decorator_call<'a>(
        &self,
        tree: &'a NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> DecoratorCall<'a> {
        let expression_id = self.unwrap_decorator_expression(tree, expression_id);
        match tree.get(expression_id) {
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => DecoratorCall {
                callee: self.unwrap_decorator_expression(tree, *left),
                arguments: Some(dynamic_arguments.as_slice()),
            },
            _ => DecoratorCall {
                callee: expression_id,
                arguments: None,
            },
        }
    }

    /// Resolve a decorator call for an annotation.
    pub(crate) fn decorator_call_for_annotation<'a>(
        &self,
        tree: &'a NodeTree,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<DecoratorCall<'a>> {
        let Annotation::Decorator { expression, .. } = tree.get(annotation_id) else {
            return None;
        };
        Some(self.decorator_call(tree, *expression))
    }

    /// Resolve a named decorator call for an annotation.
    pub(crate) fn decorator_call_named<'a>(
        &self,
        tree: &'a NodeTree,
        annotation_id: LocalNodeId<Annotation>,
        name: StringId,
    ) -> Option<DecoratorCall<'a>> {
        let call = self.decorator_call_for_annotation(tree, annotation_id)?;
        let callee = call.callee;
        let path = match tree.get(callee) {
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => path,
            _ => return None,
        };
        if path.segments.len() == 1 && path.segments[0] == name {
            Some(call)
        } else {
            None
        }
    }
}
