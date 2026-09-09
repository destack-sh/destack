use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_mir::substitute_type;

use crate::lower::{GenericScope, NominalField, TypeLowerer};
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
    /// Lower one interface declaration to its constraint storage and dispatch shape.
    pub(in crate::lower) fn lower_interface(
        &mut self,
        definition: dir::InterfaceDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        let (fields, field_nodes, slots) = self.interface_shape(&definition)?;

        // define the constraint storage from its field nodes
        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields: field_nodes,
                copy: mir::Copy::No,
            },
        );
        self.register_interface_shape(ty, slots);

        Ok(fields)
    }

    /// Return one interface's stored fields, their nodes, and its dispatch slots.
    pub(in crate::lower) fn interface_shape(
        &mut self,
        definition: &dir::InterfaceDefinition,
    ) -> CompilerResult<(
        Vec<NominalField>,
        Vec<mir::LocalNodeId<mir::Field>>,
        Vec<mir::DynamicSlot>,
    )> {
        // flatten the interface and its bases into one member list by name
        let mut entries = FxIndexMap::default();
        self.collect_interface_members(definition, &mut entries)?;

        // split the flattened members into storage fields and dispatch slots
        let mut fields = Vec::new();
        let mut field_nodes = Vec::new();
        let mut slots = Vec::with_capacity(entries.len());
        for (name, entry) in entries {
            match entry {
                // store a property and dispatch it through its own slot
                InterfaceMember::Field { node, field } => {
                    fields.push(field);
                    field_nodes.push(node);
                    slots.push(mir::DynamicSlot::Field { field: node, name });
                }
                // dispatch a method through its slot alone
                InterfaceMember::Method { slot } => slots.push(slot),
            }
        }

        Ok((fields, field_nodes, slots))
    }

    /// Register one constraint's unkeyed dispatch shape once.
    pub(in crate::lower) fn register_interface_shape(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        slots: Vec<mir::DynamicSlot>,
    ) {
        self.lower
            .dynamic_shapes
            .entry(ty)
            .or_insert(mir::DynamicShape {
                constraint: ty,
                slots,
                is_keyed: false,
            });
    }

    /// Collect one interface's members into a flattened list keyed by name.
    fn collect_interface_members(
        &mut self,
        definition: &dir::InterfaceDefinition,
        entries: &mut FxIndexMap<StringId, InterfaceMember>,
    ) -> CompilerResult<()> {
        // flatten the inherited members ahead of the derived ones
        for heritage in &definition.extends {
            let base = heritage.ty;
            let dir::Type::Application(application) = self.lower.ty(base)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "an interface inheriting a non-nominal target".to_string(),
                }
                .into());
            };
            let base_definition = match self.lower.definition(application.symbol)? {
                Some(dir::Definition::Interface(base_definition)) => base_definition.clone(),
                _ => {
                    return Err(LowerError::Unsupported {
                        anchor: self.lower.module.into(),
                        construct: "an interface inheriting a non-interface target".to_string(),
                    }
                    .into());
                }
            };

            // lower the base members under the base's own parameters, then apply the arguments
            let arguments = self
                .lower
                .types(base.module_id)?
                .type_ids(application.arguments)
                .to_vec();
            let mut lowered = Vec::with_capacity(arguments.len());
            for argument in &arguments {
                lowered.push(self.lower_generic_argument(*argument)?);
            }
            let template = base_definition
                .template
                .map(|template| template.into_global(application.symbol.module_id));
            let base_parameters = GenericScope::from_templates(self.lower, None, template)?;
            let mut base_entries = FxIndexMap::default();
            self.under(&base_parameters)
                .collect_interface_members(&base_definition, &mut base_entries)?;
            for (name, entry) in base_entries {
                let entry = match entry {
                    InterfaceMember::Field { node, field } => {
                        let ty = substitute_type(self.tree, self.tree.get(node).ty, &lowered);
                        let node = self.tree.intern_field(
                            mir::Field {
                                name: Some(name),
                                ty,
                            },
                            Vec::new(),
                        );

                        InterfaceMember::Field { node, field }
                    }
                    InterfaceMember::Method { slot } => InterfaceMember::Method {
                        slot: match slot {
                            mir::DynamicSlot::Function { name, signature } => {
                                mir::DynamicSlot::Function {
                                    name,
                                    signature: substitute_type(self.tree, signature, &lowered),
                                }
                            }
                            other => other,
                        },
                    },
                };
                entries.insert(name, entry);
            }
        }

        // lower each property into a field node and dispatch slot
        let fields = self.lower.instance_fields(&definition.members)?;
        for field in fields {
            // lower the declared property type and read its written name
            let declared = self.lower.symbol_type(field.symbol)?;
            let declared = self.lower(declared)?;
            let dir::StaticKey::Name(name) = field.key else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a computed interface member".to_string(),
                }
                .into());
            };

            // let a derived property override the inherited entry in place
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

            let declared = self.lower.symbol_type(method.symbol)?;
            let dir::Type::FunctionSignature(signature) = self.lower.ty(declared)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "an interface method without a plain signature".to_string(),
                }
                .into());
            };

            // skip methods declaring their own type parameters
            let signature = *self.lower.types(declared.module_id)?.signature(signature);
            let template = signature.template.or_else(|| {
                self.lower
                    .state(method.symbol.module_id)
                    .ok()?
                    .generics
                    .template_by_symbol(method.symbol)
                    .map(|template| template.into_global(method.symbol.module_id))
            });
            if let Some(template) = template {
                let generics = &self.lower.state(template.module_id)?.generics;
                let is_generic = generics
                    .get_template(template.local_id)
                    .parameters
                    .iter()
                    .any(|parameter| {
                        let binding = generics.get_parameter(*parameter);

                        binding.kind == dir::GenericParameterKind::Type && binding.is_writable()
                    });
                if is_generic {
                    continue;
                }
            }

            // lower the bare signature under the method's own template
            let signature = self.lower_bare_signature(
                &signature,
                declared.module_id,
                template,
                Some(method.symbol),
            )?;

            // key the slot by the name the method declares
            let Some(name) = self.lower.symbol_name(method.symbol)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
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
