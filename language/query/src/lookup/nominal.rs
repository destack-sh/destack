use std::slice;

use destack_dir as dir;

use crate::ModuleQueryContext;

impl ModuleQueryContext<'_> {
    /// Return every symbol target from one recorded member resolution.
    pub(crate) fn member_symbol_targets(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> Option<Vec<dir::GlobalSymbolId>> {
        let resolution = self.resolutions().member_resolution(node_id)?;

        let accesses = match resolution {
            dir::OperationResolution::One(access) => slice::from_ref(access),
            dir::OperationResolution::Union { arms, .. } => arms.as_slice(),
        };

        let mut symbols = Vec::new();
        for access in accesses {
            access.target.collect_symbols(&mut symbols);
        }

        Some(symbols)
    }
}
