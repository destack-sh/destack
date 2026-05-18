use destack_artifact::{ArtifactKey, ArtifactPayload, ModuleQueryIndex, WorkspaceQueryIndex};
use destack_qir::QueryIndex;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{ArtifactReader, ProviderContext, ProviderError, ProviderResult, Revision};

use crate::dir::{
    build_annotation_candidates_for_module, build_call_candidates_for_module,
    build_extension_candidates_for_module, build_import_candidates_for_module,
    build_nominal_relations_for_module, build_reference_targets_for_module,
    build_specifier_candidates_for_module, build_workspace_symbol_candidates_for_module,
};

use super::{
    AnnotationIndex, CallIndex, ExtensionIndex, ImportIndex, ModuleQueryContext, NominalIndex,
    Query, ReferenceEntry, ReferenceIndex, SpecifierIndex, SymbolIndex,
    require_module_query_context,
};

impl Query {
    /// Provide one query artifact.
    pub fn provide(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactPayload> {
        // dispatch by query artifact shape
        match context.artifact_key() {
            ArtifactKey::ModuleQueryIndex { module, profile } => {
                self.provide_module_query_index(context, module, profile)
            }
            ArtifactKey::WorkspaceQueryIndex { profile } => {
                self.provide_workspace_query_index(context, profile)
            }
            artifact_key => Err(ProviderError::internal(format!(
                "non query artifact key reached query provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Provide one module query index artifact.
    fn provide_module_query_index(
        &self,
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let revision = context.revision();
        let artifacts = ArtifactReader::new(context, self.repository().artifact_store().clone());

        // build the module index from one checked query context
        let context = require_module_query_context(
            self.repository(),
            revision,
            module_id,
            profile_id,
            &artifacts,
        )?;
        let index = self.build_module_query_index(&context);
        let payload = ModuleQueryIndex { index };

        Ok(ArtifactPayload::ModuleQueryIndex(payload))
    }

    /// Provide one workspace query index artifact.
    fn provide_workspace_query_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let repository = self.repository();
        let revision = context.revision();
        let artifacts = ArtifactReader::new(context, repository.artifact_store().clone());

        // collect modules in the requested profile
        let module_ids = repository
            .module_ids(revision)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let mut keys = Vec::new();

        // collect module index dependencies
        for module_id in module_ids {
            if !self.has_profile(revision, module_id, profile_id)? {
                continue;
            }

            keys.push(ArtifactKey::module_query_index(module_id, profile_id));
        }

        // require module indexes together so the executor can fan them out
        let versions = artifacts.require_all(&keys)?;
        // retain exact module index versions without duplicating index payloads
        let payload = WorkspaceQueryIndex { modules: versions };

        Ok(ArtifactPayload::WorkspaceQueryIndex(payload))
    }

    /// Return whether one module has the requested profile in this revision.
    fn has_profile(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<bool, Box<ProviderError>> {
        let profile = self
            .repository()
            .module_profile_by_id(revision, module_id, profile_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(profile.is_some())
    }

    /// Build one module-scoped query index.
    fn build_module_query_index(&self, context: &ModuleQueryContext<'_>) -> QueryIndex {
        let module_id = context.module_id();

        // visible symbols and importable exports
        let symbols = SymbolIndex::new(build_workspace_symbol_candidates_for_module(context));
        let imports = ImportIndex::new(build_import_candidates_for_module(context));

        // reference targets
        let references = build_reference_targets_for_module(context)
            .into_iter()
            .map(|target_symbol| ReferenceEntry {
                target_symbol,
                module_id,
            })
            .collect();
        let references = ReferenceIndex::new(references);

        // navigation relations
        let calls = CallIndex::new(build_call_candidates_for_module(context));
        let nominal = NominalIndex::new(build_nominal_relations_for_module(context));
        let extensions = ExtensionIndex::new(build_extension_candidates_for_module(context));

        // refactor targets
        let specifiers = SpecifierIndex::new(build_specifier_candidates_for_module(context));
        let annotations = AnnotationIndex::new(build_annotation_candidates_for_module(context));

        QueryIndex {
            symbols,
            imports,
            references,
            calls,
            nominal,
            extensions,
            specifiers,
            annotations,
        }
    }
}
