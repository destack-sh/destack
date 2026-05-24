use destack_artifact::ToDiagnostic;
use destack_dir as dir;
use destack_source::{DiagnosticCollection, ModuleId};
use indexmap::IndexSet;

use crate::check::solve::Decision;
use crate::check::{
    CallFailure, CallOutcome, CheckComponentState, CheckError, Constraint, ConstraintOrigin,
    ConstructFailure, ConstructOutcome, OperatorFailureReason, OperatorOutcome, OperatorTerm,
    OperatorTermKind, Place, PlaceTarget, ShapeMemberTerm, StaticRelation, StaticTerm,
    TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult, DiagnosticAnchor};

/// Source-only type markers that must not reach committed DIR types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeTermMarker {
    /// The `intrinsic` marker type.
    Intrinsic,
    /// The `const` assertion marker type.
    Const,
}

impl TypeTermMarker {
    /// Return whether this marker matches one type term.
    fn matches(self, term: &TypeTerm) -> bool {
        match self {
            Self::Intrinsic => matches!(term, TypeTerm::Intrinsic),
            Self::Const => matches!(term, TypeTerm::ConstAssertion),
        }
    }
}

impl CheckComponentState<'_> {
    /// Commit solved component diagnostics.
    pub(in crate::check) fn commit_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut reported = IndexSet::new();
        let constraints = self.constraints();

        // replay every solved constraint
        for constraint in &constraints {
            self.check_constraint(constraint, &mut reported)?;
        }

        self.check_obligations()?;

        let mut collection = DiagnosticCollection::new();

        let modules = self.component_modules.clone();

        // collect module diagnostics in stable order
        for module in modules {
            let diagnostics = std::mem::take(&mut self.module_mut(module)?.work.diagnostics);

            for diagnostic in diagnostics {
                collection.insert(diagnostic.to_diagnostic(self.context)?);
            }
        }

        Ok(collection)
    }

    /// Check one solved constraint for diagnostics.
    fn check_constraint(
        &mut self,
        constraint: &Constraint,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let has_unresolved_input = self.has_unresolved_input(constraint)?;

        if has_unresolved_input {
            return Ok(());
        }

        match constraint {
            Constraint::DefineType {
                result,
                term,
                origin,
            } => {
                self.check_type_definition(*result, term, *origin, reported)?;
            }
            Constraint::DefineStatic {
                result,
                term,
                origin,
            } => {
                self.check_static_definition(*result, term, *origin, reported)?;
            }
            Constraint::RelateType {
                relation,
                left,
                right,
                origin,
            } => {
                self.check_type_relation(*relation, *left, *right, *origin)?;
            }
            Constraint::RelateStatic {
                relation,
                left,
                right,
                origin,
            } => {
                self.check_static_relation(*relation, *left, *right, *origin)?;
            }
            Constraint::RequirePlaceWrite { place, origin } => {
                self.check_place_write(*place, *origin)?;
            }
        }

        Ok(())
    }

    /// Return whether any constraint input is unresolved.
    fn has_unresolved_input(&self, constraint: &Constraint) -> CompilerResult<bool> {
        let variables = match constraint {
            Constraint::DefineType {
                result: _,
                term,
                origin: _,
            } => term.referenced_variables(),
            Constraint::DefineStatic {
                result: _,
                term,
                origin: _,
            } => term.referenced_variables(),
            Constraint::RelateType {
                relation: _,
                left,
                right,
                origin: _,
            }
            | Constraint::RelateStatic {
                relation: _,
                left,
                right,
                origin: _,
            } => smallvec::smallvec![*left, *right],
            Constraint::RequirePlaceWrite { place, origin: _ } => place.referenced_variables(),
        };

        // wait for real inputs, not for the result that may itself be failed
        for variable in variables {
            if self.variable_solution(variable)?.is_none() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check one type definition for diagnostics.
    fn check_type_definition(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
        origin: ConstraintOrigin,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        if self.type_term_contains_marker(term, TypeTermMarker::Intrinsic)? {
            let diagnostic = self.invalid_intrinsic_type_diagnostic(origin, result.module)?;
            let module = self.diagnostic_module(origin, result.module);

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        }
        if self.type_term_contains_marker(term, TypeTermMarker::Const)? {
            let diagnostic = self.invalid_const_type_diagnostic(origin, result.module)?;
            let module = self.diagnostic_module(origin, result.module);

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        }

        if self.solved_type_term(result)?.is_none() {
            if reported.insert(result) {
                self.report_type_term_failure(term, origin, result.module)?;
            }

            return Ok(());
        }

        Ok(())
    }

    /// Return whether one term depends on a source marker.
    fn type_term_contains_marker(
        &self,
        term: &TypeTerm,
        marker: TypeTermMarker,
    ) -> CompilerResult<bool> {
        let mut seen = IndexSet::new();

        self.type_term_contains_marker_inner(term, marker, &mut seen)
    }

    /// Return whether one term depends on a source marker.
    fn type_term_contains_marker_inner(
        &self,
        term: &TypeTerm,
        marker: TypeTermMarker,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<bool> {
        if marker.matches(term) {
            return Ok(true);
        }

        // follow solved type variables
        for variable in term.referenced_variables() {
            if !seen.insert(variable) {
                continue;
            }
            let Some(term) = self.solved_type_term(variable)? else {
                continue;
            };
            if self.type_term_contains_marker_inner(&term, marker, seen)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check one static definition for diagnostics.
    fn check_static_definition(
        &mut self,
        result: VariableId,
        term: &StaticTerm,
        origin: ConstraintOrigin,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let static_term = self.solved_static_term(result)?;
        if static_term.is_none() {
            if reported.insert(result) {
                let diagnostic = self.static_diagnostic(origin, result.module)?;

                self.module_mut(result.module)?
                    .work
                    .diagnostics
                    .push(diagnostic);
            }

            return Ok(());
        }

        match (static_term, term) {
            (Some(StaticTerm::Literal(_)), StaticTerm::Expression(_)) => return Ok(()),
            (Some(value), expected) => {
                let decision =
                    self.decide_static_term_relation(StaticRelation::Equal, &value, expected)?;
                if decision != Decision::No {
                    return Ok(());
                }
            }
            _ => {}
        }

        let diagnostic = self.static_diagnostic(origin, result.module)?;
        self.module_mut(result.module)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Check one solved type relation for diagnostics.
    fn check_type_relation(
        &mut self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let decision = self.decide_type_relation(relation, left, right)?;

        match decision {
            Decision::Yes => {}
            Decision::No | Decision::Undecidable => {
                let diagnostic = self.type_relation_diagnostic(relation, origin, left.module)?;

                self.module_mut(left.module)?
                    .work
                    .diagnostics
                    .push(diagnostic);
            }
        }

        Ok(())
    }

    /// Check one solved static relation for diagnostics.
    fn check_static_relation(
        &mut self,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let decision = self.decide_static_relation(relation, left, right)?;

        match decision {
            Decision::Yes => {}
            Decision::No | Decision::Undecidable => {
                let diagnostic = self.static_diagnostic(origin, left.module)?;

                self.module_mut(left.module)?
                    .work
                    .diagnostics
                    .push(diagnostic);
            }
        }

        Ok(())
    }

    /// Check one writable place requirement for diagnostics.
    fn check_place_write(&mut self, place: Place, origin: ConstraintOrigin) -> CompilerResult<()> {
        if self.is_writable_place(place)? {
            return Ok(());
        }

        let diagnostic = self.not_writable_diagnostic(origin, place.source.module_id)?;

        self.module_mut(place.source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Return whether one place is writable.
    fn is_writable_place(&self, place: Place) -> CompilerResult<bool> {
        let is_writable = match place.target {
            PlaceTarget::Binding { symbol } => self.is_writable_binding(symbol)?,
            PlaceTarget::Member { owner, key } => self.is_writable_member(owner, key)?,
            PlaceTarget::Index { .. } => true,
            PlaceTarget::Dereference { .. } => true,
            PlaceTarget::Pattern { pattern: _ } => true,
        };

        Ok(is_writable)
    }

    /// Return whether one local binding can be assigned.
    fn is_writable_binding(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let check_module = self.module(symbol.module_id)?;
        let bindings = check_module.binding_table();
        let local_symbol = bindings.get_symbol(symbol.local_id);
        let is_writable = local_symbol.binding_mutability.is_some_and(|mutability| {
            matches!(
                mutability,
                dir::Mutability::Mutable | dir::Mutability::Exclusive
            )
        }) && check_module
            .input
            .resolved
            .imports
            .symbol_target(symbol.local_id)
            .is_none();

        Ok(is_writable)
    }

    /// Return whether one member target can be assigned.
    fn is_writable_member(&self, owner: VariableId, key: dir::StaticKey) -> CompilerResult<bool> {
        let Some(owner) = self.solved_type_term(owner)? else {
            return Err(CompilerError::Internal {
                message: format!("place owner {owner:?} was not solved"),
            });
        };

        // structural fields carry their write access directly
        if let TypeTerm::Shape { members } = owner {
            for member in members {
                if let ShapeMemberTerm::Field {
                    key: member_key,
                    is_readonly,
                    ..
                } = member
                    && member_key.matches(&key)
                {
                    return Ok(!is_readonly);
                }
            }

            return Ok(true);
        }

        // committed shapes carry the same field access
        if let TypeTerm::Literal(dir::Type::Shape(shape)) = owner {
            for field in shape.fields {
                if field.key.matches(&key) {
                    return Ok(!field.is_readonly);
                }
            }

            return Ok(true);
        }

        Ok(true)
    }

    /// Return a generic type inference diagnostic.
    fn type_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::CannotInferType { anchor, module })
    }

    /// Return a generic static evaluation diagnostic.
    fn static_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::CannotEvaluateStatic { anchor, module })
    }

    /// Report one type term that could not produce its result.
    fn report_type_term_failure(
        &mut self,
        term: &TypeTerm,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<()> {
        let diagnostic = match term {
            TypeTerm::Member { key, .. } => {
                self.missing_member_diagnostic(origin, fallback, *key)?
            }
            TypeTerm::Call(call) => self.call_failure_diagnostic(call.source, origin, fallback)?,
            TypeTerm::Construct(construct) => {
                self.construct_failure_diagnostic(construct.source, origin, fallback)?
            }
            TypeTerm::Operator(operator) => {
                self.operator_failure_diagnostic(operator, origin, fallback)?
            }
            TypeTerm::Index(index) => {
                self.call_failure_diagnostic(index.source, origin, fallback)?
            }
            TypeTerm::KeyMembership(membership) => {
                self.type_diagnostic(ConstraintOrigin::Node(membership.source), fallback)?
            }
            TypeTerm::InstanceCheck(instance) => {
                self.type_diagnostic(ConstraintOrigin::Node(instance.source), fallback)?
            }
            TypeTerm::Identity(identity) => {
                self.type_diagnostic(ConstraintOrigin::Node(identity.source), fallback)?
            }
            TypeTerm::Template(template) => {
                self.type_diagnostic(ConstraintOrigin::Node(template.source), fallback)?
            }
            TypeTerm::TaggedTemplate(template) => {
                self.type_diagnostic(ConstraintOrigin::Node(template.source), fallback)?
            }
            _ => self.type_diagnostic(origin, fallback)?,
        };
        let module = self.diagnostic_module(origin, fallback);

        self.module_mut(module)?.work.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return the diagnostic for one failed call.
    fn call_failure_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .module(source.module_id)?
            .decisions
            .call
            .get(&source)
            .and_then(|outcome| match outcome {
                CallOutcome::Rejected(failure) => Some(*failure),
                CallOutcome::Resolved(_) => None,
            });

        match failure {
            Some(CallFailure::NotCallable) => self.not_callable_diagnostic(origin, fallback),
            Some(CallFailure::NoMatch) => self.no_matching_call_diagnostic(origin, fallback),
            None => Err(CompilerError::Internal {
                message: format!("failed call {} has no recorded outcome", source.local_id.id),
            }),
        }
    }

    /// Return the diagnostic for one failed construct.
    fn construct_failure_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .module(source.module_id)?
            .decisions
            .construct
            .get(&source)
            .and_then(|outcome| match outcome {
                ConstructOutcome::Rejected(failure) => Some(*failure),
                ConstructOutcome::Resolved(_) => None,
            });

        match failure {
            Some(ConstructFailure::NotConstructible) => {
                self.not_callable_diagnostic(origin, fallback)
            }
            Some(ConstructFailure::NoMatch) => self.no_matching_call_diagnostic(origin, fallback),
            None => Err(CompilerError::Internal {
                message: format!(
                    "failed construct {} has no recorded outcome",
                    source.local_id.id
                ),
            }),
        }
    }

    /// Return the diagnostic for one failed operator.
    fn operator_failure_diagnostic(
        &self,
        operator: &OperatorTerm,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .module(operator.source.module_id)?
            .decisions
            .operator
            .get(&operator.source)
            .and_then(|outcome| match outcome {
                OperatorOutcome::Rejected(failure) => Some(*failure),
                OperatorOutcome::Resolved(_) => None,
            });

        match failure {
            Some(failure) => match failure.reason {
                OperatorFailureReason::NoMatch => {
                    self.no_matching_operator_diagnostic(origin, fallback, failure.kind)
                }
                OperatorFailureReason::InvalidStrictEquality => {
                    self.invalid_strict_equality_diagnostic(origin, fallback)
                }
            },
            None => Err(CompilerError::Internal {
                message: format!(
                    "failed operator {} has no recorded outcome",
                    operator.source.local_id.id
                ),
            }),
        }
    }

    /// Return a missing member diagnostic.
    fn missing_member_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
        key: dir::StaticKey,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;
        let check_module = self.module(module)?;
        let key = match key {
            dir::StaticKey::Name(name) => check_module.input.strings.get(name).to_string(),
            dir::StaticKey::Number(name) => check_module.input.strings.get(name).to_string(),
            dir::StaticKey::Symbol(symbol) => format!("{symbol:?}"),
        };

        Ok(CheckError::MissingMember {
            anchor,
            module,
            key,
        })
    }

    /// Return a non-callable callee diagnostic.
    fn not_callable_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::NotCallable { anchor, module })
    }

    /// Return a no matching call overload diagnostic.
    fn no_matching_call_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::NoMatchingCall { anchor, module })
    }

    /// Return a no matching operator diagnostic.
    fn no_matching_operator_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
        operator: OperatorTermKind,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;
        let operator = match operator {
            OperatorTermKind::Unary(operator) => operator.text(),
            OperatorTermKind::Binary(operator) => operator.text(),
        }
        .to_string();

        Ok(CheckError::NoMatchingOperator {
            anchor,
            module,
            operator,
        })
    }

    /// Return an invalid strict equality diagnostic.
    fn invalid_strict_equality_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::InvalidStrictEquality { anchor, module })
    }

    /// Return the module that owns one diagnostic origin.
    fn diagnostic_module(&self, origin: ConstraintOrigin, fallback: ModuleId) -> ModuleId {
        match origin {
            ConstraintOrigin::Node(node) => node.module_id,
            ConstraintOrigin::Symbol(symbol) => symbol.module_id,
            ConstraintOrigin::Synthetic => fallback,
        }
    }

    /// Return a type relation diagnostic.
    fn type_relation_diagnostic(
        &self,
        relation: TypeRelation,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;
        let diagnostic = match relation {
            TypeRelation::Equal => CheckError::CannotInferType { anchor, module },
            TypeRelation::Assignable => CheckError::NotAssignable { anchor, module },
            TypeRelation::Castable => CheckError::InvalidCast { anchor, module },
            TypeRelation::Satisfies => CheckError::ConstraintNotSatisfied { anchor, module },
            TypeRelation::Extends => CheckError::DoesNotExtend { anchor, module },
            TypeRelation::Implements => CheckError::DoesNotImplement { anchor, module },
        };

        Ok(diagnostic)
    }

    /// Return a not writable diagnostic.
    fn not_writable_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::NotWritable { anchor, module })
    }

    /// Return an invalid intrinsic type diagnostic.
    fn invalid_intrinsic_type_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::InvalidIntrinsicType { anchor, module })
    }

    /// Return an invalid const type diagnostic.
    fn invalid_const_type_diagnostic(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin, fallback)?;

        Ok(CheckError::InvalidConstType { anchor, module })
    }

    /// Return the diagnostic anchor for a constraint origin.
    fn anchor(
        &self,
        origin: ConstraintOrigin,
        fallback: ModuleId,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = match origin {
            ConstraintOrigin::Node(node) => node.module_id,
            ConstraintOrigin::Symbol(symbol) => symbol.module_id,
            ConstraintOrigin::Synthetic => fallback,
        };
        let check_module = self.module(module)?;
        let source = match origin {
            ConstraintOrigin::Node(node) if node.module_id == module => node.local_id,
            ConstraintOrigin::Symbol(symbol) if symbol.module_id == module => {
                let Some(source) = check_module.symbol_source_node(symbol) else {
                    return Ok((module, DiagnosticAnchor::Module(module)));
                };

                source
            }
            ConstraintOrigin::Synthetic
            | ConstraintOrigin::Node(_)
            | ConstraintOrigin::Symbol(_) => {
                return Ok((module, DiagnosticAnchor::Module(module)));
            }
        };
        let Some(span) = check_module.input.parsed.tree.get_span_by_id(source.id) else {
            return Err(CompilerError::Internal {
                message: format!("check node {} has no source span", source.id),
            });
        };

        Ok((module, DiagnosticAnchor::from(span)))
    }
}
