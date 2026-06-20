use serde::{Deserialize, Serialize};

use destack_source::{ComponentId, ModuleId, PackageId, ProductId, ProfileId, TargetId};

/// Provider family for one artifact key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactProvider {
    /// Loader artifacts derived directly from repository file contents.
    Loader,
    /// Compiler artifacts derived by compiler phases.
    Compiler,
    /// Linter artifacts derived by lint rules.
    Linter,
    /// Query indexes derived for navigation and refactoring.
    Query,
}

/// Semantic artifact identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactKey {
    /// Toolchain build payload for one target.
    Build { target: TargetId },

    /// Parsed module DIR.
    DirParsed { module: ModuleId },
    /// Parsed non-code module data.
    Data { module: ModuleId },

    /// Explicit global environment for one profile.
    GlobalEnvironment { profile: ProfileId },
    /// Active dependency index for one profile.
    PackageIndex { profile: ProfileId },
    /// Strongly connected component partition for one profile.
    ComponentGraph { profile: ProfileId },
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

    /// Checked DIR component.
    DirCheckedComponent {
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    },
    /// Checked DIR facade.
    DirChecked {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Materialized DIR.
    DirMaterialized {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Elaborated DIR.
    DirElaborated {
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

    /// Query index for one module profile.
    ModuleQueryIndex {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Query index for one workspace profile.
    WorkspaceQueryIndex { profile: ProfileId },

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

    /// Realized lint diagnostics for one module profile.
    ModuleLinted {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Realized lint diagnostics for one package.
    PackageLinted { package: PackageId },
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted,
}

/// High-level toolchain stage that owns one artifact kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// MIR synthesis, from elaboration through optimization.
    Lower,
    /// Target code emission per module.
    Emit,
    /// Final program assembly across modules.
    Link,
    /// Lint analysis over modules, packages, and the workspace.
    Lint,
    /// Query indexes serving editors and tooling.
    Query,
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
        Self::Query,
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
            Self::Query => "query",
        }
    }
}

impl ArtifactKey {
    /// Return the provider family responsible for this artifact.
    pub fn provider(self) -> ArtifactProvider {
        match self {
            Self::DirParsed { .. } | Self::Data { .. } => ArtifactProvider::Loader,
            Self::GlobalEnvironment { .. }
            | Self::PackageIndex { .. }
            | Self::ComponentGraph { .. }
            | Self::ProgramAnalysis { .. }
            | Self::DirBound { .. }
            | Self::DirImported { .. }
            | Self::DirExpanded { .. }
            | Self::DirExported { .. }
            | Self::DirResolved { .. }
            | Self::DirCheckedComponent { .. }
            | Self::DirChecked { .. }
            | Self::DirMaterialized { .. }
            | Self::DirElaborated { .. }
            | Self::MirLowered { .. }
            | Self::MirVerified { .. }
            | Self::MirAnalyzed { .. }
            | Self::MirOptimized { .. }
            | Self::Script { .. }
            | Self::Object { .. }
            | Self::Asset { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. } => ArtifactProvider::Compiler,
            Self::ModuleLinted { .. } | Self::PackageLinted { .. } | Self::WorkspaceLinted => {
                ArtifactProvider::Linter
            }
            Self::ModuleQueryIndex { .. } | Self::WorkspaceQueryIndex { .. } => {
                ArtifactProvider::Query
            }
        }
    }

    /// Return the package referenced by this artifact key when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::Build { target } => Some(target.package_id()),
            Self::Bundle { package, .. }
            | Self::Program { package, .. }
            | Self::Product { package, .. }
            | Self::PackageLinted { package } => Some(*package),
            _ => None,
        }
    }

    /// Build one global environment artifact key.
    pub fn global_environment(profile: ProfileId) -> Self {
        Self::GlobalEnvironment { profile }
    }

    /// Build one dependency index artifact key.
    pub fn package_index(profile: ProfileId) -> Self {
        Self::PackageIndex { profile }
    }

    /// Build one component graph artifact key.
    pub fn component_graph(profile: ProfileId) -> Self {
        Self::ComponentGraph { profile }
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

    /// Build one checked DIR component artifact key.
    pub fn dir_checked_component(
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Self {
        Self::DirCheckedComponent {
            entry,
            component,
            profile,
        }
    }

    /// Build one checked DIR facade artifact key.
    pub fn dir_checked(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirChecked { module, profile }
    }

    /// Build one materialized DIR artifact key.
    pub fn dir_materialized(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirMaterialized { module, profile }
    }

    /// Build one elaborated DIR artifact key.
    pub fn dir_elaborated(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirElaborated { module, profile }
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

    /// Build one module query index artifact key.
    pub fn module_query_index(module: ModuleId, profile: ProfileId) -> Self {
        Self::ModuleQueryIndex { module, profile }
    }

    /// Build one workspace query index artifact key.
    pub fn workspace_query_index(profile: ProfileId) -> Self {
        Self::WorkspaceQueryIndex { profile }
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
    pub fn module_linted(module: ModuleId, profile: ProfileId) -> Self {
        Self::ModuleLinted { module, profile }
    }

    /// Build one package lint artifact key.
    pub fn package_linted(package: PackageId) -> Self {
        Self::PackageLinted { package }
    }

    /// Build one workspace lint artifact key.
    pub fn workspace_linted() -> Self {
        Self::WorkspaceLinted
    }

    /// Return the toolchain stage that owns this artifact kind.
    pub fn stage(&self) -> ArtifactStage {
        match self {
            Self::DirParsed { .. } | Self::Data { .. } => ArtifactStage::Parse,
            Self::DirBound { .. } => ArtifactStage::Bind,
            Self::DirImported { .. } | Self::DirExported { .. } | Self::DirResolved { .. } => {
                ArtifactStage::Resolve
            }
            Self::ComponentGraph { .. } => ArtifactStage::Graph,
            Self::DirExpanded { .. } | Self::DirMaterialized { .. } => ArtifactStage::Macro,
            Self::DirCheckedComponent { .. } | Self::DirChecked { .. } => ArtifactStage::Check,
            Self::DirElaborated { .. }
            | Self::MirLowered { .. }
            | Self::MirVerified { .. }
            | Self::MirAnalyzed { .. }
            | Self::ProgramAnalysis { .. }
            | Self::MirOptimized { .. } => ArtifactStage::Lower,
            Self::Script { .. } | Self::Object { .. } | Self::Asset { .. } => ArtifactStage::Emit,
            Self::Build { .. } => ArtifactStage::Init,
            Self::Bundle { .. } | Self::Program { .. } | Self::Product { .. } => {
                ArtifactStage::Link
            }
            Self::ModuleLinted { .. } | Self::PackageLinted { .. } | Self::WorkspaceLinted => {
                ArtifactStage::Lint
            }
            Self::ModuleQueryIndex { .. } | Self::WorkspaceQueryIndex { .. } => {
                ArtifactStage::Query
            }
            Self::GlobalEnvironment { .. } | Self::PackageIndex { .. } => ArtifactStage::Init,
        }
    }

    /// Return the human-facing display name for this key's kind.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment { .. } => "environment",
            Self::PackageIndex { .. } => "package.index",
            Self::DirParsed { .. } => "dir.parse",
            Self::Data { .. } => "data",
            Self::DirBound { .. } => "dir.bind",
            Self::DirImported { .. } => "dir.import",
            Self::DirExpanded { .. } => "dir.expand",
            Self::DirExported { .. } => "dir.export",
            Self::DirResolved { .. } => "dir.resolve",
            Self::ComponentGraph { .. } => "component.graph",
            Self::ProgramAnalysis { .. } => "program.analyze",
            Self::DirCheckedComponent { .. } => "dir.check.component",
            Self::DirChecked { .. } => "dir.check",
            Self::DirMaterialized { .. } => "dir.materialize",
            Self::DirElaborated { .. } => "dir.elaborate",
            Self::MirLowered { .. } => "mir.lower",
            Self::MirVerified { .. } => "mir.verify",
            Self::MirAnalyzed { .. } => "mir.analyze",
            Self::MirOptimized { .. } => "mir.optimize",
            Self::ModuleQueryIndex { .. } => "module.index",
            Self::WorkspaceQueryIndex { .. } => "workspace.index",
            Self::Script { .. } => "script.emit",
            Self::Object { .. } => "object.emit",
            Self::Asset { .. } => "asset.emit",
            Self::Build { .. } => "build",
            Self::Bundle { .. } => "bundle.link",
            Self::Program { .. } => "program.link",
            Self::Product { .. } => "product.link",
            Self::ModuleLinted { .. } => "module.lint",
            Self::PackageLinted { .. } => "package.lint",
            Self::WorkspaceLinted => "workspace.lint",
        }
    }

    /// Return the stable short name for this key.
    pub fn name(&self) -> &'static str {
        match self {
            Self::GlobalEnvironment { .. } => "global_environment",
            Self::PackageIndex { .. } => "package_index",
            Self::DirParsed { .. } => "dir_parsed",
            Self::Data { .. } => "data",
            Self::DirBound { .. } => "dir_bound",
            Self::DirImported { .. } => "dir_imported",
            Self::DirExpanded { .. } => "dir_expanded",
            Self::DirExported { .. } => "dir_exported",
            Self::DirResolved { .. } => "dir_resolved",
            Self::ComponentGraph { .. } => "component_graph",
            Self::ProgramAnalysis { .. } => "program_analysis",
            Self::DirCheckedComponent { .. } => "dir_checked_component",
            Self::DirChecked { .. } => "dir_checked",
            Self::DirMaterialized { .. } => "dir_materialized",
            Self::DirElaborated { .. } => "dir_elaborated",
            Self::MirLowered { .. } => "mir_lowered",
            Self::MirVerified { .. } => "mir_verified",
            Self::MirAnalyzed { .. } => "mir_analyzed",
            Self::MirOptimized { .. } => "mir_optimized",
            Self::ModuleQueryIndex { .. } => "module_query_index",
            Self::WorkspaceQueryIndex { .. } => "workspace_query_index",
            Self::Script { .. } => "script",
            Self::Object { .. } => "object",
            Self::Asset { .. } => "asset",
            Self::Build { .. } => "build",
            Self::Bundle { .. } => "bundle",
            Self::Program { .. } => "program",
            Self::Product { .. } => "product",
            Self::ModuleLinted { .. } => "module_linted",
            Self::PackageLinted { .. } => "package_linted",
            Self::WorkspaceLinted => "workspace_linted",
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
            | Self::DirCheckedComponent { entry: module, .. }
            | Self::DirChecked { module, .. }
            | Self::DirMaterialized { module, .. }
            | Self::DirElaborated { module, .. }
            | Self::MirLowered { module, .. }
            | Self::MirVerified { module, .. }
            | Self::MirAnalyzed { module, .. }
            | Self::MirOptimized { module, .. }
            | Self::ModuleQueryIndex { module, .. }
            | Self::Script { module, .. }
            | Self::Object { module, .. }
            | Self::Asset { module, .. }
            | Self::ModuleLinted { module, .. } => Some(*module),
            Self::GlobalEnvironment { .. }
            | Self::PackageIndex { .. }
            | Self::ComponentGraph { .. }
            | Self::ProgramAnalysis { .. }
            | Self::WorkspaceQueryIndex { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
        }
    }
}

impl ArtifactKey {
    /// Return the target id encoded in this key when one exists.
    pub fn target_id(&self) -> Option<TargetId> {
        match self {
            Self::Script { target, .. }
            | Self::Object { target, .. }
            | Self::Asset { target, .. }
            | Self::Build { target }
            | Self::Bundle { target, .. }
            | Self::Program { target, .. } => Some(*target),
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
            | Self::PackageIndex { profile }
            | Self::ComponentGraph { profile }
            | Self::ProgramAnalysis { profile, .. }
            | Self::DirBound { profile, .. }
            | Self::DirImported { profile, .. }
            | Self::DirExpanded { profile, .. }
            | Self::DirExported { profile, .. }
            | Self::DirResolved { profile, .. }
            | Self::DirCheckedComponent { profile, .. }
            | Self::DirChecked { profile, .. }
            | Self::DirMaterialized { profile, .. }
            | Self::DirElaborated { profile, .. }
            | Self::MirLowered { profile, .. }
            | Self::MirVerified { profile, .. }
            | Self::MirAnalyzed { profile, .. }
            | Self::MirOptimized { profile, .. }
            | Self::ModuleQueryIndex { profile, .. }
            | Self::WorkspaceQueryIndex { profile }
            | Self::ModuleLinted { profile, .. } => Some(*profile),
            Self::DirParsed { .. }
            | Self::Data { .. }
            | Self::Script { .. }
            | Self::Object { .. }
            | Self::Asset { .. }
            | Self::Build { .. }
            | Self::Bundle { .. }
            | Self::Program { .. }
            | Self::Product { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
        }
    }
}
