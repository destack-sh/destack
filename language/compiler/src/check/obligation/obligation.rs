use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CompilerResult, DiagnosticAnchor};

use crate::check::{
    Answer, CheckError, CheckState, Condition, PatternTerm, Place, TermId, TypeOperand,
};

/// Selector for one active match case.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// match (value) { Some(item) => item }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum MatchCase {
    /// Default selector.
    ///
    /// Examples:
    /// ```ds
    /// match (value) { _ => value }
    /// ```
    Default,
    /// Pattern selector.
    ///
    /// Examples:
    /// ```ds
    /// match (value) { Some(item) => item }
    /// ```
    PatternTerm {
        /// The pattern checked for this case.
        pattern: TermId<PatternTerm>,
        /// The optional guard type.
        guard: Option<TypeOperand>,
    },
}

/// User-facing check that requires solved terms or whole-expression context.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// const { name } = user
/// sizeOf<T>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Obligation {
    /// Match cases must cover every possible selector value.
    ///
    /// ```ds
    /// match (value) {
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
    /// ```ds
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
    /// ```ds
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
    /// ```ds
    /// value = 2;
    /// ```
    WritablePlace {
        /// The place being written.
        place: Place,
        /// The static condition under which this obligation exists.
        condition: Condition,
    },
    /// A type must resolve to one fixed storage representation.
    ///
    /// ```ds
    /// sizeOf<T>()
    /// ```
    Concrete {
        /// The source expression requiring concrete representation.
        source: dir::GlobalNodeIdAny,
        /// The type that must have concrete representation.
        ty: TypeOperand,
        /// The static condition under which this obligation exists.
        condition: Condition,
    },
}

impl CheckState<'_> {
    /// Push one solved check after reduction.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) {
        self.inference.push_obligation(obligation);
    }

    /// Check solved obligations for diagnostics.
    pub(in crate::check) fn check_obligations(&mut self) -> CompilerResult<Vec<CheckError>> {
        let obligations = self.inference.obligations().cloned().collect::<Vec<_>>();
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
                let condition = self.decide_condition(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, &condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Answer::Ready(true) {
                    return self.check_match_exhaustive(source, value, &cases);
                }
            }
            Obligation::IrrefutablePattern {
                source,
                pattern,
                value,
                condition,
            } => {
                let condition = self.decide_condition(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, &condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Answer::Ready(true) {
                    return self.check_irrefutable_pattern(source, pattern, value);
                }
            }
            Obligation::TryPropagation {
                source,
                value,
                return_type,
                condition,
            } => {
                let condition = self.decide_condition(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, &condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Answer::Ready(true) {
                    return self.check_try_propagates(source, value, return_type);
                }
            }
            Obligation::WritablePlace { place, condition } => {
                let condition = self.decide_condition(&condition)?;
                if let Some(diagnostic) =
                    self.obligation_condition_diagnostic(place.source, &condition)
                {
                    return Ok(Some(diagnostic));
                }
                if condition == Answer::Ready(true) {
                    return self.check_writable_place(place);
                }
            }
            Obligation::Concrete {
                source,
                ty,
                condition,
            } => {
                let condition = self.decide_condition(&condition)?;
                if let Some(diagnostic) = self.obligation_condition_diagnostic(source, &condition) {
                    return Ok(Some(diagnostic));
                }
                if condition == Answer::Ready(true) {
                    return self.check_concrete_type(source, ty);
                }
            }
        }

        Ok(None)
    }

    /// Return a pending condition diagnostic for one obligation when needed.
    fn obligation_condition_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        condition: &Answer<bool>,
    ) -> Option<CheckError> {
        match condition {
            Answer::Ready(_) => None,
            Answer::Pending(_) => {
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

    /// Check that one type has concrete representation.
    fn check_concrete_type(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: TypeOperand,
    ) -> CompilerResult<Option<CheckError>> {
        let concrete = self.type_operand_concrete(source.module_id, ty)?;

        match concrete {
            Some(true) => Ok(None),
            Some(false) => {
                let (module, anchor) = self.source_anchor(source);

                Ok(Some(CheckError::LayoutNotConcrete { anchor, module }))
            }
            None => {
                let (module, anchor) = self.source_anchor(source);

                Ok(Some(CheckError::CannotSolve { anchor, module }))
            }
        }
    }
}
