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

        let candidates = match &resolution.target {
            dir::MemberTarget::Symbol(candidate) => slice::from_ref(candidate),
            dir::MemberTarget::Existential(candidates)
            | dir::MemberTarget::Universal(candidates) => candidates.as_slice(),
            dir::MemberTarget::Field(_)
            | dir::MemberTarget::Element(_)
            | dir::MemberTarget::Index(_) => return Some(Vec::new()),
        };

        let symbols = candidates
            .iter()
            .map(|candidate| candidate.symbol)
            .collect();

        Some(symbols)
    }
}
