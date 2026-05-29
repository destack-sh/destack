use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckError, CheckState, Condition, Decision, MatchCase, Obligation, Origin, TypeLiteralTerm,
    TypeOperand, TypeTerm, VariableId,
};

impl CheckState<'_> {
    /// Require one match expression to cover every known value.
    pub(in crate::check) fn require_exhaustive_match(
        &mut self,
        source: dir::LocalNodeIdAny,
        value: VariableId,
        cases: Vec<MatchCase>,
        condition: Condition,
    ) {
        let obligation = Obligation::ExhaustiveMatch {
            source: source.into_global(value.module),
            value,
            cases,
            condition,
        };

        self.add_obligation(obligation);
    }
}

impl CheckState<'_> {
    /// Check whether one match covers every known selector value.
    pub(in crate::check) fn check_match_exhaustive(
        &mut self,
        source: dir::GlobalNodeIdAny,
        value: VariableId,
        cases: &[MatchCase],
    ) -> CompilerResult<Option<CheckError>> {
        if self.match_has_default_case(cases) {
            return Ok(None);
        }

        let Some(values) = self.match_finite_values(value)? else {
            return Ok(None);
        };

        let mut is_exhaustive = true;

        // require every finite value to be covered
        for value in &values {
            if !self.match_cases_cover_value(
                Origin::Node(source),
                source.module_id,
                cases,
                value,
            )? {
                is_exhaustive = false;

                break;
            }
        }
        if is_exhaustive {
            return Ok(None);
        }

        let (module, anchor) = self.source_anchor(source);
        let diagnostic = CheckError::NonExhaustivePattern { anchor, module };

        Ok(Some(diagnostic))
    }

    /// Return whether a match has a default case.
    fn match_has_default_case(&self, cases: &[MatchCase]) -> bool {
        for case in cases {
            if matches!(case, MatchCase::Default) {
                return true;
            }
        }

        false
    }

    /// Return the finite values of one selector type when known.
    fn match_finite_values(
        &mut self,
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
        &mut self,
        elements: Vec<TypeOperand>,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let mut values = Vec::with_capacity(elements.len());

        // collect literal union members
        for element in elements {
            let Some(TypeTerm::Literal(TypeLiteralTerm::Scalar(literal))) =
                self.type_operand_term(element)?
            else {
                return Ok(None);
            };

            values.push(literal);
        }

        Ok(Some(values))
    }

    /// Return whether match cases cover one scalar literal.
    fn match_cases_cover_value(
        &mut self,
        origin: Origin,
        module: ModuleId,
        cases: &[MatchCase],
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        for case in cases {
            if self.match_case_covers_value(origin, module, case, value)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one match case covers one scalar literal.
    fn match_case_covers_value(
        &mut self,
        origin: Origin,
        module: ModuleId,
        case: &MatchCase,
        value: &dir::ScalarLiteral,
    ) -> CompilerResult<bool> {
        let covers = match case {
            MatchCase::Default => true,
            MatchCase::PatternTerm {
                pattern,
                guard: None,
            } => {
                let value = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

                self.pattern_covers_type(origin, module, *pattern, &value)? == Decision::Yes
            }
            MatchCase::PatternTerm {
                pattern: _,
                guard: Some(_),
            } => false,
        };

        Ok(covers)
    }
}
