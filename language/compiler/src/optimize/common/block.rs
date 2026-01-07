use std::collections::HashMap;

use destack_mir as mir;

/// Substitute values in a terminator according to the given map.
///
/// Creates a new terminator with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn terminator_substitute_uses(
    terminator: &mir::Terminator,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    if substitutions.is_empty() {
        return terminator.clone();
    }

    let substitute = |v: &mir::Value| -> mir::Value { *substitutions.get(v).unwrap_or(v) };

    match terminator {
        mir::Terminator::Return { value } => mir::Terminator::Return {
            value: value.map(|v| substitute(&v)),
        },
        mir::Terminator::Jump { target, arguments } => mir::Terminator::Jump {
            target: *target,
            arguments: arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => mir::Terminator::Branch {
            condition: substitute(condition),
            then_target: *then_target,
            then_arguments: then_arguments.iter().map(&substitute).collect(),
            else_target: *else_target,
            else_arguments: else_arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => mir::Terminator::Switch {
            value: substitute(value),
            default: *default,
            default_arguments: default_arguments.iter().map(&substitute).collect(),
            cases: cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: case.target,
                    arguments: case.arguments.iter().map(&substitute).collect(),
                })
                .collect(),
        },
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => mir::Terminator::Yield {
            value: substitute(value),
            resume: *resume,
            resume_arguments: resume_arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Unreachable => mir::Terminator::Unreachable,
    }
}
