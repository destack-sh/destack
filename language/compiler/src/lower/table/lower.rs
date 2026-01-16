use destack_mir as mir;

use crate::LowerResult;

use super::super::module::ModuleLowerer;

#[allow(dead_code)]
impl ModuleLowerer<'_> {
    /// Lower dispatch tables (vtables, itabs, RTTI).
    pub(crate) fn lower_tables(&mut self) -> LowerResult<()> {
        // record layout metadata for lowered types
        self.lower_layout_metadata()?;

        // generate class vtables
        self.lower_vtables()?;

        // generate interface itabs
        self.lower_itabs()?;

        // TODO #Incomplete: RTTI descriptors and string tags

        Ok(())
    }

    /// Record layout metadata for lowered struct layouts.
    fn lower_layout_metadata(&mut self) -> LowerResult<()> {
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
}
