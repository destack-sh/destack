use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_core::StringPool;
use destack_engine::Program;
use destack_source::{ContentId, DiagnosticCollection};
use serde::Serialize;

use super::entry::{ArtifactEntry, ArtifactOutcome, ArtifactSidecar};
use super::pin::ArtifactPin;
use super::record::ArtifactRecord;
use crate::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactPayloadRef,
    ArtifactVersion, Asset, Build, Bundle, ComponentGraph, Data, DirBound, DirChecked,
    DirCheckedComponent, DirElaborated, DirExpanded, DirExported, DirImported, DirMaterialized,
    DirParsed, DirResolved, GlobalEnvironment, MirAnalyzed, MirLowered, MirOptimized, MirVerified,
    ModuleIndex, ModuleLinted, ModuleQueryIndex, Object, PackageIndex, PackageLinted, Product,
    ProgramAnalysis, Script, WorkspaceLinted, WorkspaceQueryIndex,
};

/// One versioned artifact family map.
type ArtifactMap<T> = DashMap<ArtifactVersion, Arc<T>>;

/// Cache of published semantic artifacts.
#[derive(Debug, Default)]
pub struct ArtifactCache {
    /// The exact artifact version entries.
    entries: DashMap<ArtifactVersion, ArtifactEntry>,
    /// The live retain count for each exact artifact version.
    retained_versions: DashMap<ArtifactVersion, usize>,

    /// DIR artifacts by module.
    dir_parsed: ArtifactMap<DirParsed>,
    /// Parsed data artifacts by module.
    data: ArtifactMap<Data>,

    /// Global environment by profile.
    global_environment: ArtifactMap<GlobalEnvironment>,
    /// Dependency indexes by profile.
    package_index: ArtifactMap<PackageIndex>,
    /// Module import edges by profile.
    module_index: ArtifactMap<ModuleIndex>,
    /// Component partitions by profile.
    component_graph: ArtifactMap<ComponentGraph>,
    /// Whole-program analysis by profile and target.
    program_analysis: ArtifactMap<ProgramAnalysis>,

    /// Bound DIR artifacts by module and profile.
    dir_bound: ArtifactMap<DirBound>,
    /// Imported DIR artifacts by module and profile.
    dir_imported: ArtifactMap<DirImported>,
    /// Expanded DIR artifacts by module and profile.
    dir_expanded: ArtifactMap<DirExpanded>,
    /// Exported DIR artifacts by module and profile.
    dir_exported: ArtifactMap<DirExported>,
    /// Resolved DIR artifacts by module and profile.
    dir_resolved: ArtifactMap<DirResolved>,
    /// Checked DIR component artifacts by component and profile.
    dir_checked_component: ArtifactMap<DirCheckedComponent>,
    /// Checked DIR facade artifacts by module and profile.
    dir_checked: ArtifactMap<DirChecked>,
    /// Materialized DIR artifacts by module and profile.
    dir_materialized: ArtifactMap<DirMaterialized>,
    /// Elaborated DIR artifacts by module and profile.
    dir_elaborated: ArtifactMap<DirElaborated>,

    /// Lowered MIR artifacts by module, profile, and target.
    mir_lowered: ArtifactMap<MirLowered>,
    /// Verified MIR markers by module, profile, and target.
    mir_verified: ArtifactMap<MirVerified>,
    /// Analyzed MIR link summaries by module, profile, and target.
    mir_analyzed: ArtifactMap<MirAnalyzed>,
    /// Optimized MIR artifacts by module, profile, and target.
    mir_optimized: ArtifactMap<MirOptimized>,

    /// Query indexes by module and profile.
    module_query_index: ArtifactMap<ModuleQueryIndex>,
    /// Query indexes by workspace and profile.
    workspace_query_index: ArtifactMap<WorkspaceQueryIndex>,

    /// Structured linker inputs by module and target.
    script: ArtifactMap<Script>,
    /// Compiled-code linker inputs by module and target.
    object: ArtifactMap<Object>,
    /// Opaque linker inputs by module and target.
    asset: ArtifactMap<Asset>,
    /// Build payloads by target.
    build: ArtifactMap<Build>,
    /// Bundles by package and target.
    bundle: ArtifactMap<Bundle>,
    /// Executable programs by package and target.
    program: ArtifactMap<Program>,
    /// Products by product id.
    product: ArtifactMap<Product>,

    /// Module lint surfaces by module and profile.
    module_linted: ArtifactMap<ModuleLinted>,
    /// Package lint surfaces by package.
    package_linted: ArtifactMap<PackageLinted>,
    /// Workspace lint surfaces.
    workspace_linted: ArtifactMap<WorkspaceLinted>,
}

impl ArtifactCache {
    /// Create a new semantic artifact cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase the live reference count for one exact artifact version.
    pub(crate) fn increase_ref_count(&self, version: &ArtifactVersion) {
        assert!(
            self.exists(version),
            "cannot increase ref count for missing artifact version: {version:?}"
        );

        let mut retain_count = self.retained_versions.entry(*version).or_insert(0);
        *retain_count += 1;
    }

    /// Retain one exact live artifact version with RAII release on drop.
    pub fn pin(self: &Arc<Self>, version: &ArtifactVersion) -> Option<ArtifactPin> {
        if !self.exists(version) {
            return None;
        }

        self.increase_ref_count(version);

        Some(ArtifactPin::new(Arc::clone(self), *version))
    }

    /// Decrease the live reference count for one exact artifact version.
    pub(crate) fn decrease_ref_count(&self, version: &ArtifactVersion) {
        let Some(mut retain_count) = self.retained_versions.get_mut(version) else {
            panic!("cannot decrease ref count for unpinned artifact version: {version:?}");
        };

        if *retain_count == 1 {
            drop(retain_count);
            self.retained_versions.remove(version);
        } else {
            *retain_count -= 1;
        }
    }

    /// Return all exact artifact versions retained by live pins.
    pub fn retained_versions(&self) -> Vec<ArtifactVersion> {
        self.retained_versions
            .iter()
            .map(|entry| *entry.key())
            .collect()
    }

    /// Retain reachable and explicitly pinned artifact versions.
    pub fn retain_reachable(&self, reachable: &HashSet<ArtifactVersion>) {
        // collect pinned versions
        let retained = self
            .retained_versions
            .iter()
            .map(|entry| *entry.key())
            .collect::<HashSet<_>>();

        // keep reachable and pinned versions
        let keep =
            |version: &ArtifactVersion| reachable.contains(version) || retained.contains(version);

        // prune each artifact family
        self.entries.retain(|version, _| keep(version));
        self.dir_parsed.retain(|version, _| keep(version));
        self.data.retain(|version, _| keep(version));
        self.global_environment.retain(|version, _| keep(version));
        self.package_index.retain(|version, _| keep(version));
        self.module_index.retain(|version, _| keep(version));
        self.component_graph.retain(|version, _| keep(version));
        self.dir_bound.retain(|version, _| keep(version));
        self.dir_imported.retain(|version, _| keep(version));
        self.dir_expanded.retain(|version, _| keep(version));
        self.dir_exported.retain(|version, _| keep(version));
        self.dir_resolved.retain(|version, _| keep(version));
        self.dir_checked_component
            .retain(|version, _| keep(version));
        self.dir_checked.retain(|version, _| keep(version));
        self.dir_materialized.retain(|version, _| keep(version));
        self.dir_elaborated.retain(|version, _| keep(version));
        self.mir_lowered.retain(|version, _| keep(version));
        self.mir_verified.retain(|version, _| keep(version));
        self.mir_optimized.retain(|version, _| keep(version));
        self.module_query_index.retain(|version, _| keep(version));
        self.workspace_query_index
            .retain(|version, _| keep(version));
        self.script.retain(|version, _| keep(version));
        self.object.retain(|version, _| keep(version));
        self.asset.retain(|version, _| keep(version));
        self.build.retain(|version, _| keep(version));
        self.bundle.retain(|version, _| keep(version));
        self.program.retain(|version, _| keep(version));
        self.product.retain(|version, _| keep(version));
        self.module_linted.retain(|version, _| keep(version));
        self.package_linted.retain(|version, _| keep(version));
        self.workspace_linted.retain(|version, _| keep(version));
    }

    /// Return the recorded diagnostics for one exact artifact version.
    pub fn diagnostics(&self, version: &ArtifactVersion) -> Option<Arc<DiagnosticCollection>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.diagnostics))
    }

    /// Return the exact dependencies for one artifact version.
    pub fn dependencies(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactDependency]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.dependencies))
    }

    /// Return the recorded sidecars for one exact artifact version.
    pub fn sidecars(&self, version: &ArtifactVersion) -> Option<Arc<[ArtifactSidecar]>> {
        self.entries
            .get(version)
            .map(|entry| Arc::clone(&entry.sidecars))
    }

    /// Return whether one exact artifact version entry exists.
    fn exists(&self, version: &ArtifactVersion) -> bool {
        self.entries.contains_key(version)
    }

    /// Return the exact terminal outcome for one artifact version.
    pub fn outcome(&self, version: &ArtifactVersion) -> Option<ArtifactOutcome> {
        self.entries.get(version).map(|entry| entry.outcome.clone())
    }

    /// Return the provider failure for one exact artifact version.
    pub fn failure(&self, version: &ArtifactVersion) -> Option<ArtifactFailure> {
        match self.outcome(version) {
            Some(ArtifactOutcome::Failed(failure)) => Some(failure),
            Some(ArtifactOutcome::Ok) | None => None,
        }
    }

    /// Return whether one exact artifact payload is published.
    pub fn has(&self, version: &ArtifactVersion) -> bool {
        match &version.key {
            ArtifactKey::GlobalEnvironment { .. } => self.global_environment.contains_key(version),
            ArtifactKey::PackageIndex { .. } => self.package_index.contains_key(version),
            ArtifactKey::ModuleIndex { .. } => self.module_index.contains_key(version),
            ArtifactKey::ComponentGraph { .. } => self.component_graph.contains_key(version),
            ArtifactKey::ProgramAnalysis { .. } => self.program_analysis.contains_key(version),
            ArtifactKey::DirParsed { .. } => self.dir_parsed.contains_key(version),
            ArtifactKey::Data { .. } => self.data.contains_key(version),
            ArtifactKey::DirBound { .. } => self.dir_bound.contains_key(version),
            ArtifactKey::DirImported { .. } => self.dir_imported.contains_key(version),
            ArtifactKey::DirExpanded { .. } => self.dir_expanded.contains_key(version),
            ArtifactKey::DirExported { .. } => self.dir_exported.contains_key(version),
            ArtifactKey::DirResolved { .. } => self.dir_resolved.contains_key(version),
            ArtifactKey::DirCheckedComponent { .. } => {
                self.dir_checked_component.contains_key(version)
            }
            ArtifactKey::DirChecked { .. } => self.dir_checked.contains_key(version),
            ArtifactKey::DirMaterialized { .. } => self.dir_materialized.contains_key(version),
            ArtifactKey::DirElaborated { .. } => self.dir_elaborated.contains_key(version),
            ArtifactKey::MirLowered { .. } => self.mir_lowered.contains_key(version),
            ArtifactKey::MirVerified { .. } => self.mir_verified.contains_key(version),
            ArtifactKey::MirAnalyzed { .. } => self.mir_analyzed.contains_key(version),
            ArtifactKey::MirOptimized { .. } => self.mir_optimized.contains_key(version),
            ArtifactKey::ModuleQueryIndex { .. } => self.module_query_index.contains_key(version),
            ArtifactKey::WorkspaceQueryIndex { .. } => {
                self.workspace_query_index.contains_key(version)
            }
            ArtifactKey::Script { .. } => self.script.contains_key(version),
            ArtifactKey::Object { .. } => self.object.contains_key(version),
            ArtifactKey::Asset { .. } => self.asset.contains_key(version),
            ArtifactKey::Build { .. } => self.build.contains_key(version),
            ArtifactKey::Bundle { .. } => self.bundle.contains_key(version),
            ArtifactKey::Program { .. } => self.program.contains_key(version),
            ArtifactKey::Product { .. } => self.product.contains_key(version),
            ArtifactKey::ModuleLinted { .. } => self.module_linted.contains_key(version),
            ArtifactKey::PackageLinted { .. } => self.package_linted.contains_key(version),
            ArtifactKey::WorkspaceLinted => self.workspace_linted.contains_key(version),
        }
    }

    /// Return one self-contained artifact record.
    pub fn record(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, crate::ArtifactBlobError> {
        let Some(dependencies) = self.dependencies(version) else {
            return Ok(None);
        };
        let Some(diagnostics) = self.diagnostics(version) else {
            return Ok(None);
        };
        let Some(sidecars) = self.sidecars(version) else {
            return Ok(None);
        };

        match &version.key {
            ArtifactKey::GlobalEnvironment { .. } => self
                .global_environment(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::GlobalEnvironment(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::PackageIndex { .. } => self
                .package_index(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::PackageIndex(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::ModuleIndex { .. } => self
                .module_index(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::ModuleIndex(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::ComponentGraph { .. } => self
                .component_graph(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::ComponentGraph(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::ProgramAnalysis { .. } => self
                .program_analysis(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::ProgramAnalysis(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirParsed { .. } => self
                .dir_parsed(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirParsed(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Data { .. } => self
                .data(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Data(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirBound { .. } => self
                .dir_bound(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirBound(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirImported { .. } => self
                .dir_imported(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirImported(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirExpanded { .. } => self
                .dir_expanded(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirExpanded(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirExported { .. } => self
                .dir_exported(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirExported(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirResolved { .. } => self
                .dir_resolved(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirResolved(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirCheckedComponent { .. } => self
                .dir_checked_component(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirCheckedComponent(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirChecked { .. } => self
                .dir_checked(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirChecked(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirMaterialized { .. } => self
                .dir_materialized(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirMaterialized(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::DirElaborated { .. } => self
                .dir_elaborated(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::DirElaborated(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::MirLowered { .. } => self
                .mir_lowered(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::MirLowered(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::MirVerified { .. } => self
                .mir_verified(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::MirVerified(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::MirAnalyzed { .. } => self
                .mir_analyzed(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::MirAnalyzed(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::MirOptimized { .. } => self
                .mir_optimized(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::MirOptimized(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::ModuleQueryIndex { .. } => self
                .module_query_index(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::ModuleQueryIndex(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::WorkspaceQueryIndex { .. } => self
                .workspace_query_index(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::WorkspaceQueryIndex(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Script { .. } => self
                .script(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Script(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Object { .. } => self
                .object(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Object(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Asset { .. } => self
                .asset(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Asset(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Build { .. } => self
                .build(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Build(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Bundle { .. } => self
                .bundle(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Bundle(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Program { .. } => self
                .program(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Program(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::Product { .. } => self
                .product(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::Product(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::ModuleLinted { .. } => self
                .module_linted(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::ModuleLinted(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::PackageLinted { .. } => self
                .package_linted(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::PackageLinted(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
            ArtifactKey::WorkspaceLinted => self
                .workspace_linted(version)
                .map(|payload| {
                    Self::record_from_payload(
                        version,
                        ArtifactPayloadRef::WorkspaceLinted(payload.as_ref()),
                        strings,
                        &dependencies,
                        &diagnostics,
                        &sidecars,
                    )
                })
                .transpose(),
        }
    }

    /// Build one artifact record from a borrowed payload.
    fn record_from_payload<T>(
        version: &ArtifactVersion,
        payload: T,
        strings: &StringPool,
        dependencies: &Arc<[ArtifactDependency]>,
        diagnostics: &Arc<DiagnosticCollection>,
        sidecars: &Arc<[ArtifactSidecar]>,
    ) -> Result<ArtifactRecord, crate::ArtifactBlobError>
    where
        T: Serialize,
    {
        let dependencies = dependencies.iter().cloned().collect();
        let diagnostics = diagnostics.as_ref().clone();
        let sidecars = sidecars.iter().cloned().collect();
        let record = ArtifactRecord::new(
            *version,
            payload,
            strings.clone(),
            dependencies,
            diagnostics,
            sidecars,
        )?;

        Ok(record)
    }

    /// Insert one typed artifact payload into its family map.
    fn insert_payload<T>(
        map: &ArtifactMap<T>,
        version: ArtifactVersion,
        payload: Arc<T>,
        is_expected_key: bool,
        payload_name: &'static str,
    ) {
        assert!(
            is_expected_key,
            "artifact payload did not match key: key={:?} payload={payload_name}",
            version.key
        );

        map.insert(version, payload);
    }

    /// Publish one ready artifact.
    pub fn publish(
        &self,
        version: ArtifactVersion,
        payload: ArtifactPayload,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) {
        match payload {
            ArtifactPayload::GlobalEnvironment(payload) => Self::insert_payload(
                &self.global_environment,
                version,
                payload,
                matches!(&version.key, ArtifactKey::GlobalEnvironment { .. }),
                "GlobalEnvironment",
            ),
            ArtifactPayload::PackageIndex(payload) => Self::insert_payload(
                &self.package_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::PackageIndex { .. }),
                "PackageIndex",
            ),
            ArtifactPayload::ModuleIndex(payload) => Self::insert_payload(
                &self.module_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleIndex { .. }),
                "ModuleIndex",
            ),
            ArtifactPayload::ComponentGraph(payload) => Self::insert_payload(
                &self.component_graph,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ComponentGraph { .. }),
                "ComponentGraph",
            ),
            ArtifactPayload::ProgramAnalysis(payload) => Self::insert_payload(
                &self.program_analysis,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ProgramAnalysis { .. }),
                "ProgramAnalysis",
            ),
            ArtifactPayload::DirParsed(payload) => Self::insert_payload(
                &self.dir_parsed,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirParsed { .. }),
                "DirParsed",
            ),
            ArtifactPayload::Data(payload) => Self::insert_payload(
                &self.data,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Data { .. }),
                "Data",
            ),
            ArtifactPayload::DirBound(payload) => Self::insert_payload(
                &self.dir_bound,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirBound { .. }),
                "DirBound",
            ),
            ArtifactPayload::DirImported(payload) => Self::insert_payload(
                &self.dir_imported,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirImported { .. }),
                "DirImported",
            ),
            ArtifactPayload::DirExpanded(payload) => Self::insert_payload(
                &self.dir_expanded,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirExpanded { .. }),
                "DirExpanded",
            ),
            ArtifactPayload::DirExported(payload) => Self::insert_payload(
                &self.dir_exported,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirExported { .. }),
                "DirExported",
            ),
            ArtifactPayload::DirResolved(payload) => Self::insert_payload(
                &self.dir_resolved,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirResolved { .. }),
                "DirResolved",
            ),
            ArtifactPayload::DirCheckedComponent(payload) => Self::insert_payload(
                &self.dir_checked_component,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirCheckedComponent { .. }),
                "DirCheckedComponent",
            ),
            ArtifactPayload::DirChecked(payload) => Self::insert_payload(
                &self.dir_checked,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirChecked { .. }),
                "DirChecked",
            ),
            ArtifactPayload::DirMaterialized(payload) => Self::insert_payload(
                &self.dir_materialized,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirMaterialized { .. }),
                "DirMaterialized",
            ),
            ArtifactPayload::DirElaborated(payload) => Self::insert_payload(
                &self.dir_elaborated,
                version,
                payload,
                matches!(&version.key, ArtifactKey::DirElaborated { .. }),
                "DirElaborated",
            ),
            ArtifactPayload::MirLowered(payload) => Self::insert_payload(
                &self.mir_lowered,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirLowered { .. }),
                "MirLowered",
            ),
            ArtifactPayload::MirVerified(payload) => Self::insert_payload(
                &self.mir_verified,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirVerified { .. }),
                "MirVerified",
            ),
            ArtifactPayload::MirAnalyzed(payload) => Self::insert_payload(
                &self.mir_analyzed,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirAnalyzed { .. }),
                "MirAnalyzed",
            ),
            ArtifactPayload::MirOptimized(payload) => Self::insert_payload(
                &self.mir_optimized,
                version,
                payload,
                matches!(&version.key, ArtifactKey::MirOptimized { .. }),
                "MirOptimized",
            ),
            ArtifactPayload::ModuleQueryIndex(payload) => Self::insert_payload(
                &self.module_query_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleQueryIndex { .. }),
                "ModuleQueryIndex",
            ),
            ArtifactPayload::WorkspaceQueryIndex(payload) => Self::insert_payload(
                &self.workspace_query_index,
                version,
                payload,
                matches!(&version.key, ArtifactKey::WorkspaceQueryIndex { .. }),
                "WorkspaceQueryIndex",
            ),
            ArtifactPayload::Script(payload) => Self::insert_payload(
                &self.script,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Script { .. }),
                "Script",
            ),
            ArtifactPayload::Object(payload) => Self::insert_payload(
                &self.object,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Object { .. }),
                "Object",
            ),
            ArtifactPayload::Asset(payload) => Self::insert_payload(
                &self.asset,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Asset { .. }),
                "Asset",
            ),
            ArtifactPayload::Build(payload) => Self::insert_payload(
                &self.build,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Build { .. }),
                "Build",
            ),
            ArtifactPayload::Bundle(payload) => Self::insert_payload(
                &self.bundle,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Bundle { .. }),
                "Bundle",
            ),
            ArtifactPayload::Program(payload) => Self::insert_payload(
                &self.program,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Program { .. }),
                "Program",
            ),
            ArtifactPayload::Product(payload) => Self::insert_payload(
                &self.product,
                version,
                payload,
                matches!(&version.key, ArtifactKey::Product { .. }),
                "Product",
            ),
            ArtifactPayload::ModuleLinted(payload) => Self::insert_payload(
                &self.module_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::ModuleLinted { .. }),
                "ModuleLinted",
            ),
            ArtifactPayload::PackageLinted(payload) => Self::insert_payload(
                &self.package_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::PackageLinted { .. }),
                "PackageLinted",
            ),
            ArtifactPayload::WorkspaceLinted(payload) => Self::insert_payload(
                &self.workspace_linted,
                version,
                payload,
                matches!(&version.key, ArtifactKey::WorkspaceLinted),
                "WorkspaceLinted",
            ),
        }

        self.entries.insert(
            version,
            ArtifactEntry::ok(dependencies, diagnostics, sidecars),
        );
    }

    /// Fail one artifact.
    pub fn fail(
        &self,
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) {
        self.entries.insert(
            version,
            ArtifactEntry::failed(dependencies, diagnostics, sidecars, failure),
        );
    }
}

impl ArtifactCache {
    /// Get one global environment artifact.
    pub fn global_environment(&self, version: &ArtifactVersion) -> Option<Arc<GlobalEnvironment>> {
        self.global_environment
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one dependency index artifact.
    pub fn package_index(&self, version: &ArtifactVersion) -> Option<Arc<PackageIndex>> {
        self.package_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module index artifact.
    pub fn module_index(&self, version: &ArtifactVersion) -> Option<Arc<ModuleIndex>> {
        self.module_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one component graph artifact.
    pub fn component_graph(&self, version: &ArtifactVersion) -> Option<Arc<ComponentGraph>> {
        self.component_graph
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one whole-program analysis artifact.
    pub fn program_analysis(&self, version: &ArtifactVersion) -> Option<Arc<ProgramAnalysis>> {
        self.program_analysis
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one DIR artifact.
    pub fn dir_parsed(&self, version: &ArtifactVersion) -> Option<Arc<DirParsed>> {
        self.dir_parsed
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one data artifact.
    pub fn data(&self, version: &ArtifactVersion) -> Option<Arc<Data>> {
        self.data.get(version).map(|entry| entry.value().clone())
    }

    /// Get one bound DIR artifact.
    pub fn dir_bound(&self, version: &ArtifactVersion) -> Option<Arc<DirBound>> {
        self.dir_bound
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one imported DIR artifact.
    pub fn dir_imported(&self, version: &ArtifactVersion) -> Option<Arc<DirImported>> {
        self.dir_imported
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one expanded DIR artifact.
    pub fn dir_expanded(&self, version: &ArtifactVersion) -> Option<Arc<DirExpanded>> {
        self.dir_expanded
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one exported DIR artifact.
    pub fn dir_exported(&self, version: &ArtifactVersion) -> Option<Arc<DirExported>> {
        self.dir_exported
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one resolved DIR artifact.
    pub fn dir_resolved(&self, version: &ArtifactVersion) -> Option<Arc<DirResolved>> {
        self.dir_resolved
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one checked DIR component artifact.
    pub fn dir_checked_component(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<DirCheckedComponent>> {
        self.dir_checked_component
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one checked DIR facade artifact.
    pub fn dir_checked(&self, version: &ArtifactVersion) -> Option<Arc<DirChecked>> {
        self.dir_checked
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one materialized DIR artifact.
    pub fn dir_materialized(&self, version: &ArtifactVersion) -> Option<Arc<DirMaterialized>> {
        self.dir_materialized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one elaborated DIR artifact.
    pub fn dir_elaborated(&self, version: &ArtifactVersion) -> Option<Arc<DirElaborated>> {
        self.dir_elaborated
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one lowered MIR artifact.
    pub fn mir_lowered(&self, version: &ArtifactVersion) -> Option<Arc<MirLowered>> {
        self.mir_lowered
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one verified MIR marker.
    pub fn mir_verified(&self, version: &ArtifactVersion) -> Option<Arc<MirVerified>> {
        self.mir_verified
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one analyzed MIR link summary.
    pub fn mir_analyzed(&self, version: &ArtifactVersion) -> Option<Arc<MirAnalyzed>> {
        self.mir_analyzed
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one optimized MIR artifact.
    pub fn mir_optimized(&self, version: &ArtifactVersion) -> Option<Arc<MirOptimized>> {
        self.mir_optimized
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one module query index artifact.
    pub fn module_query_index(&self, version: &ArtifactVersion) -> Option<Arc<ModuleQueryIndex>> {
        self.module_query_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one workspace query index artifact.
    pub fn workspace_query_index(
        &self,
        version: &ArtifactVersion,
    ) -> Option<Arc<WorkspaceQueryIndex>> {
        self.workspace_query_index
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one structured script artifact.
    pub fn script(&self, version: &ArtifactVersion) -> Option<Arc<Script>> {
        self.script.get(version).map(|entry| entry.value().clone())
    }

    /// Get one compiled-code object artifact.
    pub fn object(&self, version: &ArtifactVersion) -> Option<Arc<Object>> {
        self.object.get(version).map(|entry| entry.value().clone())
    }

    /// Get one asset artifact.
    pub fn asset(&self, version: &ArtifactVersion) -> Option<Arc<Asset>> {
        self.asset.get(version).map(|entry| entry.value().clone())
    }

    /// Get one build payload.
    pub fn build(&self, version: &ArtifactVersion) -> Option<Arc<Build>> {
        self.build.get(version).map(|entry| entry.value().clone())
    }

    /// Get one bundle artifact.
    pub fn bundle(&self, version: &ArtifactVersion) -> Option<Arc<Bundle>> {
        self.bundle.get(version).map(|entry| entry.value().clone())
    }

    /// Get one program artifact.
    pub fn program(&self, version: &ArtifactVersion) -> Option<Arc<Program>> {
        self.program.get(version).map(|entry| entry.value().clone())
    }

    /// Get one product artifact.
    pub fn product(&self, version: &ArtifactVersion) -> Option<Arc<Product>> {
        self.product.get(version).map(|entry| entry.value().clone())
    }

    /// Return content ids referenced by one exact artifact payload.
    pub fn content_ids(&self, version: &ArtifactVersion) -> Vec<ContentId> {
        match &version.key {
            ArtifactKey::Object { .. } => {
                if let Some(payload) = self.object(version) {
                    return payload.content_ids();
                }
            }
            ArtifactKey::Asset { .. } => {
                if let Some(payload) = self.asset(version) {
                    return payload.content_ids();
                }
            }
            ArtifactKey::Build { .. } => {
                if let Some(payload) = self.build(version) {
                    return payload.content_ids();
                }
            }
            ArtifactKey::Bundle { .. } => {
                if let Some(payload) = self.bundle(version) {
                    return payload.content_ids();
                }
            }
            ArtifactKey::Program { .. } => {
                if let Some(payload) = self.program(version) {
                    return payload.content_ids();
                }
            }
            ArtifactKey::Product { .. } => {
                if let Some(payload) = self.product(version) {
                    return payload.content_ids();
                }
            }
            _ => {}
        }

        Vec::new()
    }

    /// Get one module lint artifact.
    pub fn module_linted(&self, version: &ArtifactVersion) -> Option<Arc<ModuleLinted>> {
        self.module_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one package lint artifact.
    pub fn package_linted(&self, version: &ArtifactVersion) -> Option<Arc<PackageLinted>> {
        self.package_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }

    /// Get one workspace lint artifact.
    pub fn workspace_linted(&self, version: &ArtifactVersion) -> Option<Arc<WorkspaceLinted>> {
        self.workspace_linted
            .get(version)
            .map(|entry| entry.value().clone())
    }
}
