use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::CheckState;

/// One authored decorator expression.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct DecoratorExpression {
    /// The decorator node.
    pub(in crate::sema) decorator: dir::LocalNodeId<dir::Decorator>,
    /// The decorator target expression.
    pub(in crate::sema) target: dir::LocalNodeId<dir::Expression>,
    /// The explicit generic arguments.
    pub(in crate::sema) generic_arguments: Vec<dir::LocalNodeId<dir::GenericArgument>>,
    /// The decorator application arguments.
    pub(in crate::sema) arguments: Vec<dir::LocalNodeId<dir::Argument>>,
}

/// One resolved decorator application.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct DecoratorApplication {
    /// The authored decorator expression.
    pub(in crate::sema) expression: DecoratorExpression,
    /// The node receiving the annotation.
    pub(in crate::sema) owner: dir::GlobalNodeIdAny,
    /// The resolved decorator newtype.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
}

/// One selected decorator application.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct SelectedDecorator {
    /// The resolved authored application.
    pub(in crate::sema) application: DecoratorApplication,
    /// The selected decorator resolution.
    pub(in crate::sema) resolution: dir::DecoratorResolution,
}

impl CheckState<'_> {
    /// Return one authored decorator expression.
    fn decorator_expression(
        &self,
        module: ModuleId,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> DecoratorExpression {
        let view = self.module_view(module);
        let expression = view.get(decorator).expression;
        let (target, generic_arguments, arguments) = match view.get(expression) {
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => (*left, generic_arguments.clone(), arguments.clone()),
            _ => (expression, Vec::new(), Vec::new()),
        };

        DecoratorExpression {
            decorator,
            target,
            generic_arguments,
            arguments,
        }
    }

    /// Return decorator applications attached to one decorated node.
    pub(in crate::sema) fn decorator_expressions(
        &self,
        module: ModuleId,
        decorated: dir::LocalNodeIdAny,
    ) -> Vec<DecoratorExpression> {
        let view = self.module_view(module);
        let decorators = view.get_decorators_any(decorated);
        let mut expressions = Vec::with_capacity(decorators.len());

        // extract attached decorator expressions in source order
        for decorator in decorators {
            expressions.push(self.decorator_expression(module, decorator));
        }

        expressions
    }
}
