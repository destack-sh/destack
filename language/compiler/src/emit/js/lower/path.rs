use crate::EmitError;
use smallvec::SmallVec;
use tspp_core::StringId;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower a DIR path into a JS path.
    pub(crate) fn lower_path(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Result<js::Path, EmitError> {
        let segments: SmallVec<[StringId; 3]> = path.segments.iter().copied().collect();
        let path = js::Path { segments };
        Ok(path)
    }
}
