use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CompilerError, CompilerResult, DiagnosticAnchor};

use crate::check::{CheckComponentState, CheckModuleState, PatternTerm, Place, VariableId};

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
        /// The match cases in source order.
        cases: Vec<dir::LocalNodeId<dir::MatchCase>>,
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
        pattern: PatternTerm,
        /// The matched value type.
        value: VariableId,
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
    },
    /// A place assignment must target writable storage.
    ///
    /// ```ts
    /// value = 2;
    /// ```
    WritablePlace {
        /// The place being written.
        place: Place,
    },
}

impl CheckModuleState {
    /// Add one check obligation.
    pub(in crate::check) fn add_obligation(&mut self, obligation: Obligation) {
        self.work.obligations.push(obligation);
    }
}

impl CheckComponentState<'_> {
    /// Check solved obligations for diagnostics.
    pub(in crate::check) fn check_obligations(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // check obligations in stable module order
        for module in modules {
            let obligations = self.module(module)?.work.obligations.clone();

            for obligation in obligations {
                self.check_obligation(obligation)?;
            }
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
            } => self.check_match_exhaustive(source, value, &cases)?,
            Obligation::IrrefutablePattern {
                source,
                pattern,
                value,
            } => self.check_irrefutable_pattern(source, &pattern, value)?,
            Obligation::TryPropagation {
                source,
                value,
                return_type,
            } => self.check_try_propagates(source, value, return_type)?,
            Obligation::WritablePlace { place } => self.check_writable_place(place)?,
        }

        Ok(())
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn source_anchor(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = source.module_id;
        let check_module = self.module(module)?;
        let Some(span) = check_module
            .input
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
