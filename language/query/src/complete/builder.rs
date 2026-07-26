use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::{File, FileId};
use rustc_hash::FxHashSet;

use super::builtin::{keyword_completions, primitive_type_completions};
use super::call::call_snippet;
use super::{CompletionContext, CompletionReceiver, CursorToken};
use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin,
    CompletionTrigger, ModuleQueryContext, ProgramQueryContext, QueryResult, SORT_LOCAL_SYMBOL,
    ScopeAtOffset, SymbolUse, visible_symbols,
};

/// The repository-bound builder for completion candidates.
pub(crate) struct CompletionBuilder<'owner, 'module, 'program> {
    /// The queried module.
    pub(super) module: &'owner ModuleQueryContext<'module>,
    /// The program query context.
    pub(super) program: &'owner ProgramQueryContext<'program>,
    /// The active profile language environment.
    pub(super) environment: &'owner GlobalEnvironment,
    /// The physical source file being completed.
    pub(super) file_id: FileId,
    /// The current file being completed.
    pub(super) source_file: Arc<File>,
}

impl<'owner, 'module, 'program> CompletionBuilder<'owner, 'module, 'program> {
    /// Build one completion builder.
    pub(crate) fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
        environment: &'owner GlobalEnvironment,
        file_id: FileId,
    ) -> QueryResult<Self> {
        let source_file = module.read_file(file_id)?;

        Ok(Self {
            module,
            program,
            environment,
            file_id,
            source_file,
        })
    }

    /// Collect the raw completion candidates for one context.
    pub(crate) fn completion_candidates(
        &self,
        trigger: CompletionTrigger,
        context: &CompletionContext,
        token: Option<&CursorToken>,
        include_auto_imports: bool,
    ) -> QueryResult<CompletionCandidates> {
        // collect query shaping inputs once up front
        let prefix = token.map_or("", |token| token.text.as_str());
        let allow_short_prefix = matches!(trigger, CompletionTrigger::Invoked);

        // dispatch the primary context-specific candidate builder
        let mut items = match context {
            CompletionContext::MemberAccess {
                receiver:
                    CompletionReceiver::Type {
                        type_id,
                        is_optional,
                    },
            } => self.complete_members(Some(*type_id), *is_optional)?,
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Namespace { module_id },
            } => self.complete_namespace_members(*module_id)?,
            CompletionContext::MemberAccess {
                receiver: CompletionReceiver::Missing,
            } => Vec::new(),
            CompletionContext::TypePosition { scope } => self.complete_types(*scope)?,
            CompletionContext::ValuePosition { scope } => self.complete_values(*scope, false)?,
            CompletionContext::StatementPosition { scope } => {
                self.complete_values(*scope, matches!(trigger, CompletionTrigger::Invoked))?
            }
            CompletionContext::ObjectLiteralKey {
                existing_fields,
                scope,
            } => self.complete_object_literal_shorthands(existing_fields, *scope)?,
            CompletionContext::ObjectLiteralValue { scope } => {
                self.complete_values(*scope, false)?
            }
            CompletionContext::CallArgument { scope, .. } => self.complete_values(*scope, false)?,
            CompletionContext::NewExpression { scope } => self.complete_new_expression(*scope)?,
            CompletionContext::ImportPath { partial_path } => {
                self.complete_import_paths(partial_path)?
            }
            CompletionContext::ImportClause {
                target_module,
                existing_names,
                use_filter,
            } => self.complete_imports(*target_module, existing_names, *use_filter)?,
            CompletionContext::Suppressed => Vec::new(),
        };

        let mut is_incomplete = false;

        // layer in auto imports when this context supports them
        if include_auto_imports
            && let Some((use_filter, scope, constructable_only)) = context.auto_import_settings()
        {
            let auto_imports = self.complete_auto_imports_with_visibility(
                prefix,
                Some(use_filter),
                scope,
                allow_short_prefix,
            )?;
            let mut candidates = auto_imports.items;

            if constructable_only {
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
    fn complete_types(&self, scope: ScopeAtOffset) -> QueryResult<Vec<CompletionCandidate>> {
        let symbols = self.module.symbols();
        let view = self.module.view();

        let mut results = Vec::new();
        let mut seen_names = FxHashSet::default();

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

            let Some(kind) = self.completion_symbol_kind(visible.id)? else {
                continue;
            };
            let completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.id,
            };

            results.push(self.attach_symbol_completion(completion, symbol_id)?);
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

            let Some(kind) = self.completion_symbol_kind(symbol_id)? else {
                continue;
            };
            let completion =
                CompletionCandidate::new(name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: symbol_id,
            };

            results.push(self.attach_symbol_completion(completion, symbol_id)?);
        }

        // primitive types are always available
        results.extend(primitive_type_completions());

        Ok(results)
    }

    /// Return the exact completion kind for one visible symbol.
    fn completion_symbol_kind(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> QueryResult<Option<CompletionItemKind>> {
        let global_id = dir::GlobalSymbolId {
            module_id: self.module.module_id(),
            local_id: symbol_id,
        };
        let Some(canonical_id) = self.program.canonical_symbol(global_id)? else {
            return Ok(None);
        };
        let canonical_module = self.program.module(canonical_id.module_id)?;
        let symbols = canonical_module.symbols();
        let symbol = symbols.get_symbol(canonical_id.local_id);

        // distinguish value parameters from ordinary variable bindings
        if let Some(declaration) = symbol.declaration {
            let view = canonical_module.view();
            let is_parameter = declaration.local_id.ty == dir::NodeType::Parameter
                || (declaration.local_id.ty == dir::NodeType::Pattern
                    && view
                        .get_parent_any(declaration.local_id)
                        .is_some_and(|parent| parent.ty == dir::NodeType::Parameter));
            if is_parameter {
                return Ok(Some(CompletionItemKind::ValueParameter));
            }
        }

        Ok(Some(symbol.into()))
    }

    /// Complete values in expression position.
    fn complete_values(
        &self,
        scope: ScopeAtOffset,
        include_keywords: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let module_id = self.module.module_id();

        let mut results = Vec::new();

        // collect visible value symbols before building completions
        let candidates: Vec<(dir::LocalSymbolId, String, CompletionItemKind)> = {
            let symbols = self.module.symbols();
            let mut candidates = Vec::new();
            let mut seen_names = FxHashSet::default();

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
                if !seen_names.insert(name.clone()) {
                    continue;
                }

                let Some(kind) = self.completion_symbol_kind(visible.id)? else {
                    continue;
                };

                candidates.push((visible.id, name, kind));
            }

            candidates
        };

        // build one completion per visible symbol
        for (local_id, name, kind) in candidates {
            let symbol_id = dir::GlobalSymbolId {
                module_id,
                local_id,
            };
            if kind == CompletionItemKind::Struct {
                results.push(self.complete_struct(&name, symbol_id)?);
                continue;
            }
            if kind == CompletionItemKind::Newtype {
                results.push(self.complete_newtype(&name, symbol_id)?);
                continue;
            }

            let mut completion =
                CompletionCandidate::new(&name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);

            if kind == CompletionItemKind::Function
                && let Some(parameter_names) = self.program.symbol_parameter_names(symbol_id)?
            {
                let snippet = call_snippet(&name, &parameter_names);
                completion = completion.with_insert_text(snippet.text);
                if snippet.is_snippet {
                    completion = completion.with_snippet();
                }
            }

            completion = self.attach_symbol_completion(completion, symbol_id)?;
            results.push(completion);
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
        scope: ScopeAtOffset,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let symbols = self.module.symbols();

        let mut results = Vec::new();
        let mut seen = FxHashSet::default();

        // collect visible constructable names from the active scope
        for visible in visible_symbols(symbols, scope.scope_id, scope.scope_mark, None) {
            if !matches!(
                visible.symbol.kind,
                dir::SymbolKind::Class | dir::SymbolKind::Struct
            ) {
                continue;
            }

            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen.insert(name.clone()) {
                continue;
            }

            let kind = CompletionItemKind::from(visible.symbol);
            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.id,
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
                results.push(self.attach_symbol_completion(completion, symbol_id)?);
            }
        }

        Ok(results)
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
}
