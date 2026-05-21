use destack_workspace::ProviderContext;
use std::path::Path;

use crate::{Compiler, LinkResult};

use destack_source::{PackageId, TargetId};
use destack_workspace::Target;

/// One binary target linker.
pub(crate) struct BinaryLinker<'a> {
    /// The compiler driving the current link.
    pub(super) compiler: &'a Compiler,
    /// The pinned revision used by this link.
    pub(super) context: &'a dyn ProviderContext,
    /// The package directory that anchors output resolution.
    pub(super) package_dir: &'a Path,
    /// The configured root directory when one exists.
    pub(super) root_dir: Option<&'a Path>,
    /// The target being linked.
    pub(super) target: &'a Target,
    /// The target id being linked.
    pub(super) target_id: &'a TargetId,
    /// The target name selected by the target id.
    pub(super) target_name: String,
    /// The package owning the target.
    pub(super) package_id: PackageId,
}

impl<'a> BinaryLinker<'a> {
    /// Create one binary linker for one target.
    pub(crate) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        package_dir: &'a Path,
        root_dir: Option<&'a Path>,
        target: &'a Target,
        target_id: &'a TargetId,
        package_id: PackageId,
    ) -> LinkResult<Self> {
        let target_name = compiler
            .target_name(context.revision(), *target_id)
            .map_err(|error| Compiler::link_error(package_id, error))?;

        Ok(Self {
            compiler,
            context,
            package_dir,
            root_dir,
            target,
            target_id,
            target_name,
            package_id,
        })
    }

    /// Return the active target name.
    pub(crate) fn target_name(&self) -> &str {
        &self.target_name
    }
}
