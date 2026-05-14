use destack_artifact::{ArtifactFailure, ArtifactKey, ArtifactPayload, DiagnosticBuilder};
use destack_workspace::{ProviderContext, ProviderError, ProviderResult};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Provide one compiler owned artifact key through a session-owned attempt context.
    pub fn provide(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactPayload> {
        let artifact_key = context.artifact_key();
        let result = self.provide_artifact(context, artifact_key);

        self.finish_provide_result(context, result)
    }

    /// Provide one compiler owned artifact key.
    fn provide_artifact(
        &self,
        context: &dyn ProviderContext,
        artifact_key: ArtifactKey,
    ) -> CompilerResult<ArtifactPayload> {
        match artifact_key {
            ArtifactKey::DirParsed { .. } | ArtifactKey::Data { .. } => {
                Err(CompilerError::Internal {
                    message: format!(
                        "source artifact key reached compiler provider: {artifact_key:?}"
                    ),
                })
            }
            ArtifactKey::GlobalEnvironment { profile } => {
                self.provide_global_environment(profile, context)
            }
            ArtifactKey::DirBound { module, profile } => {
                self.provide_dir_bound(module, profile, context)
            }
            ArtifactKey::DirImported { module, profile } => {
                self.provide_dir_imported(module, profile, context)
            }
            ArtifactKey::DirExpanded { module, profile } => {
                self.provide_dir_expanded(module, profile, context)
            }
            ArtifactKey::DirExported { module, profile } => {
                self.provide_dir_exported(module, profile, context)
            }
            ArtifactKey::DirChecked { module, profile } => {
                self.provide_dir_checked(module, profile, context)
            }
            ArtifactKey::DirMaterialized { module, profile } => {
                self.provide_dir_materialized(module, profile, context)
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.provide_dir_elaborated(module, profile, context)
            }
            ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => self.provide_mir(module, profile, target, context),
            ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => self.provide_mir_verified(module, profile, target, context),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self.provide_mir_optimized(module, profile, target, context),
            ArtifactKey::ModuleOutput { module, target } => {
                let Some(profile) = self.target_profile_id(context.revision(), module, &target)
                else {
                    return Err(CompilerError::Internal {
                        message: format!("missing profile for module {module:?} target {target:?}"),
                    });
                };

                self.provide_module_output(module, profile, target, context)
            }
            ArtifactKey::PackageOutput { package, target } => {
                self.provide_package_output(package, target, context)
            }
            ArtifactKey::ModuleLinted { .. }
            | ArtifactKey::PackageLinted { .. }
            | ArtifactKey::WorkspaceLinted
            | ArtifactKey::ModuleQueryIndex { .. }
            | ArtifactKey::WorkspaceQueryIndex { .. } => Err(CompilerError::Internal {
                message: format!(
                    "non compiler artifact key reached compiler provider: {artifact_key:?}"
                ),
            }),
        }
    }

    /// Finish one direct provide attempt.
    fn finish_provide_result(
        &self,
        context: &dyn ProviderContext,
        result: CompilerResult<ArtifactPayload>,
    ) -> ProviderResult<ArtifactPayload> {
        match result {
            Ok(payload) => Ok(payload),
            Err(CompilerError::Diagnostic(diagnostic)) => {
                self.emit_built_diagnostic(context, DiagnosticBuilder::new(diagnostic))
                    .map_err(|error| Box::new(error.into_provider_error()))?;

                Err(ProviderError::failed(ArtifactFailure::diagnostics()).into())
            }
            Err(error) => Err(error.into_provider_error().into()),
        }
    }
}
