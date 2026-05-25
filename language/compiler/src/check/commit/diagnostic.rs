use destack_artifact::ToDiagnostic;
use destack_dir as dir;
use destack_source::{DiagnosticCollection, ModuleId};
use indexmap::IndexSet;

use crate::check::solve::Decision;
use crate::check::{
    AssignPatternFieldTerm, AssignPatternTerm, CallFailure, CallOutcome, CheckComponentState,
    CheckError, Constraint, ConstraintOrigin, ConstructFailure, ConstructOutcome,
    OperatorFailureReason, OperatorOutcome, OperatorTerm, OperatorTermKind, PatternFieldTerm,
    PatternRelation, PatternTerm, Place, PlaceTarget, ShapeMemberTerm, StaticRelation, StaticTerm,
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
    /// Collect solved component diagnostics.
    pub(in crate::check) fn collect_diagnostics(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut reported = IndexSet::new();
        let constraints = self.collect_constraints();

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
            Constraint::RelatePattern {
                relation,
                value,
                origin,
            } => match relation {
                PatternRelation::Match(pattern) => {
                    self.check_pattern_relation(*value, pattern, *origin)?;
                }
                PatternRelation::Assign(pattern) => {
                    self.check_assign_pattern_relation(*value, pattern, *origin)?;
                }
            },
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
            Constraint::RelatePattern {
                relation,
                value,
                origin: _,
            } => {
                let mut variables = smallvec::smallvec![*value];
                variables.extend(relation.referenced_variables());

                variables
            }
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
            let diagnostic = self.invalid_intrinsic_type_diagnostic(origin)?;
            let module = self.diagnostic_module(origin);

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        }
        if self.type_term_contains_marker(term, TypeTermMarker::Const)? {
            let diagnostic = self.invalid_const_type_diagnostic(origin)?;
            let module = self.diagnostic_module(origin);

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        }

        if self.solved_type_term(result)?.is_none() {
            if reported.insert(result) {
                self.report_type_term_failure(term, origin)?;
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
                let diagnostic = self.static_term_diagnostic(term, origin)?;

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

        let diagnostic = self.static_diagnostic(origin)?;
        self.module_mut(result.module)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Return a diagnostic for one failed static term.
    fn static_term_diagnostic(
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
            | StaticTerm::Operation(_)
            | StaticTerm::Intrinsic { .. } => self.static_diagnostic(origin),
        }
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
                let diagnostic = self.type_relation_diagnostic(relation, origin)?;

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
                let diagnostic = self.static_diagnostic(origin)?;

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

        let diagnostic = self.not_writable_diagnostic(origin)?;

        self.module_mut(place.source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Check one solved pattern expectation for diagnostics.
    fn check_pattern_relation(
        &mut self,
        value: VariableId,
        pattern: &PatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let value = TypeTerm::Variable(value);

        self.check_pattern_term_expectation(&value, pattern, origin)
    }

    /// Check one solved pattern term expectation for diagnostics.
    fn check_pattern_term_expectation(
        &mut self,
        value: &TypeTerm,
        pattern: &PatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        match pattern {
            PatternTerm::Wildcard => {}
            PatternTerm::Must { pattern }
            | PatternTerm::BorrowOf { pattern, .. }
            | PatternTerm::MoveOf { pattern, .. }
            | PatternTerm::DereferenceOf { pattern } => {
                self.check_pattern_term_expectation(value, pattern, origin)?;
            }
            PatternTerm::Assign {
                pattern,
                value: default,
            } => {
                self.check_type_term_relation(
                    TypeRelation::Assignable,
                    &TypeTerm::Variable(*default),
                    value,
                    origin,
                )?;
                self.check_pattern_term_expectation(value, pattern, origin)?;
            }
            PatternTerm::Binding { symbol, pattern } => {
                if let Some(symbol) = symbol {
                    let binding = self.pattern_binding_variable(*symbol)?;
                    let binding = TypeTerm::Variable(binding);

                    self.check_type_term_relation(TypeRelation::Equal, &binding, value, origin)?;
                }

                if let Some(pattern) = pattern {
                    self.check_pattern_term_expectation(value, pattern, origin)?;
                }
            }
            PatternTerm::Expression { value: expected } => {
                let expected = TypeTerm::Variable(*expected);

                self.check_type_term_relation(TypeRelation::Assignable, &expected, value, origin)?;
            }
            PatternTerm::Range {
                start,
                end,
                end_kind: _,
            } => {
                if let Some(start) = start {
                    let start = TypeTerm::Variable(*start);

                    self.check_type_term_relation(TypeRelation::Assignable, &start, value, origin)?;
                }

                if let Some(end) = end {
                    let end = TypeTerm::Variable(*end);

                    self.check_type_term_relation(TypeRelation::Assignable, &end, value, origin)?;
                }
            }
            PatternTerm::Type { ty } => {
                let ty = TypeTerm::Variable(*ty);

                self.check_type_term_relation(TypeRelation::Satisfies, value, &ty, origin)?;
            }
            PatternTerm::Tuple { fields } | PatternTerm::Sequence { fields } => {
                self.check_pattern_fields(value, fields, origin)?;
            }
            PatternTerm::Object { fields } => {
                self.check_pattern_fields(value, fields, origin)?;
            }
            PatternTerm::TaggedTuple { ty, fields } | PatternTerm::TaggedObject { ty, fields } => {
                let ty = TypeTerm::Variable(*ty);

                self.check_type_term_relation(TypeRelation::Satisfies, value, &ty, origin)?;
                self.check_pattern_fields(value, fields, origin)?;
            }
            PatternTerm::Union { patterns } => {
                self.check_pattern_union(value, patterns, origin)?;
            }
        }

        Ok(())
    }

    /// Check one solved assignment pattern expectation for diagnostics.
    fn check_assign_pattern_relation(
        &mut self,
        value: VariableId,
        pattern: &AssignPatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let value = TypeTerm::Variable(value);

        self.check_assign_pattern_term_expectation(&value, pattern, origin)
    }

    /// Check one solved assignment pattern term expectation for diagnostics.
    fn check_assign_pattern_term_expectation(
        &mut self,
        value: &TypeTerm,
        pattern: &AssignPatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        match pattern {
            AssignPatternTerm::Expression { place } => {
                let place = TypeTerm::Variable(*place);

                self.check_type_term_relation(TypeRelation::Assignable, value, &place, origin)?;
            }
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default = TypeTerm::Variable(*default);

                self.check_type_term_relation(TypeRelation::Assignable, &default, value, origin)?;
                self.check_assign_pattern_term_expectation(value, pattern, origin)?;
            }
            AssignPatternTerm::Sequence { fields } | AssignPatternTerm::Object { fields } => {
                self.check_assign_pattern_fields(value, fields, origin)?;
            }
        }

        Ok(())
    }

    /// Check one solved type term relation for diagnostics.
    fn check_type_term_relation(
        &mut self,
        relation: TypeRelation,
        left: &TypeTerm,
        right: &TypeTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let decision = self.decide_type_term_relation(relation, left, right)?;

        if decision == Decision::Yes {
            return Ok(());
        }

        let diagnostic = self.type_relation_diagnostic(relation, origin)?;
        let module = self.diagnostic_module(origin);

        self.module_mut(module)?.work.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return the variable for one pattern binding.
    fn pattern_binding_variable(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<VariableId> {
        let module = self.module(symbol.module_id)?;
        let Some(variable) = module.work.variables.type_by_symbol.get(&symbol).copied() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "check pattern binding symbol {} has no type variable",
                    symbol.local_id.id
                ),
            });
        };

        Ok(variable)
    }

    /// Check solved field patterns.
    fn check_pattern_fields(
        &mut self,
        value: &TypeTerm,
        fields: &[PatternFieldTerm],
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        for field in fields {
            self.check_pattern_field(value, field, origin)?;
        }

        Ok(())
    }

    /// Check one solved field pattern.
    fn check_pattern_field(
        &mut self,
        value: &TypeTerm,
        field: &PatternFieldTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        match field {
            PatternFieldTerm::Named { key, pattern } => {
                if let Some(pattern) = pattern {
                    self.check_named_pattern_field(value, key, pattern, origin)?;
                }
            }
            PatternFieldTerm::Computed { key: _, pattern }
            | PatternFieldTerm::Positional { pattern } => {
                self.check_pattern_term_expectation(value, pattern, origin)?;
            }
            PatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.check_pattern_term_expectation(value, pattern, origin)?;
                }
            }
            PatternFieldTerm::Elision => {}
        }

        Ok(())
    }

    /// Check one solved named field pattern.
    fn check_named_pattern_field(
        &mut self,
        value: &TypeTerm,
        key: &dir::StaticKey,
        pattern: &PatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let module = self.diagnostic_module(origin);
        let Some(ty) = self.member_type_term(module, value, key)? else {
            let diagnostic = self.missing_member_diagnostic(origin, *key)?;

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        };

        self.check_pattern_term_expectation(&ty, pattern, origin)
    }

    /// Check solved assignment field patterns.
    fn check_assign_pattern_fields(
        &mut self,
        value: &TypeTerm,
        fields: &[AssignPatternFieldTerm],
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        for field in fields {
            self.check_assign_pattern_field(value, field, origin)?;
        }

        Ok(())
    }

    /// Check one solved assignment field pattern.
    fn check_assign_pattern_field(
        &mut self,
        value: &TypeTerm,
        field: &AssignPatternFieldTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        match field {
            AssignPatternFieldTerm::Named { key, pattern } => {
                if let Some(pattern) = pattern {
                    self.check_named_assign_pattern_field(value, key, pattern, origin)?;
                }
            }
            AssignPatternFieldTerm::Computed { key: _, pattern }
            | AssignPatternFieldTerm::Positional { pattern } => {
                self.check_assign_pattern_term_expectation(value, pattern, origin)?;
            }
            AssignPatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.check_assign_pattern_term_expectation(value, pattern, origin)?;
                }
            }
            AssignPatternFieldTerm::Elision => {}
        }

        Ok(())
    }

    /// Check one solved named assignment field pattern.
    fn check_named_assign_pattern_field(
        &mut self,
        value: &TypeTerm,
        key: &dir::StaticKey,
        pattern: &AssignPatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let module = self.diagnostic_module(origin);
        let Some(ty) = self.member_type_term(module, value, key)? else {
            let diagnostic = self.missing_member_diagnostic(origin, *key)?;

            self.module_mut(module)?.work.diagnostics.push(diagnostic);

            return Ok(());
        };

        self.check_assign_pattern_term_expectation(&ty, pattern, origin)
    }

    /// Check whether one union pattern has at least one compatible branch.
    fn check_pattern_union(
        &mut self,
        value: &TypeTerm,
        patterns: &[PatternTerm],
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let mut decision = Decision::No;

        for pattern in patterns {
            decision = decision.or(self.pattern_term_can_apply(value, pattern, origin)?);
            if decision == Decision::Yes {
                return Ok(());
            }
        }

        let diagnostic = match decision {
            Decision::Yes => return Ok(()),
            Decision::Undecidable => self.type_diagnostic(origin)?,
            Decision::No => self.type_relation_diagnostic(TypeRelation::Satisfies, origin)?,
        };
        let module = self.diagnostic_module(origin);

        self.module_mut(module)?.work.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return whether one pattern term can apply to one solved type term.
    fn pattern_term_can_apply(
        &mut self,
        value: &TypeTerm,
        pattern: &PatternTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<Decision> {
        let decision = match pattern {
            PatternTerm::Wildcard | PatternTerm::Binding { pattern: None, .. } => Decision::Yes,
            PatternTerm::Must { pattern }
            | PatternTerm::BorrowOf { pattern, .. }
            | PatternTerm::MoveOf { pattern, .. }
            | PatternTerm::DereferenceOf { pattern }
            | PatternTerm::Assign { pattern, .. }
            | PatternTerm::Binding {
                pattern: Some(pattern),
                ..
            } => {
                return self.pattern_term_can_apply(value, pattern, origin);
            }
            PatternTerm::Expression { value: expected } => self.decide_type_term_relation(
                TypeRelation::Assignable,
                &TypeTerm::Variable(*expected),
                value,
            )?,
            PatternTerm::Range {
                start,
                end,
                end_kind: _,
            } => {
                let mut decision = Decision::Yes;

                if let Some(start) = start {
                    decision = decision.and(self.decide_type_term_relation(
                        TypeRelation::Assignable,
                        &TypeTerm::Variable(*start),
                        value,
                    )?);
                }

                if let Some(end) = end {
                    decision = decision.and(self.decide_type_term_relation(
                        TypeRelation::Assignable,
                        &TypeTerm::Variable(*end),
                        value,
                    )?);
                }

                decision
            }
            PatternTerm::Type { ty } => self.decide_type_term_relation(
                TypeRelation::Satisfies,
                value,
                &TypeTerm::Variable(*ty),
            )?,
            PatternTerm::Tuple { fields } | PatternTerm::Sequence { fields } => {
                self.pattern_fields_can_apply(value, fields, origin)?
            }
            PatternTerm::Object { fields } => {
                self.pattern_fields_can_apply(value, fields, origin)?
            }
            PatternTerm::TaggedTuple { ty, fields } | PatternTerm::TaggedObject { ty, fields } => {
                let tag = self.decide_type_term_relation(
                    TypeRelation::Satisfies,
                    value,
                    &TypeTerm::Variable(*ty),
                )?;
                let fields = self.pattern_fields_can_apply(value, fields, origin)?;

                tag.and(fields)
            }
            PatternTerm::Union { patterns } => {
                let mut decision = Decision::No;

                for pattern in patterns {
                    decision = decision.or(self.pattern_term_can_apply(value, pattern, origin)?);
                    if decision == Decision::Yes {
                        return Ok(Decision::Yes);
                    }
                }

                decision
            }
        };

        Ok(decision)
    }

    /// Return whether field patterns can apply to one solved type term.
    fn pattern_fields_can_apply(
        &mut self,
        value: &TypeTerm,
        fields: &[PatternFieldTerm],
        origin: ConstraintOrigin,
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        for field in fields {
            decision = decision.and(self.pattern_field_can_apply(value, field, origin)?);
        }

        Ok(decision)
    }

    /// Return whether one field pattern can apply to one solved type term.
    fn pattern_field_can_apply(
        &mut self,
        value: &TypeTerm,
        field: &PatternFieldTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<Decision> {
        let decision = match field {
            PatternFieldTerm::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let module = self.diagnostic_module(origin);
                let Some(ty) = self.member_type_term(module, value, key)? else {
                    return Ok(Decision::No);
                };

                self.pattern_term_can_apply(&ty, pattern, origin)?
            }
            PatternFieldTerm::Computed { key: _, pattern }
            | PatternFieldTerm::Positional { pattern } => {
                self.pattern_term_can_apply(value, pattern, origin)?
            }
            PatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.pattern_term_can_apply(value, pattern, origin)?
                } else {
                    Decision::Yes
                }
            }
            PatternFieldTerm::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Return whether one place is writable.
    fn is_writable_place(&self, place: Place) -> CompilerResult<bool> {
        let is_writable = match place.target {
            PlaceTarget::Binding { symbol } => self.is_writable_binding(symbol)?,
            PlaceTarget::Member { owner, key } => self.is_writable_member(owner, key)?,
            PlaceTarget::Index { .. } => true,
            PlaceTarget::Dereference { .. } => true,
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

        Ok(true)
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

    /// Report one type term that could not produce its result.
    fn report_type_term_failure(
        &mut self,
        term: &TypeTerm,
        origin: ConstraintOrigin,
    ) -> CompilerResult<()> {
        let diagnostic = match term {
            TypeTerm::Member(member) => self.missing_member_diagnostic(origin, member.key)?,
            TypeTerm::Call(call) => self.call_failure_diagnostic(call.source, origin)?,
            TypeTerm::Construct(construct) => {
                self.construct_failure_diagnostic(construct.source, origin)?
            }
            TypeTerm::Operator(operator) => self.operator_failure_diagnostic(operator, origin)?,
            TypeTerm::Index(index) => self.call_failure_diagnostic(index.source, origin)?,
            TypeTerm::IndexWrite(write) => self.call_failure_diagnostic(write.source, origin)?,
            TypeTerm::KeyMembership(membership) => {
                self.type_diagnostic(ConstraintOrigin::Node(membership.source))?
            }
            TypeTerm::InstanceCheck(instance) => {
                self.type_diagnostic(ConstraintOrigin::Node(instance.source))?
            }
            TypeTerm::Identity(identity) => {
                self.type_diagnostic(ConstraintOrigin::Node(identity.source))?
            }
            TypeTerm::Template(template) => {
                self.type_diagnostic(ConstraintOrigin::Node(template.source))?
            }
            TypeTerm::TaggedTemplate(template) => {
                self.type_diagnostic(ConstraintOrigin::Node(template.source))?
            }
            _ => self.type_diagnostic(origin)?,
        };
        let module = self.diagnostic_module(origin);

        self.module_mut(module)?.work.diagnostics.push(diagnostic);

        Ok(())
    }

    /// Return the diagnostic for one failed call.
    fn call_failure_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        origin: ConstraintOrigin,
    ) -> CompilerResult<CheckError> {
        let outcome = self.module(source.module_id)?.decisions.call.get(&source);

        match outcome {
            Some(CallOutcome::Rejected(CallFailure::NotCallable)) => {
                self.not_callable_diagnostic(origin)
            }
            Some(CallOutcome::Rejected(CallFailure::NoMatch)) => {
                self.no_matching_call_diagnostic(origin)
            }
            Some(CallOutcome::Rejected(CallFailure::ArgumentType {
                argument,
                parameter: _,
            })) => {
                let origin = self.variable_diagnostic_origin(*argument)?;

                self.type_relation_diagnostic(TypeRelation::Assignable, origin)
            }
            Some(CallOutcome::Resolved(_)) => Err(CompilerError::Internal {
                message: format!(
                    "resolved call {} has unsolved result type",
                    source.local_id.id
                ),
            }),
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
            Some(ConstructFailure::NotConstructible) => self.not_callable_diagnostic(origin),
            Some(ConstructFailure::NoMatch) => self.no_matching_call_diagnostic(origin),
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
                    self.no_matching_operator_diagnostic(origin, failure.kind)
                }
                OperatorFailureReason::InvalidStrictEquality => {
                    self.invalid_strict_equality_diagnostic(origin)
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
        key: dir::StaticKey,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;
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

    /// Return the module that owns one diagnostic origin.
    fn diagnostic_module(&self, origin: ConstraintOrigin) -> ModuleId {
        match origin {
            ConstraintOrigin::Node(node) => node.module_id,
            ConstraintOrigin::Symbol(symbol) => symbol.module_id,
        }
    }

    /// Return the best diagnostic origin for one variable.
    fn variable_diagnostic_origin(&self, variable: VariableId) -> CompilerResult<ConstraintOrigin> {
        let module = self.module(variable.module)?;
        Ok(module.variable(variable).source)
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

    /// Return a not writable diagnostic.
    fn not_writable_diagnostic(&self, origin: ConstraintOrigin) -> CompilerResult<CheckError> {
        let (module, anchor) = self.anchor(origin)?;

        Ok(CheckError::NotWritable { anchor, module })
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

    /// Return the diagnostic anchor for a constraint origin.
    fn anchor(&self, origin: ConstraintOrigin) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = match origin {
            ConstraintOrigin::Node(node) => node.module_id,
            ConstraintOrigin::Symbol(symbol) => symbol.module_id,
        };
        let check_module = self.module(module)?;
        let source = match origin {
            ConstraintOrigin::Node(node) => node.local_id,
            ConstraintOrigin::Symbol(symbol) => {
                let Some(source) = check_module.symbol_source_node(symbol) else {
                    return Err(CompilerError::Internal {
                        message: format!("check symbol {} has no source node", symbol.local_id.id),
                    });
                };

                source
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
