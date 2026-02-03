use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::declaration::DeclaratorConstraint;
use super::member::MemberLookupMode;

use crate::analyze::common::{
    CanonicalSymbolMode, ConstContext, ContextualTypingMode, LiteralFreshness, RelationMode,
    WideningMode,
};
use crate::timing::tags;
use crate::{
    AnalyzeError, AnalyzeOptions, AnalyzeResult, AnalyzeWarning, Assignability, BreakTargetKind,
    Compiler, FlowContext, InferContext,
};
use destack_builtin::LanguageSymbol;
use destack_dir::{
    Addressability, Argument, BindingKind, BindingOperator, Block, CastOperator, CastSource,
    Constraint, Declaration, DependencyItem, DependencyKind, DependencyMode, DependencySource,
    DynamicKey, Expression, FlowGraphBuilder, ForEachBinding, FunctionCardinality, GlobalNodeIdAny,
    GlobalSymbolId, IfCondition, InferOrigin, InferScope, InferTable, LocalNodeId, LocalNodeIdAny,
    LocalSymbolId, LocalTypeId, LoopKind, MatchCase, MatchKind, MatchSelector, MatchSource, Member,
    Mutability, NodeTree, NodeType, NormalizationMode, Pattern, PatternField, PrimitiveType,
    Property, Resolution, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StringId,
    SymbolDecorators, SymbolSpace, SymbolTable, SymbolType, Type, TypeBinaryOperator, TypeElement,
    TypeField, TypeKind, TypeLiteral, TypeTable, TypeUnaryOperator, WellKnownSymbol,
    YieldCardinality,
};
use destack_source::ModuleId;
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Object literal field metadata for excess property checks.
#[derive(Debug, Clone)]
pub(super) struct ObjectLiteralField {
    /// The field of the object literal.
    field: TypeField,
    /// The corresponding property of the object literal.
    property_id: LocalNodeId<Property>,
}

impl ObjectLiteralField {
    /// Return the object literal field type information.
    pub(super) fn field(&self) -> &TypeField {
        &self.field
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Recover static parameters for a signature when the type omitted them.
    fn recover_static_parameters_for_signature(
        &self,
        module: &Module,
        signature_ty_id: LocalTypeId,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Vec<LocalTypeId> {
        // read the type source for the signature
        let source_id = types.get_type_source(signature_ty_id);

        // check member signatures first
        if let Ok(member_id) = source_id.try_into_typed::<Member>() {
            let member = tree.get(member_id);
            if let Member::Method { signature, .. } = member
                && signature.generics.as_ref().is_some()
            {
                let placeholders = self
                    .static_parameter_placeholders_for_signature(module, signature, tree, types);
                self.set_static_parameters_for_signature_type(
                    signature_ty_id,
                    &placeholders,
                    types,
                );
                return placeholders;
            }
        }

        // check function declarations
        if let Ok(declaration_id) = source_id.try_into_typed::<Declaration>() {
            let declaration = tree.get(declaration_id);
            if let Declaration::Function { signature, .. } = declaration
                && signature.generics.as_ref().is_some()
            {
                let placeholders = self
                    .static_parameter_placeholders_for_signature(module, signature, tree, types);
                self.set_static_parameters_for_signature_type(
                    signature_ty_id,
                    &placeholders,
                    types,
                );
                return placeholders;
            }
        }

        Vec::new()
    }

    /// Store recovered static parameters for a signature type when missing.
    fn set_static_parameters_for_signature_type(
        &self,
        signature_ty_id: LocalTypeId,
        placeholders: &[LocalTypeId],
        types: &mut TypeTable,
    ) {
        if placeholders.is_empty() {
            return;
        }

        let Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(signature_ty_id).clone()
        else {
            return;
        };

        if !static_parameters.is_empty() {
            return;
        }

        let updated = Type::Function {
            asynchrony,
            cardinality,
            static_parameters: placeholders.to_vec(),
            this_parameter,
            dynamic_parameters,
            return_type,
        };
        types.update_type(signature_ty_id, updated);
    }

    /// Decide whether a scalar literal should be widened in this context.
    pub(super) fn should_widen_scalar_literal(&self, ctx: &InferContext) -> bool {
        if !matches!(ctx.widening_mode, WideningMode::Widen) {
            return false;
        }
        if !matches!(ctx.literal_freshness, LiteralFreshness::Regularized) {
            return false;
        }
        matches!(ctx.const_context, ConstContext::None)
    }

    /// Widen a scalar literal type when the context requires it.
    pub(super) fn widen_scalar_literal_type_if_needed(
        &self,
        module: &Module,
        types: &mut TypeTable,
        type_id: LocalTypeId,
        ctx: &InferContext,
        source_id: LocalNodeIdAny,
    ) -> LocalTypeId {
        if !self.should_widen_scalar_literal(ctx) {
            return type_id;
        }

        let Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(literal),
        } = types.get_type(type_id)
        else {
            return type_id;
        };

        let widened = Type::TypeLiteral {
            value: self.widen_scalar_literal_for_module(module, literal),
        };
        types.insert_type_from_any(widened, source_id)
    }

    /// Resolve the element type produced by spreading a value in an array literal.
    pub(super) fn array_spread_element_type(
        &self,
        module: &Module,
        profile: ProfileId,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // apparent types determine array literal spread shapes
        let type_id = self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );
        match types.get_type(type_id) {
            Type::Array { element, .. } => *element,
            Type::ArraySized { element, .. } => Some(*element),
            Type::Tuple { elements, .. } => {
                let elements = elements.clone();
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for element in elements {
                    let mut element_ty_id = element.ty;
                    if element.is_rest
                        && let Some(rest_element_ty_id) = self.array_spread_element_type(
                            module,
                            profile,
                            element_ty_id,
                            symbols,
                            types,
                        )
                    {
                        element_ty_id = rest_element_ty_id;
                    }
                    element_type_ids.push(element_ty_id);
                }
                if element_type_ids.is_empty() {
                    None
                } else {
                    let source_type_id = element_type_ids[0];
                    Some(self.union_type_from_list(element_type_ids, source_type_id, types))
                }
            }
            Type::Union { elements } => {
                let elements = elements.clone();
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element_type_id = self
                        .array_spread_element_type(module, profile, element_id, symbols, types)?;
                    element_type_ids.push(element_type_id);
                }
                if element_type_ids.is_empty() {
                    None
                } else {
                    let source_type_id = element_type_ids[0];
                    Some(self.union_type_from_list(element_type_ids, source_type_id, types))
                }
            }
            _ => None,
        }
    }

    /// Select the best common type for a pair of branch results.
    fn best_common_type_for_pair(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
        source_id: LocalNodeIdAny,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        expected_type: Option<LocalTypeId>,
        allow_widening: bool,
    ) -> LocalTypeId {
        let mut left_ty_id = left_ty_id;
        let mut right_ty_id = right_ty_id;

        // widen scalar literals when no contextual type is forcing a shape
        if allow_widening {
            left_ty_id =
                self.widen_scalar_literal_type_if_needed(module, types, left_ty_id, ctx, source_id);
            right_ty_id = self.widen_scalar_literal_type_if_needed(
                module,
                types,
                right_ty_id,
                ctx,
                source_id,
            );
        }

        // prefer the contextual type when both branches satisfy it
        if let Some(expected_ty_id) = expected_type {
            let left_assignable = self.is_type_assignable(
                module,
                ctx.profile,
                symbols,
                expected_ty_id,
                left_ty_id,
                types,
                &ctx.options,
            );
            let right_assignable = self.is_type_assignable(
                module,
                ctx.profile,
                symbols,
                expected_ty_id,
                right_ty_id,
                types,
                &ctx.options,
            );
            if left_assignable.is_assignable() && right_assignable.is_assignable() {
                return expected_ty_id;
            }
        }

        // prefer a common supertype when one branch subsumes the other
        let left_to_right = self.is_type_assignable(
            module,
            ctx.profile,
            symbols,
            right_ty_id,
            left_ty_id,
            types,
            &ctx.options,
        );
        if left_to_right.is_assignable() {
            return right_ty_id;
        }
        let right_to_left = self.is_type_assignable(
            module,
            ctx.profile,
            symbols,
            left_ty_id,
            right_ty_id,
            types,
            &ctx.options,
        );
        if right_to_left.is_assignable() {
            return left_ty_id;
        }

        // widen numeric branches to a shared numeric type when allowed
        let left_ty = types.get_type(left_ty_id).clone();
        let right_ty = types.get_type(right_ty_id).clone();
        let left_is_numeric = self.is_numeric_like_type(&left_ty, types);
        let right_is_numeric = self.is_numeric_like_type(&right_ty, types);
        if left_is_numeric && right_is_numeric {
            let left_is_literal = matches!(
                left_ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Float(_)
                            | ScalarLiteral::Bigint(_),
                    ),
                }
            );
            let right_is_literal = matches!(
                right_ty,
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Float(_)
                            | ScalarLiteral::Bigint(_),
                    ),
                }
            );
            let allow_numeric_widening = allow_widening
                && (!left_is_literal || !right_is_literal || self.should_widen_scalar_literal(ctx));
            if allow_numeric_widening {
                let widened = self.widen_numeric_types(&left_ty, &right_ty);
                return types.insert_type_from_any(widened, source_id);
            }
        }

        self.union_type(left_ty_id, right_ty_id, types)
    }

    /// Select the best common type for a list of branch results.
    fn best_common_type_for_list(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
        source_id: LocalNodeIdAny,
        candidates: &[LocalTypeId],
    ) -> LocalTypeId {
        let Some((&first, rest)) = candidates.split_first() else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Never,
            };
            return types.insert_type_from_any(ty, source_id);
        };

        // allow the contextual type only when all candidates satisfy it
        let expected_type = ctx.expected_type.filter(|expected_ty_id| {
            candidates.iter().all(|candidate| {
                self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    *expected_ty_id,
                    *candidate,
                    types,
                    &ctx.options,
                )
                .is_assignable()
            })
        });
        let allow_widening = expected_type.is_none();

        rest.iter().fold(first, |current, next| {
            self.best_common_type_for_pair(
                module,
                symbols,
                types,
                ctx,
                source_id,
                current,
                *next,
                expected_type,
                allow_widening,
            )
        })
    }

    /// Resolve the result type for a loop expression from collected break values.
    fn loop_break_result_type(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
        source_id: LocalNodeIdAny,
        break_values: &[LocalTypeId],
        expected_type: Option<LocalTypeId>,
    ) -> LocalTypeId {
        let Some((&first, rest)) = break_values.split_first() else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            return types.insert_type_from_any(ty, source_id);
        };

        let allow_widening = expected_type.is_none();
        rest.iter().fold(first, |current, next| {
            self.best_common_type_for_pair(
                module,
                symbols,
                types,
                ctx,
                source_id,
                current,
                *next,
                expected_type,
                allow_widening,
            )
        })
    }

    /// Resolve the yield and return types for a yield* delegate value.
    fn yield_star_delegate_types(
        &self,
        module: &Module,
        profile: ProfileId,
        value_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(LocalTypeId, Option<LocalTypeId>)> {
        if let Some(element_ty_id) =
            self.array_spread_element_type(module, profile, value_ty_id, symbols, types)
        {
            return Some((element_ty_id, None));
        }

        if let Some((yield_ty_id, return_ty_id, _next_ty_id)) =
            self.generator_type_arguments(module, profile, value_ty_id, symbols, types)
        {
            return Some((yield_ty_id, Some(return_ty_id)));
        }

        let (symbol, static_arguments) = {
            let Type::Reference {
                symbol,
                static_arguments,
            } = types.get_type(value_ty_id)
            else {
                return None;
            };

            (*symbol, static_arguments.clone())
        };
        let source_id = types.get_type_source(value_ty_id);

        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            profile,
            symbol,
            CanonicalSymbolMode::FollowAliases,
        );
        let iterable_symbol =
            self.get_well_known_type_symbol(profile, WellKnownSymbol::Iterable)?;
        if canonical_symbol != iterable_symbol {
            return None;
        }

        let unknown_ty_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            },
            source_id,
        );
        let yield_ty_id = static_arguments
            .as_ref()
            .and_then(|arguments| arguments.first())
            .map(|argument| self.convert_static_argument_type(argument, source_id, types))
            .unwrap_or(unknown_ty_id);

        Some((yield_ty_id, None))
    }

    /// Commit a flow join result using best-common-type rules.
    fn flow_join_type(
        &self,
        module: &Module,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
        source_id: LocalNodeIdAny,
        candidates: &[LocalTypeId],
    ) -> LocalTypeId {
        self.best_common_type_for_list(module, symbols, types, ctx, source_id, candidates)
    }

    /// Resolve a type expression or fall back to inference when unevaluated.
    fn resolve_type_expression(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // evaluate the type expression when possible
        let mut ty_id = self.try_evaluate_expression_to_type(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
        )?;

        // fall back to inference for unevaluated types
        if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
            ty_id =
                self.infer_expression(module, expression_id, tree, symbols, types, infer, ctx)?;
        }

        Ok(ty_id)
    }

    /// Infer a direct binding value type when none is cached yet.
    pub(crate) fn infer_direct_binding_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // only infer local bindings without cached value types
        if symbol.module_id != module.id || types.get_value_type_id(symbol).is_some() {
            return Ok(None);
        }

        // resolve the declarator for the binding
        let Some(declarator_id) =
            self.direct_binding_declarator_for_symbol(module, symbol, tree, symbols)
        else {
            return Ok(None);
        };

        // reuse declared types when present
        let declared_ty_id = types.get_declared_type_id(declarator_id.into_global_any(module.id));
        if let Some(declared_ty_id) = declared_ty_id {
            self.evaluate_type(module, profile, declared_ty_id, tree, symbols, types)?;
            types.set_value_type(symbol, declared_ty_id);
            return Ok(Some(declared_ty_id));
        }

        // infer from the initializer when available
        let declarator = tree.get(declarator_id);
        let Some(value_id) = declarator.value else {
            return Ok(None);
        };

        // seed a placeholder to avoid recursion through self references
        let scope = InferScope {
            owner: symbol,
            function_id: ctx
                .in_function
                .map(|function_id| function_id.into_global(module.id)),
        };
        let placeholder_ty_id = self.infer_var_type_for_symbol(
            infer,
            types,
            symbol,
            value_id.into_any(),
            InferOrigin::Expression(value_id.into_global_any(module.id)),
            scope,
        );
        types.set_value_type(symbol, placeholder_ty_id);

        // infer the initializer with binding defaults
        let mut value_ctx = ctx
            .reset()
            .with_expected_type(None)
            .with_contextual_typing_mode(ContextualTypingMode::Default);
        let binding_mutability = symbols.get_symbol(symbol.local_id).binding_mutability;
        if let Some(mutability) = binding_mutability {
            value_ctx = value_ctx.with_binding_mutability(mutability);
        } else {
            value_ctx = value_ctx.with_binding_initializer_defaults();
        }
        let inferred_ty_id = self.infer_expression(
            module,
            value_id,
            tree,
            symbols,
            types,
            infer,
            &mut value_ctx,
        )?;

        // commit the binding type before caching it
        let is_const_asserted = self.declarator_is_const_assertion(declarator_id, tree);
        let committed_ty_id =
            self.commit_binding_type(module, &value_ctx, inferred_ty_id, types, is_const_asserted);
        types.set_value_type(symbol, committed_ty_id);

        Ok(Some(committed_ty_id))
    }

    /// Infer an (expression) body with flow aware typing.
    pub fn infer_body(
        &self,
        module: &Module,
        body_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        context: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // build a flow graph and flow table for the function body when needed
        let previous_flow = context.flow.clone();
        if self.expression_requires_flow(tree, body_id) {
            let graph = FlowGraphBuilder::new(module.id, tree).build(body_id);
            let flow = self.compute_flow_table_for_graph(
                module, &graph, tree, symbols, types, infer, context,
            )?;

            // seed the inference context with flow information
            context.flow = Some(FlowContext {
                module_id: module.id,
                graph: Arc::new(graph),
                table: Arc::new(flow),
            });
        } else {
            context.flow = None;
        }

        // infer the expression using the flow context
        let result =
            self.infer_expression(module, body_id, tree, symbols, types, infer, context)?;

        // restore the previous flow context
        context.flow = previous_flow;

        Ok(result)
    }

    destack_base::ensure_sufficient_stack! {
    /// Infer the type of an expression.
    pub(crate) fn infer_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // reuse an inferred result when caching is enabled
        let has_flow = ctx.flow.is_some();
        let node_id = expression_id.into_global_any(module.id);
        if !ctx.is_surface_inference
            && !has_flow
            && let Some(ty_id) = types.get_inferred_type_id(node_id)
        {
            self.ensure_expression_addressability(module, expression_id, tree, types);
            return Ok(ty_id);
        }

        // resolve the flow environment for the node when flow typing is active
        let flow_environment = ctx.flow.as_ref().and_then(|flow_context| {
            if flow_context.module_id != module.id {
                // #Suspicious: flow context belongs to a different module (error?)
                return None;
            }
            let block_id = flow_context
                .graph
                .block_by_node
                .get(&expression_id.into_any())
                .copied()?;
            let environment_id = flow_context.table.entry_environment_for_block(block_id)?;
            flow_context.table.environment(environment_id).cloned()
        });
        if let Some(environment) = flow_environment {
            self.apply_flow_environment_to_context(&environment, ctx);
        }

        let expression = tree.get(expression_id);
        let ty_id: LocalTypeId = match expression {
            // declaration: analyze the declaration
            Expression::Declaration { declaration } => {
                if !self.declaration_requires_infer(module, *declaration, tree) {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                }

                self.infer_declaration(module, *declaration, tree, symbols, types, infer, ctx)?;
                let declaration = tree.get(*declaration);

                // function declarations used as expressions evaluate to function values
                if let Declaration::Function { descriptor, .. } = declaration {
                    if let Some(value_ty_id) =
                        types.get_value_type_id(descriptor.symbol.into_global(module.id))
                    {
                        value_ty_id
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        };
                        types.insert_type_from(ty, expression_id)
                    }
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // block: analyze the block
            Expression::Block { block } => {
                self.infer_block(module, *block, tree, symbols, types, infer, ctx)?
            }

            // statement: analyze the statement
            Expression::Statement { statement } => {
                self.infer_expression(module, *statement, tree, symbols, types, infer, ctx)?;
                self.warn_ignored_return_value(
                    module,
                    ctx.profile,
                    *statement,
                    tree,
                    symbols,
                    types,
                );

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // labelled statement: analyze the body with label in context
            Expression::Labelled {
                label: _,
                body: body_id,
                symbol: _,
            } => {
                self.infer_expression(module, *body_id, tree, symbols, types, infer, ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // import / exports
            Expression::Import {
                kind: _,
                source,
                target: _,
                target_module: _,
                items,
                arguments,
            }
            | Expression::UnresolvedImport {
                kind: _,
                source,
                target: _,
                items,
                arguments,
                ..
            } => {
                // reject dynamic imports when configured
                if ctx.options.no_dynamic_import
                    && matches!(module.source, ModuleSource::User)
                    && matches!(
                        source,
                        DependencySource::ImportCall | DependencySource::RequireCall
                    )
                {
                    self.error(AnalyzeError::DynamicImportDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                for item_id in items {
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, infer, ctx)?;
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.infer_argument(module, *argument_id, None, tree, symbols, types, infer, ctx)?;
                    }
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::ReExport {
                target: _,
                target_module: _,
                kind: _,
                items,
            }
            | Expression::Export { kind: _, items }
            | Expression::UnresolvedReExport {
                target: _,
                kind: _,
                items,
            } => {
                for item_id in items {
                    self.infer_dependency_item(module, *item_id, tree, symbols, types, infer, ctx)?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::ExportNamespace { name: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // let
            Expression::Let {
                descriptor: _,
                mutability,
                declarators,
            } => {
                for decl_id in declarators {
                    let mut decl_ctx = ctx.fork().with_binding_mutability(*mutability);
                    self.infer_declarator(
                        module,
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut decl_ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            // using
            Expression::Using {
                asynchrony: _,
                descriptor: _,
                declarators,
            } => {
                for decl_id in declarators {
                    let mut decl_ctx = ctx.fork().with_using_binding();
                    self.infer_declarator(
                        module,
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut decl_ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // type operations
            Expression::TypeUnary { operator, right } => {
                let mut right_ctx = if matches!(operator, TypeUnaryOperator::AsConst) {
                    ctx.fork().with_const_assertion_context()
                } else {
                    ctx.fork()
                };
                let right_ty_id = self.infer_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut right_ctx,
                )?;

                let ty = self.infer_type_unary_operation(
                    module,
                    ctx.profile,
                    expression_id,
                    operator,
                    right_ty_id,
                    symbols,
                    types,
                );
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let (left_ty_id, right_ty_id) = match operator {
                    TypeBinaryOperator::Extends | TypeBinaryOperator::Implements => {
                        let left_ty_id = self.try_evaluate_expression_to_type(
                            module,
                            ctx.profile,
                            *left,
                            tree,
                            symbols,
                            types,
                            true,
                            true,
                        )?;
                        let right_ty_id = self.resolve_type_expression(
                            module,
                            ctx.profile,
                            *right,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        (left_ty_id, right_ty_id)
                    }
                    TypeBinaryOperator::Satisfies => {
                        let right_ty_id = self.resolve_type_expression(
                            module,
                            ctx.profile,
                            *right,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        let mut left_ctx = ctx
                            .fork()
                            .with_expected_type(Some(right_ty_id))
                            .without_const_context()
                            .with_contextual_typing_mode(ContextualTypingMode::Satisfies);
                        let left_ty_id =
                            self.infer_expression(module, *left, tree, symbols, types, infer, &mut left_ctx)?;
                        (left_ty_id, right_ty_id)
                    }
                    _ => {
                        let left_ty_id =
                            self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                        let right_ty_id = self.resolve_type_expression(
                            module,
                            ctx.profile,
                            *right,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        (left_ty_id, right_ty_id)
                    }
                };

                let ty = self.infer_type_binary_operation(
                    module,
                    ctx.profile,
                    expression_id,
                    operator,
                    left_ty_id,
                    right_ty_id,
                    symbols,
                    types,
                    infer,
                    &ctx.options,
                );
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. }
            | Expression::PointerOf { .. } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            Expression::Cast {
                operator,
                source,
                value,
                target_type,
            } => {
                // reject unsafe explicit casts when configured
                if ctx.options.no_unsafe_type_assertions
                    && matches!(source, CastSource::Explicit)
                    && self.is_unsafe_type_assertion(*operator)
                    && matches!(module.source, ModuleSource::User)
                {
                    self.error(AnalyzeError::UnsafeTypeAssertionDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                // infer the source and resolve the target type
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                self.resolve_type_expression(
                    module,
                    ctx.profile,
                    *target_type,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?
            }

            Expression::OwnershipCast {
                operator: _,
                source: _,
                value,
            } => {
                let mut ownership_ctx = ctx.fork().with_explicit_ownership();
                self.infer_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut ownership_ctx,
                )?
            }

            // unary operations: compound type
            Expression::Unary { operator, right } => self.infer_unary_expression(
                module,
                expression_id,
                operator,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // value of operation: value of type
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let mut ownership_ctx = ctx.fork().with_explicit_ownership();
                let right_ty_id = self.infer_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut ownership_ctx,
                )?;

                // reject ownership conversions on explicit ownership types
                if self.type_is_explicit_ownership_wrapper(types, right_ty_id) {
                    self.error(AnalyzeError::InvalidOwnershipOperand {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        actual_ty: right_ty_id.into_global(module.id),
                    });
                }

                let ty = self.infer_value_of_operation(*mutability, *variance, right_ty_id);
                types.insert_type_from(ty, expression_id)
            }

            // reference of operation: reference of type
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let mut ownership_ctx = ctx.fork().with_explicit_ownership();
                let right_ty_id = self.infer_expression(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut ownership_ctx,
                )?;

                let ty = self.infer_reference_of_operation(*mutability, *variance, right_ty_id);
                types.insert_type_from(ty, expression_id)
            }

            // binary operations: compound type
            Expression::Binary {
                left,
                operator,
                right,
            } => self.infer_binary_expression(
                module,
                expression_id,
                operator,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // assignment operations: void
            Expression::Assign { left, right } => self.infer_assign_expression(
                module,
                expression_id,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,
            Expression::AssignBinary { left, right, .. } => self.infer_assign_binary_expression(
                module,
                expression_id,
                *left,
                *right,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // delete operation: void
            Expression::Delete { value } => {
                // enforce strict mode delete restrictions on bindings
                let enforce_strict_mode =
                    module.source_type.is_module() || ctx.options.always_strict;
                if enforce_strict_mode && matches!(module.source, ModuleSource::User) {
                    let target_id = self.unwrap_parenthesized_expression(*value, tree);

                    // forbid delete on binding references in strict mode
                    if matches!(
                        tree.get(target_id),
                        Expression::LocalReference { .. }
                            | Expression::ModuleReference { .. }
                            | Expression::GlobalReference { .. }
                    ) {
                        self.error(AnalyzeError::InvalidStrictDelete {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                // reject delete in dynamic shape restricted mode
                if ctx.options.no_dynamic_shapes && matches!(module.source, ModuleSource::User) {
                    self.error(AnalyzeError::DynamicShapesDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }
                let _value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // references: look up symbol type
            Expression::UnresolvedPath {
                path: _,
                static_arguments: _,
                space_order: _,
            } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // reference: symbol type
            Expression::LocalReference {
                path: _,
                target_symbol,
                static_arguments,
            }
            | Expression::ModuleReference {
                path: _,
                target_symbol,
                static_arguments,
            }
            | Expression::GlobalReference {
                path: _,
                target_symbol,
                static_arguments,
            } => self.infer_reference_expression(
                module,
                expression_id,
                *target_symbol,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // private identifiers only appear in brand checks
            Expression::PrivateIdentifier { name: _ } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // import meta: statically known type
            Expression::ImportMeta => {
                // resolve the import.meta interface
                let Some(import_meta_symbol) =
                    self.get_language_symbol(ctx.profile, LanguageSymbol::ImportMeta)
                else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                };

                let ty = Type::Reference {
                    symbol: import_meta_symbol,
                    static_arguments: None,
                };
                types.insert_type_from(ty, expression_id)
            }

            // this: reference to the current instance item
            Expression::This => {
                let this_symbol = self.resolve_this_symbol(module, expression_id, tree, symbols);
                if let Some(this_symbol) = this_symbol
                    && let Some(ty_id) = types.get_value_type_id(this_symbol)
                {
                    ty_id
                } else if let Some(this_ty_id) =
                    self.contextual_this_type(module, ctx, types)
                {
                    this_ty_id
                } else {
                    // report implicit this in functions and scripts
                    if ctx.options.no_implicit_this
                        && !matches!(module.source, ModuleSource::Builtin(_))
                    {
                        let is_script = module.source_type.is_script();
                        let in_function = ctx.in_function.is_some();
                        if is_script || in_function {
                            self.error(AnalyzeError::ImplicitThis {
                                node: expression_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(ctx.profile)),
                            });
                        }
                    }

                    // default to undefined in modules, unknown in scripts
                    if module.source_type.is_module() {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Undefined,
                        };
                        types.insert_type_from(ty, expression_id)
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from(ty, expression_id)
                    }
                }
            }

            // scalar literal: derive type from value
            Expression::ScalarLiteral { value } => {
                // reject managed literals when runtime-managed values are disabled
                if ctx.options.no_managed
                    && !ctx.is_explicit_ownership
                    && matches!(module.source, ModuleSource::User)
                    && self.scalar_literal_is_managed(value)
                {
                    self.error(AnalyzeError::ManagedMemoryDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                // apply contextual typing when a matching expected type is available
                let allow_contextual_literal =
                    !matches!(ctx.contextual_typing, ContextualTypingMode::Satisfies);
                if allow_contextual_literal
                    && let Some(expected_ty_id) = self.expected_type_for_scalar_literal(
                        value,
                        ctx.expected_type,
                        types,
                        &ctx.options,
                    )
                {
                    expected_ty_id
                } else {
                    let literal = if self.should_widen_scalar_literal(ctx) {
                        self.widen_scalar_literal_for_module(module, value)
                    } else {
                        self.infer_scalar_literal(value)
                    };
                    let ty = Type::TypeLiteral { value: literal };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // type literal: use the given type literal?
            // #Suspicious: using the type literal type itself as its type is strange (?)
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }
            // type as a value: type
            Expression::Type { value } => {
                let ty = Type::Value { value: *value };
                types.insert_type_from(ty, expression_id)
            }

            // array expression: infer element types and build array type
            Expression::ArrayExpression { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // resolve contextual type for array literals
                let expected_ty_id = self
                    .expected_value_type(ctx.expected_type, types)
                    .map(|expected_ty_id| {
                        self.normalize_type_with_relation(
                            module,
                            ctx.profile,
                            expected_ty_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::TYPE_OPS,
                        )
                    });
                let expected_is_tuple = expected_ty_id.is_some_and(|expected_ty_id| {
                    matches!(
                        types.get_type(expected_ty_id),
                        Type::Tuple { .. } | Type::ArraySized { .. }
                    )
                });
                let is_as_const = matches!(ctx.const_context, ConstContext::AsConst);
                let infer_tuple = expected_is_tuple || is_as_const;

                // infer element types using any contextual type
                let mut expected_element_types =
                    self.expected_element_types(expected_ty_id, elements.len(), types);
                let mut expected_array_element_type =
                    self.expected_array_element_type(expected_ty_id, types);

                // allow well known array references to supply element types
                if let Some(expected_ty_id) = expected_ty_id
                    && let Type::Reference {
                        symbol,
                        static_arguments,
                    } = types.get_type(expected_ty_id).clone()
                    && let Some(well_known) = self.well_known_array_kind(ctx.profile, symbol)
                    && let Some(Type::Array { element, .. }) = self.normalize_well_known_type_reference(
                        module,
                        symbols,
                        ctx.profile,
                        expression_id.into_any(),
                        symbol,
                        well_known,
                        static_arguments.as_deref(),
                        types,
                    )
                {
                    if expected_array_element_type.is_none() {
                        expected_array_element_type = element;
                    }
                    if expected_element_types.iter().all(|ty| ty.is_none()) {
                        expected_element_types.fill(element);
                    }
                }

                // reject sparse array holes
                for element_id in elements.iter() {
                    let element = tree.get(*element_id);
                    let value_id = element.value();
                    if matches!(tree.get(value_id), Expression::Stub) {
                        self.error(AnalyzeError::ArrayLiteralHole {
                            node: value_id
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                // infer element types and collect their contextualized types
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for (index, element_id) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    let mut element_ctx =
                        ctx.nested_literal_context().with_expected_type(expected_element_ty_id);
                    self.infer_argument(
                        module,
                        *element_id,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut element_ctx,
                    )?;
                    let argument = tree.get(*element_id);
                    let value_id = argument.value();
                    let ty_id = if let Some(ty_id) =
                        types.get_inferred_type_id(value_id.into_global_any(module.id))
                    {
                        ty_id
                    } else {
                        self.infer_expression(
                            module,
                            value_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut element_ctx,
                        )?
                    };
                    if matches!(argument, Argument::Spread { .. })
                        && let Some(spread_element_type_id) = self.array_spread_element_type(
                            module,
                            ctx.profile,
                            ty_id,
                            symbols,
                            types,
                        )
                    {
                        element_type_ids.push(spread_element_type_id);
                        continue;
                    }
                    element_type_ids.push(ty_id);
                }

                // use tuple types when an expected tuple type exists
                let ty = if infer_tuple {
                    let mut tuple_elements = Vec::with_capacity(element_type_ids.len());
                    for (index, element_id) in elements.iter().enumerate() {
                        let argument = tree.get(*element_id);
                        let mut element = TypeElement::new(element_type_ids[index]);
                        if is_as_const {
                            element.is_readonly = true;
                        }
                        match argument {
                            Argument::Labeled { label, .. } => {
                                element.label = Some(*label);
                            }
                            Argument::Spread { .. } => {
                                element.is_rest = true;
                            }
                            _ => {}
                        }
                        tuple_elements.push(element);
                    }

                    Type::Tuple {
                        elements: tuple_elements,
                        is_readonly: is_as_const,
                    }
                } else {
                    // resolve the array element type
                    let element_ty_id = if element_type_ids.is_empty() {
                        expected_array_element_type
                    } else {
                        let source_type_id = element_type_ids[0];
                        Some(self.union_type_from_list(
                            element_type_ids,
                            source_type_id,
                            types,
                        ))
                    };

                    Type::Array {
                        element: element_ty_id,
                        is_readonly: is_as_const,
                    }
                };

                let ty_id = types.insert_type_from(ty, expression_id);

                // reject managed array types when managed memory is disabled
                if ctx.options.no_managed
                    && !ctx.is_explicit_ownership
                    && matches!(module.source, ModuleSource::User)
                    && self.type_contains_managed(module, ctx.profile, ty_id, types)
                {
                    self.error(AnalyzeError::ManagedMemoryDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                ty_id
            }

            // tuple expression: preserve positional element types
            Expression::TupleExpression { elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // infer element types using any contextual type
                let expected_ty_id = self
                    .expected_value_type(ctx.expected_type, types)
                    .map(|expected_ty_id| {
                        self.normalize_type_with_relation(
                            module,
                            ctx.profile,
                            expected_ty_id,
                            symbols,
                            types,
                            NormalizationMode::Assign,
                            RelationMode::TYPE_OPS,
                        )
                    });
                let expected_element_types =
                    self.expected_element_types(expected_ty_id, elements.len(), types);

                // infer element types and collect their contextualized types
                let mut element_tys = Vec::with_capacity(elements.len());
                let is_as_const = matches!(ctx.const_context, ConstContext::AsConst);
                for (index, element_id) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    let mut element_ctx =
                        ctx.nested_literal_context().with_expected_type(expected_element_ty_id);
                    self.infer_argument(
                        module,
                        *element_id,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut element_ctx,
                    )?;
                    let element = tree.get(*element_id);
                    let value_id = element.value();
                    let ty_id = if let Some(ty_id) =
                        types.get_inferred_type_id(value_id.into_global_any(module.id))
                    {
                        ty_id
                    } else {
                        self.infer_expression(
                            module,
                            value_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut element_ctx,
                        )?
                    };
                    let contextual_ty_id = if let Some(expected_element_ty_id) =
                        expected_element_ty_id
                    {
                        if self
                            .is_type_assignable(
                                module,
                                ctx.profile,
                                symbols,
                                expected_element_ty_id,
                                ty_id,
                                types,
                                &ctx.options,
                            )
                            .is_assignable()
                        {
                            expected_element_ty_id
                        } else {
                            ty_id
                        }
                    } else if ctx.expected_type.is_some() || is_as_const {
                        ty_id
                    } else {
                        let commit_ctx = ctx.for_widening_commit();
                        self.widen_scalar_literal_type_if_needed(
                            module,
                            types,
                            ty_id,
                            &commit_ctx,
                            value_id.into_any(),
                        )
                    };
                    let mut element = TypeElement::new(contextual_ty_id);
                    if is_as_const {
                        element.is_readonly = true;
                    }
                    element_tys.push(element);
                }

                let ty = Type::Tuple {
                    elements: element_tys,
                    is_readonly: is_as_const,
                };
                types.insert_type_from(ty, expression_id)
            }

            // sequence expression (comma operator): type of last expression
            Expression::SequenceExpression { expressions } => {
                let mut last_ty = None;
                for (index, expr_id) in expressions.iter().enumerate() {
                    let is_last = index + 1 == expressions.len();
                    if is_last {
                        let mut expr_ctx =
                            ctx.fork().with_expected_type(ctx.expected_type);
                        last_ty = Some(self.infer_expression(
                            module,
                            *expr_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut expr_ctx,
                        )?);
                    } else {
                        let mut expr_ctx = ctx.fork().with_expected_type(None);
                        last_ty = Some(self.infer_expression(
                            module,
                            *expr_id,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut expr_ctx,
                        )?);
                    }
                }
                // return the type of the last expression, or void if empty (shouldn't be empty?)
                last_ty.unwrap_or_else(|| {
                    types.insert_type_from(
                        Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        },
                        expression_id,
                    )
                })
            }

            // parenthesized: same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.infer_expression(module, *inner_id, tree, symbols, types, infer, ctx)?,

            // object expression: object type
            Expression::ObjectExpression { properties } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // apply contextual object type when available
                let expected_object_ty_id = self.expected_object_type(
                    module,
                    ctx.profile,
                    ctx.expected_type,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                )?;
                let expected_object_ty_id =
                    expected_object_ty_id.filter(|expected_ty_id| {
                        matches!(types.get_type(*expected_ty_id), Type::Object { .. })
                    });
                let expected_object_ty_id = if expected_object_ty_id.is_some() {
                    expected_object_ty_id
                } else {
                    self.expected_object_type_for_literal_union(
                        module,
                        ctx.profile,
                        ctx.expected_type,
                        properties,
                        &ctx.options,
                        tree,
                        symbols,
                        types,
                    )?
                };
                let (literal_fields, shapes, spread_override) = self.infer_object_literal_shapes(
                    module,
                    properties,
                    expected_object_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                self.check_excess_object_literal_properties(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    expected_object_ty_id,
                    &literal_fields,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                )?;
                if let Some(spread_override) = spread_override {
                    return Ok(spread_override);
                }

                // synthesize the final object type from collected shapes
                let mut shape_ids = Vec::with_capacity(shapes.len());
                for shape in shapes {
                    shape_ids
                        .push(types.insert_type_from(shape.into_object_type(), expression_id));
                }

                let ty_id = match shape_ids.len() {
                    0 => types.insert_type_from(
                        Type::Object {
                            fields: Vec::new(),
                            call_signatures: Vec::new(),
                            construct_signatures: Vec::new(),
                            index_signatures: Vec::new(),
                        },
                        expression_id,
                    ),
                    1 => shape_ids[0],
                    _ => types.insert_type_from(
                        Type::Union {
                            elements: shape_ids,
                        },
                        expression_id,
                    ),
                };

                // reject managed object types when managed memory is disabled
                if ctx.options.no_managed
                    && !ctx.is_explicit_ownership
                    && matches!(module.source, ModuleSource::User)
                    && self.type_contains_managed(module, ctx.profile, ty_id, types)
                {
                    self.error(AnalyzeError::ManagedMemoryDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                ty_id
            }

            // call: return type of callee
            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let return_ty_id = self.infer_call_expression(
                    module,
                    expression_id,
                    *left,
                    static_arguments.as_deref(),
                    dynamic_arguments,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                let expects_unique_symbol = ctx.expected_type.is_some_and(|expected_ty_id| {
                    matches!(
                        types.get_type(expected_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                        }
                    )
                });
                if expects_unique_symbol
                    && let Some(callee_symbol) = self.reference_symbol_for_expression(
                        module,
                        *left,
                        ctx.profile,
                        tree,
                        symbols,
                    )
                    && self.is_well_known_symbol(
                        ctx.profile,
                        callee_symbol,
                        WellKnownSymbol::Symbol,
                    )
                    && matches!(
                        types.get_type(return_ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Symbol),
                        }
                    )
                {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::UniqueSymbol),
                    };
                    types.insert_type_from(ty, expression_id)
                } else {
                    return_ty_id
                }
            }

            Expression::Member {
                left,
                name,
                static_arguments,
            } => self.infer_member_expression(
                module,
                expression_id,
                *left,
                *name,
                static_arguments.as_deref(),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,
            Expression::PrivateMember {
                left,
                name,
                static_arguments,
            } => {
                let private_name = self.private_key_string_id(*name);
                self.infer_member_expression(
                    module,
                    expression_id,
                    *left,
                    private_name,
                    static_arguments.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?
            }

            // instantiation: apply static arguments to callable type
            Expression::Instantiation {
                left,
                static_arguments,
            } => {
                let left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let static_arguments = static_arguments.as_slice();
                if static_arguments.is_empty() {
                    return Ok(left_ty_id);
                }

                self.ensure_reference_instance_types_for_type(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    left_ty_id,
                    types,
                )?;

                let Some(signature_ty_id) = self.call_signature_for_type(left_ty_id, types)
                else {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    return Ok(left_ty_id);
                };

                let Type::Function {
                    asynchrony,
                    cardinality,
                    mut static_parameters,
                    this_parameter,
                    dynamic_parameters,
                    return_type,
                } = types.get_type(signature_ty_id).clone()
                else {
                    self.error(AnalyzeError::MissingType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                    return Ok(left_ty_id);
                };

                let mut owner_symbol = self.reference_symbol_for_expression(
                    module,
                    *left,
                    ctx.profile,
                    tree,
                    symbols,
                );

                if owner_symbol.is_none() {
                    let node_id = left.into_global_any(module.id);
                    if let Some(resolution_id) = types.get_resolution_for_node(node_id) {
                        let resolution = types.get_resolution(resolution_id);
                        if let Resolution::Static { candidate, .. } = resolution {
                            owner_symbol = Some(candidate.target_symbol);
                        }
                    }
                }

                // recover static parameters when they are missing from the signature type
                if static_parameters.is_empty() {
                    static_parameters = self.recover_static_parameters_for_signature(
                        module,
                        signature_ty_id,
                        tree,
                        types,
                    );
                }

                let resolved = self.resolve_function_signature(
                    module,
                    expression_id.into_any(),
                    owner_symbol,
                    Some(static_arguments),
                    None,
                    &static_parameters,
                    &dynamic_parameters,
                    return_type,
                    super::SignatureResolutionMode::Checking,
                    false,
                    ctx.profile,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                    infer,
                )?;

                let instantiated_fn = Type::Function {
                    asynchrony,
                    cardinality,
                    static_parameters: Vec::new(),
                    this_parameter,
                    dynamic_parameters: resolved.dynamic_parameters,
                    return_type: resolved.return_type,
                };
                let instantiated_ty_id = types.insert_type_from(instantiated_fn, expression_id);

                if let Some(owner_symbol) = owner_symbol
                    && !resolved.static_arguments.is_empty()
                {
                    self.register_instance_for_node(
                        expression_id.into_global_any(module.id),
                        owner_symbol,
                        resolved.static_arguments,
                        types,
                    );
                }

                instantiated_ty_id
            }

            // index: element type
            Expression::Index { left, right } => {
                self.infer_index_access_expression(
                    module,
                    expression_id,
                    *left,
                    *right,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?
            }

            // new: instance type
            Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => self.infer_new_expression(
                module,
                expression_id,
                *left,
                static_arguments.as_deref(),
                dynamic_arguments,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // if expression: union of branches or common type
            Expression::If {
                kind: _,
                condition,
                then_expression,
                else_expression,
            } => {
                // infer the condition
                match condition {
                    IfCondition::Expression { condition } => {
                        self.infer_expression(
                            module,
                            *condition,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                    IfCondition::Let { declarator, .. } => {
                        self.infer_declarator(
                            module,
                            *declarator,
                            expression_id,
                            DeclaratorConstraint::Satisfies,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                }

                // infer the then branch
                let mut then_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                let then_ty_id = self.infer_expression(
                    module,
                    *then_expression,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut then_ctx,
                )?;

                // infer the else branch
                let else_ty_id = if let Some(else_expr) = else_expression {
                    let mut else_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                    Some(self.infer_expression(
                        module,
                        *else_expr,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut else_ctx,
                    )?)
                } else {
                    None
                };

                // compute the result type from the branches
                if let Some(else_ty_id) = else_ty_id {
                    self.flow_join_type(
                        module,
                        symbols,
                        types,
                        ctx,
                        expression_id.into_any(),
                        &[then_ty_id, else_ty_id],
                    )
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // loop: loop body type or never
            Expression::Loop {
                kind,
                condition,
                body,
                scope: _,
                symbol,
            } => {
                if let Some(cond) = condition {
                    self.infer_expression(module, *cond, tree, symbols, types, infer, ctx)?;
                }
                let loop_expected_type = ctx.expected_type;
                let loop_symbol = symbol.into_global(module.id);
                let mut ctx = ctx
                    .fork()
                    .with_expected_type(None)
                    .in_loop_with_symbol(expression_id.into_any(), loop_symbol, loop_expected_type);
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;
                let loop_context = ctx.pop_loop_context();
                let (break_values, expected_type) = loop_context
                    .map(|context| (context.break_values, context.expected_type))
                    .unwrap_or_else(|| (Vec::new(), None));
                if *kind == LoopKind::NoTest {
                    self.loop_break_result_type(
                        module,
                        symbols,
                        types,
                        &ctx,
                        expression_id.into_any(),
                        &break_values,
                        expected_type,
                    )
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // for each: void
            Expression::ForEach {
                asynchrony: _,
                kind: _,
                binding,
                iterator,
                body,
                scope: _,
                symbol,
            } => {
                let iterator_ty_id =
                    self.infer_expression(module, *iterator, tree, symbols, types, infer, ctx)?;
                match binding {
                    ForEachBinding::Pattern { pattern } => {
                        self.infer_pattern(
                            module,
                            *pattern,
                            Some(iterator_ty_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                    ForEachBinding::Using {
                        asynchrony: _,
                        pattern,
                    } => {
                        self.infer_pattern(
                            module,
                            *pattern,
                            Some(iterator_ty_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                }
                let loop_expected_type = ctx.expected_type;
                let loop_symbol = symbol.into_global(module.id);
                let mut ctx = ctx
                    .fork()
                    .with_expected_type(None)
                    .in_loop_with_symbol(expression_id.into_any(), loop_symbol, loop_expected_type);
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // for: void
            Expression::For {
                initialization,
                condition,
                increment,
                body,
                scope: _,
                symbol,
            } => {
                if let Some(initialization) = initialization {
                    self.infer_expression(module, *initialization, tree, symbols, types, infer, ctx)?;
                }
                if let Some(condition) = condition {
                    self.infer_expression(module, *condition, tree, symbols, types, infer, ctx)?;
                }
                if let Some(increment) = increment {
                    self.infer_expression(module, *increment, tree, symbols, types, infer, ctx)?;
                }
                let loop_expected_type = ctx.expected_type;
                let loop_symbol = symbol.into_global(module.id);
                let mut ctx = ctx
                    .fork()
                    .with_expected_type(None)
                    .in_loop_with_symbol(expression_id.into_any(), loop_symbol, loop_expected_type);
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // match/switch: infer case result types
            Expression::Match {
                kind,
                value,
                cases,
                source,
                scope: _,
                symbol: _,
            } => {
                let value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;

                // match and switch set different break contexts
                let mut base_ctx = ctx.fork();
                if *source == MatchSource::Match {
                    if *kind == MatchKind::Match {
                        base_ctx = base_ctx.in_match(expression_id.into_any());
                    } else {
                        base_ctx = base_ctx.in_switch(expression_id.into_any());
                    }
                }
                let is_switch = *kind == MatchKind::Switch;
                let mut case_type_ids = Vec::new();
                for case_id in cases {
                    let mut case_ctx = base_ctx.fork();
                    let case = tree.get(*case_id);
                    let (selector, body_expr, block_body) = match case {
                        MatchCase::Expression {
                            selector,
                            body,
                            scope: _,
                        } => (selector, Some(*body), None),
                        MatchCase::Block {
                            selector,
                            body,
                            scope: _,
                        } => (selector, None, Some(*body)),
                    };
                    // infer pattern and guard from selector
                    if let MatchSelector::Pattern { pattern, guard } = selector {
                        let mut allow_pattern_infer = true;
                        if is_switch {
                            match tree.get(*pattern) {
                                Pattern::Expression { value } => {
                                    if self.is_invalid_switch_case_expression(tree, *value) {
                                        allow_pattern_infer = false;
                                    }
                                }
                                _ => allow_pattern_infer = false,
                            }
                        }
                        if allow_pattern_infer {
                            self.infer_pattern(
                                module,
                                *pattern,
                                Some(value_ty_id),
                                tree,
                                symbols,
                                types,
                                infer,
                                &mut case_ctx,
                            )?;
                        }
                        if let Some(guard_expr) = guard
                            && !is_switch
                        {
                            self.infer_expression(
                                module,
                                *guard_expr,
                                tree,
                                symbols,
                                types,
                                infer,
                                &mut case_ctx,
                            )?;
                        }
                    }

                    // apply contextual typing to the case body
                    let expected_type = if *kind == MatchKind::Match {
                        base_ctx.expected_type
                    } else {
                        None
                    };
                    let mut case_ctx = case_ctx.with_expected_type(expected_type);

                    // infer the case body and collect types for matches
                    let case_ty_id = if let Some(expr) = body_expr {
                        Some(self.infer_expression(
                            module,
                            expr,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut case_ctx,
                        )?)
                    } else if let Some(body) = block_body {
                        Some(self.infer_block(
                            module,
                            body,
                            tree,
                            symbols,
                            types,
                            infer,
                            &mut case_ctx,
                        )?)
                    } else {
                        None
                    };
                    if let Some(case_ty_id) = case_ty_id
                        && !is_switch
                    {
                        case_type_ids.push(case_ty_id);
                    }
                }
                if *kind == MatchKind::Switch {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                } else {
                    match case_type_ids.len() {
                        0 => {
                            let ty = Type::TypeLiteral {
                                value: TypeLiteral::Never,
                            };
                            types.insert_type_from(ty, expression_id)
                        }
                        1 => case_type_ids[0],
                        _ => self.flow_join_type(
                            module,
                            symbols,
                            types,
                            ctx,
                            expression_id.into_any(),
                            &case_type_ids,
                        ),
                    }
                }
            }

            // try: result type
            Expression::Try {
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => {
                // validate try shape
                if catch_expression.is_none() && finally_expression.is_none() {
                    self.error(AnalyzeError::IncompleteTry {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }

                // collect try errors for catch typing
                let has_catch = catch_expression.is_some();

                // infer the try body
                let mut try_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                try_ctx.push_try_frame(has_catch);
                let try_ty_id = self.infer_expression(
                    module,
                    *try_expression,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut try_ctx,
                )?;

                // capture try errors before entering catch
                let try_error_types = try_ctx
                    .pop_try_frame()
                    .map(|frame| frame.error_types)
                    .unwrap_or_default();

                // infer the catch pattern and expression
                let mut catch_ty_id = None;
                if let Some(catch_expr) = catch_expression {
                    if let Some(catch_pat) = catch_pattern {
                        // infer the catch error type from try branches
                        let catch_error_type_id = if try_error_types.is_empty() {
                            let value = if ctx.options.use_unknown_in_catch_variables {
                                TypeLiteral::Unknown
                            } else {
                                TypeLiteral::Any
                            };
                            types.insert_type_from_any(
                                Type::TypeLiteral { value },
                                expression_id.into_any(),
                            )
                        } else {
                            let source_type_id = try_error_types[0];
                            self.union_type_from_list(try_error_types, source_type_id, types)
                        };

                        // bind the catch pattern to the error type
                        self.infer_pattern(
                            module,
                            *catch_pat,
                            Some(catch_error_type_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }

                    // infer the catch expression with contextual typing
                    let mut catch_ctx = ctx.fork().with_expected_type(ctx.expected_type);
                    catch_ty_id = Some(self.infer_expression(
                        module,
                        *catch_expr,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut catch_ctx,
                    )?);
                }

                // infer the finally expression
                if let Some(finally_expr) = finally_expression {
                    let mut finally_ctx = ctx.fork().with_expected_type(None);
                    self.infer_expression(
                        module,
                        *finally_expr,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut finally_ctx,
                    )?;
                }

                // combine try and catch result types
                if let Some(catch_ty_id) = catch_ty_id {
                    self.flow_join_type(
                        module,
                        symbols,
                        types,
                        ctx,
                        expression_id.into_any(),
                        &[try_ty_id, catch_ty_id],
                    )
                } else {
                    try_ty_id
                }
            }

            // return: never (control flow)
            Expression::Return { value } => self.infer_return_expression(
                module,
                expression_id,
                *value,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // break: never
            Expression::Break {
                target,
                target_symbol,
                value,
            } => {
                let is_labelled = target.is_some();
                if !is_labelled && !ctx.can_break() {
                    self.error(AnalyzeError::InvalidBreak {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                        label: *target,
                    });
                }
                // determine the innermost break target
                let break_target = ctx.break_stack.last().copied();
                let is_switch_break =
                    !is_labelled && matches!(break_target, Some(BreakTargetKind::Switch));
                if is_switch_break && value.is_some() {
                    self.error(AnalyzeError::InvalidSwitchBreakValue {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }
                if let Some(val) = value {
                    let value_ty_id =
                        self.infer_expression(module, *val, tree, symbols, types, infer, ctx)?;
                    if !is_switch_break {
                        ctx.record_break_value(*target_symbol, value_ty_id);
                    }
                } else if !is_switch_break {
                    let void_ty_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Void,
                        },
                        expression_id.into_any(),
                    );
                    ctx.record_break_value(*target_symbol, void_ty_id);
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedBreak { target: _, value } => {
                if let Some(val) = value {
                    self.infer_expression(module, *val, tree, symbols, types, infer, ctx)?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // continue: never
            Expression::Continue { target, target_symbol: _ } => {
                if !ctx.can_continue() {
                    self.error(AnalyzeError::InvalidContinue {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                        label: *target,
                    });
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedContinue { target } => {
                if !ctx.can_continue() {
                    self.error(AnalyzeError::InvalidContinue {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                        label: Some(*target),
                    });
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // throw: never (control flow)
            Expression::Throw { value } => {
                // enforce no-exceptions mode
                if ctx.options.no_exceptions {
                    self.error(AnalyzeError::ExceptionsDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Never,
                };
                types.insert_type_from(ty, expression_id)
            }

            // await: awaited type
            Expression::Await { expression } => {
                // reject await when runtime is disabled
                if ctx.options.no_runtime && matches!(module.source, ModuleSource::User) {
                    self.error(AnalyzeError::RuntimeDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                if !ctx.can_await() {
                    self.error(AnalyzeError::InvalidAwait {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }
                let inner_ty_id =
                    self.infer_expression(module, *expression, tree, symbols, types, infer, ctx)?;

                let awaited_ty_id =
                    self.unwrap_awaited_type(module, symbols, ctx.profile, inner_ty_id, types);
                if awaited_ty_id == inner_ty_id
                    && !self.type_is_any_or_unknown(inner_ty_id, types)
                    && let Some(promise_ty_id) =
                        self.promise_type(ctx.profile, None, expression_id.into_any(), types)
                {
                    self.error(AnalyzeError::UnassignableType {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        expected_ty: promise_ty_id.into_global(module.id),
                        actual_ty: inner_ty_id.into_global(module.id),
                    });
                }
                awaited_ty_id
            }

            // await? should be desugared in Bind
            Expression::AwaitMaybe { .. } => {
                unreachable!("AwaitMaybe should be desugared before analysis")
            }

            // comptime: type of body (evaluated at compile time)
            Expression::Comptime { body } => {
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?
            }

            // yield: yielded type
            Expression::Yield {
                cardinality,
                value,
            } => {
                // reject yield when runtime is disabled
                if ctx.options.no_runtime && matches!(module.source, ModuleSource::User) {
                    self.error(AnalyzeError::RuntimeDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                    });
                }

                if !ctx.can_yield() {
                    self.error(AnalyzeError::InvalidYield {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }
                let mut delegate_return_type = None;
                if let Some(value_id) = value {
                    let value_ty_id =
                        self.infer_expression(module, *value_id, tree, symbols, types, infer, ctx)?;
                    if let Some(expected_yield_ty_id) = ctx.generator_yield_type {
                        if *cardinality == YieldCardinality::Generator {
                            if let Some((yield_ty_id, return_ty_id)) = self
                                .yield_star_delegate_types(
                                    module,
                                    ctx.profile,
                                    value_ty_id,
                                    symbols,
                                    types,
                                )
                            {
                                delegate_return_type = return_ty_id;
                                if self.is_type_assignable(
                                    module,
                                    ctx.profile,
                                    symbols,
                                    expected_yield_ty_id,
                                    yield_ty_id,
                                    types,
                                    &ctx.options,
                                ) == Assignability::NotAssignable
                                {
                                    self.error(AnalyzeError::UnassignableType {
                                        node: value_id
                                            .into_global_any(module.id)
                                            .into_anchored(Some(ctx.profile)),
                                        expected_ty: expected_yield_ty_id.into_global(module.id),
                                        actual_ty: yield_ty_id.into_global(module.id),
                                    });
                                }
                            }
                        } else if self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            expected_yield_ty_id,
                            value_ty_id,
                            types,
                            &ctx.options,
                        ) == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: value_id
                                    .into_global_any(module.id)
                                    .into_anchored(Some(ctx.profile)),
                                expected_ty: expected_yield_ty_id.into_global(module.id),
                                actual_ty: value_ty_id.into_global(module.id),
                            });
                        }
                    }
                }
                if let Some(return_ty_id) = delegate_return_type {
                    return_ty_id
                } else if let Some(next_ty_id) = ctx.generator_next_type {
                    next_ty_id
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // maybe unwrap: try operator
            Expression::Maybe { left } => self.infer_try_unwrap_expression(
                module,
                expression_id,
                *left,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?,

            // must unwrap: non null assertion
            Expression::Must { left } => {
                let left_ty_id =
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                let (non_nullish_ty_id, has_nullish) =
                    self.strip_nullish_from_union(left_ty_id, types);
                if has_nullish {
                    // nullish only must results in never
                    non_nullish_ty_id.unwrap_or_else(|| {
                        types.insert_type_from(
                            Type::TypeLiteral {
                                value: TypeLiteral::Never,
                            },
                            expression_id,
                        )
                    })
                } else {
                    left_ty_id
                }
            }

            // template expressions: string
            Expression::TemplateExpression { value: _ } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_TEMPLATE);

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::TaggedTemplateExpression { tag, value } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_TEMPLATE);

                self.validate_call_expression(
                    module,
                    expression_id,
                    *tag,
                    tree,
                    symbols,
                    &ctx.options,
                    ctx.profile,
                    false,
                );
                self.infer_tagged_template_expression(
                    module,
                    expression_id,
                    *tag,
                    value,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?
            }

            // range expression: analyze bounds, type is Range<T> or RangeInclusive<T>
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_ty_id =
                    self.infer_expression(module, *start, tree, symbols, types, infer, ctx)?;
                let end_ty_id =
                    self.infer_expression(module, *end, tree, symbols, types, infer, ctx)?;

                let element_ty_id = self.best_common_type_for_pair(
                    module,
                    symbols,
                    types,
                    ctx,
                    expression_id.into_any(),
                    start_ty_id,
                    end_ty_id,
                    None,
                    true,
                );
                let range_symbol = if *is_inclusive {
                    self.get_language_symbol(ctx.profile, LanguageSymbol::RangeInclusive)
                } else {
                    self.get_language_symbol(ctx.profile, LanguageSymbol::Range)
                };
                let Some(range_symbol) = range_symbol else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    return Ok(types.insert_type_from(ty, expression_id));
                };
                let static_arguments = vec![StaticArgument::Evaluated {
                    name: None,
                    value: StaticExpression::Type { ty: element_ty_id },
                }];
                let ty = Type::Reference {
                    symbol: range_symbol,
                    static_arguments: Some(static_arguments),
                };
                types.insert_type_from(ty, expression_id)
            }

            // tagged expressions for newtype construction
            Expression::TaggedScalarExpression { ty, value } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // resolve the tag type
                let ty_id = self.try_evaluate_expression_to_type(
                    module,
                    ctx.profile,
                    *ty,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    ctx.expected_type,
                    ty_id,
                    tree,
                    types,
                );

                // infer the value expression
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;

                ty_id
            }
            Expression::TaggedTupleExpression { ty, elements } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // resolve the tag type
                let ty_id = self.try_evaluate_expression_to_type(
                    module,
                    ctx.profile,
                    *ty,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    ctx.expected_type,
                    ty_id,
                    tree,
                    types,
                );

                // collect expected element types
                let expected_element_types =
                    self.expected_element_types(Some(ty_id), elements.len(), types);

                // infer each element using contextual types
                for (index, elem) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(
                        module,
                        *elem,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                ty_id
            }
            Expression::TaggedObjectExpression { ty, properties } => {
                let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_LITERAL);

                // resolve the tag type
                let ty_id = self.try_evaluate_expression_to_type(
                    module,
                    ctx.profile,
                    *ty,
                    tree,
                    symbols,
                    types,
                    true,
                    true,
                )?;
                let ty_id = self.expected_tag_reference_type_from_context(
                    *ty,
                    ctx.expected_type,
                    ty_id,
                    tree,
                    types,
                );

                // derive an expected object type from the tag
                let expected_object_ty_id = self.expected_object_type(
                    module,
                    ctx.profile,
                    Some(ty_id),
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                )?;

                // narrow expected object types to struct fields for tagged literals
                let expected_object_ty_id = self.expected_tagged_object_type(
                    module,
                    ctx.profile,
                    expression_id,
                    ty_id,
                    expected_object_ty_id,
                    tree,
                    symbols,
                    types,
                )?;

                // infer object literal shapes and fields
                let (literal_fields, shapes, spread_override) = self.infer_object_literal_shapes(
                    module,
                    properties,
                    expected_object_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                self.check_excess_object_literal_properties(
                    module,
                    ctx.profile,
                    expression_id.into_any(),
                    expected_object_ty_id,
                    &literal_fields,
                    &ctx.options,
                    tree,
                    symbols,
                    types,
                )?;

                // validate shapes against the expected type
                if let Some(expected_object_ty_id) = expected_object_ty_id
                    && spread_override.is_none()
                {
                    // validate spread shapes against the explicit type
                    for shape in shapes {
                        let shape_ty_id =
                            types.insert_type_from(shape.into_object_type(), expression_id);
                        if self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            expected_object_ty_id,
                            shape_ty_id,
                            types,
                            &ctx.options,
                        ) == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                                expected_ty: expected_object_ty_id.into_global(module.id),
                                actual_ty: shape_ty_id.into_global(module.id),
                            });
                            break;
                        }
                    }
                }

                // unwrap Type::Value to get the actual instance type
                self.expected_value_type(Some(ty_id), types).unwrap_or(ty_id)
            }

            // tree expression (JSX like)
            Expression::TreeExpression {
                left,
                arguments,
                elements,
            } => {
                if let Some(left) = left {
                    self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?;
                }
                if let Some(args) = arguments {
                    for arg in args {
                        self.infer_argument(module, *arg, None, tree, symbols, types, infer, ctx)?;
                    }
                }
                if let Some(elems) = elements {
                    for elem in elems {
                        self.infer_argument(module, *elem, None, tree, symbols, types, infer, ctx)?;
                    }
                }
                // #Incomplete: JSX element type (see Elaborate/reify)
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // error expression: error type
            Expression::Error => {
                let ty = Type::Error;
                types.insert_type_from(ty, expression_id)
            }

            // debugger: void
            Expression::Debugger => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // stub: nothing to do
            Expression::Stub => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }
        };

        // determine addressability for the expression
        self.ensure_expression_addressability(module, expression_id, tree, types);

        if !ctx.is_surface_inference {
            types.set_inferred_type(node_id, ty_id);
        }

        Ok(ty_id)
    }
    }

    /// Ensure addressability is cached for an expression.
    fn ensure_expression_addressability(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        types: &mut TypeTable,
    ) -> Addressability {
        let node_id = expression_id.into_global_any(module.id);
        if let Some(addressability) = types.get_addressability_for_node(node_id) {
            return addressability;
        }

        // walk the expression to compute addressability
        let expression = tree.get(expression_id);
        let addressability = match expression {
            Expression::Parenthesized { expression } => {
                self.ensure_expression_addressability(module, *expression, tree, types)
            }
            Expression::LocalReference { .. }
            | Expression::ModuleReference { .. }
            | Expression::GlobalReference { .. }
            | Expression::This => Addressability::Place,
            Expression::Member {
                static_arguments, ..
            }
            | Expression::PrivateMember {
                static_arguments, ..
            } => {
                if static_arguments.is_some() {
                    Addressability::Value
                } else {
                    Addressability::Place
                }
            }
            Expression::Index { .. } => Addressability::Place,
            _ => Addressability::Value,
        };

        types.set_addressability_for_node(node_id, addressability);
        addressability
    }

    /// Infer a block.
    pub(super) fn infer_block(
        &self,
        module: &Module,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if !ctx.is_surface_inference
            && let Some(ty_id) = types.get_inferred_type_id(block_id.into_global_any(module.id))
        {
            return Ok(ty_id);
        }

        let block = tree.get(block_id);

        // infer all but the last expression without contextual typing
        let last_index = block.expressions.len().saturating_sub(1);
        for (index, expression_id) in block.expressions.iter().enumerate() {
            if index == last_index {
                continue;
            }
            let mut expr_ctx = ctx.fork().with_expected_type(None);
            self.infer_expression(
                module,
                *expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut expr_ctx,
            )?;
            ctx.merge_try_error_types_from(&expr_ctx);
            ctx.merge_break_values_from(&expr_ctx);
        }

        // infer the last expression with contextual typing
        let ty_id = if let Some(last_expression_id) = block.expressions.last() {
            let mut last_ctx = ctx.fork().with_expected_type(ctx.expected_type);
            let ty_id = self.infer_expression(
                module,
                *last_expression_id,
                tree,
                symbols,
                types,
                infer,
                &mut last_ctx,
            )?;
            ctx.merge_try_error_types_from(&last_ctx);
            ctx.merge_break_values_from(&last_ctx);
            ty_id
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            types.insert_type_from(ty, block_id)
        };

        if !ctx.is_surface_inference {
            types.set_inferred_type(block_id.into_global_any(module.id), ty_id);
        }

        Ok(ty_id)
    }

    /// Find the nearest value symbol for a name in scope.
    fn find_value_symbol_by_name(
        &self,
        module: &Module,
        property_id: LocalNodeId<Property>,
        name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let key = StaticKey::Name(name);
        let mut scope = symbols.get_scope(property_id, tree);

        loop {
            // scan for a value or type value symbol with the requested name
            let mut candidates = symbols
                .active_named_symbols_up_to(scope.1, scope.2)
                .collect::<Vec<_>>();
            for (candidate_key, symbol_id) in candidates.drain(..).rev() {
                if candidate_key != key {
                    continue;
                }

                let symbol = symbols.get_symbol(symbol_id);
                if matches!(symbol.space, SymbolSpace::Value | SymbolSpace::TypeValue) {
                    return Some(symbol_id.into_global(module.id));
                }
            }

            // fall back to the parent scope
            let (parent_scope_id, parent_mark) = scope.1.parent?;
            scope = (
                parent_scope_id,
                symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }
    }

    /// Resolve contextual `this` from the current function signature.
    pub(crate) fn contextual_this_type(
        &self,
        module: &Module,
        ctx: &InferContext,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        let function_id = ctx.in_function?;
        let signature_ty_id =
            types.get_signature_type_for_node(function_id.into_global(module.id))?;
        let signature_ty = types.get_type(signature_ty_id);
        match signature_ty {
            Type::Function { this_parameter, .. } => *this_parameter,
            _ => None,
        }
    }

    /// Infer the value type for a shorthand object literal field.
    fn infer_shorthand_property_value(
        &self,
        module: &Module,
        property_id: LocalNodeId<Property>,
        name: StringId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // resolve the referenced symbol from the current scope
        let Some(target_symbol) =
            self.find_value_symbol_by_name(module, property_id, name, tree, symbols)
        else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            return Ok(types.insert_type_from_any(ty, property_id.into_any()));
        };

        // canonicalize imports before picking a type
        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            ctx.profile,
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // reuse a narrowed or declared value type when possible
        let base_ty_id = if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
            narrowed_ty_id
        } else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
            value_ty_id
        } else if let Some(inferred_ty_id) = self.infer_direct_binding_value_type(
            module,
            ctx.profile,
            canonical_symbol,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )? {
            inferred_ty_id
        } else if canonical_symbol.module_id != module.id {
            self.resolve_remote_symbol_value_type(
                module,
                ctx.profile,
                property_id.into_any(),
                canonical_symbol,
                ctx.is_surface_inference,
                types,
            )?
        } else {
            let scope = InferScope {
                owner: canonical_symbol,
                function_id: ctx.in_function.map(|f| f.into_global(module.id)),
            };
            self.infer_var_type_for_symbol(
                infer,
                types,
                canonical_symbol,
                property_id.into_any(),
                InferOrigin::Expression(property_id.into_global_any(module.id)),
                scope,
            )
        };

        // ensure instance types for referenced symbols
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            property_id.into_any(),
            base_ty_id,
            types,
        )?;

        Ok(base_ty_id)
    }

    /// Infer a property and return its TypeField if it has a static key.
    pub(super) fn infer_property(
        &self,
        module: &Module,
        property_id: LocalNodeId<Property>,
        expected_object_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<Option<ObjectLiteralField>> {
        let options = ctx.options;
        let property = tree.get(property_id);
        match property {
            Property::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                // extract the static key from the dynamic key
                let static_key = key.and_then(|key| {
                    self.static_key_from_dynamic_key(ctx.profile, key, tree, symbols, types)
                });

                // derive an expected field type from the contextual object type
                let expected_field_ty_id = static_key
                    .as_ref()
                    .and_then(|key| self.expected_field_type(expected_object_ty_id, key, types));

                // infer the value type
                let value_ty_id = if let Some(value) = value {
                    // infer explicit property values
                    let is_as_const = modifiers.as_ref().is_some_and(|modifiers| {
                        matches!(modifiers.operator, Some(BindingOperator::AsConst))
                    });
                    let mut value_ctx = if is_as_const {
                        ctx.fork()
                            .with_expected_type(expected_field_ty_id)
                            .with_const_assertion_context()
                    } else {
                        ctx.nested_literal_context()
                            .with_expected_type(expected_field_ty_id)
                    };
                    self.infer_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut value_ctx,
                    )?
                } else if let Some(DynamicKey::Name(name)) = key {
                    // infer shorthand values from the referenced symbol
                    self.infer_shorthand_property_value(
                        module,
                        property_id,
                        *name,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from_any(ty, property_id.into_any())
                };

                // infer default with the same expected type
                if let Some(default) = default {
                    let is_as_const = modifiers.as_ref().is_some_and(|modifiers| {
                        matches!(modifiers.operator, Some(BindingOperator::AsConst))
                    });
                    let mut default_ctx = if is_as_const {
                        ctx.fork()
                            .with_expected_type(expected_field_ty_id)
                            .with_const_assertion_context()
                    } else {
                        ctx.nested_literal_context()
                            .with_expected_type(expected_field_ty_id)
                    };
                    self.infer_expression(
                        module,
                        *default,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut default_ctx,
                    )?;
                }

                // is optional
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(BindingKind::Maybe)));

                // is readonly
                let mut is_readonly = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.mutability, Some(Mutability::Immutable)));
                if matches!(ctx.const_context, ConstContext::AsConst) {
                    is_readonly = true;
                }

                // static key
                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: value_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
                ..
            } => {
                let expected_method_ty_id = key
                    .and_then(|key| {
                        self.static_key_from_dynamic_key(ctx.profile, key, tree, symbols, types)
                    })
                    .and_then(|key| self.expected_field_type(expected_object_ty_id, &key, types));

                // enforce runtime constraints up front
                self.check_signature_runtime_constraints(
                    module,
                    ctx.profile,
                    property_id.into_any(),
                    signature,
                    ctx.options,
                );

                // infer the method signature with contextual typing
                let declared_signature_ty_id =
                    types.get_signature_type_for_node(property_id.into_global_any(module.id));
                let method_ty_id = if self.should_use_declared_signature(
                    module,
                    signature,
                    declared_signature_ty_id,
                    expected_method_ty_id,
                    tree,
                    types,
                ) {
                    let declared_signature_ty_id = declared_signature_ty_id
                        .expect("declared signature type required for skipped signature inference");
                    self.bind_declared_signature(
                        module,
                        property_id.into_any(),
                        signature,
                        declared_signature_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?
                } else {
                    self.infer_signature(
                        module,
                        property_id.into_any(),
                        symbol.into_global(module.id),
                        signature,
                        expected_method_ty_id,
                        declared_signature_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?
                };

                // body
                if let Some(body) = body {
                    let return_type = self.function_return_type(method_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .without_const_context()
                        .in_function_with_signature(property_id.into_any(), signature);
                    let mut context_return_type = return_type;
                    let mut ctx = if signature.cardinality == FunctionCardinality::Generator {
                        let (yield_ty_id, return_ty_id, next_ty_id) = self.generator_context_types(
                            module,
                            ctx.profile,
                            property_id.into_any(),
                            return_type,
                            symbols,
                            types,
                        );
                        context_return_type = Some(return_ty_id);
                        ctx.with_return_type(Some(return_ty_id))
                            .with_generator_types(Some(yield_ty_id), Some(next_ty_id))
                    } else {
                        ctx.with_return_type(return_type)
                    };
                    ctx = ctx.with_expected_type(context_return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id = self
                        .infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = context_return_type
                        && has_implicit_return(*body, tree)
                    {
                        infer.push_constraint(Constraint::Subtype {
                            sub_type: body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        if !self.is_infer_var_type(return_ty_id, types)
                            && !self.is_infer_var_type(body_ty_id, types)
                            && self.is_type_assignable(
                                module,
                                ctx.profile,
                                symbols,
                                return_ty_id,
                                body_ty_id,
                                types,
                                &options,
                            ) == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: body
                                    .into_global_any(module.id)
                                    .into_anchored(Some(ctx.profile)),
                                expected_ty: return_ty_id.into_global(module.id),
                                actual_ty: body_ty_id.into_global(module.id),
                            });
                        }
                    }
                }
                let static_key = key.and_then(|key| {
                    self.static_key_from_dynamic_key(ctx.profile, key, tree, symbols, types)
                });
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(BindingKind::Maybe)));
                let is_readonly = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.mutability, Some(Mutability::Immutable)));
                if let Some(key) = static_key {
                    Ok(Some(ObjectLiteralField {
                        field: TypeField {
                            key,
                            ty: method_ty_id,
                            is_optional,
                            is_readonly,
                        },
                        property_id,
                    }))
                } else {
                    Ok(None)
                }
            }
            Property::Spread { .. } => {
                unreachable!("spread properties are handled before infer_property")
            }
        }
    }

    /// Infer a dependency item.
    pub(super) fn infer_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => {
                // nothing to do
            }
            Pattern::Must(pattern_id) => {
                self.infer_pattern(
                    module,
                    *pattern_id,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.infer_pattern(
                    module,
                    *right,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Binding {
                mutability: _,
                name: _,
                symbol,
                pattern,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    let binding_symbol = symbol.into_global(module.id);
                    types.set_value_type(binding_symbol, ty_id);

                    // mirror onto the canonical symbol to avoid lookup misses
                    let canonical_symbol = self.canonical_symbol_id(
                        module,
                        symbols,
                        ctx.profile,
                        binding_symbol,
                        CanonicalSymbolMode::FollowAliases,
                    );
                    if canonical_symbol != binding_symbol {
                        types.set_value_type(canonical_symbol, ty_id);
                    }
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Expression { value } => {
                let value_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                // ensure the pattern expression is compatible with the binding type
                if let Some(binding_ty_id) = binding_ty_id {
                    let assignable = self.is_type_assignable(
                        module,
                        ctx.profile,
                        symbols,
                        binding_ty_id,
                        value_ty_id,
                        types,
                        &ctx.options,
                    );
                    if !assignable.is_assignable() {
                        self.error(AnalyzeError::UnassignableType {
                            node: value
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                            expected_ty: binding_ty_id.into_global(module.id),
                            actual_ty: value_ty_id.into_global(module.id),
                        });
                    }
                }
            }
            Pattern::Range {
                start,
                end,
                is_inclusive: _,
            } => {
                if let Some(start_pattern_id) = start {
                    self.infer_pattern(
                        module,
                        *start_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
                if let Some(end_pattern_id) = end {
                    self.infer_pattern(
                        module,
                        *end_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Tuple { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Tuple {
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                // prefer union variants from the binding type when available
                let mut ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
                if let Some(binding_ty_id) = binding_ty_id
                    && let Some(union_ty_id) =
                        self.select_union_variant_for_tagged_pattern(types, binding_ty_id, ty_id)
                {
                    ty_id = union_ty_id;
                }
                // handle scalar tagged patterns like `UserId(value)`
                if fields.len() == 1 {
                    let field = tree.get(fields[0]);
                    if let PatternField::Positional { pattern } = field {
                        self.infer_pattern(
                            module,
                            *pattern,
                            Some(ty_id),
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                        return Ok(());
                    }
                }

                self.infer_pattern_sequence(
                    module,
                    fields,
                    Some(ty_id),
                    |rest_types| Type::Tuple {
                        elements: rest_types.into_iter().map(TypeElement::new).collect(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Array { fields } => {
                self.infer_pattern_sequence(
                    module,
                    fields,
                    binding_ty_id,
                    |rest_types| Type::Array {
                        element: rest_types.first().cloned(),
                        is_readonly: false,
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Object { fields } => {
                // reject bare object patterns against nominal object values
                if let Some(binding_ty_id) = binding_ty_id
                    && self.is_nominal_object_pattern_target(
                        module,
                        ctx.profile,
                        pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                    )?
                {
                    let object_ty_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Object,
                        },
                        pattern_id.into_any(),
                    );
                    self.error(AnalyzeError::UnassignableType {
                        node: pattern_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile)),
                        expected_ty: binding_ty_id.into_global(module.id),
                        actual_ty: object_ty_id.into_global(module.id),
                    });
                }

                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                // prefer union variants from the binding type when available
                let mut ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
                if let Some(binding_ty_id) = binding_ty_id
                    && let Some(union_ty_id) =
                        self.select_union_variant_for_tagged_pattern(types, binding_ty_id, ty_id)
                {
                    ty_id = union_ty_id;
                }
                for field_id in fields {
                    self.infer_pattern_field(
                        module,
                        *field_id,
                        Some(ty_id),
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Select a union variant for a tagged pattern to preserve static arguments.
    fn select_union_variant_for_tagged_pattern(
        &self,
        types: &TypeTable,
        binding_ty_id: LocalTypeId,
        tag_ty_id: LocalTypeId,
    ) -> Option<LocalTypeId> {
        let tag_symbol = self.unwrap_type_value_symbol(types, tag_ty_id)?;

        // unwrap value wrappers before inspecting the binding union
        let binding_ty_id = types.unwrap_value_type_id(binding_ty_id);
        let Type::Union { elements } = types.get_type(binding_ty_id) else {
            return None;
        };

        for element_id in elements {
            if self
                .unwrap_type_value_symbol(types, *element_id)
                .is_some_and(|symbol| symbol == tag_symbol)
            {
                return Some(*element_id);
            }
        }

        None
    }

    /// Check whether a binding type is a nominal object for untagged patterns.
    fn is_nominal_object_pattern_target(
        &self,
        module: &Module,
        profile: ProfileId,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<bool> {
        let binding_ty = types.get_type(binding_ty_id);
        if self.is_definitely_struct_type(binding_ty) {
            return Ok(true);
        }

        let Type::Reference { symbol, .. } = binding_ty else {
            return Ok(false);
        };
        if symbol.ty() != SymbolType::Newtype {
            return Ok(false);
        }

        // resolve the underlying newtype target to determine object shape
        let Some(target_ty_id) = self.alias_target_type_id_for_symbol(
            module,
            profile,
            *symbol,
            pattern_id.into_any(),
            symbols,
            types,
        ) else {
            return Ok(false);
        };
        let target_ty_id =
            self.ensure_type_evaluated(module, profile, target_ty_id, tree, symbols, types)?;

        let target_ty = types.get_type(target_ty_id);
        let is_object = matches!(target_ty, Type::Object { .. });
        let is_struct_ref = matches!(
            target_ty,
            Type::Reference { symbol, .. } if symbol.ty() == SymbolType::Struct
        );
        Ok(is_object || is_struct_ref)
    }

    /// Infer a sequence of pattern fields (with spread syntax support).
    pub(super) fn infer_pattern_sequence(
        &self,
        module: &Module,
        fields: &Vec<LocalNodeId<PatternField>>,
        binding_ty_id: Option<LocalTypeId>,
        to_rest_type: impl Fn(Vec<LocalTypeId>) -> Type,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // map the binding type into sequence-friendly pieces
        let (binding_ty_fields, binding_array_element) = binding_ty_id
            .map(|ty_id| match types.get_type(ty_id) {
                Type::Tuple { elements, .. } => {
                    (elements.iter().map(|element| element.ty).collect(), None)
                }
                Type::ArraySized { element, .. } => (Vec::new(), Some(*element)),
                Type::Array { element, .. } => (Vec::new(), *element),
                _ => (Vec::new(), None),
            })
            .unwrap_or_else(|| (Vec::new(), None));

        // allow a direct fallback only for a single field on non-sequence types
        let allow_direct_binding_fallback = binding_ty_id.is_some()
            && binding_ty_fields.is_empty()
            && binding_array_element.is_none()
            && fields.len() == 1
            && !matches!(
                tree.get(fields[0]),
                PatternField::Spread { .. } | PatternField::Elision
            );
        let direct_binding_fallback = allow_direct_binding_fallback
            .then_some(binding_ty_id)
            .flatten();

        // nothing to infer when the sequence has no fields
        if fields.is_empty() {
            return Ok(());
        }

        // reject non-sequence bindings that cannot use the guarded fallback
        if let Some(binding_ty_id) = binding_ty_id
            && binding_ty_fields.is_empty()
            && binding_array_element.is_none()
            && !allow_direct_binding_fallback
        {
            let first_field_id = fields[0];
            let unknown_ty = Type::TypeLiteral {
                value: TypeLiteral::Unknown,
            };
            let unknown_ty_id = types.insert_type_from(unknown_ty, first_field_id);
            let actual_elements = fields
                .iter()
                .map(|_| TypeElement::new(unknown_ty_id))
                .collect();
            let actual_ty = Type::Tuple {
                elements: actual_elements,
                is_readonly: false,
            };
            let actual_ty_id = types.insert_type_from(actual_ty, first_field_id);
            let error_node = first_field_id
                .into_global_any(module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::UnassignableType {
                node: error_node,
                expected_ty: binding_ty_id.into_global(module.id),
                actual_ty: actual_ty_id.into_global(module.id),
            });
        }

        let spread_len = binding_ty_fields.len().saturating_sub(fields.len() - 1);
        let mut ty_idx = 0;
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_ty = match field {
                PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Positional { .. }
                | PatternField::Computed { .. } => {
                    if !binding_ty_fields.is_empty() {
                        let ty = binding_ty_fields.get(ty_idx).cloned();
                        ty_idx += 1;
                        ty
                    } else if let Some(element_ty_id) = binding_array_element {
                        Some(element_ty_id)
                    } else {
                        direct_binding_fallback
                    }
                }
                PatternField::Spread { .. } => {
                    if !binding_ty_fields.is_empty() {
                        let rest_types = binding_ty_fields
                            .get(ty_idx..ty_idx + spread_len)
                            .map(|s| s.to_vec())
                            .unwrap_or_default();
                        ty_idx += spread_len;
                        let rest_ty = to_rest_type(rest_types);
                        Some(types.insert_type_from(rest_ty, *field_id))
                    } else if let Some(element_ty_id) = binding_array_element {
                        let rest_ty = to_rest_type(vec![element_ty_id]);
                        Some(types.insert_type_from(rest_ty, *field_id))
                    } else {
                        None
                    }
                }
                PatternField::Elision => {
                    // elision skips a type position
                    if !binding_ty_fields.is_empty() {
                        ty_idx += 1;
                    }
                    None
                }
            };
            self.infer_pattern_field(
                module, *field_id, field_ty, tree, symbols, types, infer, ctx,
            )?;
        }
        Ok(())
    }

    /// Infer a pattern field and propagate type to bound symbol.
    pub(super) fn infer_pattern_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        match field {
            PatternField::Named {
                mutability: _,
                name,
                default: _,
                symbol,
                pattern,
            } => {
                // resolve the field type from the binding type when possible
                let field_ty_id = self.pattern_field_binding_type(
                    module,
                    field_id,
                    binding_ty_id,
                    StaticKey::Name(*name),
                    types,
                    ctx,
                )?;
                if let Some(ty_id) = field_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }

                // propagate the field type into nested patterns
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        field_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Computed {
                key,
                pattern,
                default,
                ..
            } => {
                self.infer_expression(module, *key, tree, symbols, types, infer, ctx)?;
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
                if let Some(pattern_id) = pattern {
                    self.infer_pattern(
                        module,
                        *pattern_id,
                        None,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            PatternField::Alias {
                mutability: _,
                name,
                alias: _,
                default,
                symbol,
            } => {
                // resolve the field type from the binding type when possible
                let field_ty_id = self.pattern_field_binding_type(
                    module,
                    field_id,
                    binding_ty_id,
                    StaticKey::Name(*name),
                    types,
                    ctx,
                )?;
                if let Some(ty_id) = field_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }

                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }
            }
            PatternField::Positional { pattern } => {
                self.infer_pattern(
                    module,
                    *pattern,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            PatternField::Spread {
                mutability: _,
                name: _,
                symbol,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
            }
            PatternField::Elision => {
                // elision doesn't bind anything
            }
        }
        Ok(())
    }

    /// Resolve the binding type for a named pattern field.
    fn pattern_field_binding_type(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        field_key: StaticKey,
        types: &mut TypeTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        // skip when there is no binding type to inspect
        let Some(binding_ty_id) = binding_ty_id else {
            return Ok(None);
        };

        // resolve field types when the binding type is an object or reference
        let receiver_ty = types.get_type(binding_ty_id).clone();
        let symbols = module.dir(ctx.profile).symbols.read();
        let mut visited = Vec::new();
        let field_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            field_id.into_any(),
            &symbols,
            &receiver_ty,
            &field_key,
            MemberLookupMode::Any,
            types,
            &mut visited,
        )?;

        // allow match patterns to bind fields from matching union variants
        let match_union_field =
            if field_ty_id.is_none() && (ctx.in_match.is_some() || ctx.in_switch.is_some()) {
                let union_receiver_ty = match receiver_ty {
                    Type::Reference { symbol, .. } => self
                        .apparent_instance_type(
                            module,
                            ctx.profile,
                            field_id.into_any(),
                            symbol,
                            &symbols,
                            types,
                        )
                        .map(|type_id| types.get_type(type_id).clone())
                        .unwrap_or(receiver_ty.clone()),
                    Type::Value { value } => types.get_type(value).clone(),
                    _ => receiver_ty.clone(),
                };
                if let Type::Union { elements } = union_receiver_ty {
                    let mut field_types = Vec::new();
                    for element_id in elements {
                        let element_ty = types.get_type(element_id).clone();
                        if let Some(field_ty) = self.infer_member_of_type(
                            module,
                            ctx.profile,
                            field_id.into_any(),
                            &symbols,
                            &element_ty,
                            &field_key,
                            MemberLookupMode::Any,
                            types,
                            &mut visited,
                        )? {
                            field_types.push(field_ty);
                        }
                    }
                    if field_types.is_empty() {
                        None
                    } else if field_types.len() == 1 {
                        Some(field_types[0])
                    } else {
                        Some(self.union_type_from_list(field_types, binding_ty_id, types))
                    }
                } else {
                    None
                }
            } else {
                None
            };

        // fall back to the binding type for non-object patterns
        Ok(Some(
            match_union_field.unwrap_or_else(|| field_ty_id.unwrap_or(binding_ty_id)),
        ))
    }

    /// Resolve a tagged pattern target type from an expression.
    fn evaluate_pattern_tag_type(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // evaluate the tag expression as a type
        let ty_id = self.try_evaluate_expression_to_type(
            module,
            ctx.profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
        )?;

        // unwrap type-as-value wrappers when present
        let ty_id = match types.get_type(ty_id) {
            Type::Value { value } => *value,
            _ => ty_id,
        };

        // return non-reference tag types directly
        let Type::Reference {
            symbol,
            static_arguments,
        } = types.get_type(ty_id).clone()
        else {
            return Ok(ty_id);
        };

        // skip remote symbols
        if symbol.module_id != module.id {
            return Ok(ty_id);
        }

        // skip non-newtype symbols
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        if symbol_entry.ty != SymbolType::Newtype {
            return Ok(ty_id);
        }

        // load the local nominal declaration for the tag
        let Some(primary_declaration) = symbol_entry.primary_declaration else {
            return Ok(ty_id);
        };
        if primary_declaration.module_id != module.id {
            return Ok(ty_id);
        }
        let Ok(declaration_id) = primary_declaration.try_into_typed::<Declaration>() else {
            return Ok(ty_id);
        };
        let declaration_id: LocalNodeId<Declaration> = declaration_id.into();
        let Declaration::Type {
            kind: TypeKind::Nominal,
            value,
            ..
        } = tree.get(declaration_id)
        else {
            return Ok(ty_id);
        };

        // resolve the declared type for the nominal alias
        let value_id = value.into_global_any(module.id);
        let Some(declared_ty_id) = types.get_declared_type_id(value_id) else {
            return Ok(ty_id);
        };

        // evaluate unevaluated declared types
        if matches!(types.get_type(declared_ty_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, ctx.profile, declared_ty_id, tree, symbols, types)?;
        }

        let mut declared_ty_id = declared_ty_id;

        // apply static arguments when provided
        if let Some(static_arguments) = static_arguments {
            let source_id = types.get_type_source(ty_id);
            let resolved_arguments = self.resolve_type_reference_static_arguments(
                module,
                ctx.profile,
                source_id,
                symbol,
                Some(static_arguments.as_slice()),
                true,
                &ctx.options,
                tree,
                symbols,
                types,
            )?;
            if let Some(resolved_arguments) = resolved_arguments
                && !resolved_arguments.is_empty()
            {
                let substitutions = self.build_type_parameter_substitutions_for_symbol(
                    module,
                    ctx.profile,
                    symbol,
                    source_id,
                    &resolved_arguments,
                    tree,
                    symbols,
                    types,
                );
                if !substitutions.is_empty() {
                    let mut cache = HashMap::new();
                    declared_ty_id = self.substitute_static_parameters(
                        declared_ty_id,
                        &substitutions,
                        types,
                        &mut cache,
                    );
                }
            }
        }

        Ok(declared_ty_id)
    }

    /// Get the target symbol for a reference expression.
    pub(crate) fn reference_symbol_for_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        match tree.get(expression_id) {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => Some(self.canonical_symbol_id(
                module,
                symbols,
                profile,
                *target_symbol,
                CanonicalSymbolMode::FollowAliases,
            )),
            _ => None,
        }
    }

    /// Peel nested parenthesized expressions to the underlying expression.
    pub(crate) fn unwrap_parenthesized_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> LocalNodeId<Expression> {
        loop {
            let Expression::Parenthesized { expression } = tree.get(expression_id) else {
                break;
            };

            expression_id = *expression;
        }

        expression_id
    }

    /// Return true when a cast operator is an unsafe type assertion.
    fn is_unsafe_type_assertion(&self, operator: CastOperator) -> bool {
        matches!(
            operator,
            CastOperator::AnyDowncast
                | CastOperator::UnknownDowncast
                | CastOperator::ObjectDowncast
                | CastOperator::InstanceDowncast
                | CastOperator::UnionDowncast
                | CastOperator::NullableDowncast
                | CastOperator::PointerCast
                | CastOperator::PointerToInt
                | CastOperator::IntToPointer
        )
    }

    /// Resolve a global symbol name across module boundaries.
    pub(super) fn symbol_name_for_global(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        symbols: &SymbolTable,
    ) -> Option<StringId> {
        if symbol.module_id == module.id {
            return symbols.get_symbol(symbol.local_id).name();
        }

        let remote_module = self.program.modules.get(symbol.module_id);
        let remote_module = remote_module.read();
        let remote_symbols = remote_module.dir(profile).symbols.read();
        remote_symbols.get_symbol(symbol.local_id).name()
    }

    /// Resolve the dependency item that introduced a symbol when possible.
    pub(crate) fn dependency_item_for_symbol(
        &self,
        tree: &NodeTree,
        primary_declaration: GlobalNodeIdAny,
        symbol_id: LocalSymbolId,
    ) -> Option<LocalNodeId<DependencyItem>> {
        // return dependency items directly
        if primary_declaration.local_id.ty == NodeType::DependencyItem {
            return Some(primary_declaration.local_id.into_typed());
        }

        // scan import/export expressions for matching dependency items
        if primary_declaration.local_id.ty != NodeType::Expression {
            return None;
        }
        let expression_id = primary_declaration.local_id.into_typed::<Expression>();
        let items = match tree.get(expression_id) {
            Expression::Import { items, .. }
            | Expression::ReExport { items, .. }
            | Expression::Export { items, .. } => items,
            _ => return None,
        };

        for item_id in items {
            if tree.get(*item_id).symbol() == Some(symbol_id) {
                return Some(*item_id);
            }
        }

        None
    }

    /// Check whether a remote export is type-only for a specific name.
    fn is_type_only_export_name(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        export_name: StringId,
    ) -> bool {
        let remote_module = self.program.modules.get(module_id);
        let remote_module = remote_module.read();
        let remote_exports = remote_module.dir(profile).exported_symbols.read();
        let key = StaticKey::Name(export_name);
        let has_value = remote_exports.contains_key(&(SymbolSpace::Value, key));
        let has_type = remote_exports.contains_key(&(SymbolSpace::Type, key));
        has_type && !has_value
    }

    /// Emit a type-only value error and return an error type id.
    fn type_only_value_error_type(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        types: &mut TypeTable,
    ) -> LocalTypeId {
        self.error(AnalyzeError::TypeOnlyValue {
            node: expression_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
        });
        types.insert_type_from(Type::Error, expression_id)
    }

    /// Resolve export name and module id for a remote dependency item.
    fn remote_dependency_export_target(
        &self,
        dependency: &DependencyItem,
    ) -> (Option<StringId>, Option<ModuleId>) {
        match dependency {
            DependencyItem::Remote {
                mode,
                name,
                target_module,
                ..
            } => {
                let default_name = self.program.strings.intern("default");
                let export_name = match mode {
                    DependencyMode::Item => name.map(|name| name.string()),
                    DependencyMode::Default => {
                        name.map(|name| name.string()).or(Some(default_name))
                    }
                    DependencyMode::Namespace => None,
                };
                (export_name, target_module.module_id())
            }
            DependencyItem::UnresolvedRemote {
                mode,
                name,
                target_module,
                ..
            } => {
                let default_name = self.program.strings.intern("default");
                let export_name = match mode {
                    DependencyMode::Item => name.map(|name| name.string()),
                    DependencyMode::Default => {
                        name.map(|name| name.string()).or(Some(default_name))
                    }
                    DependencyMode::Namespace => None,
                };
                (
                    export_name,
                    target_module.and_then(|target| target.module_id()),
                )
            }
            _ => (None, None),
        }
    }

    /// Infer a reference expression (local, module, or global).
    pub(super) fn infer_reference_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        static_arguments: Option<&[LocalNodeId<Argument>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        let _timing = self.timing_scope(tags::ANALYZE_INFER_EXPRESSION_REFERENCE);
        let mut allow_type_only_reference = false;

        // validate local references against type-only exports
        if target_symbol.module_id == module.id {
            let symbol_entry = symbols.get_symbol(target_symbol.local_id);
            let dependency_id = symbol_entry
                .primary_declaration
                .and_then(|primary_declaration| {
                    self.dependency_item_for_symbol(
                        tree,
                        primary_declaration,
                        target_symbol.local_id,
                    )
                });
            allow_type_only_reference =
                module.language_type.is_declaration() && dependency_id.is_some();

            // reject type-only symbols used as values
            if !self.symbol_is_value_capable(ctx.profile, target_symbol)
                && !allow_type_only_reference
            {
                let ty_id =
                    self.type_only_value_error_type(module, ctx.profile, expression_id, types);
                return Ok(ty_id);
            }

            // reject aliases that resolve to type-only symbols
            if let Some(target) = symbol_entry.target_symbol
                && !self.symbol_is_value_capable(ctx.profile, target)
                && !allow_type_only_reference
            {
                let ty_id =
                    self.type_only_value_error_type(module, ctx.profile, expression_id, types);
                return Ok(ty_id);
            }

            // resolve the dependency item that introduced this symbol
            if let Some(dependency_id) = dependency_id {
                let dependency = tree.get(dependency_id);
                let dependency_kind = match dependency {
                    DependencyItem::Local { kind, .. }
                    | DependencyItem::Remote { kind, .. }
                    | DependencyItem::UnresolvedLocal { kind, .. }
                    | DependencyItem::UnresolvedRemote { kind, .. } => Some(*kind),
                    DependencyItem::Value { .. } => None,
                };

                // reject type-only dependencies used in value positions
                let is_type_only_dependency = match dependency_kind {
                    Some(DependencyKind::Type) => true,
                    Some(DependencyKind::Value) => dependency
                        .target_symbol()
                        .is_some_and(|target| !self.symbol_is_value_capable(ctx.profile, target)),
                    None => false,
                };
                if is_type_only_dependency && !allow_type_only_reference {
                    let ty_id =
                        self.type_only_value_error_type(module, ctx.profile, expression_id, types);
                    return Ok(ty_id);
                }

                // reject export inference cycles without explicit annotations
                if ctx.is_surface_inference
                    && dependency_kind == Some(DependencyKind::Value)
                    && let Some(target_symbol) = dependency.target_symbol()
                {
                    let has_cycle = self.export_inference_has_cycle(
                        module.id,
                        ctx.profile,
                        target_symbol.module_id,
                    )?;
                    if has_cycle
                        && !self.remote_symbol_has_declared_value_type(ctx.profile, target_symbol)
                    {
                        let error_node = expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::ExportInferenceRequiresAnnotation {
                            node: error_node,
                        });
                        let ty_id =
                            types.insert_type_from_any(Type::Error, expression_id.into_any());
                        return Ok(ty_id);
                    }
                }

                // reject value imports that resolve to type-only exports
                if dependency_kind == Some(DependencyKind::Value) {
                    let (export_name, target_module_id) =
                        self.remote_dependency_export_target(dependency);

                    if let Some(export_name) = export_name
                        && let Some(target_module_id) = target_module_id
                        && self.is_type_only_export_name(target_module_id, ctx.profile, export_name)
                        && !allow_type_only_reference
                    {
                        let is_value_capable = dependency.target_symbol().is_some_and(|target| {
                            self.symbol_is_value_capable(ctx.profile, target)
                        });
                        if !is_value_capable {
                            let ty_id = self.type_only_value_error_type(
                                module,
                                ctx.profile,
                                expression_id,
                                types,
                            );
                            return Ok(ty_id);
                        }
                    }
                }
            }

            // reject export inference cycles when dependency items are unavailable
            if dependency_id.is_none()
                && ctx.is_surface_inference
                && let Some(target_symbol) = symbol_entry.target_symbol
            {
                let has_cycle = self.export_inference_has_cycle(
                    module.id,
                    ctx.profile,
                    target_symbol.module_id,
                )?;
                if has_cycle
                    && !self.remote_symbol_has_declared_value_type(ctx.profile, target_symbol)
                {
                    let error_node = expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::ExportInferenceRequiresAnnotation {
                        node: error_node,
                    });
                    let ty_id = types.insert_type_from_any(Type::Error, expression_id.into_any());
                    return Ok(ty_id);
                }
            }

            // fall back to export tables when dependency items are missing
            if dependency_id.is_none()
                && let Some(target_symbol) = symbol_entry.target_symbol
                && let Some(StaticKey::Name(name)) = symbol_entry.key
                && self.is_type_only_export_name(target_symbol.module_id, ctx.profile, name)
                && !allow_type_only_reference
            {
                let ty_id =
                    self.type_only_value_error_type(module, ctx.profile, expression_id, types);
                return Ok(ty_id);
            }
        }

        // resolve the canonical symbol for imported references
        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            ctx.profile,
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

        // reject type-only symbols in value positions
        if !self.symbol_is_value_capable(ctx.profile, canonical_symbol)
            && !allow_type_only_reference
        {
            let ty_id = self.type_only_value_error_type(module, ctx.profile, expression_id, types);
            return Ok(ty_id);
        }

        // reject globalThis references when configured
        if ctx.options.no_global_this && matches!(module.source, ModuleSource::User) {
            let global_this_name = self.program.strings.intern("globalThis");
            if self.symbol_name_for_global(module, ctx.profile, canonical_symbol, symbols)
                == Some(global_this_name)
            {
                self.error(AnalyzeError::GlobalThisDisabled {
                    node: expression_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }
        }

        // synthesize a globalThis object type on demand
        let global_this_name = self.program.strings.intern("globalThis");
        let is_global_this =
            self.symbol_name_for_global(module, ctx.profile, canonical_symbol, symbols)
                == Some(global_this_name);

        // pick the base type for the symbol by applying narrowing and inference
        let base_ty_id = if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
            narrowed_ty_id
        } else if is_global_this
            && let Some(global_this_ty_id) = self.infer_global_this_value_type(
                module,
                expression_id,
                canonical_symbol,
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?
        {
            global_this_ty_id
        } else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
            value_ty_id
        } else if let Some(inferred_ty_id) = self.infer_direct_binding_value_type(
            module,
            ctx.profile,
            canonical_symbol,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )? {
            inferred_ty_id
        } else if canonical_symbol.module_id != module.id {
            self.resolve_remote_symbol_value_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                canonical_symbol,
                ctx.is_surface_inference,
                types,
            )?
        } else {
            // local symbol without type: use InferVar for forward references
            let scope = InferScope {
                owner: canonical_symbol,
                function_id: ctx.in_function.map(|f| f.into_global(module.id)),
            };
            self.infer_var_type_for_symbol(
                infer,
                types,
                canonical_symbol,
                expression_id.into_any(),
                InferOrigin::Expression(expression_id.into_global_any(module.id)),
                scope,
            )
        };

        // evaluate local unevaluated types before use
        if canonical_symbol.module_id == module.id
            && matches!(types.get_type(base_ty_id), Type::Unevaluated(_))
        {
            self.evaluate_type(module, ctx.profile, base_ty_id, tree, symbols, types)?;
        }

        // ensure instance types for referenced symbols
        self.ensure_reference_instance_types_for_type(
            module,
            ctx.profile,
            expression_id.into_any(),
            base_ty_id,
            types,
        )?;

        // handle static arguments for generic instantiation
        let Some(static_argument_ids) = static_arguments else {
            return Ok(base_ty_id);
        };

        // resolve a callable signature for generic instantiation
        let Some(signature_ty_id) = self.call_signature_for_type(base_ty_id, types) else {
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(base_ty_id);
        };

        let Type::Function {
            asynchrony,
            cardinality,
            mut static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = types.get_type(signature_ty_id).clone()
        else {
            self.error(AnalyzeError::MissingType {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(base_ty_id);
        };

        // recover static parameters when they are missing from the signature type
        if static_parameters.is_empty() {
            static_parameters =
                self.recover_static_parameters_for_signature(module, signature_ty_id, tree, types);
        }

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            Some(canonical_symbol),
            Some(static_argument_ids),
            None,
            &static_parameters,
            &dynamic_parameters,
            return_type,
            super::SignatureResolutionMode::Checking,
            false,
            ctx.profile,
            &ctx.options,
            tree,
            symbols,
            types,
            infer,
        )?;

        let instantiated_fn = Type::Function {
            asynchrony,
            cardinality,
            static_parameters: Vec::new(),
            this_parameter,
            dynamic_parameters: resolved.dynamic_parameters,
            return_type: resolved.return_type,
        };
        let instantiated_ty_id = types.insert_type_from(instantiated_fn, expression_id);

        if !resolved.static_arguments.is_empty() {
            self.register_instance_for_node(
                expression_id.into_global_any(module.id),
                canonical_symbol,
                resolved.static_arguments,
                types,
            );
        }

        Ok(instantiated_ty_id)
    }

    /// Check excess properties on an object literal against a contextual type.
    fn check_excess_object_literal_properties(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: Option<LocalTypeId>,
        fields: &[ObjectLiteralField],
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // collect candidates for excess property checks
        // bail when no contextual type is available
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, types) else {
            return Ok(());
        };
        let mut candidates: Vec<LocalTypeId> = Vec::new();
        let mut visited = HashSet::new();
        self.collect_object_literal_candidates(
            module,
            profile,
            node_id,
            expected_ty_id,
            options,
            tree,
            symbols,
            types,
            &mut candidates,
            &mut visited,
        )?;
        if candidates.is_empty() {
            return Ok(());
        }

        // check if any candidate matches the fields
        for candidate in candidates.iter().copied() {
            if self.object_literal_matches_target(fields, candidate, types) {
                return Ok(());
            }
        }

        // if no candidate matches, report the first excess property
        let Some(candidate) = candidates.first().copied() else {
            return Ok(());
        };
        let excess_fields = self.object_literal_excess_properties(fields, candidate, types);
        if excess_fields.is_empty() {
            return Ok(());
        }

        // emit excess property diagnostics
        for (property_id, member_key) in excess_fields {
            self.error(AnalyzeError::ExcessProperty {
                node: property_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                expected_ty: candidate.into_global(module.id),
                member_key,
            });
        }

        Ok(())
    }

    /// Collect object style candidates for excess property checks.
    fn collect_object_literal_candidates(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        expected_ty_id: LocalTypeId,
        options: &AnalyzeOptions,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        candidates: &mut Vec<LocalTypeId>,
        visited: &mut HashSet<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // avoid recursive candidate discovery loops
        if !visited.insert(expected_ty_id) {
            return Ok(());
        }

        match types.get_type(expected_ty_id).clone() {
            Type::Object { .. } => {
                candidates.push(expected_ty_id);
            }
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                let instance_ty_id =
                    self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;
                if let Some(instance_ty_id) = instance_ty_id {
                    let mut candidate_id = instance_ty_id;

                    // specialize instance types with explicit static arguments
                    if let Some(static_arguments) = static_arguments.as_ref()
                        && let Some(resolved_arguments) = self
                            .resolve_type_reference_static_arguments(
                                module,
                                profile,
                                node_id,
                                symbol,
                                Some(static_arguments.as_slice()),
                                true,
                                options,
                                tree,
                                symbols,
                                types,
                            )?
                        && !resolved_arguments.is_empty()
                    {
                        let substitutions = self.build_type_parameter_substitutions_for_symbol(
                            module,
                            profile,
                            symbol,
                            node_id,
                            &resolved_arguments,
                            tree,
                            symbols,
                            types,
                        );
                        if !substitutions.is_empty() {
                            let mut cache = HashMap::new();
                            candidate_id = self.substitute_static_parameters(
                                instance_ty_id,
                                &substitutions,
                                types,
                                &mut cache,
                            );
                        }
                    }

                    let normalized = self.normalize_type(
                        module,
                        profile,
                        candidate_id,
                        symbols,
                        types,
                        NormalizationMode::Assign,
                    );
                    candidates.push(normalized);
                }
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                // expand mapped types into object candidates
                let source_id = types.get_type_source(expected_ty_id);
                let mut normalize_visited = Vec::new();
                let normalized = self.normalize_mapped_type(
                    module,
                    profile,
                    source_id,
                    parameter,
                    modifiers,
                    value,
                    symbols,
                    types,
                    NormalizationMode::Assign,
                    RelationMode::TYPE_OPS,
                    &mut normalize_visited,
                );
                self.collect_object_literal_candidates(
                    module, profile, node_id, normalized, options, tree, symbols, types,
                    candidates, visited,
                )?;
            }
            Type::Union { elements } => {
                for element in elements {
                    self.collect_object_literal_candidates(
                        module, profile, node_id, element, options, tree, symbols, types,
                        candidates, visited,
                    )?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Check if an object literal matches a target object type.
    fn object_literal_matches_target(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return false;
        };
        if !index_signatures.is_empty() {
            return true;
        }

        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                return false;
            }
        }

        true
    }

    /// Find the first excess property key for a target object type.
    fn object_literal_excess_properties(
        &self,
        fields: &[ObjectLiteralField],
        target_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> Vec<(LocalNodeId<Property>, StaticKey)> {
        let Type::Object {
            fields: target_fields,
            index_signatures,
            ..
        } = types.get_type(target_ty_id)
        else {
            return Vec::new();
        };
        if !index_signatures.is_empty() {
            return Vec::new();
        }

        let mut excess_fields = Vec::new();
        for field in fields {
            let matches = target_fields
                .iter()
                .any(|target_field| target_field.key.matches(&field.field.key));
            if !matches {
                excess_fields.push((field.property_id, field.field.key));
            }
        }

        excess_fields
    }

    /// Validate that a comptime body is static when required.
    /// Infer a return expression and constrain it to the function return type.
    fn infer_return_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        value: Option<LocalNodeId<Expression>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // validate return position
        if !ctx.can_return() {
            self.error(AnalyzeError::InvalidReturn {
                node: expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // infer the return value when present
        if let Some(value_id) = value {
            let mut return_ctx = ctx.fork().with_expected_type(ctx.return_type);
            let value_ty_id = self.infer_expression(
                module,
                value_id,
                tree,
                symbols,
                types,
                infer,
                &mut return_ctx,
            )?;

            if let Some(return_ty_id) = ctx.return_type {
                // enforce explicit ownership when implicit managed values are disabled
                self.check_no_implicit_managed_value(
                    module,
                    ctx.profile,
                    value_id,
                    return_ty_id,
                    value_ty_id,
                    tree,
                    types,
                    &ctx.options,
                );

                // constrain the return value to the declared return type
                self.constrain_return_value_type(
                    module,
                    ctx.profile,
                    value_id,
                    return_ty_id,
                    value_ty_id,
                    symbols,
                    types,
                    infer,
                    &ctx.options,
                    ctx.is_async,
                );
            }
        } else if let Some(return_ty_id) = ctx.return_type {
            let void_ty_id = types.insert_type_from_any(
                Type::TypeLiteral {
                    value: TypeLiteral::Void,
                },
                expression_id.into_any(),
            );
            infer.push_constraint(Constraint::Subtype {
                sub_type: void_ty_id,
                super_type: return_ty_id,
                variance: None,
            });
        }

        // return expressions always end control flow
        let ty = Type::TypeLiteral {
            value: TypeLiteral::Never,
        };
        Ok(types.insert_type_from(ty, expression_id))
    }

    /// Constrain a return value to the declared return type.
    fn constrain_return_value_type(
        &self,
        module: &Module,
        profile: ProfileId,
        value_id: LocalNodeId<Expression>,
        return_ty_id: LocalTypeId,
        value_ty_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
        is_async: bool,
    ) {
        // unwrap awaited values for async returns
        let (check_value_ty_id, check_return_ty_id) = if is_async {
            (
                self.unwrap_awaited_type(module, symbols, profile, value_ty_id, types),
                self.unwrap_awaited_type(module, symbols, profile, return_ty_id, types),
            )
        } else {
            (value_ty_id, return_ty_id)
        };

        // detect return types that should skip assignability errors
        let mut static_visited = HashSet::new();
        let has_static_parameters = self.type_contains_static_parameters(
            module,
            profile,
            check_return_ty_id,
            symbols,
            types,
            &mut static_visited,
        );
        let mut infer_visited = HashSet::new();
        let has_infer_vars =
            self.type_contains_infer_vars(check_return_ty_id, types, &mut infer_visited);
        let allows_fallthrough =
            self.return_type_allows_fallthrough_infer(check_return_ty_id, types);

        // emit the return type error when the assignment is invalid
        if !has_static_parameters
            && !has_infer_vars
            && !allows_fallthrough
            && self.is_type_assignable(
                module,
                profile,
                symbols,
                check_return_ty_id,
                check_value_ty_id,
                types,
                options,
            ) == Assignability::NotAssignable
        {
            self.error(AnalyzeError::UnassignableType {
                node: value_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile)),
                expected_ty: check_return_ty_id.into_global(module.id),
                actual_ty: check_value_ty_id.into_global(module.id),
            });
        }

        infer.push_constraint(Constraint::Subtype {
            sub_type: check_value_ty_id,
            super_type: check_return_ty_id,
            variance: None,
        });
    }

    fn warn_ignored_return_value(
        &self,
        module: &Module,
        profile: ProfileId,
        statement_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) {
        // only warn for call like statements
        let statement = tree.get(statement_id);
        if !matches!(statement, Expression::Call { .. } | Expression::New { .. }) {
            return;
        }

        // look up the resolution for the statement
        let node_id = statement_id.into_global_any(module.id);
        let Some(resolution_id) = types.get_resolution_for_node(node_id) else {
            return;
        };
        let resolution = types.get_resolution(resolution_id);

        // check for mustUse targets
        let mut should_warn = false;
        for symbol_id in self.resolution_target_symbols(resolution) {
            let Some(decorators) = self.symbol_decorators_for(module, profile, symbols, symbol_id)
            else {
                continue;
            };
            if decorators.is_must_use {
                should_warn = true;
                break;
            }
        }
        if !should_warn {
            return;
        }

        // emit ignored return value warning
        self.warning(AnalyzeWarning::IgnoredReturnValue {
            node: node_id.into_anchored(Some(profile)),
        });
    }

    fn resolution_target_symbols(&self, resolution: &Resolution) -> Vec<GlobalSymbolId> {
        // collect target symbols for static or dynamic resolutions
        match resolution {
            Resolution::Static { candidate, .. } => vec![candidate.target_symbol],
            Resolution::Dynamic { candidates, .. } => candidates
                .iter()
                .map(|candidate| candidate.target_symbol)
                .collect(),
            Resolution::Unresolved { .. } | Resolution::Builtin { .. } => Vec::new(),
        }
    }

    fn symbol_decorators_for(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        symbol_id: GlobalSymbolId,
    ) -> Option<SymbolDecorators> {
        // read local symbol decorators
        if symbol_id.module_id == module.id {
            let symbol = symbols.get_symbol(symbol_id.local_id);
            return Some(symbol.decorators.clone());
        }

        // ensure the target module is resolved
        if self
            .require_resolve_module_direct(symbol_id.module_id, profile)
            .is_err()
        {
            return None;
        }

        // read decorators from the target module dir
        let other_module = self.program.modules.get(symbol_id.module_id);
        let other_module = other_module.read();
        let dir = other_module.dir(profile);
        let other_symbols = dir.symbols.read();
        let symbol = other_symbols.get_symbol(symbol_id.local_id);

        Some(symbol.decorators.clone())
    }
}

/// Check whether an expression participates in implicit return typing.
pub(crate) fn has_implicit_return(expression_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
    // treat statement-like expressions as non-returning values
    match tree.get(expression_id) {
        Expression::Statement { .. }
        | Expression::Return { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. } => false,
        Expression::Block { block } => {
            // read the block expression list
            let block = tree.get(*block);

            let Some(last_expression_id) = block.expressions.last() else {
                return false;
            };

            // ignore statement-like trailing expressions
            !matches!(
                tree.get(*last_expression_id),
                Expression::Statement { .. }
                    | Expression::Return { .. }
                    | Expression::Break { .. }
                    | Expression::Continue { .. }
            )
        }
        _ => true,
    }
}
