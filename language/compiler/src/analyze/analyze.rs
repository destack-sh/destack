use crate::{AnalyzeError, AnalyzeResult, Compiler, TypeContext};
use destack_dir::{
    BinaryOperator, Declaration, EnumField, Expression, FunctionSignature, Generics, Heritage,
    LocalNodeId, LocalTypeId, Module, NodeTree, Parameter, Pattern, PrimitiveType, Property,
    ScalarLiteral, SymbolTable, Type, TypeLiteral, TypeTable, UnaryOperator, WhereClause,
    WithClause,
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
            // scalar literal: derive type from value
            Expression::ScalarLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: analyze_scalar_literal(value),
                };
                types.insert_type_from(ty, expression_id)
            }
            // type literal
            Expression::TypeLiteral { value } => {
                let ty = Type::TypeLiteral {
                    value: value.clone(),
                };
                types.insert_type_from(ty, expression_id)
            }

            // parenthesized -> same type as inner
            Expression::Parenthesized {
                expression: inner_id,
            } => self.analyze_expression(module, *inner_id, tree, symbols, types, ctx)?,

            // references -> look up symbol type
            // TODO #Incomplete: instantiate generics with type arguments
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                let symbol = symbols.get_symbol(target_symbol.into_local());
                // check for narrowed type in context
                if let Some(narrowed_ty_id) = ctx.get_narrowed(*target_symbol) {
                    narrowed_ty_id
                }
                // check if symbol has value type
                else if let Some(value_ty_id) = types.get_value_type_id(*target_symbol) {
                    value_ty_id
                }
                // check if symbol has declared type (from its declaration node)
                else if let Some(primary_declaration_id) = symbol.primary_declaration
                    && let Some(ty_id) = types.get_declared_type_id(primary_declaration_id)
                {
                    ty_id
                }
                // unknown type
                else {
                    let ty = Type::TypeLiteral {
                        value: TypeLiteral::Unknown,
                    };
                    types.insert_type_from(ty, expression_id)
                }
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

                let ty = analyze_binary_operation(
                    operator,
                    types.get_type(left_ty_id),
                    types.get_type(right_ty_id),
                );
                types.insert_type_from(ty, expression_id)
            }

            // unary operations
            Expression::Unary { operator, right } => {
                let right_ty_id =
                    self.analyze_expression(module, *right, tree, symbols, types, ctx)?;
                let ty = analyze_unary_operation(operator, types.get_type(right_ty_id));
                types.insert_type_from(ty, expression_id)
            }

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

    /// Analyze a declaration: walk nested expressions and set the symbol's type.
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
            // type alias
            Declaration::Type {
                descriptor,
                kind: _,
                mutability: _,
                static_parameters,
                value,
            } => {
                // walk static parameters
                if let Some(parameters) = static_parameters {
                    for parameter_id in parameters {
                        self.analyze_parameter(module, *parameter_id, tree, symbols, types, ctx)?;
                    }
                }

                // the aliased type is the instance type
                let instance_ty_id =
                    self.analyze_expression(module, *value, tree, symbols, types, ctx)?;
                types.set_instance_type(descriptor.symbol.into_global(module.id), instance_ty_id);

                // value type is the metatype
                let value_ty = Type::Value { ty: instance_ty_id };
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
        if let Some(clauses) = &generics.with_clauses {
            for clause_id in clauses {
                self.analyze_with_clause(module, *clause_id, tree, symbols, types, ctx)?;
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
                    self.analyze_pattern(
                        module,
                        *pattern,
                        default_ty_id,
                        tree,
                        symbols,
                        types,
                        ctx,
                    )?;
                }
            }
            Parameter::Variadic { .. } => {}
        }
        Ok(())
    }

    /// Analyze a with clause.
    fn analyze_with_clause(
        &self,
        module: &Module,
        clause_id: LocalNodeId<WithClause>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        ctx: &mut TypeContext,
    ) -> AnalyzeResult<()> {
        let clause = tree.get(clause_id);
        self.analyze_expression(module, clause.right, tree, symbols, types, ctx)?;
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

    /// Analyze a pattern.
    fn analyze_pattern(
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

            _ => {
                return Err(AnalyzeError::UnsupportedNode {
                    node: pattern_id.into_global_any(module.id),
                });
            }
        }

        Ok(())
    }
}

/// Convert a scalar literal to a type literal.
fn analyze_scalar_literal(value: &ScalarLiteral) -> TypeLiteral {
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
fn analyze_binary_operation(operator: &BinaryOperator, left: &Type, _right: &Type) -> Type {
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
fn analyze_unary_operation(operator: &UnaryOperator, right: &Type) -> Type {
    match operator {
        UnaryOperator::Not => Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean),
        },
        _ => right.clone(),
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
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
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
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String)
            }
        );

        // value type from inference
        let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
        let value_ty = types.get_value_type(x_symbol).unwrap();
        assert_eq!(
            *value_ty,
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number)
            }
        );
    }
}
