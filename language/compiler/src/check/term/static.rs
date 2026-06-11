use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, Dependency, DependencyCollector, GenericArgument,
    GenericParameterId, LayoutQuery, LayoutTerm, NameLookup, Obligation, Origin, PathLookup,
    StaticOperand, StaticRelation, SubstitutionSet, TermId, TypeOperand, TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// Term used to define a static variable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum StaticTerm {
    /// Committed static value.
    ///
    /// ```ds
    /// import { N } from "./dependency"
    /// ```
    Static(dir::GlobalStaticId),
    /// Concrete static term.
    ///
    /// ```ds
    /// "mutable"
    /// ```
    Literal(dir::StaticTerm),
    /// Static generic parameter.
    ///
    /// ```ds
    /// <comptime N: uint>
    /// ```
    Parameter(GenericParameterId),
    /// Source expression evaluated as a static term.
    ///
    /// ```ds
    /// N + 1
    /// ```
    Expression(dir::GlobalNodeId<dir::Expression>),
    /// Static member projection.
    ///
    /// ```ds
    /// Register.Width
    /// ```
    Member {
        /// The source member expression.
        source: dir::GlobalNodeIdAny,
        /// The owner type.
        owner: TypeOperand,
        /// The selected member key.
        key: dir::StaticKey,
        /// The applied static arguments.
        arguments: SmallVec<[GenericArgument; 2]>,
    },
    /// Concrete layout query.
    ///
    /// ```ds
    /// sizeOf<T>()
    /// ```
    Layout(LayoutTerm),
    /// Compiler intrinsic returning a static value.
    ///
    /// ```ds
    /// LifetimeOf<T>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[GenericArgument; 2]>,
    },
    /// Static equality comparison.
    ///
    /// ```ds
    /// this.Width == 4
    /// ```
    Equal {
        /// The left static value.
        left: StaticOperand,
        /// The right static value.
        right: StaticOperand,
        /// Whether the equality result is negated.
        is_negated: bool,
    },
    /// Type relation used as a static boolean.
    ///
    /// ```ds
    /// T extends string
    /// ```
    TypeRelation {
        /// The required type relation.
        relation: TypeRelation,
        /// The left type.
        left: TypeOperand,
        /// The right type.
        right: TypeOperand,
    },
    /// Static conditional value.
    ///
    /// ```ds
    /// C ? T : F
    /// ```
    Conditional {
        /// The static boolean condition.
        condition: StaticOperand,
        /// The value selected when the condition holds.
        then_value: StaticOperand,
        /// The value selected when the condition does not hold.
        else_value: StaticOperand,
    },
    /// Static value union.
    ///
    /// ```ds
    /// L | R
    /// ```
    Union {
        /// The union elements.
        elements: Vec<StaticOperand>,
    },
}

impl StaticTerm {
    /// Return dependencies referenced by this static term.
    pub(in crate::check) fn dependencies(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[Dependency; 2]> {
        DependencyCollector::from_static_term(state, self)
    }

    /// Return whether this term can be affected by one substitution.
    pub(in crate::check) fn needs_substitution(
        &self,
        substitution: &SubstitutionSet,
        state: &CheckState<'_>,
    ) -> CompilerResult<bool> {
        let needs_substitution = match self {
            Self::Parameter(parameter) => state
                .substitution_static_operand(substitution, *parameter)
                .is_some(),
            Self::Literal(_) | Self::Static(_) => false,
            Self::Expression(_)
            | Self::Member { .. }
            | Self::Layout(_)
            | Self::Intrinsic { .. }
            | Self::Equal { .. }
            | Self::TypeRelation { .. }
            | Self::Conditional { .. }
            | Self::Union { .. } => true,
        };

        Ok(needs_substitution)
    }

    /// Return the direct operand replacement for this term.
    pub(in crate::check) fn direct_substitution(
        &self,
        substitution: &SubstitutionSet,
        state: &CheckState<'_>,
    ) -> Option<StaticOperand> {
        let Self::Parameter(parameter) = self else {
            return None;
        };

        state.substitution_static_operand(substitution, *parameter)
    }
}

impl CheckState<'_> {
    /// Return one term id for a resolved static operand.
    pub(in crate::check) fn static_operand_term_id(
        &mut self,
        operand: StaticOperand,
    ) -> Option<TermId<StaticTerm>> {
        let operand = self.resolved_static_operand(operand)?;

        match operand {
            StaticOperand::Term(term) => Some(term),
            StaticOperand::Static(value) => {
                let term = self.inference.push_term(StaticTerm::Static(value));

                Some(term)
            }
            StaticOperand::Variable(_) => None,
        }
    }

    /// Decide one static term relation.
    pub(in crate::check) fn decide_static_term_relation(
        &self,
        relation: StaticRelation,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Answer<bool>> {
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
    ) -> CompilerResult<Answer<bool>> {
        let left = left.into();
        let right = right.into();
        let Some(left) = self.resolved_static_operand(left) else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Some(right) = self.resolved_static_operand(right) else {
            return Ok(Answer::pending(right.dependencies(self)));
        };

        match (left, right) {
            (StaticOperand::Term(left), StaticOperand::Term(right)) => {
                let left = self.inference.term(left);
                let right = self.inference.term(right);

                self.decide_static_term_relation(relation, left, right)
            }
            (StaticOperand::Term(left), StaticOperand::Static(right)) => {
                let left = self.inference.term(left);
                let right = StaticTerm::Static(right);

                self.decide_static_term_relation(relation, left, &right)
            }
            (StaticOperand::Static(left), StaticOperand::Term(right)) => {
                let left = StaticTerm::Static(left);
                let right = self.inference.term(right);

                self.decide_static_term_relation(relation, &left, right)
            }
            (StaticOperand::Static(left), StaticOperand::Static(right)) => {
                let left = StaticTerm::Static(left);
                let right = StaticTerm::Static(right);

                self.decide_static_term_relation(relation, &left, &right)
            }
            (StaticOperand::Variable(variable), _) | (_, StaticOperand::Variable(variable)) => {
                Ok(Answer::pending([Dependency::Variable(variable)]))
            }
        }
    }

    /// Constrain one static value by its declared type.
    pub(in crate::check) fn constrain_static_value_type(
        &mut self,
        origin: Origin,
        value: StaticOperand,
        ty: TypeOperand,
    ) -> CompilerResult<()> {
        let Answer::Ready(value_ty) = self.static_value_type(origin, value)? else {
            return Ok(());
        };

        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            value_ty,
            ty,
            Condition::Always,
        );

        Ok(())
    }

    /// Decide whether one static value has one declared type.
    pub(in crate::check) fn decide_static_value_type(
        &mut self,
        origin: Origin,
        value: StaticOperand,
        ty: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Answer::Ready(value_ty) = self.static_value_type(origin, value)? else {
            return Ok(Answer::pending(value.dependencies(self)));
        };
        let Answer::Ready(value_ty) = self.reduce_type_operand(origin, value_ty)? else {
            return Ok(Answer::pending(value_ty.dependencies(self)));
        };
        let Answer::Ready(ty) = self.reduce_type_operand(origin, ty)? else {
            return Ok(Answer::pending(ty.dependencies(self)));
        };

        self.decide_type_relation(TypeRelation::Assignable, value_ty, ty)
    }

    /// Return the type of one static value when it is known.
    fn static_value_type(
        &mut self,
        origin: Origin,
        value: StaticOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let module = origin.module();
        let Answer::Ready(value) = self.reduce_static_operand(origin, value)? else {
            return Ok(Answer::pending(value.dependencies(self)));
        };

        // use the declared value type of static generic parameters
        if let StaticOperand::Term(term) = value
            && let StaticTerm::Parameter(parameter) = self.inference.term(term)
        {
            let generic = self.inference.generic_parameter_binding(*parameter)?;
            let Some(ty) = generic.static_ty() else {
                return Err(CompilerError::Internal {
                    message: "static parameter term references a type generic parameter"
                        .to_string(),
                });
            };

            return Ok(Answer::Ready(ty));
        }

        // otherwise reduce the concrete static value to its singleton type
        let Answer::Ready(ty) = self.reduce_static_value_term(module, value)? else {
            return Ok(Answer::pending(value.dependencies(self)));
        };
        let ty = self.inference.push_term(ty);

        Ok(Answer::Ready(ty.into()))
    }

    /// Reduce one static operand when the solver has enough input.
    pub(in crate::check) fn reduce_static_operand(
        &mut self,
        origin: Origin,
        operand: StaticOperand,
    ) -> CompilerResult<Answer<StaticOperand>> {
        let Some(operand) = self.resolved_static_operand(operand) else {
            return Ok(Answer::pending(operand.dependencies(self)));
        };

        match operand {
            StaticOperand::Term(term) => self.reduce_static_term(origin, term),
            StaticOperand::Static(_) => Ok(Answer::Ready(operand)),
            StaticOperand::Variable(variable) => {
                Ok(Answer::pending([Dependency::Variable(variable)]))
            }
        }
    }

    /// Reduce one static term id when the solver has enough input.
    pub(in crate::check) fn reduce_static_term(
        &mut self,
        origin: Origin,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<Answer<StaticOperand>> {
        let module = origin.module();

        match self.inference.term(term) {
            &StaticTerm::Parameter(_) | &StaticTerm::Literal(_) => Ok(Answer::Ready(term.into())),
            &StaticTerm::Static(value) => Ok(Answer::Ready(StaticOperand::Static(value))),
            &StaticTerm::Expression(expression) => {
                let Some(value) = self.static_expression_term(expression)? else {
                    return Ok(Answer::pending(
                        StaticOperand::Term(term).dependencies(self),
                    ));
                };
                let value = self.inference.push_term(value);

                self.reduce_static_term(origin, value)
            }
            &StaticTerm::Member {
                source: _,
                owner,
                key,
                ref arguments,
            } if arguments.is_empty() => {
                let Answer::Ready(owner) = self.reduce_type_operand(origin, owner)? else {
                    return Ok(Answer::pending(owner.dependencies(self)));
                };
                let Some(owner) = self.resolved_type_operand(owner) else {
                    return Ok(Answer::pending(owner.dependencies(self)));
                };
                let Some(value) = self.member_static_operand(module, owner, key)? else {
                    return Ok(Answer::pending(owner.dependencies(self)));
                };

                self.reduce_static_operand(origin, value)
            }
            StaticTerm::Member { .. } => Err(CompilerError::Internal {
                message: "static member term with arguments cannot be reduced".into(),
            }),
            StaticTerm::Union { elements: _ } => self.reduce_static_union(origin, term),
            &StaticTerm::Layout(layout) => {
                let Answer::Ready(value) = self.reduce_layout_term(module, layout)? else {
                    return Ok(Answer::pending(
                        StaticOperand::Term(term).dependencies(self),
                    ));
                };
                let value = self.inference.push_term(StaticTerm::Literal(value));

                Ok(Answer::Ready(value.into()))
            }
            &StaticTerm::Intrinsic { item, arguments: _ } => {
                let Some(value) = self.memory_static_value_from_term(module, item, term)? else {
                    return Ok(Answer::pending(
                        StaticOperand::Term(term).dependencies(self),
                    ));
                };
                let value = self.inference.push_term(StaticTerm::Literal(value));

                Ok(Answer::Ready(value.into()))
            }
            &StaticTerm::Equal {
                left,
                right,
                is_negated,
            } => {
                let Answer::Ready(left) = self.reduce_static_operand(origin, left)? else {
                    return Ok(Answer::pending(left.dependencies(self)));
                };
                let Answer::Ready(right) = self.reduce_static_operand(origin, right)? else {
                    return Ok(Answer::pending(right.dependencies(self)));
                };
                let decision = self.decide_static_relation(StaticRelation::Equal, left, right)?;
                let value = match decision {
                    Answer::Ready(true) => !is_negated,
                    Answer::Ready(false) => is_negated,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                let value =
                    self.inference
                        .push_term(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                            value: dir::ScalarLiteral::Boolean(value),
                        }));

                Ok(Answer::Ready(value.into()))
            }
            &StaticTerm::TypeRelation {
                relation,
                left,
                right,
            } => {
                let decision = self.decide_type_relation(relation, left, right)?;
                let value = match decision {
                    Answer::Ready(value) => value,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                let value =
                    self.inference
                        .push_term(StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                            value: dir::ScalarLiteral::Boolean(value),
                        }));

                Ok(Answer::Ready(value.into()))
            }
            &StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let Answer::Ready(condition) = self.reduce_static_operand(origin, condition)?
                else {
                    return Ok(Answer::pending(condition.dependencies(self)));
                };

                match self.static_operand_boolean(condition) {
                    Some(true) => self.reduce_static_operand(origin, then_value),
                    Some(false) => self.reduce_static_operand(origin, else_value),
                    None => Err(CompilerError::Internal {
                        message: "static conditional condition is not boolean".into(),
                    }),
                }
            }
        }
    }

    /// Return one static boolean operand value.
    pub(in crate::check) fn static_operand_boolean(&self, operand: StaticOperand) -> Option<bool> {
        match operand {
            StaticOperand::Term(term) => match self.inference.term(term) {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::Boolean(value),
                }) => Some(*value),
                _ => None,
            },
            StaticOperand::Static(_) | StaticOperand::Variable(_) => None,
        }
    }

    /// Decide exact static equality.
    fn decide_static_equal(
        &self,
        left: &StaticTerm,
        right: &StaticTerm,
    ) -> CompilerResult<Answer<bool>> {
        if left == right {
            return Ok(Answer::Ready(true));
        }

        let decision = match (left, right) {
            (StaticTerm::Literal(left), StaticTerm::Literal(right)) => {
                if left == right {
                    Answer::Ready(true)
                } else {
                    Answer::Ready(false)
                }
            }
            (StaticTerm::Parameter(_), _)
            | (_, StaticTerm::Parameter(_))
            | (StaticTerm::Static(_), _)
            | (_, StaticTerm::Static(_))
            | (StaticTerm::Expression(_), _)
            | (_, StaticTerm::Expression(_))
            | (StaticTerm::Member { .. }, _)
            | (_, StaticTerm::Member { .. })
            | (StaticTerm::Union { .. }, _)
            | (_, StaticTerm::Union { .. })
            | (StaticTerm::Layout(_), _)
            | (_, StaticTerm::Layout(_))
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. })
            | (StaticTerm::Equal { .. }, _)
            | (_, StaticTerm::Equal { .. })
            | (StaticTerm::TypeRelation { .. }, _)
            | (_, StaticTerm::TypeRelation { .. })
            | (StaticTerm::Conditional { .. }, _)
            | (_, StaticTerm::Conditional { .. }) => self.unresolved_static_relation(left, right),
        };

        Ok(decision)
    }

    /// Decide static assignability.
    fn decide_static_assignable(
        &self,
        source: &StaticTerm,
        target: &StaticTerm,
    ) -> CompilerResult<Answer<bool>> {
        if self.decide_static_equal(source, target)? == Answer::Ready(true) {
            return Ok(Answer::Ready(true));
        }

        let decision = match (source, target) {
            (
                StaticTerm::Literal(dir::StaticTerm::Access { access: source }),
                StaticTerm::Literal(dir::StaticTerm::Access { access: target }),
            ) => self.decide_access_term_assignable(*source, *target),
            (
                StaticTerm::Parameter(parameter),
                StaticTerm::Literal(dir::StaticTerm::Access { access: target }),
            ) => self.decide_access_parameter_assignable(*parameter, *target)?,
            (
                StaticTerm::Literal(dir::StaticTerm::Lifetime { lifetime: source }),
                StaticTerm::Literal(dir::StaticTerm::Lifetime { lifetime: target }),
            ) => self.decide_lifetime_term_assignable(source, target),
            (StaticTerm::Union { elements }, target) => {
                self.decide_static_union_assignable_to_term(elements, target)?
            }
            (source, StaticTerm::Union { elements }) => {
                self.decide_static_term_assignable_to_union(source, elements)?
            }
            (StaticTerm::Literal(_), StaticTerm::Literal(_)) => Answer::Ready(false),
            (StaticTerm::Parameter(_), _)
            | (_, StaticTerm::Parameter(_))
            | (StaticTerm::Static(_), _)
            | (_, StaticTerm::Static(_))
            | (StaticTerm::Expression(_), _)
            | (_, StaticTerm::Expression(_))
            | (StaticTerm::Member { .. }, _)
            | (_, StaticTerm::Member { .. })
            | (StaticTerm::Layout(_), _)
            | (_, StaticTerm::Layout(_))
            | (StaticTerm::Intrinsic { .. }, _)
            | (_, StaticTerm::Intrinsic { .. })
            | (StaticTerm::Equal { .. }, _)
            | (_, StaticTerm::Equal { .. })
            | (StaticTerm::TypeRelation { .. }, _)
            | (_, StaticTerm::TypeRelation { .. })
            | (StaticTerm::Conditional { .. }, _)
            | (_, StaticTerm::Conditional { .. }) => {
                self.unresolved_static_relation(source, target)
            }
        };

        Ok(decision)
    }

    /// Decide whether every static union element is assignable to one target.
    fn decide_static_union_assignable_to_term(
        &self,
        elements: &[StaticOperand],
        target: &StaticTerm,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);

        // require every source element to fit the target
        for element in elements {
            let Some(element) = self.resolved_static_operand(*element) else {
                decision = decision.and(Answer::pending(element.dependencies(self)));
                continue;
            };
            let element_decision = match element {
                StaticOperand::Term(term) => {
                    self.decide_static_assignable(self.inference.term(term), target)?
                }
                StaticOperand::Static(value) => {
                    let element = StaticTerm::Static(value);

                    self.decide_static_assignable(&element, target)?
                }
                StaticOperand::Variable(variable) => {
                    decision = decision.and(Answer::pending([Dependency::Variable(variable)]));
                    continue;
                }
            };

            match element_decision {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                pending @ Answer::Pending(_) => decision = decision.and(pending),
            }
        }

        Ok(decision)
    }

    /// Decide whether one source is assignable to any static union element.
    fn decide_static_term_assignable_to_union(
        &self,
        source: &StaticTerm,
        elements: &[StaticOperand],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);

        // accept when any target element accepts the source
        for element in elements {
            let Some(element) = self.resolved_static_operand(*element) else {
                decision = decision.or(Answer::pending(element.dependencies(self)));
                continue;
            };
            let element_decision = match element {
                StaticOperand::Term(term) => {
                    self.decide_static_assignable(source, self.inference.term(term))?
                }
                StaticOperand::Static(value) => {
                    let element = StaticTerm::Static(value);

                    self.decide_static_assignable(source, &element)?
                }
                StaticOperand::Variable(variable) => {
                    decision = decision.or(Answer::pending([Dependency::Variable(variable)]));
                    continue;
                }
            };

            match element_decision {
                Answer::Ready(true) => return Ok(Answer::Ready(true)),
                Answer::Ready(false) => {}
                pending @ Answer::Pending(_) => decision = decision.or(pending),
            }
        }

        Ok(decision)
    }

    /// Decide access capability assignability.
    fn decide_access_term_assignable(
        &self,
        source: dir::Access,
        target: dir::Access,
    ) -> Answer<bool> {
        if Self::access_rank(source) >= Self::access_rank(target) {
            Answer::Ready(true)
        } else {
            Answer::Ready(false)
        }
    }

    /// Decide whether one access parameter is assignable to one access literal.
    fn decide_access_parameter_assignable(
        &self,
        parameter: GenericParameterId,
        target: dir::Access,
    ) -> CompilerResult<Answer<bool>> {
        let parameter = self.inference.generic_parameter_binding(parameter)?;
        let Some(ty) = parameter.static_ty() else {
            return Err(CompilerError::Internal {
                message: "access parameter relation received a type generic parameter".to_string(),
            });
        };

        // non-access static parameters cannot satisfy access literals
        if !self.type_operand_references_language_item(ty, dir::LanguageItem::Access)? {
            return Ok(Answer::Ready(false));
        }

        // every access mode can be read
        if target == dir::Access::Readonly {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::Ready(false))
        }
    }

    /// Return a closed or blocked answer for unreduced static terms.
    fn unresolved_static_relation(&self, left: &StaticTerm, right: &StaticTerm) -> Answer<bool> {
        let dependencies = left
            .dependencies(self)
            .into_iter()
            .chain(right.dependencies(self))
            .collect::<SmallVec<[_; 2]>>();

        if dependencies.is_empty() {
            Answer::Ready(false)
        } else {
            Answer::pending(dependencies)
        }
    }

    /// Return whether one type operand is a reference to one language item.
    fn type_operand_references_language_item(
        &self,
        operand: TypeOperand,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let references_item = match operand {
            TypeOperand::Variable(variable) => {
                let Some(operand) = self.resolved_type_variable(variable) else {
                    return Ok(false);
                };

                match operand {
                    TypeOperand::Term(term) => {
                        let term = self.inference.term(term);

                        self.type_term_references_language_item(term, item)
                    }
                    TypeOperand::Type(ty) => self.committed_type_references_language_item(ty, item),
                    TypeOperand::Variable(_) => false,
                }
            }
            TypeOperand::Term(term) => {
                let term = self.inference.term(term);

                self.type_term_references_language_item(term, item)
            }
            TypeOperand::Type(ty) => self.committed_type_references_language_item(ty, item),
        };

        Ok(references_item)
    }

    /// Return whether one type term is a reference to one language item.
    fn type_term_references_language_item(&self, term: &TypeTerm, item: dir::LanguageItem) -> bool {
        match term {
            TypeTerm::Reference { symbol, .. } => self.is_language_symbol(*symbol, item),
            TypeTerm::Type(ty) => self.committed_type_references_language_item(*ty, item),
            _ => false,
        }
    }

    /// Return whether one committed type is a reference to one language item.
    fn committed_type_references_language_item(
        &self,
        ty: dir::GlobalTypeId,
        item: dir::LanguageItem,
    ) -> bool {
        let dir::Type::Reference(reference) = self.r#type(ty) else {
            return false;
        };

        self.is_language_symbol(reference.symbol, item)
    }

    /// Return the access value represented by one static term.
    fn static_access(&self, term: &StaticTerm) -> Option<dir::Access> {
        match term {
            StaticTerm::Literal(dir::StaticTerm::Access { access }) => Some(*access),
            _ => None,
        }
    }

    /// Return the access value represented by one static operand.
    fn static_operand_access(&self, operand: StaticOperand) -> Option<dir::Access> {
        match operand {
            StaticOperand::Term(term) => self.static_access(self.inference.term(term)),
            StaticOperand::Static(_) | StaticOperand::Variable(_) => None,
        }
    }

    /// Decide lifetime assignability.
    fn decide_lifetime_term_assignable(
        &self,
        source: &dir::Lifetime,
        target: &dir::Lifetime,
    ) -> Answer<bool> {
        if source == target || matches!(source, dir::Lifetime::Static) {
            Answer::Ready(true)
        } else {
            Answer::Ready(false)
        }
    }

    /// Return whether one static term is a lifetime value.
    fn static_is_lifetime(&self, term: &StaticTerm) -> bool {
        matches!(term, StaticTerm::Literal(dir::StaticTerm::Lifetime { .. }))
    }

    /// Return whether one static operand is a lifetime value.
    fn static_operand_is_lifetime(&self, operand: StaticOperand) -> bool {
        match operand {
            StaticOperand::Term(term) => self.static_is_lifetime(self.inference.term(term)),
            StaticOperand::Static(_) | StaticOperand::Variable(_) => false,
        }
    }

    /// Return one access capability rank.
    fn access_rank(access: dir::Access) -> u8 {
        match access {
            dir::Access::Readonly => 0,
            dir::Access::Mutable => 1,
            dir::Access::Exclusive => 2,
        }
    }

    /// Return a solved static term when the static domain can decide the bounds.
    pub(in crate::check) fn static_bound_solution(
        &mut self,
        lower_bounds: &[StaticOperand],
        upper_bounds: &[StaticOperand],
        lower_operands: &[StaticOperand],
        upper_operands: &[StaticOperand],
    ) -> Option<StaticOperand> {
        if let Some(operand) = self.access_bound_solution(lower_operands, upper_operands) {
            return Some(operand);
        }
        if let Some(operand) =
            self.lifetime_bound_solution(lower_bounds, upper_bounds, lower_operands, upper_operands)
        {
            return Some(operand);
        }

        Self::single_static_bound_operand(lower_operands)
    }

    /// Return the least valid access solution for solved access bounds.
    fn access_bound_solution(
        &mut self,
        lower_operands: &[StaticOperand],
        upper_operands: &[StaticOperand],
    ) -> Option<StaticOperand> {
        if lower_operands.is_empty() && upper_operands.is_empty() {
            return None;
        }
        let mut lower_access = dir::Access::Readonly;
        let mut upper_access = dir::Access::Exclusive;
        let mut saw_access = false;

        // collect minimum required access
        for operand in lower_operands {
            let access = self.static_operand_access(*operand)?;
            lower_access = Self::stricter_access(lower_access, access);
            saw_access = true;
        }

        // collect maximum allowed access
        for operand in upper_operands {
            let access = self.static_operand_access(*operand)?;
            upper_access = Self::weaker_access(upper_access, access);
            saw_access = true;
        }
        if !saw_access || Self::access_rank(lower_access) > Self::access_rank(upper_access) {
            return None;
        }

        // preserve exact source access when inference only has an upper bound
        let access = if lower_operands.is_empty() {
            upper_access
        } else {
            lower_access
        };

        let term = self
            .inference
            .push_term(StaticTerm::Literal(dir::StaticTerm::Access { access }));

        Some(term.into())
    }

    /// Return an exact or joined lifetime solution for solved lifetime bounds.
    fn lifetime_bound_solution(
        &mut self,
        lower_bounds: &[StaticOperand],
        upper_bounds: &[StaticOperand],
        lower_operands: &[StaticOperand],
        upper_operands: &[StaticOperand],
    ) -> Option<StaticOperand> {
        if !lower_operands
            .iter()
            .all(|operand| self.static_operand_is_lifetime(*operand))
            || !upper_operands
                .iter()
                .all(|operand| self.static_operand_is_lifetime(*operand))
        {
            return None;
        }
        if let Some(operand) = Self::single_static_bound_operand(lower_operands) {
            return Some(operand);
        }

        // preserve exact source lifetime when inference only has one upper bound
        if lower_bounds.is_empty()
            && upper_bounds.len() == 1
            && let Some(operand) = upper_operands.first()
        {
            return Some(*operand);
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

            let term = self.inference.push_term(StaticTerm::Union {
                elements: elements.into_iter().map(StaticOperand::from).collect(),
            });

            return Some(term.into());
        }

        None
    }

    /// Return the stricter of two access capabilities.
    fn stricter_access(left: dir::Access, right: dir::Access) -> dir::Access {
        if Self::access_rank(left) >= Self::access_rank(right) {
            left
        } else {
            right
        }
    }

    /// Return the weaker of two access capabilities.
    fn weaker_access(left: dir::Access, right: dir::Access) -> dir::Access {
        if Self::access_rank(left) <= Self::access_rank(right) {
            left
        } else {
            right
        }
    }

    /// Return the single equivalent static bound operand.
    fn single_static_bound_operand(operands: &[StaticOperand]) -> Option<StaticOperand> {
        let first = *operands.first()?;

        for operand in operands.iter().skip(1) {
            if *operand != first {
                return None;
            }
        }

        Some(first)
    }

    /// Reduce one static union.
    fn reduce_static_union(
        &mut self,
        origin: Origin,
        term: TermId<StaticTerm>,
    ) -> CompilerResult<Answer<StaticOperand>> {
        let Some(len) = self.static_union_len(term) else {
            return Err(CompilerError::Internal {
                message: "static union reducer received a non-union term".into(),
            });
        };
        if len == 0 {
            return Err(CompilerError::Internal {
                message: "static union has no elements".into(),
            });
        }
        let mut reduced = Vec::with_capacity(len);

        // reduce each element without owning the stored element list
        for index in 0..len {
            let Some(element) = self.static_union_element(term, index) else {
                return Err(CompilerError::Internal {
                    message: "static union element index is out of range".into(),
                });
            };
            let Answer::Ready(element) = self.reduce_static_operand(origin, element)? else {
                return Ok(Answer::pending(element.dependencies(self)));
            };

            match element {
                StaticOperand::Term(term) => match self.inference.term(term) {
                    StaticTerm::Union { elements } => reduced.extend(elements.iter().copied()),
                    _ => reduced.push(element),
                },
                StaticOperand::Static(_) => reduced.push(element),
                StaticOperand::Variable(variable) => {
                    return Ok(Answer::pending([Dependency::Variable(variable)]));
                }
            }
        }

        let operand = match reduced.as_slice() {
            [element] => *element,
            _ => self
                .inference
                .push_term(StaticTerm::Union { elements: reduced })
                .into(),
        };

        Ok(Answer::Ready(operand))
    }

    /// Return the number of elements in one static union term.
    fn static_union_len(&self, term: TermId<StaticTerm>) -> Option<usize> {
        let StaticTerm::Union { elements } = self.inference.term(term) else {
            return None;
        };

        Some(elements.len())
    }

    /// Return one element from a static union term.
    fn static_union_element(
        &self,
        term: TermId<StaticTerm>,
        index: usize,
    ) -> Option<StaticOperand> {
        let StaticTerm::Union { elements } = self.inference.term(term) else {
            return None;
        };

        elements.get(index).copied()
    }
}

impl CheckState<'_> {
    /// Return one static expression as a solver static term.
    pub(in crate::check) fn static_expression_term(
        &mut self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let module = expression.module_id;
        let expression_id = expression.local_id;
        let expression_node = self.module(module).view().get(expression_id).clone();
        let source = expression.into_any();
        let term = match expression_node {
            dir::Expression::Parenthesized { expression } => {
                self.static_expression_term(expression.into_global(module))?
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
                let owner = self.node_type_operand(left.into_global_any(module))?;

                Some(StaticTerm::Member {
                    source,
                    owner,
                    key: dir::StaticKey::Name(name),
                    arguments: SmallVec::new(),
                })
            }
            dir::Expression::Type { value } => self.static_type_expression_term(module, value)?,
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
                let Some(left) = self.static_expression_term(left.into_global(module))? else {
                    return Ok(None);
                };
                let Some(right) = self.static_expression_term(right.into_global(module))? else {
                    return Ok(None);
                };
                let is_negated = matches!(
                    operator,
                    dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict
                );

                Some(StaticTerm::Equal {
                    left: self.inference.push_term(left).into(),
                    right: self.inference.push_term(right).into(),
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
                    self.static_expression_term(condition_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(then_value) =
                    self.static_expression_term(then_expression.into_global(module))?
                else {
                    return Ok(None);
                };
                let Some(else_value) =
                    self.static_expression_term(else_expression.into_global(module))?
                else {
                    return Ok(None);
                };

                Some(StaticTerm::Conditional {
                    condition: self.inference.push_term(condition).into(),
                    then_value: self.inference.push_term(then_value).into(),
                    else_value: self.inference.push_term(else_value).into(),
                })
            }
            dir::Expression::Call {
                left,
                generic_arguments,
                arguments,
                ..
            } => {
                self.static_layout_call_term(module, source, left, &generic_arguments, &arguments)?
            }
            _ => self
                .static_expression_literal(module, expression_id)?
                .map(StaticTerm::Literal),
        };

        Ok(term)
    }

    /// Return one type-space expression as a static term.
    fn static_type_expression_term(
        &mut self,
        module: ModuleId,
        value: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CompilerResult<Option<StaticTerm>> {
        let type_expression = self.module(module).view().get(value).clone();
        let term = match type_expression {
            dir::TypeExpression::Parenthesized { expression } => {
                return self.static_type_expression_term(module, expression);
            }
            dir::TypeExpression::ScalarLiteral { value } => {
                StaticTerm::Literal(dir::StaticTerm::ScalarLiteral { value })
            }
            dir::TypeExpression::Literal { value } => {
                StaticTerm::Literal(dir::StaticTerm::TypeLiteral { value })
            }
            dir::TypeExpression::Extends { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Extends,
                left: self.node_type_operand(left.into_global_any(module))?,
                right: self.node_type_operand(right.into_global_any(module))?,
            },
            dir::TypeExpression::Implements { left, right } => StaticTerm::TypeRelation {
                relation: TypeRelation::Implements,
                left: self.node_type_operand(left.into_global_any(module))?,
                right: self.node_type_operand(right.into_global_any(module))?,
            },
            dir::TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            } => {
                let condition = StaticTerm::TypeRelation {
                    relation: TypeRelation::Extends,
                    left: self.node_type_operand(left.into_global_any(module))?,
                    right: self.node_type_operand(extends_type.into_global_any(module))?,
                };
                let Some(then_value) = self.static_type_expression_term(module, then_type)? else {
                    return Ok(None);
                };
                let Some(else_value) = self.static_type_expression_term(module, else_type)? else {
                    return Ok(None);
                };

                StaticTerm::Conditional {
                    condition: self.inference.push_term(condition).into(),
                    then_value: self.inference.push_term(then_value).into(),
                    else_value: self.inference.push_term(else_value).into(),
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(term))
    }

    /// Return one locally concrete static expression value by local expression id.
    pub(in crate::check) fn static_expression_literal(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let expression_node = self.module(module).view().get(expression).clone();
        let term = match expression_node {
            dir::Expression::Identifier { name } => {
                self.static_reference_literal(module, expression, name)?
            }
            dir::Expression::QualifiedReference { path, .. } => {
                self.static_path_literal(module, expression, &path)?
            }
            dir::Expression::ScalarLiteral(value) => Some(dir::StaticTerm::ScalarLiteral { value }),
            dir::Expression::ObjectExpression { properties } => {
                self.static_object_literal(module, &properties)?
            }
            _ => None,
        };

        Ok(term)
    }

    /// Return one locally concrete static reference expression value.
    fn static_reference_literal(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let lookup =
            self.lookup_name_by_name(module, expression.into_any(), name, dir::SymbolSpace::Value);
        let symbol = match lookup {
            NameLookup::Found(candidate) => match candidate.symbol() {
                Some(symbol) => symbol,
                None => return Ok(None),
            },
            NameLookup::Missing => return Ok(None),
            NameLookup::Ambiguous(_) => {
                let path = dir::Path {
                    segments: smallvec::smallvec![name],
                };

                self.report_ambiguous_reference(module, expression.into_any(), &path);

                return Ok(None);
            }
        };

        self.static_symbol_literal(symbol)
    }

    /// Return one locally concrete static path expression value.
    fn static_path_literal(
        &mut self,
        module: ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
        path: &dir::Path,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let symbol =
            match self.lookup_path(module, expression.into_any(), path, dir::SymbolSpace::Value) {
                PathLookup::Found(candidate) => match candidate.symbol() {
                    Some(symbol) => symbol,
                    None => return Ok(None),
                },
                PathLookup::Missing => return Ok(None),
                PathLookup::Ambiguous(_) => {
                    self.report_ambiguous_reference(module, expression.into_any(), path);

                    return Ok(None);
                }
            };

        self.static_symbol_literal(symbol)
    }

    /// Return one locally concrete static symbol expression value.
    fn static_symbol_literal(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let Some(parameter) = self.inference.generic_parameter_by_symbol(symbol) else {
            return Ok(None);
        };
        if !self
            .inference
            .generic_parameter_binding(parameter)?
            .is_static()
        {
            return Ok(None);
        }

        Ok(Some(dir::StaticTerm::Parameter(parameter)))
    }

    /// Return one locally concrete static object expression value.
    fn static_object_literal(
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
            let Some(key) = key.static_key(&view) else {
                return Ok(None);
            };

            fields.push((key, *value));
        }

        let mut terms = Vec::with_capacity(properties.len());

        // solve statically known field values
        for (key, value) in fields {
            let Some(value) = self.static_expression_literal(module, value)? else {
                return Ok(None);
            };

            terms.push(dir::StaticProperty::Field { key, value });
        }

        Ok(Some(dir::StaticTerm::Object { properties: terms }))
    }

    /// Return a layout query term for one static reflection call.
    fn static_layout_call_term(
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
        let Some(query) = self.layout_query(symbol) else {
            return Ok(None);
        };
        let Some(target) = self.single_type_generic_argument(module, generic_arguments)? else {
            return Ok(None);
        };
        let layout = LayoutTerm {
            source,
            target,
            query,
        };
        self.push_obligation(Obligation::Concrete {
            source,
            ty: target,
            condition: Condition::Always,
        });

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
            dir::Expression::Identifier { name } => {
                match self.lookup_name_by_name(
                    module,
                    left.into_any(),
                    name,
                    dir::SymbolSpace::Value,
                ) {
                    NameLookup::Found(candidate) => candidate.symbol(),
                    NameLookup::Missing => None,
                    NameLookup::Ambiguous(_) => {
                        let path = dir::Path {
                            segments: smallvec::smallvec![name],
                        };

                        self.report_ambiguous_reference(module, left.into_any(), &path);

                        None
                    }
                }
            }
            dir::Expression::QualifiedReference { path, .. } => {
                match self.lookup_path(module, left.into_any(), &path, dir::SymbolSpace::Value) {
                    PathLookup::Found(candidate) => candidate.symbol(),
                    PathLookup::Missing => None,
                    PathLookup::Ambiguous(_) => {
                        self.report_ambiguous_reference(module, left.into_any(), &path);

                        None
                    }
                }
            }
            dir::Expression::Parenthesized { expression } => {
                return self.static_call_symbol(module, expression);
            }
            _ => None,
        };

        Ok(symbol)
    }

    /// Return the layout query named by one language symbol.
    fn layout_query(&self, symbol: dir::GlobalSymbolId) -> Option<LayoutQuery> {
        if self.is_language_symbol(symbol, dir::LanguageItem::SizeOf) {
            Some(LayoutQuery::Size)
        } else if self.is_language_symbol(symbol, dir::LanguageItem::AlignOf) {
            Some(LayoutQuery::Alignment)
        } else if self.is_language_symbol(symbol, dir::LanguageItem::StrideOf) {
            Some(LayoutQuery::Stride)
        } else {
            None
        }
    }

    /// Return the one type argument used by a static reflection call.
    fn single_type_generic_argument(
        &mut self,
        module: ModuleId,
        generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Option<TypeOperand>> {
        let [argument] = generic_arguments else {
            return Ok(None);
        };
        let argument_node = self.module(module).view().get(*argument).clone();
        let (dir::GenericArgument::Type { value }
        | dir::GenericArgument::AssociatedType { value, .. }) = argument_node
        else {
            return Ok(None);
        };
        let target = self.node_type_operand(value.into_global_any(module))?;

        Ok(Some(target))
    }
}
