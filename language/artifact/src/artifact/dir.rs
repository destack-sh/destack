use std::sync::Arc;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{ArtifactProjectionFingerprint, DiagnosticControlTable};

/// Parsed DIR for one source module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirParsed {
    /// The parsed tree, indexed with its structural parents.
    pub tree: dir::Tree,
    /// The parsed physical files.
    pub files: Vec<DirParsedFile>,
    /// Stable anchor expression for diagnostics.
    pub anchor_expression: dir::LocalNodeId<dir::Expression>,
}

impl DirParsed {
    /// Create a parsed DIR artifact.
    pub fn new(
        mut tree: dir::Tree,
        files: Vec<DirParsedFile>,
        anchor_expression: dir::LocalNodeId<dir::Expression>,
    ) -> Self {
        // index source roots and diagnostic anchors
        let mut roots = files
            .iter()
            .flat_map(|file| file.roots.iter().copied())
            .collect::<Vec<_>>();
        roots.extend(files.iter().map(|file| file.anchor_expression));
        roots.push(anchor_expression);
        tree.index_parents(&roots);

        Self {
            tree,
            files,
            anchor_expression,
        }
    }

    /// Return parser output for one physical file.
    pub fn file(&self, file_id: FileId) -> Option<&DirParsedFile> {
        self.files.iter().find(|file| file.file_id == file_id)
    }
}

/// Parser output for one physical file in a canonical module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirParsedFile {
    /// The source file id.
    pub file_id: FileId,
    /// The condition aliases attached to this file.
    pub aliases: Vec<String>,
    /// The top-level expressions parsed from this file.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The semantic source tokens.
    pub tokens: Vec<dir::Token>,
    /// The retained source comments.
    pub comments: Vec<dir::Comment>,
    /// Stable anchor expression for diagnostics in this file.
    pub anchor_expression: dir::LocalNodeId<dir::Expression>,
}

impl DirParsedFile {
    /// Iterate semantic token spans for this physical file.
    pub fn iter_token_spans(&self) -> impl Iterator<Item = dir::TokenSpan> + '_ {
        let file_id = self.file_id;

        self.tokens
            .iter()
            .copied()
            .map(move |token| dir::TokenSpan::new(token, file_id))
    }
}

/// Bound DIR base for one source module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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

/// Source import resolution for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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

/// Fixed-point macro expansion segment for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
    /// Expanded macro applications.
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

/// Export table over the expanded view for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirExported {
    /// Resolved exports.
    pub exports: dir::ExportTable,
    /// Global declarations contributed by this module.
    pub globals: dir::GlobalTable,
    /// Module-scope binding names, kept for missing-export diagnostics.
    pub locals: Vec<String>,
}

impl DirExported {
    /// Return modules targeted by re-export edges.
    pub fn reexport_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.exports
            .reexport_modules()
            .chain(self.globals.reexport_modules())
    }
}

/// Resolved import targets for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirResolved {
    /// Resolved imports.
    pub imports: dir::ImportTable,
    /// Resolved source references.
    pub references: dir::ReferenceTable,
    /// Resolved exported extensions.
    pub extensions: dir::ExtensionTable,
}

impl DirResolved {
    /// Return modules that own resolved import or reference targets.
    pub fn target_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.imports
            .target_modules()
            .chain(self.references.target_modules())
    }

    /// Return the fingerprint of relationships that determine component edges.
    pub(crate) fn component_edges_fingerprint(&self) -> ArtifactProjectionFingerprint {
        let mut modules = self.target_modules().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        // retain the exact symbol and namespace identities used for inference edges
        let mut symbols = Vec::new();
        let mut namespaces = Vec::new();
        for (source, reference) in &self.references.target_by_node {
            if source.local_id.ty == dir::NodeType::DependencyItem {
                continue;
            }

            match reference {
                dir::Reference::Bound(references) => {
                    symbols.extend(references.iter().copied());
                }
                dir::Reference::Namespace(module) => namespaces.push(*module),
                dir::Reference::Projected { base, .. } => match base {
                    dir::ImportTarget::Symbol(symbol) => symbols.push(*symbol),
                    dir::ImportTarget::Namespace(module) => namespaces.push(*module),
                },
                dir::Reference::Ambiguous(targets) => {
                    // retain every candidate because check may select any one
                    for target in targets {
                        match target {
                            dir::ImportTarget::Symbol(symbol) => symbols.push(*symbol),
                            dir::ImportTarget::Namespace(module) => namespaces.push(*module),
                        }
                    }
                }
                dir::Reference::Missing => {}
            }
        }
        symbols.sort_unstable();
        symbols.dedup();
        namespaces.sort_unstable();
        namespaces.dedup();

        // retain inherent extension ownership relationships
        let mut extensions = self.extensions.targets().collect::<Vec<_>>();
        extensions.sort_unstable();
        extensions.dedup();

        ArtifactProjectionFingerprint::new(&(modules, symbols, namespaces, extensions))
    }
}

/// Declared DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirDeclared {
    /// The stable fingerprint of this module's declared output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// The modules this artifact's rows mention.
    pub references: Vec<ModuleId>,
    /// Declared binding segment.
    pub bindings: Arc<dir::BindingSegment>,
    /// Declared decorator applications.
    pub decorators: Arc<dir::DecoratorSegment>,
    /// Declared types.
    pub types: Arc<dir::TypeSegment>,
    /// Declared static values.
    pub statics: Arc<dir::StaticSegment>,
    /// Declared generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// Declared definitions.
    pub definitions: Arc<dir::DefinitionSegment>,
    /// Declared node resolutions.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// Decisions made while declaring.
    pub decisions: Arc<dir::DecisionSegment>,
    /// Authored member lookup subjects.
    pub members: Arc<dir::MemberSegment>,
}

impl DirDeclared {
    /// Return the cumulative binding table for declared DIR.
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

    /// Return the cumulative type table for declared DIR.
    pub fn type_table(&self, bound: &DirBound, expanded: &DirExpanded) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for declared DIR.
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

    /// Return the cumulative generic table for declared DIR.
    pub fn generic_table(&self) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segment(self.generics.clone())
    }

    /// Return the declared member table.
    pub fn member_table(&self) -> dir::MemberTable<'static> {
        dir::MemberTable::from_segment(self.members.clone())
    }

    /// Return the cumulative definition table for declared DIR.
    pub fn definition_table(&self) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(self.definitions.clone())
    }
}

/// Elaborated DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirElaborated {
    /// The stable fingerprint of this module's elaborated output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// The modules this artifact's rows mention.
    pub references: Vec<ModuleId>,
    /// Symbols minted while deriving variants and constructors.
    pub bindings: Arc<dir::BindingSegment>,
    /// Types minted while flattening member bindings.
    pub types: Arc<dir::TypeSegment>,
    /// Flattened member bindings per owner subject.
    pub members: Arc<dir::MemberSegment>,
    /// Auto conformances for concrete nominals.
    pub auto: Arc<dir::AutoSegment>,
    /// Derived variances.
    pub generics: Arc<dir::GenericSegment>,
    /// Definitions carrying derived constructor rows.
    pub definitions: Arc<dir::DefinitionSegment>,
    /// Selected and evaluated decorator applications.
    pub decorators: Arc<dir::DecoratorSegment>,
    /// Resolutions recorded while checking declarations and decorators.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// Decisions made while elaborating declarations and decorators.
    pub decisions: Arc<dir::DecisionSegment>,
    /// Static values evaluated for decorator applications.
    pub statics: Arc<dir::StaticSegment>,
    /// Diagnostic controls applied by decorators.
    pub controls: Arc<DiagnosticControlTable>,
}

impl DirElaborated {
    /// Return the elaborated member table.
    pub fn member_table(&self) -> dir::MemberTable<'static> {
        dir::MemberTable::from_segment(self.members.clone())
    }

    /// Return the cumulative binding table for elaborated DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            declared.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative type table for elaborated DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            declared.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the elaborated auto implementation table.
    pub fn auto_table(&self) -> dir::AutoTable<'static> {
        dir::AutoTable::from_segment(self.auto.clone())
    }

    /// Return the cumulative generic table for elaborated DIR.
    pub fn generic_table(&self, declared: &DirDeclared) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segments(vec![declared.generics.clone(), self.generics.clone()])
    }

    /// Return the elaborated definition table.
    pub fn definition_table(&self) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(self.definitions.clone())
    }

    /// Return the cumulative static table for elaborated DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            declared.statics.clone(),
            self.statics.clone(),
        ])
    }
}

/// Checked DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirChecked {
    /// The stable fingerprint of this module's checked output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// Checked binding segment.
    pub bindings: Arc<dir::BindingSegment>,
    /// New decorator applications.
    pub decorators: Arc<dir::DecoratorSegment>,
    /// Checked diagnostic controls.
    pub controls: Arc<DiagnosticControlTable>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New static values.
    pub statics: Arc<dir::StaticSegment>,
    /// New resolutions.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// New decisions.
    pub decisions: Arc<dir::DecisionSegment>,
    /// New generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
}

impl DirChecked {
    /// Return the cumulative binding table for checked DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            declared.bindings.clone(),
            elaborated.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative decorator table for checked DIR.
    pub fn decorator_table(&self, elaborated: &DirElaborated) -> dir::DecoratorTable<'static> {
        dir::DecoratorTable::from_segments(vec![
            elaborated.decorators.clone(),
            self.decorators.clone(),
        ])
    }

    /// Return the cumulative type table for checked DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            declared.types.clone(),
            elaborated.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for checked DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            declared.statics.clone(),
            elaborated.statics.clone(),
            self.statics.clone(),
        ])
    }

    /// Return the cumulative resolution table for checked DIR.
    pub fn resolution_table(
        &self,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::ResolutionTable<'static> {
        dir::ResolutionTable::from_segments(vec![
            declared.resolutions.clone(),
            elaborated.resolutions.clone(),
            self.resolutions.clone(),
        ])
    }

    /// Return the cumulative decision table for checked DIR.
    pub fn decision_table(
        &self,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::DecisionTable<'static> {
        dir::DecisionTable::from_segments(vec![
            declared.decisions.clone(),
            elaborated.decisions.clone(),
            self.decisions.clone(),
        ])
    }

    /// Return the cumulative generic table for checked DIR.
    pub fn generic_table(
        &self,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
    ) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segments(vec![
            declared.generics.clone(),
            elaborated.generics.clone(),
            self.generics.clone(),
        ])
    }

    /// Return the cumulative coercion table for checked DIR.
    pub fn coercion_table(&self) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segment(self.coercions.clone())
    }

    /// Return the cumulative capture table for checked DIR.
    pub fn capture_table(&self) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segment(self.captures.clone())
    }
}

/// Comptime materialization segment for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
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
    /// New decisions.
    pub decisions: Arc<dir::DecisionSegment>,
    /// New generic slots and instances.
    pub generics: Arc<dir::GenericSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
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
        declared: &DirDeclared,
        checked: &DirChecked,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            declared.types.clone(),
            checked.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative static table for materialized DIR.
    pub fn static_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
        checked: &DirChecked,
    ) -> dir::StaticTable<'static> {
        dir::StaticTable::from_segments(vec![
            bound.statics.clone(),
            expanded.statics.clone(),
            declared.statics.clone(),
            elaborated.statics.clone(),
            checked.statics.clone(),
            self.statics.clone(),
        ])
    }

    /// Return the cumulative resolution table for materialized DIR.
    pub fn resolution_table(
        &self,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
        checked: &DirChecked,
    ) -> dir::ResolutionTable<'static> {
        dir::ResolutionTable::from_segments(vec![
            declared.resolutions.clone(),
            elaborated.resolutions.clone(),
            checked.resolutions.clone(),
            self.resolutions.clone(),
        ])
    }

    /// Return the cumulative decision table for materialized DIR.
    pub fn decision_table(
        &self,
        declared: &DirDeclared,
        elaborated: &DirElaborated,
        checked: &DirChecked,
    ) -> dir::DecisionTable<'static> {
        dir::DecisionTable::from_segments(vec![
            declared.decisions.clone(),
            elaborated.decisions.clone(),
            checked.decisions.clone(),
            self.decisions.clone(),
        ])
    }

    /// Return the cumulative generic table for materialized DIR.
    pub fn generic_table(
        &self,
        declared: &DirDeclared,
        checked: &DirChecked,
    ) -> dir::GenericTable<'static> {
        dir::GenericTable::from_segments(vec![
            declared.generics.clone(),
            checked.generics.clone(),
            self.generics.clone(),
        ])
    }

    /// Return the cumulative definition table for materialized DIR.
    pub fn definition_table(&self, elaborated: &DirElaborated) -> dir::DefinitionTable<'static> {
        dir::DefinitionTable::from_segment(elaborated.definitions.clone())
    }

    /// Return the cumulative coercion table for materialized DIR.
    pub fn coercion_table(&self, checked: &DirChecked) -> dir::CoercionTable<'static> {
        dir::CoercionTable::from_segments(vec![checked.coercions.clone(), self.coercions.clone()])
    }

    /// Return the cumulative capture table for materialized DIR.
    pub fn capture_table(&self, checked: &DirChecked) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segments(vec![checked.captures.clone(), self.captures.clone()])
    }
}
