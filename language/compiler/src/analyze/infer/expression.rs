use std::collections::HashMap;
use std::sync::Arc;

use super::declaration::DeclaratorConstraint;
use super::member::MemberLookupMode;

use crate::analyze::common::CanonicalSymbolMode;
use crate::{
    AnalyzeError, AnalyzeResult, AnalyzeWarning, Assignability, BreakTargetKind, Compiler,
    FlowContext, InferContext,
};
use destack_dir::{
    Argument, BindingKind, Block, CastOperator, CastSource, Constraint, Declaration, Declarator,
    DependencySource, DynamicKey, Expression, FlowGraphBuilder, ForEachBinding, FunctionKind,
    GlobalSymbolId, IfCondition, InferOrigin, InferScope, InferTable, LocalNodeId, LocalNodeIdAny,
    LocalSymbolId, LocalTypeId, MatchCase, MatchKind, MatchSelector, MatchSource, Mutability,
    NodeTree, NodeType, Pattern, PatternField, PrimitiveType, Property, Resolution, StaticKey,
    StringId, SymbolDecorators, SymbolSpace, SymbolTable, SymbolType, Type, TypeElement, TypeField,
    TypeKind, TypeLiteral, TypeTable,
};
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

    /// Resolve a direct binding declarator for a symbol.
    fn direct_binding_declarator_for_symbol(
        &self,
        module: &Module,
        symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<LocalNodeId<Declarator>> {
        // only local bindings can use local declaration data
        if symbol.module_id != module.id {
            return None;
        }

        // read the primary declaration for the symbol
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let primary_declaration = symbol_entry.primary_declaration?;

        // ensure the declaration is local to the module
        if primary_declaration.module_id != module.id {
            return None;
        }

        // require a direct binding for the primary declaration
        if !self.primary_declaration_is_direct_binding(
            primary_declaration.local_id,
            symbol.local_id,
            tree,
        ) {
            return None;
        }

        // walk up to find the declarator containing the binding
        if let Some(declarator_id) =
            self.declarator_parent_for_node(primary_declaration.local_id, tree)
        {
            return Some(declarator_id);
        }

        // scan let or using expressions for a matching direct binding
        self.direct_binding_declarator_in_expression(
            primary_declaration.local_id,
            symbol.local_id,
            tree,
        )
    }

    /// Return true when the primary declaration is a direct binding.
    fn primary_declaration_is_direct_binding(
        &self,
        declaration_id: LocalNodeIdAny,
        symbol: LocalSymbolId,
        tree: &NodeTree,
    ) -> bool {
        // accept direct binding patterns with no destructuring
        if declaration_id.ty == NodeType::Pattern {
            let pattern_id = declaration_id.into_typed::<Pattern>();
            let Pattern::Binding {
                symbol: binding_symbol,
                pattern,
                ..
            } = tree.get(pattern_id)
            else {
                return false;
            };

            return *binding_symbol == symbol && pattern.is_none();
        }

        // reject pattern fields because they are not primary bindings
        if declaration_id.ty == NodeType::PatternField {
            return false;
        }

        true
    }

    /// Walk up the tree to find an enclosing declarator.
    fn declarator_parent_for_node(
        &self,
        node_id: LocalNodeIdAny,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<Declarator>> {
        // climb parents until a declarator is found
        let mut current = node_id;
        loop {
            if current.ty == NodeType::Declarator {
                return Some(current.into_typed());
            }
            let parent = tree.get_parent(current.id)?;
            current = parent;
        }
    }

    /// Find a direct binding declarator inside a let or using expression.
    fn direct_binding_declarator_in_expression(
        &self,
        declaration_id: LocalNodeIdAny,
        symbol: LocalSymbolId,
        tree: &NodeTree,
    ) -> Option<LocalNodeId<Declarator>> {
        // only expressions can contain declarator lists
        if declaration_id.ty != NodeType::Expression {
            return None;
        }

        // select declarator lists from let and using expressions
        let expression_id = declaration_id.into_typed::<Expression>();
        let declarators = match tree.get(expression_id) {
            Expression::Let { declarators, .. } => declarators.as_slice(),
            Expression::Using { declarators, .. } => declarators.as_slice(),
            _ => return None,
        };

        // find a matching direct binding declarator
        for declarator_id in declarators {
            let declarator = tree.get(*declarator_id);
            let Pattern::Binding {
                symbol: binding_symbol,
                pattern,
                ..
            } = tree.get(declarator.pattern)
            else {
                continue;
            };

            if *binding_symbol == symbol && pattern.is_none() {
                return Some(*declarator_id);
            }
        }

        None
    }

    /// Infer a direct binding value type when none is cached yet.
    fn infer_direct_binding_value_type(
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

        // infer the initializer with the placeholder installed
        let mut value_ctx = ctx.fork();
        let inferred_ty_id = self.infer_expression(
            module,
            value_id,
            tree,
            symbols,
            types,
            infer,
            &mut value_ctx,
        )?;
        types.set_value_type(symbol, inferred_ty_id);

        Ok(Some(inferred_ty_id))
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
        // build a flow graph and flow table for the function body
        let graph = FlowGraphBuilder::new(module.id, tree).build(body_id);
        let flow = self
            .compute_flow_table_for_graph(module, &graph, tree, symbols, types, infer, context)?;

        // seed the inference context with flow information
        let previous_flow = context.flow.clone();
        context.flow = Some(FlowContext {
            module_id: module.id,
            graph: Arc::new(graph),
            table: Arc::new(flow),
        });

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
        if !ctx.is_surface_inference
            && !has_flow
            && let Some(ty_id) =
                types.get_inferred_type_id(expression_id.into_global_any(module.id))
        {
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
                self.infer_declaration(module, *declaration, tree, symbols, types, infer, ctx)?;
                let declaration = tree.get(*declaration);

                // lambda declarations evaluate to function values
                if let Declaration::Function {
                    descriptor,
                    signature,
                    ..
                } = declaration
                {
                    if matches!(signature.kind, FunctionKind::Lambda) {
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
                mutability: _,
                declarators,
            } => {
                for decl_id in declarators {
                    self.infer_declarator(
                        module,
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
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
                    self.infer_declarator(
                        module,
                        *decl_id,
                        expression_id,
                        DeclaratorConstraint::Assignable,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // type operations
            Expression::TypeUnary { operator, right } => {
                let right_ty_id =
                    self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;

                let ty = self.infer_type_unary_operation(operator, right_ty_id, types);
                types.insert_type_from(ty, expression_id)
            }
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                let left_ty_id = match operator {
                    destack_dir::TypeBinaryOperator::Extends
                    | destack_dir::TypeBinaryOperator::Implements => self
                        .try_evaluate_expression_to_type(
                            module,
                            ctx.profile,
                            *left,
                            tree,
                            symbols,
                            types,
                            true,
                            true,
                        )?,
                    _ => self.infer_expression(module, *left, tree, symbols, types, infer, ctx)?,
                };
                let right_ty_id =
                    self.resolve_type_expression(module, ctx.profile, *right, tree, symbols, types, infer, ctx)?;

                let ty = self.infer_type_binary_operation(
                    module,
                    ctx.profile,
                    expression_id,
                    operator,
                    left_ty_id,
                    right_ty_id,
                    symbols,
                    types,
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

            // import meta: statically known type
            Expression::ImportMeta => {
                if module.source_type.is_script() {
                    self.error(AnalyzeError::InvalidImportMeta {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }
                // #Incomplete: type for import.meta
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // this: reference to the current instance item
            Expression::This => {
                let this_symbol = self.find_this_symbol(module, expression_id, tree, symbols);
                if let Some(this_symbol) = this_symbol
                    && let Some(ty_id) = types.get_value_type_id(this_symbol)
                {
                    ty_id
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
                if let Some(expected_ty_id) =
                    self.expected_type_for_scalar_literal(value, ctx.expected_type, types)
                {
                    expected_ty_id
                } else {
                    let ty = Type::TypeLiteral {
                        value: self.infer_scalar_literal(value),
                    };
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
                // resolve contextual type for array literals
                let expected_ty_id = self.expected_value_type(ctx.expected_type, types);
                let expected_is_tuple = expected_ty_id.is_some_and(|expected_ty_id| {
                    matches!(types.get_type(expected_ty_id), Type::Tuple { .. })
                });

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
                    && let Some(Type::Array { element }) = self.normalize_well_known_type_reference(
                        module,
                        symbols,
                        ctx.profile,
                        expression_id.into_any(),
                        symbol,
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

                // infer element types and collect their contextualized types
                let mut element_type_ids = Vec::with_capacity(elements.len());
                for (index, element_id) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(
                        module,
                        *element_id,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                    let element = tree.get(*element_id);
                    let value_id = element.value();
                    let ty_id = if let Some(ty_id) =
                        types.get_inferred_type_id(value_id.into_global_any(module.id))
                    {
                        ty_id
                    } else {
                        let mut element_ctx = ctx.fork().with_expected_type(expected_element_ty_id);
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
                    element_type_ids.push(ty_id);
                }

                // use tuple types when an expected tuple type exists
                let ty = if expected_is_tuple {
                    let mut tuple_elements = Vec::with_capacity(element_type_ids.len());
                    for (index, element_id) in elements.iter().enumerate() {
                        let argument = tree.get(*element_id);
                        let mut element = TypeElement::new(element_type_ids[index]);
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
                    }
                };

                let ty_id = types.insert_type_from(ty, expression_id);

                // reject managed array types when managed memory is disabled
                if ctx.options.no_managed
                    && !ctx.is_explicit_ownership
                    && matches!(module.source, ModuleSource::User)
                {
                    let value_ty = types.get_type(ty_id);
                    if self.type_contains_managed(module, ctx.profile, value_ty, types) {
                        self.error(AnalyzeError::ManagedMemoryDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                ty_id
            }

            // tuple expression: preserve positional element types
            Expression::TupleExpression { elements } => {
                // infer element types using any contextual type
                let expected_element_types =
                    self.expected_element_types(ctx.expected_type, elements.len(), types);

                // infer element types and collect their contextualized types
                let mut element_tys = Vec::with_capacity(elements.len());
                for (index, element_id) in elements.iter().enumerate() {
                    let expected_element_ty_id =
                        expected_element_types.get(index).copied().flatten();
                    self.infer_argument(
                        module,
                        *element_id,
                        expected_element_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                    let element = tree.get(*element_id);
                    let value_id = element.value();
                    let ty_id = if let Some(ty_id) =
                        types.get_inferred_type_id(value_id.into_global_any(module.id))
                    {
                        ty_id
                    } else {
                        let mut element_ctx = ctx.fork().with_expected_type(expected_element_ty_id);
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
                    element_tys.push(TypeElement::new(ty_id));
                }

                let ty = Type::Tuple {
                    elements: element_tys,
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
                    ctx.expected_type,
                    &literal_fields,
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
                {
                    let value_ty = types.get_type(ty_id);
                    if self.type_contains_managed(module, ctx.profile, value_ty, types) {
                        self.error(AnalyzeError::ManagedMemoryDisabled {
                            node: expression_id
                                .into_global_any(module.id)
                                .into_anchored(Some(ctx.profile)),
                        });
                    }
                }

                ty_id
            }

            // call: return type of callee
            Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => self.infer_call_expression(
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
                    let mut result_ty_id = None;

                    // prefer the contextual type when both branches satisfy it
                    if let Some(expected_ty_id) = ctx.expected_type {
                        let then_assignable = self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            expected_ty_id,
                            then_ty_id,
                            types,
                            &ctx.options,
                        );
                        let else_assignable = self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            expected_ty_id,
                            else_ty_id,
                            types,
                            &ctx.options,
                        );
                        if then_assignable.is_assignable() && else_assignable.is_assignable() {
                            result_ty_id = Some(expected_ty_id);
                        }
                    }

                    // prefer a common supertype when one branch subsumes the other
                    if result_ty_id.is_none() {
                        let then_to_else = self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            else_ty_id,
                            then_ty_id,
                            types,
                            &ctx.options,
                        );
                        if then_to_else.is_assignable() {
                            result_ty_id = Some(else_ty_id);
                        }
                    }
                    if result_ty_id.is_none() {
                        let else_to_then = self.is_type_assignable(
                            module,
                            ctx.profile,
                            symbols,
                            then_ty_id,
                            else_ty_id,
                            types,
                            &ctx.options,
                        );
                        if else_to_then.is_assignable() {
                            result_ty_id = Some(then_ty_id);
                        }
                    }

                    // widen numeric branches to a shared numeric type
                    if result_ty_id.is_none() {
                        let then_ty = types.get_type(then_ty_id).clone();
                        let else_ty = types.get_type(else_ty_id).clone();
                        if self.is_numeric_like_type(&then_ty, types)
                            && self.is_numeric_like_type(&else_ty, types)
                        {
                            let widened = self.widen_numeric_types(&then_ty, &else_ty);
                            result_ty_id = Some(types.insert_type_from(widened, expression_id));
                        }
                    }

                    result_ty_id.unwrap_or_else(|| self.union_type(then_ty_id, else_ty_id, types))
                } else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Void,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // loop: loop body type or never
            Expression::Loop {
                kind: _,
                condition,
                body,
                scope: _,
                symbol: _,
            } => {
                if let Some(cond) = condition {
                    self.infer_expression(module, *cond, tree, symbols, types, infer, ctx)?;
                }
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
                self.infer_block(module, *body, tree, symbols, types, infer, &mut ctx)?;
                // #Incomplete: loop return type depends on break value
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // for each: void
            Expression::ForEach {
                asynchrony: _,
                kind: _,
                binding,
                iterator,
                body,
                scope: _,
                symbol: _,
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
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
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
                symbol: _,
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
                let mut ctx = ctx.fork().in_loop(expression_id.into_any());
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
                let mut ctx = ctx.fork();
                if *source == MatchSource::Match {
                    if *kind == MatchKind::Match {
                        ctx = ctx.in_match(expression_id.into_any());
                    } else {
                        ctx = ctx.in_switch(expression_id.into_any());
                    }
                }
                let is_switch = *kind == MatchKind::Switch;
                let mut case_type_ids = Vec::new();
                for case_id in cases {
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
                            if guard.is_some() {
                                self.error(AnalyzeError::InvalidSwitchCaseGuard {
                                    node: case_id
                                        .into_global_any(module.id)
                                        .into_anchored(Some(ctx.profile)),
                                });
                            }
                            if !matches!(tree.get(*pattern), Pattern::Expression { .. }) {
                                self.error(AnalyzeError::InvalidSwitchCasePattern {
                                    node: pattern
                                        .into_global_any(module.id)
                                        .into_anchored(Some(ctx.profile)),
                                });
                                allow_pattern_infer = false;
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
                                &mut ctx,
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
                                &mut ctx,
                            )?;
                        }
                    }

                    // apply contextual typing to the case body
                    let expected_type = if *kind == MatchKind::Match {
                        ctx.expected_type
                    } else {
                        None
                    };
                    let mut case_ctx = ctx.fork().with_expected_type(expected_type);

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
                        _ => {
                            let source_type_id = case_type_ids[0];
                            self.union_type_from_list(case_type_ids, source_type_id, types)
                        }
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
                    self.union_type(try_ty_id, catch_ty_id, types)
                } else {
                    try_ty_id
                }
            }

            // return: never (control flow)
            Expression::Return { value } => {
                // validate return position
                if !ctx.can_return() {
                    self.error(AnalyzeError::InvalidReturn {
                        node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                    });
                }

                // constrain return value to the function return type
                if let Some(val) = value {
                    let mut return_ctx = ctx.fork().with_expected_type(ctx.return_type);
                    let value_ty_id = self.infer_expression(
                        module,
                        *val,
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
                            *val,
                            return_ty_id,
                            value_ty_id,
                            tree,
                            types,
                            &ctx.options,
                        );
                    }
                    if let Some(return_ty_id) = ctx.return_type {
                        infer.push_constraint(Constraint::Subtype {
                            sub_type: value_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });
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
                types.insert_type_from(ty, expression_id)
            }

            // break: never
            Expression::Break {
                target,
                target_symbol: _,
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
                    self.infer_expression(module, *val, tree, symbols, types, infer, ctx)?;
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

                // require await operand to be promise assignable
                if let Some(promise_ty_id) =
                    self.promise_type(ctx.profile, None, expression_id.into_any(), types)
                    && self.is_type_assignable(
                        module,
                        ctx.profile,
                        symbols,
                        promise_ty_id,
                        inner_ty_id,
                        types,
                        &ctx.options,
                    ) == Assignability::NotAssignable
                    {
                        self.error(AnalyzeError::UnassignableType {
                            node: expression_id.into_global_any(module.id).into_anchored(Some(ctx.profile)),
                            expected_ty: promise_ty_id.into_global(module.id),
                            actual_ty: inner_ty_id.into_global(module.id),
                        });
                    }
                self.unwrap_awaited_type(module, symbols, ctx.profile, inner_ty_id, types)
            }

            // await? should be desugared in Bind
            Expression::AwaitMaybe { .. } => {
                unreachable!("AwaitMaybe should be desugared before analysis")
            }

            // comptime: type of body (evaluated at compile time)
            Expression::Comptime { body } => {
                // #Incomplete: validate that body can be evaluated at comptime
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?
            }

            // yield: yielded type
            Expression::Yield {
                cardinality: _,
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
                if let Some(value_id) = value {
                    self.infer_expression(module, *value_id, tree, symbols, types, infer, ctx)?;
                }
                // #Incomplete: yield type depends on generator context
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
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
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::TaggedTemplateExpression { tag, value: _ } => {
                let _tag_ty_id = self.infer_expression(module, *tag, tree, symbols, types, infer, ctx)?;
                // #Incomplete: tagged template should return type from tag function
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // range expression: analyze bounds, type is Iterable<T>
            // #Incomplete: range should satisfy Iterable<T> where T is the element type
            Expression::RangeExpression {
                start,
                end,
                is_inclusive: _,
            } => {
                self.infer_expression(module, *start, tree, symbols, types, infer, ctx)?;
                self.infer_expression(module, *end, tree, symbols, types, infer, ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }

            // tagged expressions for newtype construction
            Expression::TaggedScalarExpression { ty, value } => {
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
                    Some(ty_id),
                    &literal_fields,
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

            if !ctx.is_surface_inference {
                types.set_inferred_type(expression_id.into_global_any(module.id), ty_id);
            }

            Ok(ty_id)
        }
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

    /// Find the nearest `this` symbol visible to the expression.
    /// #Architecture: should 'find_this_symbol' be resolved during Bind? (instead of Analyze/infer)?
    fn find_this_symbol(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        let this_name = self.program.strings.intern("this");
        let mut scope = symbols.get_scope(expression_id, tree);
        loop {
            if let Some(symbol_id) =
                symbols.find_active_symbol_up_to(scope.1, StaticKey::Name(this_name), scope.2)
            {
                return Some(symbol_id.into_global(module.id));
            }
            let (parent_scope_id, parent_mark) = scope.1.parent?;
            scope = (
                parent_scope_id,
                symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }
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
                    let mut value_ctx = ctx.fork().with_expected_type(expected_field_ty_id);
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
                    let mut default_ctx = ctx.fork().with_expected_type(expected_field_ty_id);
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
                let is_readonly = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.mutability, Some(Mutability::Immutable)));

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

                // infer the method signature with contextual typing
                let method_ty_id = self.infer_signature(
                    module,
                    property_id.into_any(),
                    symbol.into_global(module.id),
                    signature,
                    expected_method_ty_id,
                    None,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // body
                if let Some(body) = body {
                    let return_type = self.function_return_type(method_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .in_function_with_signature(property_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id = self
                        .infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = return_type
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
                    .is_some_and(|m| matches!(m.kind, Some(destack_dir::BindingKind::Maybe)));
                let is_readonly = modifiers.as_ref().is_some_and(|m| {
                    matches!(m.mutability, Some(destack_dir::Mutability::Immutable))
                });
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
                    types.set_value_type(symbol.into_global(module.id), ty_id);
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
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::TaggedTuple { ty, fields } => {
                let ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
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
                    },
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
            Pattern::Object { fields } => {
                // reject bare object patterns against struct values (#Architecture should we?)
                if let Some(binding_ty_id) = binding_ty_id
                    && self.is_definitely_struct_type(types.get_type(binding_ty_id))
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
                let ty_id =
                    self.evaluate_pattern_tag_type(module, *ty, tree, symbols, types, ctx)?;
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
        let binding_ty_fields: Vec<LocalTypeId> = binding_ty_id
            .and_then(|ty_id| match types.get_type(ty_id) {
                Type::Tuple { elements } => {
                    Some(elements.iter().map(|element| element.ty).collect())
                }
                _ => None,
            })
            .unwrap_or_default();
        let spread_len = binding_ty_fields.len().saturating_sub(fields.len() - 1);
        let mut ty_idx = 0;
        for field_id in fields {
            let field = tree.get(*field_id);
            let field_ty = match field {
                PatternField::Named { .. }
                | PatternField::Alias { .. }
                | PatternField::Positional { .. } => {
                    let ty = binding_ty_fields.get(ty_idx).cloned();
                    ty_idx += 1;
                    ty
                }
                PatternField::Spread { .. } => {
                    let rest_types = binding_ty_fields
                        .get(ty_idx..ty_idx + spread_len)
                        .map(|s| s.to_vec())
                        .unwrap_or_default();
                    ty_idx += spread_len;
                    let rest_ty = to_rest_type(rest_types);
                    Some(types.insert_type_from(rest_ty, *field_id))
                }
                PatternField::Elision => {
                    // elision skips a type position
                    ty_idx += 1;
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
        let mut visited = Vec::new();
        let field_ty_id = self.infer_member_of_type(
            module,
            ctx.profile,
            field_id.into_any(),
            &receiver_ty,
            &field_key,
            MemberLookupMode::Any,
            types,
            &mut visited,
        )?;

        // fall back to the binding type for non-object patterns
        Ok(Some(field_ty_id.unwrap_or(binding_ty_id)))
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
        // resolve the canonical symbol for imported references
        let canonical_symbol = self.canonical_symbol_id(
            module,
            symbols,
            ctx.profile,
            target_symbol,
            CanonicalSymbolMode::FollowAliases,
        );

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

        // pick the base type for the symbol
        // prefer flow narrowed types when available
        let base_ty_id = if let Some(narrowed_ty_id) = ctx.get_narrowed(canonical_symbol) {
            narrowed_ty_id
        }
        // reuse a known value type for the symbol
        else if let Some(value_ty_id) = types.get_value_type_id(canonical_symbol) {
            value_ty_id
        }
        // infer value types for direct bindings when missing
        else if let Some(inferred_ty_id) = self.infer_direct_binding_value_type(
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
        }
        // import remote symbol types as needed
        else if canonical_symbol.module_id != module.id {
            self.resolve_remote_symbol_value_type(
                module,
                ctx.profile,
                expression_id.into_any(),
                canonical_symbol,
                types,
            )?
        }
        // local symbol without type: use InferVar for forward references
        else {
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
            static_parameters,
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

        let resolved = self.resolve_function_signature(
            module,
            expression_id.into_any(),
            Some(canonical_symbol),
            Some(static_argument_ids),
            &static_parameters,
            &dynamic_parameters,
            return_type,
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
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // collect candidates for excess property checks
        // bail when no contextual type is available
        let Some(expected_ty_id) = self.expected_value_type(expected_ty_id, types) else {
            return Ok(());
        };
        let mut candidates: Vec<LocalTypeId> = Vec::new();
        self.collect_object_literal_candidates(
            module,
            profile,
            node_id,
            expected_ty_id,
            types,
            &mut candidates,
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
        types: &mut TypeTable,
        candidates: &mut Vec<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        match types.get_type(expected_ty_id).clone() {
            Type::Object { .. } => {
                candidates.push(expected_ty_id);
            }
            Type::Reference { symbol, .. } => {
                let instance_ty_id =
                    self.resolve_instance_type_for_symbol(module, profile, node_id, symbol, types)?;
                if let Some(instance_ty_id) = instance_ty_id {
                    candidates.push(instance_ty_id);
                }
            }
            Type::Union { elements } => {
                for element in elements {
                    self.collect_object_literal_candidates(
                        module, profile, node_id, element, types, candidates,
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
