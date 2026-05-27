use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckError, CheckState, Decision, Obligation, PatternField, PatternTerm, TermId,
    TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};

impl CheckState<'_> {
    /// Require one binding pattern to cover its matched value type.
    pub(in crate::check) fn require_irrefutable_pattern(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        pattern: TermId<PatternTerm>,
        value: VariableId,
    ) {
        let obligation = Obligation::IrrefutablePattern {
            source: source.into_global(module),
            pattern,
            value,
            condition: self.active_static_condition(module),
        };

        self.add_obligation(obligation);
    }
}

impl CheckState<'_> {
    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::check) fn check_irrefutable_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        pattern: TermId<PatternTerm>,
        value: VariableId,
    ) -> CompilerResult<()> {
        let decision = self.pattern_covers_variable(pattern, value)?;
        let diagnostic = match decision {
            Decision::Yes => return Ok(()),
            Decision::No => {
                let (module, anchor) = self.source_anchor(source)?;

                CheckError::RefutablePattern { anchor, module }
            }
            Decision::Undecidable => {
                let (module, anchor) = self.source_anchor(source)?;

                CheckError::CannotSolve { anchor, module }
            }
        };

        self.diagnostics_mut(source.module_id).push(diagnostic);

        Ok(())
    }

    /// Return whether one pattern covers every value in one variable type.
    fn pattern_covers_variable(
        &mut self,
        pattern: TermId<PatternTerm>,
        value: VariableId,
    ) -> CompilerResult<Decision> {
        let module = value.module;
        let Some(value) = self.solved_type_term(value)? else {
            return Ok(Decision::Undecidable);
        };

        self.pattern_covers_type(module, pattern, &value)
    }

    /// Return whether one pattern covers every value in one type term.
    pub(in crate::check) fn pattern_covers_type(
        &mut self,
        module: ModuleId,
        pattern: TermId<PatternTerm>,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if let TypeTerm::Union { elements } = value {
            return self.pattern_covers_union(pattern, elements);
        }
        if let Some(values) = finite_scalar_values(value) {
            return self.pattern_covers_scalars(module, pattern, &values);
        }

        let pattern = self.terms.get(pattern).clone();
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
                return self.pattern_covers_type(module, pattern, value);
            }
            PatternTerm::Expression { value: expected } => {
                self.expression_pattern_covers_type(expected, value)?
            }
            PatternTerm::Range {
                start,
                end,
                end_kind,
            } => self.range_pattern_covers_type(start, end, end_kind, value)?,
            PatternTerm::Type { ty } => self.decide_type_term_relation(
                TypeRelation::Assignable,
                value,
                &TypeTerm::Variable(ty),
            )?,
            PatternTerm::Tuple { fields }
            | PatternTerm::Sequence { fields }
            | PatternTerm::Object { fields } => {
                self.pattern_fields_cover_type(module, &fields, value)?
            }
            PatternTerm::TaggedTuple { ty, fields } | PatternTerm::TaggedObject { ty, fields } => {
                let tag = self.decide_type_term_relation(
                    TypeRelation::Assignable,
                    value,
                    &TypeTerm::Variable(ty),
                )?;
                let fields = self.pattern_fields_cover_type(module, &fields, value)?;

                tag.and(fields)
            }
            PatternTerm::Union { patterns } => {
                self.pattern_union_covers_type(module, &patterns, value)?
            }
        };

        Ok(decision)
    }

    /// Return whether one pattern covers every scalar value listed.
    fn pattern_covers_scalars(
        &mut self,
        module: ModuleId,
        pattern: TermId<PatternTerm>,
        values: &[dir::ScalarLiteral],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        for value in values {
            let value = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

            decision = decision.and(self.pattern_covers_type(module, pattern, &value)?);
        }

        Ok(decision)
    }

    /// Return whether one pattern covers every member in a union.
    fn pattern_covers_union(
        &mut self,
        pattern: TermId<PatternTerm>,
        elements: &[TypeOperand],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        for element in elements {
            let TypeOperand::Variable(element) = *element else {
                return Ok(Decision::Undecidable);
            };

            decision = decision.and(self.pattern_covers_variable(pattern, element)?);
        }

        Ok(decision)
    }

    /// Return whether pattern alternatives cover every value in one type.
    fn pattern_union_covers_type(
        &mut self,
        module: ModuleId,
        patterns: &[TermId<PatternTerm>],
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::No;

        for pattern in patterns {
            decision = decision.or(self.pattern_covers_type(module, *pattern, value)?);
        }

        Ok(decision)
    }

    /// Return whether one expression pattern covers every value in one type.
    fn expression_pattern_covers_type(
        &mut self,
        expected: VariableId,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let Some(expected) = self.solved_type_term(expected)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Assignable, value, &expected)
    }

    /// Return whether one range pattern covers every value in one type.
    fn range_pattern_covers_type(
        &mut self,
        start: Option<VariableId>,
        end: Option<VariableId>,
        end_kind: dir::RangeEnd,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(value)) = value else {
            return Ok(Decision::Undecidable);
        };
        let start = self.solved_scalar_literal(start)?;
        let end = self.solved_scalar_literal(end)?;
        let Some(is_in_range) = scalar_in_range(value, start.as_ref(), end.as_ref(), end_kind)
        else {
            return Ok(Decision::Undecidable);
        };

        Ok(if is_in_range {
            Decision::Yes
        } else {
            Decision::No
        })
    }

    /// Return one solved scalar literal.
    fn solved_scalar_literal(
        &self,
        variable: Option<VariableId>,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        let Some(variable) = variable else {
            return Ok(None);
        };
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(None);
        };
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) = term else {
            return Ok(None);
        };

        Ok(Some(literal))
    }

    /// Return whether field patterns cover every value in one type.
    fn pattern_fields_cover_type(
        &mut self,
        module: ModuleId,
        fields: &[PatternField],
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        for field in fields {
            decision =
                decision.and(self.pattern_field_covers_type(module, field.clone(), value)?);
        }

        Ok(decision)
    }

    /// Return whether one field pattern covers every value in one type.
    fn pattern_field_covers_type(
        &mut self,
        module: ModuleId,
        field: PatternField,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            PatternField::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(field) = self.resolve_member_type(module, value, &key, &[])? else {
                    return Ok(Decision::No);
                };

                self.pattern_covers_type(module, pattern, &field)?
            }
            PatternField::Computed { pattern, .. } | PatternField::Positional { pattern } => {
                self.pattern_covers_type(module, pattern, value)?
            }
            PatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.pattern_covers_type(module, pattern, value)?
                } else {
                    Decision::Yes
                }
            }
            PatternField::Elision => Decision::Yes,
        };

        Ok(decision)
    }
}

/// Return whether one scalar literal is inside one scalar range.
fn scalar_in_range(
    value: &dir::ScalarLiteral,
    start: Option<&dir::ScalarLiteral>,
    end: Option<&dir::ScalarLiteral>,
    end_kind: dir::RangeEnd,
) -> Option<bool> {
    let value = scalar_order(value)?;
    let start = match start {
        Some(start) => Some(scalar_order(start)?),
        None => None,
    };
    let end = match end {
        Some(end) => Some(scalar_order(end)?),
        None => None,
    };

    // check lower bound
    if let Some(start) = start
        && value < start
    {
        return Some(false);
    }

    // check upper bound
    if let Some(end) = end {
        let is_above = match end_kind {
            dir::RangeEnd::Open => value >= end,
            dir::RangeEnd::Inclusive => value > end,
        };
        if is_above {
            return Some(false);
        }
    }

    Some(true)
}

/// Return one comparable scalar value.
fn scalar_order(value: &dir::ScalarLiteral) -> Option<f64> {
    let value = match value {
        dir::ScalarLiteral::Integer(value) => *value as f64,
        dir::ScalarLiteral::Bigint(value) => *value as f64,
        dir::ScalarLiteral::Float(value) => *value,
        _ => return None,
    };

    Some(value)
}

/// Return known finite scalar values for one type.
fn finite_scalar_values(value: &TypeTerm) -> Option<Vec<dir::ScalarLiteral>> {
    let values = match value {
        TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::Boolean)) => {
            vec![
                dir::ScalarLiteral::Boolean(false),
                dir::ScalarLiteral::Boolean(true),
            ]
        }
        _ => return None,
    };

    Some(values)
}
