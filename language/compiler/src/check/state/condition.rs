use smallvec::SmallVec;

use super::VariableId;

/// Predicate that guards check facts and branch refinements.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Predicate {
    /// Predicate always holds.
    Always,
    /// Predicate never holds.
    Never,
    /// Static variable must evaluate to true.
    Static(VariableId),
    /// Runtime value has the target type.
    TypeIs {
        /// The value being tested.
        value: VariableId,
        /// The target type.
        target: VariableId,
    },
    /// Static variables must be equal.
    StaticEquals {
        /// The left static value.
        left: VariableId,
        /// The right static value.
        right: VariableId,
    },
    /// Predicate is negated.
    Not(Box<Predicate>),
    /// All predicates must hold.
    All(SmallVec<[Box<Predicate>; 2]>),
    /// Any predicate may hold.
    Any(SmallVec<[Box<Predicate>; 2]>),
}

impl Predicate {
    /// Return true when this predicate is trivially true.
    pub(in crate::check) fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }

    /// Return variables read by this predicate.
    pub(in crate::check) fn variables(&self, variables: &mut SmallVec<[VariableId; 4]>) {
        match self {
            Self::Always | Self::Never => {}
            Self::Static(variable) => variables.push(*variable),
            Self::TypeIs { value, target }
            | Self::StaticEquals {
                left: value,
                right: target,
            } => {
                variables.push(*value);
                variables.push(*target);
            }
            Self::Not(predicate) => predicate.variables(variables),
            Self::All(predicates) | Self::Any(predicates) => {
                for predicate in predicates {
                    predicate.variables(variables);
                }
            }
        }
    }
}
