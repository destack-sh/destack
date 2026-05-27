use smallvec::SmallVec;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, PatternRelation, Progress, TermId, TermTable, TypeRelation, TypeTerm,
    VariableId,
};

/// Pattern relation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternTerm {
    /// Wildcard pattern.
    Wildcard,
    /// Required value pattern.
    Must {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Defaulted pattern.
    Assign {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
        /// The default value type.
        value: VariableId,
    },
    /// Borrow pattern.
    BorrowOf {
        /// The borrow mutability.
        mutability: Option<dir::Mutability>,
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Move pattern.
    MoveOf {
        /// The move mutability.
        mutability: Option<dir::Mutability>,
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Dereference pattern.
    DereferenceOf {
        /// The checked pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Binding pattern.
    Binding {
        /// The bound symbol.
        symbol: Option<dir::GlobalSymbolId>,
        /// The nested pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Value expression pattern.
    Expression {
        /// The expression value type.
        value: VariableId,
    },
    /// Range pattern.
    Range {
        /// The start value type.
        start: Option<VariableId>,
        /// The end value type.
        end: Option<VariableId>,
        /// The range end kind.
        end_kind: dir::RangeEnd,
    },
    /// Type expression pattern.
    Type {
        /// The matched type.
        ty: VariableId,
    },
    /// Tuple pattern.
    Tuple {
        /// The tuple fields.
        fields: Vec<TermId<PatternFieldTerm>>,
    },
    /// Tagged tuple pattern.
    TaggedTuple {
        /// The tag type.
        ty: VariableId,
        /// The tuple fields.
        fields: Vec<TermId<PatternFieldTerm>>,
    },
    /// Sequence pattern.
    Sequence {
        /// The sequence fields.
        fields: Vec<TermId<PatternFieldTerm>>,
    },
    /// Object pattern.
    Object {
        /// The object fields.
        fields: Vec<TermId<PatternFieldTerm>>,
    },
    /// Tagged object pattern.
    TaggedObject {
        /// The tag type.
        ty: VariableId,
        /// The object fields.
        fields: Vec<TermId<PatternFieldTerm>>,
    },
    /// Union pattern.
    Union {
        /// The alternative patterns.
        patterns: Vec<TermId<PatternTerm>>,
    },
}

/// One field inside a pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternFieldTerm {
    /// Named field.
    Named {
        /// The selected key.
        key: dir::StaticKey,
        /// The field pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Computed field.
    Computed {
        /// The computed key type.
        key: VariableId,
        /// The field pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Positional field.
    Positional {
        /// The field pattern.
        pattern: TermId<PatternTerm>,
    },
    /// Spread field.
    Spread {
        /// The spread pattern.
        pattern: Option<TermId<PatternTerm>>,
    },
    /// Elided field.
    Elision,
}

/// Assignment target pattern checked against an assigned value type.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum AssignPatternTerm {
    /// Expression assignment target.
    Expression {
        /// The target place type.
        place: VariableId,
    },
    /// Defaulted assignment target.
    Assign {
        /// The target pattern.
        pattern: TermId<AssignPatternTerm>,
        /// The default value type.
        value: VariableId,
    },
    /// Sequence destructuring target.
    Sequence {
        /// The sequence fields.
        fields: Vec<TermId<AssignPatternFieldTerm>>,
    },
    /// Object destructuring target.
    Object {
        /// The object fields.
        fields: Vec<TermId<AssignPatternFieldTerm>>,
    },
}

/// One field inside an assignment target pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum AssignPatternFieldTerm {
    /// Named field.
    Named {
        /// The selected key.
        key: dir::StaticKey,
        /// The field target pattern.
        pattern: Option<TermId<AssignPatternTerm>>,
    },
    /// Computed field.
    Computed {
        /// The computed key type.
        key: VariableId,
        /// The field target pattern.
        pattern: TermId<AssignPatternTerm>,
    },
    /// Positional field.
    Positional {
        /// The field target pattern.
        pattern: TermId<AssignPatternTerm>,
    },
    /// Spread field.
    Spread {
        /// The spread target pattern.
        pattern: Option<TermId<AssignPatternTerm>>,
    },
    /// Elided field.
    Elision,
}

impl PatternTerm {
    /// Return variables referenced by this pattern.
    pub(in crate::check) fn referenced_variables(
        &self,
        terms: &TermTable,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        self.collect_referenced_variables(terms, &mut variables);

        variables
    }

    /// Collect variables referenced by this pattern.
    fn collect_referenced_variables(
        &self,
        terms: &TermTable,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Wildcard => {}
            Self::Must { pattern }
            | Self::BorrowOf { pattern, .. }
            | Self::MoveOf { pattern, .. }
            | Self::DereferenceOf { pattern } => terms
                .get(*pattern)
                .collect_referenced_variables(terms, variables),
            Self::Assign { pattern, value } => {
                terms
                    .get(*pattern)
                    .collect_referenced_variables(terms, variables);
                variables.push(*value);
            }
            Self::Binding { symbol: _, pattern } => {
                if let Some(pattern) = pattern {
                    terms
                        .get(*pattern)
                        .collect_referenced_variables(terms, variables);
                }
            }
            Self::Expression { value } => variables.push(*value),
            Self::Range {
                start,
                end,
                end_kind: _,
            } => {
                variables.extend(*start);
                variables.extend(*end);
            }
            Self::Type { ty } => variables.push(*ty),
            Self::Tuple { fields } | Self::Sequence { fields } | Self::Object { fields } => {
                for field in fields {
                    terms
                        .get(*field)
                        .collect_referenced_variables(terms, variables);
                }
            }
            Self::TaggedTuple { ty, fields } | Self::TaggedObject { ty, fields } => {
                variables.push(*ty);
                for field in fields {
                    terms
                        .get(*field)
                        .collect_referenced_variables(terms, variables);
                }
            }
            Self::Union { patterns } => {
                for pattern in patterns {
                    terms
                        .get(*pattern)
                        .collect_referenced_variables(terms, variables);
                }
            }
        }
    }
}

impl PatternFieldTerm {
    /// Collect variables referenced by this field.
    fn collect_referenced_variables(
        &self,
        terms: &TermTable,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Named { key: _, pattern } | Self::Spread { pattern } => {
                if let Some(pattern) = pattern {
                        terms
                            .get(*pattern)
                            .collect_referenced_variables(terms, variables);
                }
            }
            Self::Computed { key, pattern } => {
                variables.push(*key);
                terms
                    .get(*pattern)
                    .collect_referenced_variables(terms, variables);
            }
            Self::Positional { pattern } => terms
                .get(*pattern)
                .collect_referenced_variables(terms, variables),
            Self::Elision => {}
        }
    }
}

impl AssignPatternTerm {
    /// Return variables referenced by this assignment pattern.
    pub(in crate::check) fn referenced_variables(
        &self,
        terms: &TermTable,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        self.collect_referenced_variables(terms, &mut variables);

        variables
    }

    /// Collect variables referenced by this assignment pattern.
    fn collect_referenced_variables(
        &self,
        terms: &TermTable,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Expression { place } => variables.push(*place),
            Self::Assign { pattern, value } => {
                terms
                    .get(*pattern)
                    .collect_referenced_variables(terms, variables);
                variables.push(*value);
            }
            Self::Sequence { fields } | Self::Object { fields } => {
                for field in fields {
                    terms
                        .get(*field)
                        .collect_referenced_variables(terms, variables);
                }
            }
        }
    }
}

impl AssignPatternFieldTerm {
    /// Collect variables referenced by this assignment pattern field.
    fn collect_referenced_variables(
        &self,
        terms: &TermTable,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Named { key: _, pattern } | Self::Spread { pattern } => {
                if let Some(pattern) = pattern {
                        terms
                            .get(*pattern)
                            .collect_referenced_variables(terms, variables);
                }
            }
            Self::Computed { key, pattern } => {
                variables.push(*key);
                terms
                    .get(*pattern)
                    .collect_referenced_variables(terms, variables);
            }
            Self::Positional { pattern } => terms
                .get(*pattern)
                .collect_referenced_variables(terms, variables),
            Self::Elision => {}
        }
    }
}

impl CheckState<'_> {
    /// Decide whether one pattern relation holds.
    pub(in crate::check) fn decide_pattern_relation(
        &mut self,
        value: VariableId,
        relation: &PatternRelation,
    ) -> CompilerResult<Decision> {
        let module = value.module;
        let value = TypeTerm::Variable(value);
        let decision = match relation {
            PatternRelation::Match(pattern) => self.decide_pattern_term(module, &value, *pattern)?,
            PatternRelation::Assign(pattern) => {
                self.decide_assign_pattern_term(module, &value, *pattern)?
            }
        };

        Ok(decision)
    }

    /// Decide whether one pattern can match one value type.
    fn decide_pattern_term(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        pattern: TermId<PatternTerm>,
    ) -> CompilerResult<Decision> {
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
            } => return self.decide_pattern_term(module, value, pattern),
            PatternTerm::Expression { value: expected } => self.decide_type_term_relation(
                TypeRelation::Assignable,
                &TypeTerm::Variable(expected),
                value,
            )?,
            PatternTerm::Range {
                start,
                end,
                end_kind: _,
            } => {
                let mut decision = Decision::Yes;

                // check lower bound
                if let Some(start) = start {
                    decision = decision.and(self.decide_type_term_relation(
                        TypeRelation::Assignable,
                        &TypeTerm::Variable(start),
                        value,
                    )?);
                }

                // check upper bound
                if let Some(end) = end {
                    decision = decision.and(self.decide_type_term_relation(
                        TypeRelation::Assignable,
                        &TypeTerm::Variable(end),
                        value,
                    )?);
                }

                decision
            }
            PatternTerm::Type { ty } => self.decide_type_term_relation(
                TypeRelation::Satisfies,
                value,
                &TypeTerm::Variable(ty),
            )?,
            PatternTerm::Tuple { fields }
            | PatternTerm::Sequence { fields }
            | PatternTerm::Object { fields } => self.decide_pattern_fields(module, value, &fields)?,
            PatternTerm::TaggedTuple { ty, fields } | PatternTerm::TaggedObject { ty, fields } => {
                let tag = self.decide_type_term_relation(
                    TypeRelation::Satisfies,
                    value,
                    &TypeTerm::Variable(ty),
                )?;
                let fields = self.decide_pattern_fields(module, value, &fields)?;

                tag.and(fields)
            }
            PatternTerm::Union { patterns } => {
                let mut decision = Decision::No;

                // accept any matching pattern alternative
                for pattern in patterns {
                    decision = decision.or(self.decide_pattern_term(module, value, pattern)?);
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
    fn decide_pattern_fields(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        fields: &[TermId<PatternFieldTerm>],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // every requested field must match
        for field in fields {
            decision = decision.and(self.decide_pattern_field(module, value, *field)?);
        }

        Ok(decision)
    }

    /// Decide whether one pattern field can match one value type.
    fn decide_pattern_field(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        field: TermId<PatternFieldTerm>,
    ) -> CompilerResult<Decision> {
        let field = self.terms.get(field).clone();
        let decision = match field {
            PatternFieldTerm::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(ty) = self.resolve_member_type(module, value, &key, &[])? else {
                    return Ok(Decision::No);
                };

                self.decide_pattern_term(module, &ty, pattern)?
            }
            PatternFieldTerm::Computed { key: _, pattern }
            | PatternFieldTerm::Positional { pattern } => {
                self.decide_pattern_term(module, value, pattern)?
            }
            PatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.decide_pattern_term(module, value, pattern)?
                } else {
                    Decision::Yes
                }
            }
            PatternFieldTerm::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Decide whether one assignment pattern can accept one value type.
    fn decide_assign_pattern_term(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        pattern: TermId<AssignPatternTerm>,
    ) -> CompilerResult<Decision> {
        let pattern = self.terms.get(pattern).clone();
        let decision = match pattern {
            AssignPatternTerm::Expression { place } => self.decide_type_term_relation(
                TypeRelation::Assignable,
                value,
                &TypeTerm::Variable(place),
            )?,
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default = self.decide_type_term_relation(
                    TypeRelation::Assignable,
                    &TypeTerm::Variable(default),
                    value,
                )?;
                let pattern = self.decide_assign_pattern_term(module, value, pattern)?;

                default.and(pattern)
            }
            AssignPatternTerm::Sequence { fields } | AssignPatternTerm::Object { fields } => {
                self.decide_assign_pattern_fields(module, value, &fields)?
            }
        };

        Ok(decision)
    }

    /// Decide whether assignment pattern fields can accept one value type.
    fn decide_assign_pattern_fields(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        fields: &[TermId<AssignPatternFieldTerm>],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // every assignment field must accept the source value
        for field in fields {
            decision = decision.and(self.decide_assign_pattern_field(module, value, *field)?);
        }

        Ok(decision)
    }

    /// Decide whether one assignment pattern field can accept one value type.
    fn decide_assign_pattern_field(
        &mut self,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        field: TermId<AssignPatternFieldTerm>,
    ) -> CompilerResult<Decision> {
        let field = self.terms.get(field).clone();
        let decision = match field {
            AssignPatternFieldTerm::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Decision::Yes);
                };
                let Some(ty) = self.resolve_member_type(module, value, &key, &[])? else {
                    return Ok(Decision::No);
                };

                self.decide_assign_pattern_term(module, &ty, pattern)?
            }
            AssignPatternFieldTerm::Computed { key: _, pattern }
            | AssignPatternFieldTerm::Positional { pattern } => {
                self.decide_assign_pattern_term(module, value, pattern)?
            }
            AssignPatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.decide_assign_pattern_term(module, value, pattern)?
                } else {
                    Decision::Yes
                }
            }
            AssignPatternFieldTerm::Elision => Decision::Yes,
        };

        Ok(decision)
    }

    /// Expect a pattern to match one value type.
    pub(in crate::check) fn expect_pattern_term(
        &mut self,
        value: VariableId,
        pattern: TermId<PatternTerm>,
    ) -> CompilerResult<Progress> {
        let pattern = self.terms.get(pattern).clone();
        let progress = match pattern {
            PatternTerm::Wildcard => Progress::Unchanged,
            PatternTerm::Must { pattern }
            | PatternTerm::BorrowOf { pattern, .. }
            | PatternTerm::MoveOf { pattern, .. }
            | PatternTerm::DereferenceOf { pattern } => self.expect_pattern_term(value, pattern)?,
            PatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default = self.solve_type_relation(TypeRelation::Assignable, default, value)?;
                let pattern = self.expect_pattern_term(value, pattern)?;

                default.merge(pattern)
            }
            PatternTerm::Binding { symbol, pattern } => {
                let binding = if let Some(symbol) = symbol {
                    let binding = self.intern_symbol_type_variable(symbol.module_id, symbol);

                    self.solve_type_relation(TypeRelation::Equal, binding, value)?
                } else {
                    Progress::Unchanged
                };
                let pattern = if let Some(pattern) = pattern {
                    self.expect_pattern_term(value, pattern)?
                } else {
                    Progress::Unchanged
                };

                binding.merge(pattern)
            }
            PatternTerm::Expression { value: expected } => {
                self.solve_type_relation(TypeRelation::Assignable, expected, value)?
            }
            PatternTerm::Range {
                start,
                end,
                end_kind: _,
            } => {
                let start = if let Some(start) = start {
                    self.solve_type_relation(TypeRelation::Assignable, start, value)?
                } else {
                    Progress::Unchanged
                };
                let end = if let Some(end) = end {
                    self.solve_type_relation(TypeRelation::Assignable, end, value)?
                } else {
                    Progress::Unchanged
                };

                start.merge(end)
            }
            PatternTerm::Type { ty } => {
                self.solve_type_relation(TypeRelation::Satisfies, value, ty)?
            }
            PatternTerm::Tuple { fields } | PatternTerm::Sequence { fields } => {
                self.expect_pattern_field_terms(value, &fields)?
            }
            PatternTerm::Object { fields } => self.expect_pattern_field_terms(value, &fields)?,
            PatternTerm::TaggedTuple { ty, fields } | PatternTerm::TaggedObject { ty, fields } => {
                let tag = self.solve_type_relation(TypeRelation::Satisfies, value, ty)?;
                let fields = self.expect_pattern_field_terms(value, &fields)?;

                tag.merge(fields)
            }
            PatternTerm::Union { patterns } => {
                let mut progress = Progress::Unchanged;
                for pattern in patterns {
                    progress = progress.merge(self.expect_pattern_term(value, pattern)?);
                }

                progress
            }
        };

        Ok(progress)
    }

    /// Expect pattern fields to match one value type.
    fn expect_pattern_field_terms(
        &mut self,
        value: VariableId,
        fields: &[TermId<PatternFieldTerm>],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for field in fields {
            progress = progress.merge(self.expect_pattern_field(value, *field)?);
        }

        Ok(progress)
    }

    /// Expect one pattern field to match one value type.
    fn expect_pattern_field(
        &mut self,
        value: VariableId,
        field: TermId<PatternFieldTerm>,
    ) -> CompilerResult<Progress> {
        let field = self.terms.get(field).clone();
        let progress = match field {
            PatternFieldTerm::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) =
                    self.resolve_member_type(value.module, &TypeTerm::Variable(value), &key, &[])?
                else {
                    return Ok(Progress::Unchanged);
                };
                let origin = self.variable_origin(value)?;
                let ty = self.solve_anonymous_type(value.module, origin, ty)?;

                self.expect_pattern_term(ty, pattern)?
            }
            PatternFieldTerm::Computed { key: _, pattern }
            | PatternFieldTerm::Positional { pattern } => {
                self.expect_pattern_term(value, pattern)?
            }
            PatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.expect_pattern_term(value, pattern)?
                } else {
                    Progress::Unchanged
                }
            }
            PatternFieldTerm::Elision => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect an assignment pattern to accept one value type.
    pub(in crate::check) fn expect_assign_pattern_term(
        &mut self,
        value: VariableId,
        pattern: TermId<AssignPatternTerm>,
    ) -> CompilerResult<Progress> {
        let pattern = self.terms.get(pattern).clone();
        let progress = match pattern {
            AssignPatternTerm::Expression { place } => {
                self.solve_type_relation(TypeRelation::Assignable, value, place)?
            }
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default = self.solve_type_relation(TypeRelation::Assignable, default, value)?;
                let pattern = self.expect_assign_pattern_term(value, pattern)?;

                default.merge(pattern)
            }
            AssignPatternTerm::Sequence { fields } | AssignPatternTerm::Object { fields } => {
                self.expect_assign_pattern_field_terms(value, &fields)?
            }
        };

        Ok(progress)
    }

    /// Expect assignment pattern fields to accept one value type.
    fn expect_assign_pattern_field_terms(
        &mut self,
        value: VariableId,
        fields: &[TermId<AssignPatternFieldTerm>],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for field in fields {
            progress = progress.merge(self.expect_assign_pattern_field(value, *field)?);
        }

        Ok(progress)
    }

    /// Expect one assignment pattern field to accept one value type.
    fn expect_assign_pattern_field(
        &mut self,
        value: VariableId,
        field: TermId<AssignPatternFieldTerm>,
    ) -> CompilerResult<Progress> {
        let field = self.terms.get(field).clone();
        let progress = match field {
            AssignPatternFieldTerm::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) =
                    self.resolve_member_type(value.module, &TypeTerm::Variable(value), &key, &[])?
                else {
                    return Ok(Progress::Unchanged);
                };
                let origin = self.variable_origin(value)?;
                let ty = self.solve_anonymous_type(value.module, origin, ty)?;

                self.expect_assign_pattern_term(ty, pattern)?
            }
            AssignPatternFieldTerm::Computed { key: _, pattern }
            | AssignPatternFieldTerm::Positional { pattern } => {
                self.expect_assign_pattern_term(value, pattern)?
            }
            AssignPatternFieldTerm::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    self.expect_assign_pattern_term(value, pattern)?
                } else {
                    Progress::Unchanged
                }
            }
            AssignPatternFieldTerm::Elision => Progress::Unchanged,
        };

        Ok(progress)
    }
}
