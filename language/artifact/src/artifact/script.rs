use destack_core::StringPool;
use destack_js::{LocalNodeIdAny, Tree};
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::SourceMapArtifact;

/// One generated script module wrapper.
#[derive(Debug, Clone)]
pub struct ScriptModule {
    /// The lowered script tree.
    pub tree: Tree,
    /// The root nodes in the lowered tree.
    pub roots: Vec<LocalNodeIdAny>,
    /// The string pool for the lowered tree.
    pub strings: StringPool,
}

/// One generated script artifact.
#[derive(Debug, Clone)]
pub struct ScriptArtifact {
    /// The emitted script language.
    pub language: ScriptLanguage,
    /// The generated script module.
    pub module: ScriptModule,
    /// The generated module dependency summary.
    pub linkage: ScriptLinkage,
    /// The generated declaration payload when one exists.
    pub declaration: Option<ScriptDeclaration>,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
    /// Whether the module has top level side effects.
    pub has_top_level_side_effects: bool,
}

/// One generated script declaration payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScriptDeclaration {
    /// The emitted declaration text.
    pub text: String,
}

/// One emitted script language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScriptLanguage {
    /// JavaScript output.
    JavaScript,
    /// TypeScript output.
    TypeScript,
}

/// The dependency summary for one generated script module.
#[derive(Debug, Clone, Default)]
pub struct ScriptLinkage {
    /// The static dependencies of the module.
    pub static_dependencies: Vec<StaticScriptDependency>,
    /// The dynamic dependencies of the module.
    pub dynamic_dependencies: Vec<DynamicScriptDependency>,
}

/// One static script dependency.
#[derive(Debug, Clone)]
pub struct StaticScriptDependency {
    /// The dependency usage in the module.
    pub usage: StaticScriptDependencyUsage,
    /// The dependency kind.
    pub kind: ScriptDependencyKind,
    /// The resolved dependency target.
    pub target: ScriptDependencyTarget,
}

/// One dynamic script dependency.
#[derive(Debug, Clone)]
pub struct DynamicScriptDependency {
    /// The resolved dependency target when one exists.
    pub target: DynamicScriptDependencyTarget,
}

/// The usage of one static script dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticScriptDependencyUsage {
    /// One import dependency.
    Import,
    /// One re-export dependency.
    Reexport,
}

/// The resolved target of one static script dependency.
#[derive(Debug, Clone)]
pub enum ScriptDependencyTarget {
    /// One dependency resolved to an internal module.
    Module {
        /// The resolved module id.
        module: ModuleId,
        /// The original import specifier.
        specifier: String,
    },
    /// One dependency that remains external.
    External {
        /// The original import specifier.
        specifier: String,
    },
}

impl ScriptDependencyTarget {
    /// Return the original dependency specifier.
    pub fn specifier(&self) -> &str {
        match self {
            Self::Module { specifier, .. } | Self::External { specifier } => specifier,
        }
    }

    /// Return the resolved internal module when one exists.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Module { module, .. } => Some(*module),
            Self::External { .. } => None,
        }
    }
}

/// The resolved target of one dynamic script dependency.
#[derive(Debug, Clone)]
pub enum DynamicScriptDependencyTarget {
    /// One dependency resolved to a concrete target.
    Resolved(ScriptDependencyTarget),
    /// One dependency whose target cannot be known statically.
    Opaque,
}

impl DynamicScriptDependencyTarget {
    /// Return the original dependency specifier when one exists.
    pub fn specifier(&self) -> Option<&str> {
        match self {
            Self::Resolved(target) => Some(target.specifier()),
            Self::Opaque => None,
        }
    }

    /// Return the resolved internal module when one exists.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Resolved(target) => target.module(),
            Self::Opaque => None,
        }
    }
}

/// The kind of a script dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptDependencyKind {
    /// Type-only dependency.
    Type,
    /// Value dependency.
    Value,
}
