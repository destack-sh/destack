use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckComponentState, Decision, GenericSubstitution, TypeLiteralTerm,
    TypeRelation, TypeTerm, VariableId,
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
        parameter: MappedParameterTerm,
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
        arguments: SmallVec<[ArgumentTerm; 4]>,
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
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
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
                let mut variables = parameter.referenced_variables();

                variables.push(*value);

                variables
            }
            Self::BestCommon { elements } => elements.iter().copied().collect(),
            Self::Widen { source } => smallvec![*source],
            Self::Exclude { source, target } => smallvec![*source, *target],
            Self::Intrinsic { item: _, arguments } => {
                arguments.iter().map(ArgumentTerm::variable).collect()
            }
        }
    }

    /// Substitute generic arguments through this type operation.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        state: &mut CheckComponentState<'_>,
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
                parameter: parameter.substitute(module, substitution, state)?,
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
                arguments: ArgumentTerm::substitute_all(arguments, module, substitution, state)?
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
        state: &mut CheckComponentState<'_>,
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

impl CheckComponentState<'_> {
    /// Widen a literal type inferred through assignability.
    pub(in crate::check) fn widen_inferred_type(term: TypeTerm) -> TypeTerm {
        let TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) = term else {
            return term;
        };
        let ty = match literal {
            dir::ScalarLiteral::Integer(_) | dir::ScalarLiteral::Float(_) => {
                TypeLiteralTerm::number()
            }
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
    pub(in crate::check) fn reduce_widen_type(
        &mut self,
        source: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.solved_type_term(source)? else {
            return Ok(None);
        };
        let term = Self::widen_inferred_type(term);

        Ok(Some(term))
    }

    /// Reduce one best common type term.
    pub(in crate::check) fn reduce_best_common_type(
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

        let term = self.reduce_best_common_terms(module, candidates)?;

        // push the chosen candidate back into element expressions
        for element in elements {
            let Some(element_term) = self.solved_type_term(*element)? else {
                return Ok(None);
            };
            self.expect_literal_type(*element, &element_term, &term)?;
        }

        Ok(Some(term))
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(in crate::check) fn reduce_best_common_terms(
        &mut self,
        module: ModuleId,
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
            variables.push(self.push_solved_type_variable(module, candidate)?);
        }

        Ok(TypeTerm::Union {
            elements: variables,
        })
    }

    /// Reduce one type exclusion term.
    pub(in crate::check) fn reduce_exclude_type(
        &mut self,
        module: ModuleId,
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
            ) if source_form.same_constructor(target_form) => {
                let Some(payload) =
                    self.reduce_exclude_type(module, *source_value, *target_value)?
                else {
                    return Ok(None);
                };
                let payload = self.push_solved_type_variable(module, payload)?;

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
}
