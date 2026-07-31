use destack_dir as dir;
use rustc_hash::FxHashSet;

use super::builder::CompletionBuilder;
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, QueryError, QueryResult,
    SORT_BUILTIN, SymbolUse,
};

impl CompletionBuilder<'_, '_, '_> {
    /// Complete members of a type after `.`.
    pub(super) fn complete_members(
        &self,
        type_id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        // FUGU #Incomplete: expose exact member lookup over checked DIR
        Err(QueryError::missing(format!(
            "member completion candidates: type={type_id:?}, optional={is_optional}"
        )))
    }

    /// Complete visible shorthand values inside an object literal.
    pub(super) fn complete_object_literal_shorthands(
        &self,
        existing_fields: &[String],
        scope: dir::LocalScope,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();
        let mut seen_names = FxHashSet::default();

        // collect visible values that can form shorthand fields
        let symbols = self.module.symbols();
        for visible in symbols
            .visible_bindings(scope)
            .filter(|binding| SymbolUse::Value.accepts_symbol_kind(binding.symbol.kind))
        {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            if existing_fields.contains(&name) {
                continue;
            }

            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.symbol_id,
            };
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Field,
                CompletionOrigin::Local,
                SORT_BUILTIN,
            );
            results.push(self.attach_symbol_completion(completion, symbol_id)?);
        }

        Ok(results)
    }
}
