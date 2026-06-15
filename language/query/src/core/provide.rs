use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ModuleQueryIndex, WorkspaceQueryIndex,
};
use destack_qir::QueryIndex;
use destack_repository::{
    ArtifactReader, ProviderContext, ProviderError, ProviderResult, Revision,
};
use destack_source::{ModuleId, ProfileId};
use std::sync::Arc;

use super::{
    AnnotationIndex, CallIndex, DefinitionIndex, ImportIndex, MemberIndex, ModuleQueryContext,
    Query, ReferenceEntry, ReferenceIndex, SpecifierIndex, SymbolIndex,
    require_module_query_context,
};

impl Query {
    /// Collect the dependency closure for one query artifact.
    pub fn collect(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactDependencySet> {
        match context.artifact_key() {
            ArtifactKey::ModuleQueryIndex { module, profile } => {
                Ok(self.collect_module_query_index(module, profile))
            }
            ArtifactKey::WorkspaceQueryIndex { profile } => {
                self.collect_workspace_query_index(context, profile)
            }
            artifact_key => Err(ProviderError::internal(format!(
                "non query artifact key reached query provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Collect inputs for one module query index artifact.
    fn collect_module_query_index(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ArtifactDependencySet {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module_id));
        dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_imported(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_exported(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_checked(module_id, profile_id));
        dependencies.require(ArtifactKey::global_environment(profile_id));

        dependencies
    }

    /// Collect inputs for one workspace query index artifact.
    fn collect_workspace_query_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactDependencySet> {
        let revision = context.revision();
        let module_ids = self
            .repository()
            .module_ids(revision)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        // one module query index per module carrying the profile
        let mut dependencies = ArtifactDependencySet::default();
        for module_id in module_ids {
            if !self.has_profile(revision, module_id, profile_id)? {
                continue;
            }

            dependencies.require(ArtifactKey::module_query_index(module_id, profile_id));
        }

        Ok(dependencies)
    }

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
        let artifacts = ArtifactReader::new(self.repository(), revision);

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

        Ok(ArtifactPayload::ModuleQueryIndex(Arc::new(payload)))
    }

    /// Provide one workspace query index artifact.
    fn provide_workspace_query_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let repository = self.repository();
        let revision = context.revision();

        // collect modules in the requested profile
        let module_ids = repository
            .module_ids(revision)
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        // resolve each module index version bound to this revision
        let mut versions = Vec::new();
        for module_id in module_ids {
            if !self.has_profile(revision, module_id, profile_id)? {
                continue;
            }

            let key = ArtifactKey::module_query_index(module_id, profile_id);
            let version = repository
                .artifact_version(revision, &key)
                .map_err(|error| ProviderError::internal(error.to_string()))?
                .ok_or_else(|| {
                    ProviderError::internal(format!(
                        "workspace query dependency not built: {key:?}"
                    ))
                })?;
            versions.push(version);
        }

        let payload = WorkspaceQueryIndex { modules: versions };

        Ok(ArtifactPayload::WorkspaceQueryIndex(Arc::new(payload)))
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
        let symbols = SymbolIndex::new(context.build_workspace_symbol_candidates());
        let members = MemberIndex::new(context.build_member_candidates());
        let imports = ImportIndex::new(context.build_import_candidates());

        // reference targets
        let references = context
            .build_reference_targets()
            .into_iter()
            .map(|target_symbol| ReferenceEntry {
                target_symbol,
                module_id,
            })
            .collect();
        let references = ReferenceIndex::new(references);

        // navigation relations
        let calls = CallIndex::new(context.build_call_candidates());
        let definitions = DefinitionIndex::new(
            context.build_nominal_relations(),
            context.build_extension_entries(),
        );

        // refactor targets
        let specifiers = SpecifierIndex::new(context.build_specifier_candidates());
        let annotations = AnnotationIndex::new(context.build_annotation_candidates());

        QueryIndex {
            symbols,
            members,
            imports,
            references,
            calls,
            definitions,
            specifiers,
            annotations,
        }
    }
}
