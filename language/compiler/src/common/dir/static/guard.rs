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
    let decorator = decorator.into_any();

    static_guard_expression(view, strings, expression, decorator)
}

/// Return the static guard represented by one decorator expression.
fn static_guard_expression(
    view: dir::View<'_>,
    strings: &StringPool,
    expression: dir::LocalNodeId<dir::Expression>,
    decorator: dir::LocalNodeIdAny,
) -> StaticGuard {
    match view.get(expression) {
        // unwrap parenthesized decorators
        dir::Expression::Parenthesized { expression } => {
            static_guard_expression(view, strings, *expression, decorator)
        }
        // read decorator calls
        dir::Expression::Call {
            left, arguments, ..
        } => static_guard_call(view, strings, *left, arguments, decorator),

        // treat bare decorators as calls without arguments
        _ => static_guard_call(view, strings, expression, &[], decorator),
    }
}

/// Return the static guard represented by one decorator call.
fn static_guard_call(
    view: dir::View<'_>,
    strings: &StringPool,
    callee: dir::LocalNodeId<dir::Expression>,
    arguments: &[dir::LocalNodeId<dir::Argument>],
    decorator: dir::LocalNodeIdAny,
) -> StaticGuard {
    // ignore user decorators
    if !is_static_if_callee(view, strings, callee) {
        StaticGuard::Ordinary
    }
    // reject @if
    else if arguments.is_empty() {
        StaticGuard::Rejected(StaticGuardError::MissingCondition { node: decorator })
    }
    // reject @if(a, b)
    else if arguments.len() != 1 {
        StaticGuard::Rejected(StaticGuardError::MultipleConditions { node: decorator })
    }
    // accept @if(condition)
    else if let dir::Argument::Positional { value } = view.get(arguments[0]) {
        StaticGuard::Condition(*value)
    }
    // reject @if(name: condition)
    else {
        StaticGuard::Rejected(StaticGuardError::InvalidCondition {
            node: arguments[0].into_any(),
        })
    }
}

/// Return whether one decorator callee is the compiler builtin `@if`.
fn is_static_if_callee(
    view: dir::View<'_>,
    strings: &StringPool,
    expression: dir::LocalNodeId<dir::Expression>,
) -> bool {
    match view.get(expression) {
        // unwrap parenthesized callees
        dir::Expression::Parenthesized { expression } => {
            is_static_if_callee(view, strings, *expression)
        }

        // match bare @if
        dir::Expression::Identifier { name } => strings.get(*name) == "if",

        // match resolved @if syntax
        dir::Expression::QualifiedReference { path, .. } => {
            let [name] = path.segments.as_slice() else {
                return false;
            };

            strings.get(*name) == "if"
        }
        // anything else is an ordinary decorator
        _ => false,
    }
}
