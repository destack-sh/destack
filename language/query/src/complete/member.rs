use destack_dir as dir;
use rustc_hash::FxHashSet;

use super::builder::CompletionBuilder;
use crate::{CompletionCandidate, CompletionOrigin, QueryResult, SORT_BUILTIN};

impl CompletionBuilder<'_, '_, '_> {
    /// Complete members of a type after `.`.
    pub(super) fn complete_members(
        &self,
        type_id: dir::GlobalTypeId,
        is_optional: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();
        let mut seen = FxHashSet::default();

        // offer the receiver's apparent members in selection precedence
        let origin = self.module.module_id();
        for member in self
            .program
            .apparent_members(origin, type_id, is_optional)?
        {
            if !seen.insert(member.name.clone()) {
                continue;
            }
            let completion = CompletionCandidate::new(
                member.name,
                member.kind,
                CompletionOrigin::Member,
                SORT_BUILTIN,
            );
            let completion = match member.symbol {
                Some(symbol) => self.attach_symbol_completion(completion, symbol)?,
                None => completion,
            };
            results.push(completion);
        }

        Ok(results)
    }
}
