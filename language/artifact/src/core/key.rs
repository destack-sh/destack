use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProductId, ProfileId, TargetId};

use crate::IndexKind;

/// Provider family for one artifact key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ArtifactProvider {
    /// Loader artifacts derived directly from repository file contents.
    Loader,
    /// Compiler artifacts derived by compiler phases.
    Compiler,
    /// Linter artifacts derived by lint rules.
    Linter,
    /// Index artifacts derived from checked compiler IR.
    Index,
}

/// One artifact kind and its owner.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ArtifactKey {
    /// Toolchain build payload for one target.
    Build { target: TargetId },

    /// Parsed module DIR.
    DirParsed { module: ModuleId },
    /// Parsed non-code module data.
    Data { module: ModuleId },

    /// Explicit global environment for one profile.
    GlobalEnvironment { profile: ProfileId },
    /// Import graph over the modules of one profile.
    ModuleGraph { profile: ProfileId },
    /// Content digest of the implicit global modules for one profile.
    GlobalEnvironmentDigest { profile: ProfileId },
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis {
        profile: ProfileId,
        target: TargetId,
    },

    /// Bound DIR.
    DirBound {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Imported DIR.
    DirImported {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Expanded DIR.
    DirExpanded {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Exported DIR.
    DirExported {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Resolved DIR imports.
    DirResolved {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Declared DIR module.
    DirDeclared {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Checked DIR module.
    DirChecked {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Materialized DIR.
    DirMaterialized {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Lowered MIR before optimization.
    MirLowered {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// Verified MIR after required semantic verification.
    MirVerified {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// MIR after required executable elaboration.
    MirElaborated {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// Per-module link summary for whole-program analysis.
    MirAnalyzed {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },

    /// One query index for a module profile.
    ModuleIndex {
        module: ModuleId,
        profile: ProfileId,
        kind: IndexKind,
    },
    /// One query index for a program profile.
    ProgramIndex { profile: ProfileId, kind: IndexKind },

    /// One structured linker input for one target.
    Script { module: ModuleId, target: TargetId },
    /// One compiled-code linker input for one target.
    Object { module: ModuleId, target: TargetId },
    /// One opaque linker input for one target.
    Asset { module: ModuleId, target: TargetId },
    /// Linked file graph for one package target.
    Bundle {
        package: PackageId,
        target: TargetId,
    },
    /// Program for one package target.
    Program {
        package: PackageId,
        target: TargetId,
    },
    /// Linked product assembled from configured target artifacts.
    Product {
        package: PackageId,
        product: ProductId,
    },

    /// Completed lint analysis for one module in one target.
    ModuleLinted {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// Completed lint analysis for one target program.
    ProgramLinted {
        profile: ProfileId,
        target: TargetId,
    },
}

/// High-level toolchain stage that owns one artifact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ArtifactStage {
    /// Build initialization: environment and dependency resolution.
    Init,
    /// Source parsing into DIR.
    Parse,
    /// Local name binding.
    Bind,
    /// Comptime expansion and materialization around check.
    Macro,
    /// Import, export, and global resolution.
    Resolve,
    /// Component graph indexes.
    Graph,
    /// Type checking.
    Check,
    /// MIR lowering, verification, analysis, and optimization.
    Lower,
    /// Target code emission per module.
    Emit,
    /// Final program assembly across modules.
    Link,
    /// Lint analysis over modules and target programs.
    Lint,
    /// Indexes serving editors and tooling.
    Index,
}

impl ArtifactStage {
    /// All artifact stages in display order.
    pub const ALL: [Self; 12] = [
        Self::Init,
        Self::Parse,
        Self::Bind,
        Self::Macro,
        Self::Resolve,
        Self::Graph,
        Self::Check,
        Self::Lower,
        Self::Emit,
        Self::Link,
        Self::Lint,
        Self::Index,
    ];

    /// Return this stage's display name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Parse => "parse",
            Self::Bind => "bind",
            Self::Macro => "macro",
            Self::Resolve => "resolve",
            Self::Graph => "graph",
            Self::Check => "check",
            Self::Lower => "lower",
            Self::Emit => "emit",
            Self::Link => "link",
            Self::Lint => "lint",
            Self::Index => "index",
        }
    }
}

impl ArtifactKey {
    /// Return the provider family responsible for this artifact.
    pub fn provider(self) -> ArtifactProvider {
        match self {
            Self::DirParsed { .. } | Self::Data { .. } => ArtifactProvider::Loader,
            Self::GlobalEnvironment { .. }
            | Self::ModuleGraph { .. }
            | Self::GlobalEnvironmentDigest { .. }
            | Self::ProgramAnalysis { .. }
            | Self::DirBound { .. }
            | Self::DirImported { .. }
            | Self::DirExpanded { .. }
            | Self::DirExported { .. }
            | Self::DirResolved { .. }
            | Self::DirDeclared { .. }
            | Self::DirChecked { .. }
            | Self::DirMaterialized { .. }
            | Self::MirLowered { .. }
            | Self::MirVerified { .. }
            | Self::MirElaborated { .. }
            | Self::MirAnalyzed { .. }
            | Self::MirOptimized { .. }
            | Self::Script { .. }
            | Self::Object { .. }
            | Self::Asset { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. } => ArtifactProvider::Compiler,
            Self::ModuleLinted { .. } | Self::ProgramLinted { .. } => ArtifactProvider::Linter,
            Self::ModuleIndex { .. } | Self::ProgramIndex { .. } => ArtifactProvider::Index,
        }
    }

    /// Return the package referenced by this artifact key when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::Build { target }
            | Self::ProgramAnalysis { target, .. }
            | Self::ProgramLinted { target, .. } => Some(target.package_id()),
            Self::Bundle { package, .. }
            | Self::Program { package, .. }
            | Self::Product { package, .. } => Some(*package),
            _ => None,
        }
    }

    /// Build one global environment artifact key.
    pub fn global_environment(profile: ProfileId) -> Self {
        Self::GlobalEnvironment { profile }
    }

    /// Build one component graph artifact key.
    pub fn module_graph(profile: ProfileId) -> Self {
        Self::ModuleGraph { profile }
    }

    /// Build one environment digest artifact key.
    pub fn global_environment_digest(profile: ProfileId) -> Self {
        Self::GlobalEnvironmentDigest { profile }
    }

    /// Build one whole-program analysis artifact key.
    pub fn program_analysis(profile: ProfileId, target: TargetId) -> Self {
        Self::ProgramAnalysis { profile, target }
    }

    /// Build one DIR artifact key.
    pub fn dir_parsed(module: ModuleId) -> Self {
        Self::DirParsed { module }
    }

    /// Build one data artifact key.
    pub fn data(module: ModuleId) -> Self {
        Self::Data { module }
    }

    /// Build one bound DIR artifact key.
    pub fn dir_bound(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirBound { module, profile }
    }

    /// Build one imported DIR artifact key.
    pub fn dir_imported(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirImported { module, profile }
    }

    /// Build one expanded DIR artifact key.
    pub fn dir_expanded(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirExpanded { module, profile }
    }

    /// Build one exported DIR artifact key.
    pub fn dir_exported(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirExported { module, profile }
    }

    /// Build one resolved DIR artifact key.
    pub fn dir_resolved(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirResolved { module, profile }
    }

    /// Build one declared DIR component artifact key.
    pub fn dir_declared(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirDeclared { module, profile }
    }

    /// Build one checked DIR module artifact key.
    pub fn dir_checked(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirChecked { module, profile }
    }

    /// Build one materialized DIR artifact key.
    pub fn dir_materialized(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirMaterialized { module, profile }
    }

    /// Build one lowered MIR artifact key.
    pub fn mir_lowered(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirLowered {
            module,
            profile,
            target,
        }
    }

    /// Build one verified MIR artifact key.
    pub fn mir_verified(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirVerified {
            module,
            profile,
            target,
        }
    }

    /// Build one elaborated MIR artifact key.
    pub fn mir_elaborated(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirElaborated {
            module,
            profile,
            target,
        }
    }

    /// Build one analyzed MIR artifact key.
    pub fn mir_analyzed(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirAnalyzed {
            module,
            profile,
            target,
        }
    }

    /// Build one optimized MIR artifact key.
    pub fn mir_optimized(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirOptimized {
            module,
            profile,
            target,
        }
    }

    /// Build one module index artifact key.
    pub fn module_index(module: ModuleId, profile: ProfileId, kind: IndexKind) -> Self {
        Self::ModuleIndex {
            module,
            profile,
            kind,
        }
    }

    /// Build one program index artifact key.
    pub fn program_index(profile: ProfileId, kind: IndexKind) -> Self {
        Self::ProgramIndex { profile, kind }
    }

    /// Build one structured script key.
    pub fn script(module: ModuleId, target: TargetId) -> Self {
        Self::Script { module, target }
    }

    /// Build one compiled-code object key.
    pub fn object(module: ModuleId, target: TargetId) -> Self {
        Self::Object { module, target }
    }

    /// Build one opaque asset key.
    pub fn asset(module: ModuleId, target: TargetId) -> Self {
        Self::Asset { module, target }
    }

    /// Build one toolchain build key.
    pub fn build(target: TargetId) -> Self {
        Self::Build { target }
    }

    /// Build one bundle artifact key.
    pub fn bundle(package: PackageId, target: TargetId) -> Self {
        Self::Bundle { package, target }
    }

    /// Build one program artifact key.
    pub fn program(package: PackageId, target: TargetId) -> Self {
        Self::Program { package, target }
    }

    /// Build one product artifact key.
    pub fn product(package: PackageId, product: ProductId) -> Self {
        Self::Product { package, product }
    }

    /// Build one module lint artifact key.
    pub fn module_linted(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::ModuleLinted {
            module,
            profile,
            target,
        }
    }

    /// Build one program lint artifact key.
    pub fn program_linted(profile: ProfileId, target: TargetId) -> Self {
        Self::ProgramLinted { profile, target }
    }

    /// Return the toolchain stage that owns this artifact kind.
    pub fn stage(&self) -> ArtifactStage {
        match self {
            Self::DirParsed { .. } | Self::Data { .. } => ArtifactStage::Parse,
            Self::DirBound { .. } => ArtifactStage::Bind,
            Self::DirImported { .. } | Self::DirExported { .. } | Self::DirResolved { .. } => {
                ArtifactStage::Resolve
            }
            Self::ModuleGraph { .. } => ArtifactStage::Graph,
            Self::GlobalEnvironmentDigest { .. } => ArtifactStage::Graph,
            Self::DirExpanded { .. } | Self::DirMaterialized { .. } => ArtifactStage::Macro,
            Self::DirDeclared { .. } | Self::DirChecked { .. } => ArtifactStage::Check,
            Self::MirLowered { .. }
            | Self::MirVerified { .. }
            | Self::MirElaborated { .. }
            | Self::MirAnalyzed { .. }
            | Self::ProgramAnalysis { .. }
            | Self::MirOptimized { .. } => ArtifactStage::Lower,
            Self::Script { .. } | Self::Object { .. } | Self::Asset { .. } => ArtifactStage::Emit,
            Self::Build { .. } => ArtifactStage::Init,
            Self::Bundle { .. } | Self::Program { .. } | Self::Product { .. } => {
                ArtifactStage::Link
            }
            Self::ModuleLinted { .. } | Self::ProgramLinted { .. } => ArtifactStage::Lint,
            Self::ModuleIndex { .. } | Self::ProgramIndex { .. } => ArtifactStage::Index,
            Self::GlobalEnvironment { .. } => ArtifactStage::Init,
        }
    }

    /// Return the human-facing display name for this key's kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment { .. } => "environment",
            Self::DirParsed { .. } => "dir.parse",
            Self::Data { .. } => "data",
            Self::DirBound { .. } => "dir.bind",
            Self::DirImported { .. } => "dir.import",
            Self::DirExpanded { .. } => "dir.expand",
            Self::DirExported { .. } => "dir.export",
            Self::DirResolved { .. } => "dir.resolve",
            Self::ModuleGraph { .. } => "module.graph",
            Self::GlobalEnvironmentDigest { .. } => "environment.digest",
            Self::ProgramAnalysis { .. } => "program.analyze",
            Self::DirDeclared { .. } => "dir.declare",
            Self::DirChecked { .. } => "dir.check",
            Self::DirMaterialized { .. } => "dir.materialize",
            Self::MirLowered { .. } => "mir.lower",
            Self::MirVerified { .. } => "mir.verify",
            Self::MirElaborated { .. } => "mir.elaborate",
            Self::MirAnalyzed { .. } => "mir.analyze",
            Self::MirOptimized { .. } => "mir.optimize",
            Self::ModuleIndex { .. } => "module.index",
            Self::ProgramIndex { .. } => "program.index",
            Self::Script { .. } => "script.emit",
            Self::Object { .. } => "object.emit",
            Self::Asset { .. } => "asset.emit",
            Self::Build { .. } => "build",
            Self::Bundle { .. } => "bundle.link",
            Self::Program { .. } => "program.link",
            Self::Product { .. } => "product.link",
            Self::ModuleLinted { .. } => "module.lint",
            Self::ProgramLinted { .. } => "program.lint",
        }
    }

    /// Return the stable short name for this key.
    pub fn name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment { .. } => "global_environment",
            Self::DirParsed { .. } => "dir_parsed",
            Self::Data { .. } => "data",
            Self::DirBound { .. } => "dir_bound",
            Self::DirImported { .. } => "dir_imported",
            Self::DirExpanded { .. } => "dir_expanded",
            Self::DirExported { .. } => "dir_exported",
            Self::DirResolved { .. } => "dir_resolved",
            Self::ModuleGraph { .. } => "module_graph",
            Self::GlobalEnvironmentDigest { .. } => "global_environment_digest",
            Self::ProgramAnalysis { .. } => "program_analysis",
            Self::DirDeclared { .. } => "dir_declared",
            Self::DirChecked { .. } => "dir_checked",
            Self::DirMaterialized { .. } => "dir_materialized",
            Self::MirLowered { .. } => "mir_lowered",
            Self::MirVerified { .. } => "mir_verified",
            Self::MirElaborated { .. } => "mir_elaborated",
            Self::MirAnalyzed { .. } => "mir_analyzed",
            Self::MirOptimized { .. } => "mir_optimized",
            Self::ModuleIndex { .. } => "module_index",
            Self::ProgramIndex { .. } => "program_index",
            Self::Script { .. } => "script",
            Self::Object { .. } => "object",
            Self::Asset { .. } => "asset",
            Self::Build { .. } => "build",
            Self::Bundle { .. } => "bundle",
            Self::Program { .. } => "program",
            Self::Product { .. } => "product",
            Self::ModuleLinted { .. } => "module_linted",
            Self::ProgramLinted { .. } => "program_linted",
        }
    }

    /// Return the module id encoded in this key when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::DirParsed { module }
            | Self::Data { module }
            | Self::DirBound { module, .. }
            | Self::DirImported { module, .. }
            | Self::DirExpanded { module, .. }
            | Self::DirExported { module, .. }
            | Self::DirResolved { module, .. }
            | Self::DirDeclared { module, .. }
            | Self::DirChecked { module, .. }
            | Self::DirMaterialized { module, .. }
            | Self::MirLowered { module, .. }
            | Self::MirVerified { module, .. }
            | Self::MirElaborated { module, .. }
            | Self::MirAnalyzed { module, .. }
            | Self::MirOptimized { module, .. }
            | Self::ModuleIndex { module, .. }
            | Self::Script { module, .. }
            | Self::Object { module, .. }
            | Self::Asset { module, .. }
            | Self::ModuleLinted { module, .. } => Some(*module),
            Self::GlobalEnvironment { .. }
            | Self::ModuleGraph { .. }
            | Self::GlobalEnvironmentDigest { .. }
            | Self::ProgramAnalysis { .. }
            | Self::ProgramIndex { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. }
            | Self::ProgramLinted { .. } => None,
        }
    }
}

impl ArtifactKey {
    /// Return the target id encoded in this key when one exists.
    pub fn target_id(&self) -> Option<TargetId> {
        match self {
            Self::ProgramAnalysis { target, .. }
            | Self::MirLowered { target, .. }
            | Self::MirVerified { target, .. }
            | Self::MirAnalyzed { target, .. }
            | Self::MirOptimized { target, .. }
            | Self::Script { target, .. }
            | Self::Object { target, .. }
            | Self::Asset { target, .. }
            | Self::Build { target }
            | Self::Bundle { target, .. }
            | Self::Program { target, .. }
            | Self::ModuleLinted { target, .. }
            | Self::ProgramLinted { target, .. } => Some(*target),
            _ => None,
        }
    }

    /// Return the product id encoded in this key when one exists.
    pub fn product_id(&self) -> Option<ProductId> {
        match self {
            Self::Product { product, .. } => Some(*product),
            _ => None,
        }
    }

    /// Return the profile id encoded in this key when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        match self {
            Self::GlobalEnvironment { profile }
            | Self::ModuleGraph { profile }
            | Self::GlobalEnvironmentDigest { profile }
            | Self::ProgramAnalysis { profile, .. }
            | Self::DirBound { profile, .. }
            | Self::DirImported { profile, .. }
            | Self::DirExpanded { profile, .. }
            | Self::DirExported { profile, .. }
            | Self::DirResolved { profile, .. }
            | Self::DirDeclared { profile, .. }
            | Self::DirChecked { profile, .. }
            | Self::DirMaterialized { profile, .. }
            | Self::MirLowered { profile, .. }
            | Self::MirVerified { profile, .. }
            | Self::MirElaborated { profile, .. }
            | Self::MirAnalyzed { profile, .. }
            | Self::MirOptimized { profile, .. }
            | Self::ModuleIndex { profile, .. }
            | Self::ProgramIndex { profile, .. }
            | Self::ModuleLinted { profile, .. }
            | Self::ProgramLinted { profile, .. } => Some(*profile),
            Self::DirParsed { .. }
            | Self::Data { .. }
            | Self::Script { .. }
            | Self::Object { .. }
            | Self::Asset { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. } => None,
        }
    }
}
