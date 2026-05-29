use smallvec::SmallVec;

use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    CheckState, Condition, Decision, Origin, PatternRelation, Progress, TermId, TermTable,
    TypeOperand, TypeRelation, TypeTerm, VariableId, VariableKind,
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
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Newtype wrapper pattern.
    Newtype {
        /// The tag type.
        ty: VariableId,
        /// The tuple fields.
        fields: SmallVec<[PatternField; 3]>,
    },
    /// Sequence pattern.
    Sequence {
        /// The sequence fields.
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Object pattern.
    Object {
        /// The object fields.
        fields: SmallVec<[PatternField; 4]>,
    },
    /// Nominal object pattern.
    NominalObject {
        /// The tag type.
        ty: VariableId,
        /// The object fields.
        fields: SmallVec<[PatternField; 3]>,
    },
    /// Union pattern.
    Union {
        /// The alternative patterns.
        patterns: Vec<TermId<PatternTerm>>,
    },
}

/// One field inside a pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternField {
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
        /// The target storage type.
        target: TypeOperand,
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
        fields: SmallVec<[AssignPatternField; 8]>,
    },
    /// Object destructuring target.
    Object {
        /// The object fields.
        fields: SmallVec<[AssignPatternField; 8]>,
    },
}

/// One field inside an assignment target pattern.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum AssignPatternField {
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
                    field.collect_referenced_variables(terms, variables);
                }
            }
            Self::Newtype { ty, fields } | Self::NominalObject { ty, fields } => {
                variables.push(*ty);
                for field in fields {
                    field.collect_referenced_variables(terms, variables);
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

impl PatternField {
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
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        self.collect_referenced_variables(state, &mut variables);

        variables
    }

    /// Collect variables referenced by this assignment pattern.
    fn collect_referenced_variables(
        &self,
        state: &CheckState<'_>,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Expression { target } => variables.extend(target.referenced_variables(state)),
            Self::Assign { pattern, value } => {
                state
                    .terms
                    .get(*pattern)
                    .collect_referenced_variables(state, variables);
                variables.push(*value);
            }
            Self::Sequence { fields } | Self::Object { fields } => {
                for field in fields {
                    field.collect_referenced_variables(state, variables);
                }
            }
        }
    }
}

impl AssignPatternField {
    /// Collect variables referenced by this assignment pattern field.
    fn collect_referenced_variables(
        &self,
        state: &CheckState<'_>,
        variables: &mut SmallVec<[VariableId; 4]>,
    ) {
        match self {
            Self::Named { key: _, pattern } | Self::Spread { pattern } => {
                if let Some(pattern) = pattern {
                    state
                        .terms
                        .get(*pattern)
                        .collect_referenced_variables(state, variables);
                }
            }
            Self::Computed { key, pattern } => {
                variables.push(*key);
                state
                    .terms
                    .get(*pattern)
                    .collect_referenced_variables(state, variables);
            }
            Self::Positional { pattern } => state
                .terms
                .get(*pattern)
                .collect_referenced_variables(state, variables),
            Self::Elision => {}
        }
    }
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
        let value = value.to_type_term(self);
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
            } => return self.reduce_pattern_term(origin, module, value, pattern),
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
            | PatternTerm::Object { fields } => {
                self.reduce_pattern_fields(origin, module, value, &fields)?
            }
            PatternTerm::Newtype { ty, fields } | PatternTerm::NominalObject { ty, fields } => {
                let tag = self.decide_type_term_relation(
                    TypeRelation::Satisfies,
                    value,
                    &TypeTerm::Variable(ty),
                )?;
                let fields = self.reduce_pattern_fields(origin, module, value, &fields)?;

                tag.and(fields)
            }
            PatternTerm::Union { patterns } => {
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
        module: destack_source::ModuleId,
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
        module: destack_source::ModuleId,
        value: &TypeTerm,
        field: PatternField,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            PatternField::Named { key, pattern } => {
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

    /// Decide whether one assignment pattern can accept one value type.
    fn reduce_assign_pattern_term(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        value: &TypeTerm,
        pattern: TermId<AssignPatternTerm>,
    ) -> CompilerResult<Decision> {
        let pattern = self.terms.get(pattern).clone();
        let decision = match pattern {
            AssignPatternTerm::Expression { target } => self.decide_type_term_relation(
                TypeRelation::Assignable,
                value,
                &target.to_type_term(self),
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
        module: destack_source::ModuleId,
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
        module: destack_source::ModuleId,
        value: &TypeTerm,
        field: AssignPatternField,
    ) -> CompilerResult<Decision> {
        let field = field;
        let decision = match field {
            AssignPatternField::Named { key, pattern } => {
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
        let pattern = self.terms.get(pattern).clone();
        let progress = match pattern {
            PatternTerm::Wildcard => Progress::Unchanged,
            PatternTerm::Must { pattern }
            | PatternTerm::BorrowOf { pattern, .. }
            | PatternTerm::MoveOf { pattern, .. }
            | PatternTerm::DereferenceOf { pattern } => {
                self.expect_pattern_term(origin, value, pattern)?
            }
            PatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default =
                    self.solve_type_relation(origin, TypeRelation::Assignable, default, value)?;
                let pattern = self.expect_pattern_term(origin, value, pattern)?;

                default.merge(pattern)
            }
            PatternTerm::Binding { symbol, pattern } => {
                let binding = if let Some(symbol) = symbol {
                    let binding = self.intern_local_symbol_type_variable(symbol.module_id, symbol);

                    self.solve_type_relation(origin, TypeRelation::Equal, binding, value)?
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
            PatternTerm::Expression { value: expected } => {
                self.solve_type_relation(origin, TypeRelation::Assignable, expected, value)?
            }
            PatternTerm::Range {
                start,
                end,
                end_kind: _,
            } => {
                let start = if let Some(start) = start {
                    self.solve_type_relation(origin, TypeRelation::Assignable, start, value)?
                } else {
                    Progress::Unchanged
                };
                let end = if let Some(end) = end {
                    self.solve_type_relation(origin, TypeRelation::Assignable, end, value)?
                } else {
                    Progress::Unchanged
                };

                start.merge(end)
            }
            PatternTerm::Type { ty } => {
                self.solve_type_relation(origin, TypeRelation::Satisfies, value, ty)?
            }
            PatternTerm::Tuple { fields } | PatternTerm::Sequence { fields } => {
                self.expect_pattern_field_terms(origin, value, &fields)?
            }
            PatternTerm::Object { fields } => {
                self.expect_pattern_field_terms(origin, value, &fields)?
            }
            PatternTerm::Newtype { ty, fields } | PatternTerm::NominalObject { ty, fields } => {
                let tag = self.solve_type_relation(origin, TypeRelation::Satisfies, value, ty)?;
                let fields = self.expect_pattern_field_terms(origin, value, &fields)?;

                tag.merge(fields)
            }
            PatternTerm::Union { patterns } => {
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
            PatternField::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) =
                    self.resolve_member_type(origin, module, &value.to_type_term(self), &key, &[])?
                else {
                    return Ok(Progress::Unchanged);
                };
                let field = self.allocate_inference_variable(module, VariableKind::Type, origin);
                self.add_type_definition(field, ty, Condition::Always);

                self.expect_pattern_term(origin, TypeOperand::Variable(field), pattern)?
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
        let pattern = self.terms.get(pattern).clone();
        let progress = match pattern {
            AssignPatternTerm::Expression { target } => {
                self.solve_type_relation(origin, TypeRelation::Assignable, value, target)?
            }
            AssignPatternTerm::Assign {
                pattern,
                value: default,
            } => {
                let default =
                    self.solve_type_relation(origin, TypeRelation::Assignable, default, value)?;
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
            AssignPatternField::Named { key, pattern } => {
                let Some(pattern) = pattern else {
                    return Ok(Progress::Unchanged);
                };
                let Some(ty) =
                    self.resolve_member_type(origin, module, &value.to_type_term(self), &key, &[])?
                else {
                    return Ok(Progress::Unchanged);
                };
                let field = self.allocate_inference_variable(module, VariableKind::Type, origin);
                self.add_type_definition(field, ty, Condition::Always);

                self.expect_assign_pattern_term(origin, TypeOperand::Variable(field), pattern)?
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
}
