use std::slice::from_ref;
use std::sync::Arc;

use destack_artifact::{
    DiagnosticBuilder, DiagnosticControlTable, DirBound, DirChecked, DirDeclared, DirElaborated,
    DirExpanded, DirParsed, DirResolved, ProfileKey,
};
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_repository::{ArtifactReader, Module, Package, ProviderContext};

use destack_source::{ModuleId, ProfileId, Span};
use smallvec::SmallVec;

use crate::sema::{
    Capture, Cause, CauseKind, CheckError, CheckEvent, CheckState, CheckWarning, FlowPoint,
    FlowPointId, FlowSite, Origin, Pass, Relation, RelationCheck, StaticPresence,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// Working state owned by one checked module.
pub(in crate::sema) struct CheckModuleState {
    // module context
    /// The requested source module.
    pub(in crate::sema) module: Arc<Module>,
    /// The package containing the requested module.
    pub(in crate::sema) package: Arc<Package>,
    /// The active target profile.
    pub(in crate::sema) profile: ProfileKey,

    // retained stages in phase order
    /// The parsed DIR input.
    pub(in crate::sema) parsed: Arc<DirParsed>,
    /// The bound DIR input.
    pub(in crate::sema) bound: Arc<DirBound>,
    /// The resolved DIR input.
    pub(in crate::sema) resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    pub(in crate::sema) expanded: Arc<DirExpanded>,
    /// The declared DIR artifact seeding this check, absent while declaring.
    pub(in crate::sema) declared: Option<Arc<DirDeclared>>,
    /// The elaborated stage backing this check, when elaborating is done.
    pub(in crate::sema) elaborated: Option<Arc<DirElaborated>>,
    /// The checked artifact materialization layers over.
    pub(in crate::sema) checked: Option<Arc<DirChecked>>,

    // referenced modules
    /// The foreign modules the stored entries mention, accumulated at store time.
    pub(in crate::sema) references: FxIndexSet<ModuleId>,
    /// External modules visible from this module.
    pub(in crate::sema) external_modules: FxIndexSet<ModuleId>,

    // committed bases built once at load
    /// The cumulative binding table.
    pub(in crate::sema) bindings: dir::BindingTable<'static>,
    /// The committed base type table.
    pub(in crate::sema) types: dir::TypeTable<'static>,
    /// The committed base static table.
    pub(in crate::sema) statics: dir::StaticTable<'static>,
    /// The committed definitions this pass shadows.
    pub(in crate::sema) definitions: Vec<Arc<dir::DefinitionSegment>>,
    /// The committed member entries this pass shadows.
    pub(in crate::sema) members: Vec<Arc<dir::MemberSegment>>,

    // open tails this pass writes over the committed bases
    /// Checked symbols synthesized from resolved language features.
    pub(in crate::sema) bindings_tail: dir::BindingSegment,
    /// Open inference types layered over the committed base.
    pub(in crate::sema) types_tail: dir::TypeTail,
    /// The static terms this pass evaluated.
    pub(in crate::sema) statics_tail: dir::StaticSegment,
    /// The definitions this pass declared or rewrote.
    pub(in crate::sema) definitions_tail: dir::DefinitionSegment,
    /// The member subjects and bindings this pass selected.
    pub(in crate::sema) members_tail: dir::MemberSegment,
    /// The committed base generic table.
    pub(in crate::sema) generics: dir::GenericTable<'static>,
    /// The generic templates and parameters this pass induced.
    pub(in crate::sema) generics_tail: dir::GenericSegment,
    /// The decorators this pass checked.
    pub(in crate::sema) decorators_tail: dir::DecoratorSegment,

    // pass-only segments with no committed base
    /// Checked node resolutions.
    pub(in crate::sema) resolutions: dir::ResolutionSegment,
    /// Decisions inference made this pass.
    pub(in crate::sema) decisions: dir::DecisionSegment,
    /// Checked implicit coercions.
    pub(in crate::sema) coercions: dir::CoercionSegment,
    /// Checked captures.
    pub(in crate::sema) captures: dir::CaptureSegment,
    /// Checked flow conclusions.
    pub(in crate::sema) flows: dir::FlowSegment,
    /// Checked diagnostic controls, an owned output seeded from elaborated.
    pub(in crate::sema) controls: DiagnosticControlTable,

    // walk state drained during output
    /// Inferred static symbol values written to statics during output.
    pub(in crate::sema) static_values: FxIndexMap<dir::GlobalSymbolId, dir::GlobalTypeId>,
    /// Captures discovered while walking this module.
    pub(in crate::sema) pending_captures: Vec<Capture>,
    /// Durable flow states discovered while walking this module.
    pub(in crate::sema) flow_points: Vec<FlowPoint>,
    /// Entry flow point for each walked source node occurrence.
    pub(in crate::sema) node_flows: FxIndexMap<dir::GlobalNodeIdAny, FlowPointId>,
    /// Generic template assumed by each checked source node.
    pub(in crate::sema) node_scopes:
        FxIndexMap<dir::GlobalNodeIdAny, Option<dir::GlobalGenericTemplateId>>,
    /// Source nodes whose end no control path reaches.
    pub(in crate::sema) unreachable_ends: FxIndexSet<dir::LocalNodeIdAny>,

    // statically false gates
    /// Presence decisions for decorated source nodes.
    pub(in crate::sema) static_presence: FxIndexMap<dir::LocalNodeIdAny, StaticPresence>,
    /// Declarations whose guards decided statically false.
    pub(in crate::sema) absent_symbols: FxIndexSet<dir::GlobalSymbolId>,

    // diagnostics drained during write
    /// Diagnostics reported while walking this module.
    pub(in crate::sema) diagnostics: Vec<DiagnosticBuilder<CheckError>>,
    /// Warnings reported while walking this module.
    pub(in crate::sema) warnings: Vec<DiagnosticBuilder<CheckWarning>>,
}

impl CheckModuleState {
    /// Load the module state required by one compiler pass.
    pub(in crate::sema) fn load(
        compiler: &Compiler,
        context: &dyn ProviderContext,
        artifacts: &ArtifactReader<'_>,
        profile: ProfileId,
        module: ModuleId,
        pass: Pass,
    ) -> CompilerResult<Self> {
        let profile_key = compiler.profile(context.revision(), profile)?.key;
        let repository_module = compiler.module(context.revision(), module)?;
        let package = compiler.package(context.revision(), repository_module.package_id)?;
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile))
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .read::<DirResolved>((module, profile))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module, profile))
            .map_err(CompilerError::from)?;

        // read the retained layers preceding this pass
        let declared = (pass != Pass::Declare)
            .then(|| artifacts.read::<DirDeclared>((module, profile)))
            .transpose()
            .map_err(CompilerError::from)?;
        let elaborated = matches!(pass, Pass::Check | Pass::Materialize)
            .then(|| artifacts.read::<DirElaborated>((module, profile)))
            .transpose()
            .map_err(CompilerError::from)?;
        let checked = (pass == Pass::Materialize)
            .then(|| artifacts.read::<DirChecked>((module, profile)))
            .transpose()
            .map_err(CompilerError::from)?;

        Ok(Self::new(
            repository_module,
            package,
            profile_key,
            parsed,
            bound,
            resolved,
            expanded,
            declared,
            elaborated,
            checked,
        ))
    }

    /// Create module state from loaded inputs and empty checked state.
    pub(in crate::sema) fn new(
        module: Arc<Module>,
        package: Arc<Package>,
        profile: ProfileKey,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
        declared: Option<Arc<DirDeclared>>,
        elaborated: Option<Arc<DirElaborated>>,
        checked: Option<Arc<DirChecked>>,
    ) -> Self {
        // stack materialization over the checked segments when present
        let checked_layer = checked.as_ref().map(|checked| {
            let declared = declared
                .as_ref()
                .expect("materialization layers over declared");
            let elaborated = elaborated
                .as_ref()
                .expect("materialization layers over elaborated");

            (
                checked.binding_table(&bound, &expanded, declared, elaborated),
                checked.type_table(&bound, &expanded, declared, elaborated),
                checked.static_table(&bound, &expanded, declared, elaborated),
                dir::TypeTail::over(vec![
                    Arc::clone(&declared.types),
                    Arc::clone(&elaborated.types),
                    Arc::clone(&checked.types),
                ]),
                dir::GenericSegment::from_base(&checked.generics),
                dir::StaticSegment::from_base(&checked.statics),
                dir::DecoratorSegment::from_base(&checked.decorators),
            )
        });

        // stack this check's overlays over the declared segments, else the expanded base
        let (bindings, types, statics, types_tail, generics_tail, statics_tail, decorators_tail) =
            if let Some(layer) = checked_layer {
                layer
            } else {
                match &declared {
                    Some(declared) => (
                        match &elaborated {
                            Some(elaborated) => {
                                elaborated.binding_table(&bound, &expanded, declared)
                            }
                            None => declared.binding_table(&bound, &expanded),
                        },
                        match &elaborated {
                            Some(elaborated) => elaborated.type_table(&bound, &expanded, declared),
                            None => declared.type_table(&bound, &expanded),
                        },
                        match &elaborated {
                            Some(elaborated) => {
                                elaborated.static_table(&bound, &expanded, declared)
                            }
                            None => declared.static_table(&bound, &expanded),
                        },
                        match &elaborated {
                            Some(elaborated) => dir::TypeTail::over(vec![
                                Arc::clone(&declared.types),
                                Arc::clone(&elaborated.types),
                            ]),
                            None => dir::TypeTail::over_base(Arc::clone(&declared.types)),
                        },
                        match &elaborated {
                            Some(elaborated) => {
                                dir::GenericSegment::from_base(&elaborated.generics)
                            }
                            None => dir::GenericSegment::from_base(&declared.generics),
                        },
                        match &elaborated {
                            Some(elaborated) => dir::StaticSegment::from_base(&elaborated.statics),
                            None => dir::StaticSegment::from_base(&declared.statics),
                        },
                        match &elaborated {
                            Some(elaborated) => {
                                dir::DecoratorSegment::from_base(&elaborated.decorators)
                            }
                            None => dir::DecoratorSegment::from_base(&declared.decorators),
                        },
                    ),
                    None => (
                        expanded.binding_table(&bound),
                        expanded.type_table(&bound),
                        expanded.static_table(&bound),
                        dir::TypeTail::from_base(&expanded.types),
                        dir::GenericSegment::new(module.id),
                        dir::StaticSegment::from_base(&expanded.statics),
                        dir::DecoratorSegment::new(module.id),
                    ),
                }
            };
        let bindings_tail = dir::BindingSegment::from_table(&bindings);

        // stack the committed generic segments in stage order under one view
        let mut generic_segments = Vec::new();
        if let Some(declared) = &declared {
            generic_segments.push(Arc::clone(&declared.generics));
        }
        if let Some(elaborated) = &elaborated {
            generic_segments.push(Arc::clone(&elaborated.generics));
        }
        if let Some(checked) = &checked {
            generic_segments.push(Arc::clone(&checked.generics));
        }
        if generic_segments.is_empty() {
            generic_segments.push(Arc::new(dir::GenericSegment::new(module.id)));
        }
        let generics = dir::GenericTable::from_segments(generic_segments);

        // shadow the committed definitions and member entries, which key by symbol and site
        let mut definitions = Vec::new();
        let mut members = Vec::new();
        if let Some(checked) = &checked {
            definitions.push(checked.definitions.clone());
            members.push(checked.members.clone());
        }
        if let Some(elaborated) = &elaborated {
            definitions.push(elaborated.definitions.clone());
            members.push(elaborated.members.clone());
        }
        if let Some(declared) = &declared {
            definitions.push(declared.definitions.clone());
            members.push(declared.members.clone());
        }
        let definitions_tail = dir::DefinitionSegment::new(module.id);
        let members_tail = dir::MemberSegment::new(module.id);

        // open the remaining segments and this module's diagnostic controls
        let resolutions = dir::ResolutionSegment::new(module.id);
        let decisions = dir::DecisionSegment::new(module.id);
        let coercions = dir::CoercionSegment::new(module.id);
        let captures = dir::CaptureSegment::new(module.id);
        let flows = dir::FlowSegment::new(module.id);
        // carry the diagnostic controls the latest stage wrote
        let controls = match (&checked, &elaborated) {
            (Some(checked), _) => (*checked.controls).clone(),
            (None, Some(elaborated)) => (*elaborated.controls).clone(),
            (None, None) => {
                let files = parsed.files.iter().map(|file| file.file_id).collect();

                DiagnosticControlTable::new(module.id, files)
            }
        };

        Self {
            module,
            package,
            profile,
            parsed,
            bound,
            resolved,
            expanded,
            declared,
            elaborated,
            checked,
            references: FxIndexSet::default(),
            external_modules: FxIndexSet::default(),
            bindings,
            types,
            statics,
            definitions,
            members,
            bindings_tail,
            types_tail,
            statics_tail,
            definitions_tail,
            members_tail,
            generics,
            generics_tail,
            decorators_tail,
            resolutions,
            decisions,
            coercions,
            captures,
            flows,
            controls,
            static_values: FxIndexMap::default(),
            static_presence: FxIndexMap::default(),
            absent_symbols: FxIndexSet::default(),
            pending_captures: Vec::new(),
            flow_points: Vec::new(),
            node_flows: FxIndexMap::default(),
            node_scopes: FxIndexMap::default(),
            unreachable_ends: FxIndexSet::default(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::sema) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(&self.parsed.tree, from_ref(&self.expanded.patch))
    }

    /// Return the full source span of one visible node's authored origin.
    pub(in crate::sema) fn source_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let source = self.view().get_source_any(node);

        self.parsed.tree.get_span_by_id(source)
    }

    /// Return the authored diagnostic span of one visible node.
    pub(in crate::sema) fn diagnostic_span(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        let view = self.view();
        let source = view.get_source_any(node);

        // prefer the authored node that produced the visible node
        if let Some(span) = self.parsed.tree.get_main_span_by_id(source) {
            return Some(span);
        }
        if let Some(span) = self.parsed.tree.get_span_by_id(source) {
            return Some(span);
        }

        view.get_span_by_id(node.id)
    }

    /// Return the member membership stored for one subject, reading the pass tail over the base.
    pub(in crate::sema) fn membership(
        &self,
        subject: &dir::MemberSubject,
    ) -> Option<&dir::Membership> {
        if let Some(membership) = self.members_tail.membership(subject) {
            return Some(membership);
        }

        self.members
            .iter()
            .find_map(|base| base.membership(subject))
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::sema) fn binding_table(&self) -> dir::BindingTable<'_> {
        self.bindings.with_tail(&self.bindings_tail)
    }

    /// Return one generic template, reading the pass tail over the committed base.
    pub(in crate::sema) fn generic_template(
        &self,
        id: dir::LocalGenericTemplateId,
    ) -> Option<&dir::GenericTemplate> {
        if let Some(template) = self.generics_tail.get_local_template(id) {
            return Some(template);
        }

        self.generics.get_template_maybe(id)
    }

    /// Return one generic parameter, reading the pass tail over the committed base.
    pub(in crate::sema) fn generic_parameter(
        &self,
        id: dir::LocalGenericParameterId,
    ) -> Option<&dir::GenericParameterBinding> {
        if let Some(parameter) = self.generics_tail.get_local_parameter(id) {
            return Some(parameter);
        }

        self.generics.get_parameter_maybe(id)
    }

    /// Return the template written for one source node, taking the pass tail first.
    pub(in crate::sema) fn written_template_by_source(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::LocalGenericTemplateId> {
        self.generics_tail.template_by_source(source).or_else(|| {
            self.declared
                .as_ref()
                .and_then(|declared| declared.generics.template_by_source(source))
        })
    }

    /// Return the template written for one symbol, reading the pass tail over the declared stage.
    pub(in crate::sema) fn written_template_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalGenericTemplateId> {
        self.generics_tail.template_by_symbol(symbol).or_else(|| {
            self.declared
                .as_ref()
                .and_then(|declared| declared.generics.template_by_symbol(symbol))
        })
    }

    /// Return the parameter written for one symbol, reading the pass tail over the declared stage.
    pub(in crate::sema) fn written_parameter_by_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::LocalGenericParameterId> {
        self.generics_tail.parameter_by_symbol(symbol).or_else(|| {
            self.declared
                .as_ref()
                .and_then(|declared| declared.generics.parameter_by_symbol(symbol))
        })
    }

    /// Return the template written for one scope, reading the pass tail over the declared stage.
    pub(in crate::sema) fn written_template_by_scope(
        &self,
        scope: dir::LocalScopeId,
    ) -> Option<dir::LocalGenericTemplateId> {
        self.generics_tail.template_by_scope(scope).or_else(|| {
            self.declared
                .as_ref()
                .and_then(|declared| declared.generics.template_by_scope(scope))
        })
    }

    /// Return the cardinality one parameter records, reading the pass tail over its seed stages.
    pub(in crate::sema) fn parameter_cardinality(
        &self,
        id: dir::LocalGenericParameterId,
    ) -> Option<dir::Cardinality> {
        self.generics_tail
            .cardinality(id)
            .or_else(|| {
                self.elaborated
                    .as_ref()
                    .and_then(|elaborated| elaborated.generics.cardinality(id))
            })
            .or_else(|| {
                self.declared
                    .as_ref()
                    .and_then(|declared| declared.generics.cardinality(id))
            })
    }

    /// Return one symbol, reading the pass tail over the committed base.
    pub(in crate::sema) fn symbol(&self, symbol: dir::LocalSymbolId) -> &dir::Symbol {
        match self.bindings_tail.get_symbol_maybe(symbol) {
            Some(symbol) => symbol,
            None => self.bindings.get_symbol(symbol),
        }
    }

    /// Return the symbol introduced by a source declaration node.
    pub(in crate::sema) fn declaration_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .declaration_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return the implicit receiver symbol introduced for one member node.
    pub(in crate::sema) fn implicit_receiver_symbol(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let symbol = self
            .binding_table()
            .implicit_receiver_symbol(node.into_global(self.module.id))?;

        Some(symbol.into_global(self.module.id))
    }

    /// Return one source symbol's declaration node.
    pub(in crate::sema) fn symbol_declaration_node(
        &self,
        symbol: dir::LocalSymbolId,
    ) -> CompilerResult<dir::LocalNodeIdAny> {
        let bindings = self.binding_table();
        let binding = bindings.get_symbol(symbol);

        // require source symbols to have local declaration nodes
        let Some(declaration) = binding.declaration else {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} has no declaration node"),
            });
        };
        if declaration.module_id != self.module.id {
            return Err(CompilerError::Internal {
                message: format!("source symbol {symbol:?} declaration points outside its module"),
            });
        }

        Ok(declaration.local_id)
    }

    /// Return one local input static visible to check.
    pub(in crate::sema) fn r#static(&self, static_id: dir::LocalStaticId) -> &dir::StaticTerm {
        // read this pass's own terms, then the committed and bound bases
        if let Some(value) = self.statics_tail.get_static_maybe(static_id) {
            return value;
        }

        // read the value from the pass segment
        if let Some(value) = self.statics.get_static_maybe(static_id) {
            return value;
        }

        // read the value from the expanded stage
        if let Some(value) = self.expanded.statics.get_static_maybe(static_id) {
            return value;
        }

        self.bound.statics.get_static(static_id)
    }

    /// Return whether one local symbol is an imported alias.
    pub(in crate::sema) fn is_import_alias(&self, symbol: dir::LocalSymbolId) -> bool {
        self.resolved.imports.symbol_resolution(symbol).is_some()
    }

    /// Return one definition, reading the pass tail over the committed base.
    pub(in crate::sema) fn definition(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&dir::Definition> {
        // read this pass's own definitions first
        if let Some(definition) = self.definitions_tail.definition(symbol) {
            return Some(definition);
        }

        self.definitions
            .iter()
            .find_map(|base| base.definition(symbol))
    }

    /// Return one definition for rewriting, copying the committed base in once.
    pub(in crate::sema) fn definition_mut(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<&mut dir::Definition> {
        // copy the committed definition into the pass tail on first write
        if self.definitions_tail.definition(symbol).is_none() {
            let base = self
                .definitions
                .iter()
                .find(|base| base.definition(symbol).is_some())?;
            let definition = base.definition(symbol)?.clone();
            let source = base.definition_source_maybe(symbol)?;
            self.definitions_tail
                .insert_definition(symbol, source, definition);
        }

        self.definitions_tail.definition_mut(symbol)
    }

    /// Return one definition's source node through the segments.
    pub(in crate::sema) fn definition_source_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalNodeIdAny> {
        // read this pass's own sources first
        if let Some(source) = self.definitions_tail.definition_source_maybe(symbol) {
            return Some(source);
        }

        self.definitions
            .iter()
            .find_map(|base| base.definition_source_maybe(symbol))
    }

    /// Iterate definitions with pass entries shadowing the committed base.
    pub(in crate::sema) fn iter_definitions(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, &dir::Definition)> + '_ {
        // drop the base entries this pass or a newer base redefined
        let shadowed = self
            .definitions
            .iter()
            .enumerate()
            .flat_map(|(depth, base)| base.iter_definitions().map(move |entry| (depth, entry)))
            .filter(|(depth, (symbol, _))| {
                self.definitions_tail.definition(*symbol).is_none()
                    && !self.definitions[..*depth]
                        .iter()
                        .any(|newer| newer.definition(*symbol).is_some())
            })
            .map(|(_, entry)| entry);

        shadowed.chain(self.definitions_tail.iter_definitions())
    }

    /// Return the extension symbols targeting one head across the segments.
    pub(in crate::sema) fn root_extensions(&self, root: dir::TypeRoot) -> Vec<dir::GlobalSymbolId> {
        // collect this pass's symbols, then the base symbols beneath them
        let mut symbols: Vec<_> = self.definitions_tail.root_extensions(root).to_vec();
        for base in self.definitions.iter() {
            for symbol in base.root_extensions(root) {
                if !symbols.contains(symbol) {
                    symbols.push(*symbol);
                }
            }
        }

        symbols
    }

    /// Return the blanket extension symbols across the segments.
    pub(in crate::sema) fn blanket_extensions(&self) -> Vec<dir::GlobalSymbolId> {
        // collect this pass's symbols, then the base symbols beneath them
        let mut symbols: Vec<_> = self.definitions_tail.blanket_extensions().to_vec();
        for base in self.definitions.iter() {
            for symbol in base.blanket_extensions() {
                if !symbols.contains(symbol) {
                    symbols.push(*symbol);
                }
            }
        }

        symbols
    }

    /// Return the member subject and key at one site, reading the pass tail over the base.
    pub(in crate::sema) fn member_subject(
        &self,
        site: dir::MemberSite,
    ) -> Option<(dir::MemberSubject, Option<dir::StaticKey>)> {
        self.members_tail
            .subject(site)
            .or_else(|| self.members.iter().find_map(|base| base.subject(site)))
    }

    /// Iterate member subjects and keys with pass entries shadowing the committed bases.
    pub(in crate::sema) fn iter_member_subjects(
        &self,
    ) -> impl Iterator<Item = (dir::MemberSite, dir::MemberSubject, Option<dir::StaticKey>)> + '_
    {
        // drop the base entries a newer segment reselected
        let shadowed = self
            .members
            .iter()
            .enumerate()
            .flat_map(|(depth, base)| base.iter_subjects().map(move |entry| (depth, entry)))
            .filter(|(depth, (site, _, _))| {
                self.members_tail.subject(*site).is_none()
                    && !self.members[..*depth]
                        .iter()
                        .any(|newer| newer.subject(*site).is_some())
            })
            .map(|(_, entry)| entry);

        shadowed.chain(self.members_tail.iter_subjects())
    }
}

impl CheckState<'_> {
    /// Return whether one module is the checked module.
    pub(in crate::sema) fn is_own_module(&self, module: ModuleId) -> bool {
        module == self.module_id
    }

    /// Return whether one module's declared tables are readable.
    pub(in crate::sema) fn is_loaded_module(&self, module: ModuleId) -> bool {
        self.is_own_module(module) || self.external_modules.contains_key(&module)
    }

    /// Return the module's working state when it is the checked module.
    pub(in crate::sema) fn module_maybe(&self, module: ModuleId) -> Option<&CheckModuleState> {
        (module == self.module_id).then_some(&self.module)
    }

    /// Return the mutable working state when it is the checked module.
    pub(in crate::sema) fn module_maybe_mut(
        &mut self,
        module: ModuleId,
    ) -> Option<&mut CheckModuleState> {
        (module == self.module_id).then_some(&mut self.module)
    }

    /// Return the symbols their static guards leave present.
    pub(in crate::sema) fn present_symbols(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        symbols
            .iter()
            .copied()
            .filter(|symbol| !self.is_absent_symbol(*symbol))
            .collect()
    }

    /// Return the unique live symbol referenced by one source node.
    pub(in crate::sema) fn reference_symbol(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalSymbolId> {
        let reference = self
            .module_resolved(source.module_id)
            .references
            .get(source)?;
        let dir::Reference::Bound(symbols) = reference else {
            return None;
        };
        let symbols = self.present_symbols(symbols);
        let [symbol] = symbols.as_slice() else {
            return None;
        };

        Some(*symbol)
    }

    /// Return whether one symbol's guard decided statically false.
    pub(in crate::sema) fn is_absent_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.module_maybe(symbol.module_id)
            .is_some_and(|module| module.absent_symbols.contains(&symbol))
    }

    /// Return whether one source node is inside a statically absent subtree.
    pub(in crate::sema) fn is_absent(&self, node: dir::GlobalNodeIdAny) -> bool {
        let module = self.module(node.module_id);
        let statics = module.statics.with_tail(&module.statics_tail);

        statics.is_absent(module.view(), node.local_id)
    }

    /// Return the checked module's working state.
    pub(in crate::sema) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.module_maybe(module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return the checked module's working state mutably.
    pub(in crate::sema) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.module_maybe_mut(module) {
            Some(state) => state,
            None => unreachable!("check module {module:?} was not loaded"),
        }
    }

    /// Return one source node type without flow narrowing, if present.
    pub(in crate::sema) fn committed_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        // read this pass's own commits first
        if let Some(ty) = self.own_node_type(node) {
            return Some(ty);
        }

        // read own declared-stage node types
        if let Some(module) = self.module_maybe(node.module_id) {
            return module.types.get_node_type_id(node);
        }

        None
    }

    /// Return one source node type without flow narrowing.
    pub(in crate::sema) fn node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.committed_node_type(node) {
            Some(ty) => Ok(ty),
            None => Err(CompilerError::Internal {
                message: format!("node type read before checking {node:?}"),
            }),
        }
    }

    /// Return an invariant label for one source node.
    pub(in crate::sema) fn node_label(&self, node: dir::GlobalNodeIdAny) -> String {
        let module = self.module(node.module_id);
        let uri = module.module.uri.as_ref();

        // include source position without inspecting the node's syntax variant
        if let Some(span) = module.diagnostic_span(node.local_id) {
            format!("node {node:?} in {uri} at {span:?}")
        } else {
            format!("node {node:?} in {uri}")
        }
    }

    /// Return one source node type through its solution, without flow narrowing.
    pub(in crate::sema) fn require_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self
            .committed_node_type(node)
            .map(|ty| self.shallow_resolve(ty))
            .transpose()?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "required node has no checked type: {}; decision={:?}",
                    self.node_label(node),
                    self.decision(node),
                ),
            });
        };

        Ok(ty)
    }

    /// Return one node type this pass committed, a hole reading through its solution.
    pub(in crate::sema) fn own_node_type(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        let ty = self.node_types.get(&node)?;

        self.shallow_resolve(ty).ok()
    }

    /// Commit one source node type.
    pub(in crate::sema) fn commit_node_type(
        &mut self,
        node: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        // write each node once
        if let Some(previous) = self.node_types.get(&node) {
            if previous == ty {
                return Ok(());
            }

            let previous = self.format_type(previous);
            let ty = self.format_type(ty);
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} received two types: {previous} and {ty}"),
            });
        }

        self.infer.log_node_type(node);
        self.node_types.insert(node, ty);

        Ok(())
    }

    /// Commit the error type for one refused source node.
    pub(in crate::sema) fn commit_error_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if let Some(ty) = self.committed_node_type(node) {
            return Ok(ty);
        }

        let ty = self.intern_type(dir::Type::Error)?;
        self.commit_node_type(node, ty)?;

        Ok(ty)
    }

    /// Enter one source node at the live cursor, storing its flow site.
    pub(in crate::sema) fn visit_site(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<FlowSite> {
        // reuse a site the walk already stored for this node
        if let Some(flow) = self.module(node.module_id).node_flows.get(&node).copied() {
            let scope = self
                .module(node.module_id)
                .node_scopes
                .get(&node)
                .copied()
                .flatten();

            return Ok(FlowSite { node, flow, scope });
        }

        // record this node's flow point and template scope
        let flow = self.flow.point();
        let scope = self.flow.template_scope();
        let state = self.module_mut(node.module_id);
        state.node_flows.insert(node, flow);
        state.node_scopes.insert(node, scope);

        Ok(FlowSite { node, flow, scope })
    }

    /// Return the stored flow site of one visited node.
    pub(in crate::sema) fn node_site(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<FlowSite> {
        let Some(flow) = self.module(node.module_id).node_flows.get(&node).copied() else {
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded runtime flow"),
            });
        };
        let origin = self.node_origin(node)?;
        let Origin::Node(_, scope) = origin else {
            unreachable!("node origin must name a source node")
        };

        Ok(FlowSite { node, flow, scope })
    }

    /// Return the work origin for one walked source node, or none.
    pub(in crate::sema) fn node_origin_maybe(&self, node: dir::GlobalNodeIdAny) -> Option<Origin> {
        let scope = self
            .module(node.module_id)
            .node_scopes
            .get(&node)
            .copied()?;

        Some(Origin::Node(node, scope))
    }

    /// Return the work origin for one checked source node.
    pub(in crate::sema) fn node_origin(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Origin> {
        let Some(scope) = self.module(node.module_id).node_scopes.get(&node).copied() else {
            let node = self.node_label(node);

            return Err(CompilerError::Internal {
                message: format!("check node {node} has no recorded origin"),
            });
        };

        Ok(Origin::Node(node, scope))
    }

    /// Return one declaration type committed this pass, if present.
    pub(in crate::sema) fn declaration_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.declaration_types.get(&symbol).copied()
    }

    /// Commit one declaration symbol type.
    pub(in crate::sema) fn commit_declaration_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.declaration_type_maybe(symbol) {
            if previous == ty {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "declaration symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.declaration_types.insert(symbol, ty);
        self.solve_symbol_variable(symbol, ty)?;

        Ok(())
    }

    /// Return one binding type committed this pass, if present.
    pub(in crate::sema) fn binding_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        self.binding_types.get(&symbol).copied()
    }

    /// Commit one binding symbol type.
    pub(in crate::sema) fn commit_binding_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        if let Some(previous) = self.binding_type_maybe(symbol) {
            if previous == ty {
                return Ok(());
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "binding symbol {symbol:?} already has type {previous:?}, got {ty:?}"
                ),
            });
        }

        self.binding_types.insert(symbol, ty);
        self.solve_symbol_variable(symbol, ty)?;

        Ok(())
    }

    /// Solve one symbol's variable with its committed type.
    fn solve_symbol_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let Some(variable) = self.infer.symbol_variables.get(&symbol).copied() else {
            return Ok(());
        };
        if !self.infer.variable(variable)?.state.is_open() {
            return Ok(());
        }

        // solve closed types; open types resolve through their own inference
        if self.type_variables(ty)?.is_empty() {
            self.commit_solution(variable, ty)?;
        }

        Ok(())
    }

    /// Commit one symbol's exact checked type into the table its kind owns.
    pub(in crate::sema) fn commit_symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the type this symbol already committed, if any
        let is_binding = self.symbol_kind(symbol)?.is_binding();
        let existing = if is_binding {
            self.binding_type_maybe(symbol)
        } else {
            self.declaration_type_maybe(symbol)
        };

        // require a second derivation to agree with the committed type
        if let Some(existing) = existing {
            let origin = self.intern_origin(Origin::Symbol(symbol));
            let origin = self.infer.origin(origin);
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.push_relation(RelationCheck::new(
                origin,
                Relation::Equal,
                existing,
                ty,
                cause,
            ))?;

            return Ok(existing);
        }

        // commit into the table matching the symbol's kind
        if is_binding {
            self.commit_binding_type(symbol, ty)?;
        } else {
            self.commit_declaration_type(symbol, ty)?;
        }

        Ok(ty)
    }

    /// Return one loaded symbol's checked type, if present.
    pub(in crate::sema) fn symbol_type_maybe(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // prefer body-owned bindings over stable declarations
        if let Some(ty) = self.binding_type_maybe(symbol) {
            return Some(ty);
        }
        if let Some(ty) = self.declaration_type_maybe(symbol) {
            return Some(ty);
        }

        // read own declared-stage symbol types, treating open variables as absent
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(ty) = module.types.get_symbol_type_id(symbol)
            && !self.type_flags(ty).is_ok_and(|flags| flags.has_variable())
        {
            return Some(ty);
        }

        // read external committed symbol types
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.types.get_symbol_type_id(symbol);
        }

        None
    }

    /// Return one symbol's checked type, importing its module as needed.
    pub(in crate::sema) fn symbol_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        if !self.is_own_module(symbol.module_id) {
            self.import_external_module(symbol.module_id)?;
        }

        // adopt the type the symbol declares
        if let Some(ty) = self.adopt_symbol_type_maybe(symbol)? {
            return Ok(ty);
        }

        // external tables are committed, so an absent type is always a failure
        if !self.is_own_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!(
                    "external symbol {} has no committed type",
                    self.format_symbol(symbol),
                ),
            });
        }

        // local symbols read through their inference hole until commitment
        let variable = self.symbol_variable(symbol);

        self.variable_type(variable)
    }

    /// Return the variable standing for one local symbol's type, allocating it once.
    pub(in crate::sema) fn symbol_variable(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> dir::TypeVariableId {
        if let Some(variable) = self.infer.symbol_variables.get(&symbol) {
            return *variable;
        }

        let variable = self.open_variable(Origin::Symbol(symbol));
        self.infer.symbol_variables.insert(symbol, variable);

        variable
    }

    /// Adopt one symbol's declared-stage value as its binding, returning the written type.
    pub(in crate::sema) fn adopt_symbol_type_maybe(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            return Ok(None);
        };

        // copy own declared-stage values into the check tables once, types stay written
        if self.is_own_module(symbol.module_id)
            && self.binding_type_maybe(symbol).is_none()
            && self.declaration_type_maybe(symbol).is_none()
            && !self.symbol_kind(symbol)?.is_type_definition()
        {
            if self.symbol_kind(symbol)?.is_binding() {
                self.binding_types.insert(symbol, ty);
            } else {
                self.declaration_types.insert(symbol, ty);
            }
        }

        Ok(Some(ty))
    }

    /// Return the checked type required for one loaded symbol.
    pub(in crate::sema) fn require_symbol_type(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let Some(ty) = self.symbol_type_maybe(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("symbol {symbol:?} has no checked type"),
            });
        };

        Ok(ty)
    }

    /// Return one definition member's checked value type.
    pub(in crate::sema) fn definition_member_type(
        &mut self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if let Some(symbol) = member.type_symbol() {
            let ty = self.symbol_type(symbol)?;

            return Ok(Some(ty));
        }

        Ok(member.value_type())
    }

    /// Return the checked value type required for one definition member.
    pub(in crate::sema) fn require_definition_member_type(
        &self,
        member: &dir::DefinitionMember,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        if let Some(symbol) = member.type_symbol() {
            return Ok(Some(self.require_symbol_type(symbol)?));
        }

        Ok(member.value_type())
    }

    /// Return whether one definition member supplies a default.
    pub(in crate::sema) fn definition_member_has_default(
        &self,
        member: &dir::DefinitionMember,
    ) -> bool {
        // read whether the member declares a default
        match member {
            dir::DefinitionMember::Method(method) => {
                method.implementation == dir::MemberImplementation::Default
            }
            dir::DefinitionMember::AssociatedType(associated) => associated.value.is_some(),
            dir::DefinitionMember::AssociatedConst(associated) => {
                self.has_static_value(associated.symbol)
            }
            dir::DefinitionMember::Field(_)
            | dir::DefinitionMember::EnumVariant(_)
            | dir::DefinitionMember::CallSignature(_)
            | dir::DefinitionMember::ConstructSignature(_)
            | dir::DefinitionMember::IndexSignature(_) => false,
        }
    }

    /// Return whether one symbol has an inferred or written static value.
    fn has_static_value(&self, symbol: dir::GlobalSymbolId) -> bool {
        let has_inferred_value = self
            .module_maybe(symbol.module_id)
            .is_some_and(|module| module.static_values.contains_key(&symbol));
        let has_static_id = self.symbol_static_id(symbol).is_some();

        has_inferred_value || has_static_id
    }

    /// Return one symbol's committed static id, if declared.
    /// Return the type static naming one class as a value, pushing it on first use.
    pub(in crate::sema) fn class_static_id(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalStaticId> {
        if let Some(id) = self.symbol_static_id(symbol) {
            return Ok(id);
        }
        let ty = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
        let state = self.module_mut(symbol.module_id);
        let id = state.statics_tail.push_static(dir::StaticTerm::Type { ty });
        let id = id.into_global(symbol.module_id);
        state.statics_tail.set_symbol_static(symbol, id);

        Ok(id)
    }

    /// Return the static id one symbol's evaluated value carries.
    pub(in crate::sema) fn symbol_static_id(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalStaticId> {
        if let Some(module) = self.module_maybe(symbol.module_id) {
            let table = module.statics.with_tail(&module.statics_tail);

            table.get_symbol_static_id(symbol)
        } else {
            let external = self.external_modules.get(&symbol.module_id)?;

            external.statics.get_symbol_static_id(symbol)
        }
    }

    /// Return the inferred static value of one source symbol.
    pub(in crate::sema) fn static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalTypeId> {
        // read this pass's own committed values first
        if let Some(value) = self
            .module_maybe(symbol.module_id)
            .and_then(|module| module.static_values.get(&symbol).copied())
        {
            return Some(value);
        }

        // read own committed terms through the static table stack
        if let Some(module) = self.module_maybe(symbol.module_id) {
            let table = module.statics.with_tail(&module.statics_tail);
            let id = table.get_symbol_static_id(symbol)?;
            let term = table.get_static_maybe(id.local_id)?.clone();

            return self.static_singleton(id, &term);
        }

        // read foreign committed terms through the loaded external tables
        let external = self.external_modules.get(&symbol.module_id)?;
        let id = external.statics.get_symbol_static_id(symbol)?;
        let term = external.statics.get_static_maybe(id.local_id)?.clone();

        self.static_singleton(id, &term)
    }

    /// Return the singleton type of one committed static.
    fn static_singleton(
        &mut self,
        id: dir::GlobalStaticId,
        term: &dir::StaticTerm,
    ) -> Option<dir::GlobalTypeId> {
        if let Some(direct) = self.static_term_type(term) {
            return Some(direct);
        }

        // struct terms read as the singleton of their committed static
        match term {
            dir::StaticTerm::Struct { .. } => self.intern_type(dir::Type::Static(id)).ok(),
            _ => None,
        }
    }

    /// Return the singleton type of one committed static term.
    fn static_term_type(&mut self, term: &dir::StaticTerm) -> Option<dir::GlobalTypeId> {
        match term {
            dir::StaticTerm::Type { ty } => Some(*ty),
            dir::StaticTerm::Literal { value } => self.intern_type(dir::Type::Literal(*value)).ok(),
            _ => None,
        }
    }

    /// Commit the inferred static value of one source symbol as a singleton type.
    pub(in crate::sema) fn commit_static_value(
        &mut self,
        symbol: dir::GlobalSymbolId,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let static_values = &mut self.module_mut(symbol.module_id).static_values;
        if static_values
            .insert(symbol, value)
            .is_some_and(|previous| previous != value)
        {
            return Err(CompilerError::Internal {
                message: format!("check symbol {symbol:?} received two static values"),
            });
        }

        Ok(())
    }

    /// Return one module's resolved names, loaded or external.
    pub(in crate::sema) fn module_resolved(&self, module: ModuleId) -> &DirResolved {
        if let Some(module) = self.module_maybe(module) {
            &module.resolved
        } else {
            &self.external_module(module).resolved
        }
    }

    /// Return one module's tree view.
    pub(in crate::sema) fn module_view(&self, module: ModuleId) -> dir::View<'_> {
        match self.module_maybe(module) {
            Some(module) => module.view(),
            None => unreachable!("foreign trees are not checking inputs: {module:?}"),
        }
    }

    /// Return one binding table by module, loaded or read from artifacts.
    pub(in crate::sema) fn binding_table(&self, module: ModuleId) -> dir::BindingTable<'_> {
        if let Some(module) = self.module_maybe(module) {
            return module.binding_table();
        }
        if let Some(external) = self.external_modules.get(&module) {
            return external.bindings.clone();
        }

        // read a referenced module's bound tables for its declared artifact
        let bound = self
            .artifacts
            .read::<DirBound>((module, self.profile))
            .unwrap_or_else(|error| {
                unreachable!("referenced module {module:?} has no bound artifact: {error}")
            });
        let expanded = self
            .artifacts
            .read::<DirExpanded>((module, self.profile))
            .unwrap_or_else(|error| {
                unreachable!("referenced module {module:?} has no expanded artifact: {error}")
            });

        expanded.binding_table(bound.as_ref())
    }

    /// Return one own-module or external static.
    pub(in crate::sema) fn r#static(&self, value: dir::GlobalStaticId) -> &dir::StaticTerm {
        if let Some(module) = self.module_maybe(value.module_id) {
            module.r#static(value.local_id)
        } else {
            self.external_module(value.module_id)
                .statics
                .get_static(value.local_id)
        }
    }

    /// Return one own-module symbol's kind without loading anything.
    pub(in crate::sema) fn own_symbol_kind(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::SymbolKind> {
        if !self.is_own_module(symbol.module_id) {
            return None;
        }

        // read the kind the symbol declares
        Some(
            self.binding_table(symbol.module_id)
                .get_symbol(symbol.local_id)
                .kind,
        )
    }

    /// Return the declaration kind for one symbol.
    pub(in crate::sema) fn symbol_kind(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::SymbolKind> {
        // read own symbols straight from the tail over the base
        if self.is_own_module(symbol.module_id) {
            return Ok(self.module.symbol(symbol.local_id).kind);
        }

        // load the foreign module the classification reads
        self.import_external_module(symbol.module_id)?;
        let binding_table = self.binding_table(symbol.module_id);
        let symbol = binding_table.get_symbol(symbol.local_id);

        Ok(symbol.kind)
    }

    /// Flatten every declared nominal owner's member bindings into the module tail.
    pub(in crate::sema) fn flatten_declared_owners(&mut self) -> CompilerResult<()> {
        // collect the module's declared nominal owners
        let module = self.module_id;
        let mut owners = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            let is_owner = matches!(
                definition,
                dir::Definition::Struct(_)
                    | dir::Definition::Class(_)
                    | dir::Definition::Enum(_)
                    | dir::Definition::Newtype(_)
                    | dir::Definition::Interface(_)
                    | dir::Definition::Extension(_)
            );
            if is_owner {
                owners.push(symbol);
            }
        }

        // flatten each owner's bindings once in both member spaces
        for symbol in owners {
            // derive each declared parameter's variance at its own context
            if let Some(template) = self.symbol_template(symbol)? {
                for parameter in self.generic_template_parameters(template)? {
                    let form = self.parameter_variance_form(parameter)?;
                    self.parameter_variance(parameter, form)?;
                }
            }

            for space in [dir::MemberSpace::Instance, dir::MemberSpace::Static] {
                let bindings = self.member_bindings(symbol, space)?;
                let Some(bindings) = bindings else {
                    continue;
                };
                self.module_mut(module)
                    .members_tail
                    .set_bindings(symbol, space, bindings.to_vec());
            }
        }

        Ok(())
    }
}

impl CheckState<'_> {
    /// Commit one node decision into its module's decision segment.
    pub(in crate::sema) fn commit_decision(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::Decision,
    ) -> CompilerResult<()> {
        if self.infer.is_deciding() {
            return Ok(());
        }
        let uses = resolution.binding_uses();

        // refuse refinements that would invalidate already committed uses
        if let Some(previous) = self.module(node.module_id).decisions.decision(node) {
            let previous_uses = previous.binding_uses();
            if !previous_uses.is_empty() && previous_uses != uses {
                return Err(CompilerError::Internal {
                    message: format!(
                        "check node {} refined its selected declaration uses: previous = {previous_uses:?}, new = {uses:?}",
                        self.node_label(node),
                    ),
                });
            }
        }

        // write the selected declaration uses
        for (symbol, binding_use) in uses {
            self.module_mut(node.module_id).flows.commit_binding_use(
                node.local_id,
                symbol,
                binding_use,
            );
        }

        // keep the last committed payload as later derivations refine the same targets
        self.module_mut(node.module_id)
            .decisions
            .set_decision(node, resolution);
        self.record_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Commit one node's lexical name into its module's resolution segment.
    pub(in crate::sema) fn commit_name(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::NameResolution,
    ) -> CompilerResult<()> {
        // write one unambiguous lexical reference
        if let Some(symbol) = resolution.single_symbol() {
            self.module_mut(node.module_id).flows.commit_binding_use(
                node.local_id,
                symbol,
                dir::BindingUse::READ,
            );
        }

        // commit the name resolution at the node
        self.module_mut(node.module_id)
            .resolutions
            .set_name_resolution(node, resolution);
        self.record_event(CheckEvent::NodeDecided { node });

        Ok(())
    }

    /// Return one node's committed lexical name.
    pub(in crate::sema) fn name_decision(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<&dir::NameResolution> {
        self.module(node.module_id)
            .resolutions
            .name_resolution(node)
    }

    /// Poison one node whose operand already reported an error.
    pub(in crate::sema) fn poison_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.commit_decision(node, dir::Decision::Poisoned)?;
        self.commit_error_node(node)
    }

    /// Return one node's committed resolution.
    pub(in crate::sema) fn decision(&self, node: dir::GlobalNodeIdAny) -> Option<&dir::Decision> {
        self.module(node.module_id).decisions.decision(node)
    }

    /// Return one module's committed resolutions.
    pub(in crate::sema) fn resolutions(&self, module: ModuleId) -> &dir::ResolutionSegment {
        &self.module(module).resolutions
    }

    /// Return one module's committed decisions.
    pub(in crate::sema) fn decisions(&self, module: ModuleId) -> &dir::DecisionSegment {
        &self.module(module).decisions
    }
}
