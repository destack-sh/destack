use destack_core::FxIndexMap;
use destack_dir as dir;

use super::builtin::{keyword_completions, primitive_type_completions};
use super::{CompletionContext, CompletionReceiver, CursorToken};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    CompletionTrigger, ModuleQueryContext, ProgramQueryContext, QueryResult, SymbolUse,
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
        token: Option<&CursorToken>,
        include_auto_imports: bool,
    ) -> QueryResult<CompletionCandidates> {
        // derive prefix rules from the request
        let prefix = token.map_or("", |token| token.text.as_str());
        let allow_short_prefix = matches!(
            trigger,
            CompletionTrigger::Invoked | CompletionTrigger::Incomplete
        );

        // collect candidates for the exact cursor context
        let mut items = match context {
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Access { site },
            } => self.collect_members(*site)?,
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Namespace { module_id },
            } => self.collect_namespace_members(*module_id)?,
            CompletionContext::TypePosition { scope } => self.collect_types(*scope, prefix)?,
            CompletionContext::ValuePosition { scope } => {
                self.collect_values(*scope, prefix, false)?
            }
            CompletionContext::StatementPosition { scope } => self.collect_values(
                *scope,
                prefix,
                matches!(trigger, CompletionTrigger::Invoked),
            )?,
            CompletionContext::ObjectLiteralKey { literal, scope } => {
                self.collect_object_literal(*literal, *scope, prefix)?
            }
            CompletionContext::ObjectLiteralValue { scope } => {
                self.collect_values(*scope, prefix, false)?
            }
            CompletionContext::CallArgument { scope, .. } => {
                self.collect_values(*scope, prefix, false)?
            }
            CompletionContext::NewExpression { scope } => {
                self.collect_new_expression(*scope, prefix)?
            }
            CompletionContext::ControlLabel { labels } => self.collect_labels(labels, prefix),
            CompletionContext::ImportPath { path } => self.collect_import_paths(path)?,
            CompletionContext::ImportClause {
                target_module,
                existing_names,
                use_filter,
            } => self.collect_imports(*target_module, existing_names, *use_filter)?,
        };

        let mut is_incomplete = false;

        // add auto imports when this context supports them
        if include_auto_imports && let Some(auto_import) = context.auto_import_context() {
            let auto_imports =
                self.collect_auto_imports(prefix, auto_import, allow_short_prefix)?;
            items.extend(auto_imports.items);
            is_incomplete = auto_imports.is_incomplete;
        }

        // resolve value types required by call argument ranking
        if context.expected_type().is_some() {
            for completion in &mut items {
                completion.type_id = self.resolve_type(completion)?;
            }
        }

        Ok(CompletionCandidates {
            items,
            is_incomplete,
        })
    }

    /// Collect types in type position.
    fn collect_types(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // collect visible type names first
        for (key, binding) in self.visible_bindings(scope, SymbolUse::Type, prefix)? {
            let dir::StaticKey::Name(name_id) = key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            let completion = CompletionCandidate::new(name, binding.kind, binding.origin);

            results.push(completion.with_symbol(binding.symbol));
        }

        // primitive types are always available
        results.extend(primitive_type_completions());

        Ok(results)
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
    ) -> QueryResult<VisibleBinding> {
        let is_alias = matches!(
            binding.kind,
            dir::SymbolKind::Import | dir::SymbolKind::ExportAlias
        );
        let (symbol, kind) = if is_alias {
            let symbol = self.symbol_target(symbol)?;
            let module = self.program.module(symbol.module_id)?;
            let binding = module.bindings()?.get_symbol(symbol.local_id);

            (symbol, binding.into())
        } else {
            (symbol, binding.into())
        };

        Ok(VisibleBinding {
            symbol,
            kind,
            origin,
        })
    }

    /// Return visible lexical and profile bindings by precedence.
    pub(super) fn visible_bindings(
        &self,
        scope: dir::LocalScope,
        symbol_use: SymbolUse,
        prefix: &str,
    ) -> QueryResult<FxIndexMap<dir::StaticKey, VisibleBinding>> {
        let module_id = self.module.module_id();
        let bindings = self.module.bindings()?;
        let view = self.module.view()?;
        let mut visible = FxIndexMap::default();

        // collect lexical bindings from nearest to farthest
        for binding in bindings
            .visible_bindings(scope)
            .filter(|binding| symbol_use.accepts_symbol_kind(binding.symbol.kind))
        {
            if self.is_initializing_binding(binding.symbol, view) {
                continue;
            }
            let dir::StaticKey::Name(name) = binding.key else {
                continue;
            };
            if match_quality(self.module.strings().get(name), prefix).is_none() {
                continue;
            }
            let symbol = binding.symbol_id.into_global(module_id);
            let completion =
                self.resolve_binding(symbol, binding.symbol, CompletionOrigin::Local)?;
            visible.entry(binding.key).or_insert(completion);
        }

        let environment = self.program.environment_bound()?;

        // add unambiguous profile globals not shadowed lexically
        for (key, resolutions) in &environment.global_resolutions_by_key {
            if visible.contains_key(key) {
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

            // select the first declaration accepted by the requested namespace
            for symbol in symbols {
                let module = self.program.module(symbol.module_id)?;
                let binding = module.bindings()?.get_symbol(symbol.local_id);
                if symbol_use.accepts_symbol_kind(binding.kind) {
                    let completion =
                        self.resolve_binding(*symbol, binding, CompletionOrigin::Builtin)?;
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
        self.visible_bindings(scope, SymbolUse::Value, prefix)
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

    /// Collect values in expression position.
    fn collect_values(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
        include_keywords: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // build one completion per visible value
        for (key, binding) in self.visible_value_bindings(scope, prefix)? {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };
            let name = self.module.strings().get(name).to_string();

            // defer constructor details until after ranking
            if binding.kind == CompletionItemKind::Struct {
                let completion =
                    CompletionCandidate::new(&name, CompletionItemKind::Struct, binding.origin)
                        .with_struct(binding.symbol);
                results.push(completion);

                continue;
            }
            if binding.kind == CompletionItemKind::Newtype {
                let completion = CompletionCandidate::new(
                    &name,
                    CompletionItemKind::Constructor,
                    binding.origin,
                )
                .with_newtype_constructors(binding.symbol);
                results.push(completion);

                continue;
            }

            // build the ordinary value candidate
            let completion = CompletionCandidate::new(&name, binding.kind, binding.origin);
            let completion = if binding.kind.is_callable() {
                completion.with_call()
            } else {
                completion
            };
            results.push(completion.with_symbol(binding.symbol));
        }

        // statement contexts can opt into keyword completions as well
        if include_keywords {
            results.extend(keyword_completions());
        }

        Ok(results)
    }

    /// Collect constructable symbols for a new expression.
    fn collect_new_expression(
        &self,
        scope: dir::LocalScope,
        prefix: &str,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // collect visible constructable names from the active scope
        for (key, binding) in self.visible_bindings(scope, SymbolUse::Value, prefix)? {
            let dir::StaticKey::Name(name_id) = key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !matches!(
                binding.kind,
                CompletionItemKind::Class | CompletionItemKind::Struct
            ) {
                continue;
            }
            if binding.kind == CompletionItemKind::Class {
                let completion =
                    CompletionCandidate::new(name, CompletionItemKind::Class, binding.origin)
                        .with_class_constructors(binding.symbol);
                results.push(completion);
            } else {
                let completion = CompletionCandidate::new(name, binding.kind, binding.origin)
                    .with_symbol(binding.symbol);
                results.push(completion);
            }
        }

        Ok(results)
    }
}
