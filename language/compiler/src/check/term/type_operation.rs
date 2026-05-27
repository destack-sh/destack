use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckState, ConstraintOrigin, Decision, GenericSubstitution, Progress, Reduction,
    Solution, TermId, TypeLiteralTerm, TypeRelation, TypeTerm, VariableId,
};

/// Type-level operation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeOperationTerm {
    /// Conditional type expression.
    ///
    /// ```ts
    /// T extends string ? A : B
    /// ```
    Conditional {
        /// The left operand.
        left: VariableId,
        /// The right operand.
        right: VariableId,
        /// The type selected when the condition holds.
        then_type: VariableId,
        /// The type selected when the condition does not hold.
        else_type: VariableId,
    },
    /// Indexed access type expression.
    ///
    /// ```ts
    /// T[K]
    /// ```
    Index {
        /// The indexed type.
        left: VariableId,
        /// The index type.
        index: VariableId,
    },
    /// Template literal type expression.
    ///
    /// ```ts
    /// `id:${T}`
    /// ```
    TemplateLiteral {
        /// The literal string segments.
        strings: Vec<dir::StringId>,
        /// The interpolated type spans.
        spans: Vec<VariableId>,
    },
    /// Type infer binding in a conditional type pattern.
    ///
    /// ```ts
    /// T extends Array<infer U> ? U : never
    /// ```
    Infer {
        /// The inferred binding name.
        name: Option<dir::StringId>,
        /// The optional inferred constraint.
        constraint: Option<VariableId>,
    },
    /// `keyof T`.
    KeyOf {
        /// The target type.
        target: VariableId,
    },
    /// Mapped type expression.
    ///
    /// ```ts
    /// { [K in keyof T]: T[K] }
    /// ```
    Mapped {
        /// The mapped parameter.
        parameter: TermId<MappedParameterTerm>,
        /// The mapped modifiers.
        modifiers: dir::MappedTypeModifiers,
        /// The mapped value type.
        value: VariableId,
    },
    /// Best common type selected for expression literals.
    ///
    /// ```ts
    /// [left, right]
    /// ```
    BestCommon {
        /// The candidate element types.
        elements: Vec<VariableId>,
    },
    /// Literal widening for inferred mutable storage.
    ///
    /// ```ts
    /// let value = 1
    /// ```
    Widen {
        /// The inferred source type.
        source: VariableId,
    },
    /// Type exclusion expression.
    ///
    /// ```ts
    /// Exclude<T, null>
    /// ```
    Exclude {
        /// The source type.
        source: VariableId,
        /// The excluded type.
        target: VariableId,
    },
    /// Compiler intrinsic returning a type.
    ///
    /// ```ts
    /// WithLifetime<T, L>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[TermId<ArgumentTerm>; 4]>,
    },
}

/// Mapped type parameter term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MappedParameterTerm {
    /// The parameter name.
    pub(in crate::check) name: dir::StringId,
    /// The parameter symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source type iterated by `in`.
    pub(in crate::check) constraint: VariableId,
    /// The optional key remap.
    pub(in crate::check) key_remap: Option<VariableId>,
}

impl TypeOperationTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => smallvec![*left, *right, *then_type, *else_type],
            Self::Index { left, index } => smallvec![*left, *index],
            Self::TemplateLiteral { strings: _, spans } => spans.iter().copied().collect(),
            Self::Infer {
                name: _,
                constraint,
            } => constraint.iter().copied().collect(),
            Self::KeyOf { target } => smallvec![*target],
            Self::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                let mut variables = state.terms.get(*parameter).referenced_variables();

                variables.push(*value);

                variables
            }
            Self::BestCommon { elements } => elements.iter().copied().collect(),
            Self::Widen { source } => smallvec![*source],
            Self::Exclude { source, target } => smallvec![*source, *target],
            Self::Intrinsic { item: _, arguments } => {
                arguments
                    .iter()
                    .flat_map(|argument| state.argument_variables(*argument))
                    .collect()
            }
        }
    }

    /// Substitute generic arguments through this type operation.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let operation = match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => Self::Conditional {
                left: state.substitute_type_variable(module, substitution, *left)?,
                right: state.substitute_type_variable(module, substitution, *right)?,
                then_type: state.substitute_type_variable(module, substitution, *then_type)?,
                else_type: state.substitute_type_variable(module, substitution, *else_type)?,
            },
            Self::Index { left, index } => Self::Index {
                left: state.substitute_type_variable(module, substitution, *left)?,
                index: state.substitute_type_variable(module, substitution, *index)?,
            },
            Self::TemplateLiteral { strings, spans } => Self::TemplateLiteral {
                strings: strings.clone(),
                spans: state.substitute_type_variables(module, substitution, spans)?,
            },
            Self::Infer { name, constraint } => Self::Infer {
                name: *name,
                constraint: constraint
                    .map(|constraint| {
                        state.substitute_type_variable(module, substitution, constraint)
                    })
                    .transpose()?,
            },
            Self::KeyOf { target } => Self::KeyOf {
                target: state.substitute_type_variable(module, substitution, *target)?,
            },
            Self::Mapped {
                parameter,
                modifiers,
                value,
            } => Self::Mapped {
                parameter: {
                    let parameter = state.terms.get(*parameter).substitute(module, substitution, state)?;

                    state.terms.push(parameter)
                },
                modifiers: *modifiers,
                value: state.substitute_type_variable(module, substitution, *value)?,
            },
            Self::BestCommon { elements } => Self::BestCommon {
                elements: state.substitute_type_variables(module, substitution, elements)?,
            },
            Self::Widen { source } => Self::Widen {
                source: state.substitute_type_variable(module, substitution, *source)?,
            },
            Self::Exclude { source, target } => Self::Exclude {
                source: state.substitute_type_variable(module, substitution, *source)?,
                target: state.substitute_type_variable(module, substitution, *target)?,
            },
            Self::Intrinsic { item, arguments } => Self::Intrinsic {
                item: *item,
                arguments: state
                    .substitute_arguments(module, substitution, arguments)?
                    .into(),
            },
        };

        Ok(operation)
    }
}

impl MappedParameterTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        variables.push(self.constraint);
        variables.extend(self.key_remap);

        variables
    }

    /// Substitute generic arguments through this mapped parameter.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            name: self.name,
            symbol: self.symbol,
            constraint: state.substitute_type_variable(module, substitution, self.constraint)?,
            key_remap: self
                .key_remap
                .map(|key_remap| state.substitute_type_variable(module, substitution, key_remap))
                .transpose()?,
        })
    }
}

impl CheckState<'_> {
    /// Reduce one conditional type expression.
    pub(in crate::check) fn reduce_conditional_term(
        &self,
        left: VariableId,
        right: VariableId,
        then_type: VariableId,
        else_type: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let selected = match decision {
            Decision::Yes => then_type,
            Decision::No => else_type,
            Decision::Undecidable => return Ok(None),
        };

        Ok(Some(TypeTerm::Variable(selected)))
    }

    /// Expect selected conditional branches to satisfy one expected type.
    pub(in crate::check) fn expect_conditional_term(
        &mut self,
        left: VariableId,
        right: VariableId,
        then_type: VariableId,
        else_type: VariableId,
        expected: VariableId,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let progress = match decision {
            // constrain the selected branch
            Decision::Yes => self.expect_conditional_branch(then_type, expected, is_exact)?,
            // constrain the selected branch
            Decision::No => self.expect_conditional_branch(else_type, expected, is_exact)?,
            // constrain every possible branch
            Decision::Undecidable => {
                let then_type = self.expect_conditional_branch(then_type, expected, is_exact)?;
                let else_type = self.expect_conditional_branch(else_type, expected, is_exact)?;

                then_type.merge(else_type)
            }
        };

        Ok(progress)
    }

    /// Reduce one indexed access type with a literal key.
    pub(in crate::check) fn reduce_type_index_term(
        &mut self,
        module: ModuleId,
        left: VariableId,
        index: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(left) = self.solved_type_term(left)? else {
            return Ok(None);
        };
        let left = match self.reduce_type_term(module, &left)? {
            Reduction {
                value: Some(term), ..
            } => term,
            Reduction { value: None, .. } => left,
        };
        let Some(index) = self.solved_type_term(index)? else {
            return Ok(None);
        };
        let Some(key) = Self::type_term_static_key(&index) else {
            return Ok(None);
        };

        self.resolve_member_type(module, &left, &key, &[])
    }

    /// Expect one indexed access type to satisfy one expected type.
    pub(in crate::check) fn expect_type_index_term(
        &mut self,
        left: VariableId,
        index: VariableId,
        expected: VariableId,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.reduce_type_index_term(expected.module, left, index)? else {
            return Ok(Progress::Unchanged);
        };
        let origin = self.variable_origin(left)?;
        let term = self.solve_anonymous_type(expected.module, origin, term)?;
        let progress = if is_exact {
            self.solve_type_equality(term, expected)?
        } else {
            self.solve_type_assignability(term, expected)?
        };

        Ok(progress)
    }

    /// Widen a literal type inferred through assignability.
    pub(in crate::check) fn widen_inferred_type(term: TypeTerm) -> TypeTerm {
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) = term else {
            return term;
        };
        let ty = match literal {
            dir::ScalarLiteral::Integer(_) => {
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
                    width: 32,
                    is_signed: true,
                }))
            }
            dir::ScalarLiteral::Float(_) => TypeLiteralTerm::number(),
            dir::ScalarLiteral::Bigint(_) => TypeLiteralTerm::bigint(),
            dir::ScalarLiteral::String(_) => TypeLiteralTerm::Primitive(dir::PrimitiveType::String),
            dir::ScalarLiteral::Null => TypeLiteralTerm::Null,
            dir::ScalarLiteral::Boolean(_) => TypeLiteralTerm::boolean(),
            dir::ScalarLiteral::Character(_) => {
                TypeLiteralTerm::Primitive(dir::PrimitiveType::Character)
            }
            dir::ScalarLiteral::RegexString { .. } => TypeLiteralTerm::Object,
        };

        TypeTerm::Literal(ty)
    }

    /// Reduce one inferred mutable storage type.
    pub(in crate::check) fn reduce_widen_term(
        &mut self,
        source: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.solved_type_term(source)? else {
            return Ok(None);
        };
        let term = Self::widen_inferred_type(term);

        Ok(Some(term))
    }

    /// Expect a widened source term to satisfy one expected type.
    pub(in crate::check) fn expect_widen_term(
        &mut self,
        source: VariableId,
        expected: VariableId,
        expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let literal = if let Some(source_term) = self.solved_type_term(source)? {
            self.expect_literal_term(source, &source_term, expected_term)?
        } else {
            Progress::Unchanged
        };
        let assignable = self.solve_type_assignability(source, expected)?;

        Ok(literal.merge(assignable))
    }

    /// Reduce one best common type term.
    pub(in crate::check) fn reduce_best_common_term(
        &mut self,
        module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<TypeTerm>> {
        if elements.is_empty() {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Unknown)));
        }
        let mut candidates = Vec::with_capacity(elements.len());

        // collect widened element candidates
        for element in elements {
            let Some(term) = self.solved_type_term(*element)? else {
                return Ok(None);
            };
            let term = Self::widen_inferred_type(term);

            candidates.push(term);
        }

        let origin = self.variable_origin(elements[0])?;
        let term = self.reduce_best_common_terms(module, origin, candidates)?;

        // push the chosen candidate back into element expressions
        for element in elements {
            let Some(element_term) = self.solved_type_term(*element)? else {
                return Ok(None);
            };
            self.expect_literal_term(*element, &element_term, &term)?;
        }

        Ok(Some(term))
    }

    /// Expect best common type elements to satisfy one expected type.
    pub(in crate::check) fn expect_best_common_term(
        &mut self,
        result: VariableId,
        elements: &[VariableId],
        expected: VariableId,
        expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push the expected element type into every literal element
        for element in elements {
            if let Some(element_term) = self.solved_type_term(*element)? {
                progress = progress.merge(self.expect_literal_term(
                    *element,
                    &element_term,
                    expected_term,
                )?);
            }

            progress = progress.merge(self.solve_type_assignability(*element, expected)?);
        }

        let expected_term = self.terms.push(expected_term.clone());
        let changed = self.solve_variable(result, Solution::Type(expected_term));

        Ok(progress.merge(Progress::from_change(result, changed)))
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(in crate::check) fn reduce_best_common_terms(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        candidates: Vec<TypeTerm>,
    ) -> CompilerResult<TypeTerm> {
        // choose the first candidate that accepts every element
        for candidate in &candidates {
            if self.is_best_common_candidate(candidate, &candidates)? {
                return Ok(candidate.clone());
            }
        }
        let mut variables = Vec::with_capacity(candidates.len());

        // preserve heterogeneous literal arrays as widened unions
        for candidate in candidates {
            variables.push(self.solve_anonymous_type(module, origin, candidate)?);
        }

        Ok(TypeTerm::Union {
            elements: variables,
        })
    }

    /// Reduce one type exclusion term.
    pub(in crate::check) fn reduce_exclude_term(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        source: VariableId,
        target: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(source_term) = self.solved_type_term(source)? else {
            return Ok(None);
        };
        let Some(target_term) = self.solved_type_term(target)? else {
            return Ok(None);
        };
        let term = match (&source_term, &target_term) {
            (
                TypeTerm::Form {
                    form: source_form,
                    payload: source_value,
                },
                TypeTerm::Form {
                    form: target_form,
                    payload: target_value,
                },
            ) if self
                .terms
                .get(*source_form)
                .same_constructor(self.terms.get(*target_form)) =>
            {
                let Some(payload) =
                    self.reduce_exclude_term(module, origin, *source_value, *target_value)?
                else {
                    return Ok(None);
                };
                let payload = self.solve_anonymous_type(module, origin, payload)?;

                TypeTerm::Form {
                    form: source_form.clone(),
                    payload,
                }
            }
            (TypeTerm::Union { elements }, target) => {
                self.reduce_exclude_union(elements, target)?
            }
            (source, target)
                if self.decide_type_term_relation(TypeRelation::Equal, source, target)?
                    == Decision::Yes =>
            {
                TypeTerm::Literal(TypeLiteralTerm::Never)
            }
            _ => source_term,
        };

        Ok(Some(term))
    }

    /// Return whether one candidate accepts every best-common element.
    fn is_best_common_candidate(
        &self,
        candidate: &TypeTerm,
        elements: &[TypeTerm],
    ) -> CompilerResult<bool> {
        for element in elements {
            let decision =
                self.decide_type_term_relation(TypeRelation::Assignable, element, candidate)?;
            if decision != Decision::Yes {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Reduce one union type exclusion.
    fn reduce_exclude_union(
        &self,
        elements: &[VariableId],
        target: &TypeTerm,
    ) -> CompilerResult<TypeTerm> {
        let mut kept = Vec::with_capacity(elements.len());

        // remove elements that are known equal to the excluded type
        for element in elements {
            let Some(element_term) = self.solved_type_term(*element)? else {
                return Ok(TypeTerm::Union {
                    elements: elements.to_vec(),
                });
            };
            if self.decide_type_term_relation(TypeRelation::Equal, &element_term, target)?
                == Decision::Yes
            {
                continue;
            }

            kept.push(*element);
        }

        let term = if kept.len() == 1 {
            TypeTerm::Variable(kept[0])
        } else {
            TypeTerm::Union { elements: kept }
        };

        Ok(term)
    }

    /// Expect one conditional branch to satisfy the expected result.
    fn expect_conditional_branch(
        &mut self,
        branch: VariableId,
        expected: VariableId,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        if is_exact {
            self.solve_type_equality(branch, expected)
        } else {
            self.solve_type_assignability(branch, expected)
        }
    }

    /// Return the structural key represented by one literal type.
    fn type_term_static_key(term: &TypeTerm) -> Option<dir::StaticKey> {
        match term {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::String(name))) => {
                Some(dir::StaticKey::Name(*name))
            }
            _ => None,
        }
    }
}
