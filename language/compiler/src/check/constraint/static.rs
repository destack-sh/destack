use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, StaticOperand, SubstitutionSet};

/// One static boolean predicate with its reduction context.
///
/// Examples:
/// ```ds
/// if (comptime N == 4) { value }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ConditionPredicate {
    /// The source that produced this predicate.
    pub(in crate::check) origin: Origin,
    /// The static boolean operand.
    pub(in crate::check) operand: StaticOperand,
}

/// Static condition under which one checked item exists.
///
/// Examples:
/// ```ds
/// if (comptime N == 4) { value }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::check) enum Condition {
    /// The item is always present.
    ///
    /// Examples:
    /// ```ds
    /// const value = 1
    /// ```
    Always,
    /// The item is never present.
    ///
    /// Examples:
    /// ```ds
    /// @if(false)
    /// const value = 1
    /// ```
    Never,
    /// The item is present when all condition predicates are true.
    ///
    /// Examples:
    /// ```ds
    /// @if(T >= 4)
    /// const value = 1
    /// ```
    When {
        /// Static boolean predicates that guard this item.
        conditions: SmallVec<[ConditionPredicate; 2]>,
    },
}

impl ConditionPredicate {
    /// Substitute generic arguments through this predicate.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let predicate = Self {
            origin: self.origin,
            operand: state.substitute_static_operand(module, substitution, self.operand)?,
        };

        Ok(predicate)
    }
}

impl Condition {
    /// Return whether this condition excludes the item.
    pub(in crate::check) fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }

    /// Return whether this condition is guaranteed by one active guard.
    pub(in crate::check) fn is_guaranteed_by(&self, guard: &Self) -> bool {
        match (self, guard) {
            (Self::Always, _) | (_, Self::Never) => true,
            (Self::Never, Self::Always | Self::When { .. }) => false,
            (Self::When { .. }, Self::Always) => false,
            (
                Self::When {
                    conditions: required,
                },
                Self::When { conditions: active },
            ) => required.iter().all(|condition| active.contains(condition)),
        }
    }

    /// Combine two static conditions with logical conjunction.
    pub(in crate::check) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::Always, condition) | (condition, Self::Always) => condition,
            (
                Self::When {
                    conditions: mut left,
                },
                Self::When { conditions: right },
            ) => {
                left.extend(right);

                Self::When { conditions: left }
            }
        }
    }

    /// Substitute generic arguments through this condition.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let condition = match self {
            Self::Always => Self::Always,
            Self::Never => Self::Never,
            Self::When { conditions } => Self::When {
                conditions: conditions
                    .iter()
                    .map(|condition| condition.substitute(module, substitution, state))
                    .collect::<CompilerResult<_>>()?,
            },
        };

        Ok(condition)
    }
}

impl CheckState<'_> {
    /// Decide whether one guarded symbol is available after generic substitution.
    pub(in crate::check) fn decide_symbol_availability(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        substitution: &SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        let condition = self.symbol_availability(symbol);
        let condition = condition.substitute(module, substitution, self)?;

        self.decide_condition(&condition)
    }

    /// Return the static condition that gates one symbol.
    pub(in crate::check) fn symbol_availability(&self, symbol: dir::GlobalSymbolId) -> Condition {
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Condition::Always;
        };
        let condition = module.availability.get(&symbol);

        condition.cloned().unwrap_or(Condition::Always)
    }

    /// Reduce one static condition through solved predicate terms.
    pub(in crate::check) fn reduce_condition(
        &mut self,
        condition: &Condition,
    ) -> CompilerResult<Condition> {
        let predicates = match condition {
            Condition::Always => return Ok(Condition::Always),
            Condition::Never => return Ok(Condition::Never),
            Condition::When { conditions } => conditions,
        };

        self.reduce_condition_predicates(predicates)
    }

    /// Reduce one static predicate list through solved predicate terms.
    pub(in crate::check) fn reduce_condition_predicates(
        &mut self,
        predicates: &[ConditionPredicate],
    ) -> CompilerResult<Condition> {
        let mut remaining = SmallVec::new();

        // reduce predicates through solved static values
        for condition in predicates {
            let Answer::Ready(operand) =
                self.reduce_static_operand(condition.origin, condition.operand)?
            else {
                remaining.push(*condition);

                continue;
            };

            if self.static_operand_boolean(operand) == Some(true) {
                continue;
            }

            // discard the guarded item when a predicate is false
            if self.static_operand_boolean(operand) == Some(false) {
                return Ok(Condition::Never);
            }

            // keep predicates that reduced but did not close
            remaining.push(ConditionPredicate {
                origin: condition.origin,
                operand,
            });
        }

        let condition = if remaining.is_empty() {
            Condition::Always
        } else {
            Condition::When {
                conditions: remaining,
            }
        };

        Ok(condition)
    }

    /// Reduce one static condition to its current decision.
    pub(in crate::check) fn decide_condition(
        &mut self,
        condition: &Condition,
    ) -> CompilerResult<Answer<bool>> {
        let condition = self.reduce_condition(condition)?;

        Ok(self.decide_reduced_condition(&condition))
    }

    /// Reduce one static predicate list to its current decision.
    pub(in crate::check) fn decide_condition_predicates(
        &mut self,
        predicates: &[ConditionPredicate],
    ) -> CompilerResult<Answer<bool>> {
        let condition = self.reduce_condition_predicates(predicates)?;

        Ok(self.decide_reduced_condition(&condition))
    }

    /// Return one already reduced condition's current decision.
    fn decide_reduced_condition(&self, condition: &Condition) -> Answer<bool> {
        match condition {
            Condition::Always => Answer::Ready(true),
            Condition::Never => Answer::Ready(false),
            Condition::When { conditions } => Answer::pending(
                conditions
                    .iter()
                    .flat_map(|condition| condition.operand.dependencies(self)),
            ),
        }
    }
}
