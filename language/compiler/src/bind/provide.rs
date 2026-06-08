use std::iter;

use destack_artifact::{ArtifactPayload, ArtifactSidecar, DirParsed};
use destack_dir as dir;
use destack_repository::{ConditionSet, Module, ProviderContext};
use destack_source::{FileContent, ModuleId, ProfileId};
use dir::NodeVisitor as _;

use super::state::BindState;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build bound DIR for one parsed module profile.
    pub(crate) fn provide_dir_bound(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module)?;
        let profile = self.profile(context.revision(), profile)?;

        // bind parsed dir
        let mut state = BindState::new(self, module.id, parsed.as_ref());
        self.bind_roots(
            &mut state,
            module.as_ref(),
            parsed.as_ref(),
            profile.conditions(),
        )?;
        let stats = state.stats;
        let dir_bound = state.finish();
        context.emit_sidecar(ArtifactSidecar::new(
            "metadata",
            iter::once(("phase", "bind")),
            FileContent::Text {
                content: stats.render_metadata(),
            },
        ));

        Ok(ArtifactPayload::DirBound(dir_bound))
    }

    /// Bind active parsed roots into a state.
    fn bind_roots(
        &self,
        state: &mut BindState<'_>,
        module: &Module,
        parsed: &DirParsed,
        conditions: &ConditionSet,
    ) -> CompilerResult<()> {
        // visit code roots
        if module.is_code() {
            let tree = &parsed.tree;
            for module_file in module.files_for_conditions(conditions) {
                state.stats.files += 1;

                let parsed_file =
                    parsed
                        .file(module_file.file_id)
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "active module file {:?} was not parsed",
                                module_file.file_id
                            ),
                        })?;

                for root in &parsed_file.roots {
                    state.push_root(*root);
                    let expression = tree.get(*root);
                    state.visit_expression(tree, *root, expression);
                }
            }
        }

        Ok(())
    }
}
