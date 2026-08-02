use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one interface declaration to its dynamic constraint row.
    ///
    /// Interface storage erases to a dynamic whose shape dispatches the
    /// declared members entry by entry.
    pub(in crate::lower) fn lower_interface(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::InterfaceDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // an inherited row would have to merge two dispatch shapes
        if !definition.extends.is_empty() {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "an inheriting interface constraint".to_string(),
            }
            .into());
        }

        // lower each property into a field node and dispatch slot
        let fields = self.lowerer.instance_fields(&definition.members);
        let mut field_nodes = Vec::with_capacity(fields.len());
        let mut slots = Vec::with_capacity(definition.members.len());
        for field in &fields {
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
            field_nodes.push(node);
            slots.push(mir::DynamicSlot::Field { field: node, name });
        }

        // lower each method into a function slot at its bare signature
        for member in &definition.members {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            let declared = self.lowerer.symbol_type(method.symbol)?;
            let reduced = self.lowerer.reduced_type(declared)?;
            let dir::Type::FunctionSignature(signature) = self.lowerer.ty(reduced)? else {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "an interface method without a plain signature".to_string(),
                }
                .into());
            };

            // a method with its own type parameters has no single slot signature
            let signature = *self.lowerer.types(reduced.module_id)?.signature(signature);
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

            let signature = self.lower_signature_row(&signature, reduced.module_id)?;
            let name = self.lowerer.symbol_name(method.symbol)?;
            slots.push(mir::DynamicSlot::Function { name, signature });
        }

        // hold the property storage in the constraint row
        self.tree.define_type(
            ty,
            mir::Type::Struct {
                fields: field_nodes,
                copy: mir::Copy::No,
            },
        );

        // register the constraint's dispatch shape once
        self.lowerer
            .dynamic_shapes
            .entry(ty)
            .or_insert(mir::DynamicShape {
                constraint: ty,
                slots,
            });

        Ok(fields)
    }
}
