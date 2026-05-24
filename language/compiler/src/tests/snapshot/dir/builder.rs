use std::collections::BTreeMap;
use std::fmt::Debug;

use destack_artifact::{
    DirBound, DirCheckedModule, DirExpanded, DirExported, DirImported, DirResolved,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::name::BindingSnapshotName;
use super::rows::DirRows;
use crate::tests::snapshot::render::SnapshotRenderer;
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

/// Add snapshot rows for one table.
pub(crate) trait SnapshotTable {
    /// Add snapshot rows to the builder.
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>);
}

/// Source overlay builder for DIR table snapshots.
pub(crate) struct DirSnapshotBuilder<'a> {
    /// The original source text.
    pub(super) source: &'a str,
    /// The parsed DIR tree.
    pub(super) tree: &'a dir::Tree,
    /// The string pool used by DIR ids.
    pub(super) strings: &'a StringPool,
    /// The binding table used for human-readable symbol labels.
    pub(super) bindings: Option<&'a dir::BindingTable<'a>>,
    /// The checked generic table.
    pub(super) generics: Option<dir::GenericTable<'static>>,
    /// The visible type table used by layout anchors.
    pub(super) types: Option<dir::TypeTable<'static>>,
    /// The visible static table used by type labels.
    pub(super) statics: Option<dir::StaticTable<'static>>,
    /// Module paths used in multi-module snapshots.
    pub(super) module_path_by_id: Option<&'a BTreeMap<ModuleId, String>>,
    /// Foreign symbol labels keyed by module and local symbol.
    pub(super) foreign_symbol_labels: BTreeMap<ModuleId, BTreeMap<dir::LocalSymbolId, String>>,
    /// Semantic type labels keyed by local type id.
    pub(super) type_labels: BTreeMap<dir::LocalTypeId, String>,
    /// Semantic static labels keyed by local static id.
    pub(super) static_labels: BTreeMap<dir::LocalStaticId, String>,
    /// Whether to render dense binding node rows.
    pub(super) binding_nodes: bool,
    /// Whether to render identifier type rows.
    pub(super) type_references: bool,
    /// Whether to render table summary rows.
    summaries: bool,
    /// The rows collected so far.
    rows: Vec<SnapshotRow>,
}

impl<'a> DirSnapshotBuilder<'a> {
    /// Create a DIR snapshot builder.
    pub(crate) fn new(source: &'a str, tree: &'a dir::Tree, strings: &'a StringPool) -> Self {
        Self {
            source,
            tree,
            strings,
            bindings: None,
            generics: None,
            types: None,
            statics: None,
            module_path_by_id: None,
            foreign_symbol_labels: BTreeMap::new(),
            type_labels: BTreeMap::new(),
            static_labels: BTreeMap::new(),
            binding_nodes: false,
            type_references: false,
            summaries: true,
            rows: Vec::new(),
        }
    }

    /// Set the binding table used for symbol labels.
    pub(crate) fn with_bindings(mut self, bindings: &'a dir::BindingTable<'a>) -> Self {
        self.bindings = Some(bindings);
        self
    }

    /// Set module paths used for module target rendering.
    pub(crate) fn with_module_paths(
        mut self,
        module_path_by_id: &'a BTreeMap<ModuleId, String>,
    ) -> Self {
        self.module_path_by_id = Some(module_path_by_id);
        self
    }

    /// Set binding tables used for foreign symbol labels.
    pub(crate) fn with_foreign_bindings(
        mut self,
        foreign_bindings: Vec<dir::BindingTable<'a>>,
    ) -> Self {
        for bindings in &foreign_bindings {
            let names = BindingSnapshotName::new(bindings, None, self.strings);
            let labels = bindings
                .symbol_ids()
                .map(|symbol_id| (symbol_id, names.symbol_path(symbol_id)))
                .collect::<BTreeMap<_, _>>();

            self.foreign_symbol_labels
                .insert(bindings.module_id, labels);
        }

        self
    }

    /// Add rows for one table.
    pub(crate) fn add_table<T>(&mut self, table: &T)
    where
        T: SnapshotTable,
    {
        table.add_snapshot_rows(self);
    }

    /// Add selected rows for a bound DIR artifact.
    pub(crate) fn add_bound(&mut self, selection: DirRows, bound: &DirBound) {
        self.binding_nodes = selection.binding_nodes;
        self.summaries = selection.summaries;

        if selection.types {
            let types = dir::TypeTable::from_segment(bound.types.clone());
            let statics = dir::StaticTable::from_segment(bound.statics.clone());
            self.types = Some(types.clone());
            self.statics = Some(statics.clone());
            self.add_static_labels(&statics);
            self.add_type_labels(&types);
        }

        if selection.binding {
            self.add_table(bound.bindings.as_ref());
        }

        if selection.types {
            self.add_table(bound.types.as_ref());
        }
    }

    /// Add selected rows for an imported DIR artifact.
    pub(crate) fn add_imported(&mut self, selection: DirRows, imported: &DirImported) {
        self.summaries = selection.summaries;

        if selection.module {
            self.add_table(imported.modules.as_ref());
        }
    }

    /// Add selected rows for a resolved DIR artifact.
    pub(crate) fn add_resolved(&mut self, selection: DirRows, resolved: &DirResolved) {
        self.summaries = selection.summaries;

        if selection.import {
            self.add_table(&resolved.imports);
        }
    }

    /// Add selected rows for an expanded DIR artifact.
    pub(crate) fn add_expanded(&mut self, selection: DirRows, expanded: &DirExpanded) {
        self.summaries = selection.summaries;

        if selection.module {
            self.add_table(expanded.modules.as_ref());
        }

        if selection.macros {
            self.add_table(&expanded.macros);
        }
    }

    /// Add selected rows for an exported DIR artifact.
    pub(crate) fn add_exported(&mut self, selection: DirRows, exported: &DirExported) {
        self.summaries = selection.summaries;

        if selection.export {
            self.add_table(&exported.exports);
            self.add_table(&exported.globals);
        }
    }

    /// Add selected rows for a checked DIR artifact.
    pub(crate) fn add_checked(
        &mut self,
        selection: DirRows,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
    ) {
        self.summaries = selection.summaries;
        self.type_references = selection.type_references;

        if selection.uses_type_labels() {
            self.generics = Some(dir::GenericTable::from_segment(checked.generics.clone()));

            let types = dir::TypeTable::from_segments(vec![
                bound.types.clone(),
                expanded.types.clone(),
                checked.types.clone(),
            ]);
            let statics = dir::StaticTable::from_segments(vec![
                bound.statics.clone(),
                expanded.statics.clone(),
                checked.statics.clone(),
            ]);
            self.types = Some(types.clone());
            self.statics = Some(statics.clone());
            self.add_static_labels(&statics);
            self.add_type_labels(&types);
        }

        if selection.types {
            self.add_table(checked.types.as_ref());
        }

        if selection.statics {
            self.add_table(checked.statics.as_ref());
        }

        if selection.resolution {
            self.add_table(checked.resolutions.as_ref());
        }

        if selection.instance {
            self.add_table(checked.generics.as_ref());
        }

        if selection.relation {
            self.add_table(checked.relations.as_ref());
        }

        if selection.coercion {
            self.add_table(checked.coercions.as_ref());
        }

        if selection.extension {
            self.add_table(checked.extensions.as_ref());
        }

        if selection.layout {
            self.add_table(checked.layouts.as_ref());
        }

        if selection.capture {
            self.add_table(checked.captures.as_ref());
        }
    }

    /// Add one row.
    pub(crate) fn push(&mut self, row: SnapshotRow) {
        if row.tag.entry == "summary" && !self.summaries {
            return;
        }

        self.rows.push(row);
    }

    /// Return the debug label for one checked generic instance.
    pub(super) fn generic_instance_label(&self, instance_id: dir::LocalInstanceId) -> String {
        let Some(generics) = &self.generics else {
            panic!("dir snapshot missing generic table for {instance_id:?}");
        };

        // render the solved semantic application
        let instance = generics.get_instance(instance_id);
        let symbol = self.symbol_path_label(instance.symbol);
        if instance.arguments.is_empty() {
            return symbol;
        }

        let arguments = instance
            .arguments
            .iter()
            .map(|argument| self.static_argument_label(argument))
            .collect::<Vec<_>>()
            .join(", ");

        format!("{symbol}<{arguments}>")
    }

    /// Return the debug label for one checked generic slot key.
    pub(super) fn generic_slot_key_label(&self, key: dir::GenericSlotKey) -> String {
        match key {
            dir::GenericSlotKey::Symbol(symbol) => self.symbol_path_label(symbol),
            dir::GenericSlotKey::Generated(name) => self.strings.get(name).to_string(),
        }
    }

    /// Return the source anchor for one DIR node.
    pub(crate) fn anchor_node(&self, node_id: dir::GlobalNodeIdAny) -> SnapshotAnchor {
        if let Some(span) = self.tree.get_span_by_id(node_id.local_id.id) {
            SnapshotAnchor::After(span)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Return the source anchor for one symbol.
    pub(crate) fn anchor_symbol(&self, symbol_id: dir::GlobalSymbolId) -> SnapshotAnchor {
        let bindings = self.binding_table();
        assert_eq!(
            symbol_id.module_id, bindings.module_id,
            "dir snapshot cannot anchor foreign symbols"
        );

        let symbol = self.symbol(symbol_id.local_id);
        if let Some(node_id) = symbol.declaration {
            self.anchor_node(node_id)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Return the source anchor for one type.
    pub(crate) fn anchor_type(&self, type_id: dir::LocalTypeId) -> SnapshotAnchor {
        let Some(types) = &self.types else {
            return SnapshotAnchor::End;
        };

        let node_id = types.get_type_source(type_id).into_global(types.module_id);

        self.anchor_node(node_id)
    }

    /// Return the source anchor for one scope.
    pub(super) fn anchor_scope(
        &self,
        module_id: ModuleId,
        scope_id: dir::LocalScopeId,
        scope: &dir::Scope,
        node_scopes: &[(dir::GlobalNodeIdAny, dir::LocalScope)],
    ) -> SnapshotAnchor {
        if let Some(owner) = scope.owner {
            let symbol_id = owner.into_global(module_id);
            return self.anchor_symbol(symbol_id);
        }

        let end_scope = dir::LocalScope::new(scope_id, dir::LocalScopeMark::end());
        let first_node = node_scopes
            .iter()
            .find(|(_, scope)| *scope == end_scope)
            .or_else(|| node_scopes.iter().find(|(_, scope)| scope.id == scope_id))
            .map(|(node_id, _)| *node_id);

        if let Some(node_id) = first_node {
            self.anchor_node(node_id)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Render the annotated source snapshot.
    pub(crate) fn render(mut self) -> String {
        SnapshotRenderer::sort_rows(&mut self.rows);

        SnapshotRenderer::new(self.source, &self.rows).render()
    }

    /// Render one symbol id using its source name when possible.
    pub(crate) fn symbol_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        if symbol_id.module_id != self.tree.module_id {
            return self.foreign_symbol_label(symbol_id);
        }

        self.binding_names().symbol(symbol_id.local_id)
    }

    /// Render one semantic symbol path.
    pub(crate) fn symbol_path_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        if symbol_id.module_id != self.tree.module_id {
            return self.foreign_symbol_label(symbol_id);
        }

        self.binding_names().symbol_path(symbol_id.local_id)
    }

    /// Render one type id using semantic type text when possible.
    pub(crate) fn type_label(&self, type_id: dir::LocalTypeId) -> String {
        self.type_labels
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| panic!("dir snapshot missing type label for {type_id:?}"))
    }

    /// Render one static id using semantic static text when possible.
    pub(crate) fn static_label(&self, static_id: dir::LocalStaticId) -> String {
        self.static_labels
            .get(&static_id)
            .cloned()
            .unwrap_or_else(|| panic!("dir snapshot missing static label for {static_id:?}"))
    }

    /// Render a type value stored in a static term.
    fn static_type_label(&self, type_id: dir::LocalTypeId) -> String {
        if let Some(label) = self.type_labels.get(&type_id) {
            return label.clone();
        }
        let types = self
            .types
            .as_ref()
            .unwrap_or_else(|| panic!("dir snapshot missing type table for {type_id:?}"));

        self.type_table_label(types, type_id)
    }

    /// Render one local symbol id using its source name when possible.
    pub(crate) fn local_symbol_label(&self, symbol_id: dir::LocalSymbolId) -> String {
        self.binding_names().symbol(symbol_id)
    }

    /// Render one local scope id.
    pub(crate) fn scope_label(&self, scope_id: dir::LocalScopeId) -> String {
        self.binding_names().scope(scope_id)
    }

    /// Render one global scope id.
    pub(crate) fn global_scope_label(&self, scope_id: dir::GlobalScopeId) -> String {
        if scope_id.module_id == self.tree.module_id {
            return self.scope_label(scope_id.local_id);
        }

        let module = self.module_label(scope_id.module_id);

        format!("{module}.scope{}", scope_id.local_id.0)
    }

    /// Render one local capture frame id.
    pub(crate) fn capture_frame_label(&self, frame_id: dir::LocalCaptureFrameId) -> String {
        let module = self.module_label(self.tree.module_id);

        format!("{module}.<frame{}>", frame_id.0)
    }

    /// Render one local scope cursor.
    pub(crate) fn scope_cursor_label(&self, scope: dir::LocalScope) -> String {
        let scope_id = self.scope_label(scope.id);
        let mark = if scope.mark == dir::LocalScopeMark::end() {
            "end".to_string()
        } else {
            scope.mark.0.to_string()
        };

        format!("{scope_id}@{mark}")
    }

    /// Render one optional local symbol id.
    pub(super) fn optional_local_symbol_label(
        &self,
        symbol_id: Option<dir::LocalSymbolId>,
    ) -> Option<String> {
        symbol_id.map(|symbol_id| self.local_symbol_label(symbol_id))
    }

    /// Render one optional local scope cursor.
    pub(super) fn optional_scope_label(&self, scope: Option<dir::LocalScope>) -> Option<String> {
        scope.map(|scope| self.scope_cursor_label(scope))
    }

    /// Render one static key.
    pub(crate) fn static_key(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings.get(name).to_string(),
            dir::StaticKey::Number(name) => format!("#number({})", self.strings.get(name)),
            dir::StaticKey::Symbol(symbol) => symbol.debug_string(self.strings),
        }
    }

    /// Render one enum variant label as lower snake case.
    pub(crate) fn variant_label<T>(value: T) -> String
    where
        T: Debug,
    {
        let debug = format!("{value:?}");

        Self::lower_snake(&debug)
    }

    /// Render one layout shape label.
    pub(crate) fn layout_shape_label(shape: &dir::LayoutShape) -> String {
        match shape {
            dir::LayoutShape::None => "none".to_string(),
            dir::LayoutShape::Scalar => "scalar".to_string(),
            dir::LayoutShape::Dynamic => "dynamic".to_string(),
            dir::LayoutShape::Struct(_) => "struct".to_string(),
            dir::LayoutShape::Tuple(_) => "tuple".to_string(),
            dir::LayoutShape::Variant(_) => "variant".to_string(),
            dir::LayoutShape::Newtype(_) => "newtype".to_string(),
            dir::LayoutShape::Function => "function".to_string(),
        }
    }

    /// Render one optional integer label.
    pub(crate) fn optional_u32_label(value: Option<u32>) -> Option<String> {
        value.map(|value| value.to_string())
    }

    /// Return one module target field.
    pub(crate) fn module_target_field(&self, target: Option<ModuleId>) -> (&'static str, String) {
        match target {
            Some(module_id) => ("module", self.module_path(module_id)),
            None => ("target", "<unresolved>".to_string()),
        }
    }

    /// Render one member candidate label.
    pub(crate) fn member_candidate_label(&self, candidate: &dir::MemberCandidate) -> String {
        self.symbol_path_label(candidate.symbol)
    }

    /// Render one call candidate label.
    pub(crate) fn call_candidate_label(&self, candidate: &dir::CallCandidate) -> String {
        self.symbol_path_label(candidate.symbol)
    }

    /// Render one static argument label.
    pub(crate) fn static_argument_label(&self, argument: &dir::StaticArgument) -> String {
        // render the argument value before adding an optional name
        let value = self.static_argument_value_label(argument.value);
        if let Some(name) = argument.name {
            format!("{}={value}", self.strings.get(name))
        } else {
            value
        }
    }

    /// Render one static argument value.
    fn static_argument_value_label(&self, static_id: dir::LocalStaticId) -> String {
        if let Some(label) = self.static_labels.get(&static_id) {
            return label.clone();
        }
        let statics = self
            .statics
            .as_ref()
            .unwrap_or_else(|| panic!("dir snapshot missing static table for {static_id:?}"));
        let term = statics.get_static(static_id);

        self.static_term_label(term)
    }

    /// Render one static term label.
    pub(crate) fn static_term_label(&self, term: &dir::StaticTerm) -> String {
        match term {
            dir::StaticTerm::Symbol { symbol } => self.symbol_path_label(*symbol),
            dir::StaticTerm::Access { access } => {
                Self::string_literal_label(&Self::variant_label(access))
            }
            dir::StaticTerm::Space { space } => {
                Self::string_literal_label(&Self::variant_label(space))
            }
            dir::StaticTerm::Place { place } => Self::place_label(place),
            dir::StaticTerm::Lifetime { lifetime } => self.lifetime_label(lifetime),
            dir::StaticTerm::ScalarLiteral { value } => self.scalar_literal_label(value),
            dir::StaticTerm::TypeLiteral { value } => Self::variant_label(value),
            dir::StaticTerm::Declaration {
                declaration,
                generic_arguments,
            } => self.static_declaration_label(*declaration, generic_arguments.as_deref()),
            dir::StaticTerm::Type { ty } => self.static_type_label(*ty),
            dir::StaticTerm::Array { elements } => {
                // render array elements recursively
                let elements = elements
                    .iter()
                    .map(|element| self.static_term_label(element))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("[{elements}]")
            }
            dir::StaticTerm::FixedArray { value, length } => {
                // render repeated fixed array syntax
                let value = self.static_term_label(value);
                let length = self.static_term_label(length);

                format!("[{value}; {length}]")
            }
            dir::StaticTerm::Tuple { elements } => {
                // render tuple elements recursively
                let elements = elements
                    .iter()
                    .map(|element| self.static_term_label(element))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("({elements})")
            }
            dir::StaticTerm::Object { properties } => {
                // render object properties recursively
                let properties = self.static_property_labels(properties);

                format!("{{{properties}}}")
            }
            dir::StaticTerm::Struct { ty, properties } => {
                // render typed struct literal syntax
                let properties = self.static_property_labels(properties);

                format!("{} {{{properties}}}", self.type_label(*ty))
            }
        }
    }

    /// Render one scalar literal label.
    pub(crate) fn scalar_literal_label(&self, literal: &dir::ScalarLiteral) -> String {
        match literal {
            dir::ScalarLiteral::Null => "null".to_string(),
            dir::ScalarLiteral::Boolean(value) => value.to_string(),
            dir::ScalarLiteral::Integer(value) => value.to_string(),
            dir::ScalarLiteral::Bigint(value) => format!("{value}n"),
            dir::ScalarLiteral::Float(value) => value.to_string(),
            dir::ScalarLiteral::Character(value) => format!("'{value}'"),
            dir::ScalarLiteral::String(value) => format!("{:?}", self.strings.get(*value)),
            dir::ScalarLiteral::RegexString { content, flags } => {
                let flags = flags.map(|flags| self.strings.get(flags)).unwrap_or("");

                format!("/{}/{flags}", self.strings.get(*content))
            }
        }
    }

    /// Render one export key label.
    pub(crate) fn export_key_label(&self, key: dir::ExportKey) -> String {
        match key {
            dir::ExportKey::Default => "<default>".to_string(),
            dir::ExportKey::Named(name) => self.static_key(name),
        }
    }

    /// Render one export selector label.
    pub(crate) fn export_selector_label(&self, selector: dir::ExportSelector) -> String {
        match selector {
            dir::ExportSelector::Default => "<default>".to_string(),
            dir::ExportSelector::Named(name) => self.static_key(name),
            dir::ExportSelector::Namespace => "<namespace>".to_string(),
        }
    }

    /// Render one macro trigger label.
    pub(crate) fn macro_trigger_label(&self, trigger: &dir::MacroTrigger) -> String {
        match trigger {
            dir::MacroTrigger::Decorator(node_id) => {
                // render the decorator node kind as the trigger
                let node_id = node_id.clone().into_any();

                format!("decorator:{}", self.node_label(node_id))
            }
            dir::MacroTrigger::AutoDerive => "auto_derive".to_string(),
        }
    }

    /// Render one declaration reference.
    pub(crate) fn declaration_label(
        &self,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> String {
        // resolve declarations through the binding table
        let declaration = declaration.into_global_any(self.tree.module_id);
        if let Some(symbol_id) = self.binding_table().symbol_for_declaration(declaration) {
            let symbol_id = symbol_id.into_global(self.tree.module_id);

            return self.symbol_path_label(symbol_id);
        }

        // fall back to the node kind for synthetic declarations
        self.node_label(declaration)
    }

    /// Render one node id.
    pub(crate) fn node_label(&self, node_id: dir::GlobalNodeIdAny) -> String {
        node_id.local_id.ty.name().replace(' ', "_")
    }

    /// Return whether to render one checked type node row.
    pub(crate) fn should_render_type_node(&self, node_id: dir::GlobalNodeIdAny) -> bool {
        if node_id.module_id != self.tree.module_id {
            return false;
        }

        if node_id.local_id.ty != dir::NodeType::Expression {
            return false;
        }

        let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.local_id.id);
        let expression = self.tree.get(expression_id);

        self.type_references || !expression.is_reference()
    }

    /// Render source text for one node when it is compact enough for a row.
    pub(crate) fn node_source(&self, node_id: dir::GlobalNodeIdAny) -> Option<String> {
        let span = self.tree.get_span_by_id(node_id.local_id.id)?;
        let source = &self.source[span.start as usize..span.end as usize];
        let source = source.trim();

        if source.is_empty() || source.contains('\n') || source.len() > 40 {
            return None;
        }

        Some(source.to_string())
    }

    /// Render one module path.
    pub(crate) fn module_path(&self, module_id: ModuleId) -> String {
        let Some(module_path_by_id) = self.module_path_by_id else {
            return module_id.to_string();
        };

        module_path_by_id
            .get(&module_id)
            .cloned()
            .unwrap_or_else(|| panic!("missing snapshot module path for {module_id}"))
    }

    /// Add semantic type labels for one visible type table.
    fn add_type_labels(&mut self, types: &dir::TypeTable<'_>) {
        for type_id in types.iter_type_ids() {
            let label = self.type_table_label(types, type_id);
            self.type_labels.insert(type_id, label);
        }
    }

    /// Add semantic static labels for one visible static table.
    fn add_static_labels(&mut self, statics: &dir::StaticTable<'_>) {
        for static_id in statics.iter_static_ids() {
            let term = statics.get_static(static_id);
            let label = self.static_term_label(term);
            self.static_labels.insert(static_id, label);
        }
    }

    /// Render one static declaration term label.
    fn static_declaration_label(
        &self,
        declaration: dir::LocalNodeId<dir::Declaration>,
        generic_arguments: Option<&[dir::StaticArgument]>,
    ) -> String {
        // render declaration references with applied static arguments
        let declaration = self.declaration_label(declaration);
        if let Some(arguments) = generic_arguments {
            let arguments = arguments
                .iter()
                .map(|argument| self.static_argument_label(argument))
                .collect::<Vec<_>>()
                .join(", ");

            format!("{declaration}<{arguments}>")
        } else {
            declaration
        }
    }

    /// Render one normalized place label.
    fn place_label(place: &dir::Place) -> String {
        let value = match place {
            dir::Place::Ambient => "ambient".to_string(),
            dir::Place::Space(space) => Self::variant_label(space),
        };

        Self::string_literal_label(&value)
    }

    /// Render one static string literal label.
    fn string_literal_label(value: &str) -> String {
        format!("{value:?}")
    }

    /// Render one normalized lifetime label.
    fn lifetime_label(&self, lifetime: &dir::Lifetime) -> String {
        match lifetime {
            dir::Lifetime::Static => "static".to_string(),
            dir::Lifetime::Symbol(symbol) => self.symbol_path_label(*symbol),
            dir::Lifetime::Generated(name) => self.strings.get(*name).to_string(),
            dir::Lifetime::Join(elements) => elements
                .iter()
                .map(|element| self.static_label(*element))
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

    /// Render static property labels.
    fn static_property_labels(&self, properties: &[dir::StaticProperty]) -> String {
        properties
            .iter()
            .map(|property| self.static_property_label(property))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Render one static property label.
    fn static_property_label(&self, property: &dir::StaticProperty) -> String {
        match property {
            dir::StaticProperty::Field { key, value } => {
                // render a static key/value field
                let key = self.static_key(*key);
                let value = self.static_term_label(value);

                format!("{key}: {value}")
            }
            dir::StaticProperty::Method { key, .. } => {
                // render a static method key without its body
                let key = key
                    .map(|key| self.static_key(key))
                    .unwrap_or_else(|| "<call>".to_string());

                format!("{key}()")
            }
            dir::StaticProperty::Spread { value } => {
                // render a static spread operand
                let value = self.static_term_label(value);

                format!("...{value}")
            }
        }
    }

    /// Return the binding snapshot names.
    fn binding_names(&self) -> BindingSnapshotName<'a> {
        BindingSnapshotName::new(self.binding_table(), Some(self.tree), self.strings)
    }

    /// Return the active binding table.
    fn binding_table(&self) -> &'a dir::BindingTable<'a> {
        let Some(bindings) = self.bindings else {
            panic!("dir snapshot needs bindings")
        };

        bindings
    }

    /// Return one symbol by local id.
    fn symbol(&self, symbol_id: dir::LocalSymbolId) -> &'a dir::Symbol {
        self.binding_table().get_symbol(symbol_id)
    }

    /// Render one foreign symbol label.
    fn foreign_symbol_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        let module = self.module_label(symbol_id.module_id);
        let symbol = self
            .foreign_symbol_labels
            .get(&symbol_id.module_id)
            .and_then(|labels| labels.get(&symbol_id.local_id))
            .cloned()
            .unwrap_or_else(|| BindingSnapshotName::local_symbol(symbol_id.local_id));

        format!("{module}.{symbol}")
    }

    /// Render one module path as a compact qualifier.
    fn module_label(&self, module_id: ModuleId) -> String {
        let module = self.module_path(module_id);
        let module = module.strip_suffix(".ds").unwrap_or(&module);
        let module = module.trim_start_matches("./");
        let module = module.trim_start_matches(['/', '\\']);
        let module = module.replace(['/', '\\'], ".");

        module.trim_matches('.').to_string()
    }

    /// Convert one CamelCase-ish debug string to lower snake case.
    fn lower_snake(value: &str) -> String {
        let mut result = String::new();

        // split before uppercase letters
        for character in value.chars() {
            if character.is_ascii_uppercase() {
                if !result.is_empty() {
                    result.push('_');
                }
                result.push(character.to_ascii_lowercase());
            } else {
                result.push(character);
            }
        }

        result
    }
}
