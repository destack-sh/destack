use crate::{
    AccessorKind, BindingAnchor, BindingKind, BindingModifier, BindingOperator, CodegenJsError,
    CodegenJsResult, CodegenJsResultExt, Expression, LocalNodeId, Member, ModuleLowerer, Property,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a binding modifier from DIR into JS AST.
    pub fn lower_binding_modifier(
        &mut self,
        modifier: dir::BindingModifier,
    ) -> CodegenJsResult<BindingModifier> {
        let kind = modifier.kind.map(|kind| match kind {
            dir::BindingKind::Must => BindingKind::Must,
            dir::BindingKind::Maybe => BindingKind::Maybe,
        });
        let anchor = modifier.anchor.map(|anchor| match anchor {
            dir::BindingAnchor::Static => BindingAnchor::Static,
            dir::BindingAnchor::Instance => BindingAnchor::Instance,
        });
        let mutability = modifier
            .mutability
            .map(|mutability| self.lower_mutability(mutability));
        let visibility = modifier
            .visibility
            .map(|visibility| self.lower_visibility(visibility));
        let operator = modifier.operator.map(|operator| match operator {
            dir::BindingOperator::AsConst => BindingOperator::AsConst,
        });
        let accessor = modifier.accessor.map(|accessor| match accessor {
            dir::AccessorKind::Accessor => AccessorKind::Accessor,
        });
        Ok(BindingModifier {
            kind,
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
    ) -> CodegenJsResult<LocalNodeId<Property>> {
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
                        self.lower_expression(*value)
                            .expect_node::<Expression>(value.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.lower_expression(*default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Property::Field {
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
                    .map(|body_id| {
                        self.lower_expression(*body_id).expect_node::<Expression>(
                            body_id.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Property::Method {
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
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                Property::Spread { modifiers, value }
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
    ) -> CodegenJsResult<LocalNodeId<Member>> {
        let member = self.dir_tree.get(member_id);
        let member = match member {
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
                        self.lower_expression(*value)
                            .expect_node::<Expression>(value.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                let default = default
                    .as_ref()
                    .map(|default| {
                        self.lower_expression(*default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Member::Field {
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
                    .map(|body_id| {
                        self.lower_expression(*body_id).expect_node::<Expression>(
                            body_id.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                Member::Method {
                    modifiers,
                    key,
                    signature,
                    body,
                }
            }
            dir::Member::Embed { .. } => {
                // Embed is a compile-time construct, shouldn't reach codegen
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: member_id.into_global_any(self.module.id),
                    message: Some("type embedding should be expanded before codegen".to_string()),
                });
            }
            dir::Member::StaticBlock { body, symbol: _ } => {
                let body = self
                    .lower_expression(*body)
                    .expect_node::<Expression>(body.into_global_any(self.module.id), self)?;
                Member::StaticBlock { body }
            }
        };
        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);
        Ok(member_id)
    }
}
