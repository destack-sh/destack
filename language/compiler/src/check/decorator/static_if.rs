use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, DecoratorCall};

/// Static `@if` decorator attached to one owner node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct StaticIfDecorator {
    /// The decorator node.
    pub(in crate::check) decorator: dir::LocalNodeId<dir::Decorator>,
    /// The extracted condition shape.
    pub(in crate::check) condition: StaticIfCondition,
}

/// Static `@if` decorator condition shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum StaticIfCondition {
    /// A single positional condition expression was provided.
    Present(dir::LocalNodeId<dir::Expression>),
    /// The decorator has no condition argument.
    Missing,
    /// The decorator has more than one condition argument.
    Multiple,
    /// The condition argument is not positional.
    Invalid {
        /// The invalid argument node.
        argument: dir::LocalNodeId<dir::Argument>,
    },
}

impl StaticIfDecorator {
    /// Return the node that should anchor condition diagnostics.
    pub(in crate::check) fn condition_anchor(&self) -> dir::LocalNodeIdAny {
        match self.condition {
            StaticIfCondition::Present(condition) => condition.into_any(),
            StaticIfCondition::Missing | StaticIfCondition::Multiple => self.decorator.into_any(),
            StaticIfCondition::Invalid { argument } => argument.into_any(),
        }
    }
}

impl CheckState<'_> {
    /// Return the static `@if` decorator represented by one call.
    pub(in crate::check) fn static_if_decorator_from_call(
        &self,
        module: ModuleId,
        call: &DecoratorCall,
    ) -> Option<StaticIfDecorator> {
        if !self.is_static_if_decorator(module, call) {
            return None;
        }
        let condition = self.static_if_condition(module, &call.arguments);

        Some(StaticIfDecorator {
            decorator: call.decorator,
            condition,
        })
    }

    /// Return whether one decorator is the compiler builtin `@if`.
    fn is_static_if_decorator(&self, module: ModuleId, call: &DecoratorCall) -> bool {
        let Some(path) = &call.path else {
            return false;
        };
        let [name] = path.segments.as_slice() else {
            return false;
        };

        self.input(module).strings.get(*name) == "if"
    }

    /// Return the static condition shape from `@if`.
    fn static_if_condition(
        &self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> StaticIfCondition {
        let [argument] = arguments else {
            if arguments.is_empty() {
                return StaticIfCondition::Missing;
            } else {
                return StaticIfCondition::Multiple;
            }
        };
        let view = self.input(module).view();
        let dir::Argument::Positional { value } = view.get(*argument) else {
            return StaticIfCondition::Invalid {
                argument: *argument,
            };
        };

        StaticIfCondition::Present(*value)
    }
}
