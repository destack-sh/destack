/// Rows to render into a DIR snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirRows {
    /// Whether to render binding table rows.
    pub(super) binding: bool,
    /// Whether to render binding node scope rows.
    pub(super) binding_nodes: bool,
    /// Whether to render type table rows.
    pub(super) types: bool,
    /// Whether to render decorator table rows.
    pub(super) decorators: bool,
    /// Whether to render expression node type rows.
    pub(super) type_nodes: bool,
    /// Whether to render identifier type rows.
    pub(super) type_references: bool,
    /// Whether to render static table rows.
    pub(super) statics: bool,
    /// Whether to render conformance rows.
    pub(super) conformances: bool,
    /// Whether to render resolution table rows.
    pub(super) resolution: bool,
    /// Whether to render generic table rows.
    pub(super) generics: bool,
    /// Whether to render definition table rows.
    pub(super) definitions: bool,
    /// Whether to render coercion table rows.
    pub(super) coercion: bool,
    /// Whether to render module table rows.
    pub(super) module: bool,
    /// Whether to render resolved import table rows.
    pub(super) import: bool,
    /// Whether to render export table rows.
    pub(super) export: bool,
    /// Whether to render capture table rows.
    pub(super) capture: bool,
    /// Whether to render flow table rows.
    pub(super) flow: bool,
    /// Whether to render macro table rows.
    pub(super) macros: bool,
    /// Whether to render summary rows.
    pub(super) summaries: bool,
}

impl DirRows {
    /// Select no tables.
    pub(crate) const fn none() -> Self {
        Self {
            binding: false,
            binding_nodes: false,
            types: false,
            decorators: false,
            type_nodes: false,
            type_references: false,
            statics: false,
            conformances: false,
            resolution: false,
            generics: false,
            definitions: false,
            coercion: false,
            module: false,
            import: false,
            export: false,
            capture: false,
            flow: false,
            macros: false,
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

    /// Select module rows.
    pub(crate) const fn modules() -> Self {
        Self {
            module: true,
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

    /// Select the standard checked DIR rows.
    pub(crate) const fn checked() -> Self {
        Self {
            types: true,
            type_nodes: false,
            type_references: false,
            resolution: true,
            generics: true,
            definitions: true,
            ..Self::none()
        }
    }

    /// Include binding node scope rows.
    pub(crate) const fn with_binding_nodes(mut self) -> Self {
        self.binding = true;
        self.binding_nodes = true;
        self
    }

    /// Include flow table rows.
    pub(crate) const fn with_flows(mut self) -> Self {
        self.flow = true;
        self
    }

    /// Include conformance rows.
    pub(crate) const fn with_conformances(mut self) -> Self {
        self.conformances = true;
        self
    }

    /// Include static table rows.
    pub(crate) const fn with_statics(mut self) -> Self {
        self.statics = true;
        self
    }

    /// Include definition table rows.
    pub(crate) const fn with_definitions(mut self) -> Self {
        self.definitions = true;
        self
    }

    /// Include decorator table rows.
    pub(crate) const fn with_decorators(mut self) -> Self {
        self.decorators = true;
        self
    }

    /// Include expression node type rows.
    pub(crate) const fn with_node_types(mut self) -> Self {
        self.types = true;
        self.type_nodes = true;
        self
    }

    /// Include identifier expression type rows.
    pub(crate) const fn with_reference_types(mut self) -> Self {
        self.type_nodes = true;
        self.type_references = true;
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

    /// Include coercion table rows.
    pub(crate) const fn with_coercion(mut self) -> Self {
        self.coercion = true;
        self
    }

    /// Include capture table rows.
    pub(crate) const fn with_capture(mut self) -> Self {
        self.capture = true;
        self
    }

    /// Include summary rows.
    pub(crate) const fn with_summaries(mut self) -> Self {
        self.summaries = true;
        self
    }

    /// Return whether module rows are selected.
    pub(crate) const fn includes_dependency(self) -> bool {
        self.module
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
            || self.decorators
            || self.statics
            || self.resolution
            || self.generics
            || self.definitions
            || self.coercion
            || self.capture
    }
}
