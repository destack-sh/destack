#![allow(clippy::too_many_arguments)]

use std::collections::HashSet;
use std::sync::Arc;

use destack_dir as dir;
use destack_source::File;

use super::builtin::{keyword_completions, primitive_type_completions};
use super::call::call_snippet;
use super::rank::filter_completions;
use super::{
    Completion, CompletionContext, CompletionCursor, CompletionKind, CompletionTrigger,
    CursorToken, SORT_LOCAL_SYMBOL,
};
use crate::{ModuleQueryContext, ProgramQueryContext, ScopeAtOffset, SymbolUse, visible_symbols};

/// The repository-bound builder for completion candidates.
pub(super) struct CompletionBuilder<'owner, 'repo> {
    /// The queried module.
    pub(super) module: &'owner ModuleQueryContext<'repo>,
    /// The program query context.
    pub(super) program: &'owner ProgramQueryContext<'repo>,
    /// The current file being completed.
    pub(super) source_file: Arc<File>,
}

/// Completion-specific predicates for DIR symbol kinds.
trait ConstructableSymbolKind {
    /// Return whether this symbol kind is constructable with `new`.
    fn is_constructable(self) -> bool;
}

impl ConstructableSymbolKind for dir::SymbolKind {
    fn is_constructable(self) -> bool {
        matches!(self, dir::SymbolKind::Class | dir::SymbolKind::Struct)
    }
}

impl<'owner, 'repo> CompletionBuilder<'owner, 'repo> {
    /// Build one completion builder.
    pub(super) fn new(
        module: &'owner ModuleQueryContext<'repo>,
        program: &'owner ProgramQueryContext<'repo>,
    ) -> Self {
        let source_file = module.source_file();

        Self {
            module,
            program,
            source_file,
        }
    }

    /// Collect the raw completion candidates for one context.
    pub(super) fn completion_candidates(
        &self,
        offset: u32,
        trigger: CompletionTrigger,
        context: &CompletionContext,
        token: Option<&CursorToken>,
    ) -> Vec<Completion> {
        // collect query-shaping inputs once up front
        let excluded_labels = if context.uses_initializer_exclusions() {
            self.completion_excluded_labels(offset)
        } else {
            HashSet::new()
        };
        let prefix = token.map_or("", |token| token.text.as_str());
        let allow_short_prefix = matches!(trigger, CompletionTrigger::Invoked);

        // dispatch the primary context-specific candidate builder
        let mut results = match context {
            CompletionContext::MemberAccess {
                receiver_node: _,
                receiver_symbol,
                receiver_type,
            } => self.complete_members(*receiver_type, *receiver_symbol),
            CompletionContext::TypePosition { scope } => self.complete_types(*scope),
            CompletionContext::ValuePosition { scope } => {
                self.complete_values(*scope, &excluded_labels, false)
            }
            CompletionContext::StatementPosition { scope } => self.complete_values(
                *scope,
                &excluded_labels,
                matches!(trigger, CompletionTrigger::Invoked),
            ),
            CompletionContext::ObjectLiteral {
                contextual_type,
                existing_fields,
                scope,
                ..
            } => self.complete_object_literal(
                *contextual_type,
                existing_fields,
                *scope,
                &excluded_labels,
            ),
            CompletionContext::ObjectLiteralValue { scope } => {
                self.complete_values(*scope, &excluded_labels, false)
            }
            CompletionContext::CallArgument { scope, .. } => {
                self.complete_values(*scope, &excluded_labels, false)
            }
            CompletionContext::NewExpression { scope } => {
                self.complete_new_expression(*scope, &excluded_labels)
            }
            CompletionContext::ImportPath { partial_path } => {
                self.complete_import_paths(partial_path)
            }
            CompletionContext::ImportClause {
                target_module,
                existing_names,
                use_filter,
            } => self.complete_imports(*target_module, existing_names, *use_filter),
            CompletionContext::Suppressed => Vec::new(),
        };

        // layer in auto imports when this context supports them
        if let Some((use_filter, scope, constructable_only)) = context.auto_import_settings() {
            let mut auto_imports = self.complete_auto_imports_with_visibility(
                prefix,
                Some(use_filter),
                scope,
                &excluded_labels,
                allow_short_prefix,
            );

            if constructable_only {
                auto_imports.retain(|completion| completion.kind.is_constructable());
            }

            results.extend(auto_imports);
        }

        results
    }

    /// Collect completion labels excluded at one cursor offset.
    fn completion_excluded_labels(&self, offset: u32) -> HashSet<String> {
        let source = self.source_file.text();

        self.module
            .current_initializer_binding_names(source, offset)
            .into_iter()
            .map(|name| self.module.strings().get(name).to_string())
            .collect()
    }

    /// Complete types in type position.
    fn complete_types(&self, scope: ScopeAtOffset) -> Vec<Completion> {
        let symbols = self.module.symbols();
        let view = self.module.view();

        let mut results = Vec::new();
        let mut seen_names = HashSet::new();

        // collect visible type names first
        for visible in visible_symbols(
            symbols,
            scope.scope_id,
            scope.scope_mark,
            Some(SymbolUse::Type),
        ) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind =
                CompletionKind::from(self.completion_symbol_kind(visible.id, visible.symbol));
            let completion = Completion::new(name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.id,
            };

            results.push(self.attach_completion_documentation(completion, symbol_id));
        }

        // then include exported dependency items usable as types
        for (item_id, _) in view.iter_nodes_of_type::<dir::DependencyItem>() {
            let Some(symbol_id) = self.module.node_symbol(item_id.into()) else {
                continue;
            };

            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.kind.can_be_used_as_type() {
                continue;
            }

            let Some(name_id) = symbol.name() else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind = CompletionKind::from(self.completion_symbol_kind(symbol_id, symbol));
            let completion = Completion::new(name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: symbol_id,
            };

            results.push(self.attach_completion_documentation(completion, symbol_id));
        }

        // primitive types are always available
        results.extend(primitive_type_completions());

        results
    }

    /// Return the completion kind source for one visible symbol.
    fn completion_symbol_kind(
        &self,
        symbol_id: dir::LocalSymbolId,
        symbol: &dir::Symbol,
    ) -> dir::SymbolKind {
        if symbol.kind != dir::SymbolKind::Variable {
            return symbol.kind;
        }

        let global_id = dir::GlobalSymbolId {
            module_id: self.module.module_id(),
            local_id: symbol_id,
        };
        let canonical_id = self.canonical_symbol(global_id);
        if canonical_id == global_id {
            return symbol.kind;
        }

        let canonical_module = self.module.module_context(canonical_id.module_id);
        let canonical_symbol = canonical_module.symbols().get_symbol(canonical_id.local_id);

        canonical_symbol.kind
    }

    /// Complete values in expression position.
    fn complete_values(
        &self,
        scope: ScopeAtOffset,
        excluded_labels: &HashSet<String>,
        include_keywords: bool,
    ) -> Vec<Completion> {
        let module_id = self.module.module_id();

        let mut results = Vec::new();

        // snapshot the visible value symbols before building completions
        let symbols_to_process: Vec<(dir::LocalSymbolId, String, dir::SymbolKind)> = {
            let symbols = self.module.symbols();
            let mut symbols_to_process = Vec::new();
            let mut seen_names = HashSet::new();

            for visible in visible_symbols(
                symbols,
                scope.scope_id,
                scope.scope_mark,
                Some(SymbolUse::Value),
            ) {
                let dir::StaticKey::Name(name_id) = visible.key else {
                    continue;
                };

                let name = self.module.strings().get(name_id).to_string();
                if excluded_labels.contains(&name) {
                    continue;
                }

                if !seen_names.insert(name.clone()) {
                    continue;
                }

                symbols_to_process.push((visible.id, name, visible.symbol.kind));
            }

            symbols_to_process
        };

        // build one completion per visible symbol
        for (local_id, name, symbol_kind) in symbols_to_process {
            let kind = CompletionKind::from(symbol_kind);
            let mut completion = Completion::new(&name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id,
                local_id,
            };

            if let Some(value_shape) = self.symbol_value_shape(symbol_id) {
                completion = completion.with_value_shape(value_shape);
            }

            if let Some(nominal_symbol) = self.symbol_nominal(symbol_id) {
                completion = completion.with_nominal_symbol(nominal_symbol);
            }
            completion = completion.with_related_nominals(self.symbol_related_nominals(symbol_id));

            if symbol_kind == dir::SymbolKind::Function {
                if let Some(parameter_names) = self.module.symbol_parameter_names(symbol_id) {
                    let snippet = call_snippet(&name, &parameter_names);
                    completion = completion.with_insert_text(snippet.text);
                    if snippet.is_snippet {
                        completion = completion.as_snippet();
                    }
                }
            }

            completion = self.attach_completion_documentation(completion, symbol_id);
            results.push(completion);
        }

        // statement contexts can opt into keyword completions as well
        if include_keywords {
            results.extend(keyword_completions());
        }

        results
    }

    /// Complete constructable symbols for a new expression.
    fn complete_new_expression(
        &self,
        scope: ScopeAtOffset,
        excluded_labels: &HashSet<String>,
    ) -> Vec<Completion> {
        let symbols = self.module.symbols();

        let mut results = Vec::new();
        let mut seen = HashSet::new();

        // collect visible constructable names from the active scope
        for visible in visible_symbols(symbols, scope.scope_id, scope.scope_mark, None) {
            if !visible.symbol.kind.is_constructable() {
                continue;
            }

            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if excluded_labels.contains(&name) {
                continue;
            }

            if !seen.insert(name.clone()) {
                continue;
            }

            let kind = CompletionKind::from(visible.symbol.kind);
            let completion = Completion::new(name, kind)
                .with_sort_order(SORT_LOCAL_SYMBOL)
                .as_local();
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.id,
            };
            let completion = if let Some(value_shape) = self.symbol_value_shape(symbol_id) {
                completion.with_value_shape(value_shape)
            } else {
                completion
            };
            let completion = if let Some(nominal_symbol) = self.symbol_nominal(symbol_id) {
                completion.with_nominal_symbol(nominal_symbol)
            } else {
                completion
            };
            let completion =
                completion.with_related_nominals(self.symbol_related_nominals(symbol_id));
            let completion = self.attach_completion_documentation(completion, symbol_id);

            results.push(completion);
        }

        results
    }
}

impl ModuleQueryContext<'_> {
    /// Return completions at the given position.
    pub fn completions(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
        trigger: CompletionTrigger,
    ) -> Vec<Completion> {
        let CompletionCursor { context, token } = self.completion_cursor(offset);
        let builder = CompletionBuilder::new(self, program);
        let completions = builder.completion_candidates(offset, trigger, &context, token.as_ref());

        filter_completions(completions, &context, token.as_ref())
    }
}

impl CompletionContext {
    /// Return auto import search settings for this completion context.
    fn auto_import_settings(&self) -> Option<(SymbolUse, ScopeAtOffset, bool)> {
        match self {
            CompletionContext::ValuePosition { scope }
            | CompletionContext::StatementPosition { scope }
            | CompletionContext::ObjectLiteralValue { scope }
            | CompletionContext::CallArgument { scope, .. } => {
                Some((SymbolUse::Value, *scope, false))
            }
            CompletionContext::TypePosition { scope } => Some((SymbolUse::Type, *scope, false)),
            CompletionContext::NewExpression { scope } => Some((SymbolUse::Value, *scope, true)),
            _ => None,
        }
    }

    /// Return whether this context should exclude initializer bindings.
    fn uses_initializer_exclusions(&self) -> bool {
        matches!(
            self,
            CompletionContext::ValuePosition { .. }
                | CompletionContext::StatementPosition { .. }
                | CompletionContext::ObjectLiteral { .. }
                | CompletionContext::ObjectLiteralValue { .. }
                | CompletionContext::CallArgument { .. }
                | CompletionContext::NewExpression { .. }
        )
    }
}
