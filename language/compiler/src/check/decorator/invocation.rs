use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

/// Decorator invocation extracted from syntax.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct DecoratorInvocation {
    /// The decorator node.
    pub(in crate::check) decorator: dir::LocalNodeId<dir::Decorator>,
    /// The decorator target expression.
    pub(in crate::check) target: dir::LocalNodeId<dir::Expression>,
    /// The decorator invocation arguments.
    pub(in crate::check) arguments: Vec<dir::LocalNodeId<dir::Argument>>,
}

impl CheckState<'_> {
    /// Return decorator invocations attached to one decorated node.
    pub(in crate::check) fn decorator_invocations(
        &self,
        module: ModuleId,
        decorated: dir::LocalNodeIdAny,
    ) -> Vec<DecoratorInvocation> {
        let view = self.module(module).view();
        let decorators = view.get_decorators_any(decorated);
        let mut invocations = Vec::with_capacity(decorators.len());

        // extract attached decorator invocations in source order
        for decorator in decorators {
            invocations.push(self.decorator_invocation(module, decorator));
        }

        invocations
    }

    /// Extract one decorator invocation.
    pub(in crate::check) fn decorator_invocation(
        &self,
        module: ModuleId,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> DecoratorInvocation {
        let view = self.module(module).view();
        let expression = view.get(decorator).expression;
        let (target, arguments) = match view.get(expression) {
            dir::Expression::Call {
                left, arguments, ..
            } => (*left, arguments.clone()),
            _ => (expression, Vec::new()),
        };

        DecoratorInvocation {
            decorator,
            target,
            arguments,
        }
    }
}
