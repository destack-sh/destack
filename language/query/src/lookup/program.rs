use std::collections::HashSet;
use std::path::PathBuf;

use destack_artifact::ModuleIndex;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};

use crate::{ProgramQueryContext, ProgramQueryProfile};

/// One indexed reference with its owning profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProgramReference {
    /// The profile that owns this reference.
    pub(crate) profile_id: ProfileId,
    /// The indexed reference occurrence.
    pub(crate) entry: dir::ReferenceEntry,
}

impl ProgramQueryContext<'_> {
    /// Search export candidates across indexed modules.
    pub(crate) fn search_export_candidates(
        &self,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<dir::ExportEntry> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect export candidates from modules reached by matching export names
        for profile in self.profiles() {
            let modules = profile.modules_matching_name(&profile.index().exports.names, &query);
            for index in profile.module_indexes(&modules) {
                entries.extend(
                    index
                        .exports
                        .entries()
                        .iter()
                        .filter(|entry| Some(entry.source.module_id) != exclude_module)
                        .filter(|entry| {
                            query.is_empty() || entry.name.to_lowercase().contains(&query)
                        })
                        .cloned(),
                );
            }
        }

        Self::sort_export_entries(&mut entries);

        entries
    }

    /// Search symbol candidates across indexed modules.
    pub(crate) fn search_symbol_candidates(
        &self,
        query: &str,
    ) -> Vec<(ProfileId, dir::SymbolEntry)> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect declaration symbols from modules reached by matching declaration names
        for profile in self.profiles() {
            let modules = profile.modules_matching_name(&profile.index().symbols.names, &query);
            for index in profile.module_indexes(&modules) {
                entries.extend(
                    index
                        .symbols
                        .entries()
                        .iter()
                        .filter(|entry| entry.name.to_lowercase().contains(&query))
                        .map(|entry| (profile.profile_id(), entry.clone())),
                );
            }
        }

        entries.sort_by(|left, right| {
            (
                left.0,
                left.1.name.as_str(),
                left.1.source.module_id,
                left.1.file.0,
                left.1.span.start,
                left.1.span.end,
            )
                .cmp(&(
                    right.0,
                    right.1.name.as_str(),
                    right.1.source.module_id,
                    right.1.file.0,
                    right.1.span.start,
                    right.1.span.end,
                ))
        });
        entries.dedup();

        entries
    }

    /// Search member candidates across indexed modules.
    pub(crate) fn search_member_candidates(
        &self,
        query: &str,
    ) -> Vec<(ProfileId, dir::MemberEntry)> {
        let mut entries = Vec::new();
        let query = query.to_lowercase();

        // collect source members from modules reached by matching member names
        for profile in self.profiles() {
            let modules = profile.modules_matching_name(&profile.index().members.names, &query);
            for index in profile.module_indexes(&modules) {
                entries.extend(
                    index
                        .members
                        .entries()
                        .iter()
                        .filter(|entry| entry.name.to_lowercase().contains(&query))
                        .map(|entry| (profile.profile_id(), entry.clone())),
                );
            }
        }

        entries.sort_by(|left, right| {
            (
                left.0,
                left.1.name.as_str(),
                left.1.source.module_id,
                left.1.file.0,
                left.1.span.start,
                left.1.span.end,
            )
                .cmp(&(
                    right.0,
                    right.1.name.as_str(),
                    right.1.source.module_id,
                    right.1.file.0,
                    right.1.span.start,
                    right.1.span.end,
                ))
        });
        entries.dedup();

        entries
    }

    /// Collect members declared on one owner symbol.
    pub(crate) fn owner_members(&self, owner_symbol: dir::GlobalSymbolId) -> Vec<dir::MemberEntry> {
        let mut entries = Vec::new();

        // collect owner members from modules reached by the owner symbol
        for profile in self.profiles() {
            for index in profile.module_indexes(profile.index().members.owners.get(&owner_symbol)) {
                entries.extend(index.members.owner_entries(owner_symbol).cloned());
            }
        }

        entries.sort_by(dir::MemberEntry::compare_by_source);
        entries.dedup();

        entries
    }

    /// Collect members contained in one declaring symbol.
    pub(crate) fn declaring_members(
        &self,
        declaring_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::MemberEntry> {
        let mut entries = Vec::new();

        // collect declaration members from modules reached by the declaring symbol
        for profile in self.profiles() {
            for index in
                profile.module_indexes(profile.index().members.declaring.get(&declaring_symbol))
            {
                entries.extend(index.members.declaring_entries(declaring_symbol).cloned());
            }
        }

        entries.sort_by(dir::MemberEntry::compare_by_source);
        entries.dedup();

        entries
    }

    /// Search decorator candidates across indexed modules.
    pub(crate) fn search_decorator_candidates(
        &self,
        name: Option<&str>,
    ) -> Vec<(ProfileId, dir::DecoratorEntry)> {
        let mut entries = Vec::new();

        // collect decorators from modules reached by the decorator name
        for profile in self.profiles() {
            let modules = match name {
                Some(name) => profile
                    .index()
                    .decorators
                    .names
                    .get(&name.to_string())
                    .to_vec(),
                None => profile.all_modules(),
            };

            for index in profile.module_indexes(&modules) {
                entries.extend(
                    index
                        .decorators
                        .search(name)
                        .cloned()
                        .map(|entry| (profile.profile_id(), entry)),
                );
            }
        }

        entries.sort_by_key(|(profile_id, entry)| {
            (
                *profile_id,
                entry.name.clone(),
                entry.decorator.module_id,
                entry.target.local_id.id,
                entry.decorator.local_id.id,
                entry.expression.local_id.id,
            )
        });
        entries.dedup();

        entries
    }

    /// Collect heritage candidates for one base symbol.
    pub(crate) fn base_heritage(
        &self,
        base_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::HeritageEntry> {
        let mut entries = Vec::new();

        // collect heritage edges from modules reached by the base symbol
        for profile in self.profiles() {
            for index in profile.module_indexes(profile.index().heritage.bases.get(&base_symbol)) {
                entries.extend(index.heritage.base_entries(base_symbol).cloned());
            }
        }
        entries.sort();
        entries.dedup();

        entries
    }

    /// Collect heritage candidates declared by one derived symbol.
    pub(crate) fn derived_heritage(
        &self,
        derived_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::HeritageEntry> {
        let mut entries = Vec::new();

        // collect heritage edges from modules reached by the derived symbol
        for profile in self.profiles() {
            for index in
                profile.module_indexes(profile.index().heritage.derived.get(&derived_symbol))
            {
                entries.extend(index.heritage.derived_entries(derived_symbol).cloned());
            }
        }
        entries.sort();
        entries.dedup();

        entries
    }

    /// Collect extension index entries for one root symbol.
    pub(crate) fn root_extensions(
        &self,
        root_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::ExtensionEntry> {
        let mut entries = Vec::new();

        // collect extension declarations from modules reached by the root symbol
        for profile in self.profiles() {
            for index in profile.module_indexes(profile.index().extensions.roots.get(&root_symbol))
            {
                entries.extend(index.extensions.root_entries(root_symbol).copied());
            }
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

        entries
    }

    /// Collect modules that may reference one target symbol.
    pub(crate) fn referencing_modules(&self, target_symbol: dir::GlobalSymbolId) -> Vec<ModuleId> {
        let mut module_ids = self
            .symbol_reference_entries(target_symbol)
            .into_iter()
            .map(|entry| entry.source.module_id)
            .collect::<Vec<_>>();

        module_ids.sort();
        module_ids.dedup();

        module_ids
    }

    /// Collect indexed references to one target symbol.
    pub(crate) fn symbol_reference_entries(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::ReferenceEntry> {
        let mut entries = self
            .symbol_program_references(target_symbol)
            .into_iter()
            .map(|reference| reference.entry)
            .collect::<Vec<_>>();

        Self::sort_reference_entries(&mut entries);
        entries.dedup();

        entries
    }

    /// Collect indexed program references to one target symbol.
    pub(crate) fn symbol_program_references(
        &self,
        target_symbol: dir::GlobalSymbolId,
    ) -> Vec<ProgramReference> {
        let mut references = Vec::new();

        // collect reference entries from modules reached by the target symbol
        for profile in self.profiles() {
            for index in
                profile.module_indexes(profile.index().references.targets.get(&target_symbol))
            {
                references.extend(index.references.target_entries(target_symbol).map(|entry| {
                    ProgramReference {
                        profile_id: profile.profile_id(),
                        entry: *entry,
                    }
                }));
            }
        }

        references.sort_by_key(|reference| {
            let entry = reference.entry;

            (
                reference.profile_id,
                entry.source.module_id,
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.source.local_id.id,
                entry.kind,
            )
        });
        references.dedup();

        references
    }

    /// Collect call candidates for one callee symbol.
    pub(crate) fn callee_calls(&self, callee_symbol: dir::GlobalSymbolId) -> Vec<dir::CallEntry> {
        let mut entries = Vec::new();

        // collect call edges from modules reached by the callee symbol
        for profile in self.profiles() {
            for index in profile.module_indexes(profile.index().calls.callees.get(&callee_symbol)) {
                entries.extend(index.calls.callee_entries(callee_symbol).copied());
            }
        }
        Self::sort_call_entries(&mut entries);

        entries
    }

    /// Collect call candidates for one caller symbol.
    pub(crate) fn caller_calls(&self, caller_symbol: dir::GlobalSymbolId) -> Vec<dir::CallEntry> {
        let mut entries = Vec::new();

        // collect call edges from modules reached by the caller symbol
        for profile in self.profiles() {
            for index in profile.module_indexes(profile.index().calls.callers.get(&caller_symbol)) {
                entries.extend(index.calls.caller_entries(caller_symbol).copied());
            }
        }
        Self::sort_call_entries(&mut entries);

        entries
    }

    /// Collect specifier candidates relevant to one set of renamed paths.
    pub(crate) fn renamed_specifiers<I>(
        &self,
        old_paths: I,
    ) -> Vec<(ProfileId, dir::SpecifierEntry)>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let old_paths = old_paths.into_iter().collect::<HashSet<_>>();
        let mut entries = Vec::new();

        // collect specifier rewrite candidates from modules reached by renamed paths
        for profile in self.profiles() {
            let mut modules = profile.index().specifiers.unresolved.clone();
            for old_path in &old_paths {
                modules.extend(profile.index().specifiers.paths.get(old_path));
            }
            modules.sort();
            modules.dedup();

            for index in profile.module_indexes(&modules) {
                entries.extend(
                    index
                        .specifiers
                        .renaming(&old_paths)
                        .map(|entry| (profile.profile_id(), entry.clone())),
                );
            }
        }

        entries.sort_by(|left, right| {
            (
                left.0,
                left.1.source.module_id,
                left.1.file.0,
                left.1.source.local_id.id,
                left.1.text.as_str(),
            )
                .cmp(&(
                    right.0,
                    right.1.source.module_id,
                    right.1.file.0,
                    right.1.source.local_id.id,
                    right.1.text.as_str(),
                ))
        });
        entries.dedup();

        entries
    }

    /// Sort and deduplicate exported symbol entries.
    fn sort_export_entries(entries: &mut Vec<dir::ExportEntry>) {
        entries.sort_by(|left, right| {
            (left.name.as_str(), left.source.module_id, left.symbol).cmp(&(
                right.name.as_str(),
                right.source.module_id,
                right.symbol,
            ))
        });
        entries.dedup();
    }

    /// Sort and deduplicate call entries.
    fn sort_call_entries(entries: &mut Vec<dir::CallEntry>) {
        entries.sort_by_key(|entry| {
            (
                entry.source.module_id,
                entry.caller,
                entry.callee,
                entry.source.local_id.id,
                entry.kind,
                entry.span.file,
                entry.span.start,
                entry.span.end,
            )
        });
        entries.dedup();
    }

    /// Sort reference entries.
    fn sort_reference_entries(entries: &mut [dir::ReferenceEntry]) {
        entries.sort_by_key(|entry| {
            (
                entry.source.module_id,
                entry.span.file,
                entry.span.start,
                entry.span.end,
                entry.source.local_id.id,
                entry.kind,
            )
        });
    }
}

impl ProgramQueryProfile {
    /// Return module ordinals whose string keys contain the query.
    fn modules_matching_name(&self, postings: &dir::Postings<String>, query: &str) -> Vec<u32> {
        let mut modules = Vec::new();

        for (index, key) in postings.keys.iter().enumerate() {
            if query.is_empty() || key.to_lowercase().contains(query) {
                modules.extend(postings.range(index));
            }
        }

        modules.sort();
        modules.dedup();

        modules
    }

    /// Return every module ordinal in this profile.
    fn all_modules(&self) -> Vec<u32> {
        (0..self.modules().len() as u32).collect()
    }

    /// Return module indexes for module ordinals.
    fn module_indexes<'a>(&'a self, modules: &'a [u32]) -> impl Iterator<Item = &'a ModuleIndex> {
        modules
            .iter()
            .map(|ordinal| {
                self.modules().get(*ordinal as usize).unwrap_or_else(|| {
                    panic!("program index module ordinal out of range: {ordinal}")
                })
            })
            .map(|index| index.as_ref())
    }
}
