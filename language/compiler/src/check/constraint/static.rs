use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, Decision, GenericSubstitution, Origin, StaticTerm, VariableId};

/// One static boolean predicate with its reduction context.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConditionPredicate {
    /// The source that produced this predicate.
    pub(in crate::check) origin: Origin,
    /// The static boolean term.
    pub(in crate::check) term: StaticTerm,
}

/// Static condition under which one checked item exists.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Condition {
    /// The item is always present.
    Always,
    /// The item is never present.
    Never,
    /// The item is present when all condition predicates are true.
    When {
        /// Static boolean predicates that guard this item.
        conditions: SmallVec<[ConditionPredicate; 2]>,
    },
}

impl ConditionPredicate {
    /// Return variables whose changes can decide this predicate.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        self.term.referenced_variables(state)
    }

    /// Substitute generic arguments through this predicate.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let predicate = Self {
            origin: self.origin,
            term: self.term.substitute(module, substitution, state)?,
        };

        Ok(predicate)
    }
}

impl Condition {
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
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Always | Self::Never => SmallVec::new(),
            Self::When { conditions } => conditions
                .iter()
                .flat_map(|condition| condition.referenced_variables(state))
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

impl Default for Condition {
    fn default() -> Self {
        Self::Always
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
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Decision> {
        let condition = self.symbol_availability(symbol);
        let condition = condition.substitute(module, substitution, self)?;

        self.reduce_condition_decision(&condition)
    }

    /// Return the static condition that gates one symbol.
    fn symbol_availability(&self, symbol: dir::GlobalSymbolId) -> Condition {
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
            let Some(term) = self.reduce_static_term(condition.origin, &condition.term)? else {
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
                    term,
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
