use tspp_dir as dir;
use tspp_source::{ModuleId, ProfileId};

use crate::source::{ImportBinding, is_simple_identifier};
use crate::{Module, ProgramQueryContext, QueryError, QueryResult, match_quality};

/// One exact declaration exposed for import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ExportDeclaration {
    /// One exported declaration symbol and kind.
    Symbol {
        /// The declaration symbol.
        symbol: dir::GlobalSymbolId,
        /// The declaration kind.
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
    /// The export target.
    pub(crate) target: dir::ExportTarget,
}

impl ExportCandidate {
    /// Resolve the exact declarations exposed by this export.
    pub(crate) fn resolve_declarations(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<ExportDeclaration>> {
        program.export_declarations(&self.target)
    }
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
        let index = self.export_index(module_id)?;
        let mut entries = Vec::new();

        // collect each non-default export
        for export in index.entries() {
            if export.name == "default" {
                continue;
            }

            for declaration in self.export_declarations(&export.target)? {
                entries.push((export.name.clone(), declaration));
            }
        }

        entries.sort();
        entries.dedup();

        Ok(entries)
    }

    /// Search export candidates across indexed modules.
    pub(crate) fn search_export_candidates(
        &self,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> QueryResult<Vec<ExportCandidate>> {
        let mut entries = Vec::new();

        // collect exact exports from modules reached by matching names
        let ordinals = self.modules_matching(&self.export_postings()?.names, |name| {
            match_quality(name, query).is_some()
        });
        for ordinal in ordinals {
            let (module, index) = self.export_index_at(ordinal)?;
            if Some(module) == exclude_module {
                continue;
            }

            for export in index.entries() {
                if export.name == "default" || match_quality(&export.name, query).is_none() {
                    continue;
                }

                entries.push(ExportCandidate {
                    module,
                    binding: ImportBinding::Named {
                        name: export.name.clone(),
                    },
                    target: export.target.clone(),
                });
            }
        }

        // expose named declarations exported through the default key
        let default_name = "default".to_string();
        let default_ordinals = self.export_postings()?.names.get(&default_name).to_vec();
        for ordinal in default_ordinals {
            let (module, index) = self.export_index_at(ordinal)?;
            if Some(module) == exclude_module {
                continue;
            }

            for export in index
                .entries()
                .iter()
                .filter(|export| export.name == "default")
            {
                for declaration in self.export_declarations(&export.target)? {
                    let ExportDeclaration::Symbol { symbol, .. } = declaration else {
                        continue;
                    };
                    let symbol_index = self.symbol_index(symbol.module_id)?;
                    let symbol = symbol_index
                        .entries()
                        .iter()
                        .find(|entry| entry.symbol == symbol)
                        .ok_or(QueryError::missing(format!("program symbol: {symbol:?}")))?;
                    if !is_simple_identifier(&symbol.name)
                        || match_quality(&symbol.name, query).is_none()
                    {
                        continue;
                    }

                    entries.push(ExportCandidate {
                        module,
                        binding: ImportBinding::Default {
                            name: symbol.name.clone(),
                        },
                        target: export.target.clone(),
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
        let ordinals = self.modules_matching(&self.symbol_postings()?.names, |name| {
            match_quality(name, &query).is_some()
        });
        for ordinal in ordinals {
            let (module_id, index) = self.symbol_index_at(ordinal)?;
            if !self.is_authored_module(module_id)? {
                continue;
            }

            // collect matching declarations from the indexed module
            entries.extend(
                index
                    .entries()
                    .iter()
                    .filter(|entry| match_quality(&entry.name, &query).is_some())
                    .map(|entry| (self.profile_id(), entry.clone())),
            );
        }

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
                .decorator_postings()?
                .names
                .get(&name.to_string())
                .to_vec(),
            None => self.module_ordinals(),
        };

        // collect decorators from the selected modules
        for ordinal in modules {
            let (_, index) = self.decorator_index_at(ordinal)?;
            entries.extend(
                index
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
        for ordinal in self.heritage_postings()?.bases.get(&base_symbol) {
            let (_, index) = self.heritage_index_at(*ordinal)?;
            entries.extend(index.base_entries(base_symbol).copied());
        }
        entries.sort_by_key(|entry| {
            (
                entry.base,
                entry.derived,
                entry.declaration,
                entry.ordinal,
                entry.kind,
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
        for ordinal in self.heritage_postings()?.derived.get(&derived_symbol) {
            let (_, index) = self.heritage_index_at(*ordinal)?;
            entries.extend(index.derived_entries(derived_symbol).copied());
        }
        entries.sort_by_key(|entry| {
            (
                entry.derived,
                entry.declaration,
                entry.ordinal,
                entry.kind,
                entry.base,
            )
        });
        entries.dedup();

        Ok(entries)
    }

    /// Collect members implementing one declared member.
    pub(crate) fn member_implementations(
        &self,
        declaration: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut implementations = Vec::new();

        // collect edges from modules reached by the declared member
        for ordinal in self.member_postings()?.declarations.get(&declaration) {
            let (_, index) = self.member_index_at(*ordinal)?;
            implementations.extend(index.implementations(declaration));
        }

        // normalize result order
        implementations.sort();
        implementations.dedup();

        Ok(implementations)
    }

    /// Collect member declarations satisfied by one implementation.
    pub(crate) fn member_declarations(
        &self,
        implementation: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut declarations = Vec::new();

        // collect edges from modules reached by the implementing member
        for ordinal in self.member_postings()?.implementations.get(&implementation) {
            let (_, index) = self.member_index_at(*ordinal)?;
            declarations.extend(index.declarations(implementation));
        }

        // normalize result order
        declarations.sort();
        declarations.dedup();

        Ok(declarations)
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
        for ordinal in self.reference_postings()?.targets.get(&target_symbol) {
            let (module_id, index) = self.reference_index_at(*ordinal)?;
            let module = Module {
                module_id,
                profile_id: self.profile_id(),
            };
            references.extend(
                index
                    .target_entries(target_symbol)
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

    /// Collect indexed references to one lexical declaration.
    pub(crate) fn symbol_declaration_references(
        &self,
        declaration_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<IndexedReference>> {
        let mut references = Vec::new();

        // collect declaration entries from modules reached by the declaration postings
        for ordinal in self
            .reference_postings()?
            .declarations
            .get(&declaration_symbol)
        {
            let (module_id, index) = self.reference_index_at(*ordinal)?;
            let module = Module {
                module_id,
                profile_id: self.profile_id(),
            };
            references.extend(index.declaration_entries(declaration_symbol).map(|entry| {
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

    /// Collect call candidates for one callee symbol.
    pub(crate) fn callee_calls(
        &self,
        callee_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::CallEntry>> {
        let mut entries = Vec::new();

        // collect call edges from modules reached by the callee symbol
        for ordinal in self.call_postings()?.callees.get(&callee_symbol) {
            let (_, index) = self.call_index_at(*ordinal)?;
            entries.extend(index.callee_entries(callee_symbol).copied());
        }
        Self::sort_call_entries(&mut entries);

        Ok(entries)
    }

    /// Collect call candidates for one caller symbol.
    pub(crate) fn caller_calls(
        &self,
        caller_symbol: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::CallEntry>> {
        let index = self.call_index(caller_symbol.module_id)?;
        let mut entries = index
            .caller_entries(caller_symbol)
            .copied()
            .collect::<Vec<_>>();
        Self::sort_call_entries(&mut entries);

        Ok(entries)
    }

    /// Sort and deduplicate exported symbol entries.
    fn sort_export_entries(entries: &mut Vec<ExportCandidate>) {
        entries.sort_by(|left, right| {
            (&left.binding, left.module, &left.target).cmp(&(
                &right.binding,
                right.module,
                &right.target,
            ))
        });
        entries.dedup();
    }

    /// Resolve one indexed export target to its declarations.
    fn export_declarations(
        &self,
        target: &dir::ExportTarget,
    ) -> QueryResult<Vec<ExportDeclaration>> {
        match target {
            dir::ExportTarget::Symbols(symbols) => {
                let mut declarations = Vec::new();
                for symbol in symbols {
                    let index = self.symbol_index(symbol.module_id)?;
                    let entry = index
                        .entries()
                        .iter()
                        .find(|entry| entry.symbol == *symbol)
                        .ok_or(QueryError::missing(format!("program symbol: {symbol:?}")))?;

                    declarations.push(ExportDeclaration::Symbol {
                        symbol: *symbol,
                        kind: entry.kind,
                    });
                }

                Ok(declarations)
            }
            dir::ExportTarget::Namespace(module) => {
                Ok(vec![ExportDeclaration::Namespace { module: *module }])
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
                entry.is_alias,
            )
        });
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
    fn module_ordinals(&self) -> Vec<u32> {
        (0..self.module_ids().len() as u32).collect()
    }
}
