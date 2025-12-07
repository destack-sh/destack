use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use destack_source::{FileContent, FileType, ModuleId, PackageId, Uri};

/// The id of an Artifact.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtifactId(pub u32);

impl std::fmt::Debug for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ArtifactId {
    /// Create a new ArtifactId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Scope of an artifact: module-level or package-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactScope {
    /// Artifact for a single module (e.g., generated JS, object code).
    Module(ModuleId),
    /// Artifact for an entire package (e.g., linked binary, bundle).
    Package(PackageId),
}

impl ArtifactScope {
    /// Get the module id if this is a module-scoped artifact.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Module(id) => Some(*id),
            Self::Package(_) => None,
        }
    }

    /// Get the package id if this is a package-scoped artifact.
    pub fn package(&self) -> Option<PackageId> {
        match self {
            Self::Module(_) => None,
            Self::Package(id) => Some(*id),
        }
    }
}

/// Generated artifact from code generation.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// The artifact id.
    pub id: ArtifactId,
    /// The scope (module or package).
    pub scope: ArtifactScope,
    /// The target name (e.g., "npm", "wasm").
    pub target: String,
    /// The output URI (where this artifact would be written).
    pub uri: Uri,
    /// The generated content.
    pub content: ArtifactContent,
    /// Related source artifact (e.g., source map references its JS artifact).
    pub source: Option<ArtifactId>,
}

/// Content of a generated artifact.
#[derive(Debug, Clone)]
pub enum ArtifactContent {
    /// Text-based artifact (JS, TS, .d.ts, etc.).
    Text {
        /// The generated text content.
        code: String,
        /// The file type (determines extension).
        file_type: FileType,
    },
    /// JSON artifact (source maps, etc.).
    Json {
        /// The JSON content as string.
        content: String,
        /// The parsed JSON value.
        value: serde_json::Value,
        /// The file type (determines extension).
        file_type: FileType,
    },
    /// Binary artifact (wasm, object code, linked binary).
    Binary {
        /// The generated binary content.
        bytes: Vec<u8>,
        /// The file type (determines extension).
        file_type: FileType,
    },
}

impl ArtifactContent {
    /// Create JavaScript content.
    pub fn javascript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::JavaScript,
        }
    }

    /// Create TypeScript content.
    pub fn typescript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScript,
        }
    }

    /// Create TypeScript declaration content.
    pub fn declaration(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScriptDeclaration,
        }
    }

    /// Create source map content.
    pub fn source_map(value: serde_json::Value) -> Self {
        let content = serde_json::to_string(&value).unwrap_or_default();
        Self::Json {
            content,
            value,
            file_type: FileType::SourceMap,
        }
    }

    /// Create WebAssembly content.
    pub fn wasm(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Wasm,
        }
    }

    /// Create object code content.
    pub fn object(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Object,
        }
    }

    /// Get the file type of this content.
    pub fn file_type(&self) -> FileType {
        match self {
            Self::Text { file_type, .. } => *file_type,
            Self::Json { file_type, .. } => *file_type,
            Self::Binary { file_type, .. } => *file_type,
        }
    }

    /// Convert to FileContent for writing.
    pub fn to_file_content(&self) -> FileContent {
        match self {
            Self::Text { code, .. } => FileContent::Text {
                content: code.clone(),
            },
            Self::Json { content, value, .. } => FileContent::Json {
                content: content.clone(),
                value: value.clone(),
            },
            Self::Binary { bytes, .. } => FileContent::Binary {
                content: bytes.clone(),
            },
        }
    }
}

/// Key for artifact lookup by scope and target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactKey {
    /// The scope (module or package).
    pub scope: ArtifactScope,
    /// The target name.
    pub target: String,
}

impl ArtifactKey {
    /// Create a new artifact key.
    pub fn new(scope: ArtifactScope, target: impl Into<String>) -> Self {
        Self {
            scope,
            target: target.into(),
        }
    }

    /// Create a key for a module-scoped artifact.
    pub fn module(module: ModuleId, target: impl Into<String>) -> Self {
        Self::new(ArtifactScope::Module(module), target)
    }

    /// Create a key for a package-scoped artifact.
    pub fn package(package: PackageId, target: impl Into<String>) -> Self {
        Self::new(ArtifactScope::Package(package), target)
    }
}

/// Registry of generated artifacts. THREAD-SAFE.
///
/// Artifacts are the outputs of code generation, stored separately from modules
/// to keep the Module representation clean and codegen-agnostic.
#[derive(Debug)]
pub struct ArtifactRegistry {
    /// Artifacts by id.
    artifacts_by_id: DashMap<ArtifactId, Arc<Artifact>>,
    /// Artifact ids by (scope, target, file_type) for lookup.
    artifacts_by_key: DashMap<(ArtifactKey, FileType), ArtifactId>,
    /// The next artifact id.
    next_id: AtomicU32,
}

impl Default for ArtifactRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactRegistry {
    /// Create a new artifact registry.
    pub fn new() -> Self {
        Self {
            artifacts_by_id: DashMap::new(),
            artifacts_by_key: DashMap::new(),
            next_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next artifact id.
    pub fn next_id(&self) -> ArtifactId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ArtifactId::new(id)
    }

    /// Insert an artifact into the registry.
    pub fn insert(&self, artifact: Artifact) -> ArtifactId {
        let id = artifact.id;
        let key = ArtifactKey::new(artifact.scope, artifact.target.clone());
        let file_type = artifact.content.file_type();
        self.artifacts_by_key.insert((key, file_type), id);
        self.artifacts_by_id.insert(id, Arc::new(artifact));
        id
    }

    /// Get an artifact by id.
    pub fn get(&self, id: ArtifactId) -> Option<Arc<Artifact>> {
        self.artifacts_by_id.get(&id).map(|r| r.value().clone())
    }

    /// Get an artifact by scope, target, and file type.
    pub fn get_by_key(
        &self,
        scope: ArtifactScope,
        target: &str,
        file_type: FileType,
    ) -> Option<Arc<Artifact>> {
        let key = ArtifactKey::new(scope, target);
        let id = self.artifacts_by_key.get(&(key, file_type))?;
        self.get(*id)
    }

    /// Get a module-scoped artifact.
    pub fn get_module(
        &self,
        module: ModuleId,
        target: &str,
        file_type: FileType,
    ) -> Option<Arc<Artifact>> {
        self.get_by_key(ArtifactScope::Module(module), target, file_type)
    }

    /// Get a package-scoped artifact.
    pub fn get_package(
        &self,
        package: PackageId,
        target: &str,
        file_type: FileType,
    ) -> Option<Arc<Artifact>> {
        self.get_by_key(ArtifactScope::Package(package), target, file_type)
    }

    /// Check if an artifact exists by id.
    pub fn contains(&self, id: ArtifactId) -> bool {
        self.artifacts_by_id.contains_key(&id)
    }

    /// Get all artifacts for a target.
    pub fn get_by_target(&self, target: &str) -> Vec<Arc<Artifact>> {
        self.artifacts_by_id
            .iter()
            .filter(|r| r.value().target == target)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all module-scoped artifacts for a target.
    pub fn get_modules_by_target(&self, target: &str) -> Vec<Arc<Artifact>> {
        self.artifacts_by_id
            .iter()
            .filter(|r| r.value().target == target && r.value().scope.module().is_some())
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all artifacts for a module (across all targets).
    pub fn get_by_module(&self, module: ModuleId) -> Vec<Arc<Artifact>> {
        self.artifacts_by_id
            .iter()
            .filter(|r| r.value().scope.module() == Some(module))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all artifacts for a package (across all targets).
    pub fn get_by_package(&self, package: PackageId) -> Vec<Arc<Artifact>> {
        self.artifacts_by_id
            .iter()
            .filter(|r| r.value().scope.package() == Some(package))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Remove an artifact by id.
    pub fn remove(&self, id: ArtifactId) -> Option<Arc<Artifact>> {
        let artifact = self.artifacts_by_id.remove(&id).map(|(_, v)| v)?;
        let key = ArtifactKey::new(artifact.scope, artifact.target.clone());
        let file_type = artifact.content.file_type();
        self.artifacts_by_key.remove(&(key, file_type));
        Some(artifact)
    }

    /// Clear all artifacts.
    pub fn clear(&self) {
        self.artifacts_by_id.clear();
        self.artifacts_by_key.clear();
    }

    /// Get the number of artifacts.
    pub fn len(&self) -> usize {
        self.artifacts_by_id.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.artifacts_by_id.is_empty()
    }

    /// Iterate over all artifacts.
    pub fn iter(&self) -> impl Iterator<Item = Arc<Artifact>> + '_ {
        self.artifacts_by_id.iter().map(|r| r.value().clone())
    }
}
