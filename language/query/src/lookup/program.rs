use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};

use crate::source::{ImportBinding, is_simple_identifier};
use crate::{Module, ProgramQueryContext, QueryError, QueryResult, match_quality};

/// One exact declaration exposed for import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ExportDeclaration {
    /// One exported declaration symbol and its checked kind.
    Symbol {
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The checked symbol kind.
        kind: dir::SymbolKind,
    },
    /// One exported module namespace object.
    Namespace {
        /// The target module.
        module: ModuleId,
    },
}

/// One importable program export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExportCandidate {
    /// The module that exposes the name.
    pub(crate) module: ModuleId,
    /// The import binding to introduce.
    pub(crate) binding: ImportBinding,
    /// The exposed declaration.
    pub(crate) declaration: ExportDeclaration,
}

/// One indexed reference with its owning module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexedReference {
    /// The module profile that owns this reference.
    pub(crate) module: Module,
    /// The indexed reference occurrence.
    pub(crate) entry: dir::ReferenceEntry,
}

impl ProgramQueryContext<'_> {
    /// Return the exact named exports exposed by one indexed module.
    pub(crate) fn module_exports(
        &self,
        module_id: ModuleId,
    ) -> QueryResult<Vec<(String, ExportDeclaration)>> {
        let index = self.module_index(module_id)?;
        let mut entries = Vec::new();

        // transcribe each exact non-default export
        for export in index.exports.entries() {
            if export.name == "default" {
                continue;
            }

            for declaration in self.export_declarations(export.target)? {
                entries.push((export.name.clone(), declaration));
            }
        }

        entries.sort();
        entries.dedup_by(|left, right| left.0 == right.0);

        Ok(entries)
    }

    /// Search export candidates across indexed modules.
    pub(crate) fn search_export_candidates(
        &self,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> QueryResult<Vec<ExportCandidate>> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect exact exports from modules reached by matching names
        let ordinals = self.modules_matching_name(&self.index().exports.names, &query);
        for ordinal in ordinals {
            let (module, index) = self.module_index_at(ordinal)?;
            if Some(module) == exclude_module {
                continue;
            }

            for export in index.exports.entries() {
                if export.name == "default"
                    || (!query.is_empty() && !export.name.to_lowercase().contains(&query))
                {
                    continue;
                }

                for declaration in self.export_declarations(export.target)? {
                    entries.push(ExportCandidate {
                        module,
                        binding: ImportBinding::Named {
                            name: export.name.clone(),
                        },
                        declaration,
                    });
                }
            }
        }

        // expose named declarations exported through the default key
        let default_ordinals = self.modules_matching_name(&self.index().exports.names, "default");
        for ordinal in default_ordinals {
            let (module, index) = self.module_index_at(ordinal)?;
            if Some(module) == exclude_module {
                continue;
            }

            for export in index
                .exports
                .entries()
                .iter()
                .filter(|export| export.name == "default")
            {
                for declaration in self.export_declarations(export.target)? {
                    let ExportDeclaration::Symbol { symbol, .. } = declaration else {
                        continue;
                    };
                    let symbol_index = self.module_index(symbol.module_id)?;
                    let symbol = symbol_index
                        .symbols
                        .entries()
                        .iter()
                        .find(|entry| entry.symbol == symbol)
                        .ok_or(QueryError::missing(format!("program symbol: {symbol:?}")))?;
                    if !is_simple_identifier(&symbol.name)
                        || (!query.is_empty() && !symbol.name.to_lowercase().contains(&query))
                    {
                        continue;
                    }

                    entries.push(ExportCandidate {
                        module,
                        binding: ImportBinding::Default {
                            name: symbol.name.clone(),
                        },
                        declaration,
                    });
                }
            }
        }

        Self::sort_export_entries(&mut entries);

        Ok(entries)
    }

    /// Search symbol candidates across indexed modules.
    pub(crate) fn search_symbol_candidates(
        &self,
        query: &str,
    ) -> QueryResult<Vec<(ProfileId, dir::SymbolEntry)>> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect declaration symbols from modules reached by matching declaration names
        let ordinals = self.modules_matching(&self.index().symbols.names, |name| {
            match_quality(name, &query).is_some()
        });
        for ordinal in ordinals {
            let (module_id, index) = self.module_index_at(ordinal)?;
            if !self.is_authored_module(module_id)? {
                continue;
            }

            // collect matching declarations from the indexed module
            entries.extend(
                index
                    .symbols
                    .entries()
                    .iter()
                    .filter(|entry| match_quality(&entry.name, &query).is_some())
                    .map(|entry| (self.profile_id(), entry.clone())),
            );
        }

        Ok(entries)
    }

    /// Search member candidates across indexed modules.
    pub(crate) fn search_member_candidates(
        &self,
        query: &str,
    ) -> QueryResult<Vec<(ProfileId, dir::MemberEntry)>> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect members from modules reached by matching member names
        let ordinals = self.modules_matching(&self.index().members.names, |name| {
            match_quality(name, &query).is_some()
        });
        for ordinal in ordinals {
            let (module_id, index) = self.module_index_at(ordinal)?;
            if !self.is_authored_module(module_id)? {
                continue;
            }

            // collect matching members from the indexed module
            entries.extend(
                index
                    .members
                    .entries()
                    .iter()
                    .filter(|entry| match_quality(&entry.name, &query).is_some())
                    .map(|entry| (self.profile_id(), entry.clone())),
            );
        }

        Ok(entries)
    }

    /// Collect members declared on one owner symbol.
    pub(crate) fn owner_members(
        &self,
        owner_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::MemberEntry>> {
        let mut entries = Vec::new();

        // collect owner members from modules reached by the owner symbol
        for ordinal in self.index().members.owners.get(&owner_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.members.owner_entries(owner_symbol).cloned());
        }

        entries.sort_by(dir::MemberEntry::compare_by_source);
        entries.dedup();

        Ok(entries)
    }

    /// Collect members contained in one declaring symbol.
    pub(crate) fn declaring_members(
        &self,
        declaring_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::MemberEntry>> {
        let mut entries = Vec::new();

        // collect declaration members from modules reached by the declaring symbol
        for ordinal in self.index().members.declaring.get(&declaring_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.members.declaring_entries(declaring_symbol).cloned());
        }

        entries.sort_by(dir::MemberEntry::compare_by_source);
        entries.dedup();

        Ok(entries)
    }

    /// Search decorator candidates across indexed modules.
    pub(crate) fn search_decorator_candidates(
        &self,
        name: Option<&str>,
    ) -> QueryResult<Vec<(ProfileId, dir::DecoratorEntry)>> {
        let mut entries = Vec::new();

        // select modules reached by the decorator name
        let modules = match name {
            Some(name) => self
                .index()
                .decorators
                .names
                .get(&name.to_string())
                .to_vec(),
            None => self.all_modules(),
        };

        // collect decorators from the selected modules
        for ordinal in modules {
            let (_, index) = self.module_index_at(ordinal)?;
            entries.extend(
                index
                    .decorators
                    .search(name)
                    .cloned()
                    .map(|entry| (self.profile_id(), entry)),
            );
        }

        entries.sort_by_key(|(profile_id, entry)| {
            (
                *profile_id,
                entry.decorator.module_id,
                entry.owner.local_id.id,
                entry.decorator.local_id.id,
                entry.expression.local_id.id,
                entry.name.clone(),
            )
        });
        entries.dedup();

        Ok(entries)
    }

    /// Collect heritage candidates for one base symbol.
    pub(crate) fn base_heritage(
        &self,
        base_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::HeritageEntry>> {
        let mut entries = Vec::new();

        // collect heritage edges from modules reached by the base symbol
        for ordinal in self.index().heritage.bases.get(&base_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.heritage.base_entries(base_symbol).copied());
        }
        entries.sort_by_key(|entry| {
            (
                entry.base,
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.kind,
                entry.derived,
                entry.declaration,
            )
        });
        entries.dedup();

        Ok(entries)
    }

    /// Collect heritage candidates declared by one derived symbol.
    pub(crate) fn derived_heritage(
        &self,
        derived_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::HeritageEntry>> {
        let mut entries = Vec::new();

        // collect heritage edges from modules reached by the derived symbol
        for ordinal in self.index().heritage.derived.get(&derived_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.heritage.derived_entries(derived_symbol).copied());
        }
        entries.sort_by_key(|entry| {
            (
                entry.derived,
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.kind,
                entry.base,
                entry.declaration,
            )
        });
        entries.dedup();

        Ok(entries)
    }

    /// Collect extension index entries for one root symbol.
    pub(crate) fn root_extensions(
        &self,
        root_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::ExtensionEntry>> {
        let mut entries = Vec::new();

        // collect extension declarations from modules reached by the root symbol
        for ordinal in self.index().extensions.roots.get(&root_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.extensions.root_entries(root_symbol).copied());
        }
        entries.sort_by_key(|entry| {
            (
                entry.root,
                entry.declaration,
                entry.file,
                entry.span.start,
                entry.span.end,
                entry.ty,
            )
        });
        entries.dedup();

        Ok(entries)
    }

    /// Collect indexed references to one target symbol.
    pub(crate) fn symbol_reference_entries(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::ReferenceEntry>> {
        let mut entries = self
            .symbol_program_references(target_symbol)?
            .into_iter()
            .map(|reference| reference.entry)
            .collect::<Vec<_>>();

        Self::sort_reference_entries(&mut entries);
        entries.dedup();

        Ok(entries)
    }

    /// Collect indexed program references to one target symbol.
    pub(crate) fn symbol_program_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<IndexedReference>> {
        let mut references = Vec::new();

        // collect reference entries from modules reached by the target symbol
        for ordinal in self.index().references.targets.get(&target_symbol) {
            let (module_id, index) = self.module_index_at(*ordinal)?;
            let module = Module {
                module_id,
                profile_id: self.profile_id(),
            };
            references.extend(index.references.target_entries(target_symbol).map(|entry| {
                IndexedReference {
                    module,
                    entry: *entry,
                }
            }));
        }

        references.sort_by_key(|reference| {
            let entry = reference.entry;

            (
                reference.module.profile_id,
                reference.module.module_id,
                entry.span.file,
                entry.span.start,
                entry.span.end,
            )
        });
        references.dedup();

        Ok(references)
    }

    /// Collect indexed references to one lexical declaration.
    pub(crate) fn symbol_declaration_references(
        &self,
        declaration_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<IndexedReference>> {
        let mut references = Vec::new();

        // collect declaration entries from modules reached by the declaration postings
        for ordinal in self
            .index()
            .references
            .declarations
            .get(&declaration_symbol)
        {
            let (module_id, index) = self.module_index_at(*ordinal)?;
            let module = Module {
                module_id,
                profile_id: self.profile_id(),
            };
            references.extend(
                index
                    .references
                    .declaration_entries(declaration_symbol)
                    .map(|entry| IndexedReference {
                        module,
                        entry: *entry,
                    }),
            );
        }

        references.sort_by_key(|reference| {
            let entry = reference.entry;

            (
                reference.module.profile_id,
                reference.module.module_id,
                entry.span.file,
                entry.span.start,
                entry.span.end,
            )
        });
        references.dedup();

        Ok(references)
    }

    /// Collect call candidates for one callee symbol.
    pub(crate) fn callee_calls(
        &self,
        callee_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::CallEntry>> {
        let mut entries = Vec::new();

        // collect call edges from modules reached by the callee symbol
        for ordinal in self.index().calls.callees.get(&callee_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.calls.callee_entries(callee_symbol).copied());
        }
        Self::sort_call_entries(&mut entries);

        Ok(entries)
    }

    /// Collect call candidates for one caller symbol.
    pub(crate) fn caller_calls(
        &self,
        caller_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::CallEntry>> {
        let mut entries = Vec::new();

        // collect call edges from modules reached by the caller symbol
        for ordinal in self.index().calls.callers.get(&caller_symbol) {
            let (_, index) = self.module_index_at(*ordinal)?;
            entries.extend(index.calls.caller_entries(caller_symbol).copied());
        }
        Self::sort_call_entries(&mut entries);

        Ok(entries)
    }

    /// Sort and deduplicate exported symbol entries.
    fn sort_export_entries(entries: &mut Vec<ExportCandidate>) {
        entries.sort_by(|left, right| {
            (&left.binding, left.module, left.declaration).cmp(&(
                &right.binding,
                right.module,
                right.declaration,
            ))
        });
        entries.dedup();
    }

    /// Resolve one indexed export target to its checked declarations.
    fn export_declarations(
        &self,
        target: dir::ExportTarget,
    ) -> QueryResult<Vec<ExportDeclaration>> {
        match target {
            dir::ExportTarget::Symbol(symbol) => {
                let mut declarations = Vec::new();
                for symbol in self.canonical_symbols(symbol)? {
                    let index = self.module_index(symbol.module_id)?;
                    let entry = index
                        .symbols
                        .entries()
                        .iter()
                        .find(|entry| entry.symbol == symbol)
                        .ok_or(QueryError::missing(format!("program symbol: {symbol:?}")))?;

                    declarations.push(ExportDeclaration::Symbol {
                        symbol,
                        kind: entry.kind,
                    });
                }

                Ok(declarations)
            }
            dir::ExportTarget::Namespace(module) => {
                Ok(vec![ExportDeclaration::Namespace { module }])
            }
        }
    }

    /// Sort and deduplicate call entries.
    fn sort_call_entries(entries: &mut Vec<dir::CallEntry>) {
        entries.sort_by_key(|entry| {
            (
                entry.source.module_id,
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.source.local_id.id,
                entry.caller,
                entry.callee,
                entry.kind,
            )
        });
        entries.dedup();
    }

    /// Sort reference entries by source position and identity.
    fn sort_reference_entries(entries: &mut [dir::ReferenceEntry]) {
        entries.sort_by_key(|entry| {
            (
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.source,
                entry.symbol,
                entry.is_import_alias,
            )
        });
    }

    /// Return module ordinals whose string keys contain the query.
    fn modules_matching_name(&self, postings: &dir::Postings<String>, query: &str) -> Vec<u32> {
        self.modules_matching(postings, |name| {
            query.is_empty() || name.to_lowercase().contains(query)
        })
    }

    /// Return module ordinals reached by matching posting keys.
    fn modules_matching(
        &self,
        postings: &dir::Postings<String>,
        matches: impl Fn(&str) -> bool,
    ) -> Vec<u32> {
        let mut modules = Vec::new();

        for (index, key) in postings.keys.iter().enumerate() {
            if matches(key) {
                modules.extend(postings.range(index));
            }
        }

        modules.sort();
        modules.dedup();

        modules
    }

    /// Return every module ordinal in this profile.
    fn all_modules(&self) -> Vec<u32> {
        (0..self.index().modules.len() as u32).collect()
    }
}
