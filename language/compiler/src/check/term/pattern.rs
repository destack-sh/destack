use smallvec::SmallVec;

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, MatchCase, Origin, PatternRelation, Progress, TermId, TypeLiteralTerm,
    TypeOperand, TypeRelation, TypeTerm, VariableId,
};

/// Pattern relation term with source ownership.
///
/// Examples:
/// ```ds
/// const { name } = user
/// match (value) { Some(item) => item }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternTerm {
    /// The source that induced this pattern term.
    pub(in crate::check) source: PatternSource,
    /// The pattern operation.
    pub(in crate::check) target: PatternTarget,
}

impl PatternTerm {
    /// Create one source-backed pattern term.
    pub(in crate::check) fn node(source: dir::GlobalNodeIdAny, target: PatternTarget) -> Self {
        Self {
            source: PatternSource::Node(source),
            target,
        }
    }

    /// Create one synthetic pattern term.
    pub(in crate::check) fn synthetic(origin: Origin, target: PatternTarget) -> Self {
        Self {
            source: PatternSource::Synthetic(origin),
            target,
        }
    }
}

/// Source that induced one pattern term.
///
/// Examples:
/// ```ds
/// const value = input
/// const [head, ...tail] = input
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PatternSource {
    /// Pattern came from one DIR node.
    ///
    /// Examples:
    /// ```ds
    /// const value = input
    /// ```
    Node(dir::GlobalNodeIdAny),
    /// Pattern was synthesized by check.
    ///
    /// Examples:
    /// ```ds
    /// const [head, ...tail] = input
    /// ```
    Synthetic(Origin),
}

/// Pattern operation selected from source syntax.
///
/// Examples:
/// ```ds
/// _
/// Some(value)
/// { name }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternTarget {
    /// Wildcard pattern.
    ///
    /// Examples:
    /// ```ds
    /// _
    /// ```
    Wildcard,
    /// Required value pattern.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// ```
    Must {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Defaulted pattern.
    ///
    /// Examples:
    /// ```ds
    /// value = fallback
    /// ```
    Assign {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
        /// The default value type.
        value: TypeOperand,
    },
    /// Borrow pattern.
    ///
    /// Examples:
    /// ```ds
    /// &value
    /// ```
    BorrowOf {
        /// The borrow mutability.
        mutability: Option<dir::Mutability>,
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Move pattern.
    ///
    /// Examples:
    /// ```ds
    /// ^value
    /// ```
    MoveOf {
        /// The move mutability.
        mutability: Option<dir::Mutability>,
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Dereference pattern.
    ///
    /// Examples:
    /// ```ds
    /// *value
    /// ```
    DereferenceOf {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Binding pattern.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// Some(value)
    /// ```
    Binding {
        /// The bound symbol.
        symbol: Option<dir::GlobalSymbolId>,
        /// The nested pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Value expression pattern.
    ///
    /// Examples:
    /// ```ds
    /// 1
    /// "ready"
    /// ```
    Expression {
        /// The expression value type.
        value: TypeOperand,
    },
    /// Range pattern.
    ///
    /// Examples:
    /// ```ds
    /// 0..10
    /// 0..=10
    /// ```
    Range {
        /// The start value type.
        start: Option<TypeOperand>,
        /// The end value type.
        end: Option<TypeOperand>,
        /// The range end kind.
        end_kind: dir::RangeEnd,
    },
    /// Type expression pattern.
    ///
    /// Examples:
    /// ```ds
    /// value is Ready
    /// ```
    Type {
        /// The matched type.
        ty: TypeOperand,
    },
    /// Tuple pattern.
    ///
    /// Examples:
    /// ```ds
    /// [left, right]
    /// ```
    Tuple {
        /// The tuple fields.
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Newtype wrapper pattern.
    ///
    /// Examples:
    /// ```ds
    /// UserId(raw)
    /// ```
    Newtype {
        /// The tag type.
        ty: TypeOperand,
        /// The tuple fields.
        fields: SmallVec<[PatternField; 3]>,
    },
    /// Sequence pattern.
    ///
    /// Examples:
    /// ```ds
    /// [head, ...tail]
    /// ```
    Sequence {
        /// The sequence fields.
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Object pattern.
    ///
    /// Examples:
    /// ```ds
    /// { name, age }
    /// ```
    Object {
        /// The object fields.
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Nominal object pattern.
    ///
    /// Examples:
    /// ```ds
    /// Point { x, y }
    /// ```
    NominalObject {
        /// The tag type.
        ty: TypeOperand,
        /// The object fields.
        fields: SmallVec<[PatternField; 3]>,
    },
    /// Union pattern.
    ///
    /// Examples:
    /// ```ds
    /// Some(value) | Ok(value)
    /// ```
    Union {
        /// The alternative patterns.
        patterns: Vec<TermId<PatternTerm>>,
    },
}

/// One field inside a pattern.
///
/// Examples:
/// ```ds
/// { name }
/// [head, ...tail]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternField {
    /// Named field.
    ///
    /// Examples:
    /// ```ds
    /// { name }
    /// ```
    Named {
        /// The selected key.
        key: dir::StaticKey,
        /// The resolved field type.
        value: Option<VariableId>,
        /// The field pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Computed field.
    ///
    /// Examples:
    /// ```ds
    /// { [key]: value }
    /// ```
    Computed {
        /// The computed key type.
        key: TypeOperand,
        /// The field pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Positional field.
    ///
    /// Examples:
    /// ```ds
    /// [head]
    /// ```
    Positional {
        /// The field pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Spread field.
    ///
    /// Examples:
    /// ```ds
    /// [...rest]
    /// ```
    Spread {
        /// The spread pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Elided field.
    ///
    /// Examples:
    /// ```ds
    /// [, second]
    /// ```
    Elision,
}

/// Assignment target pattern checked against an assigned value type.
///
/// Examples:
/// ```ds
/// target = value
/// [head, ...tail] = values
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum AssignPatternTerm {
    /// Expression assignment target.
    ///
    /// Examples:
    /// ```ds
    /// target = value
    /// ```
    Expression {
        /// The target storage type.
        target: TypeOperand,
    },
    /// Defaulted assignment target.
    ///
    /// Examples:
    /// ```ds
    /// target = fallback
    /// ```
    Assign {
        /// The target pattern.
        pattern: TermId<AssignPatternTerm>,
        /// The default value type.
        value: TypeOperand,
    },
    /// Sequence destructuring target.
    ///
    /// Examples:
    /// ```ds
    /// [head, ...tail] = values
    /// ```
    Sequence {
        /// The sequence fields.
        fields: SmallVec<[AssignPatternField; 8]>,
    },
    /// Object destructuring target.
    ///
    /// Examples:
    /// ```ds
    /// { name } = user
    /// ```
    Object {
        /// The object fields.
        fields: SmallVec<[AssignPatternField; 8]>,
    },
}

/// One field inside an assignment target pattern.
///
/// Examples:
/// ```ds
/// { name } = user
/// [head, ...tail] = values
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum AssignPatternField {
    /// Named field.
    ///
    /// Examples:
    /// ```ds
    /// { name } = user
    /// ```
    Named {
        /// The selected key.
        key: dir::StaticKey,
        /// The resolved field type.
        value: Option<VariableId>,
        /// The field target pattern.
        pattern: Option<TermId<AssignPatternTerm>>,
    },
    /// Computed field.
    ///
    /// Examples:
    /// ```ds
    /// { [key]: target } = user
    /// ```
    Computed {
        /// The computed key type.
        key: TypeOperand,
        /// The field target pattern.
        pattern: TermId<AssignPatternTerm>,
    },
    /// Positional field.
    ///
    /// Examples:
    /// ```ds
    /// [head] = values
    /// ```
    Positional {
        /// The field target pattern.
        pattern: TermId<AssignPatternTerm>,
    },
    /// Spread field.
    ///
    /// Examples:
    /// ```ds
    /// [...rest] = values
    /// ```
    Spread {
        /// The spread target pattern.
        pattern: Option<TermId<AssignPatternTerm>>,
    },
    /// Elided field.
    ///
    /// Examples:
    /// ```ds
    /// [, second] = values
    /// ```
    Elision,
}

impl CheckState<'_> {
    /// Decide whether one pattern relation holds.
    pub(in crate::check) fn reduce_pattern_relation(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        relation: &PatternRelation,
    ) -> CompilerResult<Decision> {
        let module = origin.module();
        let Some(value) = self.type_operand_term(value)? else {
            return Ok(Decision::Undecidable);
        };
        let decision = match relation {
            PatternRelation::Match(pattern) => {
                self.reduce_pattern_term(origin, module, &value, *pattern)?
            }
            PatternRelation::Assign(pattern) => {
                self.reduce_assign_pattern_term(origin, module, &value, *pattern)?
            }
        };

        Ok(decision)
    }

    /// Decide whether one pattern can match one value type.
    fn reduce_pattern_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        pattern: TermId<PatternTerm>,
    ) -> CompilerResult<Decision> {
        let pattern = self.inference.term(pattern).clone();
        let decision = match pattern.target {
            PatternTarget::Wildcard | PatternTarget::Binding { pattern: None, .. } => Decision::Yes,
            PatternTarget::Must { pattern }
            | PatternTarget::BorrowOf { pattern, .. }
            | PatternTarget::MoveOf { pattern, .. }
            | PatternTarget::DereferenceOf { pattern }
            | PatternTarget::Assign { pattern, .. }
            | PatternTarget::Binding {
                pattern: Some(pattern),
                ..
            } => return self.reduce_pattern_term(origin, module, value, pattern),
            PatternTarget::Expression { value: expected } => {
                self.decide_pattern_operand_relation(TypeRelation::Assignable, expected, value)?
            }
            PatternTarget::Range {
                start,
                end,
                end_kind: _,
            } => {
                let mut decision = Decision::Yes;

                // check lower bound
                if let Some(start) = start {
                    decision = decision.and(self.decide_pattern_operand_relation(
                        TypeRelation::Assignable,
                        start,
                        value,
                    )?);
                }

                // check upper bound
                if let Some(end) = end {
                    decision = decision.and(self.decide_pattern_operand_relation(
                        TypeRelation::Assignable,
                        end,
                        value,
                    )?);
                }

                decision
            }
            PatternTarget::Type { ty } => {
                self.decide_pattern_term_operand_relation(TypeRelation::Satisfies, value, ty)?
            }
            PatternTarget::Tuple { fields }
            | PatternTarget::Sequence { fields }
            | PatternTarget::Object { fields } => {
                self.reduce_pattern_fields(origin, module, value, &fields)?
            }
            PatternTarget::Newtype { ty, fields } | PatternTarget::NominalObject { ty, fields } => {
                let tag =
                    self.decide_pattern_term_operand_relation(TypeRelation::Satisfies, value, ty)?;
                let fields = self.reduce_pattern_fields(origin, module, value, &fields)?;

                tag.and(fields)
            }
            PatternTarget::Union { patterns } => {
                let mut decision = Decision::No;

                // accept any matching pattern alternative
                for pattern in patterns {
                    decision =
                        decision.or(self.reduce_pattern_term(origin, module, value, pattern)?);
                    if decision == Decision::Yes {
                        return Ok(Decision::Yes);
                    }
                }

                decision
            }
        };

        Ok(decision)
    }

    /// Decide whether pattern fields can match one value type.
    fn reduce_pattern_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        fields: &[PatternField],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // every requested field must match
        for field in fields {
            decision =
                decision.and(self.reduce_pattern_field(origin, module, value, field.clone())?);
        }

        Ok(decision)
    }

    /// Decide whether one pattern field can match one value type.
    fn reduce_pattern_field(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        field: PatternField,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            PatternField::Named {
                key,
                value: _,
                pattern,
            } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(ty) = self.resolve_member_type(origin, module, value, &key, &[])? else {
                    return Ok(Decision::No);
                };

                self.reduce_pattern_term(origin, module, &ty, pattern)?
            }
            PatternField::Computed { key: _, pattern } | PatternField::Positional { pattern } => {
                self.reduce_pattern_term(origin, module, value, pattern)?
            }
            PatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.reduce_pattern_term(origin, module, value, pattern)?
                } else {
                    Decision::Yes
                }
            }
            PatternField::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Decide whether match cases cover every value in one operand type.
    pub(in crate::check) fn decide_match_cases_cover_operand(
        &mut self,
        origin: Origin,
        cases: &[MatchCase],
        value: TypeOperand,
    ) -> CompilerResult<Decision> {
        let mut patterns = Vec::new();

        // collect unguarded covering patterns
        for case in cases {
            match case {
                MatchCase::Default => return Ok(Decision::Yes),
                MatchCase::PatternTerm {
                    pattern,
                    guard: None,
                } => patterns.push(*pattern),
                MatchCase::PatternTerm {
                    pattern: _,
                    guard: Some(_),
                } => {}
            }
        }

        // reject empty active case sets
        if patterns.is_empty() {
            return Ok(Decision::No);
        }

        // decide coverage as one union pattern
        let pattern = if let [pattern] = patterns.as_slice() {
            *pattern
        } else {
            self.inference.push_term(PatternTerm::synthetic(
                origin,
                PatternTarget::Union { patterns },
            ))
        };

        self.decide_pattern_covers_operand(origin, pattern, value)
    }

    /// Decide whether one pattern covers every value in one operand type.
    pub(in crate::check) fn decide_pattern_covers_operand(
        &mut self,
        origin: Origin,
        pattern: TermId<PatternTerm>,
        value: TypeOperand,
    ) -> CompilerResult<Decision> {
        let module = origin.module();
        let Some(value) = self.type_operand_term(value)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_pattern_covers_type(origin, module, pattern, &value)
    }

    /// Decide whether one pattern covers every value in one type term.
    pub(in crate::check) fn decide_pattern_covers_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        pattern: TermId<PatternTerm>,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        if let TypeTerm::Union { elements } = value {
            return self.decide_pattern_covers_union(origin, pattern, elements);
        }
        if let Some(values) = finite_scalar_domain(value) {
            return self.decide_pattern_covers_scalars(origin, module, pattern, &values);
        }

        let pattern = self.inference.term(pattern).clone();
        let decision = match pattern.target {
            PatternTarget::Wildcard | PatternTarget::Binding { pattern: None, .. } => Decision::Yes,
            PatternTarget::Must { pattern }
            | PatternTarget::BorrowOf { pattern, .. }
            | PatternTarget::MoveOf { pattern, .. }
            | PatternTarget::DereferenceOf { pattern }
            | PatternTarget::Assign { pattern, .. }
            | PatternTarget::Binding {
                pattern: Some(pattern),
                ..
            } => {
                return self.decide_pattern_covers_type(origin, module, pattern, value);
            }
            PatternTarget::Expression { value: expected } => {
                self.decide_expression_pattern_covers_type(expected, value)?
            }
            PatternTarget::Range {
                start,
                end,
                end_kind,
            } => self.decide_range_pattern_covers_type(start, end, end_kind, value)?,
            PatternTarget::Type { ty } => {
                self.decide_pattern_term_operand_relation(TypeRelation::Assignable, value, ty)?
            }
            PatternTarget::Tuple { fields }
            | PatternTarget::Sequence { fields }
            | PatternTarget::Object { fields } => {
                self.decide_pattern_fields_cover_type(origin, module, &fields, value)?
            }
            PatternTarget::Newtype { ty, fields } | PatternTarget::NominalObject { ty, fields } => {
                let tag =
                    self.decide_pattern_term_operand_relation(TypeRelation::Assignable, value, ty)?;
                let fields =
                    self.decide_pattern_fields_cover_type(origin, module, &fields, value)?;

                tag.and(fields)
            }
            PatternTarget::Union { patterns } => {
                self.decide_patterns_cover_type(origin, module, &patterns, value)?
            }
        };

        Ok(decision)
    }

    /// Decide whether one pattern covers every scalar value listed.
    fn decide_pattern_covers_scalars(
        &mut self,
        origin: Origin,
        module: ModuleId,
        pattern: TermId<PatternTerm>,
        values: &[dir::ScalarLiteral],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // require every scalar value to be covered
        for value in values {
            let value = TypeTerm::Literal(TypeLiteralTerm::Scalar(value.clone()));

            decision =
                decision.and(self.decide_pattern_covers_type(origin, module, pattern, &value)?);
        }

        Ok(decision)
    }

    /// Decide whether one pattern covers every member in a union.
    fn decide_pattern_covers_union(
        &mut self,
        origin: Origin,
        pattern: TermId<PatternTerm>,
        elements: &[TypeOperand],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // require every union member to be covered
        for element in elements {
            decision = decision.and(self.decide_pattern_covers_operand(origin, pattern, *element)?);
        }

        Ok(decision)
    }

    /// Decide whether pattern alternatives cover every value in one type.
    fn decide_patterns_cover_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        patterns: &[TermId<PatternTerm>],
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::No;

        // accept any covering pattern alternative
        for pattern in patterns {
            decision =
                decision.or(self.decide_pattern_covers_type(origin, module, *pattern, value)?);
        }

        Ok(decision)
    }

    /// Decide whether one expression pattern covers every value in one type.
    fn decide_expression_pattern_covers_type(
        &mut self,
        expected: TypeOperand,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let Some(expected) = self.type_operand_term(expected)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Assignable, value, &expected)
    }

    /// Decide whether one range pattern covers every value in one type.
    fn decide_range_pattern_covers_type(
        &mut self,
        start: Option<TypeOperand>,
        end: Option<TypeOperand>,
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
        variable: Option<TypeOperand>,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        let Some(variable) = variable else {
            return Ok(None);
        };
        let Some(term) = self.type_operand_term(variable)? else {
            return Ok(None);
        };
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) = term else {
            return Ok(None);
        };

        Ok(Some(literal))
    }

    /// Decide whether field patterns cover every value in one type.
    fn decide_pattern_fields_cover_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        fields: &[PatternField],
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // require every field pattern to cover
        for field in fields {
            decision = decision.and(self.decide_pattern_field_covers_type(
                origin,
                module,
                field.clone(),
                value,
            )?);
        }

        Ok(decision)
    }

    /// Decide whether one field pattern covers every value in one type.
    fn decide_pattern_field_covers_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        field: PatternField,
        value: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            PatternField::Named {
                key,
                value: _,
                pattern,
            } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(field) = self.resolve_member_type(origin, module, value, &key, &[])?
                else {
                    return Ok(Decision::No);
                };

                self.decide_pattern_covers_type(origin, module, pattern, &field)?
            }
            PatternField::Computed { pattern, .. } | PatternField::Positional { pattern } => {
                self.decide_pattern_covers_type(origin, module, pattern, value)?
            }
            PatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.decide_pattern_covers_type(origin, module, pattern, value)?
                } else {
                    Decision::Yes
                }
            }
            PatternField::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Decide whether one assignment pattern can accept one value type.
    fn reduce_assign_pattern_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        pattern: TermId<AssignPatternTerm>,
    ) -> CompilerResult<Decision> {
        let pattern = self.inference.term(pattern).clone();
        let decision = match pattern {
            AssignPatternTerm::Expression { target } => {
                self.decide_pattern_term_operand_relation(TypeRelation::Assignable, value, target)?
            }
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default =
                    self.decide_pattern_operand_relation(TypeRelation::Assignable, default, value)?;
                let pattern = self.reduce_assign_pattern_term(origin, module, value, pattern)?;

                default.and(pattern)
            }
            AssignPatternTerm::Sequence { fields } | AssignPatternTerm::Object { fields } => {
                self.reduce_assign_pattern_fields(origin, module, value, &fields)?
            }
        };

        Ok(decision)
    }

    /// Decide whether assignment pattern fields can accept one value type.
    fn reduce_assign_pattern_fields(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        fields: &[AssignPatternField],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // every assignment field must accept the source value
        for field in fields {
            decision = decision.and(self.reduce_assign_pattern_field(
                origin,
                module,
                value,
                field.clone(),
            )?);
        }

        Ok(decision)
    }

    /// Decide whether one assignment pattern field can accept one value type.
    fn reduce_assign_pattern_field(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: &TypeTerm,
        field: AssignPatternField,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            AssignPatternField::Named {
                key,
                value: _,
                pattern,
            } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(ty) = self.resolve_member_type(origin, module, value, &key, &[])? else {
                    return Ok(Decision::No);
                };

                self.reduce_assign_pattern_term(origin, module, &ty, pattern)?
            }
            AssignPatternField::Computed { key: _, pattern }
            | AssignPatternField::Positional { pattern } => {
                self.reduce_assign_pattern_term(origin, module, value, pattern)?
            }
            AssignPatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.reduce_assign_pattern_term(origin, module, value, pattern)?
                } else {
                    Decision::Yes
                }
            }
            AssignPatternField::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Expect a pattern to match one value type.
    pub(in crate::check) fn expect_pattern_term(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        pattern: TermId<PatternTerm>,
    ) -> CompilerResult<Progress> {
        let module = origin.module();
        let pattern = self.inference.term(pattern).clone();
        let progress = match pattern.target {
            PatternTarget::Wildcard => Progress::Unchanged,
            PatternTarget::Must { pattern }
            | PatternTarget::BorrowOf { pattern, .. }
            | PatternTarget::MoveOf { pattern, .. }
            | PatternTarget::DereferenceOf { pattern } => {
                self.expect_pattern_term(origin, value, pattern)?
            }
            PatternTarget::Assign {
                pattern,
                value: default,
            } => {
                let default =
                    self.relate_type_relation(origin, TypeRelation::Assignable, default, value)?;
                let pattern = self.expect_pattern_term(origin, value, pattern)?;

                default.merge(pattern)
            }
            PatternTarget::Binding { symbol, pattern } => {
                let binding = if let Some(symbol) = symbol {
                    let binding = self.import_symbol_type_operand(module, symbol);

                    self.relate_type_relation(origin, TypeRelation::Equal, binding, value)?
                } else {
                    Progress::Unchanged
                };
                let pattern = if let Some(pattern) = pattern {
                    self.expect_pattern_term(origin, value, pattern)?
                } else {
                    Progress::Unchanged
                };

                binding.merge(pattern)
            }
            PatternTarget::Expression { value: expected } => {
                self.relate_type_relation(origin, TypeRelation::Assignable, expected, value)?
            }
            PatternTarget::Range {
                start,
                end,
                end_kind: _,
            } => {
                let start = if let Some(start) = start {
                    self.relate_type_relation(origin, TypeRelation::Assignable, start, value)?
                } else {
                    Progress::Unchanged
                };
                let end = if let Some(end) = end {
                    self.relate_type_relation(origin, TypeRelation::Assignable, end, value)?
                } else {
                    Progress::Unchanged
                };

                start.merge(end)
            }
            PatternTarget::Type { ty } => {
                self.relate_type_relation(origin, TypeRelation::Satisfies, value, ty)?
            }
            PatternTarget::Tuple { fields } | PatternTarget::Sequence { fields } => {
                self.expect_pattern_field_terms(origin, value, &fields)?
            }
            PatternTarget::Object { fields } => {
                self.expect_pattern_field_terms(origin, value, &fields)?
            }
            PatternTarget::Newtype { ty, fields } | PatternTarget::NominalObject { ty, fields } => {
                let tag = self.relate_type_relation(origin, TypeRelation::Satisfies, value, ty)?;
                let fields = self.expect_pattern_field_terms(origin, value, &fields)?;

                tag.merge(fields)
            }
            PatternTarget::Union { patterns } => {
                let mut progress = Progress::Unchanged;
                for pattern in patterns {
                    progress = progress.merge(self.expect_pattern_term(origin, value, pattern)?);
                }

                progress
            }
        };

        Ok(progress)
    }

    /// Expect pattern fields to match one value type.
    fn expect_pattern_field_terms(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        fields: &[PatternField],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for field in fields {
            progress = progress.merge(self.expect_pattern_field(origin, value, field.clone())?);
        }

        Ok(progress)
    }

    /// Expect one pattern field to match one value type.
    fn expect_pattern_field(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        field: PatternField,
    ) -> CompilerResult<Progress> {
        let field = field;
        let module = origin.module();
        let progress = match field {
            PatternField::Named {
                key,
                value: field,
                pattern,
            } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(field) = field else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) = self.pattern_member_type(origin, module, value, &key)? else {
                    return Ok(Progress::Unchanged);
                };
                let ty = self.inference.push_term(ty);
                let field_type =
                    self.relate_type_equality(origin, TypeOperand::Variable(field), ty)?;
                let pattern =
                    self.expect_pattern_term(origin, TypeOperand::Variable(field), pattern)?;

                field_type.merge(pattern)
            }
            PatternField::Computed { key: _, pattern } | PatternField::Positional { pattern } => {
                self.expect_pattern_term(origin, value, pattern)?
            }
            PatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.expect_pattern_term(origin, value, pattern)?
                } else {
                    Progress::Unchanged
                }
            }
            PatternField::Elision => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect an assignment pattern to accept one value type.
    pub(in crate::check) fn expect_assign_pattern_term(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        pattern: TermId<AssignPatternTerm>,
    ) -> CompilerResult<Progress> {
        let pattern = self.inference.term(pattern).clone();
        let progress = match pattern {
            AssignPatternTerm::Expression { target } => {
                self.relate_type_relation(origin, TypeRelation::Assignable, value, target)?
            }
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default =
                    self.relate_type_relation(origin, TypeRelation::Assignable, default, value)?;
                let pattern = self.expect_assign_pattern_term(origin, value, pattern)?;

                default.merge(pattern)
            }
            AssignPatternTerm::Sequence { fields } | AssignPatternTerm::Object { fields } => {
                self.expect_assign_pattern_field_terms(origin, value, &fields)?
            }
        };

        Ok(progress)
    }

    /// Expect assignment pattern fields to accept one value type.
    fn expect_assign_pattern_field_terms(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        fields: &[AssignPatternField],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for field in fields {
            progress =
                progress.merge(self.expect_assign_pattern_field(origin, value, field.clone())?);
        }

        Ok(progress)
    }

    /// Expect one assignment pattern field to accept one value type.
    fn expect_assign_pattern_field(
        &mut self,
        origin: Origin,
        value: TypeOperand,
        field: AssignPatternField,
    ) -> CompilerResult<Progress> {
        let field = field;
        let module = origin.module();
        let progress = match field {
            AssignPatternField::Named {
                key,
                value: field,
                pattern,
            } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(field) = field else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) = self.pattern_member_type(origin, module, value, &key)? else {
                    return Ok(Progress::Unchanged);
                };
                let ty = self.inference.push_term(ty);
                let field_type =
                    self.relate_type_equality(origin, TypeOperand::Variable(field), ty)?;
                let pattern =
                    self.expect_assign_pattern_term(origin, TypeOperand::Variable(field), pattern)?;

                field_type.merge(pattern)
            }
            AssignPatternField::Computed { key: _, pattern }
            | AssignPatternField::Positional { pattern } => {
                self.expect_assign_pattern_term(origin, value, pattern)?
            }
            AssignPatternField::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.expect_assign_pattern_term(origin, value, pattern)?
                } else {
                    Progress::Unchanged
                }
            }
            AssignPatternField::Elision => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decide one relation from a pattern operand to a solved term.
    fn decide_pattern_operand_relation(
        &self,
        relation: TypeRelation,
        left: TypeOperand,
        right: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let Some(left) = self.type_operand_term(left)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(relation, &left, right)
    }

    /// Decide one relation from a solved term to a pattern operand.
    fn decide_pattern_term_operand_relation(
        &self,
        relation: TypeRelation,
        left: &TypeTerm,
        right: TypeOperand,
    ) -> CompilerResult<Decision> {
        let Some(right) = self.type_operand_term(right)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(relation, left, &right)
    }

    /// Return the solved type term for one pattern operand when available.
    fn optional_pattern_operand_term(
        &self,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };

        Ok(Some(term))
    }

    /// Return one member type selected from a pattern value.
    fn pattern_member_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        value: TypeOperand,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(value) = self.optional_pattern_operand_term(value)? else {
            return Ok(None);
        };

        self.resolve_member_type(origin, module, &value, key, &[])
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

/// Return the finite scalar domain of one primitive type.
fn finite_scalar_domain(value: &TypeTerm) -> Option<Vec<dir::ScalarLiteral>> {
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
