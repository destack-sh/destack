use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower one object property from DIR into JavaScript.
    pub(crate) fn lower_property(
        &mut self,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> Result<js::LocalNodeId<js::Property>, EmitError> {
        let property = self.dir_tree.get(property_id);

        let property = match property {
            dir::Property::Field {
                name,
                value,
                is_shorthand,
            } => js::Property::Field {
                key: js::Key::Name(self.lower_name(*name)),
                value: self.lower_expression_as::<js::Expression>(*value)?,
                is_shorthand: *is_shorthand,
            },
            dir::Property::Method {
                name,
                signature,
                body,
            } => {
                let Some(body) = body else {
                    return Err(self.unhandled(
                        property_id.into_global_any(self.module.id),
                        Some("JavaScript object methods require executable bodies".to_string()),
                    ));
                };
                let Some(name) = name else {
                    return Err(self.unhandled(
                        property_id.into_global_any(self.module.id),
                        Some("JavaScript object methods require a name".to_string()),
                    ));
                };
                let key = js::Key::Name(self.lower_name(*name));
                let role = signature
                    .role
                    .map(|role| self.lower_function_role(role))
                    .transpose()?;
                let signature = self.lower_function_signature(signature)?;
                let body = self.lower_expression_as_block(*body)?;

                js::Property::Method {
                    key,
                    role,
                    signature,
                    body,
                }
            }
            dir::Property::Spread { value } => js::Property::Spread {
                value: self.lower_expression_as::<js::Expression>(*value)?,
            },
            dir::Property::Error => {
                return Err(self.unhandled(
                    property_id.into_global_any(self.module.id),
                    Some("property error slots cannot enter JavaScript output".to_string()),
                ));
            }
        };

        Ok(self
            .tree
            .insert_from_source(property, self.module.id, property_id))
    }

    /// Lower one runtime class member from DIR into JavaScript.
    pub(crate) fn lower_member(
        &mut self,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> Result<Option<js::LocalNodeId<js::Member>>, EmitError> {
        let member = self.dir_tree.get(member_id);

        let member = match member {
            dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. }
            | dir::Member::ConstBlock { .. } => return Ok(None),
            dir::Member::Field {
                name,
                default,
                is_static,
                is_accessor,
                ..
            } => {
                let modifiers = js::MemberModifier {
                    is_static: *is_static,
                    is_accessor: *is_accessor,
                };
                let key = js::Key::Name(self.lower_name(*name));
                let default = default
                    .map(|default| self.lower_expression_as::<js::Expression>(default))
                    .transpose()?;

                js::Member::Field {
                    modifiers,
                    key,
                    default,
                }
            }
            dir::Member::Method {
                name,
                signature,
                body,
                is_static,
                is_accessor,
                ..
            } => {
                let Some(body) = body else {
                    return Ok(None);
                };
                let role = signature.role;
                let signature = self.lower_function_signature(signature)?;
                let body = self.lower_expression_as_block(*body)?;

                if role == Some(dir::FunctionRole::Constructor) {
                    js::Member::Constructor {
                        parameters: signature.parameters,
                        body,
                    }
                } else {
                    let Some(name) = name else {
                        return Err(self.unhandled(
                            member_id.into_global_any(self.module.id),
                            Some("JavaScript methods require a name".to_string()),
                        ));
                    };
                    let modifiers = js::MemberModifier {
                        is_static: *is_static,
                        is_accessor: *is_accessor,
                    };
                    let key = js::Key::Name(self.lower_name(*name));
                    let role = role
                        .map(|role| self.lower_function_role(role))
                        .transpose()?;

                    js::Member::Method {
                        modifiers,
                        key,
                        role,
                        signature,
                        body,
                    }
                }
            }
            dir::Member::StaticBlock { body } => js::Member::StaticBlock {
                body: self.lower_expression_as_block(*body)?,
            },
            dir::Member::Error => {
                return Err(self.unhandled(
                    member_id.into_global_any(self.module.id),
                    Some("member error slots cannot enter JavaScript output".to_string()),
                ));
            }
        };

        let member_id = self
            .tree
            .insert_from_source(member, self.module.id, member_id);

        Ok(Some(member_id))
    }
}
