use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Decision, Origin, StaticOperand, StaticTerm, Substitution};

/// One static boolean predicate with its reduction context.
///
/// Examples:
/// ```ds
/// if (comptime N == 4) { value }
/// ```
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
        substitution: Substitution<'_>,
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
        substitution: Substitution<'_>,
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

impl From<&Condition> for Decision {
    fn from(condition: &Condition) -> Self {
        match condition {
            Condition::Always => Self::Yes,
            Condition::Never => Self::No,
            Condition::When { .. } => Self::Undecidable,
        }
    }
}

impl CheckState<'_> {
    /// Decide whether one guarded symbol is available after generic substitution.
    pub(in crate::check) fn reduce_symbol_availability(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        substitution: Substitution<'_>,
    ) -> CompilerResult<Decision> {
        let condition = self.symbol_availability(symbol);
        let condition = condition.substitute(module, substitution, self)?;

        self.reduce_condition_decision(&condition)
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
        let Condition::When { conditions } = condition else {
            return Ok(condition.clone());
        };
        let mut remaining = SmallVec::new();

        // reduce predicates through solved static values
        for condition in conditions {
            let Some(term) = self.static_operand_term(condition.operand)? else {
                remaining.push(condition.clone());
                continue;
            };

            let Some(term) = self.reduce_static_term(condition.origin, &term)? else {
                remaining.push(condition.clone());

                continue;
            };

            match term {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(true),
                }) => {}
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(false),
                }) => return Ok(Condition::Never),
                term => remaining.push(ConditionPredicate {
                    origin: condition.origin,
                    operand: self.push_term(term).into(),
                }),
            }
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
    pub(in crate::check) fn reduce_condition_decision(
        &mut self,
        condition: &Condition,
    ) -> CompilerResult<Decision> {
        let condition = self.reduce_condition(condition)?;

        Ok(Decision::from(&condition))
    }
}
