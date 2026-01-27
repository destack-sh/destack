use std::collections::{HashMap, HashSet};

use super::expression::has_implicit_return;
use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Asynchrony, BindingAnchor, BindingKind, Constraint, Declaration, DeclarationAbstraction,
    Declarator, DependencyItem, DependencyMode, DynamicKey, Expression, FunctionCardinality,
    FunctionKind, FunctionSignature, GlobalNodeIdAny, GlobalSymbolId, InferOrigin, InferScope,
    InferTable, IntType, LocalNodeId, LocalNodeIdAny, LocalTypeId, Member, ModuleTarget,
    Mutability, NodeTree, NodeType, Parameter, Pattern, PrimitiveType, StaticKey, SymbolSpace,
    SymbolTable, Type, TypeField, TypeLiteral, TypeTable, WhereClause,
};
use destack_workspace::{Module, ModuleSource, ProfileId};

/// Describe how a declarator constrains its value type.
pub(super) enum DeclaratorConstraint {
    /// Require assignability between value and declared types.
    Assignable,
    /// Require satisfies semantics between value and declared types.
    Satisfies,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer the type of a declaration.
    pub(crate) fn infer_declaration(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // load the declaration node
        let declaration = tree.get(declaration_id);

        // dispatch by declaration kind
        match declaration {
            // global
            Declaration::Global {
                descriptor: _,
                scope: _,
                expressions,
            } => {
                for expression_id in expressions {
                    self.infer_expression(
                        module,
                        *expression_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }
            // namespace
            Declaration::Namespace {
                descriptor: _,
                generics,
                scope: _,
                expressions,
            } => {
                // infer where clauses and walk namespace expressions
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                for expression_id in expressions {
                    self.infer_expression(
                        module,
                        *expression_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )?;
                }
            }

            // type alias
            Declaration::Type { .. } => {
                // static parameter defaults are resolved during static argument evaluation
            }

            // struct
            Declaration::Struct {
                descriptor,
                generics,
                scope: _,
                members,
                heritage: _,
            } => {
                // infer where clauses for member bodies
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // resolve the nominal type for `this`
                let symbol = descriptor.symbol.into_global(module.id);
                let this_ty_id = Some(types.insert_type_from(
                    Type::Reference {
                        symbol,
                        static_arguments: None,
                    },
                    declaration_id,
                ));

                // prepare struct member context
                let mut ctx = ctx.fork().in_nominal_symbol_maybe(Some(symbol));

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, &mut ctx, this_ty_id,
                    )?;
                }
            }

            // class
            Declaration::Class {
                descriptor,
                generics,
                scope: _,
                members,
                heritage: _,
            } => {
                // infer where clauses for member bodies
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // resolve the nominal type for `this`
                let symbol = descriptor.symbol.into_global(module.id);
                let this_ty_id = Some(types.insert_type_from(
                    Type::Reference {
                        symbol,
                        static_arguments: None,
                    },
                    declaration_id,
                ));

                // prepare abstract context
                let is_abstract = descriptor.abstraction == DeclarationAbstraction::Abstract;
                let mut ctx = ctx
                    .fork()
                    .in_abstract_class_maybe(is_abstract)
                    .in_nominal_symbol_maybe(Some(symbol));

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, &mut ctx, this_ty_id,
                    )?;
                }
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind: _,
                generics,
                scope: _,
                fields,
                members,
                heritage: _,
            } => {
                // infer where clauses for member bodies
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // resolve and record enum field values
                let enum_symbol = descriptor.symbol.into_global(module.id);
                let backing_type = self.infer_enum_field_values(
                    module,
                    ctx.profile,
                    enum_symbol,
                    fields,
                    tree,
                    symbols,
                    types,
                )?;
                types.set_enum_backing_type(enum_symbol, backing_type);

                // resolve the nominal type for `this`
                let this_ty_id = Some(types.insert_type_from(
                    Type::Reference {
                        symbol: enum_symbol,
                        static_arguments: None,
                    },
                    declaration_id,
                ));

                // prepare nominal context for enum members
                let mut ctx = ctx.fork().in_nominal_symbol_maybe(Some(enum_symbol));

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, &mut ctx, this_ty_id,
                    )?;
                }
            }

            // extension
            Declaration::Extension {
                descriptor: _descriptor,
                generics,
                target_type,
                target_symbol,
                scope: _,
                members,
                heritage: _,
            } => {
                // infer where clauses for member bodies
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // assign this to the nominal target type when available
                let this_ty_id = if let Some(target) = *target_symbol {
                    // evaluate the target type to capture static arguments
                    let target_ty_id = self.try_evaluate_expression_to_type(
                        module,
                        ctx.profile,
                        *target_type,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?;
                    let static_arguments = self.unwrap_type_symbol(types, target_ty_id).and_then(
                        |(symbol, static_arguments, _)| {
                            if symbol == target {
                                static_arguments
                            } else {
                                None
                            }
                        },
                    );

                    Some(types.insert_type_from(
                        Type::Reference {
                            symbol: target,
                            static_arguments,
                        },
                        declaration_id,
                    ))
                } else {
                    None
                };

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, ctx, this_ty_id,
                    )?;
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind: _,
                generics,
                scope: _,
                members,
                heritage: _,
            } => {
                // infer where clauses for member bodies
                self.infer_where_clauses_maybe(
                    module,
                    generics.where_clauses.as_deref(),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // resolve the nominal type for `this`
                let symbol = descriptor.symbol.into_global(module.id);
                let this_ty_id = Some(types.insert_type_from(
                    Type::Reference {
                        symbol,
                        static_arguments: None,
                    },
                    declaration_id,
                ));

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, ctx, this_ty_id,
                    )?;
                }
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                scope: _,
                body,
            } => {
                // apply decorator options for this function
                let function_options = {
                    let symbol = symbols.get_symbol(descriptor.symbol);
                    ctx.options.with_symbol_decorators(&symbol.decorators)
                };

                // infer the function signature
                let declared_signature_ty_id =
                    types.get_signature_type_for_node(declaration_id.into_global_any(module.id));
                let mut signature_ctx = ctx.fork().with_options(function_options);
                let fn_ty_id = self.infer_signature(
                    module,
                    declaration_id.into_any(),
                    descriptor.symbol.into_global(module.id),
                    signature,
                    ctx.expected_type,
                    declared_signature_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut signature_ctx,
                )?;

                // merge the inferred signature into the symbol value type
                let symbol_entry = symbols.get_symbol(descriptor.symbol);
                let allow_merge = module.language_type.supports_declaration_merging()
                    || module.language_type.is_destack()
                    || symbol_entry.origin.is_global_augmentation();
                self.merge_function_value_type(
                    module,
                    ctx.profile,
                    declaration_id,
                    descriptor.symbol,
                    fn_ty_id,
                    declared_signature_ty_id,
                    symbols,
                    types,
                    allow_merge,
                );

                // infer the body when present
                if let Some(body) = body {
                    // prepare return type tracking for the body
                    let return_type = self.function_return_type(fn_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .with_options(function_options)
                        .in_function_with_signature(declaration_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the function body with implicit return typing
                    let body_ty_id =
                        self.infer_body(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = return_type
                        && has_implicit_return(*body, tree)
                    {
                        infer.push_constraint(Constraint::Subtype {
                            sub_type: body_ty_id,
                            super_type: return_ty_id,
                            variance: None,
                        });

                        let normalized_return_ty_id = self.normalize_type_for_assignability(
                            module,
                            ctx.profile,
                            return_ty_id,
                            symbols,
                            types,
                        );

                        if !self.is_infer_var_type(return_ty_id, types)
                            && !self.is_infer_var_type(body_ty_id, types)
                            && self.is_type_assignable(
                                module,
                                ctx.profile,
                                symbols,
                                normalized_return_ty_id,
                                body_ty_id,
                                types,
                                &function_options,
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
            }
        }

        Ok(())
    }

    /// Infer a member declaration.
    pub(super) fn infer_member(
        &self,
        module: &Module,
        member_id: LocalNodeId<Member>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
        this_ty_id: Option<LocalTypeId>,
    ) -> AnalyzeResult<()> {
        // capture the member node
        let member = tree.get(member_id);

        // dispatch by member kind
        match member {
            Member::Type {
                name,
                ty,
                value,
                symbol: _,
                modifiers: _,
            } => {
                // infer type member metadata expressions
                self.infer_expression(module, *name, tree, symbols, types, infer, ctx)?;

                if let Some(ty) = ty {
                    self.infer_expression(module, *ty, tree, symbols, types, infer, ctx)?;
                }

                if let Some(value) = value {
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                }

                Ok(())
            }
            Member::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                // resolve the member symbol
                let member_symbol = member.symbol().into_global(module.id);

                // infer index signatures and defaults
                if let Some(DynamicKey::NamedExpression { name: _, key }) = key {
                    let _key_type = self.try_evaluate_expression_to_type(
                        module,
                        ctx.profile,
                        *key,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?;
                    let _value_type = if let Some(value) = value {
                        self.try_evaluate_expression_to_type(
                            module,
                            ctx.profile,
                            *value,
                            tree,
                            symbols,
                            types,
                            true,
                            true,
                        )?
                    } else {
                        let ty = Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        };
                        types.insert_type_from_any(ty, member_id.into_any())
                    };
                    let _is_readonly = modifiers.as_ref().is_some_and(|modifiers| {
                        modifiers.mutability == Some(destack_dir::Mutability::Immutable)
                    });

                    if let Some(default) = default {
                        self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                    }

                    // assign a placeholder value type for computed fields
                    if types.get_value_type_id(member_symbol).is_none() {
                        let scope = InferScope {
                            owner: member_symbol,
                            function_id: ctx
                                .in_function
                                .map(|function_id| function_id.into_global(module.id)),
                        };
                        let placeholder_ty_id = self.infer_var_type_for_symbol(
                            infer,
                            types,
                            member_symbol,
                            member_id.into_any(),
                            InferOrigin::Expression(member_id.into_global_any(module.id)),
                            scope,
                        );
                        types.set_value_type(member_symbol, placeholder_ty_id);
                    }

                    return Ok(());
                }

                // evaluate the field type
                let value_ty_id = if let Some(value) = value {
                    Some(self.try_evaluate_expression_to_type(
                        module,
                        ctx.profile,
                        *value,
                        tree,
                        symbols,
                        types,
                        true,
                        true,
                    )?)
                } else {
                    None
                };

                // infer the default value when present
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?)
                } else {
                    None
                };

                // update the value shape for inferred static fields
                let is_static = modifiers
                    .as_ref()
                    .is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static));
                let static_key = key.and_then(|key| {
                    self.static_key_from_dynamic_key(ctx.profile, key, tree, symbols, types)
                });
                if is_static
                    && value_ty_id.is_none()
                    && let Some(default_ty_id) = default_ty_id
                    && let Some(static_key) = static_key
                    && let Some(owner_symbol) = ctx.in_nominal_symbol
                {
                    let is_optional = modifiers.as_ref().is_some_and(|modifiers| {
                        matches!(modifiers.kind, Some(BindingKind::Maybe))
                    });
                    let is_readonly = modifiers.as_ref().is_some_and(|modifiers| {
                        matches!(modifiers.mutability, Some(Mutability::Immutable))
                    });
                    let field = TypeField {
                        key: static_key,
                        ty: default_ty_id,
                        is_optional,
                        is_readonly,
                    };
                    self.update_value_shape_with_field(
                        member_id.into_any(),
                        owner_symbol,
                        field,
                        types,
                    );
                }

                // infer member types from defaults when no annotation exists
                if value_ty_id.is_none()
                    && let Some(default_ty_id) = default_ty_id
                    && types.get_value_type_id(member_symbol).is_none()
                {
                    types.set_value_type(member_symbol, default_ty_id);
                }

                // attach declared member types when available
                if let Some(value_ty_id) = value_ty_id
                    && types.get_value_type_id(member_symbol).is_none()
                {
                    types.set_value_type(member_symbol, value_ty_id);
                }

                // seed an inference variable when no value type metadata exists
                if types.get_value_type_id(member_symbol).is_none() {
                    let scope = InferScope {
                        owner: member_symbol,
                        function_id: ctx
                            .in_function
                            .map(|function_id| function_id.into_global(module.id)),
                    };
                    let placeholder_ty_id = self.infer_var_type_for_symbol(
                        infer,
                        types,
                        member_symbol,
                        member_id.into_any(),
                        InferOrigin::Expression(member_id.into_global_any(module.id)),
                        scope,
                    );
                    types.set_value_type(member_symbol, placeholder_ty_id);
                }

                Ok(())
            }
            Member::Method {
                key: _,
                signature,
                body,
                modifiers: _,
                ..
            } => {
                // apply decorator options for this method
                let method_options = {
                    let symbol = symbols.get_symbol(member.symbol());
                    ctx.options.with_symbol_decorators(&symbol.decorators)
                };

                // assign the implicit this binding type when available
                if let Some(this_ty_id) = this_ty_id {
                    let this_name = self.program.strings.intern("this");
                    let (_scope_id, scope, _mark) = symbols.get_scope(member_id, tree);
                    if let Some(this_symbol) =
                        symbols.find_active_symbol(scope, StaticKey::Name(this_name))
                    {
                        types.set_value_type(this_symbol.into_global(module.id), this_ty_id);
                    }
                }

                // infer the method signature
                let declared_signature_ty_id =
                    types.get_signature_type_for_node(member_id.into_global_any(module.id));
                let mut signature_ctx = ctx.fork().with_options(method_options);
                let method_ty_id = self.infer_signature(
                    module,
                    member_id.into_any(),
                    member.symbol().into_global(module.id),
                    signature,
                    None,
                    declared_signature_ty_id,
                    tree,
                    symbols,
                    types,
                    infer,
                    &mut signature_ctx,
                )?;

                // prepare the return type for body inference
                let mut return_type = self.function_return_type(method_ty_id, types);
                if let Some(this_ty_id) = this_ty_id {
                    let mut cache = HashMap::new();

                    // substitute this in the explicit this parameter
                    if let Some(this_parameter_id) = signature.this_parameter {
                        let param_symbol =
                            tree.get(this_parameter_id).symbol().into_global(module.id);
                        if let Some(param_ty_id) = types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                types,
                                &mut cache,
                            );
                            types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in dynamic parameters
                    for parameter_id in signature.dynamic_parameters.iter() {
                        let param_symbol = tree.get(*parameter_id).symbol().into_global(module.id);
                        if let Some(param_ty_id) = types.get_value_type_id(param_symbol) {
                            let mapped_ty_id = self.substitute_this_type(
                                param_ty_id,
                                this_ty_id,
                                types,
                                &mut cache,
                            );
                            types.set_value_type(param_symbol, mapped_ty_id);
                        }
                    }

                    // substitute this in the return type
                    return_type = return_type.map(|return_type| {
                        self.substitute_this_type(return_type, this_ty_id, types, &mut cache)
                    });
                }

                // infer the body when present
                if let Some(body) = body {
                    let ctx = ctx
                        .reset()
                        .with_options(method_options)
                        .in_function_with_signature(member_id.into_any(), signature);
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

                        let normalized_return_ty_id = self.normalize_type_for_assignability(
                            module,
                            ctx.profile,
                            return_ty_id,
                            symbols,
                            types,
                        );

                        if !self.is_infer_var_type(return_ty_id, types)
                            && !self.is_infer_var_type(body_ty_id, types)
                            && self.is_type_assignable(
                                module,
                                ctx.profile,
                                symbols,
                                normalized_return_ty_id,
                                body_ty_id,
                                types,
                                &method_options,
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

                Ok(())
            }
            Member::Embed { value, .. } => {
                // #Incomplete: expand embedded type into member fields?
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(())
            }
            Member::StaticBlock { body, .. } => {
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?;
                Ok(())
            }
            Member::ComptimeBlock { body, .. } => {
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?;
                Ok(())
            }
        }
    }

    /// Infer where clauses when present.
    pub(super) fn infer_where_clauses_maybe(
        &self,
        module: &Module,
        clauses: Option<&[LocalNodeId<WhereClause>]>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // infer where clause expressions
        if let Some(clauses) = clauses {
            for clause_id in clauses {
                self.infer_where_clause(module, *clause_id, tree, symbols, types, infer, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer a function signature.
    pub(super) fn infer_signature(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        owner_symbol: GlobalSymbolId,
        signature: &FunctionSignature,
        expected_fn_ty_id: Option<LocalTypeId>,
        declared_signature_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // capture options for diagnostics
        let options = ctx.options;
        let module_options = self.analyze_context_options_for_module(module.id);
        let enforce_decorator_no_managed = options.no_managed && !module_options.no_managed;

        // reject runtime features in no-runtime mode
        if options.no_runtime
            && matches!(module.source, ModuleSource::User)
            && (signature.asynchrony == Asynchrony::Async
                || signature.cardinality == FunctionCardinality::Generator)
        {
            self.error(AnalyzeError::RuntimeDisabled {
                node: node_id
                    .into_global(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // walk generics
        let where_clauses = signature
            .generics
            .as_ref()
            .and_then(|generics| generics.where_clauses.as_deref());
        self.infer_where_clauses_maybe(module, where_clauses, tree, symbols, types, infer, ctx)?;

        // collect static parameter placeholders
        let static_parameters =
            self.static_parameter_placeholders_for_signature(module, signature, tree, types);

        // extract any contextual function signature
        let expected_signature = self.expected_function_signature(expected_fn_ty_id, types);

        // collect parameter types
        let scope = InferScope {
            owner: owner_symbol,
            function_id: Some(node_id.into_global(module.id)),
        };

        // this parameter
        let expected_this_ty_id = expected_signature
            .as_ref()
            .and_then(|signature| signature.this_parameter);
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            // resolve any declared type for the this parameter
            let declared_ty_id =
                types.get_declared_type_id(this_parameter_id.into_global_any(module.id));
            let expected_ty_id = expected_this_ty_id;

            // report implicit this when no declared type exists
            if options.no_implicit_this
                && declared_ty_id.is_none()
                && expected_ty_id.is_none()
                && !matches!(module.source, ModuleSource::Builtin(_))
            {
                self.error(AnalyzeError::ImplicitThis {
                    node: this_parameter_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            let param_symbol = tree.get(this_parameter_id).symbol().into_global(module.id);
            let param_ty_id = declared_ty_id.or(expected_ty_id).unwrap_or_else(|| {
                self.infer_var_type_for_symbol(
                    infer,
                    types,
                    param_symbol,
                    this_parameter_id.into_any(),
                    InferOrigin::Parameter(this_parameter_id.into_global_any(module.id)),
                    scope,
                )
            });
            self.infer_parameter(
                module,
                this_parameter_id,
                Some(param_ty_id),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;
            types.set_value_type(param_symbol, param_ty_id);
            Some(param_ty_id)
        } else if signature.kind == FunctionKind::Lambda {
            // contextual "this" for lambdas
            if let Some(expected_this_ty_id) = expected_this_ty_id {
                let this_name = self.program.strings.intern("this");
                let (_scope_id, scope, _mark) = match node_id.ty {
                    // use the lambda declaration scope
                    NodeType::Declaration => {
                        symbols.get_scope(node_id.into_typed::<Declaration>(), tree)
                    }
                    // use the method scope for member lambdas
                    NodeType::Member => symbols.get_scope(node_id.into_typed::<Member>(), tree),
                    // use the expression scope for inline lambdas
                    NodeType::Expression => {
                        symbols.get_scope(node_id.into_typed::<Expression>(), tree)
                    }
                    // fall back to declaration scopes for internal nodes
                    _ => symbols.get_scope(LocalNodeId::<Declaration>::new(node_id.id), tree),
                };
                if let Some(this_symbol) =
                    symbols.find_active_symbol(scope, StaticKey::Name(this_name))
                {
                    types.set_value_type(this_symbol.into_global(module.id), expected_this_ty_id);
                }

                Some(expected_this_ty_id)
            } else {
                None
            }
        } else {
            None
        };

        // dynamic parameters
        let mut dynamic_param_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            // resolve declared, contextual, and default metadata
            let declared_ty_id =
                types.get_declared_type_id(parameter_id.into_global_any(module.id));
            let expected_param_ty_id = expected_signature
                .as_ref()
                .and_then(|signature| signature.dynamic_parameters.get(index).copied());
            let has_default = match tree.get(*parameter_id) {
                Parameter::Named { default, .. } => default.is_some(),
                Parameter::Pattern { default, .. } => default.is_some(),
                Parameter::Variadic { .. } => false,
            };

            // report implicit any when no type info is available
            if options.no_implicit_any
                && declared_ty_id.is_none()
                && expected_param_ty_id.is_none()
                && !has_default
                && !matches!(module.source, ModuleSource::Builtin(_))
            {
                self.error(AnalyzeError::ImplicitAny {
                    node: parameter_id
                        .into_global_any(module.id)
                        .into_anchored(Some(ctx.profile)),
                });
            }

            // select the parameter type or fall back to inference
            let param_symbol = tree.get(*parameter_id).symbol().into_global(module.id);
            let param_ty_id = declared_ty_id.or(expected_param_ty_id).unwrap_or_else(|| {
                self.infer_var_type_for_symbol(
                    infer,
                    types,
                    param_symbol,
                    parameter_id.into_any(),
                    InferOrigin::Parameter(parameter_id.into_global_any(module.id)),
                    scope,
                )
            });

            self.infer_parameter(
                module,
                *parameter_id,
                Some(param_ty_id),
                tree,
                symbols,
                types,
                infer,
                ctx,
            )?;

            types.set_value_type(param_symbol, param_ty_id);
            dynamic_param_types.push(param_ty_id);
        }

        // return type
        let return_type_node_id = signature.return_type;
        let declared_return_type = declared_signature_ty_id
            .and_then(|signature_id| self.function_return_type(signature_id, types));
        let has_concrete_declared_return = declared_return_type.is_some_and(|ty_id| {
            !self.is_infer_var_type(ty_id, types)
                && !matches!(
                    types.get_type(ty_id),
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown
                    }
                )
        });
        let return_type = if let Some(return_type_node_id) = return_type_node_id {
            Some(self.try_evaluate_expression_to_type(
                module,
                ctx.profile,
                return_type_node_id,
                tree,
                symbols,
                types,
                true,
                true,
            )?)
        } else if let Some(return_type) =
            expected_signature.and_then(|signature| signature.return_type)
        {
            Some(return_type)
        } else if has_concrete_declared_return {
            declared_return_type
        } else {
            Some(self.infer_var_type_for_node(
                infer,
                types,
                node_id.into_global(module.id),
                InferOrigin::Return(node_id.into_global(module.id)),
                scope,
            ))
        };

        // enforce no-managed decorators on signature types
        if enforce_decorator_no_managed {
            self.check_no_managed_signature(
                module,
                ctx.profile,
                signature,
                this_parameter,
                &dynamic_param_types,
                return_type,
                return_type_node_id,
                tree,
                symbols,
                types,
            )?;
        }

        // build the function type for this signature
        let ty = Type::Function {
            asynchrony: signature.asynchrony,
            cardinality: signature.cardinality,
            dynamic_parameters: dynamic_param_types,
            static_parameters,
            this_parameter,
            return_type,
        };
        let ty_id = if let Some(declared_ty_id) = declared_signature_ty_id {
            // update the declared signature type in place
            let declared_ty = types.get_type_mut(declared_ty_id);
            *declared_ty = ty;

            // invalidate normalization cache after mutating the type table
            types.invalidate_normalization_cache();
            declared_ty_id
        } else {
            types.insert_type_from_any(ty, node_id)
        };

        // record signature type for lowering
        if !ctx.is_surface_inference {
            types.set_inferred_type(node_id.into_global(module.id), ty_id);
        }

        Ok(ty_id)
    }

    /// Enforce no-managed decorators on function signatures.
    fn check_no_managed_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        signature: &FunctionSignature,
        this_parameter: Option<LocalTypeId>,
        dynamic_param_types: &[LocalTypeId],
        return_type: Option<LocalTypeId>,
        return_type_node_id: Option<LocalNodeId<Expression>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // only enforce for user modules
        if !matches!(module.source, ModuleSource::User) {
            return Ok(());
        }

        // enforce this parameter types when present
        if let Some(this_parameter_id) = signature.this_parameter
            && let Some(this_ty_id) = this_parameter
        {
            self.check_no_managed_signature_type(
                module,
                profile,
                this_parameter_id.into_global_any(module.id),
                this_ty_id,
                tree,
                symbols,
                types,
            )?;
        }

        // enforce dynamic parameter types
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let Some(param_ty_id) = dynamic_param_types.get(index).copied() else {
                continue;
            };
            self.check_no_managed_signature_type(
                module,
                profile,
                parameter_id.into_global_any(module.id),
                param_ty_id,
                tree,
                symbols,
                types,
            )?;
        }

        // enforce return type when declared
        if let (Some(return_type_node_id), Some(return_type_id)) =
            (return_type_node_id, return_type)
        {
            self.check_no_managed_signature_type(
                module,
                profile,
                return_type_node_id.into_global_any(module.id),
                return_type_id,
                tree,
                symbols,
                types,
            )?;
        }

        Ok(())
    }

    /// Enforce no-managed decorators on a single signature type.
    fn check_no_managed_signature_type(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: GlobalNodeIdAny,
        ty_id: LocalTypeId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        // resolve unevaluated types before checking managed usage
        if matches!(types.get_type(ty_id), Type::Unevaluated(_)) {
            self.evaluate_type(module, profile, ty_id, tree, symbols, types)?;
        }

        // report managed types in signatures
        if self.type_contains_managed(module, profile, ty_id, types) {
            self.error(AnalyzeError::ManagedMemoryDisabled {
                node: node_id.into_anchored(Some(profile)),
            });
        }

        Ok(())
    }

    /// Infer a parameter.
    pub(super) fn infer_parameter(
        &self,
        module: &Module,
        parameter_id: LocalNodeId<Parameter>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // evaluate declared parameter types before use
        if let Some(binding_ty_id) = binding_ty_id
            && matches!(types.get_type(binding_ty_id), Type::Unevaluated(_))
        {
            self.evaluate_type(module, ctx.profile, binding_ty_id, tree, symbols, types)?;
        }

        let parameter = tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                modifiers: _,
                name: _,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }

                // infer default expression and constrain to parameter type
                if let Some(default) = default {
                    let default_ty_id =
                        self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                    if let Some(binding_ty_id) = binding_ty_id {
                        infer.push_constraint(Constraint::Subtype {
                            sub_type: default_ty_id,
                            super_type: binding_ty_id,
                            variance: None,
                        });
                    }
                }
            }
            Parameter::Pattern {
                modifiers: _,
                pattern,
                default,
                symbol,
            } => {
                // bind parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }

                // infer default expression and pick a binding type
                let default_ty_id = if let Some(default) = default {
                    Some(self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?)
                } else {
                    None
                };
                let binding_ty_id = binding_ty_id.or(default_ty_id);

                // constrain default to the binding type
                if let (Some(default_ty_id), Some(binding_ty_id)) = (default_ty_id, binding_ty_id) {
                    infer.push_constraint(Constraint::Subtype {
                        sub_type: default_ty_id,
                        super_type: binding_ty_id,
                        variance: None,
                    });
                }

                // infer bindings within the pattern
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
            Parameter::Variadic {
                modifiers: _,
                name: _,
                symbol,
            } => {
                // bind variadic parameter symbol to its type
                if let Some(binding_ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), binding_ty_id);
                }
            }
        }
        Ok(())
    }

    /// Infer a dependency item.
    ///
    /// For imports from data/text/binary modules, this infers the appropriate type
    /// for the local binding symbol.
    pub(super) fn infer_dependency_item(
        &self,
        _module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
        _infer: &mut InferTable,
        _ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let item = tree.get(item_id);
        match item {
            DependencyItem::UnresolvedRemote { .. } => {
                // nothing to do
            }
            DependencyItem::UnresolvedLocal { .. } => {
                // nothing to do
            }
            DependencyItem::Value { .. } => {
                // nothing to do
            }
            DependencyItem::Local { .. } => {
                // nothing to do
            }
            DependencyItem::Remote {
                target_module,
                target_symbol,
                mode,
                ..
            } => {
                // check if we're importing from a non-code module
                if let ModuleTarget::Module(target_module_id) = target_module {
                    let target = self.program.modules.get(*target_module_id);
                    let target = target.read();
                    // only handle data module namespace and default imports
                    if !target.is_code()
                        && (*mode == DependencyMode::Namespace || *mode == DependencyMode::Default)
                    {
                        let ty_id =
                            self.infer_data_module_type(&target, item_id.into_any(), types)?;

                        // set the type on the canonical symbol used by the binding
                        types.set_value_type(*target_symbol, ty_id);
                    }
                }
            }
        }
        Ok(())
    }

    /// Infer the type for a data/text/binary module import.
    fn infer_data_module_type(
        &self,
        target_module: &Module,
        source_node: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> AnalyzeResult<LocalTypeId> {
        use super::super::common::json_value_to_type;
        use destack_workspace::ModuleContent;

        match &target_module.content {
            ModuleContent::Data { value, .. } => {
                // infer structural type from JSON value
                Ok(json_value_to_type(
                    value,
                    source_node,
                    types,
                    &self.program.strings,
                ))
            }
            ModuleContent::Text { .. } => {
                // text imports are always string
                Ok(types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    },
                    source_node,
                ))
            }
            ModuleContent::Binary { .. } => {
                // binary imports are uint8[] (Uint8Array on JS targets)
                let element_type = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
                    },
                    source_node,
                );
                Ok(types.insert_type_from_any(
                    Type::Array {
                        element: Some(element_type),
                        is_readonly: false,
                    },
                    source_node,
                ))
            }
            ModuleContent::Code(_) | ModuleContent::Unloaded => {
                unreachable!("code modules are handled by analyze_module_infer");
            }
        }
    }

    /// Infer where clause.
    pub(super) fn infer_where_clause(
        &self,
        module: &Module,
        clause_id: LocalNodeId<WhereClause>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let clause = tree.get(clause_id);
        let constraint_ty_id = self.try_evaluate_expression_to_type(
            module,
            ctx.profile,
            clause.right,
            tree,
            symbols,
            types,
            true,
            true,
        )?;
        let Some(parameter_symbol) =
            self.static_parameter_symbol_for_where_clause(module, clause_id, tree, symbols)
        else {
            self.error(AnalyzeError::MissingType {
                node: clause_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
            return Ok(());
        };

        // merge the constraint for static argument validation
        let constraint_ty_id = if let Some(existing_id) =
            types.get_static_parameter_constraint_type(parameter_symbol)
        {
            let merged_id = self.intersection_type_from_list(
                vec![existing_id, constraint_ty_id],
                existing_id,
                types,
            );
            types.set_static_parameter_constraint_type(parameter_symbol, merged_id);
            merged_id
        } else {
            types.set_static_parameter_constraint_type(parameter_symbol, constraint_ty_id);
            constraint_ty_id
        };

        // apply constraint to the parameter when no annotation exists
        let symbol_entry = symbols.get_symbol(parameter_symbol.local_id);
        if symbol_entry.is_static_parameter()
            && let Some(primary) = symbol_entry.primary_declaration
            && primary.local_id.ty == NodeType::Parameter
        {
            let existing_id = types.get_declared_type_id(primary);
            let should_override = existing_id.is_none()
                || existing_id.is_some_and(|ty_id| {
                    matches!(
                        types.get_type(ty_id),
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown | TypeLiteral::Any
                        }
                    )
                });
            if should_override {
                types.set_declared_type(primary, constraint_ty_id);
            }
        }

        // enforce the constraint during inference
        let parameter_ty_id = types.insert_type_from(
            Type::Reference {
                symbol: parameter_symbol,
                static_arguments: None,
            },
            clause_id,
        );
        infer.push_constraint(Constraint::Subtype {
            sub_type: parameter_ty_id,
            super_type: constraint_ty_id,
            variance: None,
        });

        Ok(())
    }

    /// Resolve a where clause parameter to a static parameter symbol.
    fn static_parameter_symbol_for_where_clause(
        &self,
        module: &Module,
        clause_id: LocalNodeId<WhereClause>,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> Option<GlobalSymbolId> {
        // capture the parameter name as a static key
        let clause = tree.get(clause_id);
        let key = StaticKey::Name(clause.left);
        let mut scope = symbols.get_scope(clause_id, tree);

        loop {
            // search the active bindings in the current scope
            let limit = scope.2.0 as usize;
            let named_symbols = &scope.1.named_symbols;
            let limit = limit.min(named_symbols.len());
            for (candidate_key, symbol_id) in named_symbols[..limit].iter().rev() {
                if *candidate_key != key {
                    continue;
                }

                // skip inactive symbols
                let symbol = symbols.get_symbol(*symbol_id);
                if !symbol.is_active() {
                    continue;
                }

                // accept static parameters from type-capable spaces
                let is_type_space =
                    matches!(symbol.space, SymbolSpace::Type | SymbolSpace::TypeValue);
                if is_type_space && symbol.is_static_parameter() {
                    return Some(symbol_id.into_global(module.id));
                }
            }

            // walk to the parent scope when present
            let Some((parent_scope_id, parent_mark)) = scope.1.parent else {
                break;
            };
            scope = (
                parent_scope_id,
                symbols.get_scope_by_id(parent_scope_id),
                parent_mark,
            );
        }

        None
    }

    /// Infer a pattern, given an optional binding type of the pattern.
    pub(super) fn infer_declarator(
        &self,
        module: &Module,
        declarator_id: LocalNodeId<Declarator>,
        _let_expression_id: LocalNodeId<Expression>,
        constraint: DeclaratorConstraint,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // capture options and the declarator node
        let options = ctx.options;
        let declarator = tree.get(declarator_id);
        let Declarator {
            pattern,
            ty: _,
            value,
        } = declarator;

        // infer type from value or annotation
        // declared type is now on the declarator node, not the let expression
        let declared_ty_id =
            types.get_declared_type_id(declarator_id.into_global(module.id).into());

        // report implicit any when no annotation or initializer exists
        if options.no_implicit_any
            && declared_ty_id.is_none()
            && value.is_none()
            && !matches!(module.source, ModuleSource::Builtin(_))
        {
            self.error(AnalyzeError::ImplicitAny {
                node: declarator_id
                    .into_global_any(module.id)
                    .into_anchored(Some(ctx.profile)),
            });
        }

        // evaluate and prepare declared types before inference
        if let Some(declared_ty_id) = declared_ty_id {
            self.evaluate_type(module, ctx.profile, declared_ty_id, tree, symbols, types)?;
            self.ensure_reference_instance_types_for_type(
                module,
                ctx.profile,
                declarator_id.into_any(),
                declared_ty_id,
                types,
            )?;
        }

        let inferred_ty_id = if let Some(value) = value {
            // apply declared type as the expected type when available
            let mut value_ctx = if let Some(declared_ty_id) = declared_ty_id {
                ctx.fork().with_expected_type(Some(declared_ty_id))
            } else {
                ctx.fork()
            };
            Some(self.infer_expression(
                module,
                *value,
                tree,
                symbols,
                types,
                infer,
                &mut value_ctx,
            )?)
        } else {
            None
        };

        // assign direct binding value types from declared or inferred types
        if let Pattern::Binding {
            symbol, pattern, ..
        } = tree.get(*pattern)
            && pattern.is_none()
        {
            let binding_ty_id = declared_ty_id.or(inferred_ty_id);
            if let Some(binding_ty_id) = binding_ty_id {
                types.set_value_type(symbol.into_global(module.id), binding_ty_id);
            }
        }

        // enforce explicit ownership when implicit managed values are disabled
        if let (Some(inferred_ty_id), Some(value_id)) = (inferred_ty_id, value) {
            if let Some(declared_ty_id) = declared_ty_id {
                self.check_no_implicit_managed_value(
                    module,
                    ctx.profile,
                    *value_id,
                    declared_ty_id,
                    inferred_ty_id,
                    tree,
                    types,
                    &options,
                );
            } else {
                self.check_no_implicit_managed_inferred(
                    module,
                    ctx.profile,
                    *value_id,
                    inferred_ty_id,
                    tree,
                    types,
                    &options,
                );
            }
        }

        // type check: if both declared and inferred, check assignability
        if let (Some(declared), Some(inferred)) = (declared_ty_id, inferred_ty_id) {
            let mut visited = HashSet::new();
            let skip_assignability = self.type_contains_error(declared, types, &mut visited)
                || self.type_contains_error(inferred, types, &mut visited);

            // resolve inference variables before assignability checks
            let resolved_declared = if self.is_infer_var_type(declared, types) {
                self.resolve_infer_type_for_check(
                    module,
                    ctx.profile,
                    symbols,
                    declared,
                    infer,
                    types,
                    &options,
                )
                .unwrap_or(declared)
            } else {
                declared
            };
            let resolved_inferred = if self.is_infer_var_type(inferred, types) {
                self.resolve_infer_type_for_check(
                    module,
                    ctx.profile,
                    symbols,
                    inferred,
                    infer,
                    types,
                    &options,
                )
                .unwrap_or(inferred)
            } else {
                inferred
            };

            // apply inference constraints when required
            if !skip_assignability && matches!(constraint, DeclaratorConstraint::Assignable) {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: inferred,
                    super_type: declared,
                    variance: None,
                });
            }

            if !skip_assignability
                && !self.is_infer_var_type(resolved_declared, types)
                && !self.is_infer_var_type(resolved_inferred, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    resolved_declared,
                    resolved_inferred,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                let error = match constraint {
                    DeclaratorConstraint::Assignable => AnalyzeError::UnassignableType {
                        node: declarator_id.into_global(module.id).into(),
                        expected_ty: resolved_declared.into_global(module.id),
                        actual_ty: resolved_inferred.into_global(module.id),
                    },
                    DeclaratorConstraint::Satisfies => AnalyzeError::UnsatisfiedType {
                        node: declarator_id.into_global(module.id).into(),
                        expected_ty: resolved_declared.into_global(module.id),
                        actual_ty: resolved_inferred.into_global(module.id),
                    },
                };
                return Err(error);
            }
        }

        // infer pattern bindings from declared or inferred type
        let binding_ty_id = declared_ty_id.or(inferred_ty_id);
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

        // ensure direct bindings always record a value type
        if let Some(binding_ty_id) = binding_ty_id
            && let Pattern::Binding { symbol, .. } = tree.get(*pattern)
        {
            let binding_symbol = symbol.into_global(module.id);
            if types.get_value_type_id(binding_symbol).is_none() {
                types.set_value_type(binding_symbol, binding_ty_id);
            }
        }

        Ok(())
    }
}
