use destack_artifact::{ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactPayload};
use destack_repository::{ProviderContext, ProviderError, ProviderResult};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect the dependency closure for one compiler owned artifact key.
    pub fn collect(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactDependencySet> {
        let artifact_key = context.artifact_key();

        self.collect_artifact(context, artifact_key)
            .map_err(|error| error.into_provider_error().into())
    }

    /// Collect the dependency closure for one compiler owned artifact key.
    fn collect_artifact(
        &self,
        context: &dyn ProviderContext,
        artifact_key: ArtifactKey,
    ) -> CompilerResult<ArtifactDependencySet> {
        match artifact_key {
            ArtifactKey::DirParsed { .. } | ArtifactKey::Data { .. } => {
                Err(CompilerError::Internal {
                    message: format!(
                        "source artifact key reached compiler provider: {artifact_key:?}"
                    ),
                })
            }
            ArtifactKey::GlobalEnvironment { profile } => {
                self.collect_global_environment(profile, context)
            }
            ArtifactKey::PackageIndex { profile } => self.collect_package_index(profile, context),
            ArtifactKey::ModuleIndex { profile } => self.collect_module_index(profile, context),
            ArtifactKey::ComponentGraph { profile } => {
                self.collect_component_graph(profile, context)
            }
            ArtifactKey::DirBound { module, profile } => {
                self.collect_dir_bound(module, profile, context)
            }
            ArtifactKey::DirImported { module, profile } => {
                self.collect_dir_imported(module, profile, context)
            }
            ArtifactKey::DirExpanded { module, profile } => {
                self.collect_dir_expanded(module, profile, context)
            }
            ArtifactKey::DirExported { module, profile } => {
                self.collect_dir_exported(module, profile, context)
            }
            ArtifactKey::DirResolved { module, profile } => {
                self.collect_dir_resolved(module, profile, context)
            }
            ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => self.collect_dir_checked_component(entry, component, profile, context),
            ArtifactKey::DirChecked { module, profile } => {
                self.collect_dir_checked(module, profile, context)
            }
            ArtifactKey::DirMaterialized { module, profile } => {
                self.collect_dir_materialized(module, profile, context)
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.collect_dir_elaborated(module, profile, context)
            }
            ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => self.collect_mir(module, profile, target, context),
            ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => self.collect_mir_verified(module, profile, target, context),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self.collect_mir_optimized(module, profile, target, context),
            ArtifactKey::ModuleOutput { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), module, &target)?;

                self.collect_module_output(module, profile, target, context)
            }
            ArtifactKey::PackageOutput { package, target } => {
                self.collect_package_output(package, target, context)
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
            ArtifactKey::PackageIndex { profile } => self.provide_package_index(profile, context),
            ArtifactKey::ModuleIndex { profile } => self.provide_module_index(profile, context),
            ArtifactKey::ComponentGraph { profile } => {
                self.provide_component_graph(profile, context)
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
            ArtifactKey::DirResolved { module, profile } => {
                self.provide_dir_resolved(module, profile, context)
            }
            ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => self.provide_dir_checked_component(entry, component, profile, context),
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
                let profile = self.profile_id_for_target(context.revision(), module, &target)?;

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
                self.emit_diagnostic(context, diagnostic)
                    .map_err(|error| Box::new(error.into_provider_error()))?;

                Err(ProviderError::failed(ArtifactFailure::diagnostics()).into())
            }
            Err(error) => Err(error.into_provider_error().into()),
        }
    }
}
