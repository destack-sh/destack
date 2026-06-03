use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, GenericArgument, Origin, Progress, Reduction, ShapeMember, Solution,
    Substitution, TupleElement, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};

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
    /// Return whether this operation is stable semantic output.
    pub(in crate::check) fn is_stable(&self) -> bool {
        matches!(
            self,
            Self::Conditional { .. }
                | Self::Index { .. }
                | Self::TemplateLiteral { .. }
                | Self::Infer { .. }
                | Self::KeyOf { .. }
                | Self::Mapped { .. }
                | Self::StringMapping { .. }
        )
    }

    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        match self {
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                let mut variables = left.referenced_variables(state);
                variables.extend(right.referenced_variables(state));
                variables.extend(then_type.referenced_variables(state));
                variables.extend(else_type.referenced_variables(state));

                variables
            }
            Self::Index { left, index } => {
                let mut variables = left.referenced_variables(state);
                variables.extend(index.referenced_variables(state));

                variables
            }
            Self::TemplateLiteral { strings: _, spans } => spans
                .iter()
                .flat_map(|span| span.referenced_variables(state))
                .collect(),
            Self::Infer {
                name: _,
                constraint,
            } => constraint
                .iter()
                .flat_map(|constraint| constraint.referenced_variables(state))
                .collect(),
            Self::KeyOf { target } => target.referenced_variables(state),
            Self::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                let mut variables = parameter.referenced_variables(state);

                variables.extend(value.referenced_variables(state));

                variables
            }
            Self::StringMapping {
                mapping: _,
                argument,
            } => argument.referenced_variables(state),
            Self::BestCommon { elements } => elements
                .iter()
                .flat_map(|element| element.referenced_variables(state))
                .collect(),
            Self::Widen { source } => {
                let mut variables = source.referenced_variables(state);

                // watch nested operands once the source has a structural solution
                if let TypeOperand::Variable(source) = source
                    && let Some(Solution::Type(solution)) =
                        state.inference.variable_solution(*source)
                {
                    let operand = TypeOperand::from(solution);

                    variables.extend(operand.referenced_variables(state));
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
                .flat_map(|argument| argument.referenced_variables(state))
                .collect(),
        }
    }

    /// Substitute generic arguments through this type operation.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: Substitution<'_>,
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
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = self.constraint.referenced_variables(state);

        variables.extend(
            self.key_remap
                .iter()
                .flat_map(|key_remap| key_remap.referenced_variables(state)),
        );

        variables
    }

    /// Substitute generic arguments through this mapped parameter.
    pub(in crate::check) fn substitute(
        &self,
        module: ModuleId,
        substitution: Substitution<'_>,
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
    /// Reduce one conditional type expression.
    pub(in crate::check) fn reduce_conditional_term(
        &self,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let selected = match decision {
            Decision::Yes => then_type,
            Decision::No => else_type,
            Decision::Undecidable => return Ok(None),
        };

        self.type_operand_term(selected)
    }

    /// Expect selected conditional branches to satisfy one expected type.
    pub(in crate::check) fn expect_conditional_term(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
        then_type: TypeOperand,
        else_type: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<Progress> {
        let decision = self.decide_type_relation(TypeRelation::Extends, left, right)?;
        let progress = match decision {
            // constrain the selected branch
            Decision::Yes => self.expect_conditional_branch(origin, then_type, expected)?,
            // constrain the selected branch
            Decision::No => self.expect_conditional_branch(origin, else_type, expected)?,
            // constrain every possible branch
            Decision::Undecidable => {
                let then_type = self.expect_conditional_branch(origin, then_type, expected)?;
                let else_type = self.expect_conditional_branch(origin, else_type, expected)?;

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
        left: TypeOperand,
        index: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(left) = self.type_operand_term(left)? else {
            return Ok(None);
        };
        let left = match self.reduce_type_term(origin, &left)? {
            Reduction {
                value: Some(term), ..
            } => term,
            Reduction { value: None, .. } => left,
        };
        let Some(index) = self.type_operand_term(index)? else {
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
        left: TypeOperand,
        index: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.reduce_type_index_term(origin, module, left, index)? else {
            return Ok(Progress::Unchanged);
        };
        let term = self.inference.push_term(term);
        let progress = self.relate_contextual_type_assignability(origin, term, expected)?;

        Ok(progress)
    }

    /// Reduce one string mapping application.
    pub(in crate::check) fn reduce_string_mapping_term(
        &mut self,
        origin: Origin,
        module: ModuleId,
        mapping: dir::StringMapping,
        argument: TypeOperand,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(argument) = self.reduce_type_operand(origin, argument)? else {
            return Ok(None);
        };
        let literal = match argument {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::String(value))) => value,
            TypeTerm::Literal(TypeLiteralTerm::Primitive(dir::PrimitiveType::String)) => {
                return Ok(Some(argument));
            }
            _ => return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Error))),
        };
        let value = self.module(module).strings.get(literal);
        let value = Self::apply_string_mapping(mapping, value);
        let value = self.module(module).strings.intern(&value);
        let literal = dir::ScalarLiteral::String(value);

        Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Scalar(literal))))
    }

    /// Apply one string mapping.
    fn apply_string_mapping(mapping: dir::StringMapping, value: &str) -> String {
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
            TypeTerm::FixedArray { element, length } => TypeTerm::FixedArray {
                element: match self.widen_inferred_operand(origin, module, element)? {
                    Some(element) => element,
                    None => return Ok(None),
                },
                length,
            },
            TypeTerm::Slice { element } => TypeTerm::Slice {
                element: match self.widen_inferred_operand(origin, module, element)? {
                    Some(element) => element,
                    None => return Ok(None),
                },
            },
            TypeTerm::Tuple { form, elements } => {
                let mut widened = Vec::with_capacity(elements.len());

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
                }
            }
            TypeTerm::Shape(shape) => {
                let members = self.inference.term(shape).members.clone();
                let mut widened = SmallVec::with_capacity(members.len());

                // widen members in source order
                for member in &members {
                    let Some(member) =
                        self.widen_inferred_shape_member(origin, module, member.clone())?
                    else {
                        return Ok(None);
                    };

                    widened.push(member);
                }
                if widened.as_slice() == members.as_slice() {
                    return Ok(Some(TypeTerm::Shape(shape)));
                }

                self.push_shape_type(widened)
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
        source: TypeOperand,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let Some(source_term) = self.type_operand_term(source)? else {
            return Ok(Reduction::pending());
        };
        let Some(term) = self.widen_inferred_type(origin, origin.module(), source_term.clone())?
        else {
            return Ok(Reduction::pending());
        };

        Ok(Reduction::value(term))
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
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };
        let Some(term) = self.reduce_type_operand_term(origin, term)? else {
            return Ok(None);
        };
        let Some(widened) = self.widen_inferred_type(origin, module, term.clone())? else {
            return Ok(None);
        };
        if widened == term {
            return Ok(Some(operand));
        }

        let widened = self.inference.push_term(widened);

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

    /// Expect a widened source term to satisfy one expected type.
    pub(in crate::check) fn expect_widen_term(
        &mut self,
        origin: Origin,
        source: TypeOperand,
        expected: TypeOperand,
        _expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        self.relate_contextual_type_assignability(origin, source, expected)
    }

    /// Reduce one best common type term.
    pub(in crate::check) fn reduce_best_common_term(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        elements: &[TypeOperand],
    ) -> CompilerResult<Option<TypeTerm>> {
        if elements.is_empty() {
            panic!("best common type requires at least one element");
        }
        let mut candidates = Vec::with_capacity(elements.len());

        // collect element candidates
        for element in elements {
            let Some(term) = self.type_operand_term(*element)? else {
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
        _result: Option<VariableId>,
        elements: &[TypeOperand],
        expected: TypeOperand,
        _expected_term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // push the expected type into every element
        for element in elements {
            progress = progress
                .merge(self.relate_contextual_type_assignability(origin, *element, expected)?);
        }

        Ok(progress)
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(in crate::check) fn reduce_best_common_terms(
        &mut self,
        _module: ModuleId,
        candidates: Vec<TypeTerm>,
    ) -> CompilerResult<TypeTerm> {
        if candidates.is_empty() {
            panic!("best common type requires at least one candidate");
        }

        // choose the first candidate that accepts every element
        for candidate in &candidates {
            if self.is_best_common_candidate(candidate, &candidates)? {
                return Ok(candidate.clone());
            }
        }
        let mut elements = Vec::with_capacity(candidates.len());

        // preserve heterogeneous literal arrays as unions
        for candidate in candidates {
            let candidate = self.inference.push_term(candidate);

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
                .inference
                .term(*source_form)
                .same_constructor(self.inference.term(*target_form)) =>
            {
                let Some(payload) =
                    self.reduce_exclude_term(module, *source_value, *target_value)?
                else {
                    return Ok(None);
                };
                let payload = self.inference.push_term(payload);

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
            let Some(term) = self.type_operand_term(kept[0])? else {
                return Ok(TypeTerm::Union { elements: kept });
            };

            term
        } else {
            TypeTerm::Union { elements: kept }
        };

        Ok(term)
    }

    /// Expect one conditional branch to satisfy the expected result.
    fn expect_conditional_branch(
        &mut self,
        origin: Origin,
        branch: TypeOperand,
        expected: TypeOperand,
    ) -> CompilerResult<Progress> {
        self.relate_contextual_type_assignability(origin, branch, expected)
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
