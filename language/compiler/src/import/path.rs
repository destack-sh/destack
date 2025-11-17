use crate::Compiler;
use dyst_ast::{self as ast, StringId};
use dyst_dir::{Module, Path, PathBase};
use dyst_source::SmallVec;

impl<'a> Compiler<'a> {
    fn lower_path_base(&self, string_id: StringId) -> Option<PathBase> {
        match self.session.strings.get(string_id).as_ref() {
            "this" => Some(PathBase::SelfValue),
            "self" => Some(PathBase::SelfValue),
            "Self" => Some(PathBase::SelfType),
            "module" => Some(PathBase::Module),
            _ => None,
        }
    }

    /// Lower a path to a DIR path.
    pub(super) fn lower_path(&mut self, module: &Module, path: &ast::Path) -> Path {
        assert!(!path.segments.is_empty());
        let segments: SmallVec<StringId, 3> = path
            .segments
            .iter()
            .map(|segment| self.session.strings.intern_from(&module.strings, *segment))
            .collect();
        match self.lower_path_base(segments[0]) {
            Some(base) => {
                if segments.len() == 1 {
                    Path::UnresolvedBase { base }
                } else {
                    Path::UnresolvedRelativeString {
                        base,
                        segments: segments.into_iter().skip(1).collect(),
                    }
                }
            }
            None => Path::UnresolvedAbsoluteString { segments },
        }
    }
}
