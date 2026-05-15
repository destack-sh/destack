/// Tables to render into a DIR snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirSnapshotSet {
    /// Whether to render binding table rows.
    pub(super) binding: bool,
    /// Whether to render binding node scope rows.
    pub(super) binding_nodes: bool,
    /// Whether to render type table rows.
    pub(super) types: bool,
    /// Whether to render dependency table rows.
    pub(super) dependency: bool,
    /// Whether to render export table rows.
    pub(super) export: bool,
    /// Whether to render capture table rows.
    pub(super) capture: bool,
    /// Whether to render guard table rows.
    pub(super) guard: bool,
    /// Whether to render macro table rows.
    pub(super) macros: bool,
    /// Whether to render layout table rows.
    pub(super) layout: bool,
}

#[allow(dead_code)]
impl DirSnapshotSet {
    /// Select no tables.
    pub(crate) const fn none() -> Self {
        Self {
            binding: false,
            binding_nodes: false,
            types: false,
            dependency: false,
            export: false,
            capture: false,
            guard: false,
            macros: false,
            layout: false,
        }
    }

    /// Select only binding table rows.
    pub(crate) const fn binding() -> Self {
        Self {
            binding: true,
            ..Self::none()
        }
    }

    /// Include binding node scope rows.
    pub(crate) const fn with_binding_nodes(mut self) -> Self {
        self.binding = true;
        self.binding_nodes = true;
        self
    }

    /// Select every DIR table row family.
    pub(crate) const fn all() -> Self {
        Self {
            binding: true,
            binding_nodes: false,
            types: true,
            dependency: true,
            export: true,
            capture: true,
            guard: true,
            macros: true,
            layout: true,
        }
    }

    /// Include type table rows.
    pub(crate) const fn with_types(mut self) -> Self {
        self.types = true;
        self
    }

    /// Include dependency table rows.
    pub(crate) const fn with_dependency(mut self) -> Self {
        self.dependency = true;
        self
    }

    /// Include export table rows.
    pub(crate) const fn with_export(mut self) -> Self {
        self.export = true;
        self
    }

    /// Include capture table rows.
    pub(crate) const fn with_capture(mut self) -> Self {
        self.capture = true;
        self
    }

    /// Include guard table rows.
    pub(crate) const fn with_guard(mut self) -> Self {
        self.guard = true;
        self
    }

    /// Include macro table rows.
    pub(crate) const fn with_macros(mut self) -> Self {
        self.macros = true;
        self
    }

    /// Include layout table rows.
    pub(crate) const fn with_layout(mut self) -> Self {
        self.layout = true;
        self
    }

    /// Return whether dependency rows are selected.
    pub(crate) const fn includes_dependency(self) -> bool {
        self.dependency
    }

    /// Return whether export rows are selected.
    pub(crate) const fn includes_export(self) -> bool {
        self.export
    }
}
