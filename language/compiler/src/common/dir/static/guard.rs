use destack_core::StringPool;
use destack_dir as dir;

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
    /// The guard has no condition.
    MissingCondition { node: dir::LocalNodeIdAny },
    /// The guard has too many condition arguments.
    MultipleConditions { node: dir::LocalNodeIdAny },
    /// The guard condition is not a positional expression.
    InvalidCondition { node: dir::LocalNodeIdAny },
}

/// Return the static guard represented by one decorator.
pub(crate) fn static_guard(
    view: dir::View<'_>,
    strings: &StringPool,
    decorator: dir::LocalNodeId<dir::Decorator>,
) -> StaticGuard {
    let expression = view.get(decorator).expression;
    let (callee, arguments) = decorator_call(view, expression);

    if !is_static_if_callee(view, strings, callee) {
        StaticGuard::Ordinary
    } else if arguments.is_empty() {
        StaticGuard::Rejected(StaticGuardError::MissingCondition {
            node: decorator.into_any(),
        })
    } else if arguments.len() != 1 {
        StaticGuard::Rejected(StaticGuardError::MultipleConditions {
            node: decorator.into_any(),
        })
    } else if let dir::Argument::Positional { value } = view.get(arguments[0]) {
        StaticGuard::Condition(*value)
    } else {
        StaticGuard::Rejected(StaticGuardError::InvalidCondition {
            node: arguments[0].into_any(),
        })
    }
}

/// Return the call expression surface for one decorator expression.
fn decorator_call(
    view: dir::View<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> (
    dir::LocalNodeId<dir::Expression>,
    Vec<dir::LocalNodeId<dir::Argument>>,
) {
    match view.get(expression) {
        dir::Expression::Parenthesized { expression } => decorator_call(view, *expression),
        dir::Expression::Call {
            left, arguments, ..
        } => (*left, arguments.clone()),
        _ => (expression, Vec::new()),
    }
}

/// Return whether one decorator callee is the compiler builtin `@if`.
fn is_static_if_callee(
    view: dir::View<'_>,
    strings: &StringPool,
    expression: dir::LocalNodeId<dir::Expression>,
) -> bool {
    match view.get(expression) {
        dir::Expression::Parenthesized { expression } => {
            is_static_if_callee(view, strings, *expression)
        }
        dir::Expression::Identifier { name } => strings.get(*name) == "if",
        dir::Expression::QualifiedReference { path, .. } => {
            let [name] = path.segments.as_slice() else {
                return false;
            };

            strings.get(*name) == "if"
        }
        _ => false,
    }
}
