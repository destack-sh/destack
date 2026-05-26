use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckComponentState, CheckError, CheckModuleState, Obligation, TypeLiteralTerm, TypeTerm,
    VariableId,
};

impl CheckModuleState {
    /// Require one match expression to cover every known value.
    pub(in crate::check) fn require_exhaustive_match(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: VariableId,
        cases: &[dir::LocalNodeId<dir::MatchCase>],
    ) {
        let obligation = Obligation::ExhaustiveMatch {
            source: source.into_global(self.input.module_id),
            value,
            cases: cases.to_vec(),
        };

        self.add_obligation(obligation);
    }
}

impl CheckComponentState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
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
        let (module, anchor) = self.source_anchor(source)?;
        let diagnostic = CheckError::NonExhaustivePattern { anchor, module };

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
}
