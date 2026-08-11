use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer, TypeSubstitution};
use crate::{CompilerResult, LowerError};

/// One named member of a flattened interface.
enum InterfaceMember {
    /// A stored property field.
    Field {
        /// The lowered field node.
        node: mir::LocalNodeId<mir::Field>,
        /// The field identity for member indexing.
        field: NominalField,
    },
    /// A dispatched method slot.
    Method {
        /// The dispatch slot.
        slot: mir::DynamicSlot,
    },
}

impl TypeLowerer<'_, '_> {
    /// Lower one interface declaration to its dynamic constraint type.
    pub(in crate::lower) fn lower_interface(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::InterfaceDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // flatten the interface and its bases into one member list by name
        let mut entries = FxIndexMap::default();
        self.collect_interface_members(symbol, &definition, &mut entries)?;

        // split the flattened members into storage fields and dispatch slots
        let mut fields = Vec::new();
        let mut field_nodes = Vec::new();
        let mut slots = Vec::with_capacity(entries.len());
        for (name, entry) in entries {
            match entry {
                InterfaceMember::Field { node, field } => {
                    fields.push(field);
                    field_nodes.push(node);
                    slots.push(mir::DynamicSlot::Field { field: node, name });
                }
                InterfaceMember::Method { slot } => slots.push(slot),
            }
        }

        // hold the property storage in the constraint struct
        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields: field_nodes,
                copy: mir::Copy::No,
            },
        );

        // register the constraint's unkeyed dispatch shape once
        self.lowerer
            .dynamic_shapes
            .entry(ty)
            .or_insert(mir::DynamicShape {
                constraint: ty,
                slots,
                is_keyed: false,
            });

        Ok(fields)
    }

    /// Collect one interface's members into a flattened list keyed by name.
    fn collect_interface_members(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: &dir::InterfaceDefinition,
        entries: &mut FxIndexMap<StringId, InterfaceMember>,
    ) -> CompilerResult<()> {
        // flatten inherited members first so derived members override in place
        for heritage in &definition.extends {
            let base = heritage.ty;
            let dir::Type::Application(application) = self.lowerer.ty(base)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an interface inheriting a non-nominal target".to_string(),
                }
                .into());
            };
            let base_definition = match self.lowerer.definition(application.symbol)? {
                Some(dir::Definition::Interface(base_definition)) => base_definition.clone(),
                _ => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "an interface inheriting a non-interface target".to_string(),
                    }
                    .into());
                }
            };

            // bind the base template at the applied heritage arguments
            let arguments = self
                .lowerer
                .types(base.module_id)?
                .type_ids(application.arguments)
                .to_vec();
            let substitution = match base_definition.template {
                Some(template) => TypeSubstitution::bind(
                    self.lowerer,
                    template.into_global(application.symbol.module_id),
                    &arguments,
                    self.type_substitution,
                )?,
                None => self.type_substitution.clone(),
            };

            // flatten the base members under its bound arguments
            self.with_substitution(&substitution)
                .collect_interface_members(application.symbol, &base_definition, entries)?;
        }

        // lower each property into a field node and dispatch slot
        let fields = self.lowerer.instance_fields(&definition.members);
        for field in fields {
            let declared = self
                .lowerer
                .symbol_type(field.symbol.into_global(symbol.module_id))?;
            let declared = self.lower(declared)?;
            let dir::StaticKey::Name(name) = field.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a computed interface member".to_string(),
                }
                .into());
            };
            let node = self.tree.intern_field(
                mir::Field {
                    name: Some(name),
                    ty: declared,
                },
                Vec::new(),
            );
            entries.insert(name, InterfaceMember::Field { node, field });
        }

        // lower each method into a function slot at its bare signature
        for member in &definition.members {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            let declared = self.lowerer.symbol_type(method.symbol)?;
            let dir::Type::FunctionSignature(signature) = self.lowerer.ty(declared)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an interface method without a plain signature".to_string(),
                }
                .into());
            };

            // reject a method with its own type parameters, which has no single slot
            let signature = *self.lowerer.types(declared.module_id)?.signature(signature);
            if let Some(template) = signature.template {
                let generics = &self.lowerer.state(template.module_id)?.generics;
                let is_generic = generics
                    .get_template(template.local_id)
                    .parameters
                    .iter()
                    .any(|parameter| {
                        generics.get_parameter(*parameter).kind == dir::GenericParameterKind::Type
                    });
                if is_generic {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a generic constraint method".to_string(),
                    }
                    .into());
                }
            }

            // leave the receiver to the dispatch and lower the bare signature
            let signature = self.lower_bare_signature(&signature, declared.module_id)?;
            let Some(name) = self.lowerer.symbol_name(method.symbol)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an anonymous interface method".to_string(),
                }
                .into());
            };
            entries.insert(
                name,
                InterfaceMember::Method {
                    slot: mir::DynamicSlot::Function {
                        name: Some(name),
                        signature: mir::TypeId::from(signature),
                    },
                },
            );
        }

        Ok(())
    }
}
