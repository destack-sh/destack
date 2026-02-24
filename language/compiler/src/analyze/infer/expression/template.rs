use std::collections::HashSet;

use super::SignatureResolutionMode;
use crate::{AnalyzeOptions, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Argument, Constraint, Expression, InferOrigin, InferScope, InferTable, InferVarId, LocalNodeId,
    LocalNodeIdAny, LocalTypeId, NodeTree, PrimitiveType, ResolvedSignature, ScalarLiteral,
    StringId, SymbolSpaceOrder, SymbolTable, TemplateLiteral, Type, TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

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
struct TemplateInferenceContext<'a> {
    /// The current module.
    module: &'a Module,
    /// The active profile.
    profile: ProfileId,
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
    /// The symbol table for lookups.
    symbols: &'a SymbolTable,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Report a template literal inference mismatch.
    fn report_template_inference_unassignable(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        param_ty_id: LocalTypeId,
        argument_ty_id: LocalTypeId,
        types: &TypeTable,
    ) {
        self.emit_unassignable_type_for_types(
            module,
            profile,
            argument_id.into_any(),
            param_ty_id,
            argument_ty_id,
            types,
        );
    }

    /// Infer a tagged template expression.
    pub(crate) fn infer_tagged_template_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tag_id: LocalNodeId<Expression>,
        template: &TemplateLiteral,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // infer the tag expression type
        let tag_ty_id = self.infer_expression(module, tag_id, tree, symbols, types, infer, ctx)?;

        // assemble template arguments for call resolution
        let template_strings_ty_id =
            self.template_strings_argument_type(ctx.profile, expression_id.into_any(), types);
        let template_arguments = match template {
            TemplateLiteral::String { .. } => &[][..],
            TemplateLiteral::InterpolatedString { arguments, .. } => arguments.as_slice(),
        };

        // require callable signatures on the tag
        let call_signatures = self.call_signatures_for_type(tag_ty_id, types);
        if call_signatures.is_empty() {
            self.emit_non_callable_for_callee_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                tag_ty_id,
                types,
            );

            for argument_id in template_arguments {
                self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
            }

            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        }

        // prepare callee metadata for overload resolution
        let callee_symbol =
            self.reference_symbol_for_expression(module, tag_id, ctx.profile, tree, symbols);
        let static_arguments = match tree.get(tag_id) {
            Expression::LocalReference {
                static_arguments, ..
            }
            | Expression::ModuleReference {
                static_arguments, ..
            }
            | Expression::GlobalReference {
                static_arguments, ..
            }
            | Expression::Member {
                static_arguments, ..
            } => static_arguments.as_deref(),
            _ => None,
        };

        // resolve the applicable tagged template overload
        let mut resolved_signature = None;
        if call_signatures.len() > 1 {
            // NOTE #Performance: tagged template overload resolution rechecks every signature
            let mut candidates = Vec::new();
            for signature_ty_id in call_signatures.iter() {
                let Some(resolved) = self.resolve_call_signature(
                    module,
                    expression_id,
                    callee_symbol,
                    static_arguments,
                    None,
                    None,
                    None,
                    *signature_ty_id,
                    None,
                    None,
                    SignatureResolutionMode::Synthesize,
                    false,
                    ctx.profile,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?
                else {
                    continue;
                };

                let Some(resolved) = self.slice_tagged_template_signature(
                    module,
                    ctx.profile,
                    template_strings_ty_id,
                    resolved,
                    symbols,
                    types,
                    &ctx.options,
                    true,
                )?
                else {
                    continue;
                };

                if !self.is_signature_applicable(
                    module,
                    ctx.profile,
                    &resolved,
                    template_arguments,
                    tree,
                    symbols,
                    types,
                    infer,
                    &ctx.options,
                )? {
                    continue;
                }

                candidates.push((*signature_ty_id, resolved));
            }

            let mut candidates = self.dedupe_signature_candidates(
                module,
                ctx.profile,
                candidates,
                symbols,
                types,
                &ctx.options,
            );
            if candidates.is_empty() {
                self.emit_no_overload_for_receiver_type(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    tag_ty_id,
                    types,
                );

                for argument_id in template_arguments {
                    self.infer_argument(
                        module,
                        *argument_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                return Ok(types.insert_type_from(ty, expression_id));
            }

            resolved_signature = Some(candidates.remove(0).1);
        } else if let Some(signature_ty_id) = call_signatures.first().copied() {
            let resolved = self.resolve_call_signature(
                module,
                expression_id,
                callee_symbol,
                static_arguments,
                None,
                None,
                None,
                signature_ty_id,
                None,
                None,
                SignatureResolutionMode::Synthesize,
                false,
                ctx.profile,
                &ctx.options,
                tree,
                symbols,
                types,
                infer,
            )?;
            if let Some(resolved) = resolved {
                resolved_signature = self.slice_tagged_template_signature(
                    module,
                    ctx.profile,
                    template_strings_ty_id,
                    resolved,
                    symbols,
                    types,
                    &ctx.options,
                    false,
                )?;
            }
        }

        let Some(resolved) = resolved_signature else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from(ty, expression_id));
        };

        // infer argument types and emit invocation constraints
        let parameter_types = resolved.dynamic_parameters.clone();
        let options = ctx.options;
        let argument_ty_ids = self.infer_invocation_arguments(
            module,
            template_arguments,
            &parameter_types,
            ctx.profile,
            None,
            &options,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

        // emit assignability errors for the tag parameters
        for ((argument_id, argument_ty_id), param_ty_id) in template_arguments
            .iter()
            .zip(argument_ty_ids.iter())
            .zip(parameter_types.iter())
        {
            let argument = tree.get(*argument_id);
            if matches!(argument, Argument::Spread { .. }) {
                continue;
            }

            if self.is_type_assignable(
                module,
                ctx.profile,
                symbols,
                *param_ty_id,
                *argument_ty_id,
                types,
                &ctx.options,
            ) == Assignability::NotAssignable
            {
                self.emit_unassignable_type_for_types(
                    module,
                    ctx.profile,
                    argument.value().into_any(),
                    *param_ty_id,
                    *argument_ty_id,
                    types,
                );
            }
        }

        Ok(resolved.return_type.unwrap_or_else(|| {
            types.insert_type_from(
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
        let name = self.program.strings.intern("TemplateStringsArray");
        if let Some(symbol) =
            self.get_declared_lib_symbol_from(profile, name, SymbolSpaceOrder::TypeThenValue)
        {
            return types.insert_type_from_any(
                Type::Reference {
                    symbol,
                    static_arguments: None,
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
        module: &Module,
        profile: ProfileId,
        template_strings_ty_id: LocalTypeId,
        resolved: ResolvedSignature,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        options: &AnalyzeOptions,
        require_assignable: bool,
    ) -> AnalyzeResult<Option<ResolvedSignature>> {
        // read the template strings parameter when present
        let Some(strings_param_ty_id) = resolved.dynamic_parameters.first().copied() else {
            return Ok(None);
        };

        // ensure instance types are materialized for the template parameter
        self.ensure_reference_instance_types_for_type(
            module,
            profile,
            types.get_type_source(strings_param_ty_id),
            strings_param_ty_id,
            types,
        )?;

        // reject template strings arguments that are not assignable
        if self.is_type_assignable(
            module,
            profile,
            symbols,
            strings_param_ty_id,
            template_strings_ty_id,
            types,
            options,
        ) == Assignability::NotAssignable
        {
            if require_assignable {
                return Ok(None);
            }

            self.emit_unassignable_type_for_types(
                module,
                profile,
                types.get_type_source(strings_param_ty_id),
                strings_param_ty_id,
                template_strings_ty_id,
                types,
            );
        }

        // drop the template strings parameter and normalize the signature
        let parameters = resolved
            .dynamic_parameters
            .iter()
            .skip(1)
            .copied()
            .collect();
        Ok(Some(ResolvedSignature {
            dynamic_parameters: parameters,
            return_type: resolved.return_type,
            static_arguments: resolved.static_arguments,
        }))
    }

    /// Infer template spans from a string literal argument.
    fn infer_template_literal_from_string_argument(
        &self,
        context: &TemplateInferenceContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        value: &str,
        types: &mut TypeTable,
        infer: &mut InferTable,
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
        let mut visited = HashSet::new();
        for (span_ty_id, span_value) in spans.iter().zip(span_values.iter()) {
            if !self.infer_template_span_from_string_value(
                context,
                *span_ty_id,
                span_value,
                types,
                infer,
                &mut visited,
            ) {
                break;
            }
        }
    }

    /// Infer template spans from a template literal argument.
    fn infer_template_literal_from_template_argument(
        &self,
        context: &TemplateInferenceContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        argument_strings: &[StringId],
        argument_spans: &[LocalTypeId],
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) {
        // handle `${infer}` templates that capture the entire argument
        if self.infer_template_literal_full_span(context, strings, spans, types, infer, options) {
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
        for (span_ty_id, argument_span) in spans.iter().zip(argument_spans.iter()) {
            self.infer_template_span_from_template_argument(
                context,
                *span_ty_id,
                *argument_span,
                types,
                infer,
                options,
            );
        }
    }

    /// Infer a single template span from a string literal value.
    fn infer_template_span_from_string_value(
        &self,
        context: &TemplateInferenceContext<'_>,
        span_ty_id: LocalTypeId,
        span_value: &str,
        types: &mut TypeTable,
        infer: &mut InferTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            context.module,
            context.profile,
            span_ty_id,
            context.span_node,
            context.source_node,
            context.symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return self.template_span_matches_string(
                context.module,
                context.profile,
                span_ty_id,
                span_value,
                context.symbols,
                types,
                visited,
            );
        };

        // reject spans that violate the constraint
        if let Some(constraint_id) = target.constraint_id
            && !self.template_span_matches_string(
                context.module,
                context.profile,
                constraint_id,
                span_value,
                context.symbols,
                types,
                visited,
            )
        {
            self.report_template_inference_unassignable(
                context.module,
                context.profile,
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
                types,
            );
            return false;
        }

        // infer a literal type when possible
        let inferred_ty = self.template_infer_literal_type(
            target.constraint_id,
            span_value,
            context.source_node,
            types,
        );
        if let Some(inferred_ty) = inferred_ty {
            infer.push_constraint(Constraint::Equal {
                left: target.infer_ty_id,
                right: inferred_ty,
            });
        }

        true
    }

    /// Infer a single template span from a template literal argument span.
    fn infer_template_span_from_template_argument(
        &self,
        context: &TemplateInferenceContext<'_>,
        span_ty_id: LocalTypeId,
        argument_span: LocalTypeId,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            context.module,
            context.profile,
            span_ty_id,
            context.span_node,
            context.source_node,
            context.symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return;
        };

        // validate argument spans against constraints
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                context.module,
                context.profile,
                context.symbols,
                constraint_id,
                argument_span,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                context.module,
                context.profile,
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
                types,
            );
            return;
        }

        // record inference bindings for the span
        infer.push_constraint(Constraint::Equal {
            left: target.infer_ty_id,
            right: argument_span,
        });
    }

    /// Infer the full template literal into a single span.
    fn infer_template_literal_full_span(
        &self,
        context: &TemplateInferenceContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) -> bool {
        // require a single empty span template
        if !strings
            .iter()
            .all(|string_id| self.program.strings.get(*string_id).is_empty())
            || spans.len() != 1
        {
            return false;
        }

        // resolve the span inference target
        let target = self.template_span_inference_target(
            context.module,
            context.profile,
            spans[0],
            context.span_node,
            context.source_node,
            context.symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return true;
        };

        // validate the constraint against the argument
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                context.module,
                context.profile,
                context.symbols,
                constraint_id,
                context.argument_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                context.module,
                context.profile,
                context.argument_id,
                context.param_ty_id,
                context.argument_ty_id,
                types,
            );
            return true;
        }

        // bind the inference variable to the full argument
        infer.push_constraint(Constraint::Equal {
            left: target.infer_ty_id,
            right: context.argument_ty_id,
        });

        true
    }

    /// Add inference constraints for template literal parameters.
    pub(crate) fn add_template_literal_inference_constraints(
        &self,
        module: &Module,
        profile: ProfileId,
        dynamic_arguments: &[LocalNodeId<Argument>],
        argument_ty_ids: &[LocalTypeId],
        parameter_types: &[LocalTypeId],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) {
        // scan argument pairs for template literal inference
        for ((argument_id, argument_ty_id), param_ty_id) in dynamic_arguments
            .iter()
            .zip(argument_ty_ids.iter())
            .zip(parameter_types.iter())
        {
            let (strings, spans) = match types.get_type(*param_ty_id) {
                Type::TemplateLiteral { strings, spans } => (strings.clone(), spans.clone()),
                _ => continue,
            };

            // collect argument context
            let argument_value_id = tree.get(*argument_id).value();
            let source_node = argument_value_id.into_any();
            let span_node = argument_id.into_any();
            let argument_ty = types.get_type(*argument_ty_id).clone();
            let context = TemplateInferenceContext {
                module,
                profile,
                argument_id: *argument_id,
                argument_ty_id: *argument_ty_id,
                param_ty_id: *param_ty_id,
                span_node,
                source_node,
                symbols,
            };

            // infer from string or template literal arguments
            match argument_ty {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                } => {
                    let value = self.program.strings.get(string_id).to_string();
                    self.infer_template_literal_from_string_argument(
                        &context, &strings, &spans, &value, types, infer,
                    );
                }
                Type::Union { elements } => {
                    self.infer_template_from_union_string_argument(
                        &context, &strings, &spans, &elements, types, infer,
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
                        types,
                        infer,
                        options,
                    );
                }
                _ => {}
            }
        }
    }

    /// Infer template spans from a union of string literal arguments.
    fn infer_template_from_union_string_argument(
        &self,
        context: &TemplateInferenceContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        elements: &[LocalTypeId],
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) {
        // collect span values for each union member when all members are string literals
        let mut union_span_values = Vec::with_capacity(elements.len());
        for element_ty_id in elements {
            let Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
            } = types.get_type(*element_ty_id)
            else {
                return;
            };

            let value = self.program.strings.get(*string_id).to_string();
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
                context.module,
                context.profile,
                *span_ty_id,
                context.span_node,
                context.source_node,
                context.symbols,
                types,
                infer,
            ) else {
                continue;
            };

            let mut inferred_span_types = Vec::new();
            for span_values in &union_span_values {
                let span_value = &span_values[span_index];

                if let Some(constraint_id) = target.constraint_id {
                    let mut visited = HashSet::new();
                    if !self.template_span_matches_string(
                        context.module,
                        context.profile,
                        constraint_id,
                        span_value,
                        context.symbols,
                        types,
                        &mut visited,
                    ) {
                        self.report_template_inference_unassignable(
                            context.module,
                            context.profile,
                            context.argument_id,
                            context.param_ty_id,
                            context.argument_ty_id,
                            types,
                        );
                        return;
                    }
                }

                if let Some(inferred_ty) = self.template_infer_literal_type(
                    target.constraint_id,
                    span_value,
                    context.source_node,
                    types,
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
                types.insert_type_from_any(
                    Type::Union {
                        elements: inferred_span_types,
                    },
                    context.source_node,
                )
            };
            infer.push_constraint(Constraint::Equal {
                left: target.infer_ty_id,
                right: inferred_span_ty_id,
            });
        }
    }

    /// Resolve a template span into an inference target when possible.
    fn template_span_inference_target(
        &self,
        module: &Module,
        profile: ProfileId,
        span_ty_id: LocalTypeId,
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> Option<TemplateSpanInferenceTarget> {
        let (infer_var_id, infer_ty_id) = self.template_span_infer_var(
            module, profile, span_ty_id, span_node, symbols, types, infer,
        )?;
        let constraint_id = self.template_span_constraint_type(
            module,
            profile,
            infer_var_id,
            source_node,
            symbols,
            types,
            infer,
        );

        Some(TemplateSpanInferenceTarget {
            infer_ty_id,
            constraint_id,
        })
    }

    /// Resolve span inference variables, including static parameter references.
    fn template_span_infer_var(
        &self,
        module: &Module,
        profile: ProfileId,
        span_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
    ) -> Option<(InferVarId, LocalTypeId)> {
        let span_ty = types.get_type(span_ty_id).clone();
        match span_ty {
            Type::InferVar { id } => Some((id, span_ty_id)),
            Type::Reference { symbol, .. } => {
                if !self.symbol_is_static_parameter(module, profile, symbol, symbols, types) {
                    return None;
                }

                if let Some(var_id) = infer.var_by_symbol_id.get(&symbol).copied()
                    && let Some(ty_id) = infer.type_for_var(var_id)
                {
                    return Some((var_id, ty_id));
                }

                let scope = InferScope {
                    owner: symbol,
                    function_id: None,
                };
                let ty_id = self.infer_var_type_for_symbol(
                    infer,
                    types,
                    symbol,
                    source_id,
                    InferOrigin::TypeParameter(symbol),
                    scope,
                );
                let var_id = infer.var_by_symbol_id.get(&symbol).copied()?;
                Some((var_id, ty_id))
            }
            _ => None,
        }
    }

    /// Resolve the constraint type for an inference variable when possible.
    fn infer_var_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        id: InferVarId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
    ) -> Option<LocalTypeId> {
        let var = infer.vars.get(id.0 as usize)?;
        if let InferOrigin::TypeParameter(symbol) = var.origin
            && self.symbol_is_static_parameter(module, profile, symbol, symbols, types)
        {
            return self.static_parameter_constraint_type(
                module, profile, symbol, source_id, symbols, types,
            );
        }
        None
    }

    /// Resolve template span constraints, skipping unknown or any.
    fn template_span_constraint_type(
        &self,
        module: &Module,
        profile: ProfileId,
        infer_var_id: InferVarId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
    ) -> Option<LocalTypeId> {
        let constraint_id = self.infer_var_constraint_type(
            module,
            profile,
            infer_var_id,
            source_id,
            symbols,
            types,
            infer,
        )?;

        // skip unconstrained spans
        if matches!(
            types.get_type(constraint_id),
            Type::TypeLiteral {
                value: TypeLiteral::Unknown | TypeLiteral::Any
            }
        ) {
            return None;
        }

        Some(constraint_id)
    }
}
