use std::collections::HashMap;

use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{LowerError, LowerResult};

impl ModuleLowerer<'_> {
    /// Record layout metadata for lowered struct layouts.
    pub(crate) fn lower_layout_metadata(&mut self) -> LowerResult<()> {
        // ensure nominal layouts are lowered before metadata assignment
        let nominal_info = self.collect_nominal_types();
        for info in &nominal_info {
            if matches!(info.kind, dir::SymbolType::Struct | dir::SymbolType::Class) {
                let _ = self.lower_nominal_layout(info.symbol)?;
            }
        }

        // collect layout data to avoid borrow conflicts
        let layouts = self.type_lowerer.layout_entries();

        // record metadata entries for each layout
        for (ty, layout) in layouts {
            // collect field offsets in layout order
            let field_offsets = layout.fields.iter().map(|field| field.offset).collect();

            // build the mir layout metadata
            let type_layout = mir::TypeLayout {
                size: layout.size as u64,
                alignment: layout.alignment,
                stride: layout.size as u64,
                policy: mir::LayoutPolicy::Default,
                field_offsets,
            };

            // attach layout metadata to the type table
            let type_table = &mut self.builder.tree_mut().type_table;
            let metadata = type_table.type_metadata_by_id.entry(ty).or_default();
            metadata.layout = Some(type_layout);
        }

        Ok(())
    }

    /// Record field maps for nominal struct and class layouts.
    pub(crate) fn lower_field_map_metadata(&mut self) -> LowerResult<()> {
        // collect nominal declaration metadata for this module
        let nominal_info = self.collect_nominal_types();

        // record field maps for nominal layouts
        for info in nominal_info {
            // restrict field maps to struct and class payloads
            if !matches!(info.kind, dir::SymbolType::Struct | dir::SymbolType::Class) {
                continue;
            }

            // resolve the instance type id
            let Some(instance_type_id) = info.instance_type_id else {
                continue;
            };

            // resolve the lowered mir type
            let mir_type = self.lower_type(instance_type_id, info.anchor)?;
            let mir_type_node = self.builder.tree().get(mir_type);

            // require a struct layout for field map generation
            let mir::Type::Struct { fields, .. } = mir_type_node else {
                continue;
            };

            // resolve the cached layout for the struct
            let layout = self
                .type_lowerer
                .layout_for_type_or_error(mir_type, info.anchor)?;

            // reject layout size mismatches
            if fields.len() != layout.fields.len() {
                return Err(LowerError::UnsupportedConstruct {
                    node: info.anchor,
                    message: "field map layout size mismatch".to_string(),
                });
            }

            // build the field map from source field indices
            let mut field_map = HashMap::new();
            for (index, field) in layout.fields.iter().enumerate() {
                // skip synthetic fields
                if field.kind != crate::FieldLayoutKind::Source {
                    continue;
                }

                let field_id = fields[index];
                field_map.insert(field.name, field_id);
            }

            // record the field map metadata
            let type_table = &mut self.builder.tree_mut().type_table;
            let metadata = type_table.type_metadata_by_id.entry(mir_type).or_default();
            metadata.field_map = field_map;
        }

        Ok(())
    }
}
