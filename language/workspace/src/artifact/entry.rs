use serde::{Deserialize, Serialize};

use destack_source::{FileId, FileKey, FileVersion, ModuleVersion};

use crate::{
    Ast, DirAnalyzed, DirBase, DirDeclared, DirElaborated, DirInterface, DirPatched, DirPrepared,
    DirResolved, MirBase, MirOptimized, ModuleGraph, ModuleOutput, PackageOutput, ProfileId,
};

use super::{ArtifactImage, IntrinsicEnvironment, LanguageEnvironment, LibraryEnvironment};

/// Serialized language environment image.
pub type LanguageEnvironmentArtifactImage = ArtifactImage<LanguageEnvironment>;

/// Canonical placeholder file id used inside persisted AST images.
pub const AST_IMAGE_FILE_ID: FileId = FileId(u32::MAX);

/// Persisted AST image payload and source metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstImage {
    /// The stable source file key.
    pub file_key: FileKey,
    /// The source file version.
    pub file_version: FileVersion,
    /// Hash of the source file content.
    pub source_hash: u64,
    /// The module version used when producing the AST.
    pub module_version: ModuleVersion,
    /// The canonicalized AST payload.
    pub ast: Ast,
}

impl AstImage {
    /// Lower one live AST into a canonical persisted payload.
    pub fn from_ast(
        ast: &Ast,
        file_key: FileKey,
        file_version: FileVersion,
        source_hash: u64,
    ) -> Self {
        let mut ast = ast.clone();
        rebind_ast_file(&mut ast, AST_IMAGE_FILE_ID);

        Self {
            file_key,
            file_version,
            source_hash,
            module_version: ast.version,
            ast,
        }
    }

    /// Raise one persisted payload back into a live AST.
    pub fn into_ast(self, file_id: FileId) -> Ast {
        let mut ast = self.ast;
        rebind_ast_file(&mut ast, file_id);
        ast
    }
}

/// Rebind all AST source spans to one file id for image codec use.
fn rebind_ast_file(ast: &mut Ast, file_id: FileId) {
    // tree spans
    ast.tree.source_map.rebind_file(file_id);

    // token spans
    for token in &mut ast.tokens {
        token.span = token.span.with_file(file_id);
    }

    // side token spans
    for token in &mut ast.side_tokens {
        token.span = token.span.with_file(file_id);
    }
}

/// Serialized AST image.
pub type AstArtifactImage = ArtifactImage<AstImage>;

/// Persisted module graph image without live profile or module identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleGraphImage {
    /// Module dependencies keyed by stable file key.
    pub dependencies: indexmap::IndexMap<FileKey, indexmap::IndexSet<FileKey>>,
    /// Versions of modules included in the graph keyed by stable file key.
    pub module_versions: indexmap::IndexMap<FileKey, ModuleVersion>,
}

impl ModuleGraphImage {
    /// Lower one live module graph into a persisted payload.
    pub fn from_graph(
        graph: &ModuleGraph,
        file_key_for_module_id: impl Fn(destack_source::ModuleId) -> Option<FileKey>,
    ) -> Option<Self> {
        let mut dependencies = indexmap::IndexMap::new();
        let mut module_versions = indexmap::IndexMap::new();

        // snapshot stable dependency edges
        for (module_id, dependency_ids) in &graph.dependencies {
            let module_key = file_key_for_module_id(*module_id)?;
            let mut dependency_keys = indexmap::IndexSet::new();
            for dependency_id in dependency_ids {
                dependency_keys.insert(file_key_for_module_id(*dependency_id)?);
            }

            dependencies.insert(module_key, dependency_keys);
        }

        // snapshot stable version stamps
        for (module_id, module_version) in &graph.module_versions {
            module_versions.insert(file_key_for_module_id(*module_id)?, *module_version);
        }

        Some(Self {
            dependencies,
            module_versions,
        })
    }

    /// Raise one persisted payload back into a live module graph.
    pub fn into_graph(
        self,
        profile_id: ProfileId,
        module_id_for_file_key: impl Fn(FileKey) -> Option<destack_source::ModuleId>,
    ) -> Option<ModuleGraph> {
        let mut graph = ModuleGraph::new(profile_id);
        let mut file_keys: Vec<_> = self.module_versions.keys().copied().collect();
        file_keys.sort_unstable();

        // rebuild the graph from stable nodes
        for file_key in file_keys {
            let module_id = module_id_for_file_key(file_key)?;
            let module_version = *self.module_versions.get(&file_key)?;
            let dependency_ids = self
                .dependencies
                .get(&file_key)
                .into_iter()
                .flat_map(|dependency_keys| dependency_keys.iter())
                .map(|dependency_key| module_id_for_file_key(*dependency_key))
                .collect::<Option<Vec<_>>>()?;

            graph.update_module(module_id, module_version, dependency_ids);
        }

        Some(graph)
    }
}

/// Serialized module graph image.
pub type ModuleGraphArtifactImage = ArtifactImage<ModuleGraphImage>;

/// Serialized base DIR image.
pub type DirBaseArtifactImage = ArtifactImage<DirBase>;

/// Serialized prepared DIR image.
pub type DirPreparedArtifactImage = ArtifactImage<DirPreparedImage>;

/// Serialized resolved DIR image.
pub type DirResolvedArtifactImage = ArtifactImage<DirResolvedImage>;

/// Persisted prepared DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirPreparedImage {
    /// The canonicalized prepared DIR.
    pub dir: DirPrepared,
}

impl DirPreparedImage {
    /// Lower one live prepared DIR into a persisted payload.
    pub fn from_dir(dir: &DirPrepared) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live prepared DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirPrepared {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Persisted resolved DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirResolvedImage {
    /// The canonicalized resolved DIR.
    pub dir: DirResolved,
}

impl DirResolvedImage {
    /// Lower one live resolved DIR into a persisted payload.
    pub fn from_dir(dir: &DirResolved) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live resolved DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirResolved {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Persisted declared DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirDeclaredImage {
    /// The canonicalized declared DIR.
    pub dir: DirDeclared,
}

impl DirDeclaredImage {
    /// Lower one live declared DIR into a persisted payload.
    pub fn from_dir(dir: &DirDeclared) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live declared DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirDeclared {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Serialized declared DIR image.
pub type DirDeclaredArtifactImage = ArtifactImage<DirDeclaredImage>;

/// Persisted interface DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirInterfaceImage {
    /// The canonicalized interface DIR.
    pub dir: DirInterface,
}

impl DirInterfaceImage {
    /// Lower one live interface DIR into a persisted payload.
    pub fn from_dir(dir: &DirInterface) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live interface DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirInterface {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Serialized interface DIR image.
pub type DirInterfaceArtifactImage = ArtifactImage<DirInterfaceImage>;

/// Persisted analyzed DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirAnalyzedImage {
    /// The canonicalized analyzed DIR.
    pub dir: DirAnalyzed,
}

impl DirAnalyzedImage {
    /// Lower one live analyzed DIR into a persisted payload.
    pub fn from_dir(dir: &DirAnalyzed) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live analyzed DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirAnalyzed {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Serialized analyzed DIR image.
pub type DirAnalyzedArtifactImage = ArtifactImage<DirAnalyzedImage>;

/// Persisted elaborated DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaboratedImage {
    /// The canonicalized elaborated DIR.
    pub dir: DirElaborated,
}

impl DirElaboratedImage {
    /// Lower one live elaborated DIR into a persisted payload.
    pub fn from_dir(dir: &DirElaborated) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live elaborated DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirElaborated {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Serialized elaborated DIR image.
pub type DirElaboratedArtifactImage = ArtifactImage<DirElaboratedImage>;

/// Persisted patched DIR image without live profile identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirPatchedImage {
    /// The canonicalized patched DIR.
    pub dir: DirPatched,
}

impl DirPatchedImage {
    /// Lower one live patched DIR into a persisted payload.
    pub fn from_dir(dir: &DirPatched) -> Self {
        let mut dir = dir.clone();
        dir.profile_id = None;
        Self { dir }
    }

    /// Raise one persisted payload back into a live patched DIR.
    pub fn into_dir(self, profile_id: ProfileId) -> DirPatched {
        let mut dir = self.dir;
        dir.profile_id = Some(profile_id);
        dir
    }
}

/// Serialized patched DIR image.
pub type DirPatchedArtifactImage = ArtifactImage<DirPatchedImage>;

/// Serialized base MIR image.
pub type MirBaseArtifactImage = ArtifactImage<MirBase>;

/// Serialized optimized MIR image.
pub type MirOptimizedArtifactImage = ArtifactImage<MirOptimized>;

/// Serialized module output image.
pub type ModuleOutputArtifactImage = ArtifactImage<ModuleOutput>;

/// Serialized package output image.
pub type PackageOutputArtifactImage = ArtifactImage<PackageOutput>;

/// Serialized intrinsic environment image.
pub type IntrinsicEnvironmentArtifactImage = ArtifactImage<IntrinsicEnvironment>;

/// Serialized library environment image.
pub type LibraryEnvironmentArtifactImage = ArtifactImage<LibraryEnvironment>;

#[cfg(test)]
mod tests {
    use crate::{
        ArtifactFamily, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, EmitFormat,
        EnvSnapshot, ExportedSymbolTable, ImportedModuleTable, ModuleBindingExportTable,
        ModuleOutputKind, PackageOutputKind, Platform, ProfileFlags, ProfileKey, Runtime, TargetId,
    };
    use destack_source::{
        FileKey, FileVersion, ModuleId, ModuleVersion, PackageId, ProfileVersion,
    };

    use super::*;

    /// Build one stable profile key for artifact image tests.
    fn test_profile_key() -> ProfileKey {
        ProfileKey::new(
            EmitFormat::Js,
            Runtime::Node,
            Platform::Web,
            None,
            None,
            None,
            Vec::new(),
            false,
            false,
            false,
            EnvSnapshot::Whitelist {
                keys: Vec::new(),
                hash: 0,
            },
            ProfileFlags::default(),
        )
    }

    /// Build one stable target id for artifact image tests.
    fn test_target_id() -> TargetId {
        TargetId::new(PackageId::EPHEMERAL, "test")
    }

    /// Build one test artifact image header.
    fn test_header(family: ArtifactFamily) -> ArtifactImageHeader {
        let profile = test_profile_key();
        let image_key = match family {
            ArtifactFamily::ModuleGraph => ArtifactImageKey::ModuleGraph { profile },
            ArtifactFamily::DirBase => ArtifactImageKey::DirBase {
                module: ModuleId::EPHEMERAL,
            },
            ArtifactFamily::Ast => ArtifactImageKey::Ast {
                module: ModuleId::EPHEMERAL,
            },
            ArtifactFamily::DirPrepared => ArtifactImageKey::DirPrepared {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirResolved => ArtifactImageKey::DirResolved {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirDeclared => ArtifactImageKey::DirDeclared {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirInterface => ArtifactImageKey::DirInterface {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirAnalyzed => ArtifactImageKey::DirAnalyzed {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirElaborated => ArtifactImageKey::DirElaborated {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::DirPatched => ArtifactImageKey::DirPatched {
                module: ModuleId::EPHEMERAL,
                profile,
            },
            ArtifactFamily::MirBase => ArtifactImageKey::MirBase {
                module: ModuleId::EPHEMERAL,
                profile,
                target: test_target_id(),
            },
            ArtifactFamily::MirOptimized => ArtifactImageKey::MirOptimized {
                module: ModuleId::EPHEMERAL,
                profile,
                target: test_target_id(),
            },
            ArtifactFamily::ModuleOutput => ArtifactImageKey::ModuleOutput {
                module: ModuleId::EPHEMERAL,
                target: test_target_id(),
            },
            ArtifactFamily::PackageOutput => ArtifactImageKey::PackageOutput {
                package: PackageId::EPHEMERAL,
                target: test_target_id(),
            },
            ArtifactFamily::LanguageEnvironment => {
                ArtifactImageKey::LanguageEnvironment { profile }
            }
            ArtifactFamily::IntrinsicEnvironment => {
                ArtifactImageKey::IntrinsicEnvironment { profile }
            }
            ArtifactFamily::LibraryEnvironment => ArtifactImageKey::LibraryEnvironment { profile },
        };

        ArtifactImageHeader::new(
            image_key,
            "test".to_string(),
            if family == ArtifactFamily::Ast {
                None
            } else {
                Some(ProfileVersion::new(1))
            },
            0,
            None,
            0,
        )
    }

    /// Roundtrip one AST image through bytes.
    #[test]
    fn test_ast_image_roundtrip() {
        let image = AstArtifactImage::new(
            test_header(ArtifactFamily::Ast),
            AstImage::from_ast(
                &Ast::new(ModuleId::EPHEMERAL, ModuleVersion::INITIAL),
                FileKey::EPHEMERAL,
                FileVersion::INITIAL,
                0,
            ),
        )
        .unwrap();
        let bytes = image.serialize().expect("serialize ast image");
        let loaded = AstArtifactImage::deserialize(&bytes).expect("deserialize ast image");

        loaded
            .validate_for_family(ArtifactFamily::Ast)
            .expect("validate ast image");
        assert_eq!(loaded.header, image.header);
        assert_eq!(loaded.payload.file_key, image.payload.file_key);
        assert_eq!(loaded.payload.file_version, image.payload.file_version);
        assert_eq!(loaded.payload.source_hash, image.payload.source_hash);
        assert_eq!(loaded.payload.module_version, image.payload.module_version);
        assert_eq!(loaded.payload.ast.id, image.payload.ast.id);
        assert_eq!(loaded.payload.ast.version, image.payload.ast.version);
    }

    /// Strip and restore live profile identity for persisted prepared DIR images.
    #[test]
    fn test_dir_prepared_image_payload_roundtrip_rebinds_profile() {
        let base = DirBase::new_base(ModuleId::EPHEMERAL, ModuleVersion::INITIAL, 0);
        let dir = DirPrepared::from_base_with(
            &base,
            ProfileId::new(7),
            base.tree.as_ref().clone(),
            base.symbols.as_ref().clone(),
            Vec::new(),
            None,
            ModuleBindingExportTable::default(),
            ImportedModuleTable::default(),
            ExportedSymbolTable::default(),
        );

        let payload = DirPreparedImage::from_dir(&dir);
        assert_eq!(payload.dir.profile_id, None);

        let rebound = payload.into_dir(ProfileId::new(11));
        assert_eq!(rebound.profile_id, Some(ProfileId::new(11)));
        assert_eq!(rebound.id, dir.id);
        assert_eq!(rebound.version, dir.version);
    }

    /// Strip and restore live profile identity for persisted resolved DIR images.
    #[test]
    fn test_dir_resolved_image_payload_roundtrip_rebinds_profile() {
        let base = DirBase::new_base(ModuleId::EPHEMERAL, ModuleVersion::INITIAL, 0);
        let prepared = DirPrepared::from_base_with(
            &base,
            ProfileId::new(7),
            base.tree.as_ref().clone(),
            base.symbols.as_ref().clone(),
            Vec::new(),
            None,
            ModuleBindingExportTable::default(),
            ImportedModuleTable::default(),
            ExportedSymbolTable::default(),
        );
        let dir = DirResolved::from_prepared_with(
            &prepared,
            prepared.tree.as_ref().clone(),
            prepared.symbols.as_ref().clone(),
            None,
            Vec::new(),
            ModuleBindingExportTable::default(),
            ImportedModuleTable::default(),
            ExportedSymbolTable::default(),
        );

        let payload = DirResolvedImage::from_dir(&dir);
        assert_eq!(payload.dir.profile_id, None);

        let rebound = payload.into_dir(ProfileId::new(11));
        assert_eq!(rebound.profile_id, Some(ProfileId::new(11)));
        assert_eq!(rebound.id, dir.id);
        assert_eq!(rebound.version, dir.version);
    }

    /// Strip and restore live profile identity for persisted module graph images.
    #[test]
    fn test_module_graph_image_payload_roundtrip_rebinds_profile() {
        let graph = ModuleGraph::new(ProfileId::new(9));
        let payload = ModuleGraphImage::from_graph(&graph, |_| Some(FileKey::EPHEMERAL))
            .unwrap_or_else(|| panic!("expected stable module graph image payload"));
        let rebound = payload
            .into_graph(ProfileId::new(11), |_| Some(ModuleId::EPHEMERAL))
            .unwrap_or_else(|| panic!("expected live module graph from payload"));

        assert_eq!(rebound.profile_id, ProfileId::new(11));
        assert_eq!(rebound.dependencies, graph.dependencies);
        assert_eq!(rebound.dependents, graph.dependents);
        assert_eq!(rebound.module_versions, graph.module_versions);
    }

    /// Roundtrip one base MIR image through bytes.
    #[test]
    fn test_mir_base_image_roundtrip() {
        let image = MirBaseArtifactImage::new(
            test_header(ArtifactFamily::MirBase),
            MirBase::new(
                ModuleId::EPHEMERAL,
                ModuleVersion::INITIAL,
                test_target_id(),
            ),
        )
        .expect("build mir base image");
        let bytes = image.serialize().expect("serialize mir base image");
        let loaded = MirBaseArtifactImage::deserialize(&bytes).expect("deserialize mir base image");

        loaded
            .validate_for_family(ArtifactFamily::MirBase)
            .expect("validate mir base image");
        assert_eq!(loaded.header, image.header);
        assert_eq!(loaded.payload.id, image.payload.id);
        assert_eq!(loaded.payload.version, image.payload.version);
        assert_eq!(loaded.payload.target, image.payload.target);
    }

    /// Roundtrip one module output image through bytes.
    #[test]
    fn test_module_output_image_roundtrip() {
        let image = ModuleOutputArtifactImage::new(
            test_header(ArtifactFamily::ModuleOutput),
            ModuleOutput::new(ModuleOutputKind::JavaScript, Vec::new()),
        )
        .expect("build module output image");
        let bytes = image.serialize().expect("serialize module output image");
        let loaded = ModuleOutputArtifactImage::deserialize(&bytes)
            .expect("deserialize module output image");

        loaded
            .validate_for_family(ArtifactFamily::ModuleOutput)
            .expect("validate module output image");
        assert_eq!(loaded.header, image.header);
        assert_eq!(loaded.payload.kind, image.payload.kind);
        assert_eq!(loaded.payload.entries.len(), image.payload.entries.len());
    }

    /// Roundtrip one package output image through bytes.
    #[test]
    fn test_package_output_image_roundtrip() {
        let image = PackageOutputArtifactImage::new(
            test_header(ArtifactFamily::PackageOutput),
            PackageOutput::new(PackageOutputKind::Executable, Vec::new()),
        )
        .expect("build package output image");
        let bytes = image.serialize().expect("serialize package output image");
        let loaded = PackageOutputArtifactImage::deserialize(&bytes)
            .expect("deserialize package output image");

        loaded
            .validate_for_family(ArtifactFamily::PackageOutput)
            .expect("validate package output image");
        assert_eq!(loaded.header, image.header);
        assert_eq!(loaded.payload.kind, image.payload.kind);
        assert_eq!(loaded.payload.entries.len(), image.payload.entries.len());
    }

    /// Roundtrip one language environment image through bytes.
    #[test]
    fn test_language_environment_image_roundtrip() {
        let image = LanguageEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LanguageEnvironment),
            LanguageEnvironment::default(),
        )
        .unwrap();
        let bytes = image
            .serialize()
            .expect("serialize language environment image");
        let loaded = LanguageEnvironmentArtifactImage::deserialize(&bytes)
            .expect("deserialize language environment image");

        loaded
            .validate_for_family(ArtifactFamily::LanguageEnvironment)
            .expect("validate language environment image");
        assert_eq!(loaded.header, image.header);
        assert_eq!(loaded.payload.items, image.payload.items);
        assert_eq!(loaded.payload.symbols, image.payload.symbols);
    }

    /// Roundtrip one intrinsic environment image through bytes.
    #[test]
    fn test_intrinsic_environment_image_roundtrip() {
        let image = IntrinsicEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::IntrinsicEnvironment),
            IntrinsicEnvironment::default(),
        )
        .unwrap();
        let bytes = image
            .serialize()
            .expect("serialize intrinsic environment image");
        let loaded = IntrinsicEnvironmentArtifactImage::deserialize(&bytes)
            .expect("deserialize intrinsic environment image");

        loaded
            .validate_for_family(ArtifactFamily::IntrinsicEnvironment)
            .expect("validate intrinsic environment image");
        assert_eq!(loaded.header, image.header);
    }

    /// Roundtrip one library environment image through bytes.
    #[test]
    fn test_library_environment_image_roundtrip() {
        let image = LibraryEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LibraryEnvironment),
            LibraryEnvironment::default(),
        )
        .unwrap();
        let bytes = image
            .serialize()
            .expect("serialize library environment image");
        let loaded = LibraryEnvironmentArtifactImage::deserialize(&bytes)
            .expect("deserialize library environment image");

        loaded
            .validate_for_family(ArtifactFamily::LibraryEnvironment)
            .expect("validate library environment image");
        assert_eq!(loaded.header, image.header);
    }

    /// Reject one artifact image when the header family is wrong.
    #[test]
    fn test_artifact_image_rejects_wrong_family() {
        let mut image = LanguageEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LanguageEnvironment),
            LanguageEnvironment::default(),
        )
        .unwrap();
        image.header.artifact_image_key = ArtifactImageKey::LibraryEnvironment {
            profile: test_profile_key(),
        };

        let error = image
            .validate_for_family(ArtifactFamily::LanguageEnvironment)
            .expect_err("artifact image family mismatch should fail");
        match error {
            ArtifactImageError::InvalidArtifactFamily { expected, found } => {
                assert_eq!(expected, ArtifactFamily::LanguageEnvironment);
                assert_eq!(found, ArtifactFamily::LibraryEnvironment);
            }
            other => panic!("unexpected cache error: {other:?}"),
        }
    }

    /// Reject one artifact image when the payload hash does not match.
    #[test]
    fn test_artifact_image_rejects_payload_hash_mismatch() {
        let mut image = LanguageEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LanguageEnvironment),
            LanguageEnvironment::default(),
        )
        .unwrap();
        image.header.payload_hash ^= 1;

        let error = image
            .validate_for_family(ArtifactFamily::LanguageEnvironment)
            .expect_err("artifact image hash mismatch should fail");
        assert!(matches!(
            error,
            ArtifactImageError::InvalidPayloadHash { .. }
        ));
    }

    /// Reject truncated artifact image bytes.
    #[test]
    fn test_deserialize_artifact_image_rejects_truncated_bytes() {
        let image = LanguageEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LanguageEnvironment),
            LanguageEnvironment::default(),
        )
        .unwrap();
        let bytes = image.serialize().unwrap();
        let truncated = &bytes[..bytes.len() / 2];
        let result: Result<ArtifactImage<LanguageEnvironment>, ArtifactImageError> =
            ArtifactImage::deserialize(truncated);

        assert!(matches!(
            result,
            Err(ArtifactImageError::InvalidLayout(..)
                | ArtifactImageError::InvalidPayloadHash { .. }
                | ArtifactImageError::Deserialize(..))
        ));
    }

    /// Enforce artifact image size limits during serialization.
    #[test]
    fn test_artifact_image_enforces_size_limit() {
        let image = LanguageEnvironmentArtifactImage::new(
            test_header(ArtifactFamily::LanguageEnvironment),
            LanguageEnvironment::default(),
        )
        .unwrap();
        let result = image.serialize_with_limit(1);

        assert!(matches!(
            result,
            Err(ArtifactImageError::SizeLimitExceeded { .. })
        ));
    }
}
