use std::path::Path;

use crate::{Compiler, CompilerContext};

use destack_source::{ModuleId, PackageId, Span, TargetId};
use destack_workspace::Target;

/// One script target linker.
pub(crate) struct ScriptLinker<'a> {
    /// The compiler driving the current link.
    pub(super) compiler: &'a Compiler,
    /// The pinned revision used by this link.
    pub(super) context: &'a CompilerContext<'a>,
    /// The package directory that anchors output resolution.
    pub(super) package_dir: &'a Path,
    /// The configured root directory when one exists.
    pub(super) root_dir: Option<&'a Path>,
    /// The target being linked.
    pub(super) target: &'a Target,
    /// The target id being linked.
    pub(super) target_id: &'a TargetId,
    /// The package owning the target.
    pub(super) package_id: PackageId,
}

impl<'a> ScriptLinker<'a> {
    /// Create one script linker for one target.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a CompilerContext<'a>,
        package_dir: &'a Path,
        root_dir: Option<&'a Path>,
        target: &'a Target,
        target_id: &'a TargetId,
        package_id: PackageId,
    ) -> Self {
        Self {
            compiler,
            context,
            package_dir,
            root_dir,
            target,
            target_id,
            package_id,
        }
    }

    /// Return the display name for the active target.
    pub(crate) fn target_name(&self) -> String {
        self.compiler.target_name(self.target_id)
    }

    /// Return the anchor span for one linked module.
    pub(crate) fn module_anchor_span(&self, module_id: ModuleId) -> Span {
        let module = self.context.module(module_id);

        Span::empty(module.file_id)
    }
}
