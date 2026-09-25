use indexmap::{IndexMap, IndexSet};
use tspp_repository::JsOutputMode;
use tspp_source::ModuleId;

use super::super::AssetReference;

use super::OutputLayout;

/// The linked JS module set for one target.
#[derive(Debug, Clone, Default)]
pub(crate) struct ModuleSet {
    /// The discovered entry modules for this target.
    pub(super) entry_modules: Vec<ModuleId>,
    /// The ordered bundled modules included in the target.
    pub(super) modules: Vec<ModuleId>,
    /// The retained external static dependency targets.
    pub(super) external_targets: IndexSet<String>,
    /// The retained dynamic import targets.
    pub(super) dynamic_targets: IndexSet<String>,
    /// Whether the set contains dynamic imports without static targets.
    pub(super) has_opaque_dynamic_imports: bool,
}

/// One stable output id inside one JS output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct OutputId(pub(crate) usize);

/// One output kind in the current JS output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OutputKind {
    /// One statically discovered entry output.
    Entry,
    /// One asynchronously discovered entry output.
    DynamicEntry,
    /// One shared non-entry output.
    Shared,
}

/// One emitted JS output.
#[derive(Debug, Clone)]
pub(crate) struct Output {
    /// The emitted output kind.
    pub(super) kind: OutputKind,
    /// The member modules carried by this output.
    pub(super) modules: Vec<ModuleId>,
    /// The facade module used for naming and manifest input metadata when one exists.
    pub(super) facade_module: Option<ModuleId>,
    /// The configured manual output name when one exists.
    pub(super) manual_name: Option<String>,
    /// The bundled outgoing static output dependencies.
    pub(super) static_output_dependencies: Vec<OutputId>,
    /// The bundled outgoing dynamic output dependencies.
    pub(super) dynamic_output_dependencies: Vec<OutputId>,
    /// The retained external static imports.
    pub(super) external_imports: Vec<String>,
    /// The retained external dynamic imports.
    pub(super) external_dynamic_imports: Vec<String>,
}

/// One output graph for one JS target.
#[derive(Debug, Clone)]
pub(crate) struct OutputGraph {
    /// The bundle mode represented by this graph.
    pub(super) bundle_mode: JsOutputMode,
    /// The emitted outputs in stable output order.
    pub(super) outputs: Vec<Output>,
    /// The output index for each source module.
    pub(super) output_ids_by_module: IndexMap<ModuleId, OutputId>,
}

/// The planned asset references keyed by source module.
pub(crate) type AssetReferenceMap = IndexMap<ModuleId, AssetReference>;

/// One authoritative output plan for one JS target.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    /// The linked JS modules for this target.
    module_set: ModuleSet,
    /// The JS output graph for this target.
    output_graph: OutputGraph,
    /// The output placement for the JS outputs.
    output_layout: OutputLayout,
    /// The asset reference map.
    asset_reference_map: AssetReferenceMap,
}

impl ModuleSet {
    /// Return the discovered entry modules for this module set.
    pub(crate) fn entry_modules(&self) -> &[ModuleId] {
        &self.entry_modules
    }

    /// Return the first discovered entry module when one exists.
    pub(crate) fn first_entry_module(&self) -> Option<ModuleId> {
        self.entry_modules.first().copied()
    }

    /// Return the ordered internal modules for this module set.
    pub(crate) fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return the retained external targets for this module set.
    pub(crate) fn external_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.external_targets.iter()
    }

    /// Return the retained dynamic targets for this module set.
    pub(crate) fn dynamic_targets(&self) -> indexmap::set::Iter<'_, String> {
        self.dynamic_targets.iter()
    }
}

impl Output {
    /// Return the emitted output kind.
    pub(crate) fn kind(&self) -> OutputKind {
        self.kind
    }

    /// Return the member modules carried by this output.
    pub(crate) fn modules(&self) -> &[ModuleId] {
        &self.modules
    }

    /// Return the facade module carried by this output when one exists.
    pub(crate) fn facade_module(&self) -> Option<ModuleId> {
        self.facade_module
    }

    /// Return the configured manual output name when one exists.
    pub(crate) fn manual_name(&self) -> Option<&str> {
        self.manual_name.as_deref()
    }

    /// Return whether this output is a static entry.
    pub(crate) fn is_entry(&self) -> bool {
        self.kind == OutputKind::Entry
    }

    /// Return whether this output is a dynamic entry.
    pub(crate) fn is_dynamic_entry(&self) -> bool {
        self.kind == OutputKind::DynamicEntry
    }

    /// Return the bundled outgoing static output dependencies.
    pub(crate) fn static_output_dependencies(&self) -> &[OutputId] {
        &self.static_output_dependencies
    }

    /// Return the bundled outgoing dynamic output dependencies.
    pub(crate) fn dynamic_output_dependencies(&self) -> &[OutputId] {
        &self.dynamic_output_dependencies
    }

    /// Return the retained external static imports.
    pub(crate) fn external_imports(&self) -> &[String] {
        &self.external_imports
    }

    /// Return the retained external dynamic imports.
    pub(crate) fn external_dynamic_imports(&self) -> &[String] {
        &self.external_dynamic_imports
    }
}

impl OutputGraph {
    /// Build one empty output graph for targets without planned JS outputs.
    pub(crate) fn default_empty(bundle_mode: JsOutputMode) -> Self {
        Self {
            bundle_mode,
            outputs: Vec::new(),
            output_ids_by_module: IndexMap::new(),
        }
    }

    /// Return the bundle mode represented by this graph.
    pub(crate) fn bundle_mode(&self) -> JsOutputMode {
        self.bundle_mode
    }

    /// Return the emitted outputs in stable output order.
    pub(crate) fn outputs(&self) -> &[Output] {
        &self.outputs
    }

    /// Return one emitted output by its stable output id when it exists.
    pub(crate) fn output(&self, output_id: OutputId) -> Option<&Output> {
        self.outputs.get(output_id.0)
    }

    /// Return one output id by its source module when it exists.
    pub(crate) fn output_id_for_module(&self, module_id: ModuleId) -> Option<OutputId> {
        self.output_ids_by_module.get(&module_id).copied()
    }

    /// Return whether two modules belong to the same output.
    pub(crate) fn shares_output(&self, left: ModuleId, right: ModuleId) -> bool {
        self.output_id_for_module(left) == self.output_id_for_module(right)
    }
}

impl Plan {
    /// Create one output plan.
    pub(super) fn new(
        module_set: ModuleSet,
        output_graph: OutputGraph,
        output_layout: OutputLayout,
        asset_reference_map: AssetReferenceMap,
    ) -> Self {
        Self {
            module_set,
            output_graph,
            output_layout,
            asset_reference_map,
        }
    }

    /// Return the linked JS modules for this target.
    pub(crate) fn module_set(&self) -> &ModuleSet {
        &self.module_set
    }

    /// Return the JS output graph for this target.
    pub(crate) fn output_graph(&self) -> &OutputGraph {
        &self.output_graph
    }

    /// Return the output placement for the JS outputs.
    pub(crate) fn output_layout(&self) -> &OutputLayout {
        &self.output_layout
    }

    /// Return the asset reference map.
    pub(crate) fn asset_reference_map(&self) -> &AssetReferenceMap {
        &self.asset_reference_map
    }
}
