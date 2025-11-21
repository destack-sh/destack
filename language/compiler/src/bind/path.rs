use crate::Compiler;
use dyst_ast::{self as ast, StringId};
use dyst_dir::{Module, Path, PathBase};
use dyst_source::SmallVec;

impl<'a> Compiler<'a> {
    fn bind_path_base(&self, string_id: StringId) -> Option<PathBase> {
        match self.session.strings.get(string_id).as_ref() {
            "this" => Some(PathBase::SelfValue),
            "self" => Some(PathBase::SelfValue),
            "Self" => Some(PathBase::SelfType),
            "module" => Some(PathBase::Module),
            _ => None,
        }
    }

    /// Bind a path to a DIR path.
    pub(super) fn bind_path(&self, module: &Module, path: &ast::Path) -> Path {
        assert!(!path.segments.is_empty());
        let segments: SmallVec<StringId, 3> = path
            .segments
            .iter()
            .map(|segment| self.session.strings.intern_from(&module.ast_strings, *segment))
            .collect();
        match self.bind_path_base(segments[0]) {
            Some(base) => {
                if segments.len() == 1 {
                    Path::Base { base }
                } else {
                    Path::RelativeString {
                        base,
                        segments: segments.into_iter().skip(1).collect(),
                    }
                }
            }
            None => Path::AbsoluteString { segments },
        }
    }
}
