use std::sync::Arc;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{ByteRange, FileId, ModuleId};
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
    /// Return the token with exactly this source range.
    pub fn token(&self, range: ByteRange) -> Option<dir::Token> {
        let index = self
            .tokens
            .binary_search_by_key(&range.start, |token| token.start())
            .ok()?;
        let token = self.tokens[index];

        (token.end() == range.end).then_some(token)
    }

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

/// Source import resolution for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirImported {
    /// Resolved module imports.
    pub modules: Arc<dir::ModuleSegment>,
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

    /// Return the fingerprint of relationships that determine the component graph.
    pub(crate) fn component_relations_fingerprint(&self) -> ArtifactProjectionFingerprint {
        let mut modules = self.target_modules().collect::<Vec<_>>();
        modules.sort_unstable();
        modules.dedup();

        // collect interface implementations indexed by the module graph
        let mut implementations = self.extensions.implementations().collect::<Vec<_>>();
        implementations.sort_unstable();
        implementations.dedup();

        ArtifactProjectionFingerprint::new(&(modules, implementations))
    }
}

/// Declared DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirDeclared {
    /// The stable fingerprint of this module's declared output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// The modules this artifact's entries mention.
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
    /// Flow conclusions proved while declaring.
    pub flows: Arc<dir::FlowSegment>,
}

/// Elaborated DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirElaborated {
    /// The stable fingerprint of this module's elaborated output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// The modules this artifact's entries mention.
    pub references: Vec<ModuleId>,
    /// Symbols minted while deriving variants and constructors.
    pub bindings: Arc<dir::BindingSegment>,
    /// Types minted while flattening member bindings.
    pub types: Arc<dir::TypeSegment>,
    /// Flattened member bindings per owner subject.
    pub members: Arc<dir::MemberSegment>,
    /// Derived variances.
    pub generics: Arc<dir::GenericSegment>,
    /// Definitions carrying derived constructor entries.
    pub definitions: Arc<dir::DefinitionSegment>,
    /// The layout policies committed for the module's nominal declarations.
    pub representations: Arc<dir::RepresentationSegment>,
    /// Selected and evaluated decorator applications.
    pub decorators: Arc<dir::DecoratorSegment>,
    /// Resolutions recorded while checking declarations and decorators.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// Decisions made while elaborating declarations and decorators.
    pub decisions: Arc<dir::DecisionSegment>,
    /// Static values evaluated for decorator applications.
    pub statics: Arc<dir::StaticSegment>,
    /// Flow conclusions proved while elaborating.
    pub flows: Arc<dir::FlowSegment>,
    /// Diagnostic controls applied by decorators.
    pub controls: Arc<DiagnosticControlTable>,
}

/// Checked DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirChecked {
    /// The stable fingerprint of this module's checked output.
    pub fingerprint: ArtifactProjectionFingerprint,
    /// The modules the checked entries mention.
    pub references: Vec<ModuleId>,
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
    /// New generic slots and the instantiations the bodies perform.
    pub generics: Arc<dir::GenericSegment>,
    /// Member subjects and bindings settled while checking.
    pub members: Arc<dir::MemberSegment>,
    /// New implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
    /// Flow conclusions.
    pub flows: Arc<dir::FlowSegment>,
    /// Whether the values of each checked type copy.
    pub representations: Arc<dir::RepresentationSegment>,
}

/// Materialized DIR for one module under one profile.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirMaterialized {
    /// Tree changes.
    pub patch: dir::Patch,
    /// The symbols the synthesized bodies declare.
    pub bindings: Arc<dir::BindingSegment>,
    /// The lexical names the synthesized bodies resolve.
    pub resolutions: Arc<dir::ResolutionSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// Closed generic instances and their materialized types.
    pub generics: Arc<dir::GenericSegment>,
    /// Grounded node decisions.
    pub decisions: Arc<dir::DecisionSegment>,
    /// Grounded implicit coercions.
    pub coercions: Arc<dir::CoercionSegment>,
    /// Whether the values of each materialized type copy.
    pub representations: Arc<dir::RepresentationSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

/// Analyzed DIR for one module under one profile, projected from the settled module for tooling.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DirAnalyzed {
    /// The types the projected memberships name.
    pub types: Arc<dir::TypeSegment>,
    /// The membership each settled member subject projects.
    pub members: Arc<dir::MemberSegment>,
}

/// The DIR stages of one module read up to one stage.
#[derive(Debug, Clone)]
pub struct DirView {
    /// The parsed stage.
    pub parsed: Arc<DirParsed>,
    /// The bound stage.
    pub bound: Arc<DirBound>,
    /// The imported stage.
    pub imported: Arc<DirImported>,
    /// The expanded stage.
    pub expanded: Arc<DirExpanded>,
    /// The resolved stage.
    pub resolved: Option<Arc<DirResolved>>,
    /// The declared stage.
    pub declared: Option<Arc<DirDeclared>>,
    /// The elaborated stage.
    pub elaborated: Option<Arc<DirElaborated>>,
    /// The checked stage.
    pub checked: Option<Arc<DirChecked>>,
    /// The materialized stage.
    pub materialized: Option<Arc<DirMaterialized>>,
    /// The analyzed stage.
    pub analyzed: Option<Arc<DirAnalyzed>>,

    /// The binding table over the stages read.
    bindings: dir::BindingTable<'static>,
    /// The module table over the stages read.
    modules: dir::ModuleTable<'static>,
    /// The type table over the stages read.
    types: dir::TypeTable<'static>,
    /// The static table over the stages read.
    statics: dir::StaticTable<'static>,
    /// The generic table over the stages read, from the declared stage on.
    generics: Option<dir::GenericTable<'static>>,
    /// The definition table over the stages read, from the declared stage on.
    definitions: Option<dir::DefinitionTable<'static>>,
    /// The member table over the stages read, from the declared stage on.
    members: Option<dir::MemberTable<'static>>,
    /// The decorator table over the stages read, from the declared stage on.
    decorators: Option<dir::DecoratorTable<'static>>,
    /// The resolution table over the stages read, from the declared stage on.
    resolutions: Option<dir::ResolutionTable<'static>>,
    /// The decision table over the stages read, from the declared stage on.
    decisions: Option<dir::DecisionTable<'static>>,
    /// The flow table over the stages read, from the declared stage on.
    flows: Option<dir::FlowTable<'static>>,
    /// The coercion table over the stages read, from the checked stage on.
    coercions: Option<dir::CoercionTable<'static>>,
    /// The capture table over the stages read, from the checked stage on.
    captures: Option<dir::CaptureTable<'static>>,
    /// The representation table over the stages read, from the elaborated stage on.
    representations: Option<dir::RepresentationTable<'static>>,
}

/// Stack one table from the segments the read stages contribute, absent when they contribute none.
fn stacked<S, T>(segments: Vec<Arc<S>>, build: impl FnOnce(Vec<Arc<S>>) -> T) -> Option<T> {
    (!segments.is_empty()).then(|| build(segments))
}

impl DirView {
    /// Stack the stages read through expansion.
    pub fn expanded(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
    ) -> Self {
        Self::new(
            parsed, bound, imported, expanded, None, None, None, None, None, None,
        )
    }

    /// Return the tree with every patch the stacked stages wrote.
    pub fn tree(&self) -> dir::View<'_> {
        let view = dir::View::new(&self.parsed.tree).patched(&self.expanded.patch);

        match &self.materialized {
            Some(materialized) => view.patched(&materialized.patch),
            None => view,
        }
    }

    /// Stack the stages read through declaration.
    pub fn declared(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
    ) -> Self {
        Self::new(
            parsed,
            bound,
            imported,
            expanded,
            Some(resolved),
            Some(declared),
            None,
            None,
            None,
            None,
        )
    }

    /// Stack the stages read through checking.
    pub fn checked(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
        elaborated: Arc<DirElaborated>,
        checked: Arc<DirChecked>,
    ) -> Self {
        Self::new(
            parsed,
            bound,
            imported,
            expanded,
            Some(resolved),
            Some(declared),
            Some(elaborated),
            Some(checked),
            None,
            None,
        )
    }

    /// Stack the stages read through materialization.
    pub fn materialized(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
        elaborated: Arc<DirElaborated>,
        checked: Arc<DirChecked>,
        materialized: Arc<DirMaterialized>,
    ) -> Self {
        Self::new(
            parsed,
            bound,
            imported,
            expanded,
            Some(resolved),
            Some(declared),
            Some(elaborated),
            Some(checked),
            Some(materialized),
            None,
        )
    }

    /// Stack every stage through the analyzed one.
    pub fn analyzed(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        resolved: Arc<DirResolved>,
        declared: Arc<DirDeclared>,
        elaborated: Arc<DirElaborated>,
        checked: Arc<DirChecked>,
        materialized: Arc<DirMaterialized>,
        analyzed: Arc<DirAnalyzed>,
    ) -> Self {
        Self::new(
            parsed,
            bound,
            imported,
            expanded,
            Some(resolved),
            Some(declared),
            Some(elaborated),
            Some(checked),
            Some(materialized),
            Some(analyzed),
        )
    }

    /// Stack the stages read, each retained stage present from the first read on.
    pub fn new(
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        imported: Arc<DirImported>,
        expanded: Arc<DirExpanded>,
        resolved: Option<Arc<DirResolved>>,
        declared: Option<Arc<DirDeclared>>,
        elaborated: Option<Arc<DirElaborated>>,
        checked: Option<Arc<DirChecked>>,
        materialized: Option<Arc<DirMaterialized>>,
        analyzed: Option<Arc<DirAnalyzed>>,
    ) -> Self {
        let mut bindings = vec![bound.bindings.clone(), expanded.bindings.clone()];
        let modules = vec![imported.modules.clone(), expanded.modules.clone()];
        let mut types = vec![bound.types.clone(), expanded.types.clone()];
        let mut statics = vec![bound.statics.clone(), expanded.statics.clone()];
        let mut generics = Vec::new();
        let mut definitions = Vec::new();
        let mut members = Vec::new();
        let mut decorators = Vec::new();
        let mut resolutions = Vec::new();
        let mut decisions = Vec::new();
        let mut flows = Vec::new();
        let mut coercions = Vec::new();
        let mut captures = Vec::new();
        let mut representations = Vec::new();

        // append each retained stage's segments in stage order
        if let Some(declared) = &declared {
            bindings.push(declared.bindings.clone());
            types.push(declared.types.clone());
            statics.push(declared.statics.clone());
            generics.push(declared.generics.clone());
            definitions.push(declared.definitions.clone());
            members.push(declared.members.clone());
            decorators.push(declared.decorators.clone());
            resolutions.push(declared.resolutions.clone());
            decisions.push(declared.decisions.clone());
            flows.push(declared.flows.clone());
        }
        if let Some(elaborated) = &elaborated {
            bindings.push(elaborated.bindings.clone());
            types.push(elaborated.types.clone());
            statics.push(elaborated.statics.clone());
            generics.push(elaborated.generics.clone());
            definitions.push(elaborated.definitions.clone());
            members.push(elaborated.members.clone());
            decorators.push(elaborated.decorators.clone());
            resolutions.push(elaborated.resolutions.clone());
            decisions.push(elaborated.decisions.clone());
            flows.push(elaborated.flows.clone());
            representations.push(elaborated.representations.clone());
        }
        if let Some(checked) = &checked {
            bindings.push(checked.bindings.clone());
            types.push(checked.types.clone());
            statics.push(checked.statics.clone());
            generics.push(checked.generics.clone());
            members.push(checked.members.clone());
            decorators.push(checked.decorators.clone());
            resolutions.push(checked.resolutions.clone());
            decisions.push(checked.decisions.clone());
            flows.push(checked.flows.clone());
            coercions.push(checked.coercions.clone());
            captures.push(checked.captures.clone());
            representations.push(checked.representations.clone());
        }
        if let Some(materialized) = &materialized {
            bindings.push(materialized.bindings.clone());
            types.push(materialized.types.clone());
            generics.push(materialized.generics.clone());
            resolutions.push(materialized.resolutions.clone());
            decisions.push(materialized.decisions.clone());
            coercions.push(materialized.coercions.clone());
            representations.push(materialized.representations.clone());
        }
        if let Some(analyzed) = &analyzed {
            types.push(analyzed.types.clone());
            members.push(analyzed.members.clone());
        }

        Self {
            parsed,
            bound,
            imported,
            expanded,
            resolved,
            declared,
            elaborated,
            checked,
            materialized,
            analyzed,
            bindings: dir::BindingTable::from_segments(bindings),
            modules: dir::ModuleTable::from_segments(modules),
            types: dir::TypeTable::from_segments(types),
            statics: dir::StaticTable::from_segments(statics),
            generics: stacked(generics, dir::GenericTable::from_segments),
            definitions: stacked(definitions, dir::DefinitionTable::from_segments),
            members: stacked(members, dir::MemberTable::from_segments),
            decorators: stacked(decorators, dir::DecoratorTable::from_segments),
            resolutions: stacked(resolutions, dir::ResolutionTable::from_segments),
            decisions: stacked(decisions, dir::DecisionTable::from_segments),
            flows: stacked(flows, dir::FlowTable::from_segments),
            coercions: stacked(coercions, dir::CoercionTable::from_segments),
            captures: stacked(captures, dir::CaptureTable::from_segments),
            representations: stacked(representations, dir::RepresentationTable::from_segments),
        }
    }

    /// Return the parsed stage.
    pub fn parsed(&self) -> &Arc<DirParsed> {
        &self.parsed
    }

    /// Return the resolved stage.
    pub fn resolved(&self) -> &Arc<DirResolved> {
        self.resolved
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a resolved stage"))
    }

    /// Return the binding table.
    pub fn bindings(&self) -> &dir::BindingTable<'static> {
        &self.bindings
    }

    /// Return the module table.
    pub fn modules(&self) -> &dir::ModuleTable<'static> {
        &self.modules
    }

    /// Return the type table.
    pub fn types(&self) -> &dir::TypeTable<'static> {
        &self.types
    }

    /// Return the static table.
    pub fn statics(&self) -> &dir::StaticTable<'static> {
        &self.statics
    }

    /// Return the generic table.
    pub fn generics(&self) -> &dir::GenericTable<'static> {
        self.generics
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the definition table.
    pub fn definitions(&self) -> &dir::DefinitionTable<'static> {
        self.definitions
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the member table.
    pub fn members(&self) -> &dir::MemberTable<'static> {
        self.members
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the decorator table.
    pub fn decorators(&self) -> &dir::DecoratorTable<'static> {
        self.decorators
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the resolution table.
    pub fn resolutions(&self) -> &dir::ResolutionTable<'static> {
        self.resolutions
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the decision table.
    pub fn decisions(&self) -> &dir::DecisionTable<'static> {
        self.decisions
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the flow table.
    pub fn flows(&self) -> &dir::FlowTable<'static> {
        self.flows
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a declared stage"))
    }

    /// Return the coercion table.
    pub fn coercions(&self) -> &dir::CoercionTable<'static> {
        self.coercions
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a checked stage"))
    }

    /// Return the capture table.
    pub fn captures(&self) -> &dir::CaptureTable<'static> {
        self.captures
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without a checked stage"))
    }

    /// Return the representation table.
    pub fn representations(&self) -> &dir::RepresentationTable<'static> {
        self.representations
            .as_ref()
            .unwrap_or_else(|| unreachable!("DIR view without an elaborated stage"))
    }
}
