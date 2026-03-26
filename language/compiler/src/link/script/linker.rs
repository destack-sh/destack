use std::path::Path;

use crate::Compiler;

use destack_source::PackageId;
use destack_workspace::{Target, TargetId};

/// One script target linker.
pub(crate) struct ScriptLinker<'a> {
    /// The compiler driving the current link.
    pub(super) compiler: &'a Compiler,
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
        package_dir: &'a Path,
        root_dir: Option<&'a Path>,
        target: &'a Target,
        target_id: &'a TargetId,
        package_id: PackageId,
    ) -> Self {
        Self {
            compiler,
            package_dir,
            root_dir,
            target,
            target_id,
            package_id,
        }
    }
}
