use destack_artifact::ToDiagnostic;
use destack_dir as dir;
use destack_source::{DiagnosticCollection, ModuleId};
use indexmap::IndexSet;

use crate::check::{
    CallDecision, CallFailure, CheckError, CheckState, Constraint, ConstraintOrigin,
    ConstructDecision, ConstructFailure, Definition, IdentityDecision, OperatorDecision,
    OperatorFailureReason, OperatorTerm, OperatorTermKind, StaticRelation, StaticTerm,
    TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult, DiagnosticAnchor};

/// Source-only type markers that cannot be committed as DIR types.
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

impl CheckState<'_> {
    /// Collect final diagnostics for one checked component.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        self.collect_definition_diagnostics()?;
        self.collect_constraint_diagnostics()?;
        self.collect_decision_diagnostics()?;
        self.check_obligations()?;

        let mut collection = DiagnosticCollection::new();
        let modules = self.component_modules.clone();

        // drain diagnostics in stable component order
        for module in modules {
            let diagnostics = std::mem::take(self.diagnostics_mut(module));

            for diagnostic in diagnostics {
                collection.insert(diagnostic.to_diagnostic(self.context)?);
            }
        }

        Ok(collection)
    }

    /// Collect diagnostics for definitions that did not solve.
    fn collect_definition_diagnostics(&mut self) -> CompilerResult<()> {
        let definitions = self.variables.definitions.clone();
        let mut reported = IndexSet::new();

        // validate solved definitions
        for definition in definitions {
            self.collect_definition_diagnostic(&definition, &mut reported)?;
        }

        Ok(())
    }

    /// Collect one definition diagnostic.
    fn collect_definition_diagnostic(
        &mut self,
        definition: &Definition,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let condition = match definition {
            Definition::Type { condition, .. } | Definition::Static { condition, .. } => condition,
        };
        match self.decide_static_condition(condition)? {
            crate::check::Decision::No => return Ok(()),
            crate::check::Decision::Undecidable => {
                let origin = match definition {
                    Definition::Type { origin, .. } | Definition::Static { origin, .. } => *origin,
                };
                let diagnostic = self.static_diagnostic(origin)?;
                let module = self.diagnostic_module(origin);

                self.diagnostics_mut(module).push(diagnostic);

                return Ok(());
            }
            crate::check::Decision::Yes => {}
        }

        match definition {
            Definition::Type {
                result,
                term,
                origin,
                condition: _,
            } => {
                let term = self.terms.get(*term).clone();
                self.collect_type_definition_diagnostic(*result, &term, *origin, reported)?;
            }
            Definition::Static {
                result,
                term,
                origin,
                condition: _,
            } => {
                let term = self.terms.get(*term).clone();
                self.collect_static_definition_diagnostic(*result, &term, *origin, reported)?;
            }
        }

        Ok(())
    }

    /// Collect one type definition diagnostic.
    fn collect_type_definition_diagnostic(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
        origin: ConstraintOrigin,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        if self.type_term_contains_marker(term, TypeTermMarker::Intrinsic)? {
            let diagnostic = self.invalid_intrinsic_type_diagnostic(origin)?;
            let module = self.diagnostic_module(origin);

            self.diagnostics_mut(module).push(diagnostic);

            return Ok(());
        }

        if self.type_term_contains_marker(term, TypeTermMarker::Const)? {
            let diagnostic = self.invalid_const_type_diagnostic(origin)?;
            let module = self.diagnostic_module(origin);

            self.diagnostics_mut(module).push(diagnostic);

            return Ok(());
        }

        if self.solved_type_term(result)?.is_none() && reported.insert(result) {
            let diagnostic = self.type_term_failure_diagnostic(term, origin)?;
            let module = self.diagnostic_module(origin);

            self.diagnostics_mut(module).push(diagnostic);
        }

        Ok(())
    }

    /// Return whether one type term contains a source-only marker.
    fn type_term_contains_marker(
        &self,
        term: &TypeTerm,
        marker: TypeTermMarker,
    ) -> CompilerResult<bool> {
        let mut seen = IndexSet::new();

        self.type_term_contains_marker_inner(term, marker, &mut seen)
    }

    /// Return whether one type term transitively contains a source-only marker.
    fn type_term_contains_marker_inner(
        &self,
        term: &TypeTerm,
        marker: TypeTermMarker,
        seen: &mut IndexSet<VariableId>,
    ) -> CompilerResult<bool> {
        if marker.matches(term) {
            return Ok(true);
        }

        // follow solved type variables once
        for variable in term.referenced_variables(self) {
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

    /// Collect one static definition diagnostic.
    fn collect_static_definition_diagnostic(
        &mut self,
        result: VariableId,
        term: &StaticTerm,
        origin: ConstraintOrigin,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let solved = self.solved_static_term(result)?;
        let is_valid = match (&solved, term) {
            (Some(StaticTerm::Literal(_)), StaticTerm::Expression(_)) => true,
            (Some(value), expected) => {
                self.decide_static_term_relation(StaticRelation::Equal, value, expected)?
                    != crate::check::Decision::No
            }
            (None, _) => false,
        };

        if !is_valid && reported.insert(result) {
            let diagnostic = self.static_term_failure_diagnostic(term, origin)?;

            self.diagnostics_mut(result.module).push(diagnostic);
        }

        Ok(())
    }

    /// Return a static definition diagnostic.
    fn static_term_failure_diagnostic(
        &self,
        term: &StaticTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        match term {
            StaticTerm::Layout(_) => self.layout_not_realizable_diagnostic(origin),
            StaticTerm::Literal(_)
            | StaticTerm::Variable(_)
            | StaticTerm::Expression(_)
            | StaticTerm::Member { .. }
            | StaticTerm::Join { .. }
            | StaticTerm::Intrinsic { .. }
            | StaticTerm::Equal { .. }
            | StaticTerm::TypeRelation { .. }
            | StaticTerm::Conditional { .. } => self.static_diagnostic(origin),
        }
    }

    /// Collect diagnostics for final constraint decisions.
    fn collect_constraint_diagnostics(&mut self) -> CompilerResult<()> {
        let constraints = self.variables.constraints.clone();

        // render rejected constraints
        for constraint in constraints {
            self.collect_constraint_diagnostic(&constraint)?;
        }

        Ok(())
    }

    /// Collect one constraint diagnostic.
    fn collect_constraint_diagnostic(&mut self, constraint: &Constraint) -> CompilerResult<()> {
        let condition = constraint.condition();
        match self.decide_static_condition(&condition)? {
            crate::check::Decision::No => return Ok(()),
            crate::check::Decision::Undecidable => {
                let diagnostic = self.static_diagnostic(constraint.origin())?;
                let module = self.diagnostic_module(constraint.origin());

                self.diagnostics_mut(module).push(diagnostic);

                return Ok(());
            }
            crate::check::Decision::Yes => {}
        }

        match constraint {
            Constraint::Type {
                relation,
                left,
                right,
                origin,
                condition: _,
            } => {
                let decision = self.decide_type_relation(*relation, *left, *right)?;
                if decision != crate::check::Decision::Yes {
                    let diagnostic = self.type_relation_diagnostic(*relation, *origin)?;
                    let module = self.diagnostic_module(*origin);

                    self.diagnostics_mut(module).push(diagnostic);
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
                if decision != crate::check::Decision::Yes {
                    let diagnostic = self.static_diagnostic(*origin)?;
                    let module = self.diagnostic_module(*origin);

                    self.diagnostics_mut(module).push(diagnostic);
                }
            }
            Constraint::Pattern {
                relation,
                value,
                origin,
                condition: _,
            } => {
                let decision = self.decide_pattern_relation(*value, relation)?;
                if decision != crate::check::Decision::Yes {
                    let diagnostic =
                        self.type_relation_diagnostic(TypeRelation::Satisfies, *origin)?;

                    self.diagnostics_mut(value.module).push(diagnostic);
                }
            }
        }

        Ok(())
    }

    /// Collect diagnostics from rejected solver decisions.
    fn collect_decision_diagnostics(&mut self) -> CompilerResult<()> {
        let identities = self.solutions.identity.clone();
        let calls = self.solutions.call.clone();
        let constructs = self.solutions.construct.clone();
        let operators = self.solutions.operator.clone();
        let members = self.solutions.member.clone();
        let layouts = self.solutions.layout.clone();

        // render identity failures
        for (source, decision) in identities {
            if matches!(decision, IdentityDecision::Rejected(_)) {
                let diagnostic =
                    self.invalid_strict_equality_diagnostic(ConstraintOrigin::Node(source))?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        // render call failures
        for (source, decision) in calls {
            if let CallDecision::Rejected(failure) = decision {
                let diagnostic =
                    self.call_failure_diagnostic(source, ConstraintOrigin::Node(source), failure)?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        // render construct failures
        for (source, decision) in constructs {
            if let ConstructDecision::Rejected(failure) = decision {
                let diagnostic = self.construct_failure_diagnostic(
                    source,
                    ConstraintOrigin::Node(source),
                    failure,
                )?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        // render operator failures
        for (source, decision) in operators {
            if let OperatorDecision::Rejected(failure) = decision {
                let diagnostic = self.operator_failure_diagnostic(
                    failure.kind,
                    failure.reason,
                    ConstraintOrigin::Node(source),
                )?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        // render member failures
        for (source, decision) in members {
            if let crate::check::MemberDecision::Rejected(_) = decision {
                let diagnostic = self.type_diagnostic(ConstraintOrigin::Node(source))?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        // render layout failures
        for (source, decision) in layouts {
            if matches!(decision, crate::check::LayoutDecision::Rejected(_)) {
                let diagnostic =
                    self.layout_not_realizable_diagnostic(ConstraintOrigin::Node(source))?;

                self.diagnostics_mut(source.module_id).push(diagnostic);
            }
        }

        Ok(())
    }

    /// Return a diagnostic for one failed type term.
    fn type_term_failure_diagnostic(
        &self,
        term: &TypeTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let diagnostic = match term {
            TypeTerm::Member(member) => {
                let member = self.terms.get(*member);
                let origin = member.source.map_or(origin, ConstraintOrigin::Node);

                self.missing_member_diagnostic(origin, member.key)?
            }
            TypeTerm::Call(call) => {
                let call = self.terms.get(*call);

                self.call_failure_from_decision(call.source, origin)?
            }
            TypeTerm::Construct(construct) => {
                let construct = self.terms.get(*construct);

                self.construct_failure_from_decision(construct.source, origin)?
            }
            TypeTerm::Operator(operator) => {
                let operator = self.terms.get(*operator);

                self.operator_failure_from_decision(operator, origin)?
            }
            TypeTerm::Index(index) => {
                let index = self.terms.get(*index);

                self.call_failure_from_decision(index.source, origin)?
            }
            TypeTerm::IndexSet(set) => {
                let set = self.terms.get(*set);

                self.call_failure_from_decision(set.source, origin)?
            }
            TypeTerm::Identity(identity) => {
                let identity = self.terms.get(*identity);

                self.identity_failure_from_decision(identity.source)?
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.terms.get(*awaited);

                self.await_failure_diagnostic(awaited.source)?
            }
            TypeTerm::RangeValue(range) => {
                let range = self.terms.get(*range);

                self.type_diagnostic(ConstraintOrigin::Node(range.source))?
            }
            TypeTerm::Tree(tree) => {
                let tree = self.terms.get(*tree);

                self.type_diagnostic(ConstraintOrigin::Node(tree.source))?
            }
            TypeTerm::TypeValue(value) => {
                let value = self.terms.get(*value);

                self.type_diagnostic(ConstraintOrigin::Node(value.source))?
            }
            TypeTerm::ImportMeta(meta) => {
                let meta = self.terms.get(*meta);

                self.type_diagnostic(ConstraintOrigin::Node(meta.source))?
            }
            TypeTerm::Receiver(receiver) => {
                let receiver = self.terms.get(*receiver);

                self.type_diagnostic(ConstraintOrigin::Node(receiver.source))?
            }
            TypeTerm::Super(term) => {
                let term = self.terms.get(*term);

                self.type_diagnostic(ConstraintOrigin::Node(term.source))?
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = self.terms.get(*membership);

                self.type_diagnostic(ConstraintOrigin::Node(membership.source))?
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = self.terms.get(*instance);

                self.type_diagnostic(ConstraintOrigin::Node(instance.source))?
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.terms.get(*yielded);

                self.type_diagnostic(ConstraintOrigin::Node(yielded.source))?
            }
            TypeTerm::Template(template) => {
                let template = self.terms.get(*template);

                self.type_diagnostic(ConstraintOrigin::Node(template.source))?
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = self.terms.get(*template);

                self.call_failure_from_decision(template.source, origin)?
            }
            TypeTerm::Try(_) | TypeTerm::TryFailure(_) => self.type_diagnostic(origin)?,
            _ => self.type_diagnostic(origin)?,
        };

        Ok(diagnostic)
    }

    /// Return the diagnostic for one failed await expression.
    fn await_failure_diagnostic(&self, source: dir::GlobalNodeIdAny) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(ConstraintOrigin::Node(source))?;

        Ok(CheckError::InvalidAwait {
            anchor,
            module,
            message: "await requires a Promise value".to_owned(),
        })
    }

    /// Return the diagnostic for one failed identity comparison.
    fn identity_failure_from_decision(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<CheckError> {
        match self.solutions.identity.get(&source) {
            Some(IdentityDecision::Rejected(_)) => {
                self.invalid_strict_equality_diagnostic(ConstraintOrigin::Node(source))
            }
            Some(IdentityDecision::Resolved(_)) | None => {
                self.type_diagnostic(ConstraintOrigin::Node(source))
            }
        }
    }

    /// Return the diagnostic for one failed call.
    fn call_failure_from_decision(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        match self.solutions.call.get(&source) {
            Some(CallDecision::Rejected(failure)) => {
                self.call_failure_diagnostic(source, origin, failure.clone())
            }
            Some(CallDecision::Resolved(_)) | None => self.type_diagnostic(origin),
        }
    }

    /// Return the diagnostic for one call failure reason.
    fn call_failure_diagnostic(
        &self,
        _source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
        failure: CallFailure,
    ) -> CompilerResult<CheckError> {
        match failure {
            CallFailure::NotCallable => self.not_callable_diagnostic(origin),
            CallFailure::NoMatch => self.no_matching_call_diagnostic(origin),
            CallFailure::ArgumentType {
                argument,
                parameter: _,
            } => {
                let origin = argument
                    .variable()
                    .map(|argument| self.variable_diagnostic_origin(argument))
                    .unwrap_or(origin);

                self.type_relation_diagnostic(TypeRelation::Assignable, origin)
            }
        }
    }

    /// Return the diagnostic for one failed construct expression.
    fn construct_failure_from_decision(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .solutions
            .construct
            .get(&source)
            .and_then(|decision| match decision {
                ConstructDecision::Rejected(failure) => Some(*failure),
                ConstructDecision::Resolved(_) => None,
            });

        match failure {
            Some(failure) => self.construct_failure_diagnostic(source, origin, failure),
            None => self.type_diagnostic(origin),
        }
    }

    /// Return the diagnostic for one construct failure reason.
    fn construct_failure_diagnostic(
        &self,
        _source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
        failure: ConstructFailure,
    ) -> CompilerResult<CheckError> {
        match failure {
            ConstructFailure::NotConstructible => self.not_callable_diagnostic(origin),
            ConstructFailure::NoMatch => self.no_matching_call_diagnostic(origin),
        }
    }

    /// Return the diagnostic for one failed operator term.
    fn operator_failure_from_decision(
        &self,
        operator: &OperatorTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .solutions
            .operator
            .get(&operator.source)
            .and_then(|decision| match decision {
                OperatorDecision::Rejected(failure) => Some(*failure),
                OperatorDecision::Resolved(_) => None,
            });

        match failure {
            Some(failure) => self.operator_failure_diagnostic(failure.kind, failure.reason, origin),
            None => self.type_diagnostic(origin),
        }
    }

    /// Return the diagnostic for one operator failure reason.
    fn operator_failure_diagnostic(
        &self,
        operator: OperatorTermKind,
        reason: OperatorFailureReason,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        match reason {
            OperatorFailureReason::NoMatch => {
                self.no_matching_operator_diagnostic(origin, operator)
            }
            OperatorFailureReason::InvalidStrictEquality => {
                self.invalid_strict_equality_diagnostic(origin)
            }
        }
    }

    /// Return a generic type inference diagnostic.
    fn type_diagnostic(&self, origin: ConstraintOrigin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::CannotSolve { anchor, module })
    }

    /// Return a generic static evaluation diagnostic.
    fn static_diagnostic(&self, origin: ConstraintOrigin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::CannotSolve { anchor, module })
    }

    /// Return a missing member diagnostic.
    fn missing_member_diagnostic(
        &self,
        origin: ConstraintOrigin,
        key: dir::StaticKey,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;
        let key = key.debug_string(&self.input(module).strings);

        Ok(CheckError::MissingMember {
            anchor,
            module,
            key,
        })
    }

    /// Return a non-callable callee diagnostic.
    fn not_callable_diagnostic(&self, origin: ConstraintOrigin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::NotCallable { anchor, module })
    }

    /// Return a no matching call overload diagnostic.
    fn no_matching_call_diagnostic(&self, origin: ConstraintOrigin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::NoMatchingCall { anchor, module })
    }

    /// Return a no matching operator diagnostic.
    fn no_matching_operator_diagnostic(
        &self,
        origin: ConstraintOrigin,
        operator: OperatorTermKind,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;
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
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::InvalidStrictEquality { anchor, module })
    }

    /// Return a type relation diagnostic.
    fn type_relation_diagnostic(
        &self,
        relation: TypeRelation,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;
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

    /// Return an invalid intrinsic type diagnostic.
    fn invalid_intrinsic_type_diagnostic(
        &self,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::InvalidIntrinsicType { anchor, module })
    }

    /// Return an invalid const type diagnostic.
    fn invalid_const_type_diagnostic(
        &self,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::InvalidConstType { anchor, module })
    }

    /// Return a layout realization diagnostic.
    fn layout_not_realizable_diagnostic(
        &self,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::LayoutNotConcrete { anchor, module })
    }

    /// Return the module that owns one diagnostic origin.
    fn diagnostic_module(&self, origin: ConstraintOrigin) -> ModuleId {
        match origin {
            ConstraintOrigin::Node(node) => node.module_id,
            ConstraintOrigin::Symbol(symbol) => symbol.module_id,
        }
    }

    /// Return the best diagnostic origin for one variable.
    fn variable_diagnostic_origin(&self, variable: VariableId) -> ConstraintOrigin {
        self.variable(variable).source
    }

    /// Return the diagnostic anchor for a constraint origin.
    fn anchor(&self, origin: ConstraintOrigin) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = self.diagnostic_module(origin);
        let source = match origin {
            ConstraintOrigin::Node(node) => node.local_id,
            ConstraintOrigin::Symbol(symbol) => self
                .symbol_source_node(module, symbol)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("check symbol {} has no source node", symbol.local_id.id),
                })?,
        };
        let span = self
            .input(module)
            .parsed
            .tree
            .get_span_by_id(source.id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check node {} has no source span", source.id),
            })?;

        Ok((module, DiagnosticAnchor::from(span)))
    }
}
