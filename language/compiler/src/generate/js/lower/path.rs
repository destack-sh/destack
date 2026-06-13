use destack_core::StringId;
use destack_dir as dir;
use destack_js as js;
use smallvec::SmallVec;

use crate::generate::js::{CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a DIR path into a JS path.
    pub(crate) fn lower_path(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CodegenJsResult<js::Path> {
        let segments: SmallVec<[StringId; 3]> = path.segments.iter().copied().collect();
        let path = js::Path { segments };
        Ok(path)
    }
}
