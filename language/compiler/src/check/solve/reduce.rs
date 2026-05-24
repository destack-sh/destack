use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    ArgumentTerm, CheckComponentState, FormTerm, ShapeMemberTerm, StaticTerm, TupleElementTerm,
    TypeOperationTerm, TypeRelation, TypeTerm, VariableId,
};

use super::Decision;
use super::decompose::{shape_field, shape_field_type};
use super::queue::Progress;

/// How an expected type flows into one defining term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeExpectationMode {
    /// The defining term must have exactly the solved result type.
    Exact,
    /// The defining term must be assignable to an upper bound.
    UpperBound,
}

impl CheckComponentState<'_> {
    /// Reduce one type definition.
    pub(super) fn reduce_type_definition(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let forward = match self.reduce_type_term(result.module, term)? {
            Some(term) => self.solve_type_variable(result, term)?,
            None => Progress::Unchanged,
        };
        let backward = self.expect_type_term(result, term)?;

        Ok(forward.merge(backward))
    }

    /// Reduce one static definition constraint.
    pub(super) fn reduce_static_definition(
        &mut self,
        result: VariableId,
        term: &StaticTerm,
    ) -> CompilerResult<Progress> {
        let Some(term) = self.reduce_static_term(result.module, term)? else {
            return Ok(Progress::Unchanged);
        };

        self.solve_static_variable(result, term)
    }

    /// Apply an expected result type to the term that defines it.
    fn expect_type_term(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<Progress> {
        let upper_bounds = self
            .module(result.module)?
            .variable(result)
            .upper_bounds
            .clone();
        let mut progress = Progress::Unchanged;

        // push the exact result when it is known
        let Some(result_term) = self.solved_type_term(result)? else {
            return self.expect_type_term_upper_bounds(result, term, &upper_bounds);
        };
        progress = progress.merge(self.expect_type_term_result(
            result,
            result,
            &result_term,
            term,
            TypeExpectationMode::Exact,
        )?);

        // push solved upper bounds as contextual expectations
        progress =
            progress.merge(self.expect_type_term_upper_bounds(result, term, &upper_bounds)?);

        Ok(progress)
    }

    /// Apply solved upper bounds to the term that defines a variable.
    fn expect_type_term_upper_bounds(
        &mut self,
        result: VariableId,
        term: &TypeTerm,
        upper_bounds: &[VariableId],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // apply each solved upper bound independently
        for upper_bound in upper_bounds {
            let Some(expected) = self.solved_type_term(*upper_bound)? else {
                continue;
            };
            progress = progress.merge(self.expect_type_term_result(
                result,
                *upper_bound,
                &expected,
                term,
                TypeExpectationMode::UpperBound,
            )?);
        }

        Ok(progress)
    }

    /// Apply one expected result type to the term that defines it.
    fn expect_type_term_result(
        &mut self,
        result: VariableId,
        expected: VariableId,
        expected_term: &TypeTerm,
        term: &TypeTerm,
        mode: TypeExpectationMode,
    ) -> CompilerResult<Progress> {
        let progress = match term {
            TypeTerm::Variable(variable) => match mode {
                TypeExpectationMode::Exact => self.relate_type_equal(*variable, expected)?,
                TypeExpectationMode::UpperBound => {
                    self.relate_type_assignable(*variable, expected)?
                }
            },
            TypeTerm::Form { form, payload } => {
                let TypeTerm::Form {
                    form: result_form,
                    payload: result_payload,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let form = self.expect_form(form, &result_form)?;
                let payload = self.relate_type_assignable(*payload, *result_payload)?;

                form.merge(payload)
            }
            TypeTerm::Array { element } => {
                let TypeTerm::Array {
                    element: result_element,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.relate_type_assignable(*element, *result_element)?
            }
            TypeTerm::Slice {
                element,
                is_readonly: _,
            } => {
                let TypeTerm::Slice {
                    element: result_element,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.relate_type_assignable(*element, *result_element)?
            }
            TypeTerm::FixedArray {
                element,
                length,
                is_readonly: _,
            } => {
                let TypeTerm::FixedArray {
                    element: result_element,
                    length: result_length,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };
                let element = self.relate_type_assignable(*element, *result_element)?;
                let length = self.relate_static_equal(*length, *result_length)?;

                element.merge(length)
            }
            TypeTerm::Tuple {
                form: _,
                elements,
                is_readonly: _,
            } => {
                let TypeTerm::Tuple {
                    form: _,
                    elements: result_elements,
                    is_readonly: _,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_tuple_elements(elements, &result_elements)?
            }
            TypeTerm::Shape { members } => {
                let TypeTerm::Shape {
                    members: result_members,
                } = expected_term
                else {
                    return Ok(Progress::Unchanged);
                };

                self.expect_shape_members(members, &result_members)?
            }
            TypeTerm::Call(call) => self.expect_call_result(call, expected)?,
            TypeTerm::Construct(construct) => {
                self.expect_construct_result(result.module, construct, expected)?
            }
            TypeTerm::Operator(operator) => self.expect_operator_result(operator, expected)?,
            TypeTerm::Index(index) => self.expect_index_result(index, expected)?,
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => Progress::Unchanged,
            TypeTerm::Await(awaited) => self.expect_await_result(awaited, expected)?,
            TypeTerm::Try(tried) => self.expect_try_result(tried, expected)?,
            TypeTerm::Operation(TypeOperationTerm::Exclude { source, target: _ }) => {
                self.relate_type_assignable(*source, expected)?
            }
            TypeTerm::Operation(TypeOperationTerm::Conditional { .. })
            | TypeTerm::Operation(TypeOperationTerm::Index { .. })
            | TypeTerm::Operation(TypeOperationTerm::TemplateLiteral { .. })
            | TypeTerm::Operation(TypeOperationTerm::Infer { .. })
            | TypeTerm::Operation(TypeOperationTerm::KeyOf { .. })
            | TypeTerm::Operation(TypeOperationTerm::Mapped { .. })
            | TypeTerm::Operation(TypeOperationTerm::BestCommon { .. })
            | TypeTerm::Operation(TypeOperationTerm::Widen { .. })
            | TypeTerm::Operation(TypeOperationTerm::Intrinsic { .. })
            | TypeTerm::Member { .. }
            | TypeTerm::Reference { .. }
            | TypeTerm::Function(_)
            | TypeTerm::Range { .. }
            | TypeTerm::Union { .. }
            | TypeTerm::Intersection { .. }
            | TypeTerm::Predicate { .. }
            | TypeTerm::Intrinsic
            | TypeTerm::ConstAssertion
            | TypeTerm::Literal(_) => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Apply expected form fields to one form term.
    fn expect_form(&mut self, form: &FormTerm, target: &FormTerm) -> CompilerResult<Progress> {
        let progress = match (form, target) {
            (
                FormTerm::Borrowed { lifetime, access },
                FormTerm::Borrowed {
                    lifetime: target_lifetime,
                    access: target_access,
                },
            ) => {
                let lifetime = self.relate_static_equal(*lifetime, *target_lifetime)?;
                let access = self.relate_static_equal(*access, *target_access)?;

                lifetime.merge(access)
            }
            (
                FormTerm::Placed { place },
                FormTerm::Placed {
                    place: target_place,
                },
            ) => self.relate_static_equal(*place, *target_place)?,
            (FormTerm::Managed, FormTerm::Managed)
            | (FormTerm::Owned, FormTerm::Owned)
            | (FormTerm::Raw, FormTerm::Raw)
            | (FormTerm::Readonly, FormTerm::Readonly) => Progress::Unchanged,
            _ => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Apply expected tuple elements to a tuple term.
    fn expect_tuple_elements(
        &mut self,
        elements: &[TupleElementTerm],
        targets: &[TupleElementTerm],
    ) -> CompilerResult<Progress> {
        if elements.len() != targets.len() {
            return Ok(Progress::Unchanged);
        }
        let mut progress = Progress::Unchanged;

        // push each expected element type
        for (element, target) in elements.iter().zip(targets) {
            progress = progress.merge(self.relate_type_assignable(element.ty, target.ty)?);
        }

        Ok(progress)
    }

    /// Apply expected shape fields to a shape term.
    fn expect_shape_members(
        &mut self,
        members: &[ShapeMemberTerm],
        targets: &[ShapeMemberTerm],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // apply expected types to common fields
        for target in targets {
            let Some((target_key, target_ty)) = shape_field(target) else {
                continue;
            };
            let Some(member_ty) = shape_field_type(members, target_key) else {
                continue;
            };

            progress = progress.merge(self.relate_type_assignable(member_ty, target_ty)?);
        }

        Ok(progress)
    }

    /// Reduce one type term when the solver has enough input.
    fn reduce_type_term(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match term {
            TypeTerm::Variable(variable) => self.solved_type_term(*variable)?,
            TypeTerm::Member {
                source,
                owner,
                key,
                arguments,
            } if arguments.is_empty() => self.reduce_member_type(module, *source, *owner, *key)?,
            TypeTerm::Operation(TypeOperationTerm::Exclude { source, target }) => {
                self.reduce_exclude_type(module, *source, *target)?
            }
            TypeTerm::Operation(TypeOperationTerm::BestCommon { elements }) => {
                self.reduce_best_common_type(module, elements)?
            }
            TypeTerm::Operation(TypeOperationTerm::Widen { source }) => {
                self.reduce_widen_type(*source)?
            }
            TypeTerm::Operation(TypeOperationTerm::Intrinsic { item, arguments }) => {
                self.reduce_memory_type(module, *item, arguments)?
            }
            TypeTerm::Reference {
                source,
                symbol,
                arguments,
            } => self.reduce_named_type(module, *source, *symbol, arguments)?,
            TypeTerm::Call(call) => self.reduce_call_type(module, call)?,
            TypeTerm::Construct(construct) => self.reduce_construct_type(module, construct)?,
            TypeTerm::Operator(operator) => self.reduce_operator_type(operator)?,
            TypeTerm::Index(index) => self.reduce_index_type(module, index)?,
            TypeTerm::KeyMembership(_)
            | TypeTerm::InstanceCheck(_)
            | TypeTerm::Identity(_)
            | TypeTerm::Template(_)
            | TypeTerm::TaggedTemplate(_) => None,
            TypeTerm::Await(awaited) => self.reduce_await_type(module, awaited)?,
            TypeTerm::Try(tried) => self.reduce_try_type(module, tried)?,
            TypeTerm::Member { .. } => None,
            term => Some(term.clone()),
        };

        Ok(term)
    }

    /// Reduce one static term when the solver has enough input.
    fn reduce_static_term(
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
            StaticTerm::LifetimeJoin { elements } => {
                let Some(term) = self.reduce_lifetime_join(module, elements)? else {
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

    /// Reduce one inferred mutable storage type.
    fn reduce_widen_type(&mut self, source: VariableId) -> CompilerResult<Option<TypeTerm>> {
        let Some(term) = self.solved_type_term(source)? else {
            return Ok(None);
        };
        let term = Self::widen_inferred_type(term);

        Ok(Some(term))
    }

    /// Reduce one named type reference when it names a structural alias.
    fn reduce_named_type(
        &mut self,
        module: ModuleId,
        source: Option<dir::GlobalNodeIdAny>,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(value) = self.structural_alias_value(symbol)? else {
            return Ok(Some(TypeTerm::Reference {
                source,
                symbol,
                arguments: arguments.to_vec(),
            }));
        };
        let value = self
            .module_mut(symbol.module_id)?
            .type_expression_variable(value);
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(None);
        };
        if arguments.is_empty() {
            return Ok(Some(term));
        }
        let substitution = self.generic_substitution(symbol, arguments)?;
        let term = self.substitute_type_term(module, &substitution, &term)?;

        Ok(term)
    }

    /// Return the value expression for one non nominal type alias.
    fn structural_alias_value(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::LocalNodeId<dir::TypeExpression>>> {
        if !self.modules.contains_key(&symbol.module_id) {
            return Ok(None);
        }
        let module = self.module(symbol.module_id)?;
        let Some(node) = module.symbol_source_node(symbol) else {
            return Ok(None);
        };
        if node.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
        let declaration = module.input.parsed.tree.get(declaration);
        let value = match declaration {
            dir::Declaration::Type(declaration) if !declaration.is_nominal => {
                Some(declaration.value)
            }
            _ => None,
        };

        Ok(value)
    }

    /// Reduce one best common type term.
    pub(super) fn reduce_best_common_type(
        &mut self,
        module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<TypeTerm>> {
        if elements.is_empty() {
            return Ok(Some(TypeTerm::Literal(dir::Type::Unknown)));
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
            let _ = self.expect_type(*element, &element_term, &term)?;
        }

        Ok(Some(term))
    }

    /// Reduce a concrete set of candidate terms to their best common type.
    pub(super) fn reduce_best_common_terms(
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

    /// Reduce one type exclusion term.
    fn reduce_exclude_type(
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
                TypeTerm::Literal(dir::Type::Never)
            }
            _ => source_term,
        };

        Ok(Some(term))
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

    /// Reduce one lifetime union.
    fn reduce_lifetime_join(
        &mut self,
        module: ModuleId,
        elements: &[VariableId],
    ) -> CompilerResult<Option<dir::StaticTerm>> {
        let mut lifetimes = Vec::with_capacity(elements.len());

        // collect solved lifetime element ids
        for element in elements {
            let Some(term) = self.static_value(*element)? else {
                return Ok(None);
            };
            let dir::StaticTerm::Lifetime { .. } = term else {
                return Ok(None);
            };
            let lifetime = self.module_mut(module)?.intern_static(term);

            lifetimes.push(lifetime);
        }

        Ok(Some(dir::StaticTerm::Lifetime {
            lifetime: dir::Lifetime::Join(lifetimes),
        }))
    }
}
