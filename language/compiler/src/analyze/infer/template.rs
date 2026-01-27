use std::collections::HashSet;

use crate::{AnalyzeError, AnalyzeOptions, Assignability, Compiler};
use destack_dir::{
    Argument, Constraint, InferOrigin, InferScope, InferTable, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, NodeTree, ScalarLiteral, StringId, SymbolTable, Type, TypeLiteral, TypeTable,
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
    ) {
        self.error(AnalyzeError::UnassignableType {
            node: argument_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            expected_ty: param_ty_id.into_global(module.id),
            actual_ty: argument_ty_id.into_global(module.id),
        });
    }

    /// Infer template spans from a string literal argument.
    fn infer_template_literal_from_string_argument(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        param_ty_id: LocalTypeId,
        strings: &[StringId],
        spans: &[LocalTypeId],
        value: &str,
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
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
                module,
                profile,
                argument_id,
                argument_ty_id,
                param_ty_id,
                *span_ty_id,
                span_value,
                span_node,
                source_node,
                symbols,
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
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        param_ty_id: LocalTypeId,
        strings: &[StringId],
        spans: &[LocalTypeId],
        argument_strings: &[StringId],
        argument_spans: &[LocalTypeId],
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) {
        // handle `${infer}` templates that capture the entire argument
        if self.infer_template_literal_full_span(
            module,
            profile,
            argument_id,
            argument_ty_id,
            param_ty_id,
            strings,
            spans,
            span_node,
            source_node,
            symbols,
            types,
            infer,
            options,
        ) {
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
                module,
                profile,
                argument_id,
                argument_ty_id,
                param_ty_id,
                *span_ty_id,
                *argument_span,
                span_node,
                source_node,
                symbols,
                types,
                infer,
                options,
            );
        }
    }

    /// Infer a single template span from a string literal value.
    fn infer_template_span_from_string_value(
        &self,
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        param_ty_id: LocalTypeId,
        span_ty_id: LocalTypeId,
        span_value: &str,
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            module,
            profile,
            span_ty_id,
            span_node,
            source_node,
            symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return self.template_span_matches_string(
                module, profile, span_ty_id, span_value, symbols, types, visited,
            );
        };

        // reject spans that violate the constraint
        if let Some(constraint_id) = target.constraint_id
            && !self.template_span_matches_string(
                module,
                profile,
                constraint_id,
                span_value,
                symbols,
                types,
                visited,
            )
        {
            self.report_template_inference_unassignable(
                module,
                profile,
                argument_id,
                param_ty_id,
                argument_ty_id,
            );
            return false;
        }

        // infer a literal type when possible
        let inferred_ty =
            self.template_infer_literal_type(target.constraint_id, span_value, source_node, types);
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
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        param_ty_id: LocalTypeId,
        span_ty_id: LocalTypeId,
        argument_span: LocalTypeId,
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) {
        // resolve the span inference target when possible
        let target = self.template_span_inference_target(
            module,
            profile,
            span_ty_id,
            span_node,
            source_node,
            symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return;
        };

        // validate argument spans against constraints
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                constraint_id,
                argument_span,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                module,
                profile,
                argument_id,
                param_ty_id,
                argument_ty_id,
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
        module: &Module,
        profile: ProfileId,
        argument_id: LocalNodeId<Argument>,
        argument_ty_id: LocalTypeId,
        param_ty_id: LocalTypeId,
        strings: &[StringId],
        spans: &[LocalTypeId],
        span_node: LocalNodeIdAny,
        source_node: LocalNodeIdAny,
        symbols: &SymbolTable,
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
            module,
            profile,
            spans[0],
            span_node,
            source_node,
            symbols,
            types,
            infer,
        );
        let Some(target) = target else {
            return true;
        };

        // validate the constraint against the argument
        if let Some(constraint_id) = target.constraint_id
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                constraint_id,
                argument_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            self.report_template_inference_unassignable(
                module,
                profile,
                argument_id,
                param_ty_id,
                argument_ty_id,
            );
            return true;
        }

        // bind the inference variable to the full argument
        infer.push_constraint(Constraint::Equal {
            left: target.infer_ty_id,
            right: argument_ty_id,
        });

        true
    }

    /// Add inference constraints for template literal parameters.
    pub(super) fn add_template_literal_inference_constraints(
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

            // infer from string or template literal arguments
            match argument_ty {
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(string_id)),
                } => {
                    let value = self.program.strings.get(string_id).to_string();
                    self.infer_template_literal_from_string_argument(
                        module,
                        profile,
                        *argument_id,
                        *argument_ty_id,
                        *param_ty_id,
                        &strings,
                        &spans,
                        &value,
                        span_node,
                        source_node,
                        symbols,
                        types,
                        infer,
                    );
                }
                Type::TemplateLiteral {
                    strings: argument_strings,
                    spans: argument_spans,
                } => {
                    self.infer_template_literal_from_template_argument(
                        module,
                        profile,
                        *argument_id,
                        *argument_ty_id,
                        *param_ty_id,
                        &strings,
                        &spans,
                        &argument_strings,
                        &argument_spans,
                        span_node,
                        source_node,
                        symbols,
                        types,
                        infer,
                        options,
                    );
                }
                _ => {}
            }
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
    ) -> Option<(destack_dir::InferVarId, LocalTypeId)> {
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
        id: destack_dir::InferVarId,
        source_id: LocalNodeIdAny,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &InferTable,
    ) -> Option<LocalTypeId> {
        let var = infer.vars.get(id.0 as usize)?;
        if let InferOrigin::TypeParameter(symbol) = var.origin {
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
        infer_var_id: destack_dir::InferVarId,
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
