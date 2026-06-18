// generated bridge target, do not edit

use destack_bridge_language as bridge;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Diagnostic, Module, PackageId, ProfileId};

/// Scope accepted by lint operations.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct Scope {
    content: ScopeContent,
}

/// Concrete payload enum content.
#[derive(Debug, Clone)]
enum ScopeContent {
    /// One module profile.
    Module {
        /// Loaded source module.
        module: Module,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// One source package.
    Package {
        /// Source package.
        package: PackageId,
    },
    /// Whole workspace.
    Workspace,
}

#[wasm_bindgen]
impl Scope {
    /// Create one payload variant.
    #[wasm_bindgen(js_name = "module")]
    pub fn module(module: Module, profile: ProfileId) -> Self {
        Self {
            content: ScopeContent::Module { module, profile },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "package")]
    pub fn package(package: PackageId) -> Self {
        Self {
            content: ScopeContent::Package { package },
        }
    }

    /// Create one payload variant.
    #[wasm_bindgen(js_name = "workspace")]
    pub fn workspace() -> Self {
        Self {
            content: ScopeContent::Workspace,
        }
    }
}

impl Scope {
    /// Convert this WASM payload enum into one bridge enum.
    pub(crate) fn into_bridge(self) -> bridge::Scope {
        match self.content {
            ScopeContent::Module { module, profile } => bridge::Scope::Module {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
            ScopeContent::Package { package } => bridge::Scope::Package {
                package: package.into_bridge(),
            },
            ScopeContent::Workspace => bridge::Scope::Workspace,
        }
    }
}

/// One linter request.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct LintRequest {
    scope: Scope,
}

#[wasm_bindgen]
impl LintRequest {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(scope: Scope) -> Self {
        Self { scope }
    }

    /// Scope to lint.
    #[wasm_bindgen(getter, js_name = "scope")]
    pub fn scope(&self) -> Scope {
        self.scope.clone()
    }
}

impl LintRequest {
    /// Convert this WASM value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::LintRequest {
        bridge::LintRequest {
            scope: self.scope.into_bridge(),
        }
    }
}

/// One linter output.
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct LintOutput {
    diagnostics: Vec<Diagnostic>,
}

#[wasm_bindgen]
impl LintOutput {
    /// Create one value.
    #[wasm_bindgen(constructor)]
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    /// Diagnostics emitted by lint rules.
    #[wasm_bindgen(getter, js_name = "diagnostics")]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }
}

impl LintOutput {
    /// Convert one bridge value into one WASM value.
    pub(crate) fn from_bridge(value: bridge::LintOutput) -> Self {
        Self {
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(|item| Diagnostic::from_bridge(item))
                .collect(),
        }
    }
}
