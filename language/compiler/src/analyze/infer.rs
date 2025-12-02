use crate::{AnalyzeResult, Compiler, TypeContext};
use destack_dir::{
    BinaryOperator, Declaration, EnumField, Expression, FunctionSignature, Generics, Heritage,
    LocalNodeId, LocalTypeId, Module, NodeTree, Parameter, Pattern, PrimitiveType, Property,
    ScalarLiteral, SymbolTable, Type, TypeLiteral, TypeTable, WhereClause, WithClause,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Analyze an expression: infer its type, resolve overloads, instantiate instances.
    pub(super) fn analyze_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // return cached type if already analyzed
        if let Some(ty_id) =
            types.get_inferred_type_id_for_node(expression_id.into_global_any(module.id))
        {
            return Ok(ty_id);
        }

        let expression = tree.get(expression_id);
        let ty = self.infer_expression(module, expression, tree, symbols, types, ctx)?;

        // store the inferred type
        let ty_id = types.insert_type_from(ty, expression_id);
        types.set_inferred_type_for_node(expression_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Analyze a declaration.
    pub(super) fn analyze_declaration(
        &self,
        module: &Module,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<LocalTypeId> {
        // return cached type if already analyzed
        if let Some(ty_id) =
            types.get_inferred_type_id_for_node(declaration_id.into_global_any(module.id))
        {
            return Ok(ty_id);
        }

        let declaration = tree.get(declaration_id);
        let ty = self.infer_declaration(
            module,
            declaration,
            declaration_id,
            tree,
            symbols,
            types,
            ctx,
        )?;

        // store the inferred type
        let ty_id = types.insert_type_from(ty, declaration_id);
        types.set_inferred_type_for_node(declaration_id.into_global_any(module.id), ty_id);

        Ok(ty_id)
    }

    /// Compute the type of an expression.
    fn infer_expression(
        &self,
        module: &Module,
        expression: &Expression,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<Type> {
        let ty = match expression {
            // scalar literal: derive type from value
            Expression::ScalarLiteral { value } => {
                Type::Scalar(scalar_literal_to_type_literal(value))
            }
            // type literal
            Expression::TypeLiteral { value } => Type::Scalar(value.clone()),

            // parenthesized -> same type as inner
            Expression::Parenthesized { expression: inner } => {
                let inner_ty_id =
                    self.analyze_expression(module, *inner, tree, symbols, types, ctx)?;
                return Ok(types.get_type(inner_ty_id).clone());
            }

            // references -> look up symbol type
            // TODO #Incomplete: instantiate generics with type arguments
            // TODO #Incomplete: use ctx.get_narrowed() for narrowed types
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                // check for narrowed type in context (CFG-sensitive)
                if let Some(narrowed_ty_id) = ctx.get_narrowed(*target_symbol) {
                    return Ok(types.get_type(narrowed_ty_id).clone());
                }

                // check if symbol has value type
                if let Some(ty) = types.get_value_type(*target_symbol) {
                    return Ok(ty.clone());
                }

                // check if symbol has declared type (from its declaration node)
                let symbol = symbols.get_symbol(target_symbol.into_local());
                if let Some(primary_declaration_id) = symbol.primary_declaration
                    && let Some(ty) = types.get_declared_type(primary_declaration_id)
                {
                    return Ok(ty.clone());
                }

                // unknown type
                Type::Scalar(TypeLiteral::Unknown)
            }

            // let binding -> void (but walk value and propagate to pattern)
            Expression::Let { pattern, value, .. } => {
                let value_ty_id = if let Some(value) = value {
                    Some(self.analyze_expression(module, *value, tree, symbols, types, ctx)?)
                } else {
                    None
                };

                if let Some(value_ty_id) = value_ty_id {
                    self.infer_pattern(module, *pattern, value_ty_id, tree, symbols, types, ctx)?;
                }

                Type::Scalar(TypeLiteral::Void)
            }

            // binary operations
            // TODO #Incomplete: resolve operator overloads
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_ty_id =
                    self.analyze_expression(module, *left, tree, symbols, types, ctx)?;
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;

                infer_binary_operator_type(
                    operator,
                    types.get_type(left_ty_id),
                    types.get_type(right_ty_id),
                )
            }

            // unary operations
            Expression::Unary { right, .. } => {
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                types.get_type(right_ty_id).clone()
            }

            // block -> type of last expression
            Expression::Block { block } => {
                let block = tree.get(*block);
                let mut last_ty = Type::Scalar(TypeLiteral::Void);
                for expr in &block.expressions {
                    let ty_id =
                        self.analyze_expression(module, *expr, tree, symbols, types, ctx)?;
                    last_ty = types.get_type(ty_id).clone();
                }
                last_ty
            }

            // statement -> void (but analyze inner)
            Expression::Statement { statement } => {
                self.analyze_expression(module, *statement, tree, symbols, types, ctx)?;
                Type::Scalar(TypeLiteral::Void)
            }

            // if expression
            // TODO #Incomplete: narrow types in branches based on condition
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                // analyze condition
                self.analyze_expression(module, *condition, tree, symbols, types, ctx)?;

                // TODO: fork context and apply narrowings for then/else branches
                let mut then_ctx = ctx.fork();
                let then_ty_id = self.analyze_expression(
                    module,
                    *then_expression,
                    tree,
                    symbols,
                    types,
                    &mut then_ctx,
                )?;

                if let Some(else_expr) = else_expression {
                    let mut else_ctx = ctx.fork();
                    let _else_ty_id = self.analyze_expression(
                        module,
                        *else_expr,
                        tree,
                        symbols,
                        types,
                        &mut else_ctx,
                    )?;
                    // TODO #Incomplete: compute union of then/else types
                    // TODO: merge contexts back
                    ctx.merge(&then_ctx);
                    ctx.merge(&else_ctx);
                }

                types.get_type(then_ty_id).clone()
            }

            // call expression
            // TODO #Incomplete: resolve function overloads
            // TODO #Incomplete: instantiate generic return type
            Expression::Call {
                left,
                dynamic_arguments,
                ..
            } => {
                let callee_ty_id =
                    self.analyze_expression(module, *left, tree, symbols, types, ctx)?;

                // analyze arguments
                for arg_id in dynamic_arguments {
                    let arg = tree.get(*arg_id);
                    self.analyze_expression(module, arg.value(), tree, symbols, types, ctx)?;
                }

                let callee_ty = types.get_type(callee_ty_id);
                if let Type::Function { return_type, .. } = callee_ty
                    && let Some(ret_ty_id) = return_type
                {
                    return Ok(types.get_type(*ret_ty_id).clone());
                }

                Type::Scalar(TypeLiteral::Unknown)
            }

            // member access
            // TODO #Incomplete: look up field type from object type
            Expression::Member { left, .. } => {
                self.analyze_expression(module, *left, tree, symbols, types, ctx)?;
                Type::Scalar(TypeLiteral::Unknown)
            }

            // array expression
            Expression::ArrayExpression { elements } => {
                if let Some(element_id) = elements.first() {
                    let element = tree.get(*element_id);
                    let element_ty_id = self.analyze_expression(
                        module,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                    Type::Array {
                        element: Some(element_ty_id),
                    }
                } else {
                    Type::Array { element: None }
                }
            }

            // tuple expression
            Expression::TupleExpression { elements } => {
                let mut element_types = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element = tree.get(*element_id);
                    let element_ty_id = self.analyze_expression(
                        module,
                        element.value(),
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                    element_types.push(element_ty_id);
                }
                Type::Tuple {
                    elements: element_types,
                }
            }

            // object expression
            // TODO #Incomplete: infer field types
            Expression::ObjectExpression { .. } => Type::Object { fields: vec![] },

            // assignment -> void
            Expression::Assign { right, .. } | Expression::AssignBinary { right, .. } => {
                self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                Type::Scalar(TypeLiteral::Void)
            }

            // declaration -> analyze it and return void
            Expression::Declaration { declaration } => {
                self.analyze_declaration(module, *declaration, tree, symbols, types, ctx)?;
                Type::Scalar(TypeLiteral::Void)
            }

            // imports/exports -> void
            Expression::Import { .. }
            | Expression::ReExport { .. }
            | Expression::Export { .. }
            | Expression::UnresolvedImport { .. }
            | Expression::UnresolvedReExport { .. } => Type::Scalar(TypeLiteral::Void),

            // fallback
            _ => Type::Scalar(TypeLiteral::Unknown),
        };

        Ok(ty)
    }

    /// Analyze a declaration: walk nested expressions and set the symbol's type.
    /// Returns the type of the declaration *as an expression* (usually void).
    fn infer_declaration(
        &self,
        module: &Module,
        declaration: &Declaration,
        declaration_id: LocalNodeId<Declaration>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<Type> {
        match declaration {
            // function declaration
            Declaration::Function {
                descriptor,
                signature,
                body,
                ..
            } => {
                // walk signature
                self.infer_signature(module, signature, tree, symbols, types, ctx)?;

                // analyze function body if present
                if let Some(body) = body {
                    self.analyze_expression(module, *body, tree, symbols, types, ctx)?;
                }

                // TODO #Incomplete: build proper Type::Function from signature
                // for now, use unknown as a placeholder
                let fn_type = Type::Scalar(TypeLiteral::Unknown);
                let ty_id = types.insert_type_from(fn_type, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), ty_id);

                // declaration expression evaluates to void
                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // type alias -> the aliased type becomes the instance type
            Declaration::Type {
                descriptor,
                static_parameters,
                value,
                ..
            } => {
                // walk static parameters
                if let Some(params) = static_parameters {
                    self.infer_parameters(module, params, tree, symbols, types, ctx)?;
                }

                // the aliased type is the instance type
                let instance_ty_id =
                    self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // struct
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                properties,
                ..
            } => {
                // walk generics and heritage
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(module, heritage, tree, symbols, types, ctx)?;
                self.infer_properties(module, properties, tree, symbols, types, ctx)?;

                // TODO #Incomplete: build proper Type::Object from properties
                // for now, use empty object as placeholder for instance type
                let instance_ty = Type::Object { fields: vec![] };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype (type descriptor)
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // class
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                properties,
                ..
            } => {
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(module, heritage, tree, symbols, types, ctx)?;
                self.infer_properties(module, properties, tree, symbols, types, ctx)?;

                // TODO #Incomplete: build proper Type::Object from properties
                let instance_ty = Type::Object { fields: vec![] };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // interface
            Declaration::Interface {
                descriptor,
                generics,
                heritage,
                properties,
                ..
            } => {
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(module, heritage, tree, symbols, types, ctx)?;
                self.infer_properties(module, properties, tree, symbols, types, ctx)?;

                // TODO #Incomplete: build proper Type::Object from properties
                let instance_ty = Type::Object { fields: vec![] };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // enum
            Declaration::Enum {
                descriptor,
                generics,
                heritage,
                fields,
                properties,
                ..
            } => {
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.infer_heritage(module, heritage, tree, symbols, types, ctx)?;
                self.infer_enum_fields(module, fields, tree, symbols, types, ctx)?;
                self.infer_properties(module, properties, tree, symbols, types, ctx)?;

                // TODO #Incomplete: build proper union type from variants
                let instance_ty = Type::Scalar(TypeLiteral::Unknown);
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // namespace
            Declaration::Namespace {
                descriptor,
                generics,
                expressions,
                ..
            } => {
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                for expr in expressions {
                    self.analyze_expression(module, *expr, tree, symbols, types, ctx)?;
                }

                // namespaces have an object-like type with their exports
                // TODO #Incomplete: build proper Type::Object from exports
                let instance_ty = Type::Object { fields: vec![] };
                let instance_ty_id = types.insert_type_from(instance_ty, declaration_id);
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value {
                    of: instance_ty_id,
                };
                let value_ty_id = types.insert_type_from(value_ty, declaration_id);
                types.set_value_type(descriptor.symbol.into_global(module.id), value_ty_id);

                Ok(Type::Scalar(TypeLiteral::Void))
            }

            // extension
            Declaration::Extension {
                generics,
                target_type,
                heritage,
                properties,
                ..
            } => {
                self.infer_generics(module, generics, tree, symbols, types, ctx)?;
                self.analyze_expression(module, *target_type, tree, symbols, types, ctx)?;
                self.infer_heritage(module, heritage, tree, symbols, types, ctx)?;
                self.infer_properties(module, properties, tree, symbols, types, ctx)?;
                Ok(Type::Scalar(TypeLiteral::Void))
            }
        }
    }

    /// Analyze a list of properties.
    fn infer_properties(
        &self,
        module: &Module,
        properties: &[LocalNodeId<Property>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        for property_id in properties {
            self.infer_property(module, *property_id, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze a single property.
    fn infer_property(
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

    /// Analyze generics: static parameters, with clauses, where clauses.
    fn infer_generics(
        &self,
        module: &Module,
        generics: &Generics,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        if let Some(params) = &generics.static_parameters {
            self.infer_parameters(module, params, tree, symbols, types, ctx)?;
        }
        if let Some(clauses) = &generics.with_clauses {
            self.infer_with_clauses(module, clauses, tree, symbols, types, ctx)?;
        }
        if let Some(clauses) = &generics.where_clauses {
            self.infer_where_clauses(module, clauses, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze heritage: extends, implements, embedded types.
    fn infer_heritage(
        &self,
        module: &Module,
        heritage: &Heritage,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        if let Some(extends) = &heritage.extends_types {
            for expr_id in extends {
                self.analyze_expression(module, *expr_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(implements) = &heritage.implements_types {
            for expr_id in implements {
                self.analyze_expression(module, *expr_id, tree, symbols, types, ctx)?;
            }
        }
        if let Some(embedded) = &heritage.embedded_types {
            for expr_id in embedded {
                self.analyze_expression(module, *expr_id, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze function signature.
    fn infer_signature(
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
            self.infer_generics(module, generics, tree, symbols, types, ctx)?;
        }
        // walk dynamic parameters
        self.infer_parameters(module, &signature.dynamic_parameters, tree, symbols, types, ctx)?;
        // walk return type
        if let Some(ret_type) = signature.return_type {
            self.analyze_expression(module, ret_type, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze parameters.
    fn infer_parameters(
        &self,
        module: &Module,
        parameters: &[LocalNodeId<Parameter>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        for param_id in parameters {
            self.infer_parameter(module, *param_id, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze a single parameter.
    fn infer_parameter(
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
            Parameter::Named { default, .. } => {
                if let Some(default) = default {
                    self.analyze_expression(module, *default, tree, symbols, types, ctx)?;
                }
            }
            Parameter::Pattern {
                pattern, default, ..
            } => {
                if let Some(default) = default {
                    let default_ty_id =
                        self.analyze_expression(module, *default, tree, symbols, types, ctx)?;
                    self.infer_pattern(module, *pattern, default_ty_id, tree, symbols, types, ctx)?;
                }
            }
            Parameter::Variadic { .. } => {}
        }
        Ok(())
    }

    /// Analyze with clauses.
    fn infer_with_clauses(
        &self,
        module: &Module,
        clauses: &[LocalNodeId<WithClause>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        for clause_id in clauses {
            let clause = tree.get(*clause_id);
            self.analyze_expression(module, clause.right, tree, symbols, types, ctx)?;
        }
        Ok(())
    }

    /// Analyze where clauses.
    fn infer_where_clauses(
        &self,
        module: &Module,
        clauses: &[LocalNodeId<WhereClause>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        for clause_id in clauses {
            let clause = tree.get(*clause_id);
            match clause {
                WhereClause::Assertion { right, .. } => {
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                }
                WhereClause::Guard { guard, .. } => {
                    self.analyze_expression(module, *guard, tree, symbols, types, ctx)?;
                }
            }
        }
        Ok(())
    }

    /// Analyze enum fields.
    fn infer_enum_fields(
        &self,
        module: &Module,
        fields: &[LocalNodeId<EnumField>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        for field_id in fields {
            let field = tree.get(*field_id);
            if let Some(value) = field.value {
                self.analyze_expression(module, value, tree, symbols, types, ctx)?;
            }
        }
        Ok(())
    }

    /// Analyze a pattern: propagate types to bound symbols.
    fn infer_pattern(
        &self,
        module: &Module,
        pattern_id: LocalNodeId<Pattern>,
        value_ty_id: LocalTypeId,
        tree: &NodeTree,
        _symbols: &SymbolTable,
        types: &mut TypeTable,
        _ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let pattern = tree.get(pattern_id);
        match pattern {
            // simple binding: x = value
            Pattern::Binding { symbol, .. } => {
                types.set_value_type(symbol.into_global(module.id), value_ty_id);
            }

            // tuple destructuring: (a, b) = value
            Pattern::Tuple { fields } => {
                let value_ty = types.get_type(value_ty_id);
                if let Type::Tuple { elements } = value_ty {
                    let elements = elements.clone();
                    for (i, field_id) in fields.iter().enumerate() {
                        if let Some(elem_ty_id) = elements.get(i) {
                            let field = tree.get(*field_id);
                            if let Some(symbol) = field.symbol() {
                                types.set_value_type(symbol.into_global(module.id), *elem_ty_id);
                            }
                        }
                    }
                }
            }

            // object destructuring: { a, b } = value
            Pattern::Object { fields } => {
                // TODO #Incomplete: look up field types from object type
                for field_id in fields {
                    let field = tree.get(*field_id);
                    if let Some(symbol) = field.symbol() {
                        // for now, mark as unknown
                        let unknown_ty_id =
                            types.insert_type_from(Type::Scalar(TypeLiteral::Unknown), *field_id);
                        types.set_value_type(symbol.into_global(module.id), unknown_ty_id);
                    }
                }
            }

            // slice/array destructuring: [a, b] = value
            Pattern::Slice { fields } => {
                let value_ty = types.get_type(value_ty_id);
                if let Type::Array {
                    element: Some(elem_ty_id),
                } = value_ty
                {
                    let elem_ty_id = *elem_ty_id;
                    for field_id in fields {
                        let field = tree.get(*field_id);
                        if let Some(symbol) = field.symbol() {
                            types.set_value_type(symbol.into_global(module.id), elem_ty_id);
                        }
                    }
                }
            }

            // TODO #Incomplete: handle more pattern types (Rest, TaggedTuple, TaggedObject, etc.)
            _ => {}
        }

        Ok(())
    }
}

/// Convert a scalar literal to a type literal.
fn scalar_literal_to_type_literal(value: &ScalarLiteral) -> TypeLiteral {
    match value {
        ScalarLiteral::Boolean(_) => TypeLiteral::Primitive(PrimitiveType::Boolean),
        ScalarLiteral::Byte(_) | ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) => {
            TypeLiteral::Primitive(PrimitiveType::Number)
        }
        ScalarLiteral::String(_) | ScalarLiteral::RegexString { .. } => {
            TypeLiteral::Primitive(PrimitiveType::String)
        }
        ScalarLiteral::ByteString(_) => TypeLiteral::Primitive(PrimitiveType::String),
        ScalarLiteral::Bigint(_) => TypeLiteral::Primitive(PrimitiveType::Bigint),
        ScalarLiteral::Character(_) => TypeLiteral::Primitive(PrimitiveType::Character),
    }
}

/// Infer the result type of a binary operation.
fn infer_binary_operator_type(operator: &BinaryOperator, left: &Type, _right: &Type) -> Type {
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
        | BinaryOperator::InstanceOf => Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Boolean)),

        // logical operators return boolean
        BinaryOperator::And | BinaryOperator::Or => {
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Boolean))
        }

        // arithmetic operators: for now, assume result type follows left operand
        // TODO #Incomplete: proper numeric type promotion
        _ => left.clone(),
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
            .get_inferred_type_for_node(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Number))
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
            .get_inferred_type_for_node(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::String))
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
            .get_inferred_type_for_node(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Boolean))
        );
    }

    #[test]
    fn test_analyze_binary_add_numbers() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "1 + 2");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type_for_node(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Number))
        );
    }

    #[test]
    fn test_analyze_comparison_returns_boolean() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "1 < 2");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let expression_id = module.roots[0];
        let ty = types
            .get_inferred_type_for_node(expression_id.into_global_any(module.id))
            .unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Boolean))
        );
    }

    #[test]
    fn test_analyze_let_propagates_to_symbol() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "let x = 42");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let ty = types.get_value_type(x_symbol).unwrap();

        assert_eq!(
            *ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Number))
        );
    }

    #[test]
    fn test_analyze_with_declared_type() {
        let test = TestProgram::memory_sequential();
        let file = test.file("test.ds", "let x: string = 42");
        test.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
        test.compile_dump_clean();

        let module = test.module_for_file(&file);
        let module = module.read();
        let types = module.types.read();

        // declared type on Let expression
        let let_expression_id = module.roots[0];
        let declared = types
            .get_declared_type(let_expression_id.into_global_any(module.id))
            .unwrap();
        assert_eq!(
            *declared,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::String))
        );

        // value type from inference
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let value_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::Number))
        );
    }
}
