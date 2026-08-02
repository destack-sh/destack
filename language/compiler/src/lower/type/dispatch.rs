use std::mem;

use destack_artifact::DiagnosticLike;
use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{GenericInstanceKey, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// One erasure fact backing a dynamic dispatch table.
#[derive(Debug)]
pub(in crate::lower) enum DynamicSource {
    /// A nominal class erased at a constraint, filling slots from its members.
    Class(dir::GlobalSymbolId),
    /// A concrete object row erased at a structural contract, with its written keys.
    Object(Vec<StringId>),
}

impl ModuleLowerer<'_> {
    /// Publish the registered dispatch shapes and tables over final layouts.
    pub(in crate::lower) fn publish_dispatch(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        let shapes = mem::take(&mut self.dynamic_shapes);
        let sources = mem::take(&mut self.dynamic_sources);

        // derive the table each erased concrete row answers its constraint with
        for ((concrete, constraint), source) in sources {
            let Some(shape) = shapes.get(&constraint) else {
                return Err(CompilerError::Internal {
                    message: "an erasure without its registered constraint shape".to_string(),
                });
            };
            let fields = Self::laid_out_fields(builder, concrete);

            // keep unsupported erasures isolated per table
            let entries = match self.dispatch_entries(&shape.slots, &source, &fields) {
                Ok(entries) => entries,
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    errors.push(diagnostic);

                    continue;
                }
                Err(error) => return Err(error),
            };

            // index the concrete fields by name for keyed constraints
            let mut names = Vec::new();
            if self.keyed_constraints.contains(&constraint) {
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

    /// Derive the dispatch entries one source supplies for the constraint slots.
    fn dispatch_entries(
        &self,
        slots: &[mir::DynamicSlot],
        source: &DynamicSource,
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

        let mut entries = Vec::with_capacity(slots.len());
        for slot in slots {
            let entry = match (slot, source) {
                // read field slots at their laid-out offsets
                (mir::DynamicSlot::Field { name, .. }, DynamicSource::Class(_)) => {
                    mir::DynamicEntry::Field {
                        offset: field_offset(*name)?,
                    }
                }
                // read unwritten optional row slots as undefined
                (mir::DynamicSlot::Field { name, .. }, DynamicSource::Object(written)) => {
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
                    DynamicSource::Class(symbol),
                ) => {
                    let method = self.implementing_method(*symbol, *name)?;
                    let key = GenericInstanceKey::non_generic(method);
                    let function = self.functions.get(&key).copied().ok_or_else(|| {
                        CompilerError::Internal {
                            message: "a dispatch method without a declared function".to_string(),
                        }
                    })?;

                    mir::DynamicEntry::Function { function }
                }
                (mir::DynamicSlot::Function { name: Some(_), .. }, DynamicSource::Object(_)) => {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a function member on a structural contract".to_string(),
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

    /// Return the named field offsets of one laid-out concrete row.
    fn laid_out_fields(
        builder: &mir::ModuleBuilder,
        concrete: mir::LocalNodeId<mir::Type>,
    ) -> Vec<(StringId, u32)> {
        let layout = builder
            .layouts()
            .layout_id(concrete)
            .map(|id| builder.layouts().layout(id));
        let mut fields = Vec::new();
        if let Some(mir::LayoutShape::Struct(layout)) = layout.map(|layout| &layout.shape) {
            fields.extend(
                layout
                    .fields
                    .iter()
                    .filter_map(|field| field.name.map(|name| (name, field.offset))),
            );
        }

        fields
    }
}
