use std::collections::{HashMap, HashSet};

use super::SignatureResolutionMode;
use crate::analyze::common::{InferContext, ModuleTypeView, NormalizationMode, RelationMode};
use crate::{AnalyzeResult, Assignability, Compiler, InferState};
use destack_dir::{
    Argument, Constraint, Expression, InferOrigin, InferScope, InferVarId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, PrimitiveType, ResolvedSignature, ScalarLiteral, StringId,
    SymbolSpaceOrder, TemplateLiteral, Type, TypeLiteral, TypeTable,
};
use destack_workspace::ProfileId;

/// Prepared inference metadata for a template literal span.
#[derive(Debug)]
struct TemplateSpanInferenceTarget {
    /// The inference variable type id used for constraints.
    infer_ty_id: LocalTypeId,
    /// The resolved constraint type when available.
    constraint_id: Option<LocalTypeId>,
}

/// Shared context for template-literal argument inference.
#[derive(Clone, Copy)]
struct TemplateInferenceContext {
    /// The argument node being inferred.
    argument_id: LocalNodeId<Argument>,
    /// The argument type id.
    argument_ty_id: LocalTypeId,
    /// The parameter template type id.
    param_ty_id: LocalTypeId,
    /// The node used to allocate span-local inference types.
    span_node: LocalNodeIdAny,
    /// The source node for inferred literal types.
    source_node: LocalNodeIdAny,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Report a template literal inference mismatch.
    fn report_template_inference_unassignable(
        &self,
        ctx: ModuleTypeView<'_>,
        argument_id: LocalNodeId<Argument>,
        param_ty_id: LocalTypeId,
        argument_ty_id: LocalTypeId,
    ) {
        self.emit_unassignable_type_for_types(
            ctx,
            argument_id.into_any(),
            param_ty_id,
            argument_ty_id,
        );
    }

    /// Infer a tagged template expression.
    pub(crate) fn infer_tagged_template_expression(
        &self,
        ctx: &mut InferContext<'_>,
        expression_id: LocalNodeId<Expression>,
        tag_id: LocalNodeId<Expression>,
        template: &TemplateLiteral,
        state: &mut InferState,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the tag expression type
        let tag_ty_id = self.infer_expression(&mut ctx.reborrow(), tag_id, state)?;

        // assemble template arguments for call resolution
        let template_strings_ty_id =
            self.template_strings_argument_type(ctx.profile, expression_id.into_any(), ctx.types);
        let template_arguments = match template {
            TemplateLiteral::String { .. } => &[][..],
            TemplateLiteral::InterpolatedString { arguments, .. } => arguments.as_slice(),
        };

        // require callable signatures on the tag
        let call_signatures = self.call_signatures_for_type(tag_ty_id, ctx.types);
        if call_signatures.is_empty() {
            self.emit_non_callable_for_callee_type(
                ctx.module_type_view(),
                expression_id.into_any(),
                tag_ty_id,
            );

            for argument_id in template_arguments {
                self.infer_argument(&mut ctx.reborrow(), *argument_id, None, state)?;
            }

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        }

        // prepare callee metadata for overload resolution
        let callee_symbol = self.reference_symbol_for_expression(ctx.tree_symbol_view(), tag_id);
        let generic_arguments = match ctx.tree.get(tag_id) {
            Expression::LocalReference {
                generic_arguments, ..
            }
            | Expression::ModuleReference {
                generic_arguments, ..
            }
            | Expression::GlobalReference {
                generic_arguments, ..
            }
            | Expression::Member {
                generic_arguments, ..
            } => Some(generic_arguments.as_slice()),
            _ => None,
        };

        // resolve the applicable tagged template overload
        let mut resolved_signature = None;
        if call_signatures.len() > 1 {
            // NOTE #Performance: tagged template overload resolution rechecks every signature
            let mut candidates = Vec::new();
            for signature_ty_id in call_signatures.iter() {
                let Some(resolved) = self.resolve_call_signature(
                    &mut ctx.reborrow(),
                    *signature_ty_id,
                    super::call::CallSignatureResolutionContext {
                        expression_id,
                        callee_symbol,
                        generic_arguments: generic_arguments,
                        prefilled_static_arguments: None,
                        bound_substitutions: None,
                        arguments: None,
                        call_receiver_ty_id: None,
                        expected_return_type: None,
                        mode: SignatureResolutionMode::Synthesize,
                        allow_missing_value_arguments: false,
                    },
                )?
                else {
                    continue;
                };

                let Some(resolved) = self.slice_tagged_template_signature(
                    &mut ctx.reborrow(),
                    template_strings_ty_id,
                    resolved,
                    true,
                )?
                else {
                    continue;
                };

                if !self.is_signature_applicable(
                    &mut ctx.reborrow(),
                    &resolved,
                    template_arguments,
                )? {
                    continue;
                }

                candidates.push((*signature_ty_id, resolved));
            }

            let mut candidates = self.dedupe_signature_candidates(&mut ctx.reborrow(), candidates);
            if candidates.is_empty() {
                self.emit_no_overload_for_receiver_type(
                    ctx.module_type_view(),
                    expression_id.into_any(),
                    tag_ty_id,
                );

                for argument_id in template_arguments {
                    self.infer_argument(&mut ctx.reborrow(), *argument_id, None, state)?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(ctx.types.insert_type_from(ty, expression_id));
            }

            resolved_signature = Some(candidates.remove(0).1);
        } else if let Some(signature_ty_id) = call_signatures.first().copied() {
            let resolved = self.resolve_call_signature(
                &mut ctx.reborrow(),
                signature_ty_id,
                super::call::CallSignatureResolutionContext {
                    expression_id,
                    callee_symbol,
                    generic_arguments: generic_arguments,
                    prefilled_static_arguments: None,
                    bound_substitutions: None,
                    arguments: None,
                    call_receiver_ty_id: None,
                    expected_return_type: None,
                    mode: SignatureResolutionMode::Synthesize,
                    allow_missing_value_arguments: false,
                },
            )?;
            if let Some(resolved) = resolved {
                resolved_signature = self.slice_tagged_template_signature(
                    &mut ctx.reborrow(),
                    template_strings_ty_id,
                    resolved,
                    false,
                )?;
            }
        }

        let Some(resolved) = resolved_signature else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(ctx.types.insert_type_from(ty, expression_id));
        };

        // infer argument types and emit invocation constraints
        let parameter_types = resolved.parameters.clone();
        let argument_ty_ids = self.infer_invocation_arguments(
            &mut ctx.reborrow(),
            template_arguments,
            &parameter_types,
            None,
            state,
        )?;

        // emit assignability errors for the tag parameters
        for ((argument_id, argument_ty_id), param_ty_id) in template_arguments
            .iter()
            .zip(argument_ty_ids.iter())
            .zip(parameter_types.iter())
        {
            let argument = ctx.tree.get(*argument_id);
            if matches!(argument, Argument::Spread { .. }) {
                continue;
            }

            if self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                *param_ty_id,
                *argument_ty_id,
            ) == Assignability::NotAssignable
            {
                self.emit_unassignable_type_for_types(
                    ctx.module_type_view(),
                    argument.value().into_any(),
                    *param_ty_id,
                    *argument_ty_id,
                );
            }
        }

        Ok(resolved.return_type.unwrap_or_else(|| {
            ctx.types.insert_type_from(
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                },
                expression_id,
            )
        }))
    }

    /// Build the template strings argument type for tagged templates.
    fn template_strings_argument_type(
        &self,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        // prefer the builtin template strings array type when available
        let name = self.repository.strings.intern("TemplateStringsArray");
        if let Some(symbol) =
            self.get_declared_library_symbol_from(profile, name, SymbolSpaceOrder::TypeThenValue)
        {
            return types.insert_type_from_any(
                Type::Reference {
                    symbol,
                    generic_arguments: None,
                },
                source_id,
            );
        }

        // fall back to readonly string arrays when no lib type exists
        let string_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            },
            source_id,
        );
        types.insert_type_from_any(
            Type::Array {
                element: Some(string_ty_id),
                is_readonly: true,
            },
            source_id,
        )
    }

    /// Drop the template strings parameter when resolving tagged template signatures.
    fn slice_tagged_template_signature(
        &self,
        ctx: &mut InferContext<'_>,
        template_strings_ty_id: LocalTypeId,
        resolved: ResolvedSignature,
        require_assignable: bool,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        // read the template strings parameter when present
        let Some(strings_param_ty_id) = resolved.parameters.first().copied() else {
            return Ok(None);
        };
        let strings_source_id = ctx.types.get_type_source(strings_param_ty_id);

        // ensure instance types are materialized for the template parameter
        self.ensure_reference_instance_types_for_type(
            &mut ctx.type_context_reborrow(),
            strings_source_id,
            strings_param_ty_id,
        )?;

        // reject template strings arguments that are not assignable
        if self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            strings_param_ty_id,
            template_strings_ty_id,
        ) == Assignability::NotAssignable
        {
            if require_assignable {
                return Ok(None);
            }

            self.emit_unassignable_type_for_types(
                ctx.module_type_view(),
                ctx.types.get_type_source(strings_param_ty_id),
                strings_param_ty_id,
                template_strings_ty_id,
            );
        }

        // drop the template strings parameter and normalize the signature
        let parameters = resolved.parameters.iter().skip(1).copied().collect();
        Ok(Some(ResolvedSignature {
            parameters: parameters,
            return_type: resolved.return_type,
            generic_arguments: resolved.generic_arguments,
        }))
    }

    /// Infer template spans from a string literal argument.
    fn infer_template_literal_from_string_argument(
        &self,
        context: &TemplateInferenceContext,
        strings: &[StringId],
        spans: &[LocalTypeId],
        value: &str,
        ctx: &mut InferContext<'_>,
    ) {
        // align the literal segments with the template
        let Some(span_values) = self.match_template_literal_to_string(strings, value) else {
            return;
        };

        // require aligned span counts
        if span_values.len() != spans.len() {
            return;
        }

        // match each span against the literal segments
        let mut repeated_span_values = HashMap::new();
        let mut visited = HashSet::new();
        for (span_ty_id, span_value) in spans.iter().zip(span_values.iter()) {
            if !self.infer_template_span_from_string_value(
                context,
                *span_ty_id,
                span_value,
                &mut ctx.reborrow(),
                &mut repeated_span_values,
                &mut visited,
            ) {
                break;
            }
        }
    }

    /// Infer template spans from a template literal argument.
    fn infer_template_literal_from_template_argument(
        &self,
        context: &TemplateInferenceContext,
        strings: &[StringId],
        spans: &[LocalTypeId],
        argument_strings: &[StringId],
        argument_spans: &[LocalTypeId],
        ctx: &mut InferContext<'_>,
    ) {
        // handle `${infer}` templates that capture the entire argument
        if self.infer_template_literal_full_span(context, strings, spans, &mut ctx.reborrow()) {
            return;
        }

        // require aligned shapes and literal parts
        if strings.len() != argument_strings.len() || spans.len() != argument_spans.len() {
            return;
        }

        // require matching literal parts
        for (left, right) in strings.iter().zip(argument_strings.iter()) {
            if left != right {
                return;
            }
        }

        // infer spans pairwise
        let mut repeated_span_types = HashMap::new();
        for (span_ty_id, argument_span) in spans.iter().zip(argument_spans.iter()) {
            if !self.infer_template_span_from_template_argument(
                context,
                *span_ty_id,
                *argument_span,
                &mut ctx.reborrow(),
                &mut repeated_span_types,
            ) {
                break;
            }
        }
    }

    /// Infer a single template span from a string literal value.
    fn infer_template_span_from_string_value(
        &self,
        context: &TemplateInferenceContext,
        span_ty_id: LocalTypeId,
        span_value: &str,
        ctx: &mut InferContext<'_>,
        repeated_span_values: &mut HashMap<LocalTypeId, StringId>,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            &mut ctx.reborrow(),
            span_ty_id,
            context.span_node,
            context.source_node,
        );
        let Some(target) = target else {
            return self.template_span_matches_string(
                &mut ctx.type_context_reborrow(),
                span_ty_id,
                span_value,
                visited,
            );
        };
        let span_value_id = self.repository.strings.intern(span_value);
        if let Some(previous_span_value_id) = repeated_span_values.get(&target.infer_ty_id) {
            if *previous_span_value_id != span_value_id {
                self.report_template_inference_unassignable(
                    ctx.module_type_view(),
                    context.argument_id,
                    context.param_ty_id,
                    context.argument_ty_id,
                );
                return false;
            }
        } else {
            repeated_span_values.insert(target.infer_ty_id, span_value_id);
        }

        // reject spans that violate the constraint
        if let Some(constraint_id) = target.constraint_id
            && !self.template_span_matches_string(
                &mut ctx.type_context_reborrow(),
                constraint_id,
                span_value,
                visited,
            )
        {
            self.report_template_inference_unassignable(
                ctx.module_type_view(),
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
            );
            return false;
        }

        // infer a literal type when possible
        let inferred_ty = self.template_infer_literal_type(
            target.constraint_id,
            span_value,
            context.source_node,
            ctx.types,
        );
        if let Some(inferred_ty) = inferred_ty {
            ctx.infer.push_constraint(Constraint::Equal {
                left: target.infer_ty_id,
                right: inferred_ty,
            });
        }

        true
    }

    /// Infer a single template span from a template literal argument span.
    fn infer_template_span_from_template_argument(
        &self,
        context: &TemplateInferenceContext,
        span_ty_id: LocalTypeId,
        argument_span: LocalTypeId,
        ctx: &mut InferContext<'_>,
        repeated_span_types: &mut HashMap<LocalTypeId, LocalTypeId>,
    ) -> bool {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            &mut ctx.reborrow(),
            span_ty_id,
            context.span_node,
            context.source_node,
        );
        let Some(target) = target else {
            return true;
        };

        if let Some(previous_argument_span) = repeated_span_types.get(&target.infer_ty_id).copied()
        {
            if !self.template_inference_repeated_span_types_match(
                &mut ctx.reborrow(),
                previous_argument_span,
                argument_span,
            ) {
                self.report_template_inference_unassignable(
                    ctx.module_type_view(),
                    context.argument_id,
                    context.param_ty_id,
                    context.argument_ty_id,
                );
                return false;
            }
        } else {
            repeated_span_types.insert(target.infer_ty_id, argument_span);
        };

        // validate argument spans against constraints
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                constraint_id,
                argument_span,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                ctx.module_type_view(),
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
            );
            return false;
        }

        // record inference bindings for the span
        ctx.infer.push_constraint(Constraint::Equal {
            left: target.infer_ty_id,
            right: argument_span,
        });
        true
    }

    /// Return true when repeated template span argument types match in both directions.
    fn template_inference_repeated_span_types_match(
        &self,
        ctx: &mut InferContext<'_>,
        left_type_id: LocalTypeId,
        right_type_id: LocalTypeId,
    ) -> bool {
        let left_type_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            left_type_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let right_type_id = self.normalize_type_with_relation(
            &mut ctx.type_context_reborrow(),
            right_type_id,
            NormalizationMode::Assign,
            RelationMode::ASSIGN,
        );
        let left_to_right = self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            left_type_id,
            right_type_id,
        ) != Assignability::NotAssignable;
        if !left_to_right {
            return false;
        }

        self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
            right_type_id,
            left_type_id,
        ) != Assignability::NotAssignable
    }

    /// Infer the full template literal into a single span.
    fn infer_template_literal_full_span(
        &self,
        context: &TemplateInferenceContext,
        strings: &[StringId],
        spans: &[LocalTypeId],
        ctx: &mut InferContext<'_>,
    ) -> bool {
        // require a single empty span template
        if !strings
            .iter()
            .all(|string_id| self.repository.strings.get(*string_id).is_empty())
            || spans.len() != 1
        {
            return false;
        }

        // resolve the span inference target
        let target = self.template_span_inference_target(
            &mut ctx.reborrow(),
            spans[0],
            context.span_node,
            context.source_node,
        );
        let Some(target) = target else {
            return true;
        };

        // validate the constraint against the argument
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                &mut ctx.type_context_reborrow(),
                constraint_id,
                context.argument_ty_id,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                ctx.module_type_view(),
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
            );
            return true;
        }

        // bind the inference variable to the full argument
        ctx.infer.push_constraint(Constraint::Equal {
            left: target.infer_ty_id,
            right: context.argument_ty_id,
        });

        true
    }

    /// Add inference constraints for template literal parameters.
    pub(crate) fn add_template_literal_inference_constraints(
        &self,
        ctx: &mut InferContext<'_>,
        arguments: &[LocalNodeId<Argument>],
        argument_ty_ids: &[LocalTypeId],
        parameter_types: &[LocalTypeId],
    ) {
        // scan argument pairs for template literal inference
        for ((argument_id, argument_ty_id), param_ty_id) in arguments
            .iter()
            .zip(argument_ty_ids.iter())
            .zip(parameter_types.iter())
        {
            let (strings, spans) = match ctx.types.get_type(*param_ty_id) {
                Type::TemplateLiteral { strings, spans } => (strings.clone(), spans.clone()),
                _ => continue,
            };

            // collect argument context
            let argument_value_id = ctx.tree.get(*argument_id).value();
            let source_node = argument_value_id.into_any();
            let span_node = argument_id.into_any();
            let argument_ty = ctx.types.get_type(*argument_ty_id).clone();
            let context = TemplateInferenceContext {
                argument_id: *argument_id,
                argument_ty_id: *argument_ty_id,
                param_ty_id: *param_ty_id,
                span_node,
                source_node,
            };

            // infer from string or template literal arguments
            match argument_ty {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                } => {
                    let value = self.repository.strings.get(string_id).to_string();
                    self.infer_template_literal_from_string_argument(
                        &context,
                        &strings,
                        &spans,
                        &value,
                        &mut ctx.reborrow(),
                    );
                }
                Type::Union { elements } => {
                    self.infer_template_from_union_string_argument(
                        &context,
                        &strings,
                        &spans,
                        &elements,
                        &mut ctx.reborrow(),
                    );
                }
                Type::TemplateLiteral {
                    strings: argument_strings,
                    spans: argument_spans,
                } => {
                    self.infer_template_literal_from_template_argument(
                        &context,
                        &strings,
                        &spans,
                        &argument_strings,
                        &argument_spans,
                        &mut ctx.reborrow(),
                    );
                }
                _ => {}
            }
        }
    }

    /// Infer template spans from a union of string literal arguments.
    fn infer_template_from_union_string_argument(
        &self,
        context: &TemplateInferenceContext,
        strings: &[StringId],
        spans: &[LocalTypeId],
        elements: &[LocalTypeId],
        ctx: &mut InferContext<'_>,
    ) {
        // collect span values for each union member when all members are string literals
        let mut union_span_values = Vec::with_capacity(elements.len());
        for element_ty_id in elements {
            let Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
            } = ctx.types.get_type(*element_ty_id)
            else {
                return;
            };

            let value = self.repository.strings.get(*string_id).to_string();
            let Some(span_values) = self.match_template_literal_to_string(strings, &value) else {
                return;
            };
            if span_values.len() != spans.len() {
                return;
            }

            union_span_values.push(span_values);
        }

        // infer each parameter span from the union member span values
        for (span_index, span_ty_id) in spans.iter().enumerate() {
            let Some(target) = self.template_span_inference_target(
                &mut ctx.reborrow(),
                *span_ty_id,
                context.span_node,
                context.source_node,
            ) else {
                continue;
            };

            let mut inferred_span_types = Vec::new();
            for span_values in &union_span_values {
                let span_value = &span_values[span_index];

                if let Some(constraint_id) = target.constraint_id {
                    let mut visited = HashSet::new();
                    if !self.template_span_matches_string(
                        &mut ctx.type_context_reborrow(),
                        constraint_id,
                        span_value,
                        &mut visited,
                    ) {
                        self.report_template_inference_unassignable(
                            ctx.module_type_view(),
                            context.argument_id,
                            context.param_ty_id,
                            context.argument_ty_id,
                        );
                        return;
                    }
                }

                if let Some(inferred_ty) = self.template_infer_literal_type(
                    target.constraint_id,
                    span_value,
                    context.source_node,
                    ctx.types,
                ) {
                    inferred_span_types.push(inferred_ty);
                }
            }

            if inferred_span_types.is_empty() {
                continue;
            }

            // bind one union candidate per span so solve keeps literal union precision
            let inferred_span_ty_id = if inferred_span_types.len() == 1 {
                inferred_span_types[0]
            } else {
                ctx.types.insert_type_from_any(
                    Type::Union {
                        elements: inferred_span_types,
                    },
                    context.source_node,
                )
            };
            ctx.infer.push_constraint(Constraint::Equal {
                left: target.infer_ty_id,
                right: inferred_span_ty_id,
            });
        }
    }

    /// Resolve a template span into an inference target when possible.
    fn template_span_inference_target(
        &self,
        ctx: &mut InferContext<'_>,
        span_ty_id: LocalTypeId,
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
    ) -> Option<TemplateSpanInferenceTarget> {
        let (infer_var_id, infer_ty_id) =
            self.template_span_infer_var(&mut ctx.reborrow(), span_ty_id, span_node)?;
        let constraint_id =
            self.template_span_constraint_type(&mut ctx.reborrow(), infer_var_id, source_node);

        Some(TemplateSpanInferenceTarget {
            infer_ty_id,
            constraint_id,
        })
    }

    /// Resolve span inference variables, including static parameter references.
    fn template_span_infer_var(
        &self,
        ctx: &mut InferContext<'_>,
        span_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> Option<(InferVarId, LocalTypeId)> {
        let span_ty = ctx.types.get_type(span_ty_id).clone();
        match span_ty {
            Type::InferVar { id } => Some((id, span_ty_id)),
            Type::Reference { symbol, .. } => {
                if !self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol) {
                    return None;
                }

                if let Some(var_id) = ctx.infer.var_by_symbol_id.get(&symbol).copied()
                    && let Some(ty_id) = ctx.infer.type_for_var(var_id)
                {
                    return Some((var_id, ty_id));
                }

                let scope = InferScope {
                    owner: symbol,
                    function_id: None,
                };
                let ty_id = self.infer_var_type_for_symbol(
                    &mut *ctx.infer,
                    &mut *ctx.types,
                    symbol,
                    source_id,
                    InferOrigin::TypeParameter(symbol),
                    scope,
                );
                let var_id = ctx.infer.var_by_symbol_id.get(&symbol).copied()?;
                Some((var_id, ty_id))
            }
            _ => None,
        }
    }

    /// Resolve the constraint type for an inference variable when possible.
    fn infer_var_constraint_type(
        &self,
        ctx: &mut InferContext<'_>,
        id: InferVarId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        let var = ctx.infer.vars.get(id.0 as usize)?;
        if let InferOrigin::TypeParameter(symbol) = var.origin
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), symbol)
        {
            return self.generic_parameter_constraint_type(
                &mut ctx.type_context_reborrow(),
                symbol,
                source_id,
            );
        }
        None
    }

    /// Resolve template span constraints, skipping unknown or any.
    fn template_span_constraint_type(
        &self,
        ctx: &mut InferContext<'_>,
        infer_var_id: InferVarId,
        source_id: LocalNodeIdAny,
    ) -> Option<LocalTypeId> {
        let constraint_id =
            self.infer_var_constraint_type(&mut ctx.reborrow(), infer_var_id, source_id)?;

        // skip unconstrained spans
        if matches!(
            ctx.types.get_type(constraint_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any
            }
        ) {
            return None;
        }

        Some(constraint_id)
    }
}
