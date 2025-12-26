use crate::{
    AnalyzeError, AnalyzeResult, Assignability, Compiler, Constraint, InferContext, InferOrigin,
    InferScope, InferTable,
};
use destack_dir::{
    Declaration, DeclarationAbstraction, Declarator, DependencyItem, DynamicKey, EnumField,
    Expression, Extension, ExtensionKind, FunctionSignature, Generics, GlobalSymbolId, Heritage,
    Lineage, LocalNodeId, LocalNodeIdAny, LocalSymbolId, LocalTypeId, Member, NodeTree, Parameter,
    StaticKey, SymbolTable, Type, TypeField, TypeKind, TypeLiteral, TypeTable, WhereClause,
};
use destack_workspace::Module;

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
        let declaration = tree.get(declaration_id);
        match declaration {
            // namespace
            Declaration::Namespace {
                descriptor: _,
                generics,
                scope: _,
                expressions,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
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
            Declaration::Type {
                descriptor,
                kind,
                mutability: _,
                static_parameters,
                value,
            } => {
                // walk
                if let Some(parameters) = static_parameters {
                    for parameter_id in parameters {
                        self.infer_parameter(
                            module,
                            *parameter_id,
                            None,
                            tree,
                            symbols,
                            types,
                            infer,
                            ctx,
                        )?;
                    }
                }

                // type instance type: type value
                let instance_ty_id =
                    self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                match *kind {
                    TypeKind::Structural => {
                        types.set_instance_type(
                            descriptor.symbol.into_global(module.id),
                            instance_ty_id,
                        );
                    }
                    TypeKind::Nominal => {
                        let ty = Type::Reference {
                            symbol: descriptor.symbol.into_global(module.id),
                            static_arguments: None,
                        };
                        let ty_id = types.insert_type_from(ty, declaration_id);
                        types.set_instance_type(descriptor.symbol.into_global(module.id), ty_id);
                    }
                }

                // type value type: type metatype
                let value_ty = Type::Value {
                    value: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // struct
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) = self
                        .infer_member(module, *member_id, tree, symbols, types, infer, ctx, None)?
                    {
                        fields.push(field);
                    }
                }

                // struct instance type: object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // struct nominal type: reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // struct value type: nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // class
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // context
                let is_abstract = descriptor.abstraction == DeclarationAbstraction::Abstract;
                let mut ctx = ctx.fork().in_abstract_class_maybe(is_abstract);

                // type fields
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) = self.infer_member(
                        module, *member_id, tree, symbols, types, infer, &mut ctx, None,
                    )? {
                        fields.push(field);
                    }
                }

                // class instance type: object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // class nominal type: reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // class value type: nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind: _,
                generics,
                heritage,
                scope: _,
                fields,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                // enum instance type: nominal enum value
                let enum_symbol = descriptor.symbol.into_global(module.id);
                let instance_ty = Type::Reference {
                    symbol: enum_symbol,
                    static_arguments: None,
                };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(enum_symbol, instance_ty_id);

                // enum fields are exposed on the enum value
                let mut value_fields = Vec::new();
                for field_id in fields {
                    self.infer_enum_field(module, *field_id, tree, symbols, types, infer, ctx)?;

                    let field = tree.get(*field_id);
                    let field_symbol = field.symbol.into_global(module.id);
                    types.set_value_type(field_symbol, instance_ty_id);

                    value_fields.push(TypeField {
                        key: StaticKey::Name(field.name),
                        ty: instance_ty_id,
                        is_optional: false,
                        is_readonly: true,
                    });
                }

                for member_id in members {
                    self.infer_member(module, *member_id, tree, symbols, types, infer, ctx, None)?;
                }

                // enum value type: object with variant fields
                let value_ty = Type::Object {
                    fields: value_fields,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(enum_symbol, value_ty_id);
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                target_symbol,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_expression(module, *target_type, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol), // (put lineage on extension symbol itself)
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // assign this to the target type when available
                let this_ty_id = target_symbol.and_then(|target| {
                    types.get_instance_type_id(target).or_else(|| {
                        let ty = Type::Reference {
                            symbol: target,
                            static_arguments: None,
                        };
                        Some(types.insert_type_from(ty, declaration_id))
                    })
                });

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) = self.infer_member(
                        module, *member_id, tree, symbols, types, infer, ctx, this_ty_id,
                    )? {
                        fields.push(field);
                    }
                }

                // extension instance type: object type with its methods
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                let extension_symbol = descriptor.symbol.into_global(module.id);
                types.set_instance_type(extension_symbol, instance_ty_id);

                // register extension
                if let Some(target) = target_symbol {
                    let kind = if module.id == target.module_id {
                        ExtensionKind::Inherent
                    } else if descriptor.name.is_some() {
                        ExtensionKind::Nominal
                    } else {
                        ExtensionKind::Local
                    };
                    let lineage = types.get_lineage_id_for_symbol(extension_symbol);
                    let extension = Extension::new(extension_symbol, kind, *target, lineage);
                    types.insert_extension(extension);
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind: _,
                generics,
                heritage,
                scope: _,
                members,
            } => {
                // walk
                self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
                self.infer_heritage(
                    module,
                    heritage,
                    declaration_id.into_any(),
                    Some(descriptor.symbol),
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;

                // collect type fields from members
                let mut fields = Vec::new();
                for member_id in members {
                    if let Some(field) = self
                        .infer_member(module, *member_id, tree, symbols, types, infer, ctx, None)?
                    {
                        fields.push(field);
                    }
                }

                // interface instance type: object type with fields
                let instance_ty = Type::Object { fields };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // interface nominal type: reference type
                let nominal_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let nominal_ty_id = types.insert_type_from(nominal_ty, declaration_id);

                // interface value type: nominal type
                let value_ty = Type::Value {
                    value: nominal_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                scope: _,
                body,
            } => {
                let fn_ty_id = self.infer_signature(
                    module,
                    declaration_id.into_any(),
                    descriptor.symbol.into_global(module.id),
                    signature,
                    ctx.expected_type,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
                // register the function type as the value_type for the function's symbol #Suspicious
                types.set_value_type(descriptor.symbol.into_global(module.id), fn_ty_id);

                if let Some(body) = body {
                    let return_type = self.function_return_type(fn_ty_id, types);
                    let ctx = ctx
                        .reset()
                        .in_function_with_signature(declaration_id.into_any(), signature);
                    let mut ctx = ctx
                        .with_return_type(return_type)
                        .with_expected_type(return_type);

                    // infer the function body with implicit return typing
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
                            && self.check_is_type_assignable(return_ty_id, body_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: body.into_global_any(module.id),
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
    ) -> AnalyzeResult<Option<TypeField>> {
        let member = tree.get(member_id);
        match member {
            Member::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                // extract the static key from the dynamic key
                let static_key = key.and_then(|k| match k {
                    DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                    // dynamic keys can't be used for static type inference
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });

                // evaluate the field type
                let value_ty_id = if let Some(value) = value {
                    self.try_evaluate_expression_to_type(module, *value, tree, symbols, types)?
                } else {
                    // no value, return unknown type
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type(ty)
                };

                // analyze default if present
                if let Some(default) = default {
                    self.infer_expression(module, *default, tree, symbols, types, infer, ctx)?;
                }

                // check if the field is optional
                let is_optional = modifiers
                    .as_ref()
                    .is_some_and(|m| matches!(m.kind, Some(destack_dir::BindingKind::Maybe)));

                // check if the field is readonly
                let is_readonly = modifiers.as_ref().is_some_and(|m| {
                    matches!(m.mutability, Some(destack_dir::Mutability::Immutable))
                });

                // only return a field if we have a static key
                if let Some(key) = static_key {
                    Ok(Some(TypeField {
                        key,
                        ty: value_ty_id,
                        is_optional,
                        is_readonly,
                    }))
                } else {
                    Ok(None)
                }
            }
            Member::Method {
                key,
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

                // extract the static key from the dynamic key
                let static_key = key.and_then(|k| match k {
                    DynamicKey::Name(name) => Some(StaticKey::Name(name)),
                    DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
                });

                // signature
                let method_ty_id = self.infer_signature(
                    module,
                    member_id.into_any(),
                    member.symbol().into_global(module.id),
                    signature,
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
                            && self.check_is_type_assignable(return_ty_id, body_ty_id, types)
                                == Assignability::NotAssignable
                        {
                            self.error(AnalyzeError::UnassignableType {
                                node: body.into_global_any(module.id),
                                expected_ty: return_ty_id.into_global(module.id),
                                actual_ty: body_ty_id.into_global(module.id),
                            });
                        }
                    }
                }

                // return a field if we have a static key
                if let Some(key) = static_key {
                    Ok(Some(TypeField {
                        key,
                        ty: method_ty_id,
                        is_optional: false,
                        is_readonly: true,
                    }))
                } else {
                    Ok(None)
                }
            }
            Member::Embed { value, .. } => {
                // NOTE #Incomplete: expand embedded type into member fields?
                self.infer_expression(module, *value, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
            Member::StaticBlock { body, .. } => {
                self.infer_expression(module, *body, tree, symbols, types, infer, ctx)?;
                Ok(None)
            }
        }
    }

    /// Infer generics.
    pub(super) fn infer_generics(
        &self,
        module: &Module,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        if let Some(static_parameters) = &generics.static_parameters {
            for parameter_id in static_parameters {
                self.infer_parameter(
                    module,
                    *parameter_id,
                    None,
                    tree,
                    symbols,
                    types,
                    infer,
                    ctx,
                )?;
            }
        }
        if let Some(clauses) = &generics.where_clauses {
            for clause_id in clauses {
                self.infer_where_clause(module, *clause_id, tree, symbols, types, infer, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer heritage.
    pub(super) fn infer_heritage(
        &self,
        module: &Module,
        heritage: &Heritage,
        _node_id: LocalNodeIdAny,
        symbol: Option<LocalSymbolId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        _infer: &mut InferTable,
        _ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        // resolve heritage targets from evaluated types when possible
        let collect_symbol = |expression_id: LocalNodeId<Expression>,
                              symbols: &SymbolTable,
                              types: &mut TypeTable|
         -> AnalyzeResult<Option<GlobalSymbolId>> {
            let ty_id =
                self.try_evaluate_expression_to_type(module, expression_id, tree, symbols, types)?;
            let ty = types.get_type(ty_id);
            let type_symbol = match ty {
                Type::Reference { symbol, .. } => Some(*symbol),
                Type::Value { value } => match types.get_type(*value) {
                    Type::Reference { symbol, .. } => Some(*symbol),
                    _ => None,
                },
                _ => None,
            };

            if let Some(type_symbol) = type_symbol {
                return Ok(Some(type_symbol));
            }

            let expression = tree.get(expression_id);
            Ok(expression.target_symbol())
        };

        // analyze and extract extends symbols
        let mut extends_symbols = Vec::new();
        if let Some(extend_types) = &heritage.extends_types {
            for expression_id in extend_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(module, symbols, target_symbol);
                    extends_symbols.push(canonical_symbol);
                }
            }
        }

        // analyze and extract implements symbols
        let mut implements_symbols = Vec::new();
        if let Some(implements_types) = &heritage.implements_types {
            for expression_id in implements_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(module, symbols, target_symbol);
                    implements_symbols.push(canonical_symbol);
                }
            }
        }

        // analyze and extract embedded symbols
        let mut embedded_symbols = Vec::new();
        if let Some(embedded_types) = &heritage.embedded_types {
            for expression_id in embedded_types {
                if let Some(target_symbol) = collect_symbol(*expression_id, symbols, types)? {
                    let canonical_symbol = self.canonical_symbol_id(module, symbols, target_symbol);
                    embedded_symbols.push(canonical_symbol);
                }
            }
        }

        // build and store lineage if we have a declaring symbol
        if let Some(symbol) = symbol {
            // remember lineage (validation happens in validate phase)
            let lineage = Lineage {
                extends: extends_symbols.first().copied(),
                implements: implements_symbols,
                embedded: embedded_symbols,
            };
            if !lineage.is_empty() {
                let lineage_id = types.insert_lineage(lineage);
                types.set_lineage_for_symbol(symbol.into_global(module.id), lineage_id);
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
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // walk generics
        if let Some(generics) = &signature.generics {
            self.infer_generics(module, generics, tree, symbols, types, infer, ctx)?;
        }

        // collect static parameter placeholders
        let static_parameters =
            self.collect_static_parameter_placeholders(module, signature, tree, types);

        // extract any contextual function signature
        let expected_signature = self.expected_function_signature(expected_fn_ty_id, types);

        // collect parameter types
        let scope = InferScope {
            owner: owner_symbol,
            function_id: Some(node_id.into_global(module.id)),
        };

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

        // get return type
        let return_type = if let Some(return_type_expr_id) = signature.return_type {
            Some(self.try_evaluate_expression_to_type(
                module,
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
            return_type,
        };
        let ty_id = types.insert_type_from_any(ty, node_id);

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

    /// Infer an argument.
    pub(super) fn infer_dependency_item(
        &self,
        _module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
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
            DependencyItem::Remote { .. } => {
                // nothing to do
            }
        }
        Ok(())
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
            // infer explicit enum value expression when present
            self.infer_expression(module, value, tree, symbols, types, infer, ctx)?;
        }
        Ok(())
    }

    /// Infer a pattern, given an optional binding type of the pattern.
    pub(super) fn infer_declarator(
        &self,
        module: &Module,
        declarator_id: LocalNodeId<Declarator>,
        _let_expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<()> {
        let declarator = tree.get(declarator_id);
        let Declarator { pattern, ty, value } = declarator;

        // infer type from value or annotation
        // declared type is now on the declarator node, not the let expression
        let declared_ty_id =
            types.get_declared_type_id(declarator_id.into_global(module.id).into());
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
            infer.push_constraint(Constraint::Subtype {
                sub_type: inferred,
                super_type: declared,
                variance: None,
            });
            if !self.is_infer_var_type(declared, types)
                && !self.is_infer_var_type(inferred, types)
                && self.check_is_type_assignable(declared, inferred, types)
                    == Assignability::NotAssignable
            {
                return Err(AnalyzeError::UnassignableType {
                    node: declarator_id.into_global(module.id).into(),
                    expected_ty: declared.into_global(module.id),
                    actual_ty: inferred.into_global(module.id),
                });
            }
        }

        // analyze the type expression if present
        if let Some(ty_id) = ty {
            let _ = self.try_evaluate_expression_to_type(module, *ty_id, tree, symbols, types)?;
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
