use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::solve::Decision;
use crate::check::{
    CheckComponentState, CheckError, DisposeMode, Obligation, StaticTerm, TypeLiteralTerm,
    TypeRelation, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult, DiagnosticAnchor};

impl CheckComponentState<'_> {
    /// Check solved obligations for diagnostics.
    pub(super) fn check_obligations(&mut self) -> CompilerResult<()> {
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
            Obligation::RequireType { source, variable } => {
                self.check_type_required(source, variable)?;
            }
            Obligation::RequireExhaustiveMatch {
                source,
                value,
                cases,
            } => self.check_match_exhaustive(source, value, &cases)?,
            Obligation::RequireTryPropagation {
                source,
                value,
                return_type,
            } => self.check_try_propagates(source, value, return_type)?,
            Obligation::RequirePattern {
                source,
                value,
                pattern,
            } => self.check_pattern_applies(source, value, pattern)?,
            Obligation::RequireDispose {
                source,
                value,
                mode,
            } => self.check_dispose(source, value, mode)?,
            Obligation::RequireWhereClause {
                source,
                left,
                relation,
                right,
            } => self.check_where_clause(source, left, relation, right)?,
            Obligation::RequireStaticCondition { source, condition } => {
                self.check_static_condition(source, condition)?;
            }
            Obligation::RequireFlowCondition { source, condition } => {
                self.check_flow_condition(source, condition)?;
            }
        }

        Ok(())
    }

    /// Check whether one required type was solved.
    fn check_type_required(
        &mut self,
        source: dir::GlobalNodeIdAny,
        variable: VariableId,
    ) -> CompilerResult<()> {
        if self.solved_type_term(variable)?.is_some() {
            return Ok(());
        }
        let diagnostic = self.type_required_diagnostic(source)?;

        self.module_mut(source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Check whether one try expression can propagate through its return type.
    fn check_try_propagates(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        return_type: Option<VariableId>,
    ) -> CompilerResult<()> {
        let Some(return_type) = return_type else {
            let diagnostic = self.invalid_control_flow_diagnostic(
                source,
                "? can only propagate from a function body",
            )?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);

            return Ok(());
        };
        let decision = self.decide_try_propagation(source, value, return_type)?;
        if decision == Decision::No {
            let diagnostic = self.from_failure_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }
        if decision == Decision::Undecidable {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }

        Ok(())
    }

    /// Check whether one pattern can apply to one value.
    fn check_pattern_applies(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<()> {
        let pattern = self
            .module(source.module_id)?
            .input
            .parsed
            .tree
            .get(pattern);
        let decision = self.pattern_applies_to_value(source.module_id, value, pattern)?;
        if decision == Decision::No {
            let diagnostic = self.invalid_pattern_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }
        if decision == Decision::Undecidable {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }

        Ok(())
    }

    /// Return whether one pattern can match one value type.
    fn pattern_applies_to_value(
        &self,
        module: ModuleId,
        value: VariableId,
        pattern: &dir::Pattern,
    ) -> CompilerResult<Decision> {
        let Some(value) = self.solved_type_term(value)? else {
            return Ok(Decision::Undecidable);
        };
        let decision = match pattern {
            dir::Pattern::Wildcard
            | dir::Pattern::Binding {
                name: _,
                pattern: None,
            } => Decision::Yes,
            dir::Pattern::Binding {
                name: _,
                pattern: Some(pattern),
            }
            | dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::MoveOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                let pattern = self.module(module)?.input.parsed.tree.get(*pattern);

                return self.pattern_applies_to_type_term(module, &value, pattern);
            }
            dir::Pattern::Union { patterns } => {
                return self.any_pattern_applies_to_type_term(module, &value, patterns);
            }
            dir::Pattern::Expression { value: _ }
            | dir::Pattern::Range {
                start: _,
                end: _,
                end_kind: _,
            } => {
                if self.type_term_has_finite_scalar_values(&value)? {
                    Decision::Yes
                } else {
                    Decision::Undecidable
                }
            }
            dir::Pattern::Assign { .. }
            | dir::Pattern::TypeExpression { .. }
            | dir::Pattern::Tuple { .. }
            | dir::Pattern::TaggedTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::TaggedObject { .. } => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Return whether one pattern can match one solved type term.
    fn pattern_applies_to_type_term(
        &self,
        module: ModuleId,
        value: &TypeTerm,
        pattern: &dir::Pattern,
    ) -> CompilerResult<Decision> {
        let decision = match pattern {
            dir::Pattern::Wildcard
            | dir::Pattern::Binding {
                name: _,
                pattern: None,
            } => Decision::Yes,
            dir::Pattern::Binding {
                name: _,
                pattern: Some(pattern),
            }
            | dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::MoveOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                let pattern = self.module(module)?.input.parsed.tree.get(*pattern);

                return self.pattern_applies_to_type_term(module, value, pattern);
            }
            dir::Pattern::Union { patterns } => {
                return self.any_pattern_applies_to_type_term(module, value, patterns);
            }
            dir::Pattern::Expression { value: _ }
            | dir::Pattern::Range {
                start: _,
                end: _,
                end_kind: _,
            } => {
                if self.type_term_has_finite_scalar_values(value)? {
                    Decision::Yes
                } else {
                    Decision::Undecidable
                }
            }
            dir::Pattern::Assign { .. }
            | dir::Pattern::TypeExpression { .. }
            | dir::Pattern::Tuple { .. }
            | dir::Pattern::TaggedTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::TaggedObject { .. } => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Return whether any pattern can match one solved type term.
    fn any_pattern_applies_to_type_term(
        &self,
        module: ModuleId,
        value: &TypeTerm,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) -> CompilerResult<Decision> {
        let mut saw_undecidable = false;

        // accept the first valid union branch
        for pattern in patterns {
            let pattern = self.module(module)?.input.parsed.tree.get(*pattern);
            match self.pattern_applies_to_type_term(module, value, pattern)? {
                Decision::Yes => return Ok(Decision::Yes),
                Decision::Undecidable => saw_undecidable = true,
                Decision::No => {}
            }
        }

        if saw_undecidable {
            Ok(Decision::Undecidable)
        } else {
            Ok(Decision::No)
        }
    }

    /// Check whether one resource can be disposed.
    fn check_dispose(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        mode: DisposeMode,
    ) -> CompilerResult<()> {
        let Some(_value) = self.solved_type_term(value)? else {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);

            return Ok(());
        };
        let diagnostic = match mode {
            DisposeMode::Sync => self.does_not_implement_diagnostic(source)?,
            DisposeMode::Async => self.does_not_implement_diagnostic(source)?,
        };

        self.module_mut(source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Check whether one where clause relation holds.
    fn check_where_clause(
        &mut self,
        source: dir::GlobalNodeIdAny,
        left: VariableId,
        relation: TypeRelation,
        right: VariableId,
    ) -> CompilerResult<()> {
        let decision = self.decide_type_relation(relation, left, right)?;
        if decision == Decision::No {
            let diagnostic = self.relation_failure_diagnostic(source, relation)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }
        if decision == Decision::Undecidable {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }

        Ok(())
    }

    /// Check whether one static condition is true.
    fn check_static_condition(
        &mut self,
        source: dir::GlobalNodeIdAny,
        condition: VariableId,
    ) -> CompilerResult<()> {
        let Some(term) = self.solved_static_term(condition)? else {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);

            return Ok(());
        };
        if Self::static_term_is_true(&term) {
            return Ok(());
        }
        let diagnostic = self.invalid_static_condition_diagnostic(source)?;

        self.module_mut(source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Check whether one flow condition is usable as a condition.
    fn check_flow_condition(
        &mut self,
        source: dir::GlobalNodeIdAny,
        condition: VariableId,
    ) -> CompilerResult<()> {
        if self.solved_type_term(condition)?.is_none() {
            let diagnostic = self.type_required_diagnostic(source)?;

            self.module_mut(source.module_id)?
                .work
                .diagnostics
                .push(diagnostic);
        }

        Ok(())
    }

    /// Return a missing type diagnostic.
    fn type_required_diagnostic(&self, source: dir::GlobalNodeIdAny) -> CompilerResult<CheckError> {
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
        let anchor = DiagnosticAnchor::from(span);

        Ok(CheckError::CannotInferType { anchor, module })
    }

    /// Return an invalid control flow diagnostic.
    fn invalid_control_flow_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        message: &str,
    ) -> CompilerResult<CheckError> {
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

        Ok(CheckError::InvalidControlFlow {
            anchor: DiagnosticAnchor::from(span),
            module,
            message: message.to_owned(),
        })
    }

    /// Return an invalid propagated failure diagnostic.
    fn from_failure_diagnostic(&self, source: dir::GlobalNodeIdAny) -> CompilerResult<CheckError> {
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

        Ok(CheckError::DoesNotImplement {
            anchor: DiagnosticAnchor::from(span),
            module,
        })
    }

    /// Return a failed relation diagnostic.
    fn relation_failure_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
        relation: TypeRelation,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.source_anchor(source)?;
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

    /// Return an invalid static condition diagnostic.
    fn invalid_static_condition_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.source_anchor(source)?;

        Ok(CheckError::InvalidStaticCondition { anchor, module })
    }

    /// Return an invalid pattern diagnostic.
    fn invalid_pattern_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.source_anchor(source)?;

        Ok(CheckError::NonExhaustivePattern { anchor, module })
    }

    /// Return a missing implementation diagnostic.
    fn does_not_implement_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.source_anchor(source)?;

        Ok(CheckError::DoesNotImplement { anchor, module })
    }

    /// Return whether one solved static term is the boolean value true.
    fn static_term_is_true(term: &StaticTerm) -> bool {
        matches!(
            term,
            StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                value: dir::ScalarLiteral::Boolean(true),
            })
        )
    }

    /// Return the diagnostic anchor for one source node.
    fn source_anchor(
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

    /// Check whether one match covers every known selector value.
    fn check_match_exhaustive(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<()> {
        if self.match_has_irrefutable_case(source.module_id, cases)? {
            return Ok(());
        }
        let Some(values) = self.match_finite_values(value)? else {
            return Ok(());
        };
        let mut is_exhaustive = true;

        // require every finite value to be covered
        for value in &values {
            if !self.match_cases_cover_value(source.module_id, cases, value)? {
                is_exhaustive = false;

                break;
            }
        }
        if is_exhaustive {
            return Ok(());
        }
        let diagnostic = self.non_exhaustive_match_diagnostic(source)?;

        self.module_mut(source.module_id)?
            .work
            .diagnostics
            .push(diagnostic);

        Ok(())
    }

    /// Return whether a match has an unguarded irrefutable case.
    fn match_has_irrefutable_case(
        &self,
        module: ModuleId,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) -> CompilerResult<bool> {
        for case in cases {
            let case = self.module(module)?.input.parsed.tree.get(*case);
            let selector = case.selector();

            if self.match_selector_is_irrefutable(module, selector)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one match selector covers every remaining value.
    fn match_selector_is_irrefutable(
        &self,
        module: ModuleId,
        selector: &dir::MatchSelector,
    ) -> CompilerResult<bool> {
        let is_irrefutable = match selector {
            dir::MatchSelector::Default => true,
            dir::MatchSelector::Pattern {
                pattern,
                guard: None,
            } => self.pattern_is_irrefutable(module, *pattern)?,
            dir::MatchSelector::Pattern {
                pattern: _,
                guard: Some(_),
            } => false,
        };

        Ok(is_irrefutable)
    }

    /// Return whether one pattern covers every remaining value.
    fn pattern_is_irrefutable(
        &self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> CompilerResult<bool> {
        let pattern = self.module(module)?.input.parsed.tree.get(pattern);
        let is_irrefutable = match pattern {
            dir::Pattern::Wildcard => true,
            dir::Pattern::Binding {
                name: _,
                pattern: None,
            } => true,
            dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::MoveOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.pattern_is_irrefutable(module, *pattern)?
            }
            dir::Pattern::Union { patterns } => self.any_pattern_irrefutable(module, patterns)?,
            dir::Pattern::Binding {
                name: _,
                pattern: Some(_),
            }
            | dir::Pattern::Assign { .. }
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. }
            | dir::Pattern::TypeExpression { .. }
            | dir::Pattern::Tuple { .. }
            | dir::Pattern::TaggedTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::TaggedObject { .. } => false,
        };

        Ok(is_irrefutable)
    }

    /// Return whether any pattern is irrefutable.
    fn any_pattern_irrefutable(
        &self,
        module: ModuleId,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
    ) -> CompilerResult<bool> {
        for pattern in patterns {
            if self.pattern_is_irrefutable(module, *pattern)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the finite values of one selector type when known.
    fn match_finite_values(
        &self,
        value: VariableId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(None);
        };
        let values = match term {
            TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::Boolean)) => {
                vec![
                    dir::ScalarLiteral::Boolean(false),
                    dir::ScalarLiteral::Boolean(true),
                ]
            }
            TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) => vec![literal],
            TypeTerm::Union { elements } => {
                let Some(values) = self.literal_union_values(elements)? else {
                    return Ok(None);
                };

                values
            }
            _ => return Ok(None),
        };

        Ok(Some(values))
    }

    /// Return whether one type term has known finite scalar values.
    fn type_term_has_finite_scalar_values(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let has_values = match term {
            TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::Boolean)) => true,
            TypeTerm::Literal(TypeLiteralTerm::Scalar(_)) => true,
            TypeTerm::Union { elements } => self.literal_union_values(elements.clone())?.is_some(),
            _ => false,
        };

        Ok(has_values)
    }

    /// Return literal values from one solved union.
    fn literal_union_values(
        &self,
        elements: Vec<VariableId>,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let mut values = Vec::with_capacity(elements.len());

        // collect literal union members
        for element in elements {
            let Some(TypeTerm::Literal(TypeLiteralTerm::Scalar(literal))) =
                self.solved_type_term(element)?
            else {
                return Ok(None);
            };

            values.push(literal);
        }

        Ok(Some(values))
    }

    /// Return whether match cases cover one scalar literal.
    fn match_cases_cover_value(
        &self,
        module: ModuleId,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        for case in cases {
            let case = self.module(module)?.input.parsed.tree.get(*case);
            let selector = case.selector();

            if self.match_selector_covers_value(module, selector, value)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one selector covers one scalar literal.
    fn match_selector_covers_value(
        &self,
        module: ModuleId,
        selector: &dir::MatchSelector,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        let covers = match selector {
            dir::MatchSelector::Default => true,
            dir::MatchSelector::Pattern {
                pattern,
                guard: None,
            } => self.pattern_covers_value(module, *pattern, value)?,
            dir::MatchSelector::Pattern {
                pattern: _,
                guard: Some(_),
            } => false,
        };

        Ok(covers)
    }

    /// Return whether one pattern covers one scalar literal.
    fn pattern_covers_value(
        &self,
        module: ModuleId,
        pattern: dir::LocalNodeId<dir::Pattern>,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        let pattern = self.module(module)?.input.parsed.tree.get(pattern);
        let covers = match pattern {
            dir::Pattern::Wildcard
            | dir::Pattern::Binding {
                name: _,
                pattern: None,
            } => true,
            dir::Pattern::Expression { value: expression } => {
                self.expression_matches_literal(module, *expression, value)?
            }
            dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::MoveOf {
                mutability: _,
                right: pattern,
            }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.pattern_covers_value(module, *pattern, value)?
            }
            dir::Pattern::Union { patterns } => {
                self.any_pattern_covers_value(module, patterns, value)?
            }
            dir::Pattern::Binding {
                name: _,
                pattern: Some(_),
            }
            | dir::Pattern::Assign { .. }
            | dir::Pattern::Range { .. }
            | dir::Pattern::TypeExpression { .. }
            | dir::Pattern::Tuple { .. }
            | dir::Pattern::TaggedTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::TaggedObject { .. } => false,
        };

        Ok(covers)
    }

    /// Return whether any pattern covers one scalar literal.
    fn any_pattern_covers_value(
        &self,
        module: ModuleId,
        patterns: &[dir::LocalNodeId<dir::Pattern>],
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        for pattern in patterns {
            if self.pattern_covers_value(module, *pattern, value)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one expression is the given scalar literal.
    fn expression_matches_literal(
        &self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        let expression = self.module(module)?.input.parsed.tree.get(expression);
        let matches = match expression {
            dir::Expression::ScalarLiteral(literal) => literal == value,
            dir::Expression::Parenthesized { expression } => {
                return self.expression_matches_literal(module, *expression, value);
            }
            _ => false,
        };

        Ok(matches)
    }

    /// Return a non-exhaustive match diagnostic.
    fn non_exhaustive_match_diagnostic(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<CheckError> {
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
        let anchor = DiagnosticAnchor::from(span);

        Ok(CheckError::NonExhaustivePattern { anchor, module })
    }
}
