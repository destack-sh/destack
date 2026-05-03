use destack_artifact::{ArtifactKey, ArtifactPayload};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use crate::{Compiler, CompilerResult, ElaborateError};

impl Compiler {
    /// Build elaborated DIR for one checked module.
    pub(crate) fn provide_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        assert_eq!(
            artifact_key,
            context.artifact_key(),
            "compiler attempted to provide the wrong artifact"
        );

        let declared = self.dir_declared(context, module, profile)?;
        let checked = self.dir_checked(context, module, profile)?;
        let module_data = self.module(context.revision(), module);

        let mut tree = declared.tree.clone();
        let mut symbols = declared.symbols.clone();
        let mut types = checked.types.clone();

        self.elaborate_module_transform(
            &module_data,
            profile,
            context,
            &mut tree,
            &mut symbols,
            &mut types,
        )?;

        self.elaborate_module_reify(
            &module_data,
            profile,
            context,
            &mut tree,
            &mut symbols,
            &mut types,
        )?;

        Err(ElaborateError::Internal {
            anchor: module.into(),
            module,
            message: "DIR elaboration ran but has no durable patch emitter yet".to_string(),
        }
        .into())
    }
}
