// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ComponentId, ModuleId, PackageId, ProductId, ProfileId, TargetId};

/// External artifact key crossing bridge boundaries.
#[derive(Debug)]
#[napi(object, js_name = "ArtifactKey")]
pub struct ArtifactKey {
    /// Payload variant label.
    pub kind: String,
    /// Build target.
    pub target: Option<TargetId>,
    /// Source module.
    pub module: Option<ModuleId>,
    /// Semantic profile.
    pub profile: Option<ProfileId>,
    /// Component entry module.
    pub entry: Option<ModuleId>,
    /// Checked component id.
    pub component: Option<ComponentId>,
    /// Source package.
    pub package: Option<PackageId>,
    /// Product.
    pub product_product: Option<ProductId>,
}

impl ArtifactKey {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ArtifactKey> {
        match self.kind.as_str() {
            "build" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Build { target })
            }
            "dirParsed" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirParsed { module })
            }
            "data" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Data { module })
            }
            "globalEnvironment" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::GlobalEnvironment { profile })
            }
            "packageIndex" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::PackageIndex { profile })
            }
            "moduleIndex" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ModuleIndex { profile })
            }
            "componentGraph" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ComponentGraph { profile })
            }
            "programAnalysis" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ProgramAnalysis { profile, target })
            }
            "dirBound" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirBound { module, profile })
            }
            "dirImported" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirImported { module, profile })
            }
            "dirExpanded" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirExpanded { module, profile })
            }
            "dirExported" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirExported { module, profile })
            }
            "dirResolved" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirResolved { module, profile })
            }
            "dirCheckedComponent" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.entry else {
                    return Err(missing_payload("entry"));
                };
                let entry = value.into_bridge()?;
                let Some(value) = self.component else {
                    return Err(missing_payload("component"));
                };
                let component = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirCheckedComponent {
                    entry,
                    component,
                    profile,
                })
            }
            "dirChecked" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirChecked { module, profile })
            }
            "dirMaterialized" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirMaterialized { module, profile })
            }
            "dirElaborated" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirElaborated { module, profile })
            }
            "mirLowered" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::MirLowered {
                    module,
                    profile,
                    target,
                })
            }
            "mirVerified" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::MirVerified {
                    module,
                    profile,
                    target,
                })
            }
            "mirAnalyzed" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::MirAnalyzed {
                    module,
                    profile,
                    target,
                })
            }
            "mirOptimized" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::MirOptimized {
                    module,
                    profile,
                    target,
                })
            }
            "moduleQueryIndex" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ModuleQueryIndex { module, profile })
            }
            "workspaceQueryIndex" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::WorkspaceQueryIndex { profile })
            }
            "script" => {
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Script { module, target })
            }
            "object" => {
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Object { module, target })
            }
            "asset" => {
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Asset { module, target })
            }
            "bundle" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Bundle { package, target })
            }
            "program" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Program { package, target })
            }
            "product" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                let Some(value) = self.product_product else {
                    return Err(missing_payload("productProduct"));
                };
                let product = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Product { package, product })
            }
            "moduleLinted" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ModuleLinted { module, profile })
            }
            "packageLinted" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                Ok(bridge::ArtifactKey::PackageLinted { package })
            }
            "workspaceLinted" => {
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                if self.product_product.is_some() {
                    return Err(unexpected_payload("product_product"));
                }
                Ok(bridge::ArtifactKey::WorkspaceLinted)
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(ArtifactKey),
                self.kind
            ))),
        }
    }
}

/// Return one missing payload error.
fn missing_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is missing"))
}

/// Return one unexpected payload error.
fn unexpected_payload(kind: &str) -> napi::Error {
    napi::Error::from_reason(format!("{kind} payload is unexpected"))
}

impl ArtifactKey {
    /// Convert one bridge payload enum into one NAPI payload enum.
    pub(crate) fn from_bridge(value: bridge::ArtifactKey) -> Self {
        match value {
            bridge::ArtifactKey::Build { target } => Self {
                kind: "build".to_string(),
                target: Some(TargetId::from_bridge(target)),
                module: None,
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirParsed { module } => Self {
                kind: "dirParsed".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: None,
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::Data { module } => Self {
                kind: "data".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: None,
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::GlobalEnvironment { profile } => Self {
                kind: "globalEnvironment".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::PackageIndex { profile } => Self {
                kind: "packageIndex".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::ModuleIndex { profile } => Self {
                kind: "moduleIndex".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::ComponentGraph { profile } => Self {
                kind: "componentGraph".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::ProgramAnalysis { profile, target } => Self {
                kind: "programAnalysis".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: Some(TargetId::from_bridge(target)),
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirBound { module, profile } => Self {
                kind: "dirBound".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirImported { module, profile } => Self {
                kind: "dirImported".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirExpanded { module, profile } => Self {
                kind: "dirExpanded".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirExported { module, profile } => Self {
                kind: "dirExported".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirResolved { module, profile } => Self {
                kind: "dirResolved".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Self {
                kind: "dirCheckedComponent".to_string(),
                entry: Some(ModuleId::from_bridge(entry)),
                component: Some(ComponentId::from_bridge(component)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirChecked { module, profile } => Self {
                kind: "dirChecked".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirMaterialized { module, profile } => Self {
                kind: "dirMaterialized".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::DirElaborated { module, profile } => Self {
                kind: "dirElaborated".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => Self {
                kind: "mirLowered".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: Some(TargetId::from_bridge(target)),
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => Self {
                kind: "mirVerified".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: Some(TargetId::from_bridge(target)),
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::MirAnalyzed {
                module,
                profile,
                target,
            } => Self {
                kind: "mirAnalyzed".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: Some(TargetId::from_bridge(target)),
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => Self {
                kind: "mirOptimized".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: Some(TargetId::from_bridge(target)),
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::ModuleQueryIndex { module, profile } => Self {
                kind: "moduleQueryIndex".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::WorkspaceQueryIndex { profile } => Self {
                kind: "workspaceQueryIndex".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                module: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::Script { module, target } => Self {
                kind: "script".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: Some(TargetId::from_bridge(target)),
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::Object { module, target } => Self {
                kind: "object".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: Some(TargetId::from_bridge(target)),
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::Asset { module, target } => Self {
                kind: "asset".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: Some(TargetId::from_bridge(target)),
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::Bundle { package, target } => Self {
                kind: "bundle".to_string(),
                package: Some(PackageId::from_bridge(package)),
                target: Some(TargetId::from_bridge(target)),
                module: None,
                profile: None,
                entry: None,
                component: None,
                product_product: None,
            },
            bridge::ArtifactKey::Program { package, target } => Self {
                kind: "program".to_string(),
                package: Some(PackageId::from_bridge(package)),
                target: Some(TargetId::from_bridge(target)),
                module: None,
                profile: None,
                entry: None,
                component: None,
                product_product: None,
            },
            bridge::ArtifactKey::Product { package, product } => Self {
                kind: "product".to_string(),
                package: Some(PackageId::from_bridge(package)),
                product_product: Some(ProductId::from_bridge(product)),
                target: None,
                module: None,
                profile: None,
                entry: None,
                component: None,
            },
            bridge::ArtifactKey::ModuleLinted { module, profile } => Self {
                kind: "moduleLinted".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                target: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
            bridge::ArtifactKey::PackageLinted { package } => Self {
                kind: "packageLinted".to_string(),
                package: Some(PackageId::from_bridge(package)),
                target: None,
                module: None,
                profile: None,
                entry: None,
                component: None,
                product_product: None,
            },
            bridge::ArtifactKey::WorkspaceLinted => Self {
                kind: "workspaceLinted".to_string(),
                target: None,
                module: None,
                profile: None,
                entry: None,
                component: None,
                package: None,
                product_product: None,
            },
        }
    }
}
