// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ComponentId, ModuleId, PackageId, ProfileId, TargetId};

/// External artifact key crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactKey {
    /// Payload variant label.
    pub kind: String,
    /// Source module.
    pub module: Option<ModuleId>,
    /// Semantic profile.
    pub profile: Option<ProfileId>,
    /// Component entry module.
    pub entry: Option<ModuleId>,
    /// Checked component id.
    pub component: Option<ComponentId>,
    /// Build target.
    pub target: Option<TargetId>,
    /// Source package.
    pub package: Option<PackageId>,
}

impl ArtifactKey {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::ArtifactKey> {
        match self.kind.as_str() {
            "dirParsed" => {
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DirParsed { module })
            }
            "data" => {
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                Ok(bridge::ArtifactKey::Data { module })
            }
            "globalEnvironment" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::GlobalEnvironment { profile })
            }
            "dependencyIndex" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::DependencyIndex { profile })
            }
            "dirBound" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                let Some(value) = self.profile else {
                    return Err(missing_payload("profile"));
                };
                let profile = value.into_bridge()?;
                Ok(bridge::ArtifactKey::WorkspaceQueryIndex { profile })
            }
            "moduleOutput" => {
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
                let Some(value) = self.module else {
                    return Err(missing_payload("module"));
                };
                let module = value.into_bridge()?;
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::ModuleOutput { module, target })
            }
            "packageOutput" => {
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
                let Some(value) = self.target else {
                    return Err(missing_payload("target"));
                };
                let target = value.into_bridge()?;
                Ok(bridge::ArtifactKey::PackageOutput { package, target })
            }
            "moduleLinted" => {
                if self.entry.is_some() {
                    return Err(unexpected_payload("entry"));
                }
                if self.component.is_some() {
                    return Err(unexpected_payload("component"));
                }
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                Ok(bridge::ArtifactKey::PackageLinted { package })
            }
            "workspaceLinted" => {
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
                if self.target.is_some() {
                    return Err(unexpected_payload("target"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
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
            bridge::ArtifactKey::DirParsed { module } => Self {
                kind: "dirParsed".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::Data { module } => Self {
                kind: "data".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::GlobalEnvironment { profile } => Self {
                kind: "globalEnvironment".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                module: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DependencyIndex { profile } => Self {
                kind: "dependencyIndex".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                module: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirBound { module, profile } => Self {
                kind: "dirBound".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirImported { module, profile } => Self {
                kind: "dirImported".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirExpanded { module, profile } => Self {
                kind: "dirExpanded".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirExported { module, profile } => Self {
                kind: "dirExported".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirResolved { module, profile } => Self {
                kind: "dirResolved".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
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
                module: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirChecked { module, profile } => Self {
                kind: "dirChecked".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirMaterialized { module, profile } => Self {
                kind: "dirMaterialized".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::DirElaborated { module, profile } => Self {
                kind: "dirElaborated".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
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
            },
            bridge::ArtifactKey::ModuleQueryIndex { module, profile } => Self {
                kind: "moduleQueryIndex".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::WorkspaceQueryIndex { profile } => Self {
                kind: "workspaceQueryIndex".to_string(),
                profile: Some(ProfileId::from_bridge(profile)),
                module: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::ModuleOutput { module, target } => Self {
                kind: "moduleOutput".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                target: Some(TargetId::from_bridge(target)),
                profile: None,
                entry: None,
                component: None,
                package: None,
            },
            bridge::ArtifactKey::PackageOutput { package, target } => Self {
                kind: "packageOutput".to_string(),
                package: Some(PackageId::from_bridge(package)),
                target: Some(TargetId::from_bridge(target)),
                module: None,
                profile: None,
                entry: None,
                component: None,
            },
            bridge::ArtifactKey::ModuleLinted { module, profile } => Self {
                kind: "moduleLinted".to_string(),
                module: Some(ModuleId::from_bridge(module)),
                profile: Some(ProfileId::from_bridge(profile)),
                entry: None,
                component: None,
                target: None,
                package: None,
            },
            bridge::ArtifactKey::PackageLinted { package } => Self {
                kind: "packageLinted".to_string(),
                package: Some(PackageId::from_bridge(package)),
                module: None,
                profile: None,
                entry: None,
                component: None,
                target: None,
            },
            bridge::ArtifactKey::WorkspaceLinted => Self {
                kind: "workspaceLinted".to_string(),
                module: None,
                profile: None,
                entry: None,
                component: None,
                target: None,
                package: None,
            },
        }
    }
}
