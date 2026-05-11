use std::collections::BTreeMap;

use crate::platform::model::{BindingEntry, CatalogBindingProvider, CatalogBindingSimulation};

use super::{ModuleLayout, WorkspaceLayout};

/// One generated binding module.
#[derive(Clone)]
pub(crate) struct ModuleSpec {
    /// The platform module name.
    pub(crate) name: String,
    /// The generated and handwritten file layout.
    pub(crate) layout: ModuleLayout,
    /// Whether host routing should be emitted.
    pub(crate) has_host_dispatch: bool,
    /// Whether simulation routing should be emitted.
    pub(crate) has_simulation_dispatch: bool,
}

impl ModuleSpec {
    /// Build one generated binding module from one binding set.
    pub(crate) fn new(
        name: &str,
        bindings: &BTreeMap<String, BindingEntry>,
        workspace_layout: &WorkspaceLayout,
    ) -> Self {
        // dispatch support
        let has_host_dispatch = bindings
            .values()
            .any(|entry| entry.provider != CatalogBindingProvider::Runtime);
        let has_simulation_dispatch = bindings.values().any(|entry| {
            entry.provider != CatalogBindingProvider::Runtime
                && entry.simulation != CatalogBindingSimulation::Unsupported
        });

        Self {
            name: name.to_string(),
            layout: workspace_layout.module_layout(name),
            has_host_dispatch,
            has_simulation_dispatch,
        }
    }
}

impl ModuleSpec {
    /// Build generated module metadata for one binding catalog.
    pub(crate) fn collect(
        catalog: &BTreeMap<String, BTreeMap<String, BindingEntry>>,
        workspace_layout: &WorkspaceLayout,
    ) -> BTreeMap<String, Self> {
        let mut modules = BTreeMap::new();

        // module records
        for (name, bindings) in catalog {
            modules.insert(name.clone(), Self::new(name, bindings, workspace_layout));
        }

        modules
    }
}
