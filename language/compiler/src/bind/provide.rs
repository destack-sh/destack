use destack_artifact::{ArtifactPayload, DirBound, DirParsed};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{ConditionSet, Module, ProviderContext};
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
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module);
        let profile = self.profile(context.revision(), profile);

        // bind parsed dir
        let dir_bound =
            self.bind_dir_parsed(module.as_ref(), parsed.as_ref(), &profile.conditions)?;

        Ok(ArtifactPayload::DirBound(dir_bound))
    }

    /// Bind one parsed DIR module.
    pub(crate) fn bind_dir_parsed(
        &self,
        module: &Module,
        parsed: &DirParsed,
        conditions: &ConditionSet,
    ) -> CompilerResult<DirBound> {
        let mut state = BindState::new(self, module, parsed);

        // visit code roots
        if module.is_code() {
            let tree = &parsed.tree;
            for module_file in module.files_for_conditions(conditions) {
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

        Ok(state.finish())
    }
}
