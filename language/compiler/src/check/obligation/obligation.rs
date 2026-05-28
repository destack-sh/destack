use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CompilerError, CompilerResult, DiagnosticAnchor};

use crate::check::{
    CheckError, CheckState, Decision, PatternTerm, Place, StaticCondition, TermId, TypeOperand,
    VariableId,
};

/// Selector for one active match case.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MatchCase {
    /// Default selector.
    Default,
    /// Pattern selector.
    PatternTerm {
        /// The pattern checked for this case.
        pattern: TermId<PatternTerm>,
        /// The optional guard type.
        guard: Option<VariableId>,
    },
}

/// User-facing check that requires solved terms or whole-expression context.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Obligation {
    /// Match cases must cover every possible selector value.
    ///
    /// ```ts
    /// match value {
    ///     true => 1,
    ///     false => 0,
    /// }
    /// ```
    ExhaustiveMatch {
        /// The match expression.
        source: dir::GlobalNodeIdAny,
        /// The matched value type.
        value: VariableId,
        /// The active match cases in source order.
        cases: Vec<MatchCase>,
        /// The static condition under which this obligation exists.
        condition: StaticCondition,
    },
    /// Binding patterns in non-matching positions must always succeed.
    ///
    /// ```ts
    /// let { name } = user;
    /// ```
    IrrefutablePattern {
        /// The checked pattern source.
        source: dir::GlobalNodeIdAny,
        /// The checked pattern term.
        pattern: TermId<PatternTerm>,
        /// The matched value type.
        value: VariableId,
        /// The static condition under which this obligation exists.
        condition: StaticCondition,
    },
    /// Try propagation must fit the enclosing return type.
    ///
    /// ```ts
    /// value?
    /// ```
    TryPropagation {
        /// The try expression.
        source: dir::GlobalNodeIdAny,
        /// The tried value type.
        value: VariableId,
        /// The enclosing function return type.
        return_type: Option<VariableId>,
        /// The static condition under which this obligation exists.
        condition: StaticCondition,
    },
    /// A place assignment must target writable storage.
    ///
    /// ```ts
    /// value = 2;
    /// ```
    WritablePlace {
        /// The place being written.
        place: Place,
        /// The static condition under which this obligation exists.
        condition: StaticCondition,
    },
    /// A `Dynamic<T>` constraint must support runtime dynamic dispatch.
    DynamicSafe {
        /// The `Dynamic<T>` source expression.
        source: dir::GlobalNodeIdAny,
        /// The constraint that must be dynamically representable.
        constraint: TypeOperand,
        /// The static condition under which this obligation exists.
        condition: StaticCondition,
    },
}

impl CheckState<'_> {
    /// Add one check obligation.
    pub(in crate::check) fn add_obligation(&mut self, obligation: Obligation) {
        self.variables.obligations.push(obligation);
    }
}

impl CheckState<'_> {
    /// Check solved obligations for diagnostics.
    pub(in crate::check) fn check_obligations(&mut self) -> CompilerResult<()> {
        let obligations = self.variables.obligations.clone();

        // check obligations in collection order
        for obligation in obligations {
            self.check_obligation(obligation)?;
        }

        Ok(())
    }

    /// Check one solved obligation for diagnostics.
    fn check_obligation(&mut self, obligation: Obligation) -> CompilerResult<()> {
        match obligation {
            Obligation::ExhaustiveMatch {
                source,
                value,
                cases,
                condition,
            } => {
                if self.check_obligation_condition(source, &condition)? {
                    self.check_match_exhaustive(source, value, &cases)?;
                }
            }
            Obligation::IrrefutablePattern {
                source,
                pattern,
                value,
                condition,
            } => {
                if self.check_obligation_condition(source, &condition)? {
                    self.check_irrefutable_pattern(source, pattern, value)?;
                }
            }
            Obligation::TryPropagation {
                source,
                value,
                return_type,
                condition,
            } => {
                if self.check_obligation_condition(source, &condition)? {
                    self.check_try_propagates(source, value, return_type)?;
                }
            }
            Obligation::WritablePlace { place, condition } => {
                if self.check_obligation_condition(place.source, &condition)? {
                    self.check_writable_place(place)?;
                }
            }
            Obligation::DynamicSafe { .. } => {}
        }

        Ok(())
    }

    /// Report undecidable guards and return whether an obligation is active.
    fn check_obligation_condition(
        &mut self,
        source: dir::GlobalNodeIdAny,
        condition: &StaticCondition,
    ) -> CompilerResult<bool> {
        let is_active = match self.decide_static_condition(condition)? {
            Decision::Yes => true,
            Decision::No => false,
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(source)?;
                let diagnostic = CheckError::CannotSolve { anchor, module };

                self.diagnostics_mut(source.module_id).push(diagnostic);

                false
            }
        };

        Ok(is_active)
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn source_anchor(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = source.module_id;
        let Some(span) = self
            .input(module)
            .parsed
            .tree
            .get_span_by_id(source.local_id.id)
        else {
            return Err(CompilerError::Internal {
                message: format!("check node {} has no source span", source.local_id.id),
            });
        };

        Ok((module, DiagnosticAnchor::from(span)))
    }
}
