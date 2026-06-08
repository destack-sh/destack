use destack_artifact::ToDiagnostic;
use destack_dir as dir;
use destack_source::{DiagnosticCollection, ModuleId};

use crate::check::{
    CallDecision, CallFailure, CheckError, CheckState, Constraint, ConstructDecision,
    ConstructFailure, Decision, MemberDecision, MemberFailure, OperatorDecision,
    OperatorFailureReason, OperatorTermKind, Origin, StaticRelation, TypeRelation,
};
use crate::{CompilerResult, DiagnosticAnchor};

impl CheckState<'_> {
    /// Collect final diagnostics for one checked component.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut diagnostics = Vec::new();

        self.collect_walk_diagnostics(&mut diagnostics);
        self.collect_selection_diagnostics(&mut diagnostics);
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

    /// Collect diagnostics for rejected selections.
    fn collect_selection_diagnostics(&self, diagnostics: &mut Vec<CheckError>) {
        self.collect_member_diagnostics(diagnostics);
        self.collect_call_diagnostics(diagnostics);
        self.collect_construct_diagnostics(diagnostics);
        self.collect_operator_diagnostics(diagnostics);
    }

    /// Collect rejected member diagnostics.
    fn collect_member_diagnostics(&self, diagnostics: &mut Vec<CheckError>) {
        for (source, decision) in self.inference.members() {
            let MemberDecision::Rejected(failure) = decision else {
                continue;
            };
            let MemberFailure::Missing { key } = failure;
            let module = source.module_id;
            let anchor = self.diagnostic_anchor(module, source.local_id);
            let key = self.member_key_label(module, key);
            let diagnostic = CheckError::MissingMember {
                anchor,
                module,
                key,
            };

            diagnostics.push(diagnostic);
        }
    }

    /// Collect rejected call diagnostics.
    fn collect_call_diagnostics(&self, diagnostics: &mut Vec<CheckError>) {
        for (source, decision) in self.inference.calls() {
            let CallDecision::Rejected(failure) = decision else {
                continue;
            };
            let module = source.module_id;
            let anchor = self.diagnostic_anchor(module, source.local_id);
            let diagnostic = match failure {
                CallFailure::NotCallable => CheckError::NotCallable { anchor, module },
                CallFailure::NoMatch | CallFailure::ArgumentType { .. } => {
                    CheckError::NoMatchingCall { anchor, module }
                }
            };

            diagnostics.push(diagnostic);
        }
    }

    /// Collect rejected construct diagnostics.
    fn collect_construct_diagnostics(&self, diagnostics: &mut Vec<CheckError>) {
        for (source, decision) in self.inference.constructs() {
            let ConstructDecision::Rejected(failure) = decision else {
                continue;
            };
            let module = source.module_id;
            let anchor = self.diagnostic_anchor(module, source.local_id);
            let diagnostic = match failure {
                ConstructFailure::NotConstructible => CheckError::NotCallable { anchor, module },
                ConstructFailure::NoMatch => CheckError::NoMatchingCall { anchor, module },
            };

            diagnostics.push(diagnostic);
        }
    }

    /// Collect rejected operator diagnostics.
    fn collect_operator_diagnostics(&self, diagnostics: &mut Vec<CheckError>) {
        for (source, decision) in self.inference.operators() {
            let OperatorDecision::Rejected(failure) = decision else {
                continue;
            };
            let module = source.module_id;
            let anchor = self.diagnostic_anchor(module, source.local_id);
            let diagnostic = match failure.reason {
                OperatorFailureReason::NoMatch => CheckError::NoMatchingOperator {
                    anchor,
                    module,
                    operator: self.operator_label(failure.kind).to_owned(),
                },
                OperatorFailureReason::InvalidStrictEquality => {
                    CheckError::InvalidStrictEquality { anchor, module }
                }
            };

            diagnostics.push(diagnostic);
        }
    }

    /// Return a diagnostic label for one member key.
    fn member_key_label(&self, module: ModuleId, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.module(module).strings.get(name).to_owned(),
            dir::StaticKey::Index(index) => index.to_string(),
            dir::StaticKey::Symbol(symbol) => {
                symbol.debug_string(self.compiler.repository.string_pool())
            }
        }
    }

    /// Return a diagnostic label for one operator.
    fn operator_label(&self, kind: OperatorTermKind) -> &'static str {
        match kind {
            OperatorTermKind::Unary(operator) => operator.text(),
            OperatorTermKind::Binary(operator) => operator.text(),
        }
    }

    /// Collect diagnostics for final constraint decisions.
    fn collect_constraint_diagnostics(
        &mut self,
        diagnostics: &mut Vec<CheckError>,
    ) -> CompilerResult<()> {
        let constraints = self.inference.constraints().cloned().collect::<Vec<_>>();

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
                let reduced_left = self.reduce_type_operand(*origin, *left)?;
                let reduced_right = self.reduce_type_operand(*origin, *right)?;
                let decision = match (reduced_left, reduced_right) {
                    (Some(left), Some(right)) => {
                        self.decide_type_relation(*relation, left, right)?
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
                let decision = self.decide_pattern_relation(*origin, *value, relation)?;
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
        self.origin_diagnostic_anchor(origin)
    }
}
