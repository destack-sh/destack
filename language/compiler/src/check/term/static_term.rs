use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckComponentState, Decision, GenericSubstitution, LayoutTerm,
    StaticOperationTerm, StaticRelation, VariableId,
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
        arguments: Vec<ArgumentTerm>,
    },
    /// Static value operation.
    ///
    /// ```ts
    /// L | R
    /// ```
    Operation(StaticOperationTerm),
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
        arguments: SmallVec<[ArgumentTerm; 4]>,
    },
}

impl StaticTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
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
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
            }
            Self::Operation(operation) => variables.extend(operation.referenced_variables()),
            Self::Layout(layout) => variables.extend(layout.referenced_variables()),
            Self::Intrinsic { item: _, arguments } => {
                variables.extend(arguments.iter().map(ArgumentTerm::variable));
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
        state: &mut CheckComponentState<'_>,
    ) -> CompilerResult<StaticTerm> {
        let term = match self {
            StaticTerm::Literal(dir::StaticTerm::Symbol { symbol }) => {
                if let Some(argument) = substitution.static_symbol(*symbol) {
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
                if let Some(argument) = substitution.static_symbol(*symbol) {
                    if let Some(term) = state.solved_static_term(argument)? {
                        term
                    } else {
                        self.clone()
                    }
                } else {
                    self.clone()
                }
            }
            StaticTerm::Operation(operation) => {
                StaticTerm::Operation(operation.substitute(module, substitution, state)?)
            }
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
                arguments: ArgumentTerm::substitute_all(arguments, module, substitution, state)?
                    .into(),
            },
            StaticTerm::Intrinsic { item, arguments } => StaticTerm::Intrinsic {
                item: *item,
                arguments: ArgumentTerm::substitute_all(arguments, module, substitution, state)?
                    .into(),
            },
            StaticTerm::Variable(_) | StaticTerm::Expression(_) | StaticTerm::Literal(_) => {
                self.clone()
            }
        };

        Ok(term)
    }
}

impl CheckComponentState<'_> {
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
                if let Some(term) = self.static_expression_term(expression.clone())? {
                    StaticTerm::Literal(term)
                } else {
                    term.clone()
                }
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
                let Some(term) = self.member_static_term(module, &owner, key)? else {
                    return Ok(None);
                };

                term
            }
            StaticTerm::Operation(operation) => {
                let Some(term) = self.reduce_static_operation(module, operation)? else {
                    return Ok(None);
                };

                StaticTerm::Literal(term)
            }
            StaticTerm::Layout(layout) => {
                let Some(term) = self.reduce_layout_static(module, layout)? else {
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
            | (StaticTerm::Operation(_), _)
            | (_, StaticTerm::Operation(_))
            | (StaticTerm::Layout(_), _)
            | (_, StaticTerm::Layout(_))
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. }) => Decision::Undecidable,
        };

        Ok(decision)
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
