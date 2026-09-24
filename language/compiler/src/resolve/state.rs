use destack_artifact::{DiagnosticAnchor, DiagnosticBuilder, DirResolved, EnvironmentBound};
use destack_core::{NameMatch, StringPool, find_best_match};
use destack_dir as dir;
use destack_repository::ArtifactReader;
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexSet;
use smallvec::{SmallVec, smallvec};

use crate::export::{ExportLookup, ExportResolver};
use crate::resolve::stats::ResolveStats;
use crate::{
    CompilerResult, ResolveError, ResolveWarning, diagnostic_suggestion_distance, rename_suggestion,
};

/// Resolve phase state for one module.
pub(in crate::resolve) struct ResolveState<'a> {
    /// Revision-bound artifact reader.
    pub(in crate::resolve) artifacts: ArtifactReader<'a>,
    /// The current module.
    pub(in crate::resolve) module: ModuleId,
    /// The expanded DIR view.
    pub(in crate::resolve) view: dir::View<'a>,
    /// The expanded binding table.
    pub(in crate::resolve) bindings: dir::BindingTable<'static>,
    /// The expanded module table.
    pub(in crate::resolve) modules: dir::ModuleTable<'static>,
    /// The shared string pool.
    pub(in crate::resolve) strings: &'a StringPool,
    /// The import table being built.
    pub(in crate::resolve) imports: dir::ImportTable,
    /// The reference resolutions being built.
    pub(in crate::resolve) references: dir::ReferenceTable,
    /// The errors produced while resolving.
    pub(in crate::resolve) errors: Vec<DiagnosticBuilder<ResolveError>>,
    /// The warnings produced while resolving.
    pub(in crate::resolve) warnings: Vec<DiagnosticBuilder<ResolveWarning>>,
    /// The work counts accumulated while resolving.
    pub(in crate::resolve) stats: ResolveStats,
    /// Import expressions collected from active roots.
    pub(in crate::resolve) import_expressions: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Namespace path references collected from active roots.
    pub(in crate::resolve) path_references: Vec<PathReference>,
    /// Source-visible global keys referenced by active roots.
    pub(in crate::resolve) global_keys: IndexSet<dir::StaticKey>,
    /// Language items used by active roots without source imports.
    pub(in crate::resolve) language_items: IndexSet<dir::LanguageItem>,
    /// Function contexts visible while walking active roots.
    pub(in crate::resolve) function_stack: Vec<FunctionContext>,
    /// The memoized export lookups shared by this provider run.
    pub(in crate::resolve) exports: ExportResolver,
}

/// One namespace path reference to resolve after imports are known.
#[derive(Debug, Clone)]
pub(in crate::resolve) struct PathReference {
    /// The source node that owns the path.
    pub(in crate::resolve) source: dir::GlobalNodeIdAny,
    /// The source path.
    pub(in crate::resolve) path: dir::Path,
}

/// Function context used to choose async and generator language items.
#[derive(Debug, Clone, Copy)]
pub(in crate::resolve) struct FunctionContext {
    /// The function asynchrony.
    pub(in crate::resolve) asynchrony: dir::Asynchrony,
    /// Whether the function is a generator.
    pub(in crate::resolve) is_generator: bool,
}

impl<'a> ResolveState<'a> {
    /// Create resolve state for one module.
    pub(in crate::resolve) fn new(
        artifacts: ArtifactReader<'a>,
        profile: ProfileId,
        module: ModuleId,
        view: dir::View<'a>,
        bindings: dir::BindingTable<'static>,
        modules: dir::ModuleTable<'static>,
        strings: &'a StringPool,
    ) -> Self {
        // initialize phase output
        Self {
            artifacts,
            module,
            view,
            bindings,
            modules,
            strings,
            imports: dir::ImportTable::new(module),
            references: dir::ReferenceTable::new(module),
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: ResolveStats::default(),
            import_expressions: Vec::new(),
            path_references: Vec::new(),
            global_keys: IndexSet::new(),
            language_items: IndexSet::new(),
            function_stack: Vec::new(),
            exports: ExportResolver::new(profile),
        }
    }

    /// Collect one source path reference for later target lookup.
    pub(in crate::resolve) fn collect_path_reference(&mut self, reference: PathReference) {
        let root = reference.path.segments.first().copied();

        // expression member chains already visit their root identifier
        if reference.source.local_id.ty != dir::NodeType::TypeExpression
            && reference.path.segments.len() > 1
        {
            self.path_references.push(reference);

            return;
        }

        // collect global keys only for valid roots with no local winner
        if let Some(root) = root {
            self.stats.local_binding_lookups += 1;
            let key = dir::StaticKey::Name(root);
            let local_symbols = self.visible_symbols(reference.source.local_id, key);
            if local_symbols.is_empty() && self.global_keys.insert(key) {
                self.stats.required_globals += 1;
            }
        }

        self.path_references.push(reference);
    }

    /// Return symbols visible at one source node.
    pub(in crate::resolve) fn visible_symbols(
        &self,
        source: dir::LocalNodeIdAny,
        key: dir::StaticKey,
    ) -> SmallVec<[dir::GlobalSymbolId; 2]> {
        let lookup = self.bindings.lookup_symbol_at(&self.view, source, key);

        self.symbols_from_lookup(lookup)
    }

    /// Return global symbols from one local lookup result.
    fn symbols_from_lookup(&self, lookup: dir::SymbolLookup) -> SmallVec<[dir::GlobalSymbolId; 2]> {
        match lookup {
            dir::SymbolLookup::Found(symbol) => smallvec![symbol.into_global(self.module)],
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(self.module))
                .collect(),
            dir::SymbolLookup::Missing => SmallVec::new(),
        }
    }

    /// Use one language item from active roots.
    pub(in crate::resolve) fn use_language_item(&mut self, item: dir::LanguageItem) {
        if self.language_items.insert(item) {
            self.stats.language_item_uses += 1;
        }
    }

    /// Enter one function context.
    pub(in crate::resolve) fn enter_function(&mut self, signature: &dir::FunctionSignature) {
        self.function_stack.push(FunctionContext {
            asynchrony: signature.asynchrony,
            is_generator: signature.is_generator,
        });
    }

    /// Leave the current function context.
    pub(in crate::resolve) fn leave_function(&mut self) {
        self.function_stack.pop();
    }

    /// Return the current function context.
    pub(in crate::resolve) fn current_function(&self) -> Option<FunctionContext> {
        self.function_stack.last().copied()
    }

    /// Drain resolve errors.
    pub(in crate::resolve) fn take_errors(&mut self) -> Vec<DiagnosticBuilder<ResolveError>> {
        std::mem::take(&mut self.errors)
    }

    /// Drain resolve warnings.
    pub(in crate::resolve) fn take_warnings(&mut self) -> Vec<DiagnosticBuilder<ResolveWarning>> {
        std::mem::take(&mut self.warnings)
    }

    /// Resolve one exported target through this state's export resolver.
    pub(in crate::resolve) fn resolve_export_target(
        &mut self,
        module: ModuleId,
        key: dir::ExportKey,
    ) -> CompilerResult<ExportLookup> {
        self.exports
            .resolve_export_target(&self.artifacts, module, key)
    }

    /// Record the default tree builder this module's literals pull in.
    pub(in crate::resolve) fn record_tree_builder(&mut self, builder: Option<dir::GlobalSymbolId>) {
        self.imports.tree_target = builder;
    }

    /// Finish resolved DIR.
    pub(in crate::resolve) fn finish(self, environment: &EnvironmentBound) -> DirResolved {
        let extensions = self.resolved_extensions(environment);

        DirResolved {
            imports: self.imports,
            references: self.references,
            extensions,
        }
    }

    /// Resolve every exported extension to its target root declaration.
    fn resolved_extensions(&self, environment: &EnvironmentBound) -> dir::ExtensionTable {
        let mut extensions = dir::ExtensionTable::new(self.module);

        for symbol in self.bindings.symbol_ids() {
            let binding = self.bindings.get_symbol(symbol);
            if binding.kind != dir::SymbolKind::Extension {
                continue;
            }
            let Some(declaration) = binding.declaration else {
                continue;
            };
            if declaration.module_id != self.module {
                continue;
            }
            let Ok(declaration) = declaration.local_id.try_into_typed::<dir::Declaration>() else {
                continue;
            };
            let dir::Declaration::Extension(extension) = self.view.get(declaration) else {
                continue;
            };

            // resolve the extension target and every interface it implements
            let target = self.extension_target(environment, extension.target_type);
            let interfaces = extension
                .implements_types
                .iter()
                .filter_map(|implemented| self.interface_symbol(*implemented))
                .collect::<Vec<_>>();
            if !interfaces.is_empty() {
                extensions.insert_implementation(
                    symbol,
                    dir::ExtensionImplementation {
                        root: target,
                        interfaces,
                    },
                );
            }
            if binding.export_kind.is_none() {
                continue;
            }
            let Some(target) = target else {
                continue;
            };

            extensions.insert(symbol, target);
        }

        extensions
    }

    /// Return the interface symbol one implements clause names.
    fn interface_symbol(
        &self,
        node: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::GlobalSymbolId> {
        let dir::TypeExpression::Reference { .. } = self.view.get(node) else {
            return None;
        };
        let reference = self.references.get(node.into_global_any(self.module))?;
        let dir::Reference::Bound(symbols) = reference else {
            return None;
        };

        symbols.first().copied()
    }

    /// Return the target root declaration of one extension target head.
    fn extension_target(
        &self,
        environment: &EnvironmentBound,
        node: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Option<dir::GlobalSymbolId> {
        let mut node = node;
        loop {
            match self.view.get(node) {
                dir::TypeExpression::Keyword { value } => {
                    return environment.language.symbol(value.representation_item()?);
                }
                dir::TypeExpression::Reference { .. } => {
                    let reference = self.references.get(node.into_global_any(self.module))?;
                    let dir::Reference::Bound(symbols) = reference else {
                        return None;
                    };

                    return symbols.first().copied();
                }
                dir::TypeExpression::Readonly { target_type }
                | dir::TypeExpression::OwnedOf { target_type, .. }
                | dir::TypeExpression::BorrowedOf { target_type, .. }
                | dir::TypeExpression::PointerOf { target_type, .. } => node = *target_type,
                _ => return None,
            }
        }
    }

    /// Report one missing export diagnostic.
    pub(in crate::resolve) fn report_missing_export(
        &mut self,
        target_module: ModuleId,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        key: dir::ExportKey,
        specifier: dir::StringId,
    ) -> CompilerResult<()> {
        // render diagnostic payload
        let anchor = self.anchor_node(item_id.id)?;
        let name = self.export_key_text(key);
        let target = self.strings.get(specifier).to_string();

        // a matching unexported binding beats any similar-name suggestion
        let exported = self
            .exports
            .exported_module(&self.artifacts, target_module)?;
        if exported.locals.contains(&name) {
            let error = ResolveError::NotExported {
                anchor,
                name: name.clone(),
                target: target.clone(),
            };
            let diagnostic =
                DiagnosticBuilder::new(error).help(format!("export '{name}' from '{target}'"));
            self.errors.push(diagnostic);

            return Ok(());
        }

        let best = self.closest_export_key(target_module, &name)?;
        let error = ResolveError::MissingExport {
            anchor: anchor.clone(),
            name,
            target,
            suggestion: best.as_ref().map(|best| best.candidate.clone()),
        };

        // plain items span the bare name, so the rename patches cleanly
        let mut diagnostic = DiagnosticBuilder::new(error);
        let is_plain_name = matches!(
            self.view.get(item_id),
            dir::DependencyItem::Binding {
                alias: None,
                name: Some(_),
                ..
            }
        );
        if is_plain_name
            && let Some(best) = best
            && let Some(suggestion) = rename_suggestion(&anchor, &best)
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }

        // record the diagnostic
        self.errors.push(diagnostic);

        Ok(())
    }

    /// Report one ambiguous export diagnostic.
    pub(in crate::resolve) fn report_ambiguous_export(
        &mut self,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        key: dir::ExportKey,
        specifier: dir::StringId,
        resolutions: &[dir::ExportResolution],
    ) -> CompilerResult<()> {
        // render diagnostic payload
        let anchor = self.anchor_node(item_id.id)?;
        let name = self.export_key_text(key);
        let target = self.strings.get(specifier).to_string();
        let error = ResolveError::AmbiguousExport {
            anchor,
            name: name.clone(),
            target,
        };

        // point at each declaration supplying the name
        let mut diagnostic = DiagnosticBuilder::new(error);
        for resolution in resolutions {
            for target in resolution.declaration.iter() {
                diagnostic = diagnostic.reference(target, format!("one '{name}' is declared here"));
            }
        }
        diagnostic = diagnostic.help(format!("import '{name}' directly from one origin module"));

        // record the diagnostic
        self.errors.push(diagnostic);

        Ok(())
    }

    /// Report one repeated import of the same resolved module.
    pub(in crate::resolve) fn report_duplicate_import(
        &mut self,
        repeated: dir::LocalNodeId<dir::Expression>,
        first: dir::LocalNodeId<dir::Expression>,
        specifier: dir::StringId,
    ) -> CompilerResult<()> {
        let anchor = self.anchor_node(repeated.id)?;
        let first = self.anchor_node(first.id)?;
        let specifier = self.strings.get(specifier).to_string();
        let warning = ResolveWarning::DuplicateImport { anchor, specifier };
        let diagnostic = DiagnosticBuilder::new(warning).label(first, "first imported here");
        self.warnings.push(diagnostic);

        Ok(())
    }

    /// Return the source anchor for one local node id.
    fn anchor_node(&self, node_id: u32) -> Result<DiagnosticAnchor, ResolveError> {
        self.view
            .get_span_by_id(node_id)
            .map(DiagnosticAnchor::from)
            .ok_or_else(|| ResolveError::Internal {
                anchor: DiagnosticAnchor::from(self.module),
                module: self.module,
                message: format!("missing source span for resolved node {node_id}"),
            })
    }

    /// Render one export key for diagnostics.
    fn export_key_text(&self, key: dir::ExportKey) -> String {
        match key {
            dir::ExportKey::Default => "default".to_string(),
            dir::ExportKey::Named(key) => self.static_key_text(key),
        }
    }

    /// Render one static key for diagnostics.
    fn static_key_text(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings.get(name).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
        }
    }

    /// Return the closest exact export key visible from one target module.
    fn closest_export_key(
        &mut self,
        target_module: ModuleId,
        key: &str,
    ) -> CompilerResult<Option<NameMatch<String>>> {
        let exported = self
            .exports
            .exported_module(&self.artifacts, target_module)?;
        let candidates = exported
            .exports
            .exports()
            .map(|(key, _)| self.export_key_text(*key));

        Ok(find_best_match(
            key,
            candidates,
            diagnostic_suggestion_distance(key),
        ))
    }
}
