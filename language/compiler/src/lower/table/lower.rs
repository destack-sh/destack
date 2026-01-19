use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower dispatch tables (vtables, itabs, RTTI).
    pub(crate) fn lower_tables(&mut self) -> LowerResult<()> {
        // record layout metadata for lowered types
        self.lower_layout_metadata()?;

        // record lineage metadata for nominal types
        self.lower_lineage_metadata()?;

        // record field maps for nominal layouts
        self.lower_field_map_metadata()?;

        // assign deterministic metadata names for nominal types
        self.assign_nominal_metadata_names()?;

        // assign deterministic metadata names for anonymous types
        self.assign_anonymous_metadata_names()?;

        // ensure every mir type has a metadata name
        self.ensure_metadata_names_assigned()?;

        // generate class vtables
        self.lower_vtables()?;

        // generate interface itabs
        self.lower_itabs()?;

        // TODO #Incomplete: RTTI descriptors and string tags (TypeId<->TypeTag)

        Ok(())
    }
}
