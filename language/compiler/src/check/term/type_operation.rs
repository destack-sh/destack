use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, GenericArgument, Origin, ShapeMember, SubstitutionSet, TermId,
    TupleElement, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
};
use crate::{CompilerError, CompilerResult};

/// Type-level operation term.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum TypeOperationTerm {
    /// String mapping type operator application.
    ///
    /// ```ds
    /// Uppercase<T>
    /// ```
    StringMapping {
        /// The string mapping.
        mapping: dir::StringMapping,
        /// The string type argument.
        argument: TypeOperand,
    },
    /// Conditional type expression.
    ///
    /// ```ds
    /// T extends string ? A : B
    /// ```
    Conditional {
        /// The left operand.
        left: TypeOperand,
        /// The right operand.
        right: TypeOperand,
        /// The type selected when the condition holds.
        then_type: TypeOperand,
        /// The type selected when the condition does not hold.
        else_type: TypeOperand,
    },
    /// Mapped type expression.
    ///
    /// ```ds
    /// { [K in keyof T]: T[K] }
    /// ```
    Mapped {
        /// The mapped parameter.
        parameter: MappedParameter,
        /// The mapped modifiers.
        modifiers: dir::MappedTypeModifiers,
        /// The mapped value type.
        value: TypeOperand,
    },
    /// Indexed access type expression.
    ///
    /// ```ds
    /// T[K]
    /// ```
    Index {
        /// The indexed type.
        left: TypeOperand,
        /// The index type.
        index: TypeOperand,
    },
    /// Template literal type expression.
    ///
    /// ```ds
    /// `id:${T}`
    /// ```
    TemplateLiteral {
        /// The literal string segments.
        strings: Vec<dir::StringId>,
        /// The interpolated type spans.
        spans: Vec<TypeOperand>,
    },
    /// Type infer binding in a conditional type pattern.
    ///
    /// ```ds
    /// T extends Array<infer U> ? U : never
    /// ```
    Infer {
        /// The inferred binding name.
        name: Option<dir::StringId>,
        /// The optional inferred constraint.
        constraint: Option<TypeOperand>,
    },
    /// `keyof T`.
    KeyOf {
        /// The target type.
        target: TypeOperand,
    },
    /// Best common type selected for expression literals.
    ///
    /// ```ds
    /// [left, right]
    /// ```
    BestCommon {
        /// The candidate element types.
        elements: Vec<TypeOperand>,
    },
    /// Literal widening for inferred mutable storage.
    ///
    /// ```ds
    /// let value = 1
    /// ```
    Widen {
        /// The inferred source type.
        source: TypeOperand,
    },
    /// Type exclusion expression.
    ///
    /// ```ds
    /// Exclude<T, null>
    /// ```
    Exclude {
        /// The source type.
        source: TypeOperand,
        /// The excluded type.
        target: TypeOperand,
    },
    /// Type extraction expression.
    ///
    /// ```ds
    /// Extract<T, U>
    /// ```
    Extract {
        /// The source type.
        source: TypeOperand,
        /// The extracted type.
        target: TypeOperand,
    },
    /// Compiler intrinsic returning a type.
    ///
    /// ```ds
    /// WithLifetime<T, L>
    /// ```
    Intrinsic {
        /// The intrinsic language item.
        item: dir::LanguageItem,
        /// The intrinsic arguments.
        arguments: SmallVec<[GenericArgument; 2]>,
    },
}

/// Mapped type parameter term.
///
/// Examples:
/// ```ds
/// { [K in keyof T]: T[K] }
/// { [K in keyof T as Rename<K>]: T[K] }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MappedParameter {
    /// The parameter name.
    pub(in crate::check) name: dir::StringId,
    /// The parameter symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The source type iterated by `in`.
    pub(in crate::check) constraint: TypeOperand,
    /// The optional key remap.
    pub(in crate::check) key_remap: Option<TypeOperand>,
}

impl TypeOperationTerm {
    /// Return whether this operation must reduce before it can be committed.
    pub(in crate::check) fn must_reduce_for_commit(&self) -> bool {
        matches!(
            self,
            Self::BestCommon { .. }
                | Self::Widen { .. }
                | Self::Exclude { .. }
                | Self::Extract { .. }
                | Self::Intrinsic { .. }
        )
    }

    /// Return whether this operation must reduce before member resolution.
    pub(in crate::check) fn must_reduce_for_member_resolution(&self) -> bool {
        matches!(
            self,
            Self::StringMapping { .. }
                | Self::Index { .. }
                | Self::BestCommon { .. }
                | Self::Widen { .. }
                | Self::Exclude { .. }
                | Self::Extract { .. }
                | Self::Intrinsic { .. }
        )
    }

    /// Substitute generic arguments through this type operation.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        let operation = match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => Self::Conditional {
                left: state.substitute_type_operand(module, substitution, *left)?,
                right: state.substitute_type_operand(module, substitution, *right)?,
                then_type: state.substitute_type_operand(module, substitution, *then_type)?,
                else_type: state.substitute_type_operand(module, substitution, *else_type)?,
            },
            Self::Index { left, index } => Self::Index {
                left: state.substitute_type_operand(module, substitution, *left)?,
                index: state.substitute_type_operand(module, substitution, *index)?,
            },
            Self::TemplateLiteral { strings, spans } => Self::TemplateLiteral {
                strings: strings.clone(),
                spans: state.substitute_type_operands(module, substitution, spans)?,
            },
            Self::Infer { name, constraint } => Self::Infer {
                name: *name,
                constraint: constraint
                    .map(|constraint| {
                        state.substitute_type_operand(module, substitution, constraint)
                    })
                    .transpose()?,
            },
            Self::KeyOf { target } => Self::KeyOf {
                target: state.substitute_type_operand(module, substitution, *target)?,
            },
            Self::Mapped {
                parameter,
                modifiers,
                value,
            } => Self::Mapped {
                parameter: parameter.substitute(module, substitution, state)?,
                modifiers: *modifiers,
                value: state.substitute_type_operand(module, substitution, *value)?,
            },
            Self::StringMapping { mapping, argument } => Self::StringMapping {
                mapping: *mapping,
                argument: state.substitute_type_operand(module, substitution, *argument)?,
            },
            Self::BestCommon { elements } => Self::BestCommon {
                elements: state.substitute_type_operands(module, substitution, elements)?,
            },
            Self::Widen { source } => Self::Widen {
                source: state.substitute_type_operand(module, substitution, *source)?,
            },
            Self::Exclude { source, target } => Self::Exclude {
                source: state.substitute_type_operand(module, substitution, *source)?,
                target: state.substitute_type_operand(module, substitution, *target)?,
            },
            Self::Extract { source, target } => Self::Extract {
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
    /// Substitute generic arguments through this mapped parameter.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        state: &mut CheckState<'_>,
    ) -> CompilerResult<Self> {
        Ok(Self {
            name: self.name,
            symbol: self.symbol,
            constraint: state.substitute_type_operand(module, substitution, self.constraint)?,
            key_remap: self
                .key_remap
                .map(|key_remap| state.substitute_type_operand(module, substitution, key_remap))
                .transpose()?,
        })
    }
}

impl CheckState<'_> {
    /// Reduce one type extraction term.
    pub(in crate::check) fn reduce_extract_term(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        self.reduce_filter_term(origin, source, target, TypeFilter::Extract)
    }

    /// Reduce one conditional type expression.
    pub(in crate::check) fn reduce_conditional_term(
        &mut self,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let selected = match decision {
            Answer::Ready(true) => then_type,
            Answer::Ready(false) => else_type,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        Ok(Answer::Ready(selected))
    }

    /// Check selected conditional branches to satisfy one expected type.
    pub(in crate::check) fn expect_conditional_term(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;

        match decision {
            // constrain the selected branch
            Answer::Ready(true) => self.expect_conditional_branch(origin, then_type, expected)?,
            // constrain the selected branch
            Answer::Ready(false) => self.expect_conditional_branch(origin, else_type, expected)?,
            // constrain every possible branch
            Answer::Pending(_) => {
                self.expect_conditional_branch(origin, then_type, expected)?;
                self.expect_conditional_branch(origin, else_type, expected)?;
            }
        };

        Ok(Answer::Ready(()))
    }

    /// Reduce one indexed access type with a literal key.
    pub(in crate::check) fn reduce_type_index_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: TypeOperand,
        index: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Answer::Ready(left) = self.reduce_type_operand(origin, left)? else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Answer::Ready(index) = self.reduce_type_operand(origin, index)? else {
            return Ok(Answer::pending(index.dependencies(self)));
        };
        let Some(index) = self.type_operand_term_id(index)? else {
            return Ok(Answer::pending(index.dependencies(self)));
        };
        let index_term = self.inference.term(index);
        let Some(key) = index_term.static_key() else {
            let index = TypeOperand::Term(index);
            let operation = self
                .inference
                .push_term(TypeOperationTerm::Index { left, index });

            let term = self.type_term_operand(TypeTerm::Operation(operation));

            return Ok(Answer::Ready(term));
        };

        let Some(term) = self.resolve_member_type_operand(origin, module, left, &key, &[])? else {
            let term = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

            return Ok(Answer::Ready(term));
        };

        Ok(Answer::Ready(term))
    }

    /// Check one indexed access type to satisfy one expected type.
    pub(in crate::check) fn expect_type_index_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        left: TypeOperand,
        index: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<Answer<()>> {
        let term = match self.reduce_type_index_term(origin, module, left, index)? {
            Answer::Ready(term) => term,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            term,
            expected,
            Condition::Always,
        );

        Ok(Answer::Ready(()))
    }

    /// Reduce one string mapping application.
    pub(in crate::check) fn reduce_string_mapping_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        mapping: dir::StringMapping,
        argument: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Answer::Ready(argument) = self.reduce_type_operand(origin, argument)? else {
            return Ok(Answer::pending(argument.dependencies(self)));
        };
        let Some(argument) = self.type_operand_term_id(argument)? else {
            return Ok(Answer::pending(argument.dependencies(self)));
        };
        let literal = match self.inference.term(argument) {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::String(value))) => *value,
            TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String)) => {
                return Ok(Answer::Ready(argument.into()));
            }
            _ => {
                let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Error));

                return Ok(Answer::Ready(ty));
            }
        };
        let value = self.module(module).strings.get(literal);
        let value = Self::map_string_literal(mapping, value);
        let value = self.module(module).strings.intern(&value);
        let literal = dir::ScalarLiteral::String(value);

        let ty = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)));

        Ok(Answer::Ready(ty))
    }

    /// Map one string literal through a string mapping.
    fn map_string_literal(mapping: dir::StringMapping, value: &str) -> String {
        match mapping {
            dir::StringMapping::Uppercase => value.to_uppercase(),
            dir::StringMapping::Lowercase => value.to_lowercase(),
            dir::StringMapping::Capitalize => Self::capitalize_type_string(value),
            dir::StringMapping::Uncapitalize => Self::uncapitalize_type_string(value),
        }
    }

    /// Capitalize one type string.
    fn capitalize_type_string(value: &str) -> String {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut output = first.to_uppercase().collect::<String>();

        output.push_str(chars.as_str());

        output
    }

    /// Uncapitalize one type string.
    fn uncapitalize_type_string(value: &str) -> String {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut output = first.to_lowercase().collect::<String>();

        output.push_str(chars.as_str());

        output
    }

    /// Reduce one inferred mutable storage type.
    pub(in crate::check) fn reduce_widen_term(
        &mut self,
        origin: Origin,
        source: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        match self.widen_inferred_operand(origin, origin.module(), source)? {
            Some(term) => Ok(Answer::Ready(term)),
            None => Ok(Answer::pending(source.dependencies(self))),
        }
    }

    /// Widen one scalar literal.
    pub(in crate::check) fn widen_scalar_literal(literal: dir::ScalarLiteral) -> TypeLiteralTerm {
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
            dir::ScalarLiteral::Undefined => TypeLiteralTerm::Undefined,
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
        let Answer::Ready(operand) = self.reduce_type_operand(origin, operand)? else {
            return Ok(None);
        };
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(None);
        };

        self.widen_inferred_term(origin, module, operand, term)
    }

    /// Widen one inferred term.
    fn widen_inferred_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        operand: TypeOperand,
        term: TermId<TypeTerm>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let term = match self.inference.term(term) {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(literal)) => {
                TypeTerm::Literal(Self::widen_scalar_literal(*literal))
            }
            TypeTerm::Array { element } => {
                let element = *element;
                let Some(element) = self.widen_inferred_operand(origin, module, element)? else {
                    return Ok(None);
                };

                TypeTerm::Array { element }
            }
            TypeTerm::FixedArray { element, length } => {
                let element = *element;
                let length = *length;
                let Some(element) = self.widen_inferred_operand(origin, module, element)? else {
                    return Ok(None);
                };

                TypeTerm::FixedArray { element, length }
            }
            TypeTerm::Slice { element } => {
                let element = *element;
                let Some(element) = self.widen_inferred_operand(origin, module, element)? else {
                    return Ok(None);
                };

                TypeTerm::Slice { element }
            }
            TypeTerm::Tuple { form, elements } => {
                let form = *form;
                let elements = elements.iter().copied().collect::<Vec<_>>();
                let mut widened = Vec::with_capacity(elements.len());
                let mut is_changed = false;

                // widen elements in source order
                for element in elements {
                    let Some(ty) = self.widen_inferred_operand(origin, module, element.ty)? else {
                        return Ok(None);
                    };

                    is_changed |= ty != element.ty;
                    widened.push(TupleElement { ty, ..element });
                }
                if !is_changed {
                    return Ok(Some(operand));
                }

                TypeTerm::Tuple {
                    form,
                    elements: widened,
                }
            }
            TypeTerm::Shape(shape) => {
                let shape = *shape;
                let members = self
                    .inference
                    .term(shape)
                    .members
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();
                let mut widened = SmallVec::with_capacity(members.len());
                let mut is_changed = false;

                // widen members in source order
                for member in members {
                    let Some(widened_member) =
                        self.widen_inferred_shape_member(origin, module, member)?
                    else {
                        return Ok(None);
                    };

                    is_changed |= widened_member != member;
                    widened.push(widened_member);
                }
                if !is_changed {
                    return Ok(Some(operand));
                }

                let term = self.push_shape_type(widened);

                return Ok(Some(self.type_term_operand(term)));
            }
            TypeTerm::Union { elements } => {
                let elements = elements.iter().copied().collect::<Vec<_>>();
                let mut widened = Vec::with_capacity(elements.len());

                // widen elements in source order
                for element in elements {
                    let Some(element) = self.widen_inferred_operand(origin, module, element)?
                    else {
                        return Ok(None);
                    };

                    widened.push(element);
                }

                let Answer::Ready(term) = self.reduce_best_common_term(&widened)? else {
                    return Ok(None);
                };

                return Ok(Some(term));
            }
            TypeTerm::Intersection { elements } => {
                let elements = elements.iter().copied().collect::<Vec<_>>();
                let mut widened = Vec::with_capacity(elements.len());
                let mut is_changed = false;

                // widen elements in source order
                for element in elements {
                    let Some(widened_element) =
                        self.widen_inferred_operand(origin, module, element)?
                    else {
                        return Ok(None);
                    };

                    is_changed |= widened_element != element;
                    widened.push(widened_element);
                }
                if !is_changed {
                    return Ok(Some(operand));
                }

                TypeTerm::Intersection { elements: widened }
            }
            _ => return Ok(Some(operand)),
        };

        Ok(Some(self.type_term_operand(term)))
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
            ShapeMember::Spread {
                origin: spread_origin,
                source,
            } => ShapeMember::Spread {
                origin: spread_origin,
                source: match self.widen_inferred_operand(origin, module, source)? {
                    Some(source) => source,
                    None => return Ok(None),
                },
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

    /// Reduce one best common type term.
    pub(in crate::check) fn reduce_best_common_term(
        &mut self,
        elements: &[TypeOperand],
    ) -> CompilerResult<Answer<TypeOperand>> {
        if elements.is_empty() {
            let term = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Never));

            return Ok(Answer::Ready(term));
        }
        let mut candidates = Vec::with_capacity(elements.len());

        // collect element candidates
        for element in elements {
            let Some(_) = self.type_operand_term_id(*element)? else {
                return Ok(Answer::pending(element.dependencies(self)));
            };

            candidates.push(*element);
        }

        self.reduce_best_common_terms(&candidates)
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(in crate::check) fn reduce_best_common_terms(
        &mut self,
        candidates: &[TypeOperand],
    ) -> CompilerResult<Answer<TypeOperand>> {
        if candidates.is_empty() {
            let term = self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Never));

            return Ok(Answer::Ready(term));
        }

        // choose the first candidate that accepts every element
        for candidate in candidates {
            if self.is_best_common_candidate(*candidate, candidates)? {
                return Ok(Answer::Ready(*candidate));
            }
        }
        let mut elements = Vec::with_capacity(candidates.len());

        // preserve heterogeneous literal arrays as unions
        for candidate in candidates {
            elements.push(*candidate);
        }

        let term = self.type_term_operand(TypeTerm::Union { elements });

        Ok(Answer::Ready(term))
    }

    /// Reduce one type exclusion term.
    pub(in crate::check) fn reduce_exclude_term(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        target: TypeOperand,
    ) -> CompilerResult<Answer<TypeOperand>> {
        self.reduce_filter_term(origin, source, target, TypeFilter::Exclude)
    }

    /// Reduce one type filter term.
    fn reduce_filter_term(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        target: TypeOperand,
        filter: TypeFilter,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let Some(source_term_id) = self.type_operand_term_id(source)? else {
            return Ok(Answer::pending(source.dependencies(self)));
        };
        let Some(target_term_id) = self.type_operand_term_id(target)? else {
            return Ok(Answer::pending(target.dependencies(self)));
        };
        if let Some((source_form, source_value, target_value)) = {
            let source_term = self.inference.term(source_term_id);
            let target_term = self.inference.term(target_term_id);

            match (source_term, target_term) {
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
                    .inference
                    .term(*source_form)
                    .same_constructor(self.inference.term(*target_form)) =>
                {
                    Some((*source_form, *source_value, *target_value))
                }
                _ => None,
            }
        } {
            let payload =
                match self.reduce_filter_term(origin, source_value, target_value, filter)? {
                    Answer::Ready(payload) => payload,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
            let term = self.type_term_operand(TypeTerm::Form {
                form: source_form,
                payload,
            });

            return Ok(Answer::Ready(term));
        }

        if let TypeTerm::Union { elements: _ } = self.inference.term(source_term_id) {
            let term = match self.reduce_filter_union(source_term_id, target_term_id, filter)? {
                Answer::Ready(term) => term,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            return Ok(Answer::Ready(term));
        }

        let matches = match self.decide_type_term_id_relation(
            Some(origin.module()),
            TypeRelation::Assignable,
            source_term_id,
            target_term_id,
        )? {
            Answer::Ready(matches) => matches,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let term = if filter.keeps(matches) {
            source
        } else {
            self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Never))
        };

        Ok(Answer::Ready(term))
    }

    /// Return whether one candidate accepts every best-common element.
    fn is_best_common_candidate(
        &mut self,
        candidate: TypeOperand,
        elements: &[TypeOperand],
    ) -> CompilerResult<bool> {
        for element in elements {
            let decision =
                self.decide_type_relation(TypeRelation::Assignable, *element, candidate)?;
            if decision != Answer::Ready(true) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Reduce one union type filter.
    fn reduce_filter_union(
        &mut self,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
        filter: TypeFilter,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let TypeTerm::Union { elements } = self.inference.term(source) else {
            return Err(CompilerError::Internal {
                message: "type filter union reducer received a non-union source".into(),
            });
        };
        let len = elements.len();
        let mut kept = Vec::with_capacity(len);

        // keep elements selected by the filter
        for index in 0..len {
            let TypeTerm::Union { elements } = self.inference.term(source) else {
                return Err(CompilerError::Internal {
                    message: "type filter union source changed during reduction".into(),
                });
            };
            let Some(element) = elements.get(index).copied() else {
                return Err(CompilerError::Internal {
                    message: "type filter union element index is out of range".into(),
                });
            };
            let Some(element_term) = self.type_operand_term_id(element)? else {
                return Ok(Answer::pending(element.dependencies(self)));
            };
            let matches = match self.decide_filtered_type_term(element_term, target)? {
                Answer::Ready(matches) => matches,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            if filter.keeps(matches) {
                kept.push(element);
            }
        }

        let term = match kept.as_slice() {
            [] => self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Never)),
            [element] => *element,
            _ => self.type_term_operand(TypeTerm::Union { elements: kept }),
        };

        Ok(Answer::Ready(term))
    }

    /// Decide whether one source branch satisfies a filter target.
    fn decide_filtered_type_term(
        &mut self,
        source: TermId<TypeTerm>,
        target: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = if let TypeTerm::Union { elements } = self.inference.term(target) {
            let len = elements.len();

            // union targets accept any matching branch
            {
                let mut decision = Answer::Ready(false);

                for index in 0..len {
                    let TypeTerm::Union { elements } = self.inference.term(target) else {
                        return Err(CompilerError::Internal {
                            message: "type filter union target changed during reduction".into(),
                        });
                    };
                    let Some(element) = elements.get(index).copied() else {
                        return Err(CompilerError::Internal {
                            message: "type filter target element index is out of range".into(),
                        });
                    };
                    let Some(element) = self.type_operand_term_id(element)? else {
                        return Ok(Answer::pending(element.dependencies(self)));
                    };
                    let element = self.decide_type_term_id_relation(
                        None,
                        TypeRelation::Assignable,
                        source,
                        element,
                    )?;

                    decision = decision.or(element);
                }

                decision
            }
        } else {
            // single targets use assignability
            self.decide_type_term_id_relation(None, TypeRelation::Assignable, source, target)?
        };

        Ok(decision)
    }

    /// Check one conditional branch to satisfy the expected result.
    fn expect_conditional_branch(
        &mut self,
        origin: Origin,
        branch: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<()> {
        self.constrain_type(
            origin,
            TypeRelation::Assignable,
            branch,
            expected,
            Condition::Always,
        );

        Ok(())
    }
}

/// The selected type filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeFilter {
    /// Keep matching union members.
    Extract,
    /// Keep non-matching union members.
    Exclude,
}

impl TypeFilter {
    /// Return whether one filter decision keeps the source.
    fn keeps(self, decision: bool) -> bool {
        match self {
            Self::Extract => decision,
            Self::Exclude => !decision,
        }
    }
}
