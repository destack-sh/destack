use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};
use {destack_dir as dir, destack_js as js};

impl ModuleLowerer<'_> {
    /// Lower one type annotation expression into a JS type.
    pub fn lower_type_annotation_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        let global_id = expression_id.into_global_any(self.module.id);

        // prefer the semantic type lane when analysis already declared the annotation
        if let Some(type_id) = self.types.get_declared_type_id(global_id) {
            return self.lower_type(type_id);
        }

        self.lower_expression_as_type(expression_id)
    }

    /// Lower a mutability from DIR into JS AST.
    pub fn lower_mutability(&self, mutability: dir::Mutability) -> js::Mutability {
        match mutability {
            dir::Mutability::Immutable => js::Mutability::Immutable,
            dir::Mutability::Mutable => js::Mutability::Mutable,
        }
    }

    /// Lower one for each declaration kind from DIR into JS AST.
    pub fn lower_for_each_declaration_kind(
        &self,
        declaration_kind: dir::ForEachDeclarationKind,
    ) -> js::ForEachDeclarationKind {
        match declaration_kind {
            dir::ForEachDeclarationKind::Var => js::ForEachDeclarationKind::Var,
            dir::ForEachDeclarationKind::Let => js::ForEachDeclarationKind::Let,
            dir::ForEachDeclarationKind::Const => js::ForEachDeclarationKind::Const,
        }
    }

    /// Lower one tuple element from DIR into JS AST.
    pub fn lower_tuple_element(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        element: &dir::TypeElement,
    ) -> CodegenJsResult<js::LocalNodeId<js::TupleElement>> {
        let label = element
            .label
            .map(|label| self.strings.intern_from(self.source_strings, label));
        let ty = self.lower_type(element.ty)?;
        let tuple_element = js::TupleElement {
            label,
            ty,
            is_optional: element.is_optional,
            is_readonly: element.is_readonly,
            is_rest: element.is_rest,
        };

        Ok(self
            .tree
            .insert_from_source_any(tuple_element, self.module.id, source_id))
    }

    /// Lower generics from DIR into JS AST.
    pub fn lower_generics(&mut self, generics: &dir::Generics) -> CodegenJsResult<js::Generics> {
        let static_parameters = generics
            .static_parameters
            .as_ref()
            .map(|static_parameters| {
                static_parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(*parameter))
                    .collect::<Result<Vec<_>, CodegenJsError>>()
            })
            .transpose()?;
        let generics = js::Generics { static_parameters };
        Ok(generics)
    }

    /// Lower Heritage from DIR into JS AST.
    pub fn lower_heritage(&mut self, heritage: &dir::Heritage) -> CodegenJsResult<js::Heritage> {
        let extends_types = heritage
            .extends_types
            .as_ref()
            .map(|extends_types| {
                extends_types
                    .iter()
                    .map(|extends_type| self.lower_type_annotation_expression(*extends_type))
                    .collect::<Result<Vec<_>, CodegenJsError>>()
            })
            .transpose()?;
        let implements_types = heritage
            .implements_types
            .as_ref()
            .map(|implements_types| {
                implements_types
                    .iter()
                    .map(|implements_type| self.lower_type_annotation_expression(*implements_type))
                    .collect::<Result<Vec<_>, CodegenJsError>>()
            })
            .transpose()?;
        let heritage = js::Heritage {
            extends_types,
            implements_types,
        };
        Ok(heritage)
    }

    /// Lower a primitive type from DIR into JS AST.
    pub fn lower_primitive_type_value(&self, primitive: dir::PrimitiveType) -> js::PrimitiveType {
        match primitive {
            dir::PrimitiveType::Boolean => js::PrimitiveType::Boolean,
            dir::PrimitiveType::Character => js::PrimitiveType::String,
            dir::PrimitiveType::String => js::PrimitiveType::String,
            dir::PrimitiveType::Bigint => js::PrimitiveType::Bigint,
            dir::PrimitiveType::Number => js::PrimitiveType::Number,
            dir::PrimitiveType::Int(_) => js::PrimitiveType::Number,
            dir::PrimitiveType::Float(_) => js::PrimitiveType::Number,
            dir::PrimitiveType::Symbol => js::PrimitiveType::Symbol,
            dir::PrimitiveType::UniqueSymbol => js::PrimitiveType::UniqueSymbol,
        }
    }

    /// Lower a primitive type from DIR into JS AST.
    pub fn lower_primitive_type(
        &mut self,
        _ty_id: dir::LocalTypeId,
        primitive: dir::PrimitiveType,
    ) -> CodegenJsResult<js::PrimitiveType> {
        Ok(self.lower_primitive_type_value(primitive))
    }

    /// Lower a type literal value from DIR into JS AST.
    /// Returns None for unsupported type literals (like Infer, Composite).
    pub fn lower_type_literal_value(
        &mut self,
        literal: &dir::TypeLiteral,
    ) -> Option<js::TypeLiteral> {
        let literal = match literal {
            dir::TypeLiteral::Never => js::TypeLiteral::Never,
            dir::TypeLiteral::Any => js::TypeLiteral::Any,
            dir::TypeLiteral::Undefined => js::TypeLiteral::Undefined,
            dir::TypeLiteral::Unknown => js::TypeLiteral::Unknown,
            dir::TypeLiteral::Object => js::TypeLiteral::Object,
            dir::TypeLiteral::Void => js::TypeLiteral::Void,
            dir::TypeLiteral::Null => js::TypeLiteral::Null,
            dir::TypeLiteral::Primitive(primitive) => {
                let primitive = self.lower_primitive_type_value(*primitive);
                js::TypeLiteral::Primitive(primitive)
            }
            dir::TypeLiteral::ScalarLiteral(scalar_literal) => {
                let scalar_literal = self.lower_scalar_literal(scalar_literal);
                js::TypeLiteral::ScalarLiteral(scalar_literal)
            }
            _ => return None,
        };
        Some(literal)
    }

    /// Lower a type literal from DIR into JS AST.
    pub fn lower_type_literal(
        &mut self,
        ty_id: dir::LocalTypeId,
        literal: &dir::TypeLiteral,
    ) -> CodegenJsResult<js::TypeLiteral> {
        self.lower_type_literal_value(literal).ok_or_else(|| {
            let source_id = self.types.get_type_source(ty_id);
            CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: None,
            }
        })
    }

    /// Lower a type unary operator from DIR into JS AST.
    pub fn lower_type_unary_operator(
        &self,
        ty_id: dir::LocalTypeId,
        operator: dir::TypeUnaryOperator,
    ) -> CodegenJsResult<js::TypeUnaryOperator> {
        let operator = match operator {
            dir::TypeUnaryOperator::Not => js::TypeUnaryOperator::Not,
            dir::TypeUnaryOperator::Must => js::TypeUnaryOperator::Must,
            dir::TypeUnaryOperator::Type => js::TypeUnaryOperator::Type,
            dir::TypeUnaryOperator::Newtype => {
                let source_id = self.types.get_type_source(ty_id);
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: None,
                });
            }
            dir::TypeUnaryOperator::Readonly => js::TypeUnaryOperator::Readonly,
            dir::TypeUnaryOperator::Typeof => js::TypeUnaryOperator::Typeof,
            dir::TypeUnaryOperator::Keyof => js::TypeUnaryOperator::Keyof,
            dir::TypeUnaryOperator::AsComptime => js::TypeUnaryOperator::AsComptime,
            dir::TypeUnaryOperator::AsConst => js::TypeUnaryOperator::AsConst,
        };
        Ok(operator)
    }

    /// Lower one mapped type modifier from DIR into JS AST.
    fn lower_type_modifier(&self, modifier: dir::TypeModifier) -> js::TypeModifier {
        match modifier {
            dir::TypeModifier::Add => js::TypeModifier::Add,
            dir::TypeModifier::Remove => js::TypeModifier::Remove,
            dir::TypeModifier::None => js::TypeModifier::None,
        }
    }

    /// Lower one mapped type modifier set from DIR into JS AST.
    fn lower_type_mapped_modifiers(
        &self,
        modifiers: dir::TypeMappedModifiers,
    ) -> js::TypeMappedModifiers {
        js::TypeMappedModifiers {
            readonly: self.lower_type_modifier(modifiers.readonly),
            optional: self.lower_type_modifier(modifiers.optional),
        }
    }

    /// Lower one type predicate subject from DIR into JS AST.
    fn lower_type_predicate_subject(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        subject: dir::TypePredicateSubject,
    ) -> CodegenJsResult<js::TypePredicateSubject> {
        let subject = match subject {
            dir::TypePredicateSubject::Unresolved(name) => {
                let name = self.strings.intern_from(self.source_strings, name);
                js::TypePredicateSubject::Name(name)
            }
            dir::TypePredicateSubject::Symbol(symbol_id) => {
                if symbol_id.module_id != self.module.id {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "remote type predicate subjects need source-backed lowering in js output"
                                .to_string(),
                        ),
                    });
                }

                let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
                let Some(dir::StaticKey::Name(name) | dir::StaticKey::Number(name)) = symbol.key
                else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "type predicate subjects need path-like symbols in js output"
                                .to_string(),
                        ),
                    });
                };

                let name = self.strings.intern_from(self.source_strings, name);
                js::TypePredicateSubject::Name(name)
            }
            dir::TypePredicateSubject::This => js::TypePredicateSubject::This,
        };

        Ok(subject)
    }

    /// Lower one intrinsic type reference into a JS path type.
    fn lower_intrinsic_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        intrinsic: dir::IntrinsicType,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        let intrinsic_name = match intrinsic {
            dir::IntrinsicType::Uppercase => "Uppercase",
            dir::IntrinsicType::Lowercase => "Lowercase",
            dir::IntrinsicType::Capitalize => "Capitalize",
            dir::IntrinsicType::Uncapitalize => "Uncapitalize",
            dir::IntrinsicType::NoInfer => "NoInfer",
            dir::IntrinsicType::BuiltinIteratorReturn => "BuiltinIteratorReturn",
        };
        let segment = self.strings.intern(intrinsic_name);
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let ty = js::Type::Path {
            path,
            static_arguments: None,
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower a type binary operator from DIR into JS AST.
    pub fn lower_type_binary_operator(
        &self,
        _ty_id: dir::LocalTypeId,
        operator: dir::TypeBinaryOperator,
    ) -> CodegenJsResult<js::TypeBinaryOperator> {
        let operator = match operator {
            dir::TypeBinaryOperator::Cast => js::TypeBinaryOperator::Cast,
            dir::TypeBinaryOperator::In => js::TypeBinaryOperator::In,
            dir::TypeBinaryOperator::Is => js::TypeBinaryOperator::Is,
            dir::TypeBinaryOperator::InstanceOf => js::TypeBinaryOperator::InstanceOf,
            dir::TypeBinaryOperator::Satisfies => js::TypeBinaryOperator::Satisfies,
            dir::TypeBinaryOperator::Extends => js::TypeBinaryOperator::Extends,
            dir::TypeBinaryOperator::Implements => js::TypeBinaryOperator::Implements,
        };
        Ok(operator)
    }

    /// Lower one DIR static argument into one JS type argument.
    pub(crate) fn lower_static_type_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        let argument = self.dir_tree.get(argument_id);

        match argument {
            dir::Argument::Positional { value, .. } => {
                self.lower_type_annotation_expression(*value)
            }
            dir::Argument::Named { .. } | dir::Argument::Labeled { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: argument_id.into_global_any(self.module.id),
                    message: Some(
                        "named static arguments are not lowered to js type arguments".to_string(),
                    ),
                });
            }
            dir::Argument::Spread { .. } => Err(CodegenJsError::UnsupportedConstruct {
                node: argument_id.into_global_any(self.module.id),
                message: Some(
                    "spread static arguments are not lowered to js type arguments".to_string(),
                ),
            }),
            dir::Argument::Error { value } => Err(CodegenJsError::UnsupportedConstruct {
                node: value.into_global_any(self.module.id),
                message: Some(
                    "argument error slots are not lowered to js type arguments".to_string(),
                ),
            }),
        }
    }

    /// Lower one source static argument list into JS type arguments.
    pub(crate) fn lower_static_type_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::Type>>> {
        let static_arguments = arguments
            .iter()
            .map(|argument| self.lower_static_type_argument(*argument))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        Ok(static_arguments)
    }

    /// Lower one semantic static expression into one JS type argument.
    fn lower_semantic_static_type_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &dir::StaticExpression,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        match value {
            dir::StaticExpression::Unevaluated { node } => {
                self.lower_type_annotation_expression(*node)
            }
            dir::StaticExpression::ScalarLiteral { value } => {
                let literal = self.lower_scalar_literal(value);
                let ty = js::Type::Scalar(js::TypeLiteral::ScalarLiteral(literal));

                Ok(self
                    .tree
                    .insert_from_source_any(ty, self.module.id, source_id))
            }
            dir::StaticExpression::TypeLiteral { value } => {
                let literal = self.lower_type_literal_value(value).ok_or_else(|| {
                    CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "unsupported static type literal in js type arguments".to_string(),
                        ),
                    }
                })?;
                let ty = js::Type::Scalar(literal);

                Ok(self
                    .tree
                    .insert_from_source_any(ty, self.module.id, source_id))
            }
            dir::StaticExpression::Type { ty } => self.lower_type(*ty),
            dir::StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                let declaration = self.dir_tree.get(*declaration);
                let symbol = declaration.symbol().into_global(self.module.id);

                self.lower_reference_type_from_symbol(
                    source_id,
                    symbol,
                    static_arguments.as_deref(),
                )
            }
            dir::StaticExpression::ArrayExpression { .. }
            | dir::StaticExpression::TupleExpression { .. }
            | dir::StaticExpression::ObjectExpression { .. } => {
                Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: Some(
                        "static value arguments do not lower to js type arguments".to_string(),
                    ),
                })
            }
        }
    }

    /// Lower one semantic static argument into one JS type argument.
    fn lower_semantic_static_type_argument(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        argument: &dir::StaticArgument,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        match argument {
            dir::StaticArgument::Unevaluated { node } => {
                if node.module_id != self.module.id {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "remote unevaluated static arguments need source-backed lowering in js output"
                                .to_string(),
                        ),
                    });
                }

                let Ok(argument_id) = node.try_into_local_typed::<dir::Argument>() else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "static argument is missing local argument syntax for js type lowering"
                                .to_string(),
                        ),
                    });
                };

                self.lower_static_type_argument(argument_id)
            }
            dir::StaticArgument::Evaluated { value, .. } => {
                self.lower_semantic_static_type_expression(source_id, value)
            }
        }
    }

    /// Lower one semantic static argument list into JS type arguments.
    fn lower_semantic_static_type_arguments(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        arguments: &[dir::StaticArgument],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::Type>>> {
        arguments
            .iter()
            .map(|argument| self.lower_semantic_static_type_argument(source_id, argument))
            .collect()
    }

    /// Lower one reference symbol into a JS path type.
    fn lower_reference_type_from_symbol(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        symbol_id: dir::GlobalSymbolId,
        static_arguments: Option<&[dir::StaticArgument]>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        if symbol_id.module_id != self.module.id {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "remote semantic type references need source-backed lowering in js output"
                        .to_string(),
                ),
            });
        }

        let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
        let Some(dir::StaticKey::Name(name) | dir::StaticKey::Number(name)) = symbol.key else {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "type reference symbol is missing a path-like key in js output".to_string(),
                ),
            });
        };

        let segment = self.strings.intern_from(self.source_strings, name);
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let static_arguments = static_arguments
            .map(|arguments| self.lower_semantic_static_type_arguments(source_id, arguments))
            .transpose()?;
        let ty = js::Type::Path {
            path,
            static_arguments,
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one source-backed reference type into a JS path type when exact path syntax exists.
    fn try_lower_reference_type_from_source(
        &mut self,
        source_id: dir::LocalNodeIdAny,
    ) -> CodegenJsResult<Option<js::LocalNodeId<js::Type>>> {
        let Ok(expression_id) = source_id.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };
        let expression = self.dir_tree.get(expression_id);
        let (path, static_arguments) = match expression {
            dir::Expression::UnresolvedPath {
                path,
                static_arguments,
                ..
            }
            | dir::Expression::LocalReference {
                path,
                static_arguments,
                ..
            }
            | dir::Expression::ModuleReference {
                path,
                static_arguments,
                ..
            }
            | dir::Expression::GlobalReference {
                path,
                static_arguments,
                ..
            } => (path, static_arguments.as_deref()),
            _ => return Ok(None),
        };

        let path = self.lower_path(source_id, path)?;
        let static_arguments = static_arguments
            .map(|arguments| self.lower_static_type_arguments(arguments))
            .transpose()?;
        let ty = js::Type::Path {
            path,
            static_arguments,
        };

        let type_id = self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id);

        Ok(Some(type_id))
    }

    /// Lower one semantic object type field into a JS object type field.
    fn lower_object_type_field(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        field: &dir::TypeField,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeField>> {
        let mut modifiers = js::BindingModifier::default();

        // field modifiers
        if field.is_optional {
            modifiers.kind = Some(js::BindingKind::Maybe);
        }
        if field.is_readonly {
            modifiers.mutability = Some(js::Mutability::Immutable);
        }

        let modifiers = if modifiers == js::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        };
        let key = Some(self.lower_static_key(source_id, field.key)?);
        let field = match self.types.get_type(field.ty) {
            dir::Type::Function { .. } => {
                let signature = self.lower_semantic_function_type_signature(
                    source_id,
                    field.ty,
                    js::FunctionKind::Function,
                    None,
                )?;
                js::TypeField::Method {
                    modifiers,
                    key,
                    signature,
                }
            }
            _ => {
                let ty = self.lower_type(field.ty)?;
                js::TypeField::Field { modifiers, key, ty }
            }
        };

        Ok(self
            .tree
            .insert_from_source_any(field, self.module.id, source_id))
    }

    /// Lower one semantic function type parameter with one synthesized name.
    fn lower_semantic_function_parameter(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        name: String,
        ty_id: dir::LocalTypeId,
    ) -> CodegenJsResult<js::LocalNodeId<js::Parameter>> {
        let name = self.strings.intern(&name);
        let ty = Some(self.lower_type(ty_id)?);
        let parameter = js::Parameter::Named {
            modifiers: None,
            name,
            ty,
            default: None,
        };

        Ok(self
            .tree
            .insert_from_source_any(parameter, self.module.id, source_id))
    }

    /// Lower one semantic function type signature into a JS function signature.
    fn lower_semantic_function_type_signature(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        ty_id: dir::LocalTypeId,
        kind: js::FunctionKind,
        mode: Option<js::FunctionMode>,
    ) -> CodegenJsResult<js::FunctionSignature> {
        let dir::Type::Function {
            asynchrony,
            cardinality,
            static_parameters,
            this_parameter,
            dynamic_parameters,
            return_type,
        } = self.types.get_type(ty_id)
        else {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "semantic function signature lowering expected a function type".to_string(),
                ),
            });
        };

        // generics
        let static_parameters = static_parameters
            .iter()
            .enumerate()
            .map(|(index, parameter_type_id)| {
                self.lower_semantic_function_parameter(
                    source_id,
                    format!("T{index}"),
                    *parameter_type_id,
                )
            })
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let generics = if static_parameters.is_empty() {
            None
        } else {
            Some(js::Generics {
                static_parameters: Some(static_parameters),
            })
        };

        // this parameter
        let this_parameter = this_parameter
            .map(|this_type_id| {
                self.lower_semantic_function_parameter(source_id, "this".to_string(), this_type_id)
            })
            .transpose()?;

        // dynamic parameters
        let dynamic_parameters = dynamic_parameters
            .iter()
            .enumerate()
            .map(|(index, parameter_type_id)| {
                self.lower_semantic_function_parameter(
                    source_id,
                    format!("arg{index}"),
                    *parameter_type_id,
                )
            })
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        // return type
        let return_type = return_type
            .map(|return_type_id| self.lower_type(return_type_id))
            .transpose()?;

        let abstraction = js::FunctionAbstraction::Concrete;
        let asynchrony = self.lower_asynchrony(*asynchrony);
        let cardinality = self.lower_function_cardinality(*cardinality);

        Ok(js::FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics,
            this_parameter,
            dynamic_parameters,
            return_type,
        })
    }

    /// Lower one semantic object type index signature into a JS object type field.
    fn lower_object_type_index_signature(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        signature: &dir::TypeIndexSignature,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeField>> {
        let modifiers = if signature.is_readonly {
            Some(js::BindingModifier {
                mutability: Some(js::Mutability::Immutable),
                ..js::BindingModifier::default()
            })
        } else {
            None
        };
        let name = self
            .strings
            .intern_from(self.source_strings, signature.name);
        let key_type = self.lower_type(signature.key_type)?;
        let value_type = self.lower_type(signature.value_type)?;
        let field = js::TypeField::IndexSignature {
            modifiers,
            name,
            key_type,
            value_type,
        };

        Ok(self
            .tree
            .insert_from_source_any(field, self.module.id, source_id))
    }

    /// Lower a type from DIR into JS AST.
    pub fn lower_type(
        &mut self,
        ty_id: dir::LocalTypeId,
    ) -> CodegenJsResult<js::LocalNodeId<js::Type>> {
        let source_id = self.types.get_type_source(ty_id);
        let ty = self.types.get_type(ty_id);

        let ty_id = match ty {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Intrinsic(intrinsic),
            } => self.lower_intrinsic_type(source_id, *intrinsic)?,
            dir::Type::TypeLiteral { value: scalar } => {
                let literal = self.lower_type_literal(ty_id, scalar)?;
                let ty = js::Type::Scalar(literal);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::This => {
                self.tree
                    .insert_from_source_any(js::Type::This, self.module.id, source_id)
            }
            dir::Type::Unevaluated(expression) => self.lower_expression_as_type(*expression)?,
            dir::Type::Reference {
                symbol,
                static_arguments,
            } => {
                let type_id = self
                    .try_lower_reference_type_from_source(source_id)?
                    .map_or_else(
                        || {
                            self.lower_reference_type_from_symbol(
                                source_id,
                                *symbol,
                                static_arguments.as_deref(),
                            )
                        },
                        Ok,
                    )?;
                self.set_global_node_symbol(type_id, *symbol);
                type_id
            }
            dir::Type::Conditional {
                distributive_symbol: _,
                left,
                right,
                then_type,
                else_type,
            } => {
                let left = self.lower_type(*left)?;
                let right = self.lower_type(*right)?;
                let then_type = self.lower_type(*then_type)?;
                let else_type = self.lower_type(*else_type)?;
                let ty = js::Type::Conditional {
                    left,
                    right,
                    then_type,
                    else_type,
                };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                let name = self
                    .strings
                    .intern_from(self.source_strings, parameter.name);
                let constraint = self.lower_type(parameter.constraint)?;
                let key_remap = parameter
                    .key_remap
                    .map(|key_remap| self.lower_type(key_remap))
                    .transpose()?;
                let parameter = js::TypeMappedParameter {
                    name,
                    constraint,
                    key_remap,
                };
                let modifiers = self.lower_type_mapped_modifiers(*modifiers);
                let value = self.lower_type(*value)?;
                let ty = js::Type::Mapped {
                    parameter,
                    modifiers,
                    value,
                };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Index { left, index } => {
                let left = self.lower_type(*left)?;
                let index = self.lower_type(*index)?;
                let ty = js::Type::Index { left, index };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::TemplateLiteral { strings, spans } => {
                let strings = strings
                    .iter()
                    .map(|string| self.strings.intern_from(self.source_strings, *string))
                    .collect::<Vec<_>>();
                let spans = spans
                    .iter()
                    .map(|span| self.lower_type(*span))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let template = js::TypeTemplateLiteral { strings, spans };
                let ty = js::Type::TemplateLiteral(template);

                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                let target = self.strings.intern_from(self.source_strings, *target);
                let qualifier = qualifier
                    .as_ref()
                    .map(|path| self.lower_path(source_id, path))
                    .transpose()?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        self.lower_semantic_static_type_arguments(source_id, arguments)
                    })
                    .transpose()?;
                let ty = js::Type::Import {
                    target,
                    qualifier,
                    static_arguments,
                };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Infer { name, constraint } => {
                let name = self.strings.intern_from(self.source_strings, *name);
                let constraint = constraint
                    .map(|constraint| self.lower_type(constraint))
                    .transpose()?;
                let ty = js::Type::Infer { name, constraint };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                let subject = self.lower_type_predicate_subject(source_id, *subject)?;
                let target = target.map(|target| self.lower_type(target)).transpose()?;
                let ty = js::Type::Predicate {
                    asserts: *asserts,
                    subject,
                    target,
                };

                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }

            dir::Type::Unary { operator, right } => {
                let operator = self.lower_type_unary_operator(ty_id, *operator)?;
                let right = self.lower_type(*right)?;
                let ty = js::Type::Unary { operator, right };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.lower_type(*left)?;
                let operator = self.lower_type_binary_operator(ty_id, *operator)?;
                let right = self.lower_type(*right)?;
                let ty = js::Type::Binary {
                    left,
                    operator,
                    right,
                };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }

            dir::Type::Array {
                element,
                is_readonly,
            } => {
                let element = element
                    .map(|element| self.lower_type(element))
                    .transpose()?;
                let mut array_id = self.tree.insert_from_source_any(
                    js::Type::Array { element },
                    self.module.id,
                    source_id,
                );
                if *is_readonly {
                    let readonly = js::Type::Unary {
                        operator: js::TypeUnaryOperator::Readonly,
                        right: array_id,
                    };
                    array_id =
                        self.tree
                            .insert_from_source_any(readonly, self.module.id, source_id);
                }
                array_id
            }
            dir::Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut properties = fields
                    .iter()
                    .map(|field| self.lower_object_type_field(source_id, field))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let call_signatures = call_signatures
                    .iter()
                    .map(|signature_id| {
                        let signature = self.lower_semantic_function_type_signature(
                            source_id,
                            *signature_id,
                            js::FunctionKind::Function,
                            Some(js::FunctionMode::Call),
                        )?;
                        let field = js::TypeField::Method {
                            modifiers: None,
                            key: None,
                            signature,
                        };

                        Ok(self
                            .tree
                            .insert_from_source_any(field, self.module.id, source_id))
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                properties.extend(call_signatures);

                let construct_signatures = construct_signatures
                    .iter()
                    .map(|signature_id| {
                        let signature = self.lower_semantic_function_type_signature(
                            source_id,
                            *signature_id,
                            js::FunctionKind::Function,
                            Some(js::FunctionMode::New),
                        )?;
                        let field = js::TypeField::Method {
                            modifiers: None,
                            key: None,
                            signature,
                        };

                        Ok(self
                            .tree
                            .insert_from_source_any(field, self.module.id, source_id))
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                properties.extend(construct_signatures);

                let index_signatures = index_signatures
                    .iter()
                    .map(|signature| self.lower_object_type_index_signature(source_id, signature))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                properties.extend(index_signatures);

                let ty = js::Type::Object { properties };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Tuple {
                elements,
                is_readonly,
            } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_tuple_element(source_id, element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let mut tuple_id = self.tree.insert_from_source_any(
                    js::Type::Tuple { elements },
                    self.module.id,
                    source_id,
                );
                if *is_readonly {
                    let readonly = js::Type::Unary {
                        operator: js::TypeUnaryOperator::Readonly,
                        right: tuple_id,
                    };
                    tuple_id =
                        self.tree
                            .insert_from_source_any(readonly, self.module.id, source_id);
                }
                tuple_id
            }
            dir::Type::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = js::Type::Union { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Intersection { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = js::Type::Intersection { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Function { .. } => {
                let signature = self.lower_semantic_function_type_signature(
                    source_id,
                    ty_id,
                    js::FunctionKind::Lambda,
                    None,
                )?;
                let ty = js::Type::Function { signature };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: None,
                });
            }
        };

        Ok(ty_id)
    }
}
