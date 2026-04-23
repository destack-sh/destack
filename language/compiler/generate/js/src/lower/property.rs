use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Build one JS binding modifier for properties and members.
    fn build_member_modifier(
        &self,
        kind: Option<js::BindingKind>,
        anchor: Option<js::BindingAnchor>,
        mutability: Option<js::Mutability>,
        visibility: Option<js::Visibility>,
        operator: Option<js::BindingOperator>,
        accessor: Option<js::AccessorKind>,
    ) -> Option<js::BindingModifier> {
        let modifiers = js::BindingModifier {
            kind,
            variance: None,
            anchor,
            mutability,
            visibility,
            operator,
            definite: false,
            accessor,
        };

        if modifiers == js::BindingModifier::default() {
            None
        } else {
            Some(modifiers)
        }
    }

    /// Lower a property from DIR into JS AST.
    pub fn lower_property(
        &mut self,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Property>> {
        let property = self.dir_tree.get(property_id);
        let property = match property {
            dir::Property::Field {
                key,
                value,
                is_shorthand,
                symbol: _,
            } => {
                let modifiers = None;
                let key = self.lower_key(*key)?;
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;

                js::Property::Field {
                    modifiers,
                    key,
                    value,
                    is_shorthand: *is_shorthand,
                }
            }
            dir::Property::Method {
                key,
                signature,
                body,
                symbol: _,
            } => {
                let modifiers = None;
                let key = key.map(|key| self.lower_key(key)).transpose()?;
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .map(|body| self.lower_expression_as_block(body))
                    .transpose()?;

                js::Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread { value, symbol: _ } => {
                let modifiers = None;
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;

                js::Property::Spread { modifiers, value }
            }
            dir::Property::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: property_id.into_global_any(self.module.id),
                    message: Some("property error slots are not lowered to JS".to_string()),
                });
            }
        };
        let property_id = self
            .tree
            .insert_from_source(property, self.module.id, property_id);

        Ok(property_id)
    }

    /// Lower a member from DIR into JS AST.
    /// Lower a type member from DIR into JS AST.
    pub fn lower_type_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
    ) -> CodegenJsResult<js::LocalNodeId<js::TypeMember>> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
            dir::TypeMember::Field {
                is_optional,
                is_readonly,
                key,
                declared_type,
                ..
            } => {
                let modifiers = self.build_member_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    None,
                    if *is_readonly {
                        Some(js::Mutability::Immutable)
                    } else {
                        None
                    },
                    None,
                    None,
                    None,
                );
                let key = self.lower_key(*key)?;
                let Some(declared_type) = declared_type else {
                    return Err(CodegenJsError::MissingType {
                        node: member_id.into_global_any(self.module.id),
                        message: Some("type member field is missing its declared type".to_string()),
                    });
                };
                let ty = self.lower_type_annotation_expression(*declared_type)?;

                js::TypeMember::Field { modifiers, key, ty }
            }
            dir::TypeMember::Method {
                is_optional,
                key,
                signature,
                body,
                ..
            } => {
                // default methods on nominal interfaces still need explicit JS elaboration
                if body.is_some() {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: member_id.into_global_any(self.module.id),
                        message: Some(
                            "nominal interface default methods are not lowered to JS yet"
                                .to_string(),
                        ),
                    });
                }

                let modifiers = self.build_member_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                let key = self.lower_key(*key)?;
                let signature = self.lower_function_signature(signature)?;

                js::TypeMember::Method {
                    modifiers,
                    key,
                    signature,
                }
            }
            dir::TypeMember::CallSignature { signature, .. } => {
                let modifiers = self.build_member_modifier(None, None, None, None, None, None);
                let generic_parameters =
                    self.lower_generic_parameters(&signature.generic_parameters)?;
                let this_parameter = signature
                    .this_parameter
                    .map(|parameter| self.lower_parameter(parameter))
                    .transpose()?;
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(*parameter))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let return_type = signature
                    .return_type
                    .map(|return_type| self.lower_type_annotation_expression(return_type))
                    .transpose()?;

                js::TypeMember::CallSignature {
                    modifiers,
                    signature: js::FunctionTypeDeclaration {
                        generic_parameters,
                        this_parameter,
                        parameters,
                        return_type,
                    },
                }
            }
            dir::TypeMember::ConstructSignature { signature, .. } => {
                let modifiers = self.build_member_modifier(None, None, None, None, None, None);
                let generic_parameters =
                    self.lower_generic_parameters(&signature.generic_parameters)?;
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(*parameter))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let return_type = signature
                    .return_type
                    .map(|return_type| self.lower_type_annotation_expression(return_type))
                    .transpose()?;

                js::TypeMember::ConstructSignature {
                    modifiers,
                    signature: js::ConstructorTypeDeclaration {
                        is_abstract: signature.is_abstract,
                        generic_parameters,
                        parameters,
                        return_type,
                    },
                }
            }
            dir::TypeMember::AssociatedType { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("associated type members are compile-time only".to_string()),
                });
            }
            dir::TypeMember::AssociatedConst { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some(
                        "associated comptime constants are compile-time only".to_string(),
                    ),
                });
            }
            dir::TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
                ..
            } => {
                let modifiers = self.build_member_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    None,
                    if *is_readonly {
                        Some(js::Mutability::Immutable)
                    } else {
                        None
                    },
                    None,
                    None,
                    None,
                );
                let name = self.strings.intern_from(self.source_strings, *name);
                let key_type = self.lower_type_annotation_expression(*key_type)?;
                let value_type = self.lower_type_annotation_expression(*value_type)?;

                js::TypeMember::IndexSignature {
                    modifiers,
                    name,
                    key_type,
                    value_type,
                }
            }
            dir::TypeMember::Embed { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("embedded type members are not lowered to JS".to_string()),
                });
            }
            dir::TypeMember::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("type member error slots are not lowered to JS".to_string()),
                });
            }
        };

        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(member_id)
    }

    /// Lower a member from DIR into JS AST.
    pub fn lower_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Member>> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
            dir::Member::AssociatedType { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("associated type members are compile-time only".to_string()),
                });
            }
            dir::Member::AssociatedConst { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some(
                        "associated comptime constants are compile-time only".to_string(),
                    ),
                });
            }
            dir::Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                is_readonly,
                mutability,
                visibility,
                is_static,
                is_definite,
                is_accessor,
                symbol: _,
                ..
            } => {
                let modifiers = self.build_member_modifier(
                    if *is_optional {
                        Some(js::BindingKind::Maybe)
                    } else {
                        None
                    },
                    if *is_static {
                        Some(js::BindingAnchor::Static)
                    } else {
                        None
                    },
                    if *is_readonly {
                        Some(js::Mutability::Immutable)
                    } else {
                        mutability.map(|mutability| self.lower_mutability(mutability))
                    },
                    visibility.map(|visibility| self.lower_visibility(visibility)),
                    None,
                    if *is_accessor {
                        Some(js::AccessorKind::Accessor)
                    } else {
                        None
                    },
                );
                let modifiers = modifiers.map(|mut modifiers| {
                    modifiers.definite = *is_definite;
                    modifiers
                });
                let key = self.lower_key(*key)?;
                let value = declared_type
                    .map(|value| self.lower_type_annotation_expression(value))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default)
                            .expect_node::<js::Expression>(
                                default.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;

                js::Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            }
            dir::Member::Method {
                key,
                signature,
                body,
                visibility,
                is_static,
                is_accessor,
                symbol: _,
                ..
            } => {
                let modifiers = self.build_member_modifier(
                    None,
                    if *is_static {
                        Some(js::BindingAnchor::Static)
                    } else {
                        None
                    },
                    None,
                    visibility.map(|visibility| self.lower_visibility(visibility)),
                    None,
                    if *is_accessor {
                        Some(js::AccessorKind::Accessor)
                    } else {
                        None
                    },
                );
                let key = key.map(|key| self.lower_key(key)).transpose()?;
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .map(|body_id| self.lower_expression_as_block(body_id))
                    .transpose()?;

                js::Member::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Member::Embed { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("type embedding should be expanded before codegen".to_string()),
                });
            }
            dir::Member::StaticBlock { body, .. } => {
                let body = self.lower_expression_as_block(*body)?;

                js::Member::StaticBlock { body }
            }
            dir::Member::ComptimeBlock { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("comptime blocks are compile-time only".to_string()),
                });
            }
            dir::Member::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("member error slots are not lowered to JS".to_string()),
                });
            }
        };
        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(member_id)
    }
}
