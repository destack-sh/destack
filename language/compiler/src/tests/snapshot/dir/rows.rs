/// Rows to render into a DIR snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirRows {
    /// Whether to render binding table rows.
    pub(super) binding: bool,
    /// Whether to render binding node scope rows.
    pub(super) binding_nodes: bool,
    /// Whether to render type table rows.
    pub(super) types: bool,
    /// Whether to render identifier type rows.
    pub(super) type_references: bool,
    /// Whether to render static table rows.
    pub(super) statics: bool,
    /// Whether to render resolution table rows.
    pub(super) resolution: bool,
    /// Whether to render instance table rows.
    pub(super) instance: bool,
    /// Whether to render relation table rows.
    pub(super) relation: bool,
    /// Whether to render extension table rows.
    pub(super) extension: bool,
    /// Whether to render dependency table rows.
    pub(super) dependency: bool,
    /// Whether to render resolved import table rows.
    pub(super) import: bool,
    /// Whether to render export table rows.
    pub(super) export: bool,
    /// Whether to render capture table rows.
    pub(super) capture: bool,
    /// Whether to render macro table rows.
    pub(super) macros: bool,
    /// Whether to render layout table rows.
    pub(super) layout: bool,
    /// Whether to render summary rows.
    pub(super) summaries: bool,
}

#[allow(dead_code)]
impl DirRows {
    /// Select no tables.
    pub(crate) const fn none() -> Self {
        Self {
            binding: false,
            binding_nodes: false,
            types: false,
            type_references: false,
            statics: false,
            resolution: false,
            instance: false,
            relation: false,
            extension: false,
            dependency: false,
            import: false,
            export: false,
            capture: false,
            macros: false,
            layout: false,
            summaries: false,
        }
    }

    /// Select only binding table rows.
    pub(crate) const fn binding() -> Self {
        Self {
            binding: true,
            ..Self::none()
        }
    }

    /// Select dependency rows.
    pub(crate) const fn dependencies() -> Self {
        Self {
            dependency: true,
            ..Self::none()
        }
    }

    /// Select resolved import rows.
    pub(crate) const fn imports() -> Self {
        Self {
            import: true,
            ..Self::none()
        }
    }

    /// Select export rows.
    pub(crate) const fn exports() -> Self {
        Self {
            export: true,
            ..Self::none()
        }
    }

    /// Select macro expansion rows.
    pub(crate) const fn macros() -> Self {
        Self {
            macros: true,
            ..Self::none()
        }
    }

    /// Select the standard checked DIR rows.
    pub(crate) const fn checked() -> Self {
        Self {
            types: true,
            type_references: true,
            resolution: true,
            instance: true,
            relation: true,
            extension: true,
            capture: true,
            ..Self::none()
        }
    }

    /// Include binding node scope rows.
    pub(crate) const fn with_binding_nodes(mut self) -> Self {
        self.binding = true;
        self.binding_nodes = true;
        self
    }

    /// Include static table rows.
    pub(crate) const fn with_statics(mut self) -> Self {
        self.statics = true;
        self
    }

    /// Exclude identifier type rows.
    pub(crate) const fn without_reference_types(mut self) -> Self {
        self.type_references = false;
        self
    }

    /// Include export table rows.
    pub(crate) const fn with_export(mut self) -> Self {
        self.export = true;
        self
    }

    /// Include layout table rows.
    pub(crate) const fn with_layout(mut self) -> Self {
        self.layout = true;
        self
    }

    /// Include summary rows.
    pub(crate) const fn with_summaries(mut self) -> Self {
        self.summaries = true;
        self
    }

    /// Return whether dependency rows are selected.
    pub(crate) const fn includes_dependency(self) -> bool {
        self.dependency
    }

    /// Return whether resolved import rows are selected.
    pub(crate) const fn includes_import(self) -> bool {
        self.import
    }

    /// Return whether export rows are selected.
    pub(crate) const fn includes_export(self) -> bool {
        self.export
    }

    /// Return whether expanded DIR rows are selected.
    pub(crate) const fn includes_expanded(self) -> bool {
        self.macros
    }

    /// Return whether selected rows need semantic type labels.
    pub(crate) const fn uses_type_labels(self) -> bool {
        self.types
            || self.statics
            || self.resolution
            || self.instance
            || self.relation
            || self.capture
            || self.layout
    }
}
