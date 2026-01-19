use destack_dir::{Declaration, LocalNodeId, Member};

use crate::{LowerError, LowerResult};

use crate::lower::module::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a declaration into MIR.
    pub(crate) fn lower_declaration(
        &mut self,
        declaration_id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<()> {
        match declaration {
            Declaration::Function { .. } => {
                self.lower_function(declaration_id, declaration)?;
                Ok(())
            }

            // struct declarations: lower the type and its methods
            Declaration::Struct {
                descriptor,
                members,
                ..
            } => {
                let type_symbol = descriptor.symbol.into_global(self.module_id);

                // lower the struct type so it's cached
                let struct_mir_type =
                    if let Some(instance_type_id) = self.types.get_instance_type_id(type_symbol) {
                        Some(
                            self.lower_type(
                                instance_type_id,
                                declaration_id
                                    .into_global_any(self.module_id)
                                    .into_anchored(Some(self.profile)),
                            )?,
                        )
                    } else {
                        None
                    };

                // lower methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let Member::Method { .. } = member {
                        self.lower_method(*member_id, member, struct_mir_type, declaration_id)?;
                    }
                }

                Ok(())
            }

            // class declarations: lower the type and its methods
            Declaration::Class {
                descriptor,
                members,
                ..
            } => {
                let type_symbol = descriptor.symbol.into_global(self.module_id);

                // lower the nominal reference type for class methods
                let anchor = declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));
                let reference_type_id = self
                    .nominal_reference_type_id_for_symbol(type_symbol)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node: anchor,
                        message: "class missing nominal reference type".to_string(),
                    })?;
                let class_mir_type = Some(self.lower_type(reference_type_id, anchor)?);

                // lower methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let Member::Method { .. } = member {
                        self.lower_method(*member_id, member, class_mir_type, declaration_id)?;
                    }
                }

                Ok(())
            }
            // interface declarations are metadata only during lower
            Declaration::Interface { .. } => Ok(()),
            _ => Err(LowerError::UnsupportedConstruct {
                node: declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "unsupported declaration".to_string(),
            }),
        }
    }
}
