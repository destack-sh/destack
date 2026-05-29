use destack_artifact::ToDiagnostic;
use destack_dir as dir;
use destack_source::{DiagnosticCollection, ModuleId};
use indexmap::IndexSet;

use crate::check::{
    CallFailure, CallSelection, CheckError, CheckState, Constraint, ConstructFailure,
    ConstructSelection, Decision, Definition, IdentitySelection, OperatorFailureReason,
    OperatorSelection, OperatorTerm, OperatorTermKind, Origin, ShapeMember, StaticRelation,
    StaticTerm, TypeLiteralTerm, TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerResult, DiagnosticAnchor};

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
        let mut diagnostics = Vec::new();

        self.collect_walk_diagnostics(&mut diagnostics);
        self.collect_definition_diagnostics(&mut diagnostics)?;
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

    /// Collect diagnostics for definitions that did not solve.
    fn collect_definition_diagnostics(
        &mut self,
        diagnostics: &mut Vec<CheckError>,
    ) -> CompilerResult<()> {
        let definitions = self.variables.definitions.clone();
        let mut reported = IndexSet::new();

        // validate solved definitions
        for definition in definitions {
            self.collect_definition_diagnostic(&definition, diagnostics, &mut reported)?;
        }

        Ok(())
    }

    /// Collect one definition diagnostic.
    fn collect_definition_diagnostic(
        &mut self,
        definition: &Definition,
        diagnostics: &mut Vec<CheckError>,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let condition = match definition {
            Definition::Type { condition, .. } | Definition::Static { condition, .. } => condition,
        };
        match self.reduce_condition_decision(condition)? {
            Decision::No => return Ok(()),
            Decision::Undecidable => return Ok(()),
            Decision::Yes => {}
        }

        match definition {
            Definition::Type {
                result,
                term,
                origin,
                condition: _,
            } => {
                let term = self.terms.get(*term).clone();
                self.collect_type_definition_diagnostic(
                    *result,
                    &term,
                    *origin,
                    diagnostics,
                    reported,
                )?;
            }
            Definition::Static {
                result,
                term,
                origin,
                condition: _,
            } => {
                let term = self.terms.get(*term).clone();
                self.collect_static_definition_diagnostic(
                    *result,
                    &term,
                    *origin,
                    diagnostics,
                    reported,
                )?;
            }
        }

        Ok(())
    }

    /// Collect one type definition diagnostic.
    fn collect_type_definition_diagnostic(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
        origin: Origin,
        diagnostics: &mut Vec<CheckError>,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        if self.type_term_contains_marker(term, TypeTermMarker::Intrinsic)? {
            let diagnostic = self.invalid_intrinsic_type_diagnostic(origin)?;

            diagnostics.push(diagnostic);

            return Ok(());
        }

        if self.type_term_contains_marker(term, TypeTermMarker::Const)? {
            let diagnostic = self.invalid_const_type_diagnostic(origin)?;

            diagnostics.push(diagnostic);

            return Ok(());
        }

        if self.solved_type_term(result)?.is_none() && reported.insert(result) {
            let diagnostic = self.type_term_failure_diagnostic(term, origin)?;

            diagnostics.push(diagnostic);
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
        origin: Origin,
        diagnostics: &mut Vec<CheckError>,
        reported: &mut IndexSet<VariableId>,
    ) -> CompilerResult<()> {
        let solved = self.solved_static_term(result)?;
        let is_valid = match (&solved, term) {
            (Some(StaticTerm::Literal(_)), StaticTerm::Expression(_)) => true,
            (Some(value), expected) => {
                self.decide_static_term_relation(StaticRelation::Equal, value, expected)?
                    != Decision::No
            }
            (None, _) => false,
        };

        if !is_valid && reported.insert(result) {
            let diagnostic = self.static_term_failure_diagnostic(term, origin)?;

            diagnostics.push(diagnostic);
        }

        Ok(())
    }

    /// Return a static definition diagnostic.
    fn static_term_failure_diagnostic(
        &self,
        term: &StaticTerm,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        match term {
            StaticTerm::Layout(_) => self.layout_not_realizable_diagnostic(origin),
            StaticTerm::Literal(_)
            | StaticTerm::Variable(_)
            | StaticTerm::Parameter(_)
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
    fn collect_constraint_diagnostics(
        &mut self,
        diagnostics: &mut Vec<CheckError>,
    ) -> CompilerResult<()> {
        let constraints = self.variables.constraints.clone();

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
                let left = self.reduce_type_operand(*origin, *left)?;
                let right = self.reduce_type_operand(*origin, *right)?;
                if let (
                    TypeRelation::Assignable,
                    Some(TypeTerm::Shape { members: left }),
                    Some(TypeTerm::Shape { members: right }),
                ) = (*relation, &left, &right)
                    && let Some(diagnostic) =
                        self.fresh_object_excess_property_diagnostic(*origin, left, right)?
                {
                    diagnostics.push(diagnostic);

                    return Ok(());
                }

                let decision = match (left, right) {
                    (Some(left), Some(right)) => {
                        self.decide_type_term_relation(*relation, &left, &right)?
                    }
                    _ => Decision::Undecidable,
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
        }

        Ok(())
    }

    /// Return an excess property diagnostic for a fresh object literal.
    fn fresh_object_excess_property_diagnostic(
        &self,
        origin: Origin,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<Option<CheckError>> {
        let Origin::Node(source_node) = origin else {
            return Ok(None);
        };
        if source_node.local_id.ty != dir::NodeType::Expression {
            return Ok(None);
        }

        let module = source_node.module_id;
        let expression = source_node.local_id.into_typed::<dir::Expression>();
        let object_properties = {
            let view = self.module(module).view();
            let dir::Expression::ObjectExpression { properties } = view.get(expression) else {
                return Ok(None);
            };

            properties.clone()
        };

        for property in object_properties {
            let extra = self.fresh_object_excess_property(module, property, source, target)?;
            if let Some((source, key)) = extra {
                let anchor = self.diagnostic_anchor(module, source);
                let key = self.static_key_label(module, key);
                let diagnostic = CheckError::ExcessProperty {
                    anchor,
                    module,
                    key,
                };

                return Ok(Some(diagnostic));
            }
        }

        Ok(None)
    }

    /// Return one excess property from a fresh object literal.
    fn fresh_object_excess_property(
        &self,
        module: ModuleId,
        property: dir::LocalNodeId<dir::Property>,
        source: &[ShapeMember],
        target: &[ShapeMember],
    ) -> CompilerResult<Option<(dir::LocalNodeIdAny, dir::StaticKey)>> {
        let view = self.module(module).view();
        let Some(key) = view.get(property).key() else {
            return Ok(None);
        };
        let Some(key) = key.static_key(view.tree()) else {
            return Ok(None);
        };

        if self.shape_field_type(source, key).is_none() {
            return Ok(None);
        }
        if self.shape_accepts_key(target, key)? {
            return Ok(None);
        }

        Ok(Some((property.into_any(), key)))
    }

    /// Return whether a target shape can accept one property key.
    fn shape_accepts_key(
        &self,
        target: &[ShapeMember],
        key: dir::StaticKey,
    ) -> CompilerResult<bool> {
        if self.shape_field_type(target, key).is_some() {
            return Ok(true);
        }

        for member in target {
            let ShapeMember::IndexSignature { key_type, .. } = member else {
                continue;
            };
            let key_type = key_type.to_type_term(self);
            let key = Self::static_key_type_term(key);
            let decision =
                self.decide_type_term_relation(TypeRelation::Assignable, &key, &key_type)?;
            if decision != Decision::No {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return a literal type term for a static property key.
    fn static_key_type_term(key: dir::StaticKey) -> TypeTerm {
        let literal = match key {
            dir::StaticKey::Name(name) => TypeLiteralTerm::Scalar(dir::ScalarLiteral::String(name)),
            dir::StaticKey::Index(index) => i64::try_from(index)
                .map(|index| TypeLiteralTerm::Scalar(dir::ScalarLiteral::Integer(index)))
                .unwrap_or_else(|_| TypeLiteralTerm::integer()),
            dir::StaticKey::Symbol(_) => {
                TypeLiteralTerm::Primitive(dir::PrimitiveType::UniqueSymbol)
            }
        };

        TypeTerm::Literal(literal)
    }

    /// Return a label for a static property key.
    fn static_key_label(&self, module: ModuleId, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.module(module).strings.get(name).to_owned(),
            dir::StaticKey::Index(index) => format!("#{index}"),
            dir::StaticKey::Symbol(symbol) => symbol.debug_string(&self.module(module).strings),
        }
    }

    /// Return a diagnostic for one failed type term.
    fn type_term_failure_diagnostic(
        &self,
        term: &TypeTerm,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let diagnostic = match term {
            TypeTerm::Member(member) => {
                let member = self.terms.get(*member);

                self.missing_member_diagnostic(member.origin, member.key)?
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

                self.type_diagnostic(Origin::Node(range.source))?
            }
            TypeTerm::Tree(tree) => {
                let tree = self.terms.get(*tree);

                self.type_diagnostic(Origin::Node(tree.source))?
            }
            TypeTerm::TypeValue(value) => {
                let value = self.terms.get(*value);

                self.type_diagnostic(Origin::Node(value.source))?
            }
            TypeTerm::ImportMeta(meta) => {
                let meta = self.terms.get(*meta);

                self.type_diagnostic(Origin::Node(meta.source))?
            }
            TypeTerm::Receiver(receiver) => {
                let receiver = self.terms.get(*receiver);

                self.type_diagnostic(Origin::Node(receiver.source))?
            }
            TypeTerm::Super(term) => {
                let term = self.terms.get(*term);

                self.type_diagnostic(Origin::Node(term.source))?
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = self.terms.get(*membership);

                self.type_diagnostic(Origin::Node(membership.source))?
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = self.terms.get(*instance);

                self.type_diagnostic(Origin::Node(instance.source))?
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.terms.get(*yielded);

                self.type_diagnostic(Origin::Node(yielded.source))?
            }
            TypeTerm::Template(template) => {
                let template = self.terms.get(*template);

                self.type_diagnostic(Origin::Node(template.source))?
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
        let (module, anchor) = self.anchor(Origin::Node(source));

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
            Some(IdentitySelection::Rejected(_)) => {
                self.invalid_strict_equality_diagnostic(Origin::Node(source))
            }
            Some(IdentitySelection::Resolved(_)) | None => {
                self.type_diagnostic(Origin::Node(source))
            }
        }
    }

    /// Return the diagnostic for one failed call.
    fn call_failure_from_decision(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        match self.solutions.call.get(&source) {
            Some(CallSelection::Rejected(failure)) => {
                self.call_failure_diagnostic(source, origin, failure.clone())
            }
            Some(CallSelection::Resolved(_)) | None => self.type_diagnostic(origin),
        }
    }

    /// Return the diagnostic for one call failure reason.
    fn call_failure_diagnostic(
        &self,
        _source: dir::GlobalNodeIdAny,
        origin: Origin,
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
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .solutions
            .construct
            .get(&source)
            .and_then(|decision| match decision {
                ConstructSelection::Rejected(failure) => Some(*failure),
                ConstructSelection::Resolved(_) => None,
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
        origin: Origin,
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
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let failure = self
            .solutions
            .operator
            .get(&operator.source)
            .and_then(|decision| match decision {
                OperatorSelection::Rejected(failure) => Some(*failure),
                OperatorSelection::Resolved(_) => None,
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
        origin: Origin,
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
    fn type_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::CannotSolve { anchor, module })
    }

    /// Return a generic static evaluation diagnostic.
    fn static_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::CannotSolve { anchor, module })
    }

    /// Return a missing member diagnostic.
    fn missing_member_diagnostic(
        &self,
        origin: Origin,
        key: dir::StaticKey,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);
        let key = key.debug_string(&self.module(module).strings);

        Ok(CheckError::MissingMember {
            anchor,
            module,
            key,
        })
    }

    /// Return a non-callable callee diagnostic.
    fn not_callable_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::NotCallable { anchor, module })
    }

    /// Return a no matching call overload diagnostic.
    fn no_matching_call_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::NoMatchingCall { anchor, module })
    }

    /// Return a no matching operator diagnostic.
    fn no_matching_operator_diagnostic(
        &self,
        origin: Origin,
        operator: OperatorTermKind,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);
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
    fn invalid_strict_equality_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::InvalidStrictEquality { anchor, module })
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

    /// Return an invalid intrinsic type diagnostic.
    fn invalid_intrinsic_type_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::InvalidIntrinsicType { anchor, module })
    }

    /// Return an invalid const type diagnostic.
    fn invalid_const_type_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::InvalidConstType { anchor, module })
    }

    /// Return a layout realization diagnostic.
    fn layout_not_realizable_diagnostic(&self, origin: Origin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin);

        Ok(CheckError::LayoutNotConcrete { anchor, module })
    }

    /// Return the best diagnostic origin for one variable.
    fn variable_diagnostic_origin(&self, variable: VariableId) -> Origin {
        self.variable(variable).source
    }

    /// Return the diagnostic anchor for a constraint origin.
    fn anchor(&self, origin: Origin) -> (ModuleId, DiagnosticAnchor) {
        self.diagnostic_anchor_for_origin(origin)
    }
}
