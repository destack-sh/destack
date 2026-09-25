use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use tspp_artifact::{DirBound, DirExpanded, DirExported, DirImported, DirResolved, DirView};
use tspp_core::{StringId, StringPool};
use tspp_dir as dir;
use tspp_source::ModuleId;

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
    /// The local binding name index.
    pub(super) binding_names: Option<BindingSnapshotName<'a>>,
    /// The checked generic table.
    pub(super) generics: Option<dir::GenericTable<'static>>,
    /// The checked definition table.
    pub(super) definitions: Option<dir::DefinitionTable<'static>>,
    /// The member selections labels read through.
    pub(super) members: Option<dir::MemberTable<'static>>,
    /// The visible type table used by layout anchors.
    pub(super) types: Option<dir::TypeTable<'static>>,
    /// The lexical resolution table used by place labels.
    pub(super) names: Option<dir::ResolutionTable<'static>>,
    /// The visible static table used by type labels.
    pub(super) statics: Option<dir::StaticTable<'static>>,
    /// Module paths used in multi-module snapshots.
    pub(super) module_path_by_id: Option<&'a BTreeMap<ModuleId, String>>,
    /// Foreign binding metadata keyed by module.
    pub(super) foreign_bindings: BTreeMap<ModuleId, dir::BindingTable<'a>>,
    /// Foreign generic tables keyed by module.
    pub(super) foreign_generics: BTreeMap<ModuleId, dir::GenericTable<'static>>,
    /// Foreign definition tables keyed by module.
    pub(super) foreign_definitions: BTreeMap<ModuleId, dir::DefinitionTable<'static>>,
    /// Foreign type tables keyed by module.
    pub(super) foreign_types: BTreeMap<ModuleId, dir::TypeTable<'static>>,
    /// Foreign static tables keyed by module.
    pub(super) foreign_statics: BTreeMap<ModuleId, dir::StaticTable<'static>>,
    /// Foreign symbol labels keyed by module and local symbol.
    pub(super) foreign_symbol_labels:
        RefCell<BTreeMap<ModuleId, BTreeMap<dir::LocalSymbolId, String>>>,
    /// Source-visible global names keyed by resolved symbol.
    pub(super) global_names_by_symbol: BTreeMap<dir::GlobalSymbolId, BTreeSet<StringId>>,
    /// Language items keyed by resolved global symbol.
    pub(super) language_item_by_symbol: BTreeMap<dir::GlobalSymbolId, dir::LanguageItem>,
    /// Semantic type labels keyed by global type id.
    pub(super) type_labels: BTreeMap<dir::GlobalTypeId, String>,
    /// Semantic static labels keyed by global static id.
    pub(super) static_labels: BTreeMap<dir::GlobalStaticId, String>,
    /// Static values rendered with their decorator applications.
    pub(super) decorator_statics: BTreeSet<dir::GlobalStaticId>,
    /// Whether to render dense binding node rows.
    pub(super) binding_nodes: bool,
    /// Whether to render the witness rows the generic tables record.
    pub(super) witnesses: bool,
    /// Whether to render expression node type rows.
    pub(super) type_nodes: bool,
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
            binding_names: None,
            generics: None,
            definitions: None,
            members: None,
            types: None,
            names: None,
            statics: None,
            module_path_by_id: None,
            foreign_bindings: BTreeMap::new(),
            foreign_generics: BTreeMap::new(),
            foreign_definitions: BTreeMap::new(),
            foreign_types: BTreeMap::new(),
            foreign_statics: BTreeMap::new(),
            foreign_symbol_labels: RefCell::new(BTreeMap::new()),
            global_names_by_symbol: BTreeMap::new(),
            language_item_by_symbol: BTreeMap::new(),
            type_labels: BTreeMap::new(),
            static_labels: BTreeMap::new(),
            decorator_statics: BTreeSet::new(),
            binding_nodes: false,
            witnesses: false,
            type_nodes: false,
            type_references: false,
            summaries: true,
            rows: Vec::new(),
        }
    }

    /// Set the binding table used for symbol labels.
    pub(crate) fn with_bindings(mut self, bindings: &'a dir::BindingTable<'a>) -> Self {
        self.bindings = Some(bindings);
        self.binding_names = Some(BindingSnapshotName::new(
            bindings,
            Some(self.tree),
            self.strings,
        ));

        self
    }

    /// Set semantic tables used for foreign semantic labels.
    pub(crate) fn with_foreign_tables(
        mut self,
        foreign_tables: Vec<(
            Option<dir::GenericTable<'static>>,
            dir::DefinitionTable<'static>,
            dir::TypeTable<'static>,
            dir::StaticTable<'static>,
        )>,
    ) -> Self {
        for (generics, definitions, types, statics) in foreign_tables {
            if let Some(generics) = generics {
                self.foreign_generics.insert(generics.module_id, generics);
            }
            self.foreign_definitions
                .insert(definitions.module_id, definitions);
            self.foreign_types.insert(types.module_id, types);
            self.foreign_statics.insert(statics.module_id, statics);
        }

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

    /// Set binding metadata used for foreign symbol labels.
    pub(crate) fn with_foreign_bindings(
        mut self,
        foreign_bindings: Vec<dir::BindingTable<'a>>,
    ) -> Self {
        for bindings in foreign_bindings {
            self.foreign_bindings.insert(bindings.module_id, bindings);
        }

        self
    }

    /// Install the final cumulative type table stage rows dedup against.
    pub(crate) fn set_effective_types(&mut self, types: dir::TypeTable<'static>) {
        self.types = Some(types);
    }

    /// Return whether one node type row survives later segment overrides.
    pub(crate) fn is_effective_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> bool {
        match &self.types {
            Some(types) => types.get_node_type_id(node) == Some(ty),
            None => true,
        }
    }

    /// Return whether one symbol type row survives later segment overrides.
    pub(crate) fn is_effective_symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> bool {
        match &self.types {
            Some(types) => types.get_symbol_type_id(symbol) == Some(ty),
            None => true,
        }
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
        self.witnesses = selection.witnesses;
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
        self.witnesses = selection.witnesses;
        self.summaries = selection.summaries;

        if selection.module {
            self.add_table(imported.modules.as_ref());
        }
    }

    /// Add selected rows for a resolved DIR artifact.
    pub(crate) fn add_resolved(&mut self, selection: DirRows, resolved: &DirResolved) {
        self.witnesses = selection.witnesses;
        self.summaries = selection.summaries;
        self.add_global_names(&resolved.imports);
        self.add_language_items(&resolved.imports);

        if selection.import {
            self.add_table(&resolved.imports);
            self.add_table(&resolved.references);
            self.add_table(&resolved.extensions);
        }
    }

    /// Add selected rows for an expanded DIR artifact.
    pub(crate) fn add_expanded(&mut self, selection: DirRows, expanded: &DirExpanded) {
        self.witnesses = selection.witnesses;
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
        self.witnesses = selection.witnesses;
        self.summaries = selection.summaries;

        if selection.export {
            self.add_table(&exported.exports);
            self.add_table(&exported.globals);
        }
    }

    /// Add selected rows for a checked DIR artifact.
    pub(crate) fn add_checked(&mut self, selection: DirRows, view: &DirView) {
        let declared = view
            .declared
            .as_ref()
            .unwrap_or_else(|| unreachable!("a checked view without its declared stage"));
        let elaborated = view
            .elaborated
            .as_ref()
            .unwrap_or_else(|| unreachable!("a checked view without its elaborated stage"));
        let checked = view
            .checked
            .as_ref()
            .unwrap_or_else(|| unreachable!("a checked view without its checked stage"));
        self.witnesses = selection.witnesses;
        self.summaries = selection.summaries;
        self.type_nodes = selection.type_nodes;
        self.type_references = selection.type_references;

        if selection.uses_type_labels() {
            self.generics = Some(view.generics().clone());
            self.definitions = Some(view.definitions().clone());
            self.members = Some(view.members().clone());

            let types = view.types().clone();
            let statics = view.statics().clone();
            // keep an installed final table, which stage rows dedup against
            if self.types.is_none() {
                self.types = Some(types.clone());
            }
            self.statics = Some(statics.clone());
            self.add_static_labels(&statics);
            self.add_type_labels(&types);
        }

        if selection.types {
            self.add_table(declared.types.as_ref());
            self.add_table(elaborated.types.as_ref());
            self.add_table(checked.types.as_ref());
        }

        if selection.decorators {
            self.add_table(declared.decorators.as_ref());
            self.add_table(elaborated.decorators.as_ref());
            self.add_table(checked.decorators.as_ref());
        }

        if selection.statics {
            for (_, application) in elaborated.decorators.iter_applications() {
                self.decorator_statics.insert(application.value);
            }
            for (_, application) in checked.decorators.iter_applications() {
                self.decorator_statics.insert(application.value);
            }
            self.add_table(declared.statics.as_ref());
            self.add_table(elaborated.statics.as_ref());
            self.add_table(checked.statics.as_ref());
        }

        if selection.resolution {
            // stack the layers so instance rows derive from winning rows only
            self.names = Some(view.resolutions().clone());
            self.add_table(view.resolutions());
            self.add_table(view.decisions());
        }

        if selection.generics {
            self.add_table(declared.generics.as_ref());
            self.add_table(elaborated.generics.as_ref());
            self.add_table(checked.generics.as_ref());
        }

        if selection.definitions {
            self.add_table(declared.definitions.as_ref());
        }

        if selection.coercion {
            self.add_table(checked.coercions.as_ref());
        }

        if selection.capture {
            self.add_table(checked.captures.as_ref());
        }

        if selection.flow {
            self.add_table(view.flows());
        }

        if selection.copy {
            self.add_copy_rows(view.definitions(), view.representations());
        }
    }

    /// Add the copy row of each nominal declaration deriving Copy.
    fn add_copy_rows(
        &mut self,
        definitions: &dir::DefinitionTable<'_>,
        representations: &dir::RepresentationTable<'_>,
    ) {
        for (symbol, _) in definitions.iter_definitions() {
            if !representations.derives_copy(symbol).unwrap_or(false) {
                continue;
            }
            let row = SnapshotRow::new(self.anchor_symbol(symbol), "copy", "definition")
                .field("symbol", self.symbol_path_label(symbol));
            self.push(row);
        }
    }

    /// Add selected rows for a materialized DIR artifact.
    pub(crate) fn add_materialized(&mut self, selection: DirRows, view: &DirView) {
        let materialized = view
            .materialized
            .as_ref()
            .unwrap_or_else(|| unreachable!("a materialized view without its materialized stage"));
        self.witnesses = selection.witnesses;
        self.summaries = selection.summaries;
        self.type_nodes = selection.type_nodes;
        self.type_references = selection.type_references;

        // label rows through the whole stack, up to and including the materialized tail
        if selection.uses_type_labels() {
            self.generics = Some(view.generics().clone());
            self.definitions = Some(view.definitions().clone());
            self.members = Some(view.members().clone());

            let types = view.types().clone();
            let statics = view.statics().clone();
            self.types = Some(types.clone());
            self.statics = Some(statics.clone());
            self.add_static_labels(&statics);
            self.add_type_labels(&types);
        }

        // render the tail segments only, so the rows are what materialization added
        if selection.types {
            self.add_table(materialized.types.as_ref());
        }

        if selection.resolution {
            self.names = Some(view.resolutions().clone());
        }

        if selection.generics {
            self.add_table(materialized.generics.as_ref());
        }

        if selection.definitions {
            let declared = view
                .declared
                .as_ref()
                .unwrap_or_else(|| unreachable!("a checked view without its declared stage"));
            self.add_table(declared.definitions.as_ref());
        }

        if selection.copy {
            self.add_copy_rows(view.definitions(), view.representations());
        }
    }

    /// Add one row.
    pub(crate) fn push(&mut self, row: SnapshotRow) {
        if row.tag.entry == "summary" && !self.summaries {
            return;
        }

        // later table layers shadow earlier rows of the same identity,
        //  matching the stacked view consumers read
        let identity = |row: &SnapshotRow| {
            row.fields
                .iter()
                .take(2)
                .map(|field| (field.key.clone(), field.value.clone()))
                .collect::<Vec<_>>()
        };
        let replaced = self.rows.iter().position(|existing| {
            existing.tag == row.tag
                && existing.anchor == row.anchor
                && identity(existing) == identity(&row)
        });
        match replaced {
            Some(index) => {
                self.rows[index] = row;
            }
            None => {
                self.rows.push(row);
            }
        }
    }

    /// Add semantic language item identities from resolved imports.
    pub(crate) fn add_language_items(&mut self, imports: &dir::ImportTable) {
        for (item, symbol) in &imports.language_symbol_by_item {
            self.language_item_by_symbol.insert(*symbol, *item);
        }
    }

    /// Add source-visible global names from resolved imports.
    pub(crate) fn add_global_names(&mut self, imports: &dir::ImportTable) {
        for (key, resolutions) in &imports.global_resolution_by_key {
            let Some(name) = key.name() else {
                continue;
            };
            for resolution in resolutions {
                let Some(symbols) = resolution.target.symbol_ids() else {
                    continue;
                };

                for symbol in symbols {
                    self.global_names_by_symbol
                        .entry(*symbol)
                        .or_default()
                        .insert(name);
                }
            }
        }
    }

    /// Return the region names one template declares ahead of one parameter.
    pub(super) fn region_names_before(
        &self,
        generics: &dir::GenericTable<'_>,
        parameter: dir::LocalGenericParameterId,
    ) -> Vec<String> {
        generics.region_names_before(parameter, |symbol| self.symbol_label(symbol))
    }

    /// Return the source anchor for one DIR node.
    pub(crate) fn anchor_node(&self, node_id: dir::GlobalNodeIdAny) -> SnapshotAnchor {
        // anchor non-source nodes at the end
        if node_id.module_id != self.tree.module_id {
            return SnapshotAnchor::End;
        }

        // anchor generated nodes at the end
        if node_id.local_id.id as usize >= self.tree.node_count() {
            return SnapshotAnchor::End;
        }

        if let Some(span) = self.tree.get_span_by_id(node_id.local_id.id) {
            SnapshotAnchor::After(span)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Return the source anchor for one name resolution row.
    pub(crate) fn name_resolution_anchor(&self, node_id: dir::GlobalNodeIdAny) -> SnapshotAnchor {
        self.anchor_node(node_id)
    }

    /// Render the dotted source path label for one reference node.
    pub(crate) fn reference_source_label(&self, node: dir::GlobalNodeIdAny) -> String {
        assert_eq!(
            node.module_id, self.tree.module_id,
            "dir snapshot cannot render foreign source paths"
        );

        let path = match node.local_id.ty {
            dir::NodeType::Expression => {
                let node_id = node.local_id.into_typed();
                self.expression_source_path_segments(node_id)
            }
            dir::NodeType::TypeExpression => {
                let node_id = node.local_id.into_typed();
                match self.tree.get(node_id) {
                    dir::TypeExpression::Reference { path, .. } => {
                        path.segments.iter().copied().collect()
                    }
                    dir::TypeExpression::Lifetime { name } => vec![*name],
                    _ => panic!("reference table type source has no name"),
                }
            }
            dir::NodeType::DependencyItem => {
                let node_id: dir::LocalNodeId<dir::DependencyItem> = node.local_id.into_typed();
                let item = self.tree.get(node_id);
                let Some(selector) = item.export_selector() else {
                    panic!("reference table dependency source has no export selector");
                };

                return self.export_selector_label(selector);
            }
            _ => panic!("reference table source has an unsupported node type"),
        };

        path.iter()
            .map(|segment| self.strings.get(*segment))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Return source path segments for one path-bearing expression.
    fn expression_source_path_segments(
        &self,
        node_id: dir::LocalNodeId<dir::Expression>,
    ) -> Vec<dir::StringId> {
        let mut segments = Vec::new();
        self.collect_expression_source_path_segments(node_id, &mut segments);

        segments
    }

    /// Collect source path segments from one path-bearing expression.
    fn collect_expression_source_path_segments(
        &self,
        node_id: dir::LocalNodeId<dir::Expression>,
        segments: &mut Vec<dir::StringId>,
    ) {
        match self.tree.get(node_id) {
            dir::Expression::Identifier { name } => {
                segments.push(*name);
            }
            dir::Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                self.collect_expression_source_path_segments(*left, segments);
                segments.push(*name);
            }
            _ => panic!("path table expression source is not a path-bearing expression"),
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
        } else if let Some(owner) = self
            .binding_table()
            .get_scope_by_id(symbol.scope.id)
            .owner
            .filter(|owner| *owner != symbol_id.local_id)
        {
            self.anchor_symbol(owner.into_global(symbol_id.module_id))
        } else {
            SnapshotAnchor::End
        }
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

    /// Return whether one local symbol has no source name.
    pub(crate) fn is_anonymous_symbol(&self, symbol_id: dir::GlobalSymbolId) -> bool {
        if symbol_id.module_id == self.tree.module_id {
            return self.symbol(symbol_id.local_id).name().is_none();
        }

        self.foreign_bindings
            .get(&symbol_id.module_id)
            .is_some_and(|bindings| bindings.get_symbol(symbol_id.local_id).name().is_none())
    }

    /// Render one anonymous extension path segment.
    pub(crate) fn anonymous_extension_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        let definitions = self.definition_table(symbol_id.module_id);
        let mut index = 0;

        // count anonymous extensions in definition order
        for (symbol, definition) in definitions.iter_definitions() {
            if !matches!(definition, dir::Definition::Extension(_)) {
                continue;
            }

            if !self.is_anonymous_symbol(symbol) {
                continue;
            }

            index += 1;
            if symbol == symbol_id {
                return format!("<extension#{index}>");
            }
        }

        panic!("dir snapshot missing anonymous extension {symbol_id:?}");
    }

    /// Return one visible checked definition.
    pub(crate) fn definition(&self, symbol_id: dir::GlobalSymbolId) -> Option<&dir::Definition> {
        self.definition_table(symbol_id.module_id)
            .definition(symbol_id)
    }

    /// Render the declaration source for one local symbol.
    pub(crate) fn symbol_source(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        if symbol_id.module_id != self.tree.module_id {
            return None;
        }

        let symbol = self.symbol(symbol_id.local_id);
        let declaration = symbol.declaration?;
        if declaration.module_id != self.tree.module_id {
            return None;
        }

        self.node_source(declaration)
    }

    /// Render one global type id using semantic type text when possible.
    pub(crate) fn global_type_label(&self, type_id: dir::GlobalTypeId) -> String {
        if let Some(label) = self.type_labels.get(&type_id) {
            return label.clone();
        }

        if type_id.module_id != self.tree.module_id {
            if let Some(types) = self.foreign_types.get(&type_id.module_id) {
                return self.type_table_label(types, type_id.local_id);
            }

            let module = self.module_path(type_id.module_id);
            return format!("{module}.type{}", type_id.local_id.0);
        }
        let types = self
            .types
            .as_ref()
            .unwrap_or_else(|| panic!("dir snapshot missing type table for {type_id:?}"));

        self.type_table_label(types, type_id.local_id)
    }

    /// Render one symbol's checked type.
    pub(crate) fn global_symbol_type_label(&self, symbol: dir::GlobalSymbolId) -> String {
        let type_id = if symbol.module_id != self.tree.module_id {
            let types = self
                .foreign_types
                .get(&symbol.module_id)
                .unwrap_or_else(|| {
                    panic!("dir snapshot missing foreign type table for symbol {symbol:?}")
                });

            types.get_symbol_type_id(symbol)
        } else {
            let types = self
                .types
                .as_ref()
                .unwrap_or_else(|| panic!("dir snapshot missing type table for symbol {symbol:?}"));

            types.get_symbol_type_id(symbol)
        };
        let type_id =
            type_id.unwrap_or_else(|| panic!("dir snapshot symbol {symbol:?} has no type"));

        self.global_type_label(type_id)
    }

    /// Render one global static id using semantic static text when possible.
    pub(crate) fn global_static_label(&self, static_id: dir::GlobalStaticId) -> String {
        if let Some(label) = self.static_labels.get(&static_id) {
            return label.clone();
        }

        if static_id.module_id != self.tree.module_id {
            if let Some(statics) = self.foreign_statics.get(&static_id.module_id) {
                let term = statics.get_static(static_id.local_id);

                return self.static_term_label(term);
            }

            let module = self.module_path(static_id.module_id);
            return format!("{module}.static{}", static_id.local_id.0);
        }
        let statics = self
            .statics
            .as_ref()
            .unwrap_or_else(|| panic!("dir snapshot missing static table for {static_id:?}"));
        let term = statics.get_static(static_id.local_id);

        self.static_term_label(term)
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

    /// Render one static selection.
    pub(crate) fn static_key(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings.get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
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

    /// Return one module target field.
    pub(crate) fn module_target_field(&self, target: Option<ModuleId>) -> (&'static str, String) {
        match target {
            Some(module_id) => ("module", self.module_path(module_id)),
            None => ("target", "<unresolved>".to_string()),
        }
    }

    /// Render one member candidate label.
    pub(crate) fn member_candidate_label(&self, candidate: &dir::MemberCandidate) -> String {
        self.symbol_path_label(candidate.key.symbol)
    }

    /// Render one function target label.
    pub(crate) fn function_target_label(&self, function: &dir::FunctionTarget) -> String {
        self.symbol_path_label(function.key.symbol)
    }

    /// Render one static term label.
    pub(crate) fn static_term_label(&self, term: &dir::StaticTerm) -> String {
        match term {
            dir::StaticTerm::Literal { value } => self.scalar_literal_label(value),
            dir::StaticTerm::Type { ty } => self.global_type_label(*ty),
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
            dir::StaticTerm::Newtype { ty, value } => {
                let name = self.global_type_label(*ty);
                let arguments = match value.as_ref() {
                    dir::StaticTerm::Tuple { elements } => elements
                        .iter()
                        .map(|element| self.static_term_label(element))
                        .collect::<Vec<_>>()
                        .join(", "),
                    value => self.static_term_label(value),
                };

                format!("{name}({arguments})")
            }
            dir::StaticTerm::Object { properties } => {
                // render object properties recursively
                let properties = self.static_property_labels(properties);

                if properties.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{ {properties} }}")
                }
            }
            dir::StaticTerm::Struct { ty, properties } => {
                // render typed struct literal syntax
                let properties = self.static_property_labels(properties);

                if properties.is_empty() {
                    format!("{} {{}}", self.global_type_label(*ty))
                } else {
                    format!("{} {{ {properties} }}", self.global_type_label(*ty))
                }
            }
        }
    }

    /// Render one scalar literal label.
    pub(crate) fn scalar_literal_label(&self, literal: &dir::Literal) -> String {
        match literal {
            dir::Literal::Null => "null".to_string(),
            dir::Literal::Undefined => "undefined".to_string(),
            dir::Literal::Boolean(value) => value.to_string(),
            dir::Literal::Integer(value) => value.to_string(),
            dir::Literal::Bigint(value) => format!("{value}n"),
            dir::Literal::Float(value) => value.to_string(),
            dir::Literal::Character(value) => format!("'{value}'"),
            dir::Literal::String(value) => format!("{:?}", self.strings.get(*value)),
            dir::Literal::RegexString { content, flags } => {
                let flags = flags.map(|flags| self.strings.get(flags)).unwrap_or("");

                format!("/{}/{flags}", self.strings.get(*content))
            }
        }
    }

    /// Render one scalar literal as a plain row value.
    pub(crate) fn scalar_literal_value_label(&self, literal: &dir::Literal) -> String {
        match literal {
            dir::Literal::String(value) => self.strings.get(*value).to_string(),
            _ => self.scalar_literal_label(literal),
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

    /// Render one export target as ordered labels.
    pub(crate) fn export_target_labels(&self, target: &dir::ExportTarget) -> Vec<String> {
        match target {
            dir::ExportTarget::Symbols(symbols) => symbols
                .iter()
                .map(|symbol| self.symbol_path_label(*symbol))
                .collect(),
            dir::ExportTarget::Namespace(module) => vec![self.module_path(*module)],
        }
    }

    /// Render one macro trigger label.
    pub(crate) fn macro_trigger_label(&self, trigger: &dir::MacroTrigger) -> String {
        match trigger {
            dir::MacroTrigger::Decorator(node_id) => {
                // render the decorator node kind as the trigger
                let node_id = (*node_id).into_any();

                format!("decorator:{}", self.node_label(node_id))
            }
            dir::MacroTrigger::AutoDerive => "auto_derive".to_string(),
        }
    }

    /// Render one node id.
    pub(crate) fn node_label(&self, node_id: dir::GlobalNodeIdAny) -> String {
        node_id.local_id.ty.name().replace(' ', "_")
    }

    /// Return whether to render one checked type node row.
    pub(crate) fn should_render_type_node(
        &self,
        node_id: dir::GlobalNodeIdAny,
        type_id: dir::GlobalTypeId,
    ) -> bool {
        if !self.type_nodes {
            return false;
        }
        if node_id.module_id != self.tree.module_id {
            return false;
        }

        if node_id.local_id.ty != dir::NodeType::Expression {
            return false;
        }
        if self.node_is_nested_inside_type_context(node_id.local_id) {
            return false;
        }

        let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.local_id.id);
        let expression = self.tree.get(expression_id);
        if self.expression_has_boring_type_node(expression) {
            return false;
        }
        if self.expression_has_boring_void_type_node(node_id, expression, type_id) {
            return false;
        }

        self.type_references || !expression.is_reference()
    }

    /// Return whether one expression type row is structural noise.
    fn expression_has_boring_type_node(&self, expression: &dir::Expression) -> bool {
        // do blocks are value expressions and keep their rows
        if let dir::Expression::Block(block) = expression {
            return self.tree.get(*block).form != dir::BlockForm::Do;
        }

        matches!(
            expression,
            dir::Expression::Import { .. }
                | dir::Expression::Export { .. }
                | dir::Expression::Let { .. }
                | dir::Expression::LetElse { .. }
                | dir::Expression::Using { .. }
                | dir::Expression::Return { .. }
        ) || self.expression_is_named_declaration(expression)
    }

    /// Return whether one multiline control expression only reports `void`.
    fn expression_has_boring_void_type_node(
        &self,
        node_id: dir::GlobalNodeIdAny,
        expression: &dir::Expression,
        type_id: dir::GlobalTypeId,
    ) -> bool {
        if self.node_source(node_id).is_some() {
            return false;
        }
        if !self.global_type_is_void(type_id) {
            return false;
        }

        matches!(
            expression,
            dir::Expression::If { .. }
                | dir::Expression::While { .. }
                | dir::Expression::ForEach { .. }
                | dir::Expression::For { .. }
                | dir::Expression::Loop { .. }
                | dir::Expression::Try { .. }
                | dir::Expression::Match { .. }
                | dir::Expression::Switch { .. }
        )
    }

    /// Return whether one global type id names `void`.
    fn global_type_is_void(&self, type_id: dir::GlobalTypeId) -> bool {
        let types = if type_id.module_id == self.tree.module_id {
            self.types.as_ref()
        } else {
            self.foreign_types.get(&type_id.module_id)
        };

        matches!(
            types.map(|types| types.get_type(type_id.local_id)),
            Some(dir::Type::Void)
        )
    }

    /// Return whether one expression is a named declaration wrapper.
    fn expression_is_named_declaration(&self, expression: &dir::Expression) -> bool {
        let dir::Expression::Declaration(declaration) = expression else {
            return false;
        };
        let declaration = self.tree.get(*declaration);

        !matches!(
            declaration,
            dir::Declaration::Function(function)
                if function.signature.form == dir::FunctionForm::Lambda
        )
    }

    /// Return whether one node is nested inside non-runtime type context.
    fn node_is_nested_inside_type_context(&self, node_id: dir::LocalNodeIdAny) -> bool {
        let mut current = self.tree.get_parent(node_id.id);

        // follow the source tree until a type-only parent is found
        while let Some(node_id) = current {
            if matches!(
                node_id.ty,
                dir::NodeType::TypeExpression | dir::NodeType::GenericArgument
            ) {
                return true;
            }

            current = self.tree.get_parent(node_id.id);
        }

        false
    }

    /// Render source text for one node when it is compact enough for a row.
    pub(crate) fn node_source(&self, node_id: dir::GlobalNodeIdAny) -> Option<String> {
        // skip non-source nodes
        if node_id.module_id != self.tree.module_id {
            return None;
        }
        if node_id.local_id.id as usize >= self.tree.node_count() {
            return None;
        }

        let span = self.tree.get_span_by_id(node_id.local_id.id)?;
        let source = &self.source[span.start as usize..span.end as usize];
        let source = source.trim();

        if source.is_empty() || source.contains('\n') || source.len() > 80 {
            return None;
        }

        Some(source.to_string())
    }

    /// Render the main source region for one node.
    pub(crate) fn node_main_source(&self, node_id: dir::GlobalNodeIdAny) -> Option<String> {
        // skip non-source nodes
        if node_id.module_id != self.tree.module_id {
            return None;
        }
        let span = self.tree.get_main_span_by_id(node_id.local_id.id)?;
        let source = &self.source[span.start as usize..span.end as usize];
        let source = source.trim();

        (!source.is_empty()).then(|| source.to_string())
    }

    /// Render source text for one name resolution row.
    pub(crate) fn name_resolution_source(&self, node_id: dir::GlobalNodeIdAny) -> Option<String> {
        if node_id.module_id != self.tree.module_id {
            return None;
        }

        // prefer the referenced path over the full type instance
        if node_id.local_id.ty == dir::NodeType::TypeExpression {
            let type_id = dir::LocalNodeId::<dir::TypeExpression>::new(node_id.local_id.id);
            if let dir::TypeExpression::Reference { path, .. } = self.tree.get(type_id) {
                let source = path
                    .segments
                    .iter()
                    .map(|segment| self.strings.get(*segment))
                    .collect::<Vec<_>>()
                    .join(".");

                return Some(source);
            }
        }

        self.node_source(node_id)
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
            self.type_labels
                .insert(type_id.into_global(types.module_id), label);
        }
    }

    /// Add semantic static labels for one visible static table.
    fn add_static_labels(&mut self, statics: &dir::StaticTable<'_>) {
        for static_id in statics.iter_static_ids() {
            let term = statics.get_static(static_id);
            let label = self.static_term_label(term);
            self.static_labels
                .insert(static_id.into_global(statics.module_id), label);
        }
    }

    /// Render static property labels.
    fn static_property_labels(&self, properties: &[dir::StaticProperty]) -> String {
        properties
            .iter()
            .map(|property| self.static_property_label(property))
            .collect::<Vec<_>>()
            .join("; ")
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
        }
    }

    /// Return the binding snapshot names.
    fn binding_names(&self) -> &BindingSnapshotName<'a> {
        self.binding_names
            .as_ref()
            .unwrap_or_else(|| panic!("dir snapshot needs binding names"))
    }

    /// Return the checked definition table for one visible module.
    fn definition_table(&self, module_id: ModuleId) -> &dir::DefinitionTable<'static> {
        if module_id == self.tree.module_id {
            return self
                .definitions
                .as_ref()
                .unwrap_or_else(|| panic!("dir snapshot needs definitions"));
        }

        self.foreign_definitions
            .get(&module_id)
            .unwrap_or_else(|| panic!("dir snapshot needs foreign definitions for {module_id:?}"))
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
        let is_library = self.module_path(symbol_id.module_id).starts_with("tspp://");
        let module = self.module_label(symbol_id.module_id);
        let symbol = if let Some(symbol) = self.cached_foreign_symbol_label(symbol_id) {
            symbol
        } else if let Some(bindings) = self.foreign_bindings.get(&symbol_id.module_id) {
            let names = BindingSnapshotName::new(bindings, None, self.strings);
            let labels = names.symbol_path_labels();
            let symbol = labels
                .get(&symbol_id.local_id)
                .cloned()
                .unwrap_or_else(|| BindingSnapshotName::local_symbol(symbol_id.local_id));
            self.foreign_symbol_labels
                .borrow_mut()
                .insert(symbol_id.module_id, labels);

            symbol
        } else {
            BindingSnapshotName::local_symbol(symbol_id.local_id)
        };

        // library names stay bare: the library owns their uniqueness
        if is_library {
            return symbol;
        }

        format!("{module}.{symbol}")
    }

    /// Return one cached foreign symbol label.
    fn cached_foreign_symbol_label(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        self.foreign_symbol_labels
            .borrow()
            .get(&symbol_id.module_id)
            .and_then(|labels| labels.get(&symbol_id.local_id))
            .cloned()
    }

    /// Render one module path as a compact qualifier.
    fn module_label(&self, module_id: ModuleId) -> String {
        let module = self.module_path(module_id);
        let module = module.strip_prefix("tspp://").unwrap_or(&module);
        let module = module.strip_suffix(".tspp").unwrap_or(module);
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
