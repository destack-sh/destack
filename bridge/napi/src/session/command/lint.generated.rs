// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{Diagnostic, Module, PackageId, ProfileId};

/// Scope accepted by lint operations.
#[derive(Debug)]
#[napi(object, js_name = "Scope")]
pub struct Scope {
    /// Payload variant label.
    pub kind: String,
    /// Loaded source module.
    pub module: Option<Module>,
    /// Semantic profile.
    pub profile: Option<ProfileId>,
    /// Source package.
    pub package: Option<PackageId>,
}

impl Scope {
    /// Convert this NAPI payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::Scope> {
        match self.kind.as_str() {
            "module" => {
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
                Ok(bridge::Scope::Module { module, profile })
            }
            "package" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                let Some(value) = self.package else {
                    return Err(missing_payload("package"));
                };
                let package = value.into_bridge()?;
                Ok(bridge::Scope::Package { package })
            }
            "workspace" => {
                if self.module.is_some() {
                    return Err(unexpected_payload("module"));
                }
                if self.profile.is_some() {
                    return Err(unexpected_payload("profile"));
                }
                if self.package.is_some() {
                    return Err(unexpected_payload("package"));
                }
                Ok(bridge::Scope::Workspace)
            }
            _ => Err(napi::Error::from_reason(format!(
                "unknown {}: {}",
                stringify!(Scope),
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

/// One linter request.
#[derive(Debug)]
#[napi(object, js_name = "LintRequest")]
pub struct LintRequest {
    /// Scope to lint.
    pub scope: Scope,
}

impl LintRequest {
    /// Convert this NAPI value into one bridge value.
    pub(crate) fn into_bridge(self) -> napi::Result<bridge::LintRequest> {
        Ok(bridge::LintRequest {
            scope: self.scope.into_bridge()?,
        })
    }
}

/// One linter output.
#[derive(Debug)]
#[napi(object, js_name = "LintOutput")]
pub struct LintOutput {
    /// Diagnostics emitted by lint rules.
    pub diagnostics: Vec<Diagnostic>,
}

impl LintOutput {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::LintOutput) -> Self {
        Self {
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(Diagnostic::from_bridge)
                .collect(),
        }
    }
}
