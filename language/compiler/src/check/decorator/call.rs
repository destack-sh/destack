use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::CheckState;

/// Decorator call surface extracted from syntax.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct DecoratorCall {
    /// The decorator node.
    pub(in crate::check) decorator: dir::LocalNodeId<dir::Decorator>,
    /// The decorator callee expression.
    pub(in crate::check) callee: dir::LocalNodeId<dir::Expression>,
    /// The decorator callee path when syntactically direct.
    pub(in crate::check) path: Option<dir::Path>,
    /// The decorator call arguments.
    pub(in crate::check) arguments: Vec<dir::LocalNodeId<dir::Argument>>,
}

impl CheckState<'_> {
    /// Return decorator calls attached to one owner node.
    pub(in crate::check) fn decorator_calls_for_owner(
        &self,
        module: ModuleId,
        owner: dir::LocalNodeIdAny,
    ) -> Vec<DecoratorCall> {
        let view = self.input(module).view();
        let decorators = view.get_decorators_any(owner);
        let mut calls = Vec::with_capacity(decorators.len());

        // extract attached decorator calls in source order
        for decorator in decorators {
            calls.push(self.decorator_call(module, decorator));
        }

        calls
    }

    /// Extract one decorator call surface.
    pub(in crate::check) fn decorator_call(
        &self,
        module: ModuleId,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> DecoratorCall {
        let view = self.input(module).view();
        let expression = view.get(decorator).expression;
        let (callee, path, arguments) = self.decorator_call_surface(module, expression);

        DecoratorCall {
            decorator,
            callee,
            path,
            arguments,
        }
    }

    /// Return the call-like surface of one decorator expression.
    fn decorator_call_surface(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> (
        dir::LocalNodeId<dir::Expression>,
        Option<dir::Path>,
        Vec<dir::LocalNodeId<dir::Argument>>,
    ) {
        let view = self.input(module).view();

        match view.get(expression) {
            dir::Expression::Parenthesized { expression } => {
                self.decorator_call_surface(module, *expression)
            }
            dir::Expression::Call {
                left, arguments, ..
            } => {
                let path = self.decorator_path(module, *left);

                (*left, path, arguments.clone())
            }
            _ => (
                expression,
                self.decorator_path(module, expression),
                Vec::new(),
            ),
        }
    }

    /// Return the source path of one decorator callee expression.
    fn decorator_path(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::Path> {
        let view = self.input(module).view();

        match view.get(expression) {
            dir::Expression::Parenthesized { expression } => {
                self.decorator_path(module, *expression)
            }
            dir::Expression::Identifier { name } => Some(dir::Path {
                segments: smallvec::smallvec![*name],
            }),
            dir::Expression::QualifiedReference { path, .. } => Some(path.clone()),
            _ => None,
        }
    }
}
