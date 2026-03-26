use destack_source::ModuleId;
use destack_workspace::BundleMode;
use indexmap::IndexMap;

/// One stable output id inside one script output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ScriptOutputId(pub(crate) usize);

/// One output kind in the current script output graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptOutputKind {
    /// One statically discovered entry output.
    Entry,
    /// One asynchronously discovered entry output.
    DynamicEntry,
    /// One shared non-entry output.
    Shared,
}

/// One emitted script output node.
#[derive(Debug, Clone)]
pub(crate) struct ScriptOutputNode {
    /// The emitted output kind.
    pub(super) kind: ScriptOutputKind,
    /// The member modules carried by this output.
    pub(super) modules: Vec<ModuleId>,
    /// The facade module used for naming and manifest input metadata when one exists.
    pub(super) facade_module: Option<ModuleId>,
    /// The configured manual output name when one exists.
    pub(super) manual_name: Option<String>,
    /// The bundled outgoing static output dependencies.
    pub(super) static_output_dependencies: Vec<ScriptOutputId>,
    /// The bundled outgoing dynamic output dependencies.
    pub(super) dynamic_output_dependencies: Vec<ScriptOutputId>,
    /// The retained external static imports.
    pub(super) external_imports: Vec<String>,
    /// The retained external dynamic imports.
    pub(super) external_dynamic_imports: Vec<String>,
}

impl ScriptOutputNode {
    /// Return the emitted output kind.
    pub(crate) fn kind(&self) -> ScriptOutputKind {
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
        self.kind == ScriptOutputKind::Entry
    }

    /// Return whether this output is a dynamic entry.
    pub(crate) fn is_dynamic_entry(&self) -> bool {
        self.kind == ScriptOutputKind::DynamicEntry
    }

    /// Return the bundled outgoing static output dependencies.
    pub(crate) fn static_output_dependencies(&self) -> &[ScriptOutputId] {
        &self.static_output_dependencies
    }

    /// Return the bundled outgoing dynamic output dependencies.
    pub(crate) fn dynamic_output_dependencies(&self) -> &[ScriptOutputId] {
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

/// One output graph for one script target.
#[derive(Debug, Clone)]
pub(crate) struct ScriptOutputGraph {
    /// The bundle mode represented by this graph.
    pub(super) bundle_mode: BundleMode,
    /// The emitted outputs in stable output order.
    pub(super) outputs: Vec<ScriptOutputNode>,
    /// The output index for each source module.
    pub(super) output_ids_by_module: IndexMap<ModuleId, ScriptOutputId>,
}

impl ScriptOutputGraph {
    /// Return the bundle mode represented by this graph.
    pub(crate) fn bundle_mode(&self) -> BundleMode {
        self.bundle_mode
    }

    /// Return the emitted outputs in stable output order.
    pub(crate) fn outputs(&self) -> &[ScriptOutputNode] {
        &self.outputs
    }

    /// Return one emitted output by its stable output id when it exists.
    pub(crate) fn output(&self, output_id: ScriptOutputId) -> Option<&ScriptOutputNode> {
        self.outputs.get(output_id.0)
    }

    /// Return one output id by its source module when it exists.
    pub(crate) fn output_id_for_module(&self, module_id: ModuleId) -> Option<ScriptOutputId> {
        self.output_ids_by_module.get(&module_id).copied()
    }

    /// Return whether two modules belong to the same output.
    pub(crate) fn shares_output(&self, left: ModuleId, right: ModuleId) -> bool {
        self.output_id_for_module(left) == self.output_id_for_module(right)
    }
}
