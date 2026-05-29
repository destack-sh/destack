use crate::{CodegenJsError, CodegenJsResult, ModuleLowerer};
use destack_dir as dir;
use destack_js as js;

impl ModuleLowerer<'_> {
    /// Lower one type annotation expression into a JS type.
    pub fn lower_type_annotation_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let global_id = expression_id.into_global_any(self.module.id);

        // lower through the semantic type lane
        if let Some(type_id) = self.types.get_node_type_id(global_id) {
            return self.lower_type(type_id);
        }

        Err(CodegenJsError::UnsupportedConstruct {
            node: global_id,
            message: Some("missing declared type for type annotation".to_string()),
        })
    }

    /// Lower a mutability from DIR into JS AST.
    pub fn lower_mutability(&self, mutability: dir::Mutability) -> js::Mutability {
        match mutability {
            dir::Mutability::Immutable => js::Mutability::Immutable,
            dir::Mutability::Mutable | dir::Mutability::Exclusive => js::Mutability::Mutable,
        }
    }

    /// Lower one for each declaration kind from DIR into JS AST.
    pub fn lower_for_each_keyword(&self, keyword: dir::BindingKeyword) -> js::BindingKeyword {
        match keyword {
            dir::BindingKeyword::Let => js::BindingKeyword::Let,
            dir::BindingKeyword::Const => js::BindingKeyword::Const,
        }
    }

    /// Lower one tuple element from DIR into JS AST.
    pub fn lower_tuple_element(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        element: &dir::TypeElement,
    ) -> CodegenJsResult<js::LocalNodeId<js::TupleElement>> {
        let label = element.label.map(|label| label);
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

    /// Lower generic parameters from DIR into JS generic parameters.
    pub fn lower_generic_parameters(
        &mut self,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::GenericParameter>>> {
        let generic_parameters = parameters
            .iter()
            .map(|parameter_id| self.lower_generic_parameter(*parameter_id))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        Ok(generic_parameters)
    }

    /// Lower one generic parameter from DIR into a JS generic parameter.
    fn lower_generic_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> CodegenJsResult<js::LocalNodeId<js::GenericParameter>> {
        let parameter = self.dir_tree.get(parameter_id);

        let parameter = match parameter {
            dir::GenericParameter::Type {
                name,
                variance,
                constraint,
                default: _,
                ..
            } => {
                let modifiers = variance.map(|variance| js::BindingModifier {
                    variance: Some(match variance {
                        dir::VarianceModifier::In => js::VarianceModifier::In,
                        dir::VarianceModifier::Out => js::VarianceModifier::Out,
                        dir::VarianceModifier::InOut => js::VarianceModifier::InOut,
                    }),
                    ..js::BindingModifier::default()
                });
                let name = *name;
                let ty = constraint
                    .map(|constraint| self.lower_type_annotation_expression(constraint))
                    .transpose()?;

                js::GenericParameter::Type {
                    modifiers,
                    name,
                    constraint: ty,
                    default: None,
                }
            }
            dir::GenericParameter::VariadicType { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: parameter_id.into_global_any(self.module.id),
                    message: Some(
                        "variadic generic parameters must be elaborated before JS lowering"
                            .to_string(),
                    ),
                });
            }
            dir::GenericParameter::Value { .. } | dir::GenericParameter::VariadicValue { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: parameter_id.into_global_any(self.module.id),
                    message: Some(
                        "value generic parameters must be elaborated before JS lowering"
                            .to_string(),
                    ),
                });
            }
            dir::GenericParameter::Error => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: parameter_id.into_global_any(self.module.id),
                    message: Some(
                        "generic parameter error slots are not lowered to JS".to_string(),
                    ),
                });
            }
        };

        Ok(self
            .tree
            .insert_from_source(parameter, self.module.id, parameter_id))
    }

    /// Lower type annotation expressions from DIR into JS types.
    pub fn lower_type_annotation_expressions(
        &mut self,
        expressions: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::TypeExpression>>> {
        expressions
            .iter()
            .map(|expression_id| self.lower_type_annotation_expression(*expression_id))
            .collect()
    }

    /// Lower a primitive type from DIR into JS AST.
    pub fn lower_primitive_type_value(&self, primitive: dir::PrimitiveType) -> js::PrimitiveType {
        match primitive {
            dir::PrimitiveType::Boolean => js::PrimitiveType::Boolean,
            dir::PrimitiveType::Character => js::PrimitiveType::String,
            dir::PrimitiveType::String => js::PrimitiveType::String,
            dir::PrimitiveType::Bigint => js::PrimitiveType::Bigint,
            dir::PrimitiveType::Integer(_) => js::PrimitiveType::Number,
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

    /// Lower one parsed type literal value from DIR into JS AST.
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
            dir::TypeLiteral::Boolean => js::TypeLiteral::Primitive(js::PrimitiveType::Boolean),
            dir::TypeLiteral::Character | dir::TypeLiteral::String => {
                js::TypeLiteral::Primitive(js::PrimitiveType::String)
            }
            dir::TypeLiteral::Bigint => js::TypeLiteral::Primitive(js::PrimitiveType::Bigint),
            dir::TypeLiteral::Number
            | dir::TypeLiteral::Integer(_)
            | dir::TypeLiteral::Float(_) => js::TypeLiteral::Primitive(js::PrimitiveType::Number),
            dir::TypeLiteral::Symbol => js::TypeLiteral::Primitive(js::PrimitiveType::Symbol),
            dir::TypeLiteral::UniqueSymbol => {
                js::TypeLiteral::Primitive(js::PrimitiveType::UniqueSymbol)
            }
            dir::TypeLiteral::BuiltinTypeFunction(_) => return None,
        };
        Some(literal)
    }

    /// Lower a parsed type literal from DIR into JS AST.
    pub fn lower_type_literal(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        literal: &dir::TypeLiteral,
    ) -> CodegenJsResult<js::TypeLiteral> {
        self.lower_type_literal_value(literal)
            .ok_or_else(|| CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: None,
            })
    }

    /// Lower one mapped type modifier from DIR into JS AST.
    fn lower_type_modifier(&self, modifier: dir::MappedTypeModifier) -> js::MappedTypeModifier {
        match modifier {
            dir::MappedTypeModifier::Present => js::MappedTypeModifier::Present,
            dir::MappedTypeModifier::Add => js::MappedTypeModifier::Add,
            dir::MappedTypeModifier::Remove => js::MappedTypeModifier::Remove,
            dir::MappedTypeModifier::None => js::MappedTypeModifier::None,
        }
    }

    /// Lower one mapped type modifier set from DIR into JS AST.
    fn lower_type_mapped_modifiers(
        &self,
        modifiers: dir::MappedTypeModifiers,
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
        subject: dir::PredicateSubject,
    ) -> CodegenJsResult<js::TypePredicateSubject> {
        let subject = match subject {
            dir::PredicateSubject::Symbol(symbol_id) => {
                if symbol_id.module_id != self.module.id {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "remote type predicate subjects need source-backed lowering in JS output"
                                .to_string(),
                        ),
                    });
                }

                let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
                let Some(dir::StaticKey::Name(name)) = symbol.key else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "type predicate subjects need path-like symbols in JS output"
                                .to_string(),
                        ),
                    });
                };

                let name = name;
                js::TypePredicateSubject::Identifier(name)
            }
            dir::PredicateSubject::This => js::TypePredicateSubject::This,
        };

        Ok(subject)
    }

    /// Lower one builtin type function reference into a JS path type.
    fn lower_builtin_type_function(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        function: dir::BuiltinTypeFunction,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let function_name = match function {
            dir::BuiltinTypeFunction::Uppercase => "Uppercase",
            dir::BuiltinTypeFunction::Lowercase => "Lowercase",
            dir::BuiltinTypeFunction::Capitalize => "Capitalize",
            dir::BuiltinTypeFunction::Uncapitalize => "Uncapitalize",
            dir::BuiltinTypeFunction::NoInfer => "NoInfer",
            dir::BuiltinTypeFunction::BuiltinIteratorReturn => "BuiltinIteratorReturn",
        };
        let segment = self.strings.intern(function_name);
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments: vec![],
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one DIR static argument into one JS type argument.
    pub(crate) fn lower_static_type_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let argument = self.dir_tree.get(argument_id);

        match argument {
            dir::GenericArgument::Type { value, .. }
            | dir::GenericArgument::AssociatedType { value, .. } => {
                self.lower_type_annotation_expression(*value)
            }
            dir::GenericArgument::SpreadType { .. } => Err(CodegenJsError::UnsupportedConstruct {
                node: argument_id.into_global_any(self.module.id),
                message: Some(
                    "variadic generic arguments must be elaborated before JS lowering".to_string(),
                ),
            }),
            dir::GenericArgument::Value { .. }
            | dir::GenericArgument::SpreadValue { .. }
            | dir::GenericArgument::AssociatedConst { .. } => {
                Err(CodegenJsError::UnsupportedConstruct {
                    node: argument_id.into_global_any(self.module.id),
                    message: Some(
                        "value generic arguments must be elaborated before JS lowering".to_string(),
                    ),
                })
            }
            dir::GenericArgument::Error => Err(CodegenJsError::UnsupportedConstruct {
                node: argument_id.into_global_any(self.module.id),
                message: Some(
                    "argument error slots are not lowered to JS type arguments".to_string(),
                ),
            }),
        }
    }

    /// Lower one source static argument list into JS type arguments.
    pub(crate) fn lower_static_type_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::TypeExpression>>> {
        let generic_arguments = arguments
            .iter()
            .map(|argument| self.lower_static_type_argument(*argument))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        Ok(generic_arguments)
    }

    /// Lower one semantic static expression into one JS type argument.
    fn lower_semantic_static_type_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &dir::StaticTerm,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        match value {
            dir::StaticTerm::ScalarLiteral { value } => {
                let literal = self.lower_scalar_literal(value);
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::ScalarLiteral(literal));

                Ok(self
                    .tree
                    .insert_from_source_any(ty, self.module.id, source_id))
            }
            dir::StaticTerm::TypeLiteral { value } => {
                let literal = self.lower_type_literal_value(value).ok_or_else(|| {
                    CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "unsupported static type literal in JS type arguments".to_string(),
                        ),
                    }
                })?;
                let ty = js::TypeExpression::Scalar(literal);

                Ok(self
                    .tree
                    .insert_from_source_any(ty, self.module.id, source_id))
            }
            dir::StaticTerm::Type { ty } => self.lower_type(*ty),
            dir::StaticTerm::Symbol { symbol } => {
                self.lower_reference_type_from_symbol(source_id, *symbol, None)
            }
            dir::StaticTerm::Access { access } => {
                self.lower_static_string_type(source_id, &format!("{access:?}").to_lowercase())
            }
            dir::StaticTerm::Space { space } => {
                self.lower_static_string_type(source_id, &format!("{space:?}").to_lowercase())
            }
            dir::StaticTerm::Place { place } => {
                let place = match place {
                    dir::Place::Ambient => "ambient".to_string(),
                    dir::Place::Space(space) => format!("{space:?}").to_lowercase(),
                };

                self.lower_static_string_type(source_id, &place)
            }
            dir::StaticTerm::Lifetime { lifetime } => match lifetime {
                dir::Lifetime::Static => self.lower_static_string_type(source_id, "static"),
                dir::Lifetime::Symbol(symbol) => {
                    self.lower_reference_type_from_symbol(source_id, *symbol, None)
                }
                dir::Lifetime::Generated(name) => {
                    let name = self.source_strings.get(*name).to_string();

                    self.lower_static_string_type(source_id, &name)
                }
                dir::Lifetime::Join(_) => Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: Some(
                        "joined static lifetimes do not lower to JS type arguments".to_string(),
                    ),
                }),
            },
            dir::StaticTerm::Declaration {
                declaration,
                generic_arguments,
            } => {
                let Some(symbol) = self.source_symbol_for_node(*declaration) else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: declaration.into_global_any(self.module.id),
                        message: Some(
                            "static declaration references need a bound source symbol".to_string(),
                        ),
                    });
                };
                let symbol = symbol.into_global(self.module.id);

                self.lower_reference_type_from_symbol(
                    source_id,
                    symbol,
                    generic_arguments.as_deref(),
                )
            }
            dir::StaticTerm::Array { .. }
            | dir::StaticTerm::FixedArray { .. }
            | dir::StaticTerm::Tuple { .. }
            | dir::StaticTerm::Object { .. }
            | dir::StaticTerm::Struct { .. } => Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "static value arguments do not lower to JS type arguments".to_string(),
                ),
            }),
        }
    }

    /// Lower one semantic static argument into one JS type argument.
    fn lower_semantic_static_type_argument(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        argument: &dir::StaticArgument,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let value = self.statics.get_static(argument.value);

        self.lower_semantic_static_type_expression(source_id, value)
    }

    /// Lower one normalized static string into a JS string literal type.
    fn lower_static_string_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &str,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let value = self.strings.intern(value);
        let literal = js::ScalarLiteral::String(value);
        let literal = js::TypeLiteral::ScalarLiteral(literal);
        let ty = js::TypeExpression::Scalar(literal);

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one semantic static argument list into JS type arguments.
    fn lower_semantic_static_type_arguments(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        arguments: &[dir::StaticArgument],
    ) -> CodegenJsResult<Vec<js::LocalNodeId<js::TypeExpression>>> {
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
        generic_arguments: Option<&[dir::StaticArgument]>,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        if symbol_id.module_id != self.module.id {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "remote semantic type references need source-backed lowering in JS output"
                        .to_string(),
                ),
            });
        }

        let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
        let Some(dir::StaticKey::Name(name)) = symbol.key else {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "type reference symbol is missing a path-like key in JS output".to_string(),
                ),
            });
        };

        let segment = name;
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let generic_arguments = generic_arguments
            .map(|arguments| self.lower_semantic_static_type_arguments(source_id, arguments))
            .transpose()?
            .unwrap_or_default();
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments,
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one generated generic parameter into a JS path type.
    fn lower_generated_parameter_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        key: dir::GenericSlotKey,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let dir::GenericSlotKey::Generated(name) = key else {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some("generic parameter slot is not generated".to_string()),
            });
        };
        let path = js::Path {
            segments: smallvec::smallvec![name],
        };
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments: Vec::new(),
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one source-backed reference type into a JS path type when exact path syntax exists.
    fn try_lower_reference_type_from_source(
        &mut self,
        source_id: dir::LocalNodeIdAny,
    ) -> CodegenJsResult<Option<js::LocalNodeId<js::TypeExpression>>> {
        let Ok(expression_id) = source_id.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };
        let expression = self.dir_tree.get(expression_id);
        let (path, generic_arguments) = match expression {
            dir::Expression::QualifiedReference {
                path,
                generic_arguments,
                ..
            } => (path, generic_arguments.as_slice()),
            _ => return Ok(None),
        };

        let path = self.lower_path(source_id, path)?;
        let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments,
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
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeMember>> {
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
        let key = self.lower_static_key(source_id, field.key)?;
        let field = match self.types.get_type(field.ty) {
            dir::Type::Function(_) => {
                let signature =
                    self.lower_semantic_function_type_declaration(source_id, field.ty)?;
                js::TypeMember::Method {
                    modifiers,
                    key,
                    signature: js::FunctionSignature {
                        asynchrony: js::Asynchrony::Sync,
                        role: None,
                        form: js::FunctionForm::Function,
                        generic_parameters: signature.generic_parameters,
                        this_parameter: signature.this_parameter,
                        parameters: signature.parameters,
                        return_type: signature.return_type,
                        is_abstract: false,
                        is_override: false,
                        is_generator: false,
                    },
                }
            }
            _ => {
                let ty = self.lower_type(field.ty)?;
                js::TypeMember::Field { modifiers, key, ty }
            }
        };

        Ok(self
            .tree
            .insert_from_source_any(field, self.module.id, source_id))
    }

    /// Lower one semantic function type generic parameter with one synthesized name.
    fn lower_semantic_function_generic_parameter(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        name: String,
        ty_id: dir::LocalTypeId,
    ) -> CodegenJsResult<js::LocalNodeId<js::GenericParameter>> {
        let name = self.strings.intern(&name);
        let constraint = Some(self.lower_type(ty_id)?);
        let parameter = js::GenericParameter::Type {
            modifiers: None,
            name,
            constraint,
            default: None,
        };

        Ok(self
            .tree
            .insert_from_source_any(parameter, self.module.id, source_id))
    }

    /// Lower one semantic function type parameter with one synthesized name.
    fn lower_semantic_function_parameter(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        name: String,
        parameter: dir::FunctionParameterType,
    ) -> CodegenJsResult<js::LocalNodeId<js::Parameter>> {
        let name = self.strings.intern(&name);
        let ty = Some(self.lower_type(parameter.ty)?);
        let modifiers = if parameter.is_optional {
            Some(js::BindingModifier {
                kind: Some(js::BindingKind::Maybe),
                ..js::BindingModifier::default()
            })
        } else {
            None
        };
        let parameter = if parameter.is_rest {
            js::Parameter::VariadicNamed {
                modifiers,
                name,
                ty,
            }
        } else {
            js::Parameter::Named {
                modifiers,
                name,
                ty,
                default: None,
            }
        };

        Ok(self
            .tree
            .insert_from_source_any(parameter, self.module.id, source_id))
    }

    /// Lower one semantic function type into one JS function type declaration.
    fn lower_semantic_function_type_declaration(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        ty_id: dir::LocalTypeId,
    ) -> CodegenJsResult<js::FunctionTypeDeclaration> {
        let dir::Type::Function(function) = self.types.get_type(ty_id) else {
            return Err(CodegenJsError::UnsupportedConstruct {
                node: source_id.into_global(self.module.id),
                message: Some(
                    "semantic function signature lowering expected a function type".to_string(),
                ),
            });
        };

        // generic parameters
        let generic_parameters = function
            .generic_parameters
            .iter()
            .enumerate()
            .map(|(index, parameter_type_id)| {
                self.lower_semantic_function_generic_parameter(
                    source_id,
                    format!("T{index}"),
                    *parameter_type_id,
                )
            })
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        // this parameter
        let this_parameter = function
            .this_parameter
            .map(|this_type_id| {
                self.lower_semantic_function_parameter(
                    source_id,
                    "this".to_string(),
                    dir::FunctionParameterType {
                        ty: this_type_id,
                        is_optional: false,
                        is_rest: false,
                    },
                )
            })
            .transpose()?;

        // runtime parameters
        let parameters = function
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                self.lower_semantic_function_parameter(source_id, format!("arg{index}"), *parameter)
            })
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        // return type
        let return_type = function
            .return_type
            .map(|return_type_id| self.lower_type(return_type_id))
            .transpose()?;

        Ok(js::FunctionTypeDeclaration {
            generic_parameters,
            this_parameter,
            parameters,
            return_type,
        })
    }

    /// Lower one semantic object type index signature into a JS object type field.
    fn lower_object_type_index_signature(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        signature: &dir::TypeIndexSignature,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeMember>> {
        let modifiers = if signature.is_readonly {
            Some(js::BindingModifier {
                mutability: Some(js::Mutability::Immutable),
                ..js::BindingModifier::default()
            })
        } else {
            None
        };
        let name = signature.name;
        let key_type = self.lower_type(signature.key_type)?;
        let value_type = self.lower_type(signature.value_type)?;
        let field = js::TypeMember::IndexSignature {
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
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeExpression>> {
        let source_id = self.types.get_type_source(ty_id);
        let ty = self.types.get_type(ty_id);

        let ty_id = match ty {
            dir::Type::Never => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Never);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Any => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Any);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Unknown => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Unknown);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Void => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Void);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Null => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Null);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Undefined => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Undefined);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Object => {
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Object);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Primitive(primitive) => {
                let primitive = self.lower_primitive_type_value(*primitive);
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::Primitive(primitive));
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Literal(literal) => {
                let literal = self.lower_scalar_literal(literal);
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::ScalarLiteral(literal));
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Operation(operation) => match operation {
                dir::TypeOperation::BuiltinTypeFunction(function) => {
                    self.lower_builtin_type_function(source_id, *function)?
                }
                dir::TypeOperation::Conditional(conditional) => {
                    let left = self.lower_type(conditional.left)?;
                    let right = self.lower_type(conditional.right)?;
                    let then_type = self.lower_type(conditional.then_type)?;
                    let else_type = self.lower_type(conditional.else_type)?;
                    let ty = js::TypeExpression::Conditional {
                        left,
                        right,
                        then_type,
                        else_type,
                    };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::Mapped(mapped) => {
                    let name = mapped.parameter.name;
                    let source_type = self.lower_type(mapped.parameter.constraint)?;
                    let key_remap = mapped
                        .parameter
                        .key_remap
                        .map(|key_remap| self.lower_type(key_remap))
                        .transpose()?;
                    let parameter = js::TypeMappedParameter {
                        name,
                        source_type,
                        key_remap,
                    };
                    let modifiers = self.lower_type_mapped_modifiers(mapped.modifiers);
                    let value = self.lower_type(mapped.value)?;
                    let ty = js::TypeExpression::Mapped {
                        parameter,
                        modifiers,
                        value: Some(value),
                    };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::Index(index_type) => {
                    let left = self.lower_type(index_type.left)?;
                    let index = self.lower_type(index_type.index)?;
                    let ty = js::TypeExpression::Index { left, index };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::TemplateLiteral(template) => {
                    let strings = template
                        .strings
                        .iter()
                        .map(|string| *string)
                        .collect::<Vec<_>>();
                    let spans = template
                        .spans
                        .iter()
                        .map(|span| self.lower_type(*span))
                        .collect::<Result<Vec<_>, CodegenJsError>>()?;
                    let template = js::TypeTemplateLiteral { strings, spans };
                    let ty = js::TypeExpression::TemplateLiteral(template);

                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::Infer(infer) => {
                    let name = infer.name.unwrap_or_else(|| self.strings.intern("_"));
                    let constraint = infer
                        .constraint
                        .map(|constraint| self.lower_type(constraint))
                        .transpose()?;
                    let ty = js::TypeExpression::Infer { name, constraint };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::KeyOf(unary) => {
                    let target_type = self.lower_type(unary.target)?;
                    let ty = js::TypeExpression::KeyOf { target_type };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
            },
            dir::Type::Parameter(parameter) => match parameter.symbol() {
                Some(symbol) => self.lower_reference_type_from_symbol(source_id, symbol, None)?,
                None => self.lower_generated_parameter_type(source_id, parameter.key)?,
            },
            dir::Type::This => self.tree.insert_from_source_any(
                js::TypeExpression::This,
                self.module.id,
                source_id,
            ),
            dir::Type::Named(reference) => {
                let type_id = self
                    .try_lower_reference_type_from_source(source_id)?
                    .map_or_else(
                        || {
                            self.lower_reference_type_from_symbol(
                                source_id,
                                reference.symbol,
                                Some(reference.arguments.as_slice()),
                            )
                        },
                        Ok,
                    )?;
                self.set_global_node_symbol(type_id, reference.symbol);
                type_id
            }
            dir::Type::Dynamic(dynamic) => self.lower_type(dynamic.constraint)?,
            dir::Type::Predicate(predicate) => {
                let subject = self.lower_type_predicate_subject(source_id, predicate.subject)?;
                let target = predicate
                    .target
                    .map(|target| self.lower_type(target))
                    .transpose()?;
                let ty = js::TypeExpression::Predicate {
                    asserts: predicate.asserts,
                    subject,
                    target,
                };

                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }

            dir::Type::Form(form) => self.lower_type(form.value)?,
            dir::Type::Slice(slice) => {
                let element = self.lower_type(slice.element)?;
                let mut array_id = self.tree.insert_from_source_any(
                    js::TypeExpression::Array { element },
                    self.module.id,
                    source_id,
                );
                if slice.is_readonly {
                    let readonly = js::TypeExpression::Readonly {
                        target_type: array_id,
                    };
                    array_id =
                        self.tree
                            .insert_from_source_any(readonly, self.module.id, source_id);
                }
                array_id
            }
            dir::Type::FixedArray(array) => {
                let element = self.lower_type(array.element)?;
                let mut array_id = self.tree.insert_from_source_any(
                    js::TypeExpression::Array { element },
                    self.module.id,
                    source_id,
                );
                if array.is_readonly {
                    let readonly = js::TypeExpression::Readonly {
                        target_type: array_id,
                    };
                    array_id =
                        self.tree
                            .insert_from_source_any(readonly, self.module.id, source_id);
                }

                array_id
            }
            dir::Type::Range(_) => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: Some("range types must be reduced before JS lowering".to_string()),
                });
            }
            dir::Type::Shape(object) => {
                let mut members = object
                    .fields
                    .iter()
                    .map(|field| self.lower_object_type_field(source_id, field))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let call_signatures = object
                    .call_signatures
                    .iter()
                    .map(|signature_id| {
                        let signature = self
                            .lower_semantic_function_type_declaration(source_id, *signature_id)?;
                        let field = js::TypeMember::CallSignature {
                            modifiers: None,
                            signature,
                        };

                        Ok(self
                            .tree
                            .insert_from_source_any(field, self.module.id, source_id))
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                members.extend(call_signatures);

                let construct_signatures = object
                    .construct_signatures
                    .iter()
                    .map(|signature_id| {
                        let signature = self
                            .lower_semantic_function_type_declaration(source_id, *signature_id)?;
                        let field = js::TypeMember::ConstructSignature {
                            modifiers: None,
                            signature: js::ConstructorTypeDeclaration {
                                is_abstract: false,
                                generic_parameters: signature.generic_parameters,
                                parameters: signature.parameters,
                                return_type: signature.return_type,
                            },
                        };

                        Ok(self
                            .tree
                            .insert_from_source_any(field, self.module.id, source_id))
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                members.extend(construct_signatures);

                let index_signatures = object
                    .index_signatures
                    .iter()
                    .map(|signature| self.lower_object_type_index_signature(source_id, signature))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                members.extend(index_signatures);

                let ty = js::TypeExpression::Object { members };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Tuple(tuple) => {
                let elements = tuple
                    .elements
                    .iter()
                    .map(|element| self.lower_tuple_element(source_id, element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let mut tuple_id = self.tree.insert_from_source_any(
                    js::TypeExpression::Tuple { elements },
                    self.module.id,
                    source_id,
                );
                if tuple.is_readonly {
                    let readonly = js::TypeExpression::Readonly {
                        target_type: tuple_id,
                    };
                    tuple_id =
                        self.tree
                            .insert_from_source_any(readonly, self.module.id, source_id);
                }
                tuple_id
            }
            dir::Type::Union(union) => {
                let elements = union
                    .elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = js::TypeExpression::Union { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Intersection(intersection) => {
                let elements = intersection
                    .elements
                    .iter()
                    .map(|element| self.lower_type(*element))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let ty = js::TypeExpression::Intersection { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Function(_) => {
                let signature = self.lower_semantic_function_type_declaration(source_id, ty_id)?;
                let ty = js::TypeExpression::FunctionTypeDeclaration(signature);
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
