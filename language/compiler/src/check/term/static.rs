use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, ExportLookup, GenericArgument, GenericSlotId, GenericSubstitution,
    LayoutQuery, LayoutTerm, Origin, Reduction, StaticOperand, StaticRelation, TermId,
    TypeRelation, VariableId,
};

/// Term used to define a static variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticTerm {
    /// Concrete static term.
    ///
    /// ```ts
    /// "mutable"
    /// ```
    Literal(dir::StaticTerm),
    /// Static variable alias.
    ///
    /// ```ts
    /// import { L } from "./lifetimes";
    /// ```
    Variable(VariableId),
    /// Static generic parameter.
    ///
    /// ```ts
    /// <comptime N: uint>
    /// ```
    Parameter(GenericSlotId),
    /// Source expression evaluated as a static term.
    ///
    /// ```ts
    /// N + 1
    /// ```
    Expression(dir::GlobalNodeId<dir::Expression>),
    /// Static member projection.
    ///
    /// ```ts
    /// Register.Width
    /// ```
    Member {
        /// The source member expression.
        source: dir::GlobalNodeIdAny,
        /// The owner type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
        /// The applied static arguments.
        arguments: SmallVec<[GenericArgument; 4]>,
    },
    /// Static value join.
    ///
    /// ```ts
    /// L | R
    /// ```
    Join {
        /// The joined values.
        elements: Vec<VariableId>,
    },
    /// Concrete layout query.
    ///
    /// ```ts
    /// sizeOf<T>()
    /// ```
    Layout(LayoutTerm),
    /// Compiler intrinsic returning a static value.
    ///
    /// ```ts
    /// LifetimeOf<T>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[GenericArgument; 4]>,
    },
    /// Static equality comparison.
    ///
    /// ```ts
    /// this.Width == 4
    /// ```
    Equal {
        /// The left static value.
        left: TermId<StaticTerm>,
        /// The right static value.
        right: TermId<StaticTerm>,
        /// Whether the equality result is negated.
        is_negated: bool,
    },
    /// Type relation used as a static boolean.
    ///
    /// ```ts
    /// T extends string
    /// ```
    TypeRelation {
        /// The required type relation.
        relation: TypeRelation,
        /// The left type.
        left: VariableId,
        /// The right type.
        right: VariableId,
    },
    /// Static conditional value.
    ///
    /// ```ts
    /// C ? T : F
    /// ```
    Conditional {
        /// The static boolean condition.
        condition: TermId<StaticTerm>,
        /// The value selected when the condition holds.
        then_value: TermId<StaticTerm>,
        /// The value selected when the condition does not hold.
        else_value: TermId<StaticTerm>,
    },
}

impl StaticTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        match self {
            Self::Variable(variable) => variables.push(*variable),
            Self::Member {
                source: _,
                owner,
                key: _,
                arguments,
            } => {
                variables.push(*owner);
                variables.extend(
                    arguments
                        .iter()
                        .flat_map(|argument| state.argument_variables(argument)),
                );
            }
            Self::Join { elements } => variables.extend(elements.iter().copied()),
            Self::Layout(layout) => variables.extend(layout.referenced_variables()),
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(
                    arguments
                        .iter()
                        .flat_map(|argument| state.argument_variables(argument)),
                );
            }
            Self::Equal { left, right, .. } => {
                variables.extend(state.terms.get(*left).referenced_variables(state));
                variables.extend(state.terms.get(*right).referenced_variables(state));
            }
            Self::TypeRelation {
                relation: _,
                left,
                right,
            } => {
                variables.push(*left);
                variables.push(*right);
            }
            Self::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                variables.extend(state.terms.get(*condition).referenced_variables(state));
                variables.extend(state.terms.get(*then_value).referenced_variables(state));
                variables.extend(state.terms.get(*else_value).referenced_variables(state));
            }
            Self::Literal(_) | Self::Parameter(_) | Self::Expression(_) => {}
        }

        variables
    }

    /// Substitute generic arguments through one static term.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<StaticTerm> {
        let term = match self {
            StaticTerm::Expression(expression) => {
                if let Some(term) = state.build_static_expression_term(expression.clone())? {
                    term.substitute(module, substitution, state)?
                } else {
                    self.clone()
                }
            }
            StaticTerm::Join { elements } => StaticTerm::Join {
                elements: state.substitute_static_variables(module, substitution, elements)?,
            },
            StaticTerm::Layout(layout) => {
                StaticTerm::Layout(layout.substitute(module, substitution, state)?)
            }
            StaticTerm::Member {
                source,
                owner,
                key,
                arguments,
            } => StaticTerm::Member {
                source: *source,
                owner: state.substitute_type_variable(module, substitution, *owner)?,
                key: *key,
                arguments: state.substitute_arguments(module, substitution, arguments)?,
            },
            StaticTerm::Intrinsic { item, arguments } => StaticTerm::Intrinsic {
                item: *item,
                arguments: state
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
            StaticTerm::Equal {
                left,
                right,
                is_negated,
            } => StaticTerm::Equal {
                left: state.substitute_static_term(module, substitution, *left)?,
                right: state.substitute_static_term(module, substitution, *right)?,
                is_negated: *is_negated,
            },
            StaticTerm::TypeRelation {
                relation,
                left,
                right,
            } => StaticTerm::TypeRelation {
                relation: *relation,
                left: state.substitute_type_variable(module, substitution, *left)?,
                right: state.substitute_type_variable(module, substitution, *right)?,
            },
            StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            } => StaticTerm::Conditional {
                condition: state.substitute_static_term(module, substitution, *condition)?,
                then_value: state.substitute_static_term(module, substitution, *then_value)?,
                else_value: state.substitute_static_term(module, substitution, *else_value)?,
            },
            StaticTerm::Variable(variable) => {
                if let Some(argument) = state.substitution_static_variable(substitution, *variable)
                {
                    StaticTerm::Variable(argument)
                } else if let Some(term) = state.solved_static_term(*variable)? {
                    term.substitute(module, substitution, state)?
                } else {
                    self.clone()
                }
            }
            StaticTerm::Parameter(slot) => {
                if let Some(argument) = state.substitution_static_slot(substitution, *slot)
                    && let Some(term) = state.solved_static_term(argument)?
                {
                    term
                } else {
                    self.clone()
                }
            }
            StaticTerm::Literal(_) => self.clone(),
        };

        Ok(term)
    }
}

impl CheckState<'_> {
    /// Substitute generic arguments through one static term id.
    pub(in crate::check) fn substitute_static_term(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<TermId<StaticTerm>> {
        let term = self.terms.get(term).clone();
        let term = term.substitute(module, substitution, self)?;
        let term = self.terms.push(term);

        Ok(term)
    }

    /// Decide one static term relation.
    pub(in crate::check) fn decide_static_term_relation(
        &self,
        relation: StaticRelation,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Decision> {
        let decision = match relation {
            StaticRelation::Equal => self.decide_static_equal(left, right)?,
            StaticRelation::Assignable => self.decide_static_assignable(left, right)?,
        };

        Ok(decision)
    }

    /// Decide one solved static relation.
    pub(in crate::check) fn decide_static_relation(
        &self,
        relation: StaticRelation,
        left: impl Into<StaticOperand>,
        right: impl Into<StaticOperand>,
    ) -> CompilerResult<Decision> {
        let left = match left.into() {
            StaticOperand::Variable(variable) => {
                let Some(term) = self.solved_static_term(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            StaticOperand::Term(term) => self.terms.get(term).clone(),
        };
        let right = match right.into() {
            StaticOperand::Variable(variable) => {
                let Some(term) = self.solved_static_term(variable)? else {
                    return Ok(Decision::Undecidable);
                };

                term
            }
            StaticOperand::Term(term) => self.terms.get(term).clone(),
        };

        self.decide_static_term_relation(relation, &left, &right)
    }

    /// Reduce one static term when the solver has enough input.
    pub(in crate::check) fn reduce_static_term(
        &mut self,
        origin: Origin,
        term: &StaticTerm,
    ) -> CompilerResult<Option<StaticTerm>> {
        let module = origin.module();
        let term = match term {
            StaticTerm::Variable(variable) => {
                let Some(term) = self.solved_static_term(*variable)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Parameter(_) => term.clone(),
            StaticTerm::Expression(expression) => {
                let Some(term) = self.build_static_expression_term(expression.clone())? else {
                    return Ok(None);
                };

                let Some(term) = self.reduce_static_term(origin, &term)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Member {
                source: _,
                owner,
                key,
                arguments,
            } => {
                if !arguments.is_empty() {
                    return Ok(None);
                }
                let Some(owner) = self.solved_type_term(*owner)? else {
                    return Ok(None);
                };
                let owner = match self.reduce_type_term(origin, &owner)? {
                    Reduction {
                        value: Some(value),
                        progress: _,
                    } => value,
                    Reduction {
                        value: None,
                        progress: _,
                    } => owner,
                };
                let Some(term) = self.member_static_term(module, &owner, key)? else {
                    return Ok(None);
                };

                let Some(term) = self.reduce_static_term(origin, &term)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Join { elements } => {
                let Some(term) = self.reduce_static_join(module, elements)? else {
                    return Ok(None);
                };

                StaticTerm::Literal(term)
            }
            StaticTerm::Layout(layout) => {
                let Some(term) = self.reduce_layout_term(module, layout)? else {
                    return Ok(None);
                };

                StaticTerm::Literal(term)
            }
            StaticTerm::Intrinsic { item, arguments } => {
                let Some(term) = self.memory_static_value(module, *item, arguments)? else {
                    return Ok(None);
                };

                StaticTerm::Literal(term)
            }
            StaticTerm::Equal {
                left,
                right,
                is_negated,
            } => {
                let left = self.terms.get(*left).clone();
                let Some(left) = self.reduce_static_term(origin, &left)? else {
                    return Ok(None);
                };
                let right = self.terms.get(*right).clone();
                let Some(right) = self.reduce_static_term(origin, &right)? else {
                    return Ok(None);
                };
                let decision =
                    self.decide_static_term_relation(StaticRelation::Equal, &left, &right)?;
                let value = match decision {
                    Decision::Yes => !*is_negated,
                    Decision::No => *is_negated,
                    Decision::Undecidable => return Ok(None),
                };

                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(value),
                })
            }
            StaticTerm::TypeRelation {
                relation,
                left,
                right,
            } => {
                let decision = self.decide_type_relation(*relation, *left, *right)?;
                let value = match decision {
                    Decision::Yes => true,
                    Decision::No => false,
                    Decision::Undecidable => return Ok(None),
                };

                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(value),
                })
            }
            StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.terms.get(*condition).clone();
                let Some(condition) = self.reduce_static_term(origin, &condition)? else {
                    return Ok(None);
                };

                match condition {
                    StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                        value: dir::ScalarLiteral::Boolean(true),
                    }) => {
                        let then_value = self.terms.get(*then_value).clone();
                        let Some(term) = self.reduce_static_term(origin, &then_value)? else {
                            return Ok(None);
                        };

                        term
                    }
                    StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                        value: dir::ScalarLiteral::Boolean(false),
                    }) => {
                        let else_value = self.terms.get(*else_value).clone();
                        let Some(term) = self.reduce_static_term(origin, &else_value)? else {
                            return Ok(None);
                        };

                        term
                    }
                    _ => return Ok(None),
                }
            }
            StaticTerm::Literal(_) => term.clone(),
        };

        Ok(Some(term))
    }

    /// Decide exact static equality.
    fn decide_static_equal(
        &self,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Decision> {
        if left == right {
            return Ok(Decision::Yes);
        }

        let decision = match (left, right) {
            (StaticTerm::Variable(left), right) => {
                let Some(left) = self.solved_static_term(*left)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_equal(&left, right)?
            }
            (left, StaticTerm::Variable(right)) => {
                let Some(right) = self.solved_static_term(*right)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_equal(left, &right)?
            }
            (StaticTerm::Literal(left), StaticTerm::Literal(right)) => {
                self.decide_dir_static_equal(left, right)
            }
            (StaticTerm::Parameter(_), _)
            | (_, StaticTerm::Parameter(_))
            | (StaticTerm::Expression(_), _)
            | (_, StaticTerm::Expression(_))
            | (StaticTerm::Member { .. }, _)
            | (_, StaticTerm::Member { .. })
            | (StaticTerm::Join { .. }, _)
            | (_, StaticTerm::Join { .. })
            | (StaticTerm::Layout(_), _)
            | (_, StaticTerm::Layout(_))
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. })
            | (StaticTerm::Equal { .. }, _)
            | (_, StaticTerm::Equal { .. })
            | (StaticTerm::TypeRelation { .. }, _)
            | (_, StaticTerm::TypeRelation { .. })
            | (StaticTerm::Conditional { .. }, _)
            | (_, StaticTerm::Conditional { .. }) => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide static assignability.
    fn decide_static_assignable(
        &self,
        source: &StaticTerm,
        target: &StaticTerm,
    ) -> CompilerResult<Decision> {
        if self.decide_static_equal(source, target)? == Decision::Yes {
            return Ok(Decision::Yes);
        }

        let decision = match (source, target) {
            (StaticTerm::Variable(source), target) => {
                let Some(source) = self.solved_static_term(*source)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_assignable(&source, target)?
            }
            (source, StaticTerm::Variable(target)) => {
                let Some(target) = self.solved_static_term(*target)? else {
                    return Ok(Decision::Undecidable);
                };

                self.decide_static_assignable(source, &target)?
            }
            (
                StaticTerm::Literal(dir::StaticTerm::Access { access: source }),
                StaticTerm::Literal(dir::StaticTerm::Access { access: target }),
            ) => self.decide_access_term_assignable(*source, *target),
            (
                StaticTerm::Literal(dir::StaticTerm::Lifetime { lifetime: source }),
                StaticTerm::Literal(dir::StaticTerm::Lifetime { lifetime: target }),
            ) => self.decide_lifetime_term_assignable(source, target),
            (StaticTerm::Literal(_), StaticTerm::Literal(_)) => Decision::No,
            (StaticTerm::Parameter(_), _)
            | (_, StaticTerm::Parameter(_))
            | (StaticTerm::Expression(_), _)
            | (_, StaticTerm::Expression(_))
            | (StaticTerm::Member { .. }, _)
            | (_, StaticTerm::Member { .. })
            | (StaticTerm::Join { .. }, _)
            | (_, StaticTerm::Join { .. })
            | (StaticTerm::Layout(_), _)
            | (_, StaticTerm::Layout(_))
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. })
            | (StaticTerm::Equal { .. }, _)
            | (_, StaticTerm::Equal { .. })
            | (StaticTerm::TypeRelation { .. }, _)
            | (_, StaticTerm::TypeRelation { .. })
            | (StaticTerm::Conditional { .. }, _)
            | (_, StaticTerm::Conditional { .. }) => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide access capability assignability.
    fn decide_access_term_assignable(&self, source: dir::Access, target: dir::Access) -> Decision {
        if Self::access_rank(source) >= Self::access_rank(target) {
            Decision::Yes
        } else {
            Decision::No
        }
    }

    /// Return the access value represented by one static term.
    fn static_access(&self, term: &StaticTerm) -> Option<dir::Access> {
        match term {
            StaticTerm::Literal(dir::StaticTerm::Access { access }) => Some(*access),
            _ => None,
        }
    }

    /// Decide lifetime assignability.
    fn decide_lifetime_term_assignable(
        &self,
        source: &dir::Lifetime,
        target: &dir::Lifetime,
    ) -> Decision {
        if source == target || matches!(source, dir::Lifetime::Static) {
            Decision::Yes
        } else {
            Decision::No
        }
    }

    /// Return whether one static term is a lifetime value.
    fn static_is_lifetime(&self, term: &StaticTerm) -> bool {
        matches!(term, StaticTerm::Literal(dir::StaticTerm::Lifetime { .. }))
    }

    /// Return one access capability rank.
    fn access_rank(access: dir::Access) -> u8 {
        match access {
            dir::Access::Readonly => 0,
            dir::Access::Mutable => 1,
            dir::Access::Exclusive => 2,
        }
    }

    /// Reduce static bounds to a solved static term when the static domain can decide them.
    pub(in crate::check) fn reduce_static_bounds(
        &self,
        lower_bounds: &[StaticOperand],
        upper_bounds: &[StaticOperand],
        lower_terms: &[StaticTerm],
        upper_terms: &[StaticTerm],
    ) -> Option<StaticTerm> {
        if let Some(term) = self.reduce_access_bounds(lower_terms, upper_terms) {
            return Some(term);
        }
        if let Some(term) =
            self.reduce_lifetime_bounds(lower_bounds, upper_bounds, lower_terms, upper_terms)
        {
            return Some(term);
        }

        Self::single_static_bound_term(lower_terms)
    }

    /// Reduce access bounds to the least valid access solution.
    fn reduce_access_bounds(
        &self,
        lower_terms: &[StaticTerm],
        upper_terms: &[StaticTerm],
    ) -> Option<StaticTerm> {
        if lower_terms.is_empty() && upper_terms.is_empty() {
            return None;
        }
        let mut lower_rank = 0;
        let mut upper_rank = 2;
        let mut saw_access = false;

        // collect minimum required access
        for term in lower_terms {
            let access = self.static_access(term)?;
            lower_rank = lower_rank.max(Self::access_rank(access));
            saw_access = true;
        }

        // collect maximum allowed access
        for term in upper_terms {
            let access = self.static_access(term)?;
            upper_rank = upper_rank.min(Self::access_rank(access));
            saw_access = true;
        }
        if !saw_access || lower_rank > upper_rank {
            return None;
        }

        // preserve exact source access when inference only has an upper bound
        let rank = if lower_terms.is_empty() {
            upper_rank
        } else {
            lower_rank
        };
        let access = Self::access_from_rank(rank);

        Some(StaticTerm::Literal(dir::StaticTerm::Access { access }))
    }

    /// Reduce lifetime bounds to an exact or joined lifetime solution.
    fn reduce_lifetime_bounds(
        &self,
        lower_bounds: &[StaticOperand],
        upper_bounds: &[StaticOperand],
        lower_terms: &[StaticTerm],
        upper_terms: &[StaticTerm],
    ) -> Option<StaticTerm> {
        if !lower_terms.iter().all(|term| self.static_is_lifetime(term))
            || !upper_terms.iter().all(|term| self.static_is_lifetime(term))
        {
            return None;
        }
        if let Some(term) = Self::single_static_bound_term(lower_terms) {
            return Some(term);
        }

        // preserve exact source lifetime when inference only has one upper bound
        if lower_bounds.is_empty()
            && upper_bounds.len() == 1
            && let Some(term) = upper_terms.first()
        {
            return Some(term.clone());
        }

        // join source lifetimes for inferred result lifetimes
        if lower_bounds.is_empty() && upper_bounds.len() > 1 {
            let mut elements = Vec::with_capacity(upper_bounds.len());

            for bound in upper_bounds {
                let StaticOperand::Variable(variable) = bound else {
                    return None;
                };
                if !elements.contains(variable) {
                    elements.push(*variable);
                }
            }

            return Some(StaticTerm::Join { elements });
        }

        None
    }

    /// Return the access capability for one rank.
    fn access_from_rank(rank: u8) -> dir::Access {
        match rank {
            0 => dir::Access::Readonly,
            1 => dir::Access::Mutable,
            _ => dir::Access::Exclusive,
        }
    }

    /// Return the single equivalent static bound term.
    fn single_static_bound_term(terms: &[StaticTerm]) -> Option<StaticTerm> {
        let first = terms.first()?;

        for term in terms.iter().skip(1) {
            if term != first {
                return None;
            }
        }

        Some(first.clone())
    }

    /// Reduce one static join.
    fn reduce_static_join(
        &mut self,
        _module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let [element] = elements else {
            return Ok(None);
        };

        self.static_value((*element).into())
    }

    /// Decide exact DIR static equality.
    fn decide_dir_static_equal(&self, left: &dir::StaticTerm, right: &dir::StaticTerm) -> Decision {
        if left == right {
            Decision::Yes
        } else {
            Decision::No
        }
    }
}

impl CheckState<'_> {
    /// Return one locally concrete static expression value.
    pub(in crate::check) fn build_static_expression_value(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        self.build_static_expression_value_from_node(expression.module_id, expression.local_id)
    }

    /// Return one static expression as a solver static term.
    pub(in crate::check) fn build_static_expression_term(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let module = expression.module_id;
        let expression_id = expression.local_id;
        let expression_node = self.module(module).view().get(expression_id).clone();
        let source = expression.into_any();
        let term = match expression_node {
            dir::Expression::Parenthesized { expression } => {
                self.build_static_expression_term(expression.into_global(module))?
            }
            dir::Expression::ScalarLiteral(value) => {
                Some(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value,
                }))
            }
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                let owner = self.intern_node_type_variable(module, left.into_global_any(module));

                Some(StaticTerm::Member {
                    source,
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments: Vec::new().into(),
                })
            }
            dir::Expression::Type { value } => {
                self.build_static_type_expression_term(module, value)?
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if matches!(
                operator,
                dir::BinaryOperator::Equal
                    | dir::BinaryOperator::EqualStrict
                    | dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::NotEqualStrict
            ) =>
            {
                let Some(left) = self.build_static_expression_term(left.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(right) = self.build_static_expression_term(right.into_global(module))?
                else {
                    return Ok(None);
                };
                let is_negated = matches!(
                    operator,
                    dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
                );

                Some(StaticTerm::Equal {
                    left: self.terms.push(left),
                    right: self.terms.push(right),
                    is_negated,
                })
            }
            dir::Expression::If {
                condition:
                    dir::IfCondition::Expression {
                        condition: condition_expression,
                    },
                then_expression,
                else_expression: Some(else_expression),
                ..
            } => {
                let Some(condition) =
                    self.build_static_expression_term(condition_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(then_value) =
                    self.build_static_expression_term(then_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(else_value) =
                    self.build_static_expression_term(else_expression.into_global(module))?
                else {
                    return Ok(None);
                };

                Some(StaticTerm::Conditional {
                    condition: self.terms.push(condition),
                    then_value: self.terms.push(then_value),
                    else_value: self.terms.push(else_value),
                })
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => self.build_static_layout_call_term(
                module,
                source,
                left,
                &generic_arguments,
                &arguments,
            )?,
            _ => self
                .build_static_expression_value_from_node(module, expression_id)?
                .map(StaticTerm::Literal),
        };

        Ok(term)
    }

    /// Return one type-space expression as a static term.
    fn build_static_type_expression_term(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let type_expression = self.module(module).view().get(value).clone();
        let term = match type_expression {
            dir::TypeExpression::Parenthesized { expression } => {
                return self.build_static_type_expression_term(module, expression);
            }
            dir::TypeExpression::ScalarLiteral { value } => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value })
            }
            dir::TypeExpression::Literal { value } => {
                StaticTerm::Literal(dir::StaticTerm::TypeLiteral { value })
            }
            dir::TypeExpression::Extends { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Extends,
                left: self.intern_node_type_variable(module, left.into_global_any(module)),
                right: self.intern_node_type_variable(module, right.into_global_any(module)),
            },
            dir::TypeExpression::Implements { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Implements,
                left: self.intern_node_type_variable(module, left.into_global_any(module)),
                right: self.intern_node_type_variable(module, right.into_global_any(module)),
            },
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let condition = StaticTerm::TypeRelation {
                    relation: TypeRelation::Extends,
                    left: self.intern_node_type_variable(module, left.into_global_any(module)),
                    right: self
                        .intern_node_type_variable(module, extends_type.into_global_any(module)),
                };
                let Some(then_value) = self.build_static_type_expression_term(module, then_type)?
                else {
                    return Ok(None);
                };
                let Some(else_value) = self.build_static_type_expression_term(module, else_type)?
                else {
                    return Ok(None);
                };

                StaticTerm::Conditional {
                    condition: self.terms.push(condition),
                    then_value: self.terms.push(then_value),
                    else_value: self.terms.push(else_value),
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return one locally concrete static expression value by local expression id.
    fn build_static_expression_value_from_node(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let expression_node = self.module(module).view().get(expression).clone();
        let term = match expression_node {
            dir::Expression::Identifier { name } => {
                self.build_static_reference_value(module, expression, name)?
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.build_static_path_value(module, expression, &path)?
            }
            dir::Expression::ScalarLiteral(value) => Some(dir::StaticTerm::ScalarLiteral { value }),
            dir::Expression::ObjectExpression { properties } => {
                self.build_static_object_value(module, &properties)?
            }
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static reference expression value.
    fn build_static_reference_value(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(symbol) = self
            .lookup_symbol_by_name(module, expression.into_any(), name, dir::SymbolSpace::Value)
            .unique_symbol()
        else {
            return Ok(None);
        };

        self.build_static_symbol_value(module, symbol)
    }

    /// Return one locally concrete static path expression value.
    fn build_static_path_value(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let symbol = match self.lookup_path_symbol(
            module,
            expression.into_any(),
            path,
            dir::SymbolSpace::Value,
        )? {
            ExportLookup::Found(symbol) => symbol,
            ExportLookup::Missing | ExportLookup::Ambiguous(_) => return Ok(None),
        };

        self.build_static_symbol_value(module, symbol)
    }

    /// Return one locally concrete static symbol expression value.
    fn build_static_symbol_value(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(variable) = self.generic_static_variable(module, symbol)? else {
            return Ok(None);
        };
        let term = match self.solved_static_term(variable)? {
            Some(StaticTerm::Literal(term)) => Some(term),
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static object expression value.
    fn build_static_object_value(
        &mut self,
        module: ModuleId,
        properties: &[dir::LocalNodeId<dir::Property>],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let view = self.module(module).view();
        let mut fields = Vec::with_capacity(properties.len());

        // collect field keys and value expressions
        for property in properties {
            let dir::Property::Field { key, value, .. } = view.get(*property) else {
                return Ok(None);
            };
            let Some(key) = key.static_key(view.tree()) else {
                return Ok(None);
            };

            fields.push((key, *value));
        }

        let mut terms = Vec::with_capacity(properties.len());

        // solve statically known field values
        for (key, value) in fields {
            let Some(value) = self.build_static_expression_value_from_node(module, value)? else {
                return Ok(None);
            };

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Ok(Some(dir::StaticTerm::Object { properties: terms }))
    }

    /// Return a layout query term for one static reflection call.
    fn build_static_layout_call_term(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        left: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<StaticTerm>> {
        if !arguments.is_empty() {
            return Ok(None);
        }
        let Some(symbol) = self.static_call_symbol(module, left)? else {
            return Ok(None);
        };
        let query = match self.environment.language.item(symbol) {
            Some(dir::LanguageItem::SizeOf) => LayoutQuery::Size,
            Some(dir::LanguageItem::AlignOf) => LayoutQuery::Alignment,
            Some(dir::LanguageItem::StrideOf) => LayoutQuery::Stride,
            _ => return Ok(None),
        };
        let Some(target) = self.single_type_generic_argument(module, generic_arguments)? else {
            return Ok(None);
        };
        let layout = LayoutTerm {
            source,
            target,
            query,
        };

        Ok(Some(StaticTerm::Layout(layout)))
    }

    /// Return the symbol named by one static call callee.
    fn static_call_symbol(
        &mut self,
        module: ModuleId,
        left: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let left_node = self.module(module).view().get(left).clone();
        let symbol = match left_node {
            dir::Expression::Identifier { name } => self
                .lookup_symbol_by_name(module, left.into_any(), name, dir::SymbolSpace::Value)
                .unique_symbol(),
            dir::Expression::QualifiedReference { path, .. } => {
                match self.lookup_path_symbol(
                    module,
                    left.into_any(),
                    &path,
                    dir::SymbolSpace::Value,
                )? {
                    ExportLookup::Found(symbol) => Some(symbol),
                    ExportLookup::Missing | ExportLookup::Ambiguous(_) => None,
                }
            }
            dir::Expression::Parenthesized { expression } => {
                return self.static_call_symbol(module, expression);
            }
            _ => None,
        };

        Ok(symbol)
    }

    /// Return the one type argument used by a static reflection call.
    fn single_type_generic_argument(
        &mut self,
        module: ModuleId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<VariableId>> {
        let [argument] = generic_arguments else {
            return Ok(None);
        };
        let argument_node = self.module(module).view().get(*argument).clone();
        let (dir::GenericArgument::Type { value }
        | dir::GenericArgument::AssociatedType { value, .. }) = argument_node
        else {
            return Ok(None);
        };
        let target = self.intern_local_node_type_variable(module, value);

        Ok(Some(target))
    }
}
