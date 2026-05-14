use destack_artifact::{ArtifactKey, ArtifactPayload, DirBound, DirParsed};
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Module, ProviderContext};
use dir::NodeVisitor as _;

use super::state::BindState;

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Build bound DIR for one parsed module profile.
    pub(crate) fn provide_dir_bound(
        &self,
        module: ModuleId,
        _profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // require parsed source tree
        context
            .require(ArtifactKey::dir_parsed(module))
            .map_err(CompilerError::from)?;

        // load provider inputs
        let parsed = self
            .dir_parsed(context, module)
            .map_err(CompilerError::from)?;
        let module = self.module(context.revision(), module);

        // bind parsed dir
        let dir_bound = self.bind_dir_parsed(module.as_ref(), parsed.as_ref());

        Ok(ArtifactPayload::DirBound(dir_bound))
    }

    /// Bind one parsed DIR module.
    pub(in crate::bind) fn bind_dir_parsed(&self, module: &Module, parsed: &DirParsed) -> DirBound {
        let mut state = BindState::new(self, module, parsed);

        // visit code roots
        if module.is_code() {
            let tree = &parsed.tree;
            for root in &parsed.roots {
                state.push_root(*root);
                let expression = tree.get(*root);
                state.visit_expression(tree, *root, expression);
            }
        }

        state.finish()
    }
}
