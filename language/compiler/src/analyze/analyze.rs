use crate::{AnalyzeError, AnalyzeResult, Compiler, TypeContext};
use destack_dir::{
    Argument, BinaryOperator, Block, Declaration, DependencyItem, EnumField, Expression,
    FunctionSignature, Generics, Heritage, LocalNodeId, LocalTypeId, Module, Mutability, NodeTree,
    Parameter, Pattern, PatternField, PrimitiveType, Property, ScalarLiteral, SymbolTable, Type,
    TypeKind, TypeLiteral, TypeTable, UnaryOperator, VarianceBound, WhereClause,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Analyze an expression.
    pub(super) fn analyze_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(ty_id) = types.get_inferred_type_id(expression_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let expression = tree.get(expression_id);
        let ty_id: LocalTypeId = match expression {
            // declaration -> analyze the declaration
            Expression::Declaration { declaration } => {
                self.analyze_declaration(module, *declaration, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // block -> analyze the block
            Expression::Block { block } => {
                self.analyze_block(module, *block, tree, symbols, types, ctx)?
            }

            // statement -> analyze the statement
            Expression::Statement { statement } => {
                self.analyze_expression(module, *statement, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // import / exports
            Expression::Import {
                kind: _,
                target: _,
                target_module: _,
                items,
                arguments,
            }
            | Expression::UnresolvedImport {
                kind: _,
                target: _,
                items,
                arguments,
            } => {
                for item_id in items {
                    self.analyze_dependency_item(module, *item_id, tree, symbols, types, ctx)?;
                }
                if let Some(arguments) = arguments {
                    for argument_id in arguments {
                        self.analyze_argument(
                            module,
                            *argument_id,
                            None,
                            tree,
                            symbols,
                            types,
                            ctx,
                        )?;
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
                    self.analyze_dependency_item(module, *item_id, tree, symbols, types, ctx)?;
                }
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // let
            Expression::Let {
                descriptor: _,
                mutability: _,
                pattern,
                value,
            } => {
                // infer type from value or annotation
                let declared_ty_id =
                    types.get_declared_type_id(expression_id.into_global_any(module.id));
                let inferred_ty_id = if let Some(value) = value {
                    Some(self.analyze_expression(module, *value, tree, symbols, types, ctx)?)
                } else {
                    None
                };
                let binding_ty_id = declared_ty_id.or(inferred_ty_id);

                self.analyze_pattern(module, *pattern, binding_ty_id, tree, symbols, types, ctx)?;

                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // unary operations -> compound type
            // NOTE #Incomplete: resolve unary operator overloads
            Expression::Unary { operator, right } => {
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = self.analyze_unary_operation(operator, types.get_type(right_ty_id));
                types.insert_type_from(ty, expression_id)
            }
            // value of operation -> value of type
            Expression::ValueOf {
                mutability,
                variance,
                right,
            } => {
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = self.analyze_value_of_operation(
                    *mutability,
                    *variance,
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }
            // reference of operation -> reference of type
            Expression::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = self.analyze_reference_of_operation(
                    *mutability,
                    *variance,
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }
            // binary operations -> compound type
            // NOTE #Incomplete: resolve binary operator overloads
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty_id =
                    self.analyze_expression(module, *left, tree, symbols, types, ctx)?;
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;

                let ty = self.analyze_binary_operation(
                    operator,
                    types.get_type(left_ty_id),
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }
            // assignment operations -> void
            Expression::Assign { left, right } => {
                let _left_ty_id =
                    self.analyze_expression(module, *left, tree, symbols, types, ctx)?;
                let _right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::AssignBinary {
                left,
                operator: _,
                right,
            } => {
                let _left_ty_id =
                    self.analyze_expression(module, *left, tree, symbols, types, ctx)?;
                let _right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // delete operation -> void
            Expression::Delete { value } => {
                let _value_ty_id =
                    self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Void,
                };
                types.insert_type_from(ty, expression_id)
            }

            // references -> look up symbol type
            Expression::UnresolvedAbsolutePath {
                path: _,
                static_arguments: _,
            } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }
            Expression::UnresolvedRelativePath {
                path: _,
                target_symbol: _,
                remaining_path: _,
                static_arguments: _,
            } => {
                let ty = Type::TypeLiteral {
                    value: TypeLiteral::Unknown,
                };
                types.insert_type_from(ty, expression_id)
            }
            // NOTE #Incomplete: instantiate with static arguments
            Expression::LocalReference {
                path: _,
                target_symbol,
                static_arguments: _,
            }
            | Expression::ModuleReference {
                path: _,
                target_symbol,
                static_arguments: _,
            }
            | Expression::GlobalReference {
                path: _,
                target_symbol,
                static_arguments: _,
            } => {
                // TODO #Broken: resolve symbol type across modules
                // narrow symbol type in context
                if let Some(narrowed_ty_id) = ctx.get_narrowed(*target_symbol) {
                    narrowed_ty_id
                }
                // symbol value type
                else if let Some(value_ty_id) = types.get_value_type_id(*target_symbol) {
                    value_ty_id
                }
                // unknown type
                else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
            }

            // scalar literal -> derive type from value
            Expression::ScalarLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: self.analyze_scalar_literal(value),
                };
                types.insert_type_from(ty, expression_id)
            }
            // type literal -> use the given type literal?
            // nocheckin ???
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // type as a value -> type
            Expression::Type { value } => {
                let ty = Type::Value { value: *value };
                types.insert_type_from(ty, expression_id)
            }
            // array expression -> array type over each element
            Expression::ArrayExpression { elements } => {
                for element_id in elements {
                    self.analyze_argument(module, *element_id, None, tree, symbols, types, ctx)?;
                }
                let last_element_ty_id = elements
                    .last()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        let element_id = element.value();
                        self.analyze_expression(module, element_id, tree, symbols, types, ctx)
                    })
                    .transpose()?;
                let ty = Type::Array {
                    element: last_element_ty_id,
                };
                types.insert_type_from(ty, expression_id)
            }
            // tuple expression -> tuple type for each element
            Expression::TupleExpression { elements } => {
                for element_id in elements {
                    self.analyze_argument(module, *element_id, None, tree, symbols, types, ctx)?;
                }
                let element_tys: Vec<LocalTypeId> = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        let element_id = element.value();
                        self.analyze_expression(module, element_id, tree, symbols, types, ctx)
                    })
                    .collect::<Result<Vec<_>, AnalyzeError>>()?;
                let ty = Type::Tuple {
                    elements: element_tys,
                };
                types.insert_type_from(ty, expression_id)
            }
            // parenthesized -> same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.analyze_expression(module, *inner_id, tree, symbols, types, ctx)?,

            // fallback
            _ => {
                return Err(AnalyzeError::UnsupportedNode {
                    node: expression_id.into_global_any(module.id),
                });
            }
        };

        types.set_inferred_type(expression_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Analyze a block.
    fn analyze_block(
        &self,
        module: &Module,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<LocalTypeId> {
        if let Some(ty_id) = types.get_inferred_type_id(block_id.into_global_any(module.id)) {
            return Ok(ty_id);
        }

        let block = tree.get(block_id);
        for expression_id in &block.expressions {
            self.analyze_expression(module, *expression_id, tree, symbols, types, ctx)?;
        }

        // type is last expression type
        let ty_id = if let Some(last_expression_id) = block.expressions.last() {
            self.analyze_expression(module, *last_expression_id, tree, symbols, types, ctx)?
        } else {
            let ty = Type::TypeLiteral {
                value: TypeLiteral::Void,
            };
            types.insert_type_from(ty, block_id)
        };

        types.set_inferred_type(block_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Analyze a declaration.
    fn analyze_declaration(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
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
                self.analyze_generics(module, generics, tree, symbols, types, ctx)?;
                for expression_id in expressions {
                    self.analyze_expression(module, *expression_id, tree, symbols, types, ctx)?;
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
                        self.analyze_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
                    }
                }

                // type instance type -> type value
                let instance_ty_id =
                    self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
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

                // type value type -> type metatype
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
                properties,
            } => {
                // walk
                self.analyze_generics(module, generics, tree, symbols, types, ctx)?;
                self.analyze_heritage(module, heritage, tree, symbols, types, ctx)?;
                for property_id in properties {
                    self.analyze_property(module, *property_id, tree, symbols, types, ctx)?;
                }

                // struct instance type -> object type
                let instance_ty = Type::Object { fields: vec![] }; // nocheckin ???
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // struct instance type -> struct value type
                let value_ty = Type::Value {
                    value: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // enum
            Declaration::Enum {
                descriptor,
                generics,
                heritage,
                scope: _,
                fields,
                properties,
            } => {
                // walk
                self.analyze_generics(module, generics, tree, symbols, types, ctx)?;
                self.analyze_heritage(module, heritage, tree, symbols, types, ctx)?;
                for field_id in fields {
                    self.analyze_enum_field(module, *field_id, tree, symbols, types, ctx)?;
                }
                for property_id in properties {
                    self.analyze_property(module, *property_id, tree, symbols, types, ctx)?;
                }

                // enum instance type -> enum value type
                // nocheckin ???
                let instance_ty = Type::Reference {
                    symbol: descriptor.symbol.into_global(module.id),
                    static_arguments: None,
                };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // enum instance type -> enum value type
                let value_ty = Type::Value {
                    value: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);
            }

            // extension
            Declaration::Extension {
                descriptor: _,
                generics,
                target_type,
                target_symbol: _,
                heritage,
                scope: _,
                properties,
            } => {
                // walk
                self.analyze_generics(module, generics, tree, symbols, types, ctx)?;
                self.analyze_expression(module, *target_type, tree, symbols, types, ctx)?;
                self.analyze_heritage(module, heritage, tree, symbols, types, ctx)?;
                for property_id in properties {
                    self.analyze_property(module, *property_id, tree, symbols, types, ctx)?;
                }
            }

            _ => {
                return Err(AnalyzeError::UnsupportedNode {
                    node: declaration_id.into_global_any(module.id),
                });
            }
        }

        Ok(())
    }

    /// Analyze a property.
    fn analyze_property(
        &self,
        module: &Module,
        property_id: LocalNodeId<Property>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let property = tree.get(property_id);
        match property {
            Property::Field { value, default, .. } => {
                if let Some(value) = value {
                    self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
                }
                if let Some(default) = default {
                    self.analyze_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            Property::Method { body, .. } => {
                if let Some(body) = body {
                    self.analyze_expression(module, *body, tree, symbols, types, ctx)?;
                }
            }
            Property::Spread { value, .. } => {
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze generics.
    fn analyze_generics(
        &self,
        module: &Module,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        if let Some(static_parameters) = &generics.static_parameters {
            for parameter_id in static_parameters {
                self.analyze_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(clauses) = &generics.where_clauses {
            for clause_id in clauses {
                self.analyze_where_clause(module, *clause_id, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze heritage.
    fn analyze_heritage(
        &self,
        module: &Module,
        heritage: &Heritage,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        if let Some(extend_types) = &heritage.extends_types {
            for expression_id in extend_types {
                self.analyze_expression(module, *expression_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(implements_types) = &heritage.implements_types {
            for expression_id in implements_types {
                self.analyze_expression(module, *expression_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(embedded_types) = &heritage.embedded_types {
            for expression_id in embedded_types {
                self.analyze_expression(module, *expression_id, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze a function signature.
    fn analyze_signature(
        &self,
        module: &Module,
        signature: &FunctionSignature,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        // walk generics in signature
        if let Some(generics) = &signature.generics {
            self.analyze_generics(module, generics, tree, symbols, types, ctx)?;
        }
        // walk dynamic parameters
        for parameter_id in &signature.dynamic_parameters {
            self.analyze_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
        }
        // walk return type
        if let Some(return_type_id) = signature.return_type {
            self.analyze_expression(module, return_type_id, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze a parameter.
    fn analyze_parameter(
        &self,
        module: &Module,
        parameter_id: LocalNodeId<Parameter>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let parameter = tree.get(parameter_id);
        match parameter {
            Parameter::Named {
                modifiers: _,
                name: _,
                default,
                symbol: _,
            } => {
                if let Some(default) = default {
                    self.analyze_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            Parameter::Pattern {
                modifiers: _,
                pattern,
                default,
                symbol: _,
            } => {
                // infer type from default if present
                let default_ty_id = if let Some(default) = default {
                    Some(self.analyze_expression(module, *default, tree, symbols, types, ctx)?)
                } else {
                    None
                };
                self.analyze_pattern(module, *pattern, default_ty_id, tree, symbols, types, ctx)?;
            }
            Parameter::Variadic {
                modifiers: _,
                name: _,
                symbol: _,
            } => {
                // nothing to do
            }
        }
        Ok(())
    }

    /// Analyze an argument.
    fn analyze_argument(
        &self,
        module: &Module,
        argument_id: LocalNodeId<Argument>,
        _binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let argument = tree.get(argument_id);
        match argument {
            Argument::Positional { value } => {
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Named { name: _, value } => {
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Spread { value } => {
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Argument::Dynamic { key, value } => {
                self.analyze_expression(module, *key, tree, symbols, types, ctx)?;
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze a dependency item.
    fn analyze_dependency_item(
        &self,
        _module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        _types: &mut TypeTable,
        _ctx: &mut TypeContext,
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

    /// Analyze where clause.
    fn analyze_where_clause(
        &self,
        module: &Module,
        clause_id: LocalNodeId<WhereClause>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let clause = tree.get(clause_id);
        match clause {
            WhereClause::Assertion { left: _, right } => {
                self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
            }
            WhereClause::Guard { guard } => {
                self.analyze_expression(module, *guard, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze an enum field.
    fn analyze_enum_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<EnumField>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        if let Some(value) = field.value {
            self.analyze_expression(module, value, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze a pattern, given an optional binding type of the pattern.
    fn analyze_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let pattern = tree.get(pattern_id);
        match pattern {
            Pattern::Wildcard => {
                // nothing to do
            }
            Pattern::Rest { name: _, symbol } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
            }
            Pattern::Maybe(pattern_id) => {
                self.analyze_pattern(
                    module,
                    *pattern_id,
                    binding_ty_id,
                    tree,
                    symbols,
                    types,
                    ctx,
                )?;
            }
            Pattern::ReferenceOf {
                mutability: _,
                right,
            } => {
                self.analyze_pattern(module, *right, binding_ty_id, tree, symbols, types, ctx)?;
            }
            Pattern::ValueOf {
                mutability: _,
                right,
            } => {
                self.analyze_pattern(module, *right, binding_ty_id, tree, symbols, types, ctx)?;
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
                    self.analyze_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::Expression { value } => {
                self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
            }
            Pattern::Range {
                start,
                end,
                is_inclusive: _,
            } => {
                if let Some(start_pattern_id) = start {
                    self.analyze_pattern(
                        module,
                        *start_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
                if let Some(end_pattern_id) = end {
                    self.analyze_pattern(
                        module,
                        *end_pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::Tuple { fields } => {
                for field_id in fields {
                    self.analyze_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedTuple { ty, fields } => {
                self.analyze_expression(module, *ty, tree, symbols, types, ctx)?;
                for field_id in fields {
                    self.analyze_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::Slice { fields } => {
                for field_id in fields {
                    self.analyze_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::Object { fields } => {
                for field_id in fields {
                    self.analyze_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::TaggedObject { ty, fields } => {
                self.analyze_expression(module, *ty, tree, symbols, types, ctx)?;
                for field_id in fields {
                    self.analyze_pattern_field(
                        module,
                        *field_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Pattern::Union { patterns } => {
                for pattern_id in patterns {
                    self.analyze_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Analyze a pattern field and propagate type to bound symbol.
    fn analyze_pattern_field(
        &self,
        module: &Module,
        field_id: LocalNodeId<PatternField>,
        binding_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let field = tree.get(field_id);
        match field {
            PatternField::Named {
                mutability: _,
                name: _,
                default: _,
                symbol,
                pattern,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
                if let Some(pattern_id) = pattern {
                    self.analyze_pattern(
                        module,
                        *pattern_id,
                        binding_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            PatternField::Alias {
                mutability: _,
                name: _,
                alias: _,
                default,
                symbol,
            } => {
                if let Some(ty_id) = binding_ty_id {
                    types.set_value_type(symbol.into_global(module.id), ty_id);
                }
                if let Some(default) = default {
                    self.analyze_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            PatternField::Positional { pattern } => {
                self.analyze_pattern(module, *pattern, binding_ty_id, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Infer the result type of a scalar literal.
    fn analyze_scalar_literal(&self, value: &ScalarLiteral) -> TypeLiteral {
        match value {
            ScalarLiteral::Boolean(_) => TypeLiteral::Primitive(PrimitiveType::Boolean),
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) => {
                TypeLiteral::Primitive(PrimitiveType::Number)
            }
            ScalarLiteral::String(_) | ScalarLiteral::RegexString { .. } => {
                TypeLiteral::Primitive(PrimitiveType::String)
            }
            ScalarLiteral::Bigint(_) => TypeLiteral::Primitive(PrimitiveType::Bigint),
            ScalarLiteral::Character(_) => TypeLiteral::Primitive(PrimitiveType::Character),
        }
    }

    /// Infer the result type of a binary operation.
    fn analyze_binary_operation(
        &self,
        operator: &BinaryOperator,
        left: &Type,
        _right: &Type,
    ) -> Type {
        match operator {
            // comparison operators always return boolean
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::EqualStrict
            | BinaryOperator::NotEqualStrict
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::In
            | BinaryOperator::InstanceOf => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },

            // logical operators return boolean
            BinaryOperator::And | BinaryOperator::Or => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },

            _ => left.clone(),
        }
    }

    /// Infer the result type of a unary operation.
    fn analyze_unary_operation(&self, operator: &UnaryOperator, right: &Type) -> Type {
        match operator {
            UnaryOperator::Not => Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            },
            _ => right.clone(),
        }
    }

    /// Infer the result type of a value of operation.
    /// NOTE #Incomplete: resolve value of operation type
    fn analyze_value_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        right.clone()
    }

    /// Infer the result type of a reference of operation.
    /// NOTE #Incomplete: resolve reference of operation type
    fn analyze_reference_of_operation(
        &self,
        _mutability: Option<Mutability>,
        _variance: Option<VarianceBound>,
        right: &Type,
    ) -> Type {
        right.clone()
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{PrimitiveType, Type, TypeLiteral};

    use crate::{ImportTask, TestProgram};

    #[test]
    fn test_analyze_number_literal() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "42");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }

    #[test]
    fn test_analyze_string_literal() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", r#""hello""#);
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );
    }

    #[test]
    fn test_analyze_boolean_literal() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "true");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean)
            }
        );
    }

    #[test]
    fn test_analyze_binary_number_operation() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "1 + 2");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }

    #[test]
    fn test_analyze_binary_number_comparison() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "1 < 2");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean)
            }
        );
    }

    #[test]
    fn test_analyze_let_expression_infer_type() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "let x = 42");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let let_expr_id = module.roots[0];
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

        // no declared type
        assert!(
            types
                .get_declared_type(let_expr_id.into_global_any(module.id))
                .is_none()
        );

        // value_type[x] = number
        let value_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );

        // instance_type[x] = undefined
        assert!(types.get_instance_type(x_symbol).is_none());
    }

    #[test]
    fn test_analyze_let_expression_declare_type() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "let x: string = 42");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let let_expr_id = module.roots[0];
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

        // declared_type[let_expr] = string
        let declared = types
            .get_declared_type(let_expr_id.into_global_any(module.id))
            .unwrap();
        assert_eq!(
            *declared,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // value_type[x] = string
        let value_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // instance_type[x] = undefined
        assert!(types.get_instance_type(x_symbol).is_none());
    }

    #[test]
    fn test_analyze_let_expression_infer_tuple_type_with_pattern() {
        let test = TestProgram::memory_sequential();
        let file = test.file(
            "test.ds",
            r#"
let (x, y, ...rest) = (123, 'abc', true);
"#,
        );
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
        let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
    
        // value_type[x] = number
        let value_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
        
        // value_type[y] = string
        let value_ty = types.get_value_type(y_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // value_type[rest] = (boolean,)
        let value_ty = types.get_value_type(rest_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean)
            }
        );
    }
}
