// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{ComponentId, ModuleId, PackageId, ProductId, ProfileId, TargetId};

/// External artifact key crossing bridge boundaries.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct ArtifactKey {
    content: ArtifactKeyContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum ArtifactKeyContent {
    /// Toolchain build payload for one target.
    Build {
        /// Build target.
        target: TargetId,
    },
    /// Parsed module DIR.
    DirParsed {
        /// Source module.
        module: ModuleId,
    },
    /// Parsed non-code module data.
    Data {
        /// Source module.
        module: ModuleId,
    },
    /// Explicit global environment for one profile.
    GlobalEnvironment {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Active dependency index for one profile.
    PackageIndex {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Component partition for one profile.
    ComponentGraph {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis {
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Bound DIR.
    DirBound {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Imported DIR.
    DirImported {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Expanded DIR.
    DirExpanded {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Exported DIR.
    DirExported {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Resolved DIR imports.
    DirResolved {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Checked DIR component.
    DirCheckedComponent {
        /// Component entry module.
        entry: ModuleId,
        /// Checked component id.
        component: ComponentId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Checked DIR facade.
    DirChecked {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Materialized DIR.
    DirMaterialized {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Elaborated DIR.
    DirElaborated {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Lowered MIR before optimization.
    MirLowered {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Verified MIR after required semantic verification.
    MirVerified {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Per-module link summary for whole-program analysis.
    MirAnalyzed {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Query index for one module profile.
    ModuleQueryIndex {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Query index for one workspace profile.
    WorkspaceQueryIndex {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// One structured linker input for one target.
    Script {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// One compiled-code linker input for one target.
    Object {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// One opaque linker input for one target.
    Asset {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// Linked file graph for one package target.
    Bundle {
        /// Source package.
        package: PackageId,
        /// Build target.
        target: TargetId,
    },
    /// Executable program for one package target.
    Program {
        /// Source package.
        package: PackageId,
        /// Build target.
        target: TargetId,
    },
    /// Linked product assembled from configured target artifacts.
    Product {
        /// Source package.
        package: PackageId,
        /// Product.
        product: ProductId,
    },
    /// Realized lint diagnostics for one module profile.
    ModuleLinted {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Realized lint diagnostics for one package.
    PackageLinted {
        /// Source package.
        package: PackageId,
    },
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted,
}

#[wasm_bindgen]
impl ArtifactKey {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "build")]
    pub fn build(target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Build { target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirParsed")]
    pub fn dir_parsed(module: ModuleId) -> Self {
        Self {
            content: ArtifactKeyContent::DirParsed { module },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "data")]
    pub fn data(module: ModuleId) -> Self {
        Self {
            content: ArtifactKeyContent::Data { module },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "globalEnvironment")]
    pub fn global_environment(profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::GlobalEnvironment { profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "packageIndex")]
    pub fn package_index(profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::PackageIndex { profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "componentGraph")]
    pub fn component_graph(profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::ComponentGraph { profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "programAnalysis")]
    pub fn program_analysis(profile: ProfileId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::ProgramAnalysis { profile, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirBound")]
    pub fn dir_bound(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirBound { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirImported")]
    pub fn dir_imported(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirImported { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirExpanded")]
    pub fn dir_expanded(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirExpanded { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirExported")]
    pub fn dir_exported(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirExported { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirResolved")]
    pub fn dir_resolved(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirResolved { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirCheckedComponent")]
    pub fn dir_checked_component(
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Self {
        Self {
            content: ArtifactKeyContent::DirCheckedComponent {
                entry,
                component,
                profile,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirChecked")]
    pub fn dir_checked(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirChecked { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirMaterialized")]
    pub fn dir_materialized(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirMaterialized { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "dirElaborated")]
    pub fn dir_elaborated(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::DirElaborated { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "mirLowered")]
    pub fn mir_lowered(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::MirLowered {
                module,
                profile,
                target,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "mirVerified")]
    pub fn mir_verified(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::MirVerified {
                module,
                profile,
                target,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "mirAnalyzed")]
    pub fn mir_analyzed(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::MirAnalyzed {
                module,
                profile,
                target,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "mirOptimized")]
    pub fn mir_optimized(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::MirOptimized {
                module,
                profile,
                target,
            },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "moduleQueryIndex")]
    pub fn module_query_index(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::ModuleQueryIndex { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "workspaceQueryIndex")]
    pub fn workspace_query_index(profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::WorkspaceQueryIndex { profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "script")]
    pub fn script(module: ModuleId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Script { module, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "object")]
    pub fn object(module: ModuleId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Object { module, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "asset")]
    pub fn asset(module: ModuleId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Asset { module, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "bundle")]
    pub fn bundle(package: PackageId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Bundle { package, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "program")]
    pub fn program(package: PackageId, target: TargetId) -> Self {
        Self {
            content: ArtifactKeyContent::Program { package, target },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "product")]
    pub fn product(package: PackageId, product: ProductId) -> Self {
        Self {
            content: ArtifactKeyContent::Product { package, product },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "moduleLinted")]
    pub fn module_linted(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            content: ArtifactKeyContent::ModuleLinted { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "packageLinted")]
    pub fn package_linted(package: PackageId) -> Self {
        Self {
            content: ArtifactKeyContent::PackageLinted { package },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "workspaceLinted")]
    pub fn workspace_linted() -> Self {
        Self {
            content: ArtifactKeyContent::WorkspaceLinted,
        }
    }

    /// Payload variant label.
    #[wasm_bindgen(getter, js_name = "kind")]
    pub fn kind(&self) -> String {
        let label = match &self.content {
            ArtifactKeyContent::Build { .. } => "build",
            ArtifactKeyContent::DirParsed { .. } => "dirParsed",
            ArtifactKeyContent::Data { .. } => "data",
            ArtifactKeyContent::GlobalEnvironment { .. } => "globalEnvironment",
            ArtifactKeyContent::PackageIndex { .. } => "packageIndex",
            ArtifactKeyContent::ComponentGraph { .. } => "componentGraph",
            ArtifactKeyContent::ProgramAnalysis { .. } => "programAnalysis",
            ArtifactKeyContent::DirBound { .. } => "dirBound",
            ArtifactKeyContent::DirImported { .. } => "dirImported",
            ArtifactKeyContent::DirExpanded { .. } => "dirExpanded",
            ArtifactKeyContent::DirExported { .. } => "dirExported",
            ArtifactKeyContent::DirResolved { .. } => "dirResolved",
            ArtifactKeyContent::DirCheckedComponent { .. } => "dirCheckedComponent",
            ArtifactKeyContent::DirChecked { .. } => "dirChecked",
            ArtifactKeyContent::DirMaterialized { .. } => "dirMaterialized",
            ArtifactKeyContent::DirElaborated { .. } => "dirElaborated",
            ArtifactKeyContent::MirLowered { .. } => "mirLowered",
            ArtifactKeyContent::MirVerified { .. } => "mirVerified",
            ArtifactKeyContent::MirAnalyzed { .. } => "mirAnalyzed",
            ArtifactKeyContent::MirOptimized { .. } => "mirOptimized",
            ArtifactKeyContent::ModuleQueryIndex { .. } => "moduleQueryIndex",
            ArtifactKeyContent::WorkspaceQueryIndex { .. } => "workspaceQueryIndex",
            ArtifactKeyContent::Script { .. } => "script",
            ArtifactKeyContent::Object { .. } => "object",
            ArtifactKeyContent::Asset { .. } => "asset",
            ArtifactKeyContent::Bundle { .. } => "bundle",
            ArtifactKeyContent::Program { .. } => "program",
            ArtifactKeyContent::Product { .. } => "product",
            ArtifactKeyContent::ModuleLinted { .. } => "moduleLinted",
            ArtifactKeyContent::PackageLinted { .. } => "packageLinted",
            ArtifactKeyContent::WorkspaceLinted => "workspaceLinted",
        };
        label.to_string()
    }

    /// Build target.
    #[wasm_bindgen(js_name = "getTarget")]
    pub fn get_target(&self) -> Option<TargetId> {
        match &self.content {
            ArtifactKeyContent::Build { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ProgramAnalysis { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirLowered { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirVerified { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirAnalyzed { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirOptimized { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Script { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Object { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Asset { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Bundle { target: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Program { target: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Source module.
    #[wasm_bindgen(js_name = "getModule")]
    pub fn get_module(&self) -> Option<ModuleId> {
        match &self.content {
            ArtifactKeyContent::DirParsed { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Data { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirBound { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirImported { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirExpanded { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirExported { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirResolved { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirChecked { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirMaterialized { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirElaborated { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirLowered { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirVerified { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirAnalyzed { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirOptimized { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ModuleQueryIndex { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Script { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Object { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Asset { module: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ModuleLinted { module: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Semantic profile.
    #[wasm_bindgen(js_name = "getProfile")]
    pub fn get_profile(&self) -> Option<ProfileId> {
        match &self.content {
            ArtifactKeyContent::GlobalEnvironment { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::PackageIndex { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ComponentGraph { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ProgramAnalysis { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirBound { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirImported { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirExpanded { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirExported { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirResolved { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirCheckedComponent { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirChecked { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirMaterialized { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::DirElaborated { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirLowered { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirVerified { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirAnalyzed { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::MirOptimized { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ModuleQueryIndex { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::WorkspaceQueryIndex { profile: value, .. } => Some(value.clone()),
            ArtifactKeyContent::ModuleLinted { profile: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Component entry module.
    #[wasm_bindgen(js_name = "getEntry")]
    pub fn get_entry(&self) -> Option<ModuleId> {
        match &self.content {
            ArtifactKeyContent::DirCheckedComponent { entry: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Checked component id.
    #[wasm_bindgen(js_name = "getComponent")]
    pub fn get_component(&self) -> Option<ComponentId> {
        match &self.content {
            ArtifactKeyContent::DirCheckedComponent {
                component: value, ..
            } => Some(value.clone()),
            _ => None,
        }
    }

    /// Source package.
    #[wasm_bindgen(js_name = "getPackage")]
    pub fn get_package(&self) -> Option<PackageId> {
        match &self.content {
            ArtifactKeyContent::Bundle { package: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Program { package: value, .. } => Some(value.clone()),
            ArtifactKeyContent::Product { package: value, .. } => Some(value.clone()),
            ArtifactKeyContent::PackageLinted { package: value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    /// Product.
    #[wasm_bindgen(js_name = "getProduct")]
    pub fn get_product(&self) -> Option<ProductId> {
        match &self.content {
            ArtifactKeyContent::Product { product: value, .. } => Some(value.clone()),
            _ => None,
        }
    }
}

impl ArtifactKey {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactKey {
        match self.content {
            ArtifactKeyContent::Build { target } => bridge::ArtifactKey::Build {
                target: target.into_bridge(),
            },
            ArtifactKeyContent::DirParsed { module } => bridge::ArtifactKey::DirParsed {
                module: module.into_bridge(),
            },
            ArtifactKeyContent::Data { module } => bridge::ArtifactKey::Data {
                module: module.into_bridge(),
            },
            ArtifactKeyContent::GlobalEnvironment { profile } => {
                bridge::ArtifactKey::GlobalEnvironment {
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::PackageIndex { profile } => bridge::ArtifactKey::PackageIndex {
                profile: profile.into_bridge(),
            },
            ArtifactKeyContent::ComponentGraph { profile } => bridge::ArtifactKey::ComponentGraph {
                profile: profile.into_bridge(),
            },
            ArtifactKeyContent::ProgramAnalysis { profile, target } => {
                bridge::ArtifactKey::ProgramAnalysis {
                    profile: profile.into_bridge(),
                    target: target.into_bridge(),
                }
            }
            ArtifactKeyContent::DirBound { module, profile } => bridge::ArtifactKey::DirBound {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
            ArtifactKeyContent::DirImported { module, profile } => {
                bridge::ArtifactKey::DirImported {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::DirExpanded { module, profile } => {
                bridge::ArtifactKey::DirExpanded {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::DirExported { module, profile } => {
                bridge::ArtifactKey::DirExported {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::DirResolved { module, profile } => {
                bridge::ArtifactKey::DirResolved {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::DirCheckedComponent {
                entry,
                component,
                profile,
            } => bridge::ArtifactKey::DirCheckedComponent {
                entry: entry.into_bridge(),
                component: component.into_bridge(),
                profile: profile.into_bridge(),
            },
            ArtifactKeyContent::DirChecked { module, profile } => bridge::ArtifactKey::DirChecked {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
            ArtifactKeyContent::DirMaterialized { module, profile } => {
                bridge::ArtifactKey::DirMaterialized {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::DirElaborated { module, profile } => {
                bridge::ArtifactKey::DirElaborated {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::MirLowered {
                module,
                profile,
                target,
            } => bridge::ArtifactKey::MirLowered {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::MirVerified {
                module,
                profile,
                target,
            } => bridge::ArtifactKey::MirVerified {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::MirAnalyzed {
                module,
                profile,
                target,
            } => bridge::ArtifactKey::MirAnalyzed {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::MirOptimized {
                module,
                profile,
                target,
            } => bridge::ArtifactKey::MirOptimized {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::ModuleQueryIndex { module, profile } => {
                bridge::ArtifactKey::ModuleQueryIndex {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::WorkspaceQueryIndex { profile } => {
                bridge::ArtifactKey::WorkspaceQueryIndex {
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::Script { module, target } => bridge::ArtifactKey::Script {
                module: module.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::Object { module, target } => bridge::ArtifactKey::Object {
                module: module.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::Asset { module, target } => bridge::ArtifactKey::Asset {
                module: module.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::Bundle { package, target } => bridge::ArtifactKey::Bundle {
                package: package.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::Program { package, target } => bridge::ArtifactKey::Program {
                package: package.into_bridge(),
                target: target.into_bridge(),
            },
            ArtifactKeyContent::Product { package, product } => bridge::ArtifactKey::Product {
                package: package.into_bridge(),
                product: product.into_bridge(),
            },
            ArtifactKeyContent::ModuleLinted { module, profile } => {
                bridge::ArtifactKey::ModuleLinted {
                    module: module.into_bridge(),
                    profile: profile.into_bridge(),
                }
            }
            ArtifactKeyContent::PackageLinted { package } => bridge::ArtifactKey::PackageLinted {
                package: package.into_bridge(),
            },
            ArtifactKeyContent::WorkspaceLinted => bridge::ArtifactKey::WorkspaceLinted,
        }
    }
}

impl ArtifactKey {
    /// Convert one bridge payload enum into one WASM payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactKey) -> Self {
        match value {
            bridge::ArtifactKey::Build { target } => Self {
                content: ArtifactKeyContent::Build {
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::DirParsed { module } => Self {
                content: ArtifactKeyContent::DirParsed {
                    module: ModuleId::from_bridge(module),
                },
            },
            bridge::ArtifactKey::Data { module } => Self {
                content: ArtifactKeyContent::Data {
                    module: ModuleId::from_bridge(module),
                },
            },
            bridge::ArtifactKey::GlobalEnvironment { profile } => Self {
                content: ArtifactKeyContent::GlobalEnvironment {
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::PackageIndex { profile } => Self {
                content: ArtifactKeyContent::PackageIndex {
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::ComponentGraph { profile } => Self {
                content: ArtifactKeyContent::ComponentGraph {
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::ProgramAnalysis { profile, target } => Self {
                content: ArtifactKeyContent::ProgramAnalysis {
                    profile: ProfileId::from_bridge(profile),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::DirBound { module, profile } => Self {
                content: ArtifactKeyContent::DirBound {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirImported { module, profile } => Self {
                content: ArtifactKeyContent::DirImported {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirExpanded { module, profile } => Self {
                content: ArtifactKeyContent::DirExpanded {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirExported { module, profile } => Self {
                content: ArtifactKeyContent::DirExported {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirResolved { module, profile } => Self {
                content: ArtifactKeyContent::DirResolved {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Self {
                content: ArtifactKeyContent::DirCheckedComponent {
                    entry: ModuleId::from_bridge(entry),
                    component: ComponentId::from_bridge(component),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirChecked { module, profile } => Self {
                content: ArtifactKeyContent::DirChecked {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirMaterialized { module, profile } => Self {
                content: ArtifactKeyContent::DirMaterialized {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::DirElaborated { module, profile } => Self {
                content: ArtifactKeyContent::DirElaborated {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => Self {
                content: ArtifactKeyContent::MirLowered {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => Self {
                content: ArtifactKeyContent::MirVerified {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::MirAnalyzed {
                module,
                profile,
                target,
            } => Self {
                content: ArtifactKeyContent::MirAnalyzed {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => Self {
                content: ArtifactKeyContent::MirOptimized {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::ModuleQueryIndex { module, profile } => Self {
                content: ArtifactKeyContent::ModuleQueryIndex {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::WorkspaceQueryIndex { profile } => Self {
                content: ArtifactKeyContent::WorkspaceQueryIndex {
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::Script { module, target } => Self {
                content: ArtifactKeyContent::Script {
                    module: ModuleId::from_bridge(module),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::Object { module, target } => Self {
                content: ArtifactKeyContent::Object {
                    module: ModuleId::from_bridge(module),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::Asset { module, target } => Self {
                content: ArtifactKeyContent::Asset {
                    module: ModuleId::from_bridge(module),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::Bundle { package, target } => Self {
                content: ArtifactKeyContent::Bundle {
                    package: PackageId::from_bridge(package),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::Program { package, target } => Self {
                content: ArtifactKeyContent::Program {
                    package: PackageId::from_bridge(package),
                    target: TargetId::from_bridge(target),
                },
            },
            bridge::ArtifactKey::Product { package, product } => Self {
                content: ArtifactKeyContent::Product {
                    package: PackageId::from_bridge(package),
                    product: ProductId::from_bridge(product),
                },
            },
            bridge::ArtifactKey::ModuleLinted { module, profile } => Self {
                content: ArtifactKeyContent::ModuleLinted {
                    module: ModuleId::from_bridge(module),
                    profile: ProfileId::from_bridge(profile),
                },
            },
            bridge::ArtifactKey::PackageLinted { package } => Self {
                content: ArtifactKeyContent::PackageLinted {
                    package: PackageId::from_bridge(package),
                },
            },
            bridge::ArtifactKey::WorkspaceLinted => Self {
                content: ArtifactKeyContent::WorkspaceLinted,
            },
        }
    }
}
