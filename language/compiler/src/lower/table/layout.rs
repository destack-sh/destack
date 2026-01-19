use std::collections::HashMap;

use destack_base::StringId;
use destack_dir::AnchoredGlobalNodeId;
use {destack_dir as dir, destack_mir as mir};

use crate::lower::ModuleLowerer;
use crate::{FieldLayoutKind, LowerError, LowerResult};

impl ModuleLowerer<'_> {
    /// Return layout metadata for a cached struct layout.
    pub(crate) fn layout_metadata_for_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> LowerResult<Option<mir::TypeLayout>> {
        // skip if metadata already exists
        if let Some(metadata) = self.builder.tree().type_table.type_metadata_by_id.get(&ty)
            && metadata.layout.is_some()
        {
            return Ok(metadata.layout.clone());
        }

        // resolve the cached layout
        let Some(layout) = self.type_lowerer.layout_for_type(ty) else {
            return Ok(None);
        };

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
        metadata.layout = Some(type_layout.clone());

        Ok(Some(type_layout))
    }

    /// Return field map metadata for nominal struct and class layouts.
    pub(crate) fn field_map_metadata_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        mir_type: mir::LocalNodeId<mir::Type>,
        anchor: AnchoredGlobalNodeId,
    ) -> LowerResult<Option<HashMap<StringId, mir::LocalNodeId<mir::Field>>>> {
        // restrict field maps to struct and class payloads
        let Some(symbol) = self.types.symbol_for_instance_type(type_id) else {
            return Ok(None);
        };
        if !matches!(
            symbol.ty(),
            dir::SymbolType::Struct | dir::SymbolType::Class
        ) {
            return Ok(None);
        }

        // skip if metadata already exists
        if let Some(metadata) = self
            .builder
            .tree()
            .type_table
            .type_metadata_by_id
            .get(&mir_type)
            && !metadata.field_map.is_empty()
        {
            return Ok(Some(metadata.field_map.clone()));
        }

        // require a struct layout for field map generation
        let mir_type_node = self.builder.tree().get(mir_type);
        let mir::Type::Struct { fields, .. } = mir_type_node else {
            return Ok(None);
        };

        // resolve the cached layout for the struct
        let layout = self
            .type_lowerer
            .layout_for_type_or_error(mir_type, anchor)?;

        // reject layout size mismatches
        if fields.len() != layout.fields.len() {
            return Err(LowerError::UnsupportedConstruct {
                node: anchor,
                message: "field map layout size mismatch".to_string(),
            });
        }

        // build the field map from source field indices
        let mut field_map = HashMap::new();
        for (index, field) in layout.fields.iter().enumerate() {
            // skip synthetic fields
            if field.kind != FieldLayoutKind::Source {
                continue;
            }

            let field_id = fields[index];
            field_map.insert(field.name, field_id);
        }

        // record the field map metadata
        let type_table = &mut self.builder.tree_mut().type_table;
        let metadata = type_table.type_metadata_by_id.entry(mir_type).or_default();
        metadata.field_map = field_map.clone();

        Ok(Some(field_map))
    }
}
