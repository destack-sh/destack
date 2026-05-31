use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CompilerResult, DiagnosticAnchor};

use crate::check::{
    CheckError, CheckState, Condition, Decision, PatternTerm, Place, TermId, TypeOperand,
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
        guard: Option<TypeOperand>,
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
        value: TypeOperand,
        /// The active match cases in source order.
        cases: Vec<MatchCase>,
        /// The static condition under which this obligation exists.
        condition: Condition,
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
        value: TypeOperand,
        /// The static condition under which this obligation exists.
        condition: Condition,
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
        value: TypeOperand,
        /// The enclosing function return type.
        return_type: Option<TypeOperand>,
        /// The static condition under which this obligation exists.
        condition: Condition,
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
        condition: Condition,
    },
    /// A `Dynamic<T>` constraint must support runtime dynamic dispatch.
    DynamicSafe {
        /// The `Dynamic<T>` source expression.
        source: dir::GlobalNodeIdAny,
        /// The constraint that must be dynamically representable.
        constraint: TypeOperand,
        /// The static condition under which this obligation exists.
        condition: Condition,
    },
}

impl CheckState<'_> {
    /// Require one solved check after reduction.
    pub(in crate::check) fn require(&mut self, obligation: Obligation) {
        self.inference.obligations.push(obligation);
    }

    /// Check solved obligations for diagnostics.
    pub(in crate::check) fn check_obligations(&mut self) -> CompilerResult<Vec<CheckError>> {
        let obligations = self.inference.obligations.clone();
        let mut diagnostics = Vec::new();

        // check obligations in collection order
        for obligation in obligations {
            diagnostics.extend(self.check_obligation(obligation)?);
        }

        Ok(diagnostics)
    }

    /// Check one solved obligation for diagnostics.
    fn check_obligation(&mut self, obligation: Obligation) -> CompilerResult<Option<CheckError>> {
        match obligation {
            Obligation::ExhaustiveMatch {
                source,
                value,
                cases,
                condition,
            } => {
                let condition = self.reduce_condition_decision(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Decision::Yes {
                    return self.check_match_exhaustive(source, value, &cases);
                }
            }
            Obligation::IrrefutablePattern {
                source,
                pattern,
                value,
                condition,
            } => {
                let condition = self.reduce_condition_decision(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Decision::Yes {
                    return self.check_irrefutable_pattern(source, pattern, value);
                }
            }
            Obligation::TryPropagation {
                source,
                value,
                return_type,
                condition,
            } => {
                let condition = self.reduce_condition_decision(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Decision::Yes {
                    return self.check_try_propagates(source, value, return_type);
                }
            }
            Obligation::WritablePlace { place, condition } => {
                let condition = self.reduce_condition_decision(&condition)?;
                if let Some(diagnostic) =
                    self.obligation_condition_diagnostic(place.source, condition)
                {
                    return Ok(Some(diagnostic));
                }
                if condition == Decision::Yes {
                    return self.check_writable_place(place);
                }
            }
            Obligation::DynamicSafe { .. } => {}
        }

        Ok(None)
    }

    /// Return an undecidable condition diagnostic for one obligation when needed.
    fn obligation_condition_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        condition: Decision,
    ) -> Option<CheckError> {
        match condition {
            Decision::Yes | Decision::No => None,
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(source);
                Some(CheckError::CannotSolve { anchor, module })
            }
        }
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn source_anchor(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> (ModuleId, DiagnosticAnchor) {
        let module = source.module_id;
        let anchor = self.diagnostic_anchor(module, source.local_id);

        (module, anchor)
    }
}
