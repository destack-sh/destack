use crate::EmitError;
use crate::emit::js::ModuleLowerer;
use destack_dir as dir;
use destack_js as js;

impl ModuleLowerer<'_> {
    /// Lower one type annotation expression into a JS type.
    pub(crate) fn lower_type_annotation_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let global_id = expression_id.into_global_any(self.module.id);

        // lower through the semantic type lane
        if let Some(type_id) = self.types.get_node_type_id(global_id) {
            return self.lower_type(type_id, expression_id.into_any());
        }

        Err(self.unsupported_construct(
            global_id,
            Some("missing declared type for type annotation".to_string()),
        ))
    }

    /// Lower a mutability from DIR into JS AST.
    pub(crate) fn lower_mutability(&self, mutability: dir::Mutability) -> js::Mutability {
        match mutability {
            dir::Mutability::Immutable => js::Mutability::Immutable,
            dir::Mutability::Mutable | dir::Mutability::Exclusive => js::Mutability::Mutable,
        }
    }

    /// Lower one for each declaration kind from DIR into JS AST.
    pub(crate) fn lower_for_each_keyword(
        &self,
        keyword: dir::BindingKeyword,
    ) -> js::BindingKeyword {
        match keyword {
            dir::BindingKeyword::Let => js::BindingKeyword::Let,
            dir::BindingKeyword::Const => js::BindingKeyword::Const,
        }
    }

    /// Lower one tuple element from DIR into JS AST.
    pub(crate) fn lower_tuple_element(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        element: &dir::TypeElement,
    ) -> Result<js::LocalNodeId<js::TupleElement>, EmitError> {
        let label = element.label;
        let ty = self.lower_type(element.ty, source_id)?;
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
    pub(crate) fn lower_generic_parameters(
        &mut self,
        parameters: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> Result<Vec<js::LocalNodeId<js::GenericParameter>>, EmitError> {
        let generic_parameters = parameters
            .iter()
            .map(|parameter_id| self.lower_generic_parameter(*parameter_id))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(generic_parameters)
    }

    /// Lower one generic parameter from DIR into a JS generic parameter.
    fn lower_generic_parameter(
        &mut self,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> Result<js::LocalNodeId<js::GenericParameter>, EmitError> {
        let parameter = self.dir_tree.get(parameter_id);

        let parameter = match parameter {
            dir::GenericParameter::Type {
                name,
                variance,
                constraint,
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
                return Err(self.unsupported_construct(
                    parameter_id.into_global_any(self.module.id),
                    Some("variadic generic parameters are not lowered to JS yet".to_string()),
                ));
            }
            dir::GenericParameter::Value { .. } | dir::GenericParameter::VariadicValue { .. } => {
                return Err(self.unsupported_construct(
                    parameter_id.into_global_any(self.module.id),
                    Some("value generic parameters are not lowered to JS yet".to_string()),
                ));
            }
            dir::GenericParameter::Error => {
                return Err(self.unsupported_construct(
                    parameter_id.into_global_any(self.module.id),
                    Some("generic parameter error slots are not lowered to JS".to_string()),
                ));
            }
        };

        Ok(self
            .tree
            .insert_from_source(parameter, self.module.id, parameter_id))
    }

    /// Lower type annotation expressions from DIR into JS types.
    pub(crate) fn lower_type_annotation_expressions(
        &mut self,
        expressions: &[dir::LocalNodeId<dir::TypeExpression>],
    ) -> Result<Vec<js::LocalNodeId<js::TypeExpression>>, EmitError> {
        expressions
            .iter()
            .map(|expression_id| self.lower_type_annotation_expression(*expression_id))
            .collect()
    }

    /// Lower a primitive type from DIR into JS AST.
    pub(crate) fn lower_primitive_type_value(
        &self,
        primitive: dir::PrimitiveType,
    ) -> js::PrimitiveType {
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

    /// Lower one parsed type literal value from DIR into JS AST.
    pub(crate) fn lower_type_literal_value(
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
            | dir::TypeLiteral::Alias(_)
            | dir::TypeLiteral::Integer(_)
            | dir::TypeLiteral::Float(_) => js::TypeLiteral::Primitive(js::PrimitiveType::Number),
            dir::TypeLiteral::Symbol => js::TypeLiteral::Primitive(js::PrimitiveType::Symbol),
            dir::TypeLiteral::UniqueSymbol => {
                js::TypeLiteral::Primitive(js::PrimitiveType::UniqueSymbol)
            }
        };
        Some(literal)
    }

    /// Lower a parsed type literal from DIR into JS AST.
    pub(crate) fn lower_type_literal(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        literal: &dir::TypeLiteral,
    ) -> Result<js::TypeLiteral, EmitError> {
        self.lower_type_literal_value(literal)
            .ok_or_else(|| self.unsupported_construct(source_id.into_global(self.module.id), None))
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

    /// Lower one string mapping reference into a JS path type.
    fn lower_string_mapping(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        mapping: dir::StringMapping,
        target: dir::GlobalTypeId,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let mapping_name = match mapping {
            dir::StringMapping::Uppercase => "Uppercase",
            dir::StringMapping::Lowercase => "Lowercase",
            dir::StringMapping::Capitalize => "Capitalize",
            dir::StringMapping::Uncapitalize => "Uncapitalize",
        };
        self.lower_type_function(source_id, mapping_name, target)
    }

    /// Lower one unary type function into a JS path type.
    fn lower_type_function(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        name: &str,
        target: dir::GlobalTypeId,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let segment = self.strings.intern(name);
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let target = self.lower_type(target, source_id)?;
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments: vec![target],
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one DIR static argument into one JS type argument.
    pub(crate) fn lower_static_type_argument(
        &mut self,
        argument_id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let argument = self.dir_tree.get(argument_id);

        match argument {
            dir::GenericArgument::Type { value, .. }
            | dir::GenericArgument::AssociatedType { value, .. } => {
                self.lower_type_annotation_expression(*value)
            }
            dir::GenericArgument::SpreadType { .. } => Err(self.unsupported_construct(
                argument_id.into_global_any(self.module.id),
                Some("variadic generic arguments are not lowered to JS yet".to_string()),
            )),
            dir::GenericArgument::Value { .. }
            | dir::GenericArgument::SpreadValue { .. }
            | dir::GenericArgument::AssociatedConst { .. } => Err(self.unsupported_construct(
                argument_id.into_global_any(self.module.id),
                Some("value generic arguments are not lowered to JS yet".to_string()),
            )),
            dir::GenericArgument::Error => Err(self.unsupported_construct(
                argument_id.into_global_any(self.module.id),
                Some("argument error slots are not lowered to JS type arguments".to_string()),
            )),
        }
    }

    /// Lower one source static argument list into JS type arguments.
    pub(crate) fn lower_static_type_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Result<Vec<js::LocalNodeId<js::TypeExpression>>, EmitError> {
        let generic_arguments = arguments
            .iter()
            .map(|argument| self.lower_static_type_argument(*argument))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(generic_arguments)
    }

    /// Lower one semantic static expression into one JS type argument.
    fn lower_semantic_static_type_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &dir::StaticTerm,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        match value {
            dir::StaticTerm::ScalarLiteral { value } => {
                let literal = self.lower_scalar_literal(value);
                let ty = js::TypeExpression::Scalar(js::TypeLiteral::ScalarLiteral(literal));

                Ok(self
                    .tree
                    .insert_from_source_any(ty, self.module.id, source_id))
            }
            dir::StaticTerm::Type { ty } => self.lower_type(*ty, source_id),
            dir::StaticTerm::Array { .. }
            | dir::StaticTerm::FixedArray { .. }
            | dir::StaticTerm::Tuple { .. }
            | dir::StaticTerm::Newtype { .. }
            | dir::StaticTerm::Object { .. }
            | dir::StaticTerm::Struct { .. } => Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some("static value arguments do not lower to JS type arguments".to_string()),
            )),
        }
    }

    /// Lower one normalized static string into a JS string literal type.
    fn lower_static_string_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &str,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let value = self.strings.intern(value);
        let literal = js::ScalarLiteral::String(value);
        let literal = js::TypeLiteral::ScalarLiteral(literal);
        let ty = js::TypeExpression::Scalar(literal);

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one reference symbol into a JS path type.
    fn lower_reference_type_from_symbol(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        symbol_id: dir::GlobalSymbolId,
        generic_arguments: Option<&[dir::GlobalTypeId]>,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        if symbol_id.module_id != self.module.id {
            return Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some(
                    "remote semantic type references need source-backed lowering in JS output"
                        .to_string(),
                ),
            ));
        }

        let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
        let Some(dir::StaticKey::Name(name)) = symbol.key else {
            return Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some("type reference symbol is missing a path-like key in JS output".to_string()),
            ));
        };

        let segment = name;
        let path = js::Path {
            segments: smallvec::smallvec![segment],
        };
        let generic_arguments = generic_arguments
            .unwrap_or_default()
            .iter()
            .map(|argument| self.lower_type(*argument, source_id))
            .collect::<Result<Vec<_>, EmitError>>()?;
        let ty = js::TypeExpression::Path {
            path,
            generic_arguments,
        };

        Ok(self
            .tree
            .insert_from_source_any(ty, self.module.id, source_id))
    }

    /// Lower one generic parameter into a JS path type.
    fn lower_generic_parameter_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        parameter: dir::GlobalGenericParameterId,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        // reject foreign generics
        if parameter.module_id != self.module.id {
            return Err(self.internal_error(format!(
                "JS lowering cannot read foreign DIR generic {parameter:?}"
            )));
        }

        // lower by committed parameter key
        let key = self.generics.get_parameter(parameter.local_id).key;
        match key {
            dir::GenericParameterKey::Symbol(symbol) => {
                self.lower_reference_type_from_symbol(source_id, symbol, None)
            }
            dir::GenericParameterKey::Generated(_) => {
                self.lower_generated_parameter_type(source_id, key)
            }
        }
    }

    /// Lower one generated generic parameter into a JS path type.
    fn lower_generated_parameter_type(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        key: dir::GenericParameterKey,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let dir::GenericParameterKey::Generated(name) = key else {
            return Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some("generic parameter key is not generated".to_string()),
            ));
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
    ) -> Result<Option<js::LocalNodeId<js::TypeExpression>>, EmitError> {
        let Ok(expression_id) = source_id.try_into_typed::<dir::Expression>() else {
            return Ok(None);
        };
        // a static name path lowers as a reference; anything else is not a type path
        let Some(path) = self.dir_tree.reference_path(expression_id) else {
            return Ok(None);
        };
        let generic_arguments: &[dir::LocalNodeId<dir::GenericArgument>] = &[];

        let path = self.lower_path(source_id, &path)?;
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
    ) -> Result<js::LocalNodeId<js::TypeMember>, EmitError> {
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
        let field = match self.require_type(field.ty)? {
            dir::Type::FunctionSignature(_) => {
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
                let ty = self.lower_type(field.ty, source_id)?;
                js::TypeMember::Field { modifiers, key, ty }
            }
        };

        Ok(self
            .tree
            .insert_from_source_any(field, self.module.id, source_id))
    }

    /// Lower one semantic function type generic parameter.
    fn lower_semantic_function_generic_parameter(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        parameter_id: dir::GlobalGenericParameterId,
    ) -> Result<js::LocalNodeId<js::GenericParameter>, EmitError> {
        if parameter_id.module_id != self.module.id {
            return Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some(format!(
                    "JS lowering cannot read foreign semantic generic {parameter_id:?}"
                )),
            ));
        }

        let parameter = self.generics.get_parameter(parameter_id.local_id);
        let name = match parameter.key {
            dir::GenericParameterKey::Symbol(symbol) => {
                let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol));
                let Some(dir::StaticKey::Name(name)) = symbol.key else {
                    return Err(self.unsupported_construct(
                        source_id.into_global(self.module.id),
                        Some(
                            "semantic generic parameter is missing a path-like key in JS output"
                                .to_string(),
                        ),
                    ));
                };

                name
            }
            dir::GenericParameterKey::Generated(name) => name,
        };
        let constraint = parameter
            .constraint
            .map(|constraint| self.lower_type(constraint, source_id))
            .transpose()?;
        let default = parameter
            .default
            .map(|default| self.lower_type(default, source_id))
            .transpose()?;
        let parameter = js::GenericParameter::Type {
            modifiers: None,
            name,
            constraint,
            default,
        };

        Ok(self
            .tree
            .insert_from_source_any(parameter, self.module.id, source_id))
    }

    /// Lower one semantic signature parameter with one synthesized name.
    fn lower_semantic_function_parameter(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        name: String,
        parameter: dir::FunctionParameterType,
    ) -> Result<js::LocalNodeId<js::Parameter>, EmitError> {
        let name = self.strings.intern(&name);
        let ty = Some(self.lower_type(parameter.ty, source_id)?);
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
        ty_id: dir::GlobalTypeId,
    ) -> Result<js::FunctionTypeDeclaration, EmitError> {
        let dir::Type::FunctionSignature(function) = self.require_type(ty_id)? else {
            return Err(self.unsupported_construct(
                source_id.into_global(self.module.id),
                Some("semantic function signature lowering expected a function type".to_string()),
            ));
        };
        let function = *self.types.signature(function);

        // generic parameters
        let generic_parameters = match function.template {
            Some(template) if template.module_id == self.module.id => self
                .generics
                .get_template(template.local_id)
                .parameters
                .iter()
                .map(|parameter| parameter.into_global(self.module.id))
                .map(|parameter| {
                    self.lower_semantic_function_generic_parameter(source_id, parameter)
                })
                .collect::<Result<Vec<_>, EmitError>>()?,
            Some(template) => {
                return Err(self.unsupported_construct(
                    source_id.into_global(self.module.id),
                    Some(format!(
                        "JS lowering cannot read foreign semantic template {template:?}"
                    )),
                ));
            }
            None => Vec::new(),
        };

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
        let parameters = self
            .types
            .parameters(function.parameters)
            .iter()
            .copied()
            .enumerate()
            .map(|(index, parameter)| {
                self.lower_semantic_function_parameter(source_id, format!("arg{index}"), parameter)
            })
            .collect::<Result<Vec<_>, EmitError>>()?;

        // return type
        let return_type = function
            .return_type
            .map(|return_type_id| self.lower_type(return_type_id, source_id))
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
    ) -> Result<js::LocalNodeId<js::TypeMember>, EmitError> {
        let modifiers = if signature.is_readonly {
            Some(js::BindingModifier {
                mutability: Some(js::Mutability::Immutable),
                ..js::BindingModifier::default()
            })
        } else {
            None
        };
        let name = signature.name;
        let key_type = self.lower_type(signature.key_type, source_id)?;
        let value_type = self.lower_type(signature.value_type, source_id)?;
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
    // NOTE #Incomplete: per-type source provenance no longer exists (types are
    // interned/hash-consed); `source_id` anchors every node this call produces
    // to the caller's own source node instead of the type's original write site.
    pub(crate) fn lower_type(
        &mut self,
        ty_id: dir::GlobalTypeId,
        source_id: dir::LocalNodeIdAny,
    ) -> Result<js::LocalNodeId<js::TypeExpression>, EmitError> {
        let ty = self.require_type(ty_id)?;

        let ty_id = match &ty {
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
            dir::Type::Operation(operation) => match self.types.operation(*operation) {
                dir::TypeOperation::StringMapping { mapping, target } => {
                    self.lower_string_mapping(source_id, *mapping, *target)?
                }
                dir::TypeOperation::Conditional(conditional) => {
                    let left = self.lower_type(conditional.left, source_id)?;
                    let right = self.lower_type(conditional.right, source_id)?;
                    let then_type = self.lower_type(conditional.then_type, source_id)?;
                    let else_type = self.lower_type(conditional.else_type, source_id)?;
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
                    let source_type = self.lower_type(mapped.parameter.constraint, source_id)?;
                    let key_remap = mapped
                        .parameter
                        .key_remap
                        .map(|key_remap| self.lower_type(key_remap, source_id))
                        .transpose()?;
                    let parameter = js::TypeMappedParameter {
                        name,
                        source_type,
                        key_remap,
                    };
                    let modifiers = self.lower_type_mapped_modifiers(mapped.modifiers);
                    let value = self.lower_type(mapped.value, source_id)?;
                    let ty = js::TypeExpression::Mapped {
                        parameter,
                        modifiers,
                        value: Some(value),
                    };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::Index(index_type) => {
                    let left = self.lower_type(index_type.left, source_id)?;
                    let index = self.lower_type(index_type.index, source_id)?;
                    let ty = js::TypeExpression::Index { left, index };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::TemplateLiteral(template) => {
                    let strings = self.types.strings(template.strings).to_vec();
                    let spans = self
                        .types
                        .type_ids(template.spans)
                        .to_vec()
                        .iter()
                        .map(|span| self.lower_type(*span, source_id))
                        .collect::<Result<Vec<_>, EmitError>>()?;
                    let template = js::TypeTemplateLiteral { strings, spans };
                    let ty = js::TypeExpression::TemplateLiteral(template);

                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::Infer(infer) => {
                    let name = infer.name.unwrap_or_else(|| self.strings.intern("_"));
                    let constraint = infer
                        .constraint
                        .map(|constraint| self.lower_type(constraint, source_id))
                        .transpose()?;
                    let ty = js::TypeExpression::Infer { name, constraint };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::KeyOf(unary) => {
                    let target_type = self.lower_type(unary.target, source_id)?;
                    let ty = js::TypeExpression::KeyOf { target_type };
                    self.tree
                        .insert_from_source_any(ty, self.module.id, source_id)
                }
                dir::TypeOperation::NoInfer(unary) => {
                    self.lower_type_function(source_id, "NoInfer", unary.target)?
                }
                // narrowing and static operations close before lowering
                dir::TypeOperation::Narrow(_)
                | dir::TypeOperation::TypeOf(_)
                | dir::TypeOperation::Awaited(_)
                | dir::TypeOperation::TryOutput { .. }
                | dir::TypeOperation::TryResidual { .. }
                | dir::TypeOperation::StaticBinary(_)
                | dir::TypeOperation::StaticUnary(_) => {
                    return Err(self.internal_error(
                        "JS lowering cannot emit an unevaluated type operation".to_string(),
                    ));
                }
            },
            dir::Type::Parameter(parameter) => {
                self.lower_generic_parameter_type(source_id, *parameter)?
            }
            dir::Type::This => self.tree.insert_from_source_any(
                js::TypeExpression::This,
                self.module.id,
                source_id,
            ),
            dir::Type::Application(reference) => {
                let arguments = self.types.type_ids(reference.arguments);
                let type_id = self
                    .try_lower_reference_type_from_source(source_id)?
                    .map_or_else(
                        || {
                            self.lower_reference_type_from_symbol(
                                source_id,
                                reference.symbol,
                                Some(arguments),
                            )
                        },
                        Ok,
                    )?;
                self.set_global_node_symbol(type_id, reference.symbol);
                type_id
            }
            dir::Type::Dynamic(dynamic) => self.lower_type(dynamic.constraint, source_id)?,
            dir::Type::Form(form) => {
                let value = self.lower_type(form.value, source_id)?;
                match form.form {
                    dir::Form::Readonly => {
                        let ty = js::TypeExpression::Readonly { target_type: value };

                        self.tree
                            .insert_from_source_any(ty, self.module.id, source_id)
                    }
                    _ => value,
                }
            }
            dir::Type::Slice(slice) => {
                let element = self.lower_type(slice.element, source_id)?;
                self.tree.insert_from_source_any(
                    js::TypeExpression::Array { element },
                    self.module.id,
                    source_id,
                )
            }
            dir::Type::FixedArray(array) => {
                let element = self.lower_type(array.element, source_id)?;
                self.tree.insert_from_source_any(
                    js::TypeExpression::Array { element },
                    self.module.id,
                    source_id,
                )
            }
            dir::Type::Range(_) => {
                return Err(self.unsupported_construct(
                    source_id.into_global(self.module.id),
                    Some("range types must be reduced before JS lowering".to_string()),
                ));
            }
            dir::Type::Shape(object) => {
                let mut members = self
                    .types
                    .fields(object.fields)
                    .to_vec()
                    .iter()
                    .map(|field| self.lower_object_type_field(source_id, field))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                let call_signatures = self
                    .types
                    .type_ids(object.call_signatures)
                    .to_vec()
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
                    .collect::<Result<Vec<_>, EmitError>>()?;
                members.extend(call_signatures);

                let construct_signatures = self
                    .types
                    .type_ids(object.construct_signatures)
                    .to_vec()
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
                    .collect::<Result<Vec<_>, EmitError>>()?;
                members.extend(construct_signatures);

                let index_signatures = self
                    .types
                    .index_signatures(object.index_signatures)
                    .to_vec()
                    .iter()
                    .map(|signature| self.lower_object_type_index_signature(source_id, signature))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                members.extend(index_signatures);

                let ty = js::TypeExpression::Object { members };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Tuple(tuple) => {
                let elements = self
                    .types
                    .elements(tuple.elements)
                    .to_vec()
                    .iter()
                    .map(|element| self.lower_tuple_element(source_id, element))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                self.tree.insert_from_source_any(
                    js::TypeExpression::Tuple { elements },
                    self.module.id,
                    source_id,
                )
            }
            dir::Type::Union(union) => {
                let elements = self
                    .types
                    .type_ids(union.elements)
                    .to_vec()
                    .iter()
                    .map(|element| self.lower_type(*element, source_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let ty = js::TypeExpression::Union { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .types
                    .type_ids(intersection.elements)
                    .to_vec()
                    .iter()
                    .map(|element| self.lower_type(*element, source_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let ty = js::TypeExpression::Intersection { elements };
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::FunctionSignature(_) => {
                let signature = self.lower_semantic_function_type_declaration(source_id, ty_id)?;
                let ty = js::TypeExpression::FunctionTypeDeclaration(signature);
                self.tree
                    .insert_from_source_any(ty, self.module.id, source_id)
            }
            dir::Type::Memory(memory) => {
                let value = match memory {
                    dir::MemoryLiteral::Access(access) => format!("{access:?}").to_lowercase(),
                    dir::MemoryLiteral::Ownership(ownership) => ownership.text().to_string(),
                    dir::MemoryLiteral::Space(space) => format!("{space:?}").to_lowercase(),
                    dir::MemoryLiteral::Place(dir::Place::Relative) => "relative".to_string(),
                    dir::MemoryLiteral::Place(dir::Place::Space(space)) => {
                        format!("{space:?}").to_lowercase()
                    }
                    dir::MemoryLiteral::Lifetime(dir::Lifetime::Static) => "static".to_string(),
                    dir::MemoryLiteral::Lifetime(dir::Lifetime::Frame) => "frame".to_string(),
                };

                self.lower_static_string_type(source_id, &value)?
            }
            dir::Type::Static(static_id) => {
                let value = self.require_static(*static_id)?.clone();

                self.lower_semantic_static_type_expression(source_id, &value)?
            }
            _ => {
                return Err(self.unsupported_construct(source_id.into_global(self.module.id), None));
            }
        };

        Ok(ty_id)
    }
}
