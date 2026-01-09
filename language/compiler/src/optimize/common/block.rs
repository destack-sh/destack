use std::collections::HashMap;

use destack_mir as mir;

/// Check if a terminator uses a specific value.
///
/// Returns true if the value appears in any operand position of the terminator.
/// This includes branch conditions, return values, and block arguments.
pub fn terminator_uses(term: &mir::Terminator, value: mir::Value) -> bool {
    match term {
        mir::Terminator::Return { value: Some(v) } => *v == value,
        mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => false,
        mir::Terminator::Jump { arguments, .. } => arguments.contains(&value),
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            *condition == value
                || then_arguments.contains(&value)
                || else_arguments.contains(&value)
        }
        mir::Terminator::Switch {
            value: v,
            cases,
            default_arguments,
            ..
        } => {
            *v == value
                || cases.iter().any(|c| c.arguments.contains(&value))
                || default_arguments.contains(&value)
        }
        mir::Terminator::Yield {
            value: v,
            resume_arguments,
            ..
        } => *v == value || resume_arguments.contains(&value),
    }
}

/// Get all values used by a terminator.
///
/// Returns a vector of all value operands in the terminator, including
/// conditions, return values, and block arguments.
pub fn terminator_used_values(term: &mir::Terminator) -> Vec<mir::Value> {
    match term {
        mir::Terminator::Return { value: Some(v) } => vec![*v],
        mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => vec![],
        mir::Terminator::Jump { arguments, .. } => arguments.clone(),
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            let mut values = vec![*condition];
            values.extend(then_arguments.iter().copied());
            values.extend(else_arguments.iter().copied());
            values
        }
        mir::Terminator::Switch {
            value,
            cases,
            default_arguments,
            ..
        } => {
            let mut values = vec![*value];
            for case in cases {
                values.extend(case.arguments.iter().copied());
            }
            values.extend(default_arguments.iter().copied());
            values
        }
        mir::Terminator::Yield {
            value,
            resume_arguments,
            ..
        } => {
            let mut values = vec![*value];
            values.extend(resume_arguments.iter().copied());
            values
        }
    }
}

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
