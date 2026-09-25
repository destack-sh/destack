use tspp_core::StringPool;
use tspp_dir as dir;

/// One static `@if` decorator condition.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum StaticGuard {
    /// The decorator is not an `@if` guard.
    Ordinary,
    /// The guard is syntactically invalid.
    Rejected(StaticGuardError),
    /// The guard has one condition expression.
    Condition(dir::LocalNodeId<dir::Expression>),
}

/// A syntactically invalid static guard.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum StaticGuardError {
    /// The intrinsic is not a direct, non-generic, non-optional call.
    InvalidInvocation { node: dir::LocalNodeIdAny },
    /// The guard has no condition.
    MissingCondition { node: dir::LocalNodeIdAny },
    /// The guard has too many condition arguments.
    MultipleConditions { node: dir::LocalNodeIdAny },
    /// The guard condition is not a positional expression.
    InvalidCondition { node: dir::LocalNodeIdAny },
}

impl StaticGuard {
    /// Classify one decorator as an ordinary decorator or static guard.
    pub(crate) fn classify(
        view: dir::View<'_>,
        strings: &StringPool,
        decorator: dir::LocalNodeId<dir::Decorator>,
    ) -> Self {
        let expression = view.get(decorator).expression;
        let decorator = decorator.into_any();
        let arguments = match view.get(expression) {
            // accept only the exact direct intrinsic call
            dir::Expression::Call {
                position,
                left,
                generic_arguments,
                arguments,
                is_optional,
            } if Self::is_intrinsic(view, strings, *left) => {
                let is_direct = *position == dir::PostfixPosition::Direct;
                if !is_direct || !generic_arguments.is_empty() || *is_optional {
                    return Self::Rejected(StaticGuardError::InvalidInvocation { node: decorator });
                }

                arguments.as_slice()
            }

            // reject generic use of the reserved intrinsic
            dir::Expression::Instantiation { left, .. }
                if Self::is_intrinsic(view, strings, *left) =>
            {
                return Self::Rejected(StaticGuardError::InvalidInvocation { node: decorator });
            }

            // represent the bare intrinsic with no arguments
            _ if Self::is_intrinsic(view, strings, expression) => &[],

            // leave every other decorator to ordinary resolution
            _ => return Self::Ordinary,
        };

        // validate the intrinsic argument shape
        match arguments {
            [] => Self::Rejected(StaticGuardError::MissingCondition { node: decorator }),
            [argument] => match view.get(*argument) {
                dir::Argument::Positional { value } => Self::Condition(*value),
                _ => Self::Rejected(StaticGuardError::InvalidCondition {
                    node: argument.into_any(),
                }),
            },
            _ => Self::Rejected(StaticGuardError::MultipleConditions { node: decorator }),
        }
    }

    /// Return whether one expression is the reserved `if` intrinsic name.
    fn is_intrinsic(
        view: dir::View<'_>,
        strings: &StringPool,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        matches!(
            view.get(expression),
            dir::Expression::Identifier { name } if strings.get(*name) == "if"
        )
    }
}

impl StaticGuardError {
    /// Return the source node that anchors this invalid guard.
    pub(crate) fn node(self) -> dir::LocalNodeIdAny {
        match self {
            Self::InvalidInvocation { node }
            | Self::MissingCondition { node }
            | Self::MultipleConditions { node }
            | Self::InvalidCondition { node } => node,
        }
    }
}
