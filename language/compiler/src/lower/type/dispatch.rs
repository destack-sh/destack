use std::mem;

use destack_artifact::DiagnosticLike;
use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{FunctionDeclaration, GenericInstanceKey, LifetimeParameters, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// The concrete implementer behind one erasure.
#[derive(Debug)]
pub(in crate::lower) enum Implementer {
    /// A nominal class erased at a constraint, filling slots from its members.
    Class(dir::GlobalSymbolId),
    /// A concrete object type erased at a structural constraint.
    Object {
        /// The property names the object wrote, leaving other optional slots absent.
        written: Vec<StringId>,
    },
}

impl ModuleLowerer<'_> {
    /// Declare the implementer behind one collected erasure.
    pub(in crate::lower) fn declare_implementer(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // read the constraint from the erased target
        let pointer_bytes = builder.pointer_bytes();
        let lifetimes = LifetimeParameters::default();
        let dynamic = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetimes)
            .lower(target)?;
        let mir::Type::Dynamic { constraint, .. } = *builder.tree().get(dynamic) else {
            return Err(CompilerError::Internal {
                message: "a value erased outside a dynamic target".to_string(),
            });
        };

        // register the concrete object's written property names
        if let dir::Type::Object(shape) = self.ty(source)? {
            let reference = self
                .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetimes)
                .lower(source)?;
            let mir::Type::Reference { pointee, .. } = *builder.tree().get(reference) else {
                return Err(CompilerError::Internal {
                    message: "an object class without a reference representation".to_string(),
                });
            };
            let written = self
                .types(source.module_id)?
                .properties(shape.properties)
                .iter()
                .filter_map(|property| match property.key {
                    dir::StaticKey::Name(name) => Some(name),
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.implementers
                .entry((pointee, constraint))
                .or_insert(Implementer::Object { written });

            return Ok(());
        }

        // register the declaring class's constraint entries
        let dir::Type::Application(instance) = self.ty(source)? else {
            // read the dispatch shape the constraint registered
            let Some(shape) = self.dynamic_shapes.get(&constraint) else {
                return Err(CompilerError::Internal {
                    message: "an erasure reached an unregistered constraint shape".to_string(),
                });
            };

            // erase directly when the shape has empty slots, since dispatch stays inert
            if shape.slots.is_empty() {
                return Ok(());
            }

            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "erasing a structural value".to_string(),
            }
            .into());
        };

        // lower the applied class at its concrete arguments
        let arguments = self
            .types(source.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        let concrete = self
            .type_lowerer(builder.tree_mut(), pointer_bytes, &lifetimes)
            .lower_nominal(instance.symbol, &arguments)?
            .storage;
        self.implementers
            .entry((concrete, constraint))
            .or_insert(Implementer::Class(instance.symbol));

        Ok(())
    }

    /// Build the dispatch shapes and tables over final layouts.
    pub(in crate::lower) fn build_dispatch_tables(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        let shapes = mem::take(&mut self.dynamic_shapes);
        let implementers = mem::take(&mut self.implementers);

        // derive the table each erased concrete type answers its constraint with
        for ((concrete, constraint), implementer) in implementers {
            let Some(shape) = shapes.get(&constraint) else {
                return Err(CompilerError::Internal {
                    message: "an erasure without its registered constraint shape".to_string(),
                });
            };
            let fields = builder.layouts().named_field_offsets(concrete);

            // keep unsupported implementers isolated per table
            let entries = match self.dispatch_entries(&shape.slots, &implementer, &fields) {
                Ok(entries) => entries,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    errors.push(diagnostic);

                    continue;
                }
                Err(error) => return Err(error),
            };

            // index the concrete fields by name for keyed constraints
            let mut names = Vec::new();
            if shape.is_keyed {
                names.extend(fields.iter().map(|(name, offset)| mir::DynamicNamedEntry {
                    name: *name,
                    entry: mir::DynamicEntry::Field { offset: *offset },
                }));
                names.sort_by(|left, right| {
                    self.strings
                        .get(left.name)
                        .cmp(self.strings.get(right.name))
                });
            }

            // publish the table this concrete type answers the constraint with
            builder
                .dispatch_mut()
                .insert_dynamic_table(mir::DynamicTable {
                    concrete,
                    constraint,
                    entries,
                    names,
                });
        }

        // publish the constraint shapes themselves
        for (_, shape) in shapes {
            builder.dispatch_mut().insert_dynamic_shape(shape);
        }

        Ok(())
    }

    /// Derive the dispatch entries one implementer supplies for the constraint slots.
    fn dispatch_entries(
        &self,
        slots: &[mir::DynamicSlot],
        implementer: &Implementer,
        fields: &[(StringId, u32)],
    ) -> CompilerResult<Vec<mir::DynamicEntry>> {
        let field_offset = |name: StringId| {
            fields
                .iter()
                .find(|(field, _)| *field == name)
                .map(|(_, offset)| *offset)
                .ok_or_else(|| CompilerError::Internal {
                    message: "a dispatch field without a concrete layout".to_string(),
                })
        };

        // fill one entry per constraint slot
        let mut entries = Vec::with_capacity(slots.len());
        for slot in slots {
            let entry = match (slot, implementer) {
                // read field slots at their laid-out offsets
                (mir::DynamicSlot::Field { name, .. }, Implementer::Class(_)) => {
                    mir::DynamicEntry::Field {
                        offset: field_offset(*name)?,
                    }
                }
                // read unwritten optional object slots as undefined
                (mir::DynamicSlot::Field { name, .. }, Implementer::Object { written }) => {
                    match written.contains(name) {
                        true => mir::DynamicEntry::Field {
                            offset: field_offset(*name)?,
                        },
                        false => mir::DynamicEntry::Absent,
                    }
                }
                // fill function slots from the class's implementing methods
                (
                    mir::DynamicSlot::Function {
                        name: Some(name), ..
                    },
                    Implementer::Class(symbol),
                ) => {
                    let method = self.implementing_method(*symbol, *name)?;
                    let key = GenericInstanceKey::non_generic(method);
                    let function = match self.functions.get(&key) {
                        Some(FunctionDeclaration::Declared(function)) => *function,
                        // cascade from declarations that already reported their diagnostics
                        Some(FunctionDeclaration::Failed) => {
                            return Err(LowerError::Unsupported {
                                anchor: self.module.into(),
                                construct: "a dispatch method behind a failed declaration"
                                    .to_string(),
                            }
                            .into());
                        }
                        None => {
                            return Err(CompilerError::Internal {
                                message: "a dispatch method without a declared function"
                                    .to_string(),
                            });
                        }
                    };

                    mir::DynamicEntry::Function { function }
                }
                (mir::DynamicSlot::Function { name: Some(_), .. }, Implementer::Object { .. }) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a function member on a structural constraint".to_string(),
                    }
                    .into());
                }
                (mir::DynamicSlot::Function { name: None, .. }, _) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a call-signature constraint member".to_string(),
                    }
                    .into());
                }
            };
            entries.push(entry);
        }

        Ok(entries)
    }
}
