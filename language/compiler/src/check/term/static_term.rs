use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckState, Decision, GenericSubstitution, LayoutTerm, Reduction, StaticRelation,
    TermId, TypeRelation, VariableId,
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
        /// The source member expression when available.
        source: Option<dir::GlobalNodeIdAny>,
        /// The owner type.
        owner: VariableId,
        /// The selected member key.
        key: dir::StaticKey,
        /// The applied static arguments.
        arguments: Vec<TermId<ArgumentTerm>>,
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
        arguments: SmallVec<[TermId<ArgumentTerm>; 4]>,
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
                        .flat_map(|argument| state.argument_variables(*argument)),
                );
            }
            Self::Join { elements } => variables.extend(elements.iter().copied()),
            Self::Layout(layout) => variables.extend(layout.referenced_variables()),
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(
                    arguments
                        .iter()
                        .flat_map(|argument| state.argument_variables(*argument)),
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
            Self::Literal(_) | Self::Expression(_) => {}
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
            StaticTerm::Literal(dir::StaticTerm::Symbol { symbol }) => {
                if let Some(argument) = state.substitution_static_symbol(substitution, *symbol) {
                    if let Some(term) = state.solved_static_term(argument)? {
                        term
                    } else {
                        self.clone()
                    }
                } else {
                    self.clone()
                }
            }
            StaticTerm::Literal(dir::StaticTerm::Lifetime {
                lifetime: dir::Lifetime::Symbol(symbol),
            }) => {
                if let Some(argument) = state.substitution_static_symbol(substitution, *symbol) {
                    if let Some(term) = state.solved_static_term(argument)? {
                        term
                    } else {
                        self.clone()
                    }
                } else {
                    self.clone()
                }
            }
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
                if let Some(argument) = state.substitution_static_variable(substitution, *variable) {
                    StaticTerm::Variable(argument)
                } else if let Some(term) = state.solved_static_term(*variable)? {
                    term.substitute(module, substitution, state)?
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
        let term = self.terms.get(term).substitute(module, substitution, self)?;
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
        };

        Ok(decision)
    }

    /// Decide one solved static relation.
    pub(in crate::check) fn decide_static_relation(
        &self,
        relation: StaticRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<Decision> {
        let Some(left) = self.solved_static_term(left)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(right) = self.solved_static_term(right)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_static_term_relation(relation, &left, &right)
    }

    /// Reduce one static term when the solver has enough input.
    pub(in crate::check) fn reduce_static_term(
        &mut self,
        module: ModuleId,
        term: &StaticTerm,
    ) -> CompilerResult<Option<StaticTerm>> {
        let term = match term {
            StaticTerm::Variable(variable) => {
                let Some(term) = self.solved_static_term(*variable)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Expression(expression) => {
                let Some(term) = self.build_static_expression_term(expression.clone())? else {
                    return Ok(None);
                };

                let Some(term) = self.reduce_static_term(module, &term)? else {
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
                let owner = match self.reduce_type_term(module, &owner)? {
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

                let Some(term) = self.reduce_static_term(module, &term)? else {
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
                let left = self.terms.get(*left);
                let Some(left) = self.reduce_static_term(module, &left)? else {
                    return Ok(None);
                };
                let right = self.terms.get(*right);
                let Some(right) = self.reduce_static_term(module, &right)? else {
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
                let condition = self.terms.get(*condition);
                let Some(condition) = self.reduce_static_term(module, &condition)? else {
                    return Ok(None);
                };

                match condition {
                    StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                        value: dir::ScalarLiteral::Boolean(true),
                    }) => {
                        let then_value = self.terms.get(*then_value);
                        let Some(term) = self.reduce_static_term(module, &then_value)? else {
                            return Ok(None);
                        };

                        term
                    }
                    StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                        value: dir::ScalarLiteral::Boolean(false),
                    }) => {
                        let else_value = self.terms.get(*else_value);
                        let Some(term) = self.reduce_static_term(module, &else_value)? else {
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
            (StaticTerm::Expression(_), _)
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

    /// Reduce one static join.
    fn reduce_static_join(
        &mut self,
        module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let mut lifetimes = Vec::with_capacity(elements.len());

        // collect solved lifetime elements
        for element in elements {
            let Some(term) = self.static_value(*element)? else {
                return Ok(None);
            };
            let dir::StaticTerm::Lifetime { .. } = term else {
                return Ok(None);
            };
            let lifetime = self.intern_static(module, term);

            lifetimes.push(lifetime);
        }

        Ok(Some(dir::StaticTerm::Lifetime {
            lifetime: dir::Lifetime::Join(lifetimes),
        }))
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
