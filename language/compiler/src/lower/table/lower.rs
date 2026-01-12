use crate::LowerResult;

use super::super::module::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower dispatch tables (vtables, itabs, RTTI).
    ///
    /// This is Phase 3 of lowering. Currently stubbed.
    #[allow(dead_code)]
    pub(crate) fn lower_tables(&mut self) -> LowerResult<()> {
        // TODO #Incomplete: vtables for class hierarchies
        // TODO #Incomplete: itabs for interface dispatch
        // TODO #Incomplete: RTTI descriptors and string tags

        Ok(())
    }
}
