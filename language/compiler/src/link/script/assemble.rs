use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::{ModuleArtifact, PackageAssembly};
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

use crate::link::assembly::ScriptTargetAssembly;

impl Compiler {
    /// Assemble one script target from generated module artifacts.
    pub(crate) fn assemble_script_target_assembly(
        &self,
        entry_modules: &[ModuleId],
        module_ids: &[ModuleId],
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<ScriptTargetAssembly> {
        // single-file and HTML targets use the dedicated target-level linker
        if target.is_single_file() {
            return self.assemble_bundled_script_target_assembly(
                entry_modules,
                package_dir,
                target,
                target_id,
                package_id,
            );
        }

        // chunked script targets still need a real chunk planner
        if self.package_assembly(target) == PackageAssembly::Chunked {
            return Err(LinkError::InvalidTarget {
                span: self.package_anchor_span(package_id),
                package: package_id,
                target: target_id.clone(),
                message: "chunked script linking is not implemented yet".to_string(),
            });
        }

        let link_plan = self.plan_script_link(entry_modules, target, target_id, package_id)?;
        let mut files = Vec::new();

        // render each generated script artifact into final target files
        for module_id in module_ids {
            let artifact = self
                .artifacts
                .module_artifact(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                })?;

            let ModuleArtifact::Script(script) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "expected script artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                });
            };

            let module = self.program.modules.get(*module_id);
            let rewritten_module = self.rewrite_script_module_for_output_paths(
                *module_id,
                script,
                target,
                package_id,
                package_dir,
                root_dir,
            )?;
            let mut rewritten_artifact = script.clone();
            rewritten_artifact.module = rewritten_module;
            let script_files = destack_codegen_js::ScriptArtifactRenderer::new(
                module.as_ref(),
                &rewritten_artifact,
                target,
                package_dir,
                root_dir,
            )
            .render_output_files()
            .map_err(|error| LinkError::Internal {
                package: package_id,
                message: format!("failed to render script artifact: {error:?}"),
            })?;
            files.extend(script_files);
        }

        Ok(ScriptTargetAssembly {
            assembly: self.package_assembly(target),
            output_files: files,
            script_link_plan: Some(link_plan),
        })
    }
}
