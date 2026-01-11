mod itab;
mod rtti;
mod vtable;

#[allow(unused_imports)]
pub(crate) use itab::*;
#[allow(unused_imports)]
pub(crate) use rtti::*;
#[allow(unused_imports)]
pub(crate) use vtable::*;

use crate::LowerResult;

use super::module::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower dispatch tables (vtables, itabs, RTTI).
    ///
    /// This is Phase 3 of lowering. Currently stubbed.
    #[allow(dead_code)]
    pub(crate) fn lower_table(&mut self) -> LowerResult<()> {
        // TODO #Incomplete: vtables for class hierarchies
        // TODO #Incomplete: itabs for interface dispatch
        // TODO #Incomplete: RTTI descriptors and string tags

        Ok(())
    }
}
