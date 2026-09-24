use destack_artifact::{ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactPayload};
use destack_repository::{ProviderContext, ProviderError, ProviderResult};

use crate::sema::provide::Pass;
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
            ArtifactKey::EnvironmentBound { profile } => {
                self.collect_environment_bound(profile, context)
            }
            ArtifactKey::EnvironmentDeclared { profile } => {
                self.collect_environment_declared(profile, context)
            }
            ArtifactKey::ModuleGraph { package, profile } => {
                self.collect_module_graph(package, profile, context)
            }
            ArtifactKey::ProgramAnalysis { profile, target } => {
                self.collect_program_analysis(profile, target, context)
            }
            ArtifactKey::DirBound { module, profile } => {
                self.collect_dir_bound(module, profile, context)
            }
            ArtifactKey::DirImported { module, profile } => {
                self.collect_dir_imported(module, profile)
            }
            ArtifactKey::DirExpanded { module, profile } => {
                self.collect_dir_expanded(module, profile, context)
            }
            ArtifactKey::DirExported { module, profile } => {
                self.collect_dir_exported(module, profile, context)
            }
            ArtifactKey::DirResolved { module, profile } => {
                self.collect_dir_resolved(module, profile)
            }
            ArtifactKey::DirDeclared { module, profile } => {
                self.collect_dir_stage(module, profile, Pass::Declare)
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.collect_dir_stage(module, profile, Pass::Elaborate)
            }
            ArtifactKey::DirChecked { module, profile } => {
                self.collect_dir_stage(module, profile, Pass::Check)
            }
            ArtifactKey::DirMaterialized { module, profile } => {
                self.collect_dir_stage(module, profile, Pass::Materialize)
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                self.collect_dir_stage(module, profile, Pass::Analyze)
            }
            ArtifactKey::MirDeclared {
                module,
                profile,
                target,
            } => self.collect_mir_declared(module, profile, target, context),
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
            ArtifactKey::MirInstantiated {
                module,
                profile,
                target,
            } => self.collect_mir_instantiated(module, profile, target, context),
            ArtifactKey::MirElaborated {
                module,
                profile,
                target,
            } => self.collect_mir_elaborated(module, profile, target),
            ArtifactKey::MirAnalyzed {
                module,
                profile,
                target,
            } => self.collect_mir_analyzed(module, profile, target, context),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self.collect_mir_optimized(module, profile, target),
            ArtifactKey::Script { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.collect_script(module, profile, target, context)
            }
            ArtifactKey::Object { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.collect_object(module, profile, target, context)
            }
            ArtifactKey::Asset { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.collect_asset(module, profile, target, context)
            }
            ArtifactKey::Bundle { package, target } => {
                self.collect_bundle(package, target, context)
            }
            ArtifactKey::Program { package, target } => {
                self.collect_program(package, target, context)
            }
            ArtifactKey::Product { package, product } => {
                self.collect_product(package, product, context)
            }
            _ => Err(CompilerError::Internal {
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
            ArtifactKey::EnvironmentBound { profile } => {
                self.provide_environment_bound(profile, context)
            }
            ArtifactKey::EnvironmentDeclared { profile } => {
                self.provide_environment_declared(profile, context)
            }
            ArtifactKey::ModuleGraph { package, profile } => {
                self.provide_module_graph(package, profile, context)
            }
            ArtifactKey::ProgramAnalysis { profile, target } => {
                self.provide_program_analysis(profile, target, context)
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
            ArtifactKey::DirDeclared { module, profile } => {
                self.provide_dir_stage(module, profile, context, Pass::Declare)
            }
            ArtifactKey::DirElaborated { module, profile } => {
                self.provide_dir_stage(module, profile, context, Pass::Elaborate)
            }
            ArtifactKey::DirChecked { module, profile } => {
                self.provide_dir_stage(module, profile, context, Pass::Check)
            }
            ArtifactKey::DirMaterialized { module, profile } => {
                self.provide_dir_stage(module, profile, context, Pass::Materialize)
            }
            ArtifactKey::DirAnalyzed { module, profile } => {
                self.provide_dir_stage(module, profile, context, Pass::Analyze)
            }
            ArtifactKey::MirDeclared {
                module,
                profile,
                target,
            } => self.provide_mir_declared(module, profile, target, context),
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
            ArtifactKey::MirInstantiated {
                module,
                profile,
                target,
            } => self.provide_mir_instantiated(module, profile, target, context),
            ArtifactKey::MirElaborated {
                module,
                profile,
                target,
            } => self.provide_mir_elaborated(module, profile, target, context),
            ArtifactKey::MirAnalyzed {
                module,
                profile,
                target,
            } => self.provide_mir_analyzed(module, profile, target, context),
            ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => self.provide_mir_optimized(module, profile, target, context),
            ArtifactKey::Script { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.provide_script(module, profile, target, context)
            }
            ArtifactKey::Object { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.provide_object(module, profile, target, context)
            }
            ArtifactKey::Asset { module, target } => {
                let profile = self.profile_id_for_target(context.revision(), &target)?;

                self.provide_asset(module, profile, target, context)
            }
            ArtifactKey::Bundle { package, target } => {
                self.provide_bundle(package, target, context)
            }
            ArtifactKey::Program { package, target } => {
                self.provide_program(package, target, context)
            }
            ArtifactKey::Product { package, product } => {
                self.provide_product(package, product, context)
            }
            _ => Err(CompilerError::Internal {
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
