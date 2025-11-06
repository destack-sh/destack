use crate::Compiler;
use dyst_ast::{self as ast, StringId};
use dyst_dir::{Path, PathBase};
use dyst_source::{FileId, SmallVec};

impl<'a> Compiler<'a> {
    fn lower_path_base(&self, string_id: StringId) -> Option<PathBase> {
        match self.get_string(string_id) {
            "this" => Some(PathBase::SelfValue),
            "self" => Some(PathBase::SelfValue),
            "Self" => Some(PathBase::SelfType),
            "super" => Some(PathBase::SuperValue),
            "Super" => Some(PathBase::SuperType),
            "module" => Some(PathBase::Module),
            "package" => Some(PathBase::Package),
            _ => None,
        }
    }

    /// Lower a path to a DIR path.
    pub fn lower_path(&mut self, file_id: FileId, _ast: &ast::NodeTree, path: &ast::Path) -> Path {
        assert!(!path.segments.is_empty());
        let segments: SmallVec<StringId, 3> = path
            .segments
            .iter()
            .map(|segment| self.intern_string(file_id, *segment))
            .collect();
        match self.lower_path_base(segments[0]) {
            Some(base) => {
                if segments.len() == 1 {
                    Path::UnevaluatedBase { base }
                } else {
                    Path::UnevaluatedRelativeString {
                        base,
                        segments: segments.into_iter().skip(1).collect(),
                    }
                }
            }
            None => Path::UnevaluatedAbsoluteString { segments },
        }
    }
}
