use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

/// Decorator application extracted from syntax.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct DecoratorApplication {
    /// The decorator node.
    pub(in crate::check) decorator: dir::LocalNodeId<dir::Decorator>,
    /// The decorator target expression.
    pub(in crate::check) target: dir::LocalNodeId<dir::Expression>,
    /// The decorator application arguments.
    pub(in crate::check) arguments: Vec<dir::LocalNodeId<dir::Argument>>,
}

impl CheckState<'_> {
    /// Return decorator applications attached to one decorated node.
    pub(in crate::check) fn decorator_applications(
        &self,
        module: ModuleId,
        decorated: dir::LocalNodeIdAny,
    ) -> Vec<DecoratorApplication> {
        let view = self.module_view(module);
        let decorators = view.get_decorators_any(decorated);
        let mut applications = Vec::with_capacity(decorators.len());

        // extract attached decorator applications in source order
        for decorator in decorators {
            applications.push(self.decorator_application(module, decorator));
        }

        applications
    }

    /// Extract one decorator application.
    pub(in crate::check) fn decorator_application(
        &self,
        module: ModuleId,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> DecoratorApplication {
        let view = self.module_view(module);
        let expression = view.get(decorator).expression;
        let (target, arguments) = match view.get(expression) {
            dir::Expression::Call {
                left, arguments, ..
            } => (*left, arguments.clone()),
            _ => (expression, Vec::new()),
        };

        DecoratorApplication {
            decorator,
            target,
            arguments,
        }
    }
}
