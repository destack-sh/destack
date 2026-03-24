use std::path::Path;

use crate::{Compiler, LinkResult};

use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use crate::link::assembly::BinaryTargetAssembly;
use crate::link::binary::BinaryLinkPlan;

impl Compiler {
    /// Assemble one wasm binary target.
    pub(crate) fn assemble_wasm_target_assembly(
        &self,
        _entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<BinaryTargetAssembly> {
        let files = self.render_binary_target_files(
            module_ids,
            package_dir,
            root_dir,
            target,
            target_id,
            package_id,
        )?;

        Ok(BinaryTargetAssembly {
            assembly: self.package_assembly(target),
            output_files: files,
            binary_link_plan: Some(BinaryLinkPlan),
        })
    }
}
