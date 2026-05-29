use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericArgument, GenericSubstitution, Origin, Progress, Reduction,
    ShapeMember, Solution, TupleElement, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId, VariableOutput, shape_field,
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
    /// TemplateTerm literal type expression.
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
        parameter: MappedParameter,
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
        elements: Vec<TypeOperand>,
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
        source: TypeOperand,
        /// The excluded type.
        target: TypeOperand,
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
        arguments: SmallVec<[GenericArgument; 4]>,
    },
}

/// Mapped type parameter term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MappedParameter {
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
                let mut variables = parameter.referenced_variables();

                variables.push(*value);

                variables
            }
            Self::BestCommon { elements } => elements
                .iter()
                .flat_map(|element| element.referenced_variables(state))
                .collect(),
            Self::Widen { source } => {
                let mut variables = smallvec![*source];

                // watch nested operands once the source has a structural solution
                if let Some(Solution::Type(term)) = state.solutions.variable.get(source) {
                    variables.extend(state.terms.get(*term).referenced_variables(state));
                }

                variables
            }
            Self::Exclude { source, target } => {
                let mut variables = source.referenced_variables(state);

                variables.extend(target.referenced_variables(state));

                variables
            }
            Self::Intrinsic { item: _, arguments } => arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument))
                .collect(),
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
                parameter: parameter.substitute(module, substitution, state)?,
                modifiers: *modifiers,
                value: state.substitute_type_variable(module, substitution, *value)?,
            },
            Self::BestCommon { elements } => Self::BestCommon {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            Self::Widen { source } => Self::Widen {
                source: state.substitute_type_variable(module, substitution, *source)?,
            },
            Self::Exclude { source, target } => Self::Exclude {
                source: state.substitute_type_operand(module, substitution, *source)?,
                target: state.substitute_type_operand(module, substitution, *target)?,
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

impl MappedParameter {
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
        origin: Origin,
        left: VariableId,
        right: VariableId,
        then_type: VariableId,
        else_type: VariableId,
        expected: TypeOperand,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let progress = match decision {
            // constrain the selected branch
            Decision::Yes => {
                self.expect_conditional_branch(origin, then_type, expected, is_exact)?
            }
            // constrain the selected branch
            Decision::No => {
                self.expect_conditional_branch(origin, else_type, expected, is_exact)?
            }
            // constrain every possible branch
            Decision::Undecidable => {
                let then_type =
                    self.expect_conditional_branch(origin, then_type, expected, is_exact)?;
                let else_type =
                    self.expect_conditional_branch(origin, else_type, expected, is_exact)?;

                then_type.merge(else_type)
            }
        };

        Ok(progress)
    }

    /// Reduce one indexed access type with a literal key.
    pub(in crate::check) fn reduce_type_index_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: VariableId,
        index: VariableId,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(left) = self.solved_type_term(left)? else {
            return Ok(None);
        };
        let left = match self.reduce_type_term(origin, &left)? {
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

        self.resolve_member_type(origin, module, &left, &key, &[])
    }

    /// Expect one indexed access type to satisfy one expected type.
    pub(in crate::check) fn expect_type_index_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: VariableId,
        index: VariableId,
        expected: TypeOperand,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.reduce_type_index_term(origin, module, left, index)? else {
            return Ok(Progress::Unchanged);
        };
        let term = self.terms.push(term);
        let progress = if is_exact {
            self.solve_type_equality(origin, term, expected)?
        } else {
            self.solve_contextual_type_assignability(origin, term, expected)?
        };

        Ok(progress)
    }

    /// Widen one inferred type.
    pub(in crate::check) fn widen_inferred_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match term {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) => {
                TypeTerm::Literal(Self::widen_scalar_literal(literal))
            }
            TypeTerm::Array { element } => TypeTerm::Array {
                element: match self.widen_inferred_operand(origin, module, element)? {
                    Some(element) => element,
                    None => return Ok(None),
                },
            },
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly,
            } => TypeTerm::FixedArray {
                element: match self.widen_inferred_operand(origin, module, element)? {
                    Some(element) => element,
                    None => return Ok(None),
                },
                length,
                is_readonly,
            },
            TypeTerm::Slice {
                element,
                is_readonly,
            } => TypeTerm::Slice {
                element: match self.widen_inferred_operand(origin, module, element)? {
                    Some(element) => element,
                    None => return Ok(None),
                },
                is_readonly,
            },
            TypeTerm::Tuple {
                form,
                elements,
                is_readonly,
            } => {
                let mut widened = SmallVec::with_capacity(elements.len());

                // widen elements in source order
                for element in elements {
                    let Some(ty) = self.widen_inferred_operand(origin, module, element.ty)? else {
                        return Ok(None);
                    };

                    widened.push(TupleElement { ty, ..element });
                }

                TypeTerm::Tuple {
                    form,
                    elements: widened,
                    is_readonly,
                }
            }
            TypeTerm::Shape { members } => {
                let mut widened = SmallVec::with_capacity(members.len());

                // widen members in source order
                for member in members {
                    let Some(member) = self.widen_inferred_shape_member(origin, module, member)?
                    else {
                        return Ok(None);
                    };

                    widened.push(member);
                }

                TypeTerm::Shape { members: widened }
            }
            TypeTerm::Union { elements } => {
                let mut widened = Vec::with_capacity(elements.len());

                // widen elements in source order
                for element in elements {
                    let Some(element) = self.widen_inferred_operand(origin, module, element)?
                    else {
                        return Ok(None);
                    };
                    let Some(term) = self.type_operand_term(element)? else {
                        return Ok(None);
                    };

                    widened.push(term);
                }

                self.reduce_best_common_terms(module, widened)?
            }
            TypeTerm::Intersection { elements } => {
                let mut widened = Vec::with_capacity(elements.len());

                // widen elements in source order
                for element in elements {
                    let Some(element) = self.widen_inferred_operand(origin, module, element)?
                    else {
                        return Ok(None);
                    };

                    widened.push(element);
                }

                TypeTerm::Intersection { elements: widened }
            }
            _ => term,
        };

        Ok(Some(term))
    }

    /// Reduce one inferred mutable storage type.
    pub(in crate::check) fn reduce_widen_term(
        &mut self,
        origin: Origin,
        source: VariableId,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let Some(source_term) = self.solved_type_term(source)? else {
            return Ok(Reduction::pending());
        };
        let Some(term) = self.widen_inferred_type(origin, source.module, source_term.clone())?
        else {
            return Ok(Reduction::pending());
        };

        let progress = self.expect_widened_source(source, &source_term, &term)?;

        Ok(Reduction::with_progress(term, progress))
    }

    /// Widen one scalar literal.
    fn widen_scalar_literal(literal: dir::ScalarLiteral) -> TypeLiteralTerm {
        match literal {
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
        }
    }

    /// Widen one inferred operand.
    fn widen_inferred_operand(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operand: TypeOperand,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };
        let reduction = self.reduce_type_term(origin, &term)?;
        let term = reduction.value.unwrap_or(term);
        let Some(widened) = self.widen_inferred_type(origin, module, term.clone())? else {
            return Ok(None);
        };
        if widened == term {
            return Ok(Some(operand));
        }

        let widened = self.terms.push(widened);

        Ok(Some(widened.into()))
    }

    /// Widen one inferred shape member.
    fn widen_inferred_shape_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        member: ShapeMember,
    ) -> CompilerResult<Option<ShapeMember>> {
        let member = match member {
            ShapeMember::Field {
                key,
                ty,
                is_optional,
                is_readonly,
            } => ShapeMember::Field {
                key,
                ty: match self.widen_inferred_operand(origin, module, ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
                is_optional,
                is_readonly,
            },
            ShapeMember::CallSignature { ty } => ShapeMember::CallSignature {
                ty: match self.widen_inferred_operand(origin, module, ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
            },
            ShapeMember::ConstructSignature { ty } => ShapeMember::ConstructSignature {
                ty: match self.widen_inferred_operand(origin, module, ty)? {
                    Some(ty) => ty,
                    None => return Ok(None),
                },
            },
            ShapeMember::IndexSignature {
                name,
                key_type,
                value_type,
                is_optional,
                is_readonly,
            } => ShapeMember::IndexSignature {
                name,
                key_type: match self.widen_inferred_operand(origin, module, key_type)? {
                    Some(key_type) => key_type,
                    None => return Ok(None),
                },
                value_type: match self.widen_inferred_operand(origin, module, value_type)? {
                    Some(value_type) => value_type,
                    None => return Ok(None),
                },
                is_optional,
                is_readonly,
            },
        };

        Ok(Some(member))
    }

    /// Push a widened initializer type back into its expression tree.
    fn expect_widened_source(
        &mut self,
        source: VariableId,
        source_term: &TypeTerm,
        widened_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let nested = self.expect_widened_term_literals(source_term, widened_term)?;
        let check_variable = self.variable(source);
        if !matches!(check_variable.output, Some(VariableOutput::Node(_))) {
            return Ok(nested);
        }
        if self.solved_type_term(source)?.as_ref() == Some(widened_term) {
            return Ok(nested);
        }
        let widened = self.terms.push(widened_term.clone());
        let relation =
            self.solve_type_assignability(self.variable(source).source, source, widened)?;

        Ok(nested.merge(relation))
    }

    /// Push widened aggregate element types into scalar literal expression nodes.
    fn expect_widened_term_literals(
        &mut self,
        source: &TypeTerm,
        widened: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let progress = match (source, widened) {
            (TypeTerm::Array { element: source }, TypeTerm::Array { element: widened })
            | (
                TypeTerm::Slice {
                    element: source,
                    is_readonly: _,
                },
                TypeTerm::Slice {
                    element: widened,
                    is_readonly: _,
                },
            )
            | (
                TypeTerm::FixedArray {
                    element: source,
                    length: _,
                    is_readonly: _,
                },
                TypeTerm::FixedArray {
                    element: widened,
                    length: _,
                    is_readonly: _,
                },
            ) => self.expect_widened_operand_literals(*source, *widened)?,
            (
                TypeTerm::Tuple {
                    form: _,
                    elements: source,
                    is_readonly: _,
                },
                TypeTerm::Tuple {
                    form: _,
                    elements: widened,
                    is_readonly: _,
                },
            ) => self.expect_widened_tuple_literals(source, widened)?,
            (TypeTerm::Shape { members: source }, TypeTerm::Shape { members: widened }) => {
                self.expect_widened_shape_literals(source, widened)?
            }
            (TypeTerm::Operation(operation), widened) => match self.terms.get(*operation).clone() {
                TypeOperationTerm::BestCommon { elements } => {
                    self.expect_widened_best_common_literals(&elements, widened)?
                }
                _ => Progress::Unchanged,
            },
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Push one widened operand type into scalar literal expression nodes.
    fn expect_widened_operand_literals(
        &mut self,
        source: TypeOperand,
        widened: TypeOperand,
    ) -> CompilerResult<Progress> {
        let Some(widened_term) = self.type_operand_term(widened)? else {
            return Ok(Progress::Unchanged);
        };
        let Some(source_term) = self.type_operand_term(source)? else {
            return Ok(Progress::Unchanged);
        };
        let literal = match source {
            TypeOperand::Variable(variable) => {
                self.expect_literal_term(variable, &source_term, &widened_term)?
            }
            TypeOperand::Term(_) => Progress::Unchanged,
        };
        let nested = self.expect_widened_term_literals(&source_term, &widened_term)?;

        Ok(literal.merge(nested))
    }

    /// Push widened tuple element types into scalar literal expression nodes.
    fn expect_widened_tuple_literals(
        &mut self,
        source: &[TupleElement],
        widened: &[TupleElement],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // match tuple elements positionally
        for (source, widened) in source.iter().zip(widened) {
            progress = progress.merge(self.expect_widened_operand_literals(source.ty, widened.ty)?);
        }

        Ok(progress)
    }

    /// Push widened shape member types into scalar literal expression nodes.
    fn expect_widened_shape_literals(
        &mut self,
        source: &[ShapeMember],
        widened: &[ShapeMember],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // match fields by key
        for member in source {
            let Some((key, source)) = shape_field(member) else {
                continue;
            };
            let Some(widened) = self.shape_field_type(widened, key) else {
                continue;
            };

            progress = progress.merge(self.expect_widened_operand_literals(source, widened)?);
        }

        Ok(progress)
    }

    /// Push a best common type into scalar literal expression nodes.
    fn expect_widened_best_common_literals(
        &mut self,
        elements: &[TypeOperand],
        widened: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push the chosen widened type into assignable scalar literals
        for element in elements {
            let Some(element_term) = self.type_operand_term(*element)? else {
                continue;
            };
            let TypeOperand::Variable(element) = *element else {
                continue;
            };

            progress = progress.merge(self.expect_literal_term(element, &element_term, widened)?);
        }

        Ok(progress)
    }

    /// Expect a widened source term to satisfy one expected type.
    pub(in crate::check) fn expect_widen_term(
        &mut self,
        origin: Origin,
        source: VariableId,
        expected: TypeOperand,
        expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let literal = if let Some(source_term) = self.solved_type_term(source)?
            && let TypeOperand::Variable(_) = expected
        {
            self.expect_literal_term(source, &source_term, expected_term)?
        } else {
            Progress::Unchanged
        };
        let assignable = self.solve_contextual_type_assignability(origin, source, expected)?;

        Ok(literal.merge(assignable))
    }

    /// Reduce one best common type term.
    pub(in crate::check) fn reduce_best_common_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        elements: &[TypeOperand],
    ) -> CompilerResult<Option<TypeTerm>> {
        if elements.is_empty() {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Unknown)));
        }
        let mut candidates = Vec::with_capacity(elements.len());

        // collect widened element candidates
        for element in elements {
            let Some(term) = self.type_operand_term(*element)? else {
                return Ok(None);
            };
            let Some(term) = self.widen_inferred_type(origin, module, term)? else {
                return Ok(None);
            };

            candidates.push(term);
        }

        let term = self.reduce_best_common_terms(module, candidates)?;

        Ok(Some(term))
    }

    /// Expect best common type elements to satisfy one expected type.
    pub(in crate::check) fn expect_best_common_term(
        &mut self,
        origin: Origin,
        result: Option<VariableId>,
        elements: &[TypeOperand],
        expected: TypeOperand,
        expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push the expected element type into every literal element
        for element in elements {
            if let Some(element_term) = self.type_operand_term(*element)?
                && let TypeOperand::Variable(element) = *element
            {
                progress = progress.merge(self.expect_literal_term(
                    element,
                    &element_term,
                    expected_term,
                )?);
            }

            progress = progress
                .merge(self.solve_contextual_type_assignability(origin, *element, expected)?);
        }

        if let Some(result) = result {
            progress = progress.merge(self.solve_type_variable(result, expected_term.clone())?);
        }

        Ok(progress)
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(in crate::check) fn reduce_best_common_terms(
        &mut self,
        _module: ModuleId,
        candidates: Vec<TypeTerm>,
    ) -> CompilerResult<TypeTerm> {
        // choose the first candidate that accepts every element
        for candidate in &candidates {
            if self.is_best_common_candidate(candidate, &candidates)? {
                return Ok(candidate.clone());
            }
        }
        let mut elements = Vec::with_capacity(candidates.len());

        // preserve heterogeneous literal arrays as widened unions
        for candidate in candidates {
            let candidate = self.terms.push(candidate);

            elements.push(candidate.into());
        }

        Ok(TypeTerm::Union { elements })
    }

    /// Reduce one type exclusion term.
    pub(in crate::check) fn reduce_exclude_term(
        &mut self,
        module: ModuleId,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(source_term) = self.type_operand_term(source)? else {
            return Ok(None);
        };
        let Some(target_term) = self.type_operand_term(target)? else {
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
                    self.reduce_exclude_term(module, *source_value, *target_value)?
                else {
                    return Ok(None);
                };
                let payload = self.terms.push(payload);

                TypeTerm::Form {
                    form: source_form.clone(),
                    payload: payload.into(),
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
        elements: &[TypeOperand],
        target: &TypeTerm,
    ) -> CompilerResult<TypeTerm> {
        let mut kept = Vec::with_capacity(elements.len());

        // remove elements that are known equal to the excluded type
        for element in elements {
            let Some(element_term) = self.type_operand_term(*element)? else {
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
            match kept[0] {
                TypeOperand::Variable(variable) => TypeTerm::Variable(variable),
                TypeOperand::Term(term) => self.terms.get(term).clone(),
            }
        } else {
            TypeTerm::Union { elements: kept }
        };

        Ok(term)
    }

    /// Expect one conditional branch to satisfy the expected result.
    fn expect_conditional_branch(
        &mut self,
        origin: Origin,
        branch: VariableId,
        expected: TypeOperand,
        is_exact: bool,
    ) -> CompilerResult<Progress> {
        if is_exact {
            self.solve_type_equality(origin, branch, expected)
        } else {
            self.solve_contextual_type_assignability(origin, branch, expected)
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
