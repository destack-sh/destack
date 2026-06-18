// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{Diagnostic, Module, PackageId, ProfileId};

/// Scope accepted by lint operations.
#[pyclass(name = "Scope", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Scope {
    pub(crate) value: bridge::Scope,
}

#[pymethods]
impl Scope {
    /// One module profile.
    #[staticmethod]
    pub fn module(module: Module, profile: ProfileId) -> Self {
        Self {
            value: bridge::Scope::Module {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// One source package.
    #[staticmethod]
    pub fn package(package: PackageId) -> Self {
        Self {
            value: bridge::Scope::Package {
                package: package.into_bridge(),
            },
        }
    }

    /// Whole workspace.
    #[staticmethod]
    pub fn workspace() -> Self {
        Self {
            value: bridge::Scope::Workspace,
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::Scope::Module { .. } => "module",
            bridge::Scope::Package { .. } => "package",
            bridge::Scope::Workspace => "workspace",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_module_module(&self) -> Option<Module> {
        match &self.value {
            bridge::Scope::Module { module, .. } => Some(Module::from_bridge(module.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_package_package(&self) -> Option<PackageId> {
        match &self.value {
            bridge::Scope::Package { package, .. } => Some(PackageId::from_bridge(package.clone())),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_profile(&self) -> Option<ProfileId> {
        match &self.value {
            bridge::Scope::Module { profile, .. } => Some(ProfileId::from_bridge(profile.clone())),
            _ => None,
        }
    }
}

impl Scope {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Scope {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Scope) -> Self {
        Self { value }
    }
}

/// One linter request.
#[pyclass(name = "LintRequest", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct LintRequest {
    pub(crate) value: bridge::LintRequest,
}

#[pymethods]
impl LintRequest {
    /// Create one value.
    #[new]
    pub fn new(scope: Scope) -> Self {
        Self {
            value: bridge::LintRequest {
                scope: scope.into_bridge(),
            },
        }
    }

    /// Scope to lint.
    #[getter]
    pub fn scope(&self) -> Scope {
        Scope::from_bridge(self.value.scope.clone())
    }
}

#[allow(dead_code)]
impl LintRequest {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::LintRequest {
        self.value
    }
}

/// One linter output.
#[pyclass(name = "LintOutput", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct LintOutput {
    pub(crate) value: bridge::LintOutput,
}

#[pymethods]
impl LintOutput {
    /// Create one value.
    #[new]
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            value: bridge::LintOutput {
                diagnostics: diagnostics
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
            },
        }
    }

    /// Diagnostics emitted by lint rules.
    #[getter]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.value
            .diagnostics
            .clone()
            .into_iter()
            .map(|item| Diagnostic::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl LintOutput {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::LintOutput) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Scope>()?;
    module.add_class::<LintRequest>()?;
    module.add_class::<LintOutput>()?;
    Ok(())
}
