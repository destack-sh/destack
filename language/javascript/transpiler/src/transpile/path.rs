use dyst_ast::StringId;
use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::Path;
use dyst_source::SmallVec;

use crate::{TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a DIR path into a JS path.
    pub fn transpile_path(
        &self,
        _module: &'a Module,
        _scope_id: dir::NodeIdAny,
        path: &dir::Path,
        _unit: &mut TranspilerUnit,
    ) -> TranspileResult<Path> {
        let segments: SmallVec<StringId, 3> = path
            .segments
            .iter()
            .map(|segment| {
                self.session
                    .strings
                    .intern_from(&self.session.strings, *segment)
            })
            .collect();
        let path = Path { segments };
        Ok(path)
    }

    /// Render a JS path to a single string.
    pub fn render_path(&self, path: &Path, unit: &TranspilerUnit) -> String {
        // manually build a vector of &str using a loop because as_ref isn't directly usable with collect
        let mut path_str = String::new();
        for (i, segment) in path.segments.iter().enumerate() {
            let segment = unit.strings.get(*segment);
            path_str.push_str(segment.as_ref());
            if i + 1 < path.segments.len() {
                path_str.push('.');
            }
        }
        path_str
    }
}
