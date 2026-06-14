use crate::EmitError;
use destack_dir as dir;
use destack_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a property from DIR into JS AST.
    pub(crate) fn lower_property(
        &mut self,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> Result<js::LocalNodeId<js::Property>, EmitError> {
        let property = self.dir_tree.get(property_id);
        let property = match property {
            dir::Property::Field {
                key,
                value,
                is_shorthand,
            } => {
                let modifiers = None;
                let key = self.lower_key(*key)?;
                let value = self.lower_expression_as::<js::Expression>(*value)?;

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
            dir::Property::Spread { value } => {
                let modifiers = None;
                let value = self.lower_expression_as::<js::Expression>(*value)?;

                js::Property::Spread { modifiers, value }
            }
            dir::Property::Error => {
                return Err(self.unsupported_construct(
                    property_id.into_global_any(self.module.id),
                    Some("property error slots are not lowered to JS".to_string()),
                ));
            }
        };
        let property_id = self
            .tree
            .insert_from_source(property, self.module.id, property_id);

        Ok(property_id)
    }

    /// Lower a type member from DIR into JS AST.
    pub(crate) fn lower_type_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
    ) -> Result<js::LocalNodeId<js::TypeMember>, EmitError> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
            dir::TypeMember::Field {
                is_optional,
                is_readonly,
                key,
                declared_type,
                ..
            } => {
                let modifiers = (*is_optional || *is_readonly).then_some(js::BindingModifier {
                    kind: (*is_optional).then_some(js::BindingKind::Maybe),
                    mutability: (*is_readonly).then_some(js::Mutability::Immutable),
                    ..js::BindingModifier::default()
                });
                let key = self.lower_key(*key)?;
                let Some(declared_type) = declared_type else {
                    return Err(self.missing_type(member_id.into_global_any(self.module.id)));
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
                    return Err(self.unsupported_construct(
                        member_id.into_global_any(self.module.id),
                        Some(
                            "nominal interface default methods are not lowered to JS yet"
                                .to_string(),
                        ),
                    ));
                }

                let modifiers = (*is_optional).then_some(js::BindingModifier {
                    kind: Some(js::BindingKind::Maybe),
                    ..js::BindingModifier::default()
                });
                let key = self.lower_key(*key)?;
                let signature = self.lower_function_signature(signature)?;

                js::TypeMember::Method {
                    modifiers,
                    key,
                    signature,
                }
            }
            dir::TypeMember::CallSignature { signature, .. } => {
                let modifiers = None;
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
                    .collect::<Result<Vec<_>, EmitError>>()?;
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
                let modifiers = None;
                let generic_parameters =
                    self.lower_generic_parameters(&signature.generic_parameters)?;
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|parameter| self.lower_parameter(*parameter))
                    .collect::<Result<Vec<_>, EmitError>>()?;
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
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("associated type members are compile-time only".to_string()),
                ));
            }
            dir::TypeMember::AssociatedConst { .. } => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("associated comptime constants are compile-time only".to_string()),
                ));
            }
            dir::TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
                ..
            } => {
                let modifiers = (*is_optional || *is_readonly).then_some(js::BindingModifier {
                    kind: (*is_optional).then_some(js::BindingKind::Maybe),
                    mutability: (*is_readonly).then_some(js::Mutability::Immutable),
                    ..js::BindingModifier::default()
                });
                let name = *name;
                let key_type = self.lower_type_annotation_expression(*key_type)?;
                let value_type = self.lower_type_annotation_expression(*value_type)?;

                js::TypeMember::IndexSignature {
                    modifiers,
                    name,
                    key_type,
                    value_type,
                }
            }
            dir::TypeMember::Error => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("type member error slots are not lowered to JS".to_string()),
                ));
            }
        };

        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(member_id)
    }

    /// Lower a member from DIR into JS AST.
    pub(crate) fn lower_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> Result<js::LocalNodeId<js::Member>, EmitError> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
            dir::Member::AssociatedType { .. } => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("associated type members are compile-time only".to_string()),
                ));
            }
            dir::Member::AssociatedConst { .. } => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("associated comptime constants are compile-time only".to_string()),
                ));
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
                is_accessor,
                ..
            } => {
                let kind = (*is_optional).then_some(js::BindingKind::Maybe);
                let anchor = (*is_static).then_some(js::BindingAnchor::Static);
                let mutability = if *is_readonly {
                    Some(js::Mutability::Immutable)
                } else {
                    mutability.map(|mutability| self.lower_mutability(mutability))
                };
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let accessor = (*is_accessor).then_some(js::AccessorKind::Accessor);
                let modifiers = (kind.is_some()
                    || anchor.is_some()
                    || mutability.is_some()
                    || visibility.is_some()
                    || accessor.is_some())
                .then_some(js::BindingModifier {
                    kind,
                    anchor,
                    mutability,
                    visibility,
                    accessor,
                    ..js::BindingModifier::default()
                });
                let key = self.lower_key(*key)?;
                let value = declared_type
                    .map(|value| self.lower_type_annotation_expression(value))
                    .transpose()?;
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
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
                ..
            } => {
                let anchor = (*is_static).then_some(js::BindingAnchor::Static);
                let visibility = visibility.map(|visibility| self.lower_visibility(visibility));
                let accessor = (*is_accessor).then_some(js::AccessorKind::Accessor);
                let modifiers = (anchor.is_some() || visibility.is_some() || accessor.is_some())
                    .then_some(js::BindingModifier {
                        anchor,
                        visibility,
                        accessor,
                        ..js::BindingModifier::default()
                    });
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
            dir::Member::StaticBlock { body, .. } => {
                let body = self.lower_expression_as_block(*body)?;

                js::Member::StaticBlock { body }
            }
            dir::Member::ComptimeBlock { .. } => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("comptime blocks are compile-time only".to_string()),
                ));
            }
            dir::Member::Error => {
                return Err(self.unsupported_construct(
                    member_id.into_global_any(self.module.id),
                    Some("member error slots are not lowered to JS".to_string()),
                ));
            }
        };
        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(member_id)
    }
}
