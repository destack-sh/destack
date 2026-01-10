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

/// Get the arguments passed to a specific successor block from a terminator.
pub fn terminator_arguments_for_successor(
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> &[mir::Value] {
    match terminator {
        mir::Terminator::Jump { target, arguments } if *target == successor => arguments,

        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            // then branch
            if *then_target == successor {
                then_arguments
            }
            // else branch
            else if *else_target == successor {
                else_arguments
            }
            // not a successor
            else {
                &[]
            }
        }

        mir::Terminator::Switch {
            default,
            default_arguments,
            cases,
            ..
        } => {
            // default case
            if *default == successor {
                return default_arguments;
            }

            // numbered cases
            for case in cases {
                if case.target == successor {
                    return &case.arguments;
                }
            }

            &[]
        }

        mir::Terminator::Yield {
            resume,
            resume_arguments,
            ..
        } if *resume == successor => resume_arguments,

        _ => &[],
    }
}

/// Result of collecting arguments for a successor edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessorArguments<'a> {
    /// No edge to the successor was found.
    Missing,
    /// A single consistent argument list for all matching edges.
    Consistent(&'a [mir::Value]),
    /// Multiple edges to the successor disagree on arguments.
    Conflict,
}

/// Get the arguments passed to a successor, reporting conflicts.
pub fn terminator_arguments_for_successor_checked(
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> SuccessorArguments<'_> {
    // track candidate arguments and conflicts
    let mut candidate: Option<&[mir::Value]> = None;
    let mut is_conflict = false;

    /// Record candidate arguments or flag a conflict.
    fn record_arguments<'a>(
        candidate: &mut Option<&'a [mir::Value]>,
        is_conflict: &mut bool,
        args: &'a [mir::Value],
    ) {
        if let Some(existing) = *candidate {
            if existing != args {
                *is_conflict = true;
            }
        } else {
            *candidate = Some(args);
        }
    }

    // scan terminator edges
    match terminator {
        mir::Terminator::Jump { target, arguments } => {
            if *target == successor {
                record_arguments(&mut candidate, &mut is_conflict, arguments);
            }
        }
        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            if *then_target == successor {
                record_arguments(&mut candidate, &mut is_conflict, then_arguments);
            }
            if *else_target == successor {
                record_arguments(&mut candidate, &mut is_conflict, else_arguments);
            }
        }
        mir::Terminator::Switch {
            default,
            default_arguments,
            cases,
            ..
        } => {
            if *default == successor {
                record_arguments(&mut candidate, &mut is_conflict, default_arguments);
            }
            for case in cases {
                if case.target == successor {
                    record_arguments(&mut candidate, &mut is_conflict, &case.arguments);
                }
            }
        }
        mir::Terminator::Yield {
            resume,
            resume_arguments,
            ..
        } => {
            if *resume == successor {
                record_arguments(&mut candidate, &mut is_conflict, resume_arguments);
            }
        }
        _ => {}
    }

    if is_conflict {
        SuccessorArguments::Conflict
    } else if let Some(args) = candidate {
        SuccessorArguments::Consistent(args)
    } else {
        SuccessorArguments::Missing
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
