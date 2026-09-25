use rustc_hash::FxHashSet;
use tspp_core::FxIndexMap;
use tspp_dir as dir;

use super::builtin::{keyword_completions, primitive_type_completions};
use super::{CompletionContext, CompletionPosition, CompletionReceiver};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    CompletionTrigger, DeclarationUse, ModuleQueryContext, ProgramQueryContext, QueryResult,
    match_quality,
};

/// Collects completion candidates at one module position.
pub(crate) struct CompletionCollector<'owner, 'module, 'program> {
    /// The queried module.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The program query context.
    pub(super) program: &'owner ProgramQueryContext<'program>,
    /// The pattern being initialized at the cursor.
    initializing_pattern: Option<dir::LocalNodeId<dir::Pattern>>,
}

/// One binding visible to completion at the cursor.
#[derive(Debug, Clone, Copy)]
pub(super) struct VisibleBinding {
    /// The declaration selected by the binding.
    pub(super) symbol: dir::GlobalSymbolId,
    /// The completion item kind.
    pub(super) kind: CompletionItemKind,
    /// The binding origin used for ranking.
    pub(super) origin: CompletionOrigin,
}

impl<'owner, 'module, 'program> CompletionCollector<'owner, 'module, 'program> {
    /// Create one completion collector.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
        initializing_pattern: Option<dir::LocalNodeId<dir::Pattern>>,
    ) -> Self {
        Self {
            module,
            program,
            initializing_pattern,
        }
    }

    /// Collect completion candidates for one context.
    pub(crate) fn collect(
        &self,
        trigger: CompletionTrigger,
        context: &CompletionContext,
        include_auto_imports: bool,
    ) -> QueryResult<CompletionCandidates> {
        // derive prefix rules from the request
        let prefix = context
            .prefix
            .as_ref()
            .map_or("", |prefix| prefix.text.as_str());
        let position = &context.position;
        let allow_short_prefix = matches!(
            trigger,
            CompletionTrigger::Invoked | CompletionTrigger::Incomplete
        );

        // collect candidates for the exact cursor context
        let mut items = match position {
            CompletionPosition::MemberAccess {
                receiver: CompletionReceiver::Access { site },
            } => self.collect_members(*site)?,
            CompletionPosition::MemberAccess {
                receiver: CompletionReceiver::Namespace { module_id, usage },
            } => self.collect_namespace_members(*module_id, *usage)?,
            CompletionPosition::Type { scope } => {
                let mut items = self.collect_symbols(*scope, prefix, DeclarationUse::Type)?;
                items.extend(primitive_type_completions());

                items
            }
            CompletionPosition::Value { scope, .. } => {
                self.collect_symbols(*scope, prefix, DeclarationUse::Expression)?
            }
            CompletionPosition::Statement { scope } => {
                let mut items = self.collect_symbols(*scope, prefix, DeclarationUse::Expression)?;
                if matches!(trigger, CompletionTrigger::Invoked) {
                    items.extend(keyword_completions());
                }

                items
            }
            CompletionPosition::ObjectLiteralKey { literal, scope } => {
                self.collect_object_literal(*literal, *scope, prefix)?
            }
            CompletionPosition::Constructor { scope } => {
                self.collect_symbols(*scope, prefix, DeclarationUse::Constructor)?
            }
            CompletionPosition::ControlLabel { labels } => self.collect_labels(labels, prefix),
            CompletionPosition::ImportPath { path } => self.collect_import_paths(path)?,
            CompletionPosition::ImportClause {
                target_module,
                existing_names,
            } => self.collect_imports(*target_module, existing_names)?,
        };

        let mut is_incomplete = false;

        // add auto imports when this context supports them
        if include_auto_imports && let Some(auto_import) = position.auto_import_context() {
            let auto_imports =
                self.collect_auto_imports(prefix, auto_import, allow_short_prefix)?;
            items.extend(auto_imports.items);
            is_incomplete = auto_imports.is_incomplete;
        }

        // resolve value types required by call argument ranking
        if position.expected_type().is_some() {
            for completion in &mut items {
                completion.type_id = self.resolve_type(completion)?;
            }
        }

        Ok(CompletionCandidates {
            items,
            is_incomplete,
        })
    }

    /// Collect labels visible to one control transfer.
    fn collect_labels(&self, labels: &[dir::StringId], prefix: &str) -> Vec<CompletionCandidate> {
        let mut results = Vec::new();

        // retain enclosing labels that match the authored prefix
        for label in labels {
            let name = self.module.strings().get(*label);
            if match_quality(name, prefix).is_none() {
                continue;
            }
            results.push(CompletionCandidate::new(
                name.to_string(),
                CompletionItemKind::Label,
                CompletionOrigin::Local,
            ));
        }

        results
    }

    /// Resolve one visible binding to its target declaration and completion kind.
    fn resolve_binding(
        &self,
        symbol: dir::GlobalSymbolId,
        binding: &dir::Symbol,
        origin: CompletionOrigin,
        usage: DeclarationUse,
    ) -> QueryResult<Option<VisibleBinding>> {
        let is_alias = matches!(
            binding.kind,
            dir::SymbolKind::Import | dir::SymbolKind::ExportAlias
        );
        let symbol = if is_alias {
            self.symbol_target(symbol)?
        } else {
            symbol
        };
        let module = self.program.module(symbol.module_id)?;
        let binding = module.bindings()?.get_symbol(symbol.local_id);

        // retain namespace qualifiers and declarations eligible at this position
        let is_namespace = if let Some(declaration) = binding.declaration {
            module
                .resolved()?
                .references
                .get(declaration)
                .and_then(dir::Reference::namespace)
                .is_some()
        } else {
            false
        };
        let is_accepted = if is_namespace {
            usage != DeclarationUse::Value
        } else {
            usage.accepts_symbol_kind(binding.kind)
        };
        if !is_accepted {
            return Ok(None);
        }

        Ok(Some(VisibleBinding {
            symbol,
            kind: binding.into(),
            origin,
        }))
    }

    /// Return visible lexical and profile bindings by precedence.
    pub(super) fn visible_bindings(
        &self,
        scope: dir::LocalScope,
        usage: DeclarationUse,
        prefix: &str,
    ) -> QueryResult<FxIndexMap<dir::StaticKey, VisibleBinding>> {
        let module_id = self.module.module_id();
        let bindings = self.module.bindings()?;
        let view = self.module.view()?;
        let mut visible = FxIndexMap::default();
        let mut seen = FxHashSet::default();

        // collect lexical bindings from nearest to farthest
        for binding in bindings.visible_bindings(scope) {
            if self.is_initializing_binding(binding.symbol, view) {
                continue;
            }
            if !seen.insert(binding.key) {
                continue;
            }
            let dir::StaticKey::Name(name) = binding.key else {
                continue;
            };
            if match_quality(self.module.strings().get(name), prefix).is_none() {
                continue;
            }

            // resolve only the nearest declaration for each visible name
            let symbol = binding.symbol_id.into_global(module_id);
            if let Some(completion) =
                self.resolve_binding(symbol, binding.symbol, CompletionOrigin::Local, usage)?
            {
                visible.insert(binding.key, completion);
            }
        }

        let environment = self.program.environment_bound()?;

        // add unambiguous profile globals not shadowed lexically
        for (key, resolutions) in &environment.global_resolutions_by_key {
            if seen.contains(key) {
                continue;
            }
            let dir::StaticKey::Name(name) = key else {
                continue;
            };
            if match_quality(self.module.strings().get(*name), prefix).is_none() {
                continue;
            }
            let [resolution] = resolutions.as_slice() else {
                continue;
            };
            let Some(symbols) = resolution.target.symbol_ids() else {
                continue;
            };

            // select the first eligible declaration from the global binding
            for symbol in symbols {
                let module = self.program.module(symbol.module_id)?;
                let binding = module.bindings()?.get_symbol(symbol.local_id);
                if let Some(completion) =
                    self.resolve_binding(*symbol, binding, CompletionOrigin::Builtin, usage)?
                {
                    visible.insert(*key, completion);

                    break;
                }
            }
        }

        Ok(visible)
    }

    /// Return visible value bindings by precedence.
    pub(super) fn visible_value_bindings(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
    ) -> QueryResult<FxIndexMap<dir::StaticKey, VisibleBinding>> {
        self.visible_bindings(scope, DeclarationUse::Value, prefix)
    }

    /// Return whether one symbol belongs to the pattern currently being initialized.
    fn is_initializing_binding(&self, symbol: &dir::Symbol, view: dir::View<'_>) -> bool {
        let Some(pattern) = self.initializing_pattern else {
            return false;
        };
        let Some(declaration) = symbol.declaration else {
            return false;
        };
        if declaration.module_id != self.module.module_id() {
            return false;
        }

        let declaration = declaration.local_id;
        let pattern = pattern.into_any();

        view.is_inside(declaration, pattern)
    }

    /// Collect declarations eligible at the requested position.
    fn collect_symbols(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
        usage: DeclarationUse,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // select insertions from the same declaration use as qualified and imported names
        for (key, binding) in self.visible_bindings(scope, usage, prefix)? {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };

            let name = self.module.strings().get(name).to_string();
            let completion = CompletionCandidate::new(name, binding.kind, binding.origin)
                .with_declaration(binding.symbol, usage);
            results.push(completion);
        }

        Ok(results)
    }
}
