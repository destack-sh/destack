use std::sync::Arc;

use destack_dir as dir;
use destack_source::{ComponentId, FileId, ModuleId};
use serde::{Deserialize, Serialize};

/// Parsed DIR for one source module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirParsed {
    /// The parsed tree.
    pub tree: dir::Tree,
    /// The parsed parent index.
    pub parents: dir::NodeParentIndex,
    /// The parsed physical files.
    pub files: Vec<DirParsedFile>,
    /// Stable anchor expression for diagnostics.
    pub anchor_expression: dir::LocalNodeId<dir::Expression>,
}

impl DirParsed {
    /// Create a parsed DIR artifact.
    pub fn new(
        tree: dir::Tree,
        files: Vec<DirParsedFile>,
        anchor_expression: dir::LocalNodeId<dir::Expression>,
    ) -> Self {
        let parents = dir::NodeParentIndex::from_tree(&tree);

        Self {
            tree,
            parents,
            files,
            anchor_expression,
        }
    }

    /// Return parsed side data for one physical file.
    pub fn file(&self, file_id: FileId) -> Option<&DirParsedFile> {
        self.files.iter().find(|file| file.file_id == file_id)
    }

    /// Return parsed roots for one physical file.
    pub fn roots_for_file(&self, file_id: FileId) -> Option<&[dir::LocalNodeId<dir::Expression>]> {
        self.file(file_id).map(|file| file.roots.as_slice())
    }

    /// Iterate full token spans for one physical file.
    pub fn iter_token_spans_for_file(
        &self,
        file_id: FileId,
    ) -> Option<impl Iterator<Item = dir::TokenSpan> + '_> {
        let file = self.file(file_id)?;

        Some(file.iter_token_spans())
    }

    /// Iterate full side token spans for one physical file.
    pub fn iter_side_token_spans_for_file(
        &self,
        file_id: FileId,
    ) -> Option<impl Iterator<Item = dir::TokenSpan> + '_> {
        let file = self.file(file_id)?;

        Some(file.iter_side_token_spans())
    }
}

/// Parsed roots and side data for one physical file in a canonical module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirParsedFile {
    /// The source file id.
    pub file_id: FileId,
    /// The condition aliases attached to this file.
    pub aliases: Vec<String>,
    /// The top-level expressions parsed from this file.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The source file tokens.
    pub tokens: Vec<dir::TokenRange>,
    /// The source file side tokens.
    pub side_tokens: Vec<dir::TokenRange>,
    /// Stable anchor expression for diagnostics in this file.
    pub anchor_expression: dir::LocalNodeId<dir::Expression>,
}

impl DirParsedFile {
    /// Iterate full token spans for this physical file.
    pub fn iter_token_spans(&self) -> impl Iterator<Item = dir::TokenSpan> + '_ {
        let file_id = self.file_id;

        self.tokens
            .iter()
            .map(move |token| token.with_file(file_id))
    }

    /// Iterate full side token spans for this physical file.
    pub fn iter_side_token_spans(&self) -> impl Iterator<Item = dir::TokenSpan> + '_ {
        let file_id = self.file_id;

        self.side_tokens
            .iter()
            .map(move |token| token.with_file(file_id))
    }
}

/// Bound DIR base for one source module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirBound {
    /// Source bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// Source types.
    pub types: Arc<dir::TypeSegment>,
    /// Source static values.
    pub statics: Arc<dir::StaticSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable module node for module-level state.
    pub module_node: dir::LocalNodeIdAny,

    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

impl DirBound {
    /// Return the cumulative binding table for bound DIR.
    pub fn binding_table(&self) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segment(self.bindings.clone())
    }

    /// Return the cumulative type table for bound DIR.
    pub fn type_table(&self) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segment(self.types.clone())
    }

    /// Return the cumulative static table for bound DIR.
    pub fn static_table(&self) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segment(self.statics.clone())
    }
}

/// Source import resolution for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirImported {
    /// Resolved module imports.
    pub modules: Arc<dir::ModuleSegment>,
}

impl DirImported {
    /// Return the cumulative module table for imported DIR.
    pub fn module_table(&self) -> dir::ModuleTable<'static> {
        dir::ModuleTable::from_segment(Arc::clone(&self.modules))
    }
}

/// Fixed-point macro expansion segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExpanded {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New module imports.
    pub modules: Arc<dir::ModuleSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New static values.
    pub statics: Arc<dir::StaticSegment>,
    /// Expanded macro invocations.
    pub macros: dir::MacroTable,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl DirExpanded {
    /// Return the cumulative binding table for expanded DIR.
    pub fn binding_table(&self, bound: &DirBound) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![bound.bindings.clone(), self.bindings.clone()])
    }

    /// Return the cumulative module table for expanded DIR.
    pub fn module_table(&self, imported: &DirImported) -> dir::ModuleTable<'static> {
        dir::ModuleTable::from_segments(vec![imported.modules.clone(), self.modules.clone()])
    }

    /// Return the cumulative type table for expanded DIR.
    pub fn type_table(&self, bound: &DirBound) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![bound.types.clone(), self.types.clone()])
    }

    /// Return the cumulative static table for expanded DIR.
    pub fn static_table(&self, bound: &DirBound) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![bound.statics.clone(), self.statics.clone()])
    }
}

/// Export table over the expanded view for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExported {
    /// Resolved exports.
    pub exports: dir::ExportTable,
    /// Global declarations contributed by this module.
    pub globals: dir::GlobalTable,
}

/// Resolved import targets for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirResolved {
    /// Resolved imports.
    pub imports: dir::ImportTable,
    /// Resolved namespace import paths.
    pub paths: dir::PathTable,
}

/// Checked DIR output for one source component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirCheckedComponent {
    /// The checked component id.
    pub component: ComponentId,
    /// The checked module outputs in stable module order.
    pub modules: Vec<DirCheckedComponentEntry>,
}

impl DirCheckedComponent {
    /// Return checked output for one module in this component.
    pub fn module(&self, module: ModuleId) -> Option<&DirCheckedComponentEntry> {
        self.modules.iter().find(|entry| entry.module == module)
    }
}

/// Checked DIR entry for one module in a checked component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirCheckedComponentEntry {
    /// The checked module id.
    pub module: ModuleId,
    /// The checked side tables for this module.
    pub checked: DirCheckedModule,
}

/// Type-checking segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirCheckedModule {
    /// New annotation invocations.
    pub annotations: Arc<dir::AnnotationSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New static values.
    pub statics: Arc<dir::StaticSegment>,
    /// New resolutions.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// New generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// New declaration definitions.
    pub definitions: Arc<dir::DefinitionSegment>,
    /// New type relations.
    pub relations: Arc<dir::RelationSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
}

/// Facade artifact for one module checked inside a component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirChecked {
    /// The component that owns this module's checked output.
    pub component: ComponentId,
    /// The module used to enter the checked component graph.
    pub entry: ModuleId,
}

impl DirCheckedModule {
    /// Return the cumulative annotation table for checked DIR.
    pub fn annotation_table(&self) -> dir::AnnotationTable<'static> {
        dir::AnnotationTable::from_segment(self.annotations.clone())
    }

    /// Return the cumulative type table for checked DIR.
    pub fn type_table(&self, bound: &DirBound, expanded: &DirExpanded) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for checked DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            self.statics.clone(),
        ])
    }

    /// Return the cumulative resolution table for checked DIR.
    pub fn resolution_table(&self) -> dir::ResolutionTable<'static> {
        dir::ResolutionTable::from_segment(self.resolutions.clone())
    }

    /// Return the cumulative generic table for checked DIR.
    pub fn generic_table(&self) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segment(self.generics.clone())
    }

    /// Return the cumulative definition table for checked DIR.
    pub fn definition_table(&self) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(self.definitions.clone())
    }

    /// Return the cumulative relation table for checked DIR.
    pub fn relation_table(&self) -> dir::RelationTable<'static> {
        dir::RelationTable::from_segment(self.relations.clone())
    }

    /// Return the cumulative coercion table for checked DIR.
    pub fn coercion_table(&self) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segment(self.coercions.clone())
    }

    /// Return the cumulative capture table for checked DIR.
    pub fn capture_table(&self) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segment(self.captures.clone())
    }

    /// Return the cumulative layout table for checked DIR.
    pub fn layout_table(&self) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segment(self.layouts.clone())
    }
}

/// Comptime materialization segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirMaterialized {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New static values.
    pub statics: Arc<dir::StaticSegment>,
    /// New resolutions.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// New generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// New type relations.
    pub relations: Arc<dir::RelationSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl DirMaterialized {
    /// Return the cumulative binding table for materialized DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative type table for materialized DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            checked.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for materialized DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            checked.statics.clone(),
            self.statics.clone(),
        ])
    }

    /// Return the cumulative resolution table for materialized DIR.
    pub fn resolution_table(&self, checked: &DirCheckedModule) -> dir::ResolutionTable<'static> {
        dir::ResolutionTable::from_segments(vec![
            checked.resolutions.clone(),
            self.resolutions.clone(),
        ])
    }

    /// Return the cumulative generic table for materialized DIR.
    pub fn generic_table(&self, checked: &DirCheckedModule) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segments(vec![checked.generics.clone(), self.generics.clone()])
    }

    /// Return the cumulative definition table for materialized DIR.
    pub fn definition_table(&self, checked: &DirCheckedModule) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(checked.definitions.clone())
    }

    /// Return the cumulative relation table for materialized DIR.
    pub fn relation_table(&self, checked: &DirCheckedModule) -> dir::RelationTable<'static> {
        dir::RelationTable::from_segments(vec![checked.relations.clone(), self.relations.clone()])
    }

    /// Return the cumulative coercion table for materialized DIR.
    pub fn coercion_table(&self, checked: &DirCheckedModule) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segments(vec![checked.coercions.clone(), self.coercions.clone()])
    }

    /// Return the cumulative capture table for materialized DIR.
    pub fn capture_table(&self, checked: &DirCheckedModule) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segments(vec![checked.captures.clone(), self.captures.clone()])
    }

    /// Return the cumulative layout table for materialized DIR.
    pub fn layout_table(&self, checked: &DirCheckedModule) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segments(vec![checked.layouts.clone(), self.layouts.clone()])
    }
}

/// DIR-to-MIR elaboration segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New static values.
    pub statics: Arc<dir::StaticSegment>,
    /// New resolutions.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// New generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// New type relations.
    pub relations: Arc<dir::RelationSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// New guards.
    pub guards: dir::GuardTable,
}

impl DirElaborated {
    /// Return the cumulative binding table for elaborated DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        materialized: &DirMaterialized,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            materialized.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative type table for elaborated DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            checked.types.clone(),
            materialized.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for elaborated DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            checked.statics.clone(),
            materialized.statics.clone(),
            self.statics.clone(),
        ])
    }

    /// Return the cumulative resolution table for elaborated DIR.
    pub fn resolution_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::ResolutionTable<'static> {
        dir::ResolutionTable::from_segments(vec![
            checked.resolutions.clone(),
            materialized.resolutions.clone(),
            self.resolutions.clone(),
        ])
    }

    /// Return the cumulative generic table for elaborated DIR.
    pub fn generic_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segments(vec![
            checked.generics.clone(),
            materialized.generics.clone(),
            self.generics.clone(),
        ])
    }

    /// Return the cumulative definition table for elaborated DIR.
    pub fn definition_table(&self, checked: &DirCheckedModule) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(checked.definitions.clone())
    }

    /// Return the cumulative relation table for elaborated DIR.
    pub fn relation_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::RelationTable<'static> {
        dir::RelationTable::from_segments(vec![
            checked.relations.clone(),
            materialized.relations.clone(),
            self.relations.clone(),
        ])
    }

    /// Return the cumulative coercion table for elaborated DIR.
    pub fn coercion_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segments(vec![
            checked.coercions.clone(),
            materialized.coercions.clone(),
            self.coercions.clone(),
        ])
    }

    /// Return the cumulative capture table for elaborated DIR.
    pub fn capture_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segments(vec![
            checked.captures.clone(),
            materialized.captures.clone(),
            self.captures.clone(),
        ])
    }

    /// Return the cumulative layout table for elaborated DIR.
    pub fn layout_table(
        &self,
        checked: &DirCheckedModule,
        materialized: &DirMaterialized,
    ) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segments(vec![
            checked.layouts.clone(),
            materialized.layouts.clone(),
            self.layouts.clone(),
        ])
    }
}
