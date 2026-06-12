use destack_core::StringId;
use destack_dir as dir;
use destack_js as js;
use smallvec::SmallVec;

use crate::generate::js::{CodegenJsResult, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower a DIR path into a JS path.
    pub fn lower_path(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CodegenJsResult<js::Path> {
        let segments: SmallVec<[StringId; 3]> =
            path.segments.iter().map(|segment| *segment).collect();
        let path = js::Path { segments };
        Ok(path)
    }

    /// Render a JS path to a single string.
    pub fn render_path(&self, path: &js::Path) -> String {
        let mut path_str = String::new();
        for (i, segment) in path.segments.iter().enumerate() {
            let segment = self.strings.get(*segment);
            path_str.push_str(segment.as_ref());
            if i + 1 < path.segments.len() {
                path_str.push('.');
            }
        }
        path_str
    }
}
