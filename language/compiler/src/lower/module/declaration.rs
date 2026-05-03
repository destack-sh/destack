use crate::{CompilerError, CompilerResult, LowerError};
use destack_dir as dir;

use crate::lower::{GlobalBinding, ModuleLowerer, lower_mutability};

impl ModuleLowerer<'_> {
    /// Get the symbol for a nominal declaration.
    pub(crate) fn nominal_symbol_for_declaration_or_error(
        &self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        // require a nominal declaration for constructor lowering
        let is_nominal = matches!(
            declaration,
            dir::Declaration::Struct(_) | dir::Declaration::Class(_)
        );
        if !is_nominal {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "constructor must belong to a nominal declaration".to_string(),
            }
            .into());
        }

        // resolve the declaration symbol
        let symbol = declaration.symbol().into_global(self.module_id);

        Ok(symbol)
    }

    /// Lower a declaration into MIR.
    pub(crate) fn lower_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) -> CompilerResult<()> {
        match declaration {
            dir::Declaration::Function(declaration) => {
                self.lower_function(declaration_id, declaration)?;
                Ok(())
            }

            // struct declarations: lower the type and its methods
            dir::Declaration::Struct(declaration) => {
                let type_symbol = declaration.symbol.into_global(self.module_id);
                let members = &declaration.members;

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
                    if let dir::Member::Method { .. } = member {
                        self.lower_method(
                            *member_id,
                            member,
                            struct_mir_type,
                            type_symbol,
                            declaration_id,
                        )?;
                    }
                }

                Ok(())
            }

            // class declarations: lower the type and its methods
            dir::Declaration::Class(declaration) => {
                let type_symbol = declaration.symbol.into_global(self.module_id);
                let members = &declaration.members;

                // lower the nominal reference type for class methods
                let anchor = declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));
                let reference_type_id = self
                    .nominal_reference_type_id_for_symbol(type_symbol)
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(anchor),
                        message: "class missing nominal reference type".to_string(),
                    })
                    .map_err(CompilerError::from)?;
                let class_mir_type = Some(self.lower_type(reference_type_id, anchor)?);

                // lower methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let dir::Member::Method { .. } = member {
                        self.lower_method(
                            *member_id,
                            member,
                            class_mir_type,
                            type_symbol,
                            declaration_id,
                        )?;
                    }
                }

                Ok(())
            }

            // enum declarations: lower the backing type and its methods
            dir::Declaration::Enum(declaration) => {
                let type_symbol = declaration.symbol.into_global(self.module_id);
                let members = &declaration.members;
                let anchor = declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile));

                // lower the enum type so it's cached
                let enum_mir_type =
                    if let Some(instance_type_id) = self.types.get_instance_type_id(type_symbol) {
                        Some(self.lower_type(instance_type_id, anchor)?)
                    } else {
                        None
                    };

                // lower enum methods
                for member_id in members {
                    let member = self.dir_tree.get(*member_id);
                    if let dir::Member::Method { .. } = member {
                        self.lower_method(
                            *member_id,
                            member,
                            enum_mir_type,
                            type_symbol,
                            declaration_id,
                        )?;
                    }
                }

                Ok(())
            }

            // interface declarations are metadata only during lower
            dir::Declaration::Interface(_) => Ok(()),
            dir::Declaration::Type(_) => Ok(()),
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "unsupported declaration".to_string(),
            }
            .into()),
        }
    }

    /// Declare static member fields for every nominal module declaration.
    pub(crate) fn declare_static_member_fields(&mut self) -> CompilerResult<()> {
        // scan nominal declarations
        for (_, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>() {
            match declaration {
                dir::Declaration::Struct(declaration) => {
                    let owner_symbol = declaration.symbol.into_global(self.module_id);
                    self.lower_static_member_fields(owner_symbol, &declaration.members)?;
                }
                dir::Declaration::Class(declaration) => {
                    let owner_symbol = declaration.symbol.into_global(self.module_id);
                    self.lower_static_member_fields(owner_symbol, &declaration.members)?;
                }
                dir::Declaration::Enum(declaration) => {
                    let owner_symbol = declaration.symbol.into_global(self.module_id);
                    self.lower_static_member_fields(owner_symbol, &declaration.members)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Lower static member fields into globals.
    fn lower_static_member_fields(
        &mut self,
        owner_symbol: dir::GlobalSymbolId,
        members: &[dir::LocalNodeId<dir::Member>],
    ) -> CompilerResult<()> {
        // scan members for static fields
        for member_id in members {
            // skip static non-field members
            let member = self.dir_tree.get(*member_id);
            let dir::Member::Field {
                key,
                default,
                mutability,
                symbol,
                ..
            } = member
            else {
                continue;
            };
            if !member.is_static() {
                continue;
            }

            // require an initializer for static fields
            let Some(value_id) = default else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "static field requires initializer".to_string(),
                }
                .into());
            };

            // resolve the field type from the initializer expression
            let symbol_id = symbol.into_global(self.module_id);
            let type_id = self.declared_or_inferred_type_id_for_node_or_error(
                value_id.into_global_any(self.module_id),
            )?;
            let mir_type = self.lower_type(
                type_id,
                member_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
            )?;

            // resolve a constant initializer
            let initializer = self
                .lower_const_initializer(*value_id, mir_type)?
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        value_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "static field requires constant initializer".to_string(),
                })
                .map_err(CompilerError::from)?;

            // decide mutability from modifiers
            let mutability = mutability.unwrap_or(dir::Mutability::Immutable);
            let mir_mutability = lower_mutability(mutability);

            // resolve the global name from the static member path
            let name = self
                .static_member_name(owner_symbol, Some(*key))
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "static field is missing a stable name".to_string(),
                })
                .map_err(CompilerError::from)?;

            // create the MIR global and register the binding
            let global_id = self
                .builder
                .global(&name, mir_type, mir_mutability, initializer);
            let binding = GlobalBinding {
                global: global_id,
                ty: mir_type,
                mutability: mir_mutability,
                space: self.builder.tree().get(global_id).space.clone(),
            };
            self.insert_global_binding(symbol_id, binding)?;
        }

        Ok(())
    }
}
