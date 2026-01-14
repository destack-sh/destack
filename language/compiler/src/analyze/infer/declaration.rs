use std::collections::HashMap;

use crate::{AnalyzeError, AnalyzeResult, Assignability, Compiler, InferContext};
use destack_dir::{
    Constraint, Declaration, DeclarationAbstraction, Declarator, DependencyItem, DependencyMode,
    DynamicKey, EnumBackingType, EnumField, Expression, FunctionSignature,
    GlobalSymbolId, InferOrigin, InferScope, InferTable, IntType, LocalNodeId, LocalNodeIdAny,
    LocalTypeId, Member, ModuleTarget, NodeTree, Parameter, PrimitiveType, ScalarLiteral,
    StaticKey, SymbolTable, Type, TypeLiteral, TypeTable, WhereClause,
};
use destack_workspace::{Module, ProfileId};

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
    pub(super) fn infer_declaration(
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

        // capture context options
        let options = ctx.options;

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

                // infer member bodies without mutating instance shapes
                for member_id in members {
                    self.infer_member(
                        module, *member_id, tree, symbols, types, infer, ctx, this_ty_id,
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

                // prepare abstract context
                let is_abstract = descriptor.abstraction == DeclarationAbstraction::Abstract;
                let mut ctx = ctx.fork().in_abstract_class_maybe(is_abstract);

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

                // infer enum field values
                for field_id in fields {
                    self.infer_enum_field(module, *field_id, tree, symbols, types, infer, ctx)?;
                }

                // infer and record the enum backing type
                let enum_symbol = descriptor.symbol.into_global(module.id);
                let backing_type =
                    self.infer_enum_backing_type(module, ctx.profile, fields, tree, types)?;
                types.set_enum_backing_type(enum_symbol, backing_type);

                // resolve the nominal type for `this`
                let this_ty_id = Some(types.insert_type_from(
                    Type::Reference {
                        symbol: enum_symbol,
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
                // infer the function signature
                let declared_signature_ty_id =
                    types.get_signature_type_for_node(declaration_id.into_global_any(module.id));
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
                    ctx,
                )?;

                // infer the body when present
                if let Some(body) = body {
                    // prepare return type tracking for the body
                    let return_type = self.function_return_type(fn_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .in_function_with_signature(declaration_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the function body with implicit return typing
                    let body_ty_id =
                        self.infer_body(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = return_type
                        && self.has_implicit_return(*body, tree)
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
        // capture the context and member node
        let options = ctx.options;
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
                // infer index signatures and defaults
                if let Some(DynamicKey::NamedExpression { name: _, key }) = key {
                    let _key_type = self.try_evaluate_expression_to_type(
                        module,
                        ctx.profile,
                        *key,
                        tree,
                        symbols,
                        types,
                    )?;
                    let _value_type = if let Some(value) = value {
                        self.try_evaluate_expression_to_type(
                            module,
                            ctx.profile,
                            *value,
                            tree,
                            symbols,
                            types,
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

                    return Ok(());
                }

                // evaluate the field type
                let _value_ty_id = if let Some(value) = value {
                    self.try_evaluate_expression_to_type(
                        module,
                        ctx.profile,
                        *value,
                        tree,
                        symbols,
                        types,
                    )?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from_any(ty, member_id.into_any())
                };

                // analyze default if present
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
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
                // assign this type for method bodies when available
                if let Some(this_ty_id) = this_ty_id {
                    let this_name = self.program.strings.intern("this");
                    let (_scope_id, scope, _mark) = symbols.get_scope(member_id, tree);
                    if let Some(this_symbol) = scope.find(StaticKey::Name(this_name)) {
                        types.set_value_type(this_symbol.into_global(module.id), this_ty_id);
                    }
                }

                // infer the method signature
                let declared_signature_ty_id =
                    types.get_signature_type_for_node(member_id.into_global_any(module.id));
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
                    ctx,
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
                        .in_function_with_signature(member_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the method body with implicit return typing
                    let body_ty_id = self
                        .infer_expression(module, *body, tree, symbols, types, infer, &mut ctx)?;

                    // constrain implicit return types against the declared return type
                    if let Some(return_ty_id) = return_type
                        && self.has_implicit_return(*body, tree)
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
        // walk generics
        let where_clauses = signature
            .generics
            .as_ref()
            .and_then(|generics| generics.where_clauses.as_deref());
        self.infer_where_clauses_maybe(
            module,
            where_clauses,
            tree,
            symbols,
            types,
            infer,
            ctx,
        )?;

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
        let this_parameter = if let Some(this_parameter_id) = signature.this_parameter {
            let declared_ty_id =
                types.get_declared_type_id(this_parameter_id.into_global_any(module.id));
            let param_symbol = tree.get(this_parameter_id).symbol().into_global(module.id);
            let param_ty_id = declared_ty_id.unwrap_or_else(|| {
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
        } else {
            None
        };

        // dynamic parameters
        let mut dynamic_param_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let declared_ty_id =
                types.get_declared_type_id(parameter_id.into_global_any(module.id));
            let expected_param_ty_id = expected_signature
                .as_ref()
                .and_then(|signature| signature.dynamic_parameters.get(index).copied());

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
        let return_type = if let Some(return_type_expr_id) = signature.return_type {
            Some(self.try_evaluate_expression_to_type(
                module,
                ctx.profile,
                return_type_expr_id,
                tree,
                symbols,
                types,
            )?)
        } else if let Some(return_type) =
            expected_signature.and_then(|signature| signature.return_type)
        {
            Some(return_type)
        } else {
            Some(self.infer_var_type_for_node(
                infer,
                types,
                node_id.into_global(module.id),
                InferOrigin::Return(node_id.into_global(module.id)),
                scope,
            ))
        };

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
                // Infer structural type from JSON value
                Ok(json_value_to_type(
                    value,
                    source_node,
                    types,
                    &self.program.strings,
                ))
            }
            ModuleContent::Text { .. } => {
                // Text imports are always string
                Ok(types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String),
                    },
                    source_node,
                ))
            }
            ModuleContent::Binary { .. } => {
                // Binary imports are uint8[] (Uint8Array on JS targets)
                let element_type = types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
                    },
                    source_node,
                );
                Ok(types.insert_type_from_any(
                    Type::Array {
                        element: Some(element_type),
                    },
                    source_node,
                ))
            }
            ModuleContent::Code(_) | ModuleContent::Unloaded => {
                // Shouldn't happen - code modules don't reach this path
                // Return unknown type as fallback
                Ok(types.insert_type_from_any(
                    Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    },
                    source_node,
                ))
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
        match clause {
            WhereClause::Assertion { left: _, right } => {
                // NOTE #Incomplete: lower where clauses into constraints
                self.infer_expression(module, *right, tree, symbols, types, infer, ctx)?;
            }
            WhereClause::Guard { guard } => {
                // NOTE #Incomplete: apply guard constraints to the flow context
                self.infer_expression(module, *guard, tree, symbols, types, infer, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer an enum field.
    pub(super) fn infer_enum_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<EnumField>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        if let Some(value) = field.value {
            // infer explicit enum value expression
            self.infer_expression(module, value, tree, symbols, types, infer, ctx)?;
        }
        Ok(())
    }

    /// Infer the enum backing type from member values.
    pub(super) fn infer_enum_backing_type(
        &self,
        module: &Module,
        profile: ProfileId,
        fields: &[LocalNodeId<EnumField>],
        tree: &NodeTree,
        types: &TypeTable,
    ) -> AnalyzeResult<EnumBackingType> {
        // start with no backing type chosen
        let mut backing_type: Option<EnumBackingType> = None;

        // walk explicit enum member values to decide backing type
        for field_id in fields {
            let field = tree.get(*field_id);
            let Some(value_id) = field.value else {
                continue;
            };

            // read the enum member value type
            let value_type_id = types
                .get_declared_or_inferred_type_id(value_id.into_global_any(module.id))
                .ok_or(AnalyzeError::MissingType {
                    node: value_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                })?;
            let value_type = types.get_type(value_type_id);

            // map the value type to a backing type
            let field_backing_type = match value_type {
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
                } => Some(EnumBackingType::Int(int_type.simplify())),
                Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
                } => Some(EnumBackingType::Int(
                    IntType::Arbitrary {
                        width: self.options.default_int_width,
                        is_signed: true,
                    }
                    .simplify(),
                )),
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                }
                | Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_)),
                } => Some(EnumBackingType::String),
                _ => None,
            };

            let Some(field_backing_type) = field_backing_type else {
                return Err(AnalyzeError::InvalidEnumBackingType {
                    node: value_id
                        .into_global_any(module.id)
                        .into_anchored(Some(profile)),
                    ty: value_type_id.into_global(module.id),
                });
            };

            // ensure backing type consistency across members
            if let Some(existing_type) = backing_type {
                let matches = match (existing_type, field_backing_type) {
                    (EnumBackingType::Int(left), EnumBackingType::Int(right)) => left == right,
                    (EnumBackingType::String, EnumBackingType::String) => true,
                    _ => false,
                };

                if !matches {
                    return Err(AnalyzeError::InvalidEnumBackingType {
                        node: value_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                        ty: value_type_id.into_global(module.id),
                    });
                }
            } else {
                backing_type = Some(field_backing_type);
            }
        }

        // default to the configured integer width when unspecified
        let backing_type = backing_type.unwrap_or_else(|| {
            EnumBackingType::Int(
                IntType::Arbitrary {
                    width: self.options.default_int_width,
                    is_signed: true,
                }
                .simplify(),
            )
        });

        Ok(backing_type)
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

        // type check: if both declared and inferred, check assignability
        if let (Some(declared), Some(inferred)) = (declared_ty_id, inferred_ty_id) {
            // apply inference constraints when required
            if matches!(constraint, DeclaratorConstraint::Assignable) {
                infer.push_constraint(Constraint::Subtype {
                    sub_type: inferred,
                    super_type: declared,
                    variance: None,
                });
            }

            if !self.is_infer_var_type(declared, types)
                && !self.is_infer_var_type(inferred, types)
                && self.is_type_assignable(
                    module,
                    ctx.profile,
                    symbols,
                    declared,
                    inferred,
                    types,
                    &options,
                ) == Assignability::NotAssignable
            {
                let error = match constraint {
                    DeclaratorConstraint::Assignable => AnalyzeError::UnassignableType {
                        node: declarator_id.into_global(module.id).into(),
                        expected_ty: declared.into_global(module.id),
                        actual_ty: inferred.into_global(module.id),
                    },
                    DeclaratorConstraint::Satisfies => AnalyzeError::UnsatisfiedType {
                        node: declarator_id.into_global(module.id).into(),
                        expected_ty: declared.into_global(module.id),
                        actual_ty: inferred.into_global(module.id),
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

        Ok(())
    }
}
