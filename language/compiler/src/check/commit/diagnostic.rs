use destack_artifact::ToDiagnostic;
use destack_source::{DiagnosticCollection, ModuleId};

use crate::check::{
    CheckError, CheckState, Constraint, Decision, Origin, StaticRelation, TypeRelation,
};
use crate::{CompilerResult, DiagnosticAnchor};

impl CheckState<'_> {
    /// Collect final diagnostics for one checked component.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut diagnostics = Vec::new();

        self.collect_walk_diagnostics(&mut diagnostics);
        self.collect_constraint_diagnostics(&mut diagnostics)?;
        diagnostics.extend(self.check_obligations()?);

        let mut collection = DiagnosticCollection::new();

        // render diagnostics in production order
        for diagnostic in diagnostics {
            collection.insert(diagnostic.to_diagnostic(self.context)?);
        }

        Ok(collection)
    }

    /// Collect diagnostics reported during walk.
    fn collect_walk_diagnostics(&mut self, diagnostics: &mut Vec<CheckError>) {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // drain walk diagnostics in stable module order
        for module in modules {
            diagnostics.append(&mut self.module_mut(module).diagnostics);
        }
    }

    /// Collect diagnostics for final constraint decisions.
    fn collect_constraint_diagnostics(
        &mut self,
        diagnostics: &mut Vec<CheckError>,
    ) -> CompilerResult<()> {
        let constraints = self.inference.constraints_vec();

        // render rejected constraints
        for constraint in constraints {
            self.collect_constraint_diagnostic(&constraint, diagnostics)?;
        }

        Ok(())
    }

    /// Collect one constraint diagnostic.
    fn collect_constraint_diagnostic(
        &mut self,
        constraint: &Constraint,
        diagnostics: &mut Vec<CheckError>,
    ) -> CompilerResult<()> {
        let condition = constraint.condition();
        match self.reduce_condition_decision(&condition)? {
            Decision::No => return Ok(()),
            Decision::Undecidable => return Ok(()),
            Decision::Yes => {}
        }

        match constraint {
            Constraint::Type {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => {
                let left_term = self.reduce_type_operand(*origin, *left)?;
                let right_term = self.reduce_type_operand(*origin, *right)?;
                let decision = match (&left_term, &right_term) {
                    (Some(left), Some(right)) => {
                        self.decide_type_term_relation(*relation, left, right)?
                    }
                    _ => self.decide_type_relation(*relation, *left, *right)?,
                };

                if decision != Decision::Yes {
                    let diagnostic = self.type_relation_diagnostic(*relation, *origin)?;

                    diagnostics.push(diagnostic);
                }
            }
            Constraint::Pattern {
                relation,
                value,
                origin,
                condition: _,
            } => {
                let decision = self.reduce_pattern_relation(*origin, *value, relation)?;
                if decision != Decision::Yes {
                    let diagnostic =
                        self.type_relation_diagnostic(TypeRelation::Satisfies, *origin)?;

                    diagnostics.push(diagnostic);
                }
            }
            Constraint::Static {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => {
                let decision = self.decide_static_relation(*relation, *left, *right)?;
                if decision != Decision::Yes {
                    let diagnostic = self.static_relation_diagnostic(*relation, *origin)?;

                    diagnostics.push(diagnostic);
                }
            }
        }

        Ok(())
    }

    /// Return a type relation diagnostic.
    fn type_relation_diagnostic(
        &self,
        relation: TypeRelation,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);
        let diagnostic = match relation {
            TypeRelation::Equal => CheckError::CannotSolve { anchor, module },
            TypeRelation::Assignable => CheckError::NotAssignable { anchor, module },
            TypeRelation::Castable => CheckError::InvalidCast { anchor, module },
            TypeRelation::Satisfies => CheckError::ConstraintNotSatisfied { anchor, module },
            TypeRelation::Extends => CheckError::DoesNotExtend { anchor, module },
            TypeRelation::Implements => CheckError::DoesNotImplement { anchor, module },
        };

        Ok(diagnostic)
    }

    /// Return a static relation diagnostic.
    fn static_relation_diagnostic(
        &self,
        relation: StaticRelation,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);
        let diagnostic = match relation {
            StaticRelation::Equal | StaticRelation::Assignable => {
                CheckError::CannotSolve { anchor, module }
            }
        };

        Ok(diagnostic)
    }

    /// Return the diagnostic anchor for a constraint origin.
    fn anchor(&self, origin: Origin) -> (ModuleId, DiagnosticAnchor) {
        self.diagnostic_anchor_for_origin(origin)
    }
}
