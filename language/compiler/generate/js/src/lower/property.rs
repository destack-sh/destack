use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a binding modifier from DIR into JS AST.
    pub fn lower_binding_modifier(
        &mut self,
        modifier: dir::BindingModifier,
    ) -> CodegenJsResult<js::BindingModifier> {
        let kind = modifier.kind.map(|kind| match kind {
            dir::BindingKind::Must => js::BindingKind::Must,
            dir::BindingKind::Maybe => js::BindingKind::Maybe,
        });
        let variance = modifier.variance.map(|variance| match variance {
            dir::VarianceModifier::In => js::VarianceModifier::In,
            dir::VarianceModifier::Out => js::VarianceModifier::Out,
            dir::VarianceModifier::InOut => js::VarianceModifier::InOut,
        });
        let anchor = modifier.anchor.map(|anchor| match anchor {
            dir::BindingAnchor::Static => js::BindingAnchor::Static,
            dir::BindingAnchor::Instance => js::BindingAnchor::Instance,
        });
        let mutability = modifier
            .mutability
            .map(|mutability| self.lower_mutability(mutability));
        let visibility = modifier
            .visibility
            .map(|visibility| self.lower_visibility(visibility));
        let operator = modifier.operator.map(|operator| match operator {
            dir::BindingOperator::AsConst => js::BindingOperator::AsConst,
        });
        let accessor = modifier.accessor.map(|accessor| match accessor {
            dir::AccessorKind::Accessor => js::AccessorKind::Accessor,
        });

        Ok(js::BindingModifier {
            kind,
            variance,
            anchor,
            mutability,
            visibility,
            operator,
            accessor,
        })
    }

    /// Lower a property from DIR into JS AST.
    pub fn lower_property(
        &mut self,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Property>> {
        let property = self.dir_tree.get(property_id);
        let property = match property {
            dir::Property::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let key = key.as_ref().map(|key| self.lower_key(*key)).transpose()?;
                let value = value
                    .as_ref()
                    .map(|value| {
                        self.lower_expression(*value).expect_node::<js::Expression>(
                            value.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.lower_expression(*default)
                            .expect_node::<js::Expression>(
                                default.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;

                js::Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                }
            }
            dir::Property::Method {
                modifiers,
                key,
                signature,
                body,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let key = key.as_ref().map(|key| self.lower_key(*key)).transpose()?;
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .as_ref()
                    .map(|body_id| self.lower_expression_as_block(*body_id))
                    .transpose()?;

                js::Property::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Property::Spread {
                modifiers,
                value,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;

                js::Property::Spread { modifiers, value }
            }
            dir::Property::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: property_id.into_global_any(self.module.id),
                    message: Some("property error slots are not lowered to js".to_string()),
                });
            }
        };
        let property_id = self
            .tree
            .insert_from_source(property, self.module.id, property_id);

        Ok(property_id)
    }

    /// Lower a member from DIR into JS AST.
    pub fn lower_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Member>> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
            dir::Member::Type { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("associated type members are compile-time only".to_string()),
                });
            }
            dir::Member::ComptimeConst { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some(
                        "associated comptime constants are compile-time only".to_string(),
                    ),
                });
            }
            dir::Member::Field {
                modifiers,
                key,
                value,
                default,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let key = key.as_ref().map(|key| self.lower_key(*key)).transpose()?;
                let value = value
                    .as_ref()
                    .map(|value| {
                        self.lower_expression(*value).expect_node::<js::Expression>(
                            value.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.lower_expression(*default)
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
                modifiers,
                key,
                signature,
                body,
                symbol: _,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifier(modifiers))
                    .transpose()?;
                let key = key.as_ref().map(|key| self.lower_key(*key)).transpose()?;
                let signature = self.lower_function_signature(signature)?;
                let body = body
                    .as_ref()
                    .map(|body_id| self.lower_expression_as_block(*body_id))
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
                    message: Some("member error slots are not lowered to js".to_string()),
                });
            }
        };
        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(member_id)
    }
}
