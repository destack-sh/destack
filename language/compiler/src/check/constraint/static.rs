use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Decision, GenericSubstitution, StaticTerm, VariableId};

/// One static boolean predicate with its reduction context.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct StaticPredicate {
    /// The module whose checker state reduces this predicate.
    pub(in crate::check) module: ModuleId,
    /// The static boolean term.
    pub(in crate::check) term: StaticTerm,
}

/// Static condition under which one checked item exists.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticCondition {
    /// The item is always present.
    Always,
    /// The item is never present.
    Never,
    /// The item is present when all condition predicates are true.
    When {
        /// Static boolean predicates that guard this item.
        conditions: SmallVec<[StaticPredicate; 2]>,
    },
}

impl StaticPredicate {
    /// Return variables whose changes can decide this predicate.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        self.term.referenced_variables()
    }

    /// Substitute generic arguments through this predicate.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let predicate = Self {
            module,
            term: self.term.substitute(module, substitution, state)?,
        };

        Ok(predicate)
    }
}

impl StaticCondition {
    /// Return whether this condition excludes the item.
    pub(in crate::check) fn is_never(&self) -> bool {
        matches!(self, Self::Never)
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

    /// Return variables whose changes can decide this condition.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Always | Self::Never => SmallVec::new(),
            Self::When { conditions } => conditions
                .iter()
                .flat_map(StaticPredicate::referenced_variables)
                .collect(),
        }
    }

    /// Substitute generic arguments through this condition.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
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

impl Default for StaticCondition {
    fn default() -> Self {
        Self::Always
    }
}

impl CheckState<'_> {
    /// Decide whether one guarded symbol is available after generic substitution.
    pub(in crate::check) fn decide_symbol_availability(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Decision> {
        let condition = self.symbol_availability(symbol);
        let condition = condition.substitute(module, substitution, self)?;

        self.decide_static_condition(&condition)
    }

    /// Return the static condition that gates one symbol.
    fn symbol_availability(&self, symbol: dir::GlobalSymbolId) -> StaticCondition {
        let Some(availability) = self.availability.get(&symbol.module_id) else {
            return StaticCondition::Always;
        };
        let condition = availability.get(&symbol);

        condition.cloned().unwrap_or(StaticCondition::Always)
    }

    /// Decide whether one static condition is active.
    pub(in crate::check) fn decide_static_condition(
        &mut self,
        condition: &StaticCondition,
    ) -> CompilerResult<Decision> {
        match condition {
            StaticCondition::Always => Ok(Decision::Yes),
            StaticCondition::Never => Ok(Decision::No),
            StaticCondition::When { conditions } => self.decide_static_conditions(conditions),
        }
    }

    /// Decide whether all static predicate terms are true.
    fn decide_static_conditions(
        &mut self,
        conditions: &[StaticPredicate],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // reduce predicates through solved static values
        for condition in conditions {
            let Some(term) = self.reduce_static_term(condition.module, &condition.term)? else {
                decision = Decision::Undecidable;

                continue;
            };

            match term {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(true),
                }) => {}
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(false),
                }) => return Ok(Decision::No),
                _ => decision = Decision::Undecidable,
            }
        }

        Ok(decision)
    }
}
