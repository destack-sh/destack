use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{ModuleId, PackageId};

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

/// Generated artifact for a scope+target combination.
#[derive(Debug, Clone)]
pub struct Artifact {
    /// The scope (module or package).
    pub scope: ArtifactScope,
    /// The target name (e.g., "npm", "wasm").
    pub target: String,
    /// The generated content.
    pub content: ArtifactContent,
}

/// Content of a generated artifact.
#[derive(Debug, Clone)]
pub enum ArtifactContent {
    /// JavaScript output.
    JavaScript {
        /// The generated code.
        code: String,
        /// Source map (if generated).
        source_map: Option<String>,
    },
    /// TypeScript output.
    TypeScript {
        /// The generated code.
        code: String,
    },
    /// TypeScript declaration file (.d.ts).
    Declaration {
        /// The declaration content.
        dts: String,
    },
    /// Object code (not yet linked).
    ObjectCode {
        /// The compiled bytes (.o file).
        bytes: Vec<u8>,
    },
    /// Linked binary (executable or library).
    LinkedBinary {
        /// The linked bytes.
        bytes: Vec<u8>,
    },
    /// WebAssembly module.
    Wasm {
        /// The wasm bytes.
        bytes: Vec<u8>,
    },
}

/// Key for artifact lookup.
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
///
/// Artifacts are keyed by `(scope, target)` where:
/// - `scope` is either a single module or an entire package
/// - `target` is the build target name (e.g., "npm", "native")
#[derive(Debug, Default)]
pub struct ArtifactRegistry {
    /// Artifacts by (scope, target).
    artifacts: DashMap<ArtifactKey, Arc<Artifact>>,
}

impl ArtifactRegistry {
    /// Create a new artifact registry.
    pub fn new() -> Self {
        Self {
            artifacts: DashMap::new(),
        }
    }

    /// Insert an artifact into the registry.
    pub fn insert(&self, artifact: Artifact) {
        let key = ArtifactKey::new(artifact.scope, artifact.target.clone());
        self.artifacts.insert(key, Arc::new(artifact));
    }

    /// Get an artifact by scope and target.
    pub fn get(&self, scope: ArtifactScope, target: &str) -> Option<Arc<Artifact>> {
        let key = ArtifactKey::new(scope, target);
        self.artifacts.get(&key).map(|r| r.value().clone())
    }

    /// Get a module-scoped artifact.
    pub fn get_module(&self, module: ModuleId, target: &str) -> Option<Arc<Artifact>> {
        self.get(ArtifactScope::Module(module), target)
    }

    /// Get a package-scoped artifact.
    pub fn get_package(&self, package: PackageId, target: &str) -> Option<Arc<Artifact>> {
        self.get(ArtifactScope::Package(package), target)
    }

    /// Check if an artifact exists.
    pub fn contains(&self, scope: ArtifactScope, target: &str) -> bool {
        let key = ArtifactKey::new(scope, target);
        self.artifacts.contains_key(&key)
    }

    /// Get all artifacts for a target.
    pub fn get_by_target(&self, target: &str) -> Vec<Arc<Artifact>> {
        self.artifacts
            .iter()
            .filter(|r| r.key().target == target)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all module-scoped artifacts for a target.
    pub fn get_modules_by_target(&self, target: &str) -> Vec<Arc<Artifact>> {
        self.artifacts
            .iter()
            .filter(|r| r.key().target == target && r.key().scope.module().is_some())
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all artifacts for a module (across all  #targets).
    pub fn get_by_module(&self, module: ModuleId) -> Vec<Arc<Artifact>> {
        self.artifacts
            .iter()
            .filter(|r| r.key().scope.module() == Some(module))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all artifacts for a package (across all targets).
    pub fn get_by_package(&self, package: PackageId) -> Vec<Arc<Artifact>> {
        self.artifacts
            .iter()
            .filter(|r| r.key().scope.package() == Some(package))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Remove an artifact.
    pub fn remove(&self, scope: ArtifactScope, target: &str) -> Option<Arc<Artifact>> {
        let key = ArtifactKey::new(scope, target);
        self.artifacts.remove(&key).map(|(_, v)| v)
    }

    /// Clear all artifacts.
    pub fn clear(&self) {
        self.artifacts.clear();
    }

    /// Get the number of artifacts.
    pub fn len(&self) -> usize {
        self.artifacts.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}
