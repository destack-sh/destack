use crate::{CodegenJsResult, ModuleLowerer, Path};
use destack_ast::StringId;
use destack_dir as dir;
use smallvec::SmallVec;

impl ModuleLowerer<'_> {
    /// Lower a DIR path into a JS path.
    pub fn lower_path(
        &mut self,
        _scope_id: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> CodegenJsResult<Path> {
        let segments: SmallVec<[StringId; 3]> = path
            .segments
            .iter()
            .map(|segment| self.strings.intern_from(&self.module.ast.strings, *segment))
            .collect();
        let path = Path { segments };
        Ok(path)
    }

    /// Render a JS path to a single string.
    pub fn render_path(&self, path: &Path) -> String {
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
