use destack_source::{ComponentId, ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::{IndexMap, IndexSet};

use crate::{Compiler, CompilerError, CompilerResult};

/// Check-time view of one resolved source component.
#[derive(Debug, Clone)]
pub(in crate::check) struct CheckComponent {
    /// The stable component id.
    pub(in crate::check) id: ComponentId,
    /// Modules in this component.
    pub(in crate::check) modules: Vec<ModuleId>,
    /// Component dependencies outside the component itself.
    pub(in crate::check) dependencies: Vec<ModuleId>,
}

impl CheckComponent {
    /// Return the component id.
    pub(in crate::check) fn id(&self) -> ComponentId {
        self.id
    }

    /// Return the module used to enter this component graph.
    pub(in crate::check) fn entry(&self) -> CompilerResult<ModuleId> {
        self.modules
            .first()
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: "checked component has no modules".to_string(),
            })
    }

    /// Return the modules in this component.
    pub(in crate::check) fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Require an artifact key to name this component exactly.
    pub(in crate::check) fn require_artifact_key(
        &self,
        entry: ModuleId,
        component: ComponentId,
    ) -> CompilerResult<()> {
        if self.entry()? == entry && self.id == component {
            Ok(())
        } else {
            Err(CompilerError::Internal {
                message: format!(
                    "checked component key does not match resolved component: entry={entry:?} component={component}"
                ),
            })
        }
    }
}

impl Compiler {
    /// Discover the resolved component that contains one module.
    pub(in crate::check) fn resolve_check_component_for_module(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<CheckComponent> {
        let graph = self.load_resolved_dependency_graph(module, profile, context)?;
        let graph = CheckComponentGraph::new(graph);
        let component_modules = graph.component_modules(module);
        let id = ComponentId::from_modules(profile, component_modules.iter().copied());
        let dependencies = graph.dependencies(component_modules.as_slice());

        Ok(CheckComponent {
            id,
            modules: component_modules,
            dependencies,
        })
    }

    /// Discover and validate the component named by one artifact key.
    pub(in crate::check) fn resolve_check_component_for_key(
        &self,
        entry: ModuleId,
        component_id: ComponentId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<CheckComponent> {
        let component = self.resolve_check_component_for_module(entry, profile, context)?;

        component.require_artifact_key(entry, component_id)?;

        Ok(component)
    }

    /// Load resolved dependency edges reachable from one module.
    fn load_resolved_dependency_graph(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<IndexMap<ModuleId, Vec<ModuleId>>> {
        let artifacts = self.artifact_reader(context);
        let mut graph = IndexMap::new();
        let mut pending = vec![module];

        // load each resolved module once
        while let Some(module) = pending.pop() {
            if graph.contains_key(&module) {
                continue;
            }

            let resolved = artifacts
                .dir_resolved(module, profile)
                .map_err(CompilerError::from)?;
            let dependencies = resolved.imports.dependencies.clone();

            // schedule dependencies before publishing this node
            for dependency in dependencies.iter().rev() {
                if !graph.contains_key(dependency) {
                    pending.push(*dependency);
                }
            }

            graph.insert(module, dependencies);
        }

        Ok(graph)
    }
}

/// Resolved module dependency graph for component discovery.
struct CheckComponentGraph {
    /// Forward dependency edges keyed by source module.
    edges: IndexMap<ModuleId, Vec<ModuleId>>,
}

impl CheckComponentGraph {
    /// Create a resolved dependency graph.
    fn new(edges: IndexMap<ModuleId, Vec<ModuleId>>) -> Self {
        Self { edges }
    }

    /// Return modules in the strongly connected component containing one module.
    fn component_modules(&self, module: ModuleId) -> Vec<ModuleId> {
        let reverse_edges = self.reverse_edges();
        let reachable_to_module = self.reachable(module, &reverse_edges);
        let mut modules = Vec::new();

        // keep forward reachable modules that can also reach the module
        for module in self.edges.keys().copied() {
            if reachable_to_module.contains(&module) {
                modules.push(module);
            }
        }

        modules.sort_unstable();
        modules
    }

    /// Return outgoing dependencies from one component.
    fn dependencies(&self, component_modules: &[ModuleId]) -> Vec<ModuleId> {
        let component_modules = component_modules.iter().copied().collect::<IndexSet<_>>();
        let mut dependencies = IndexSet::new();

        // collect unique edges leaving the component
        for module in &component_modules {
            let Some(module_dependencies) = self.edges.get(module) else {
                continue;
            };

            for dependency in module_dependencies {
                if !component_modules.contains(dependency) {
                    dependencies.insert(*dependency);
                }
            }
        }

        let mut dependencies = dependencies.into_iter().collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies
    }

    /// Build reverse dependency edges for loaded modules.
    fn reverse_edges(&self) -> IndexMap<ModuleId, Vec<ModuleId>> {
        let mut reverse_edges = IndexMap::new();

        // ensure every loaded module has a reverse edge list
        for module in self.edges.keys().copied() {
            reverse_edges.entry(module).or_insert_with(Vec::new);
        }

        // reverse edges that point to loaded modules
        for (source, targets) in &self.edges {
            for target in targets {
                if self.edges.contains_key(target) {
                    reverse_edges
                        .entry(*target)
                        .or_insert_with(Vec::new)
                        .push(*source);
                }
            }
        }

        reverse_edges
    }

    /// Return modules reachable from one module through the provided edges.
    fn reachable(
        &self,
        module: ModuleId,
        edges: &IndexMap<ModuleId, Vec<ModuleId>>,
    ) -> IndexSet<ModuleId> {
        let mut reachable = IndexSet::new();
        let mut pending = vec![module];

        // walk graph without recursion
        while let Some(module) = pending.pop() {
            if !reachable.insert(module) {
                continue;
            }
            let Some(targets) = edges.get(&module) else {
                continue;
            };

            for target in targets.iter().rev() {
                if !reachable.contains(target) {
                    pending.push(*target);
                }
            }
        }

        reachable
    }
}
