use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::FileId;
use rustc_hash::FxHashSet;

use super::builtin::{keyword_completions, primitive_type_completions};
use super::{AutoImportSearch, CompletionContext, CompletionReceiver, CursorToken};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    CompletionTrigger, ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult,
    SORT_LOCAL_SYMBOL, SymbolUse,
};

/// Candidate collector for one module position.
pub(crate) struct CompletionBuilder<'owner, 'module, 'program> {
    /// The queried module.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The program query context.
    pub(super) program: &'owner ProgramQueryContext<'program>,
    /// The source file being completed.
    pub(super) file_id: FileId,
}

impl<'owner, 'module, 'program> CompletionBuilder<'owner, 'module, 'program> {
    /// Build one completion builder.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
        file_id: FileId,
    ) -> Self {
        Self {
            module,
            program,
            file_id,
        }
    }

    /// Collect the raw completion candidates for one context.
    pub(crate) fn build(
        &self,
        trigger: CompletionTrigger,
        context: &CompletionContext,
        token: Option<&CursorToken>,
        include_auto_imports: bool,
    ) -> QueryResult<CompletionCandidates> {
        // collect query shaping inputs once up front
        let prefix = token.map_or("", |token| token.text.as_str());
        let allow_short_prefix = matches!(
            trigger,
            CompletionTrigger::Invoked | CompletionTrigger::Incomplete
        );

        // dispatch the primary context-specific candidate builder
        let mut items = match context {
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Access { source },
            } => self.complete_members(*source)?,
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Namespace { module_id },
            } => self.complete_namespace_members(*module_id)?,
            CompletionContext::TypePosition { scope } => self.complete_types(*scope)?,
            CompletionContext::ValuePosition { scope } => self.complete_values(*scope, false)?,
            CompletionContext::StatementPosition { scope } => {
                self.complete_values(*scope, matches!(trigger, CompletionTrigger::Invoked))?
            }
            CompletionContext::ObjectLiteralKey { literal, scope } => {
                self.complete_object_literal(*literal, *scope)?
            }
            CompletionContext::ObjectLiteralValue { scope } => {
                self.complete_values(*scope, false)?
            }
            CompletionContext::CallArgument { call, scope, .. } => {
                // offer the callee's unbound parameter names before values
                let mut candidates = self.complete_unbound_parameters(*call)?;
                candidates.extend(self.complete_values(*scope, false)?);

                candidates
            }
            CompletionContext::NewExpression { scope } => self.complete_new_expression(*scope)?,
            CompletionContext::ImportPath { partial_path } => {
                self.complete_import_paths(partial_path)?
            }
            CompletionContext::ImportClause {
                target_module,
                existing_names,
                use_filter,
            } => self.complete_imports(*target_module, existing_names, *use_filter)?,
        };

        let mut is_incomplete = false;

        // layer in auto imports when this context supports them
        if include_auto_imports && let Some(auto_import) = context.auto_import_search() {
            let auto_imports = self.complete_auto_imports_with_visibility(
                prefix,
                Some(auto_import.symbol_use),
                auto_import.scope,
                allow_short_prefix,
            )?;
            let mut candidates = auto_imports.items;

            if auto_import.is_constructable_only {
                candidates.retain(|completion| completion.kind.is_constructable());
            }

            items.extend(candidates);
            is_incomplete = auto_imports.is_incomplete;
        }

        Ok(CompletionCandidates {
            items,
            is_incomplete,
        })
    }

    /// Complete types in type position.
    fn complete_types(&self, scope: dir::LocalScope) -> QueryResult<Vec<CompletionCandidate>> {
        let symbols = self.module.bindings()?;

        let mut results = Vec::new();
        let mut seen_names = FxHashSet::default();

        // collect visible type names first
        for visible in symbols
            .visible_bindings(scope)
            .filter(|binding| SymbolUse::Type.accepts_symbol_kind(binding.symbol.kind))
        {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind = self.completion_symbol_kind(visible.symbol_id)?;
            let completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.symbol_id,
            };

            results.push(self.resolve_symbol(completion, symbol_id)?);
        }

        // primitive types are always available
        results.extend(primitive_type_completions());

        Ok(results)
    }

    /// Return the exact completion kind for one visible symbol.
    fn completion_symbol_kind(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> QueryResult<CompletionItemKind> {
        let global_id = dir::GlobalSymbolId {
            module_id: self.module.module_id(),
            local_id: symbol_id,
        };
        let Some(canonical_id) = self.program.canonical_symbol(global_id)? else {
            return Err(QueryError::missing(format!(
                "completion declaration: {global_id:?}"
            )));
        };
        let canonical_module = self.program.module(canonical_id.module_id)?;
        let symbols = canonical_module.bindings()?;
        let symbol = symbols.get_symbol(canonical_id.local_id);

        Ok(symbol.into())
    }

    /// Return visible value bindings by lexical precedence.
    pub(super) fn visible_value_bindings(
        &self,
        scope: dir::LocalScope,
    ) -> QueryResult<FxIndexMap<dir::StaticKey, dir::GlobalSymbolId>> {
        let module_id = self.module.module_id();
        let bindings = self.module.bindings()?;
        let mut values = FxIndexMap::default();

        // collect the nearest value for each static name
        for visible in bindings
            .visible_bindings(scope)
            .filter(|binding| SymbolUse::Value.accepts_symbol_kind(binding.symbol.kind))
        {
            let dir::StaticKey::Name(_) = visible.key else {
                continue;
            };
            if values.contains_key(&visible.key) {
                continue;
            }

            let symbol = dir::GlobalSymbolId {
                module_id,
                local_id: visible.symbol_id,
            };
            values.insert(visible.key, symbol);
        }

        Ok(values)
    }

    /// Complete values in expression position.
    fn complete_values(
        &self,
        scope: dir::LocalScope,
        include_keywords: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();

        // build one completion per visible value
        for (key, symbol) in self.visible_value_bindings(scope)? {
            let dir::StaticKey::Name(name) = key else {
                continue;
            };
            let name = self.module.strings().get(name).to_string();
            let kind = self.completion_symbol_kind(symbol.local_id)?;

            // expand declarations with specialized constructor forms
            if kind == CompletionItemKind::Struct {
                results.push(self.complete_struct(&name, symbol)?);
                continue;
            }
            if kind == CompletionItemKind::Newtype {
                results.extend(self.complete_newtype(&name, symbol)?);
                continue;
            }

            // build the ordinary value candidate
            let completion =
                CompletionCandidate::new(&name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);
            let completion = if kind.is_callable() {
                completion.with_call()
            } else {
                completion
            };
            results.push(self.resolve_symbol(completion, symbol)?);
        }

        // statement contexts can opt into keyword completions as well
        if include_keywords {
            results.extend(keyword_completions());
        }

        Ok(results)
    }

    /// Complete constructable symbols for a new expression.
    fn complete_new_expression(
        &self,
        scope: dir::LocalScope,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let symbols = self.module.bindings()?;

        let mut results = Vec::new();
        let mut seen = FxHashSet::default();

        // collect visible constructable names from the active scope
        for visible in symbols.visible_bindings(scope) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen.insert(name.clone()) {
                continue;
            }

            let kind = self.completion_symbol_kind(visible.symbol_id)?;
            if !matches!(kind, CompletionItemKind::Class | CompletionItemKind::Struct) {
                continue;
            }
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.symbol_id,
            };
            if kind == CompletionItemKind::Class {
                results.extend(self.complete_class(&name, symbol_id)?);
            } else {
                let completion = CompletionCandidate::new(
                    name,
                    kind,
                    CompletionOrigin::Local,
                    SORT_LOCAL_SYMBOL,
                );
                results.push(self.resolve_symbol(completion, symbol_id)?);
            }
        }

        Ok(results)
    }
}

impl CompletionContext {
    /// Return the auto import search selected by this completion context.
    fn auto_import_search(&self) -> Option<AutoImportSearch> {
        match self {
            CompletionContext::ValuePosition { scope }
            | CompletionContext::StatementPosition { scope }
            | CompletionContext::ObjectLiteralValue { scope }
            | CompletionContext::CallArgument { scope, .. } => Some(AutoImportSearch {
                symbol_use: SymbolUse::Value,
                scope: *scope,
                is_constructable_only: false,
            }),
            CompletionContext::TypePosition { scope } => Some(AutoImportSearch {
                symbol_use: SymbolUse::Type,
                scope: *scope,
                is_constructable_only: false,
            }),
            CompletionContext::NewExpression { scope } => Some(AutoImportSearch {
                symbol_use: SymbolUse::Value,
                scope: *scope,
                is_constructable_only: true,
            }),
            _ => None,
        }
    }
}
