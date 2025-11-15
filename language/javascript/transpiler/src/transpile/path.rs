use dyst_ast::StringId;
use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::Path;
use dyst_source::{SmallVec, smallvec};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a DIR path base into a JS string.
    pub fn transpile_path_base(&self, base: dir::PathBase, unit: &mut TranspilerUnit) -> StringId {
        let string = match base {
            dir::PathBase::SelfType => "self",
            dir::PathBase::SelfValue => "this",
            dir::PathBase::SuperType => "super",
            dir::PathBase::SuperValue => "super",
            dir::PathBase::Module => "module",
            dir::PathBase::Package => "package",
        };
        unit.strings.intern(string)
    }

    /// Transpile a DIR path into a JS path.
    pub fn transpile_path(
        &self,
        module: &'a Module,
        scope_id: dir::NodeIdAny,
        path: &dir::Path,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<Path> {
        let path = match path {
            dir::Path::UnresolvedBase { base } => {
                let base = self.transpile_path_base(*base, unit);
                Path {
                    segments: smallvec![base],
                }
            }
            dir::Path::UnresolvedRelativeString { base, segments } => {
                let base = self.transpile_path_base(*base, unit);
                let mut segments: SmallVec<StringId, 3> = segments
                    .iter()
                    .map(|segment| unit.strings.intern_from(&module.strings, *segment))
                    .collect();
                segments.insert(0, base);
                Path { segments }
            }
            dir::Path::UnresolvedAbsoluteString { segments } => {
                let segments = segments
                    .iter()
                    .map(|segment| unit.strings.intern_from(&module.strings, *segment))
                    .collect();
                Path { segments }
            }

            _ => {
                return Err(TranspileError::UnsupportedPath {
                    node: scope_id,
                    path: path.clone(),
                });
            }
        };

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
