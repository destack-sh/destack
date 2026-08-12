use std::cmp::Ordering;

use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, MatchKind, MatchQuality,
    match_quality,
};

use super::{CompletionContext, CursorToken};

/// The contextual and lexical relevance for one completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionScore {
    /// The lexical match quality for the label.
    lexical: MatchQuality,
    /// The context-fit order bucket.
    context_order: u8,
    /// The origin order bucket.
    origin_order: u8,
    /// The item-kind order bucket for the active context.
    kind_order: u8,
    /// The deprecated order bucket.
    deprecated_order: u8,
}

/// The scorer for one completion query.
struct CompletionScorer<'a> {
    /// The completion context being scored.
    context: &'a CompletionContext,
    /// The typed lexical prefix.
    prefix: &'a str,
}

/// The scored state for one completion candidate.
struct ScoredCompletion {
    /// The original stable candidate order.
    stable_index: usize,
    /// The completion candidate.
    completion: CompletionCandidate,
    /// The derived lexical and contextual relevance.
    score: CompletionScore,
}

/// Completion-specific ordering for lexical match kinds.
trait CompletionMatchKind {
    /// Return the ordering bucket for this lexical match kind.
    fn completion_order(self) -> u8;
}

impl CompletionMatchKind for MatchKind {
    fn completion_order(self) -> u8 {
        match self {
            MatchKind::ExactWhole => 0,
            MatchKind::CaseInsensitiveWhole => 1,
            MatchKind::ExactPrefix => 2,
            MatchKind::CaseInsensitivePrefix => 3,
            MatchKind::ExactBoundary => 4,
            MatchKind::CaseInsensitiveBoundary => 5,
            MatchKind::Subsequence => 6,
            MatchKind::NoFilter => 7,
        }
    }
}

impl CompletionScore {
    /// Build the relevance for one completion candidate in one context.
    fn new(
        scorer: &CompletionScorer<'_>,
        completion: &CompletionCandidate,
        lexical: MatchQuality,
    ) -> Self {
        Self {
            lexical,
            context_order: scorer.context_order(completion),
            origin_order: scorer.origin_order(completion),
            kind_order: scorer.kind_order(completion),
            deprecated_order: u8::from(completion.is_deprecated),
        }
    }
}

impl CompletionScorer<'_> {
    /// Build one completion scorer.
    fn new<'a>(context: &'a CompletionContext, prefix: &'a str) -> CompletionScorer<'a> {
        CompletionScorer { context, prefix }
    }

    /// Score one completion candidate when it matches the typed prefix.
    fn score_completion(
        &self,
        stable_index: usize,
        completion: CompletionCandidate,
    ) -> Option<ScoredCompletion> {
        let lexical = match_quality(&completion.label, self.prefix)?;
        let score = CompletionScore::new(self, &completion, lexical);

        Some(ScoredCompletion {
            stable_index,
            completion,
            score,
        })
    }

    /// Compare two scored completion candidates.
    fn compare(&self, left: &ScoredCompletion, right: &ScoredCompletion) -> Ordering {
        left.score
            .context_order
            .cmp(&right.score.context_order)
            .then(left.score.origin_order.cmp(&right.score.origin_order))
            .then(left.score.kind_order.cmp(&right.score.kind_order))
            .then(
                left.score
                    .lexical
                    .kind
                    .completion_order()
                    .cmp(&right.score.lexical.kind.completion_order()),
            )
            .then_with(|| self.compare_auto_imports(left, right))
            .then(right.score.lexical.score.cmp(&left.score.lexical.score))
            .then(
                left.score
                    .deprecated_order
                    .cmp(&right.score.deprecated_order),
            )
            .then_with(|| self.compare_ordering_text(left, right))
            .then(left.stable_index.cmp(&right.stable_index))
    }

    /// Compare two auto-import candidates by import ordering.
    fn compare_auto_imports(&self, left: &ScoredCompletion, right: &ScoredCompletion) -> Ordering {
        let left_is_auto_import = left.completion.origin == CompletionOrigin::AutoImport;
        let right_is_auto_import = right.completion.origin == CompletionOrigin::AutoImport;

        if left_is_auto_import && right_is_auto_import {
            return left
                .completion
                .import_order
                .cmp(&right.completion.import_order);
        }

        Ordering::Equal
    }

    /// Compare two candidates by explicit ordering text.
    fn compare_ordering_text(&self, left: &ScoredCompletion, right: &ScoredCompletion) -> Ordering {
        let left_uses_text = self.uses_ordering_text(&left.completion);
        let right_uses_text = self.uses_ordering_text(&right.completion);

        if !self.prefix.is_empty() || (left_uses_text && right_uses_text) {
            return left
                .completion
                .ordering_text()
                .cmp(right.completion.ordering_text());
        }

        Ordering::Equal
    }

    /// Return whether this completion uses explicit ordering text.
    fn uses_ordering_text(&self, completion: &CompletionCandidate) -> bool {
        completion.origin == CompletionOrigin::AutoImport
            || matches!(
                completion.origin,
                CompletionOrigin::Builtin | CompletionOrigin::Keyword
            )
            || matches!(
                self.context,
                CompletionContext::ImportPath { .. } | CompletionContext::ImportClause { .. }
            )
    }

    /// Return the ordering bucket for this completion origin.
    fn origin_order(&self, completion: &CompletionCandidate) -> u8 {
        match completion.origin {
            CompletionOrigin::Contextual => 0,
            CompletionOrigin::Member => 1,
            CompletionOrigin::Local => 1,
            CompletionOrigin::Builtin => 2,
            CompletionOrigin::AutoImport => 3,
            CompletionOrigin::Keyword => 4,
        }
    }

    /// Return the context-fit order for this completion.
    fn context_order(&self, completion: &CompletionCandidate) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => u8::from(!completion.kind.is_type_like()),
            CompletionContext::NewExpression { .. } => {
                u8::from(!completion.kind.is_constructable())
            }
            CompletionContext::ObjectLiteralKey { .. } => {
                u8::from(completion.kind != CompletionItemKind::Field)
            }
            CompletionContext::CallArgument {
                expected_type: Some(expected_type),
                ..
            } => {
                // rank by exact type identity
                u8::from(completion.type_id != Some(*expected_type))
            }
            CompletionContext::MemberAccess { .. } => u8::from(!completion.kind.is_member_like()),
            CompletionContext::ImportPath { .. } => u8::from(!matches!(
                completion.kind,
                CompletionItemKind::Folder | CompletionItemKind::Module
            )),
            _ => 0,
        }
    }

    /// Return the item-kind order for the active context.
    fn kind_order(&self, completion: &CompletionCandidate) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => completion.kind.type_position_order(),
            CompletionContext::NewExpression { .. } => completion.kind.new_expression_order(),
            CompletionContext::ObjectLiteralKey { .. } => completion.kind.object_literal_order(),
            CompletionContext::ImportClause { .. } => completion.kind.import_clause_order(),
            CompletionContext::MemberAccess { .. } => completion.kind.member_access_order(),
            CompletionContext::ImportPath { .. } => completion.kind.import_path_order(),
            _ => completion.kind.value_position_order(),
        }
    }
}

impl CompletionItemKind {
    /// Return the item-kind order for type-position completions.
    fn type_position_order(self) -> u8 {
        match self {
            CompletionItemKind::AssociatedType
            | CompletionItemKind::Class
            | CompletionItemKind::Struct
            | CompletionItemKind::Interface
            | CompletionItemKind::NewtypeInterface
            | CompletionItemKind::Newtype
            | CompletionItemKind::TypeAlias
            | CompletionItemKind::Enum
            | CompletionItemKind::TypeParameter
            | CompletionItemKind::BuiltinType => 0,
            CompletionItemKind::Module | CompletionItemKind::Folder | CompletionItemKind::File => 1,
            CompletionItemKind::Keyword => 3,
            _ => 2,
        }
    }

    /// Return the item-kind order for new-expression completions.
    fn new_expression_order(self) -> u8 {
        match self {
            CompletionItemKind::Struct => 0,
            CompletionItemKind::Class => 1,
            CompletionItemKind::Function | CompletionItemKind::Constructor => 2,
            CompletionItemKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the item-kind order for object-literal completions.
    fn object_literal_order(self) -> u8 {
        match self {
            CompletionItemKind::Field => 0,
            CompletionItemKind::AssociatedConst
            | CompletionItemKind::Variable
            | CompletionItemKind::ValueParameter
            | CompletionItemKind::Constant
            | CompletionItemKind::Value => 1,
            CompletionItemKind::EnumMember => 2,
            CompletionItemKind::Method
            | CompletionItemKind::Function
            | CompletionItemKind::Constructor => 3,
            CompletionItemKind::Class | CompletionItemKind::Struct | CompletionItemKind::Enum => 4,
            CompletionItemKind::Keyword => 6,
            _ => 5,
        }
    }

    /// Return the item-kind order for member completions.
    fn member_access_order(self) -> u8 {
        match self {
            CompletionItemKind::Field
            | CompletionItemKind::Property
            | CompletionItemKind::AssociatedConst
            | CompletionItemKind::AssociatedType => 0,
            CompletionItemKind::Method
            | CompletionItemKind::Function
            | CompletionItemKind::Constructor => 1,
            CompletionItemKind::EnumMember => 2,
            CompletionItemKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the item-kind order for import-clause completions.
    fn import_clause_order(self) -> u8 {
        match self {
            CompletionItemKind::AssociatedType
            | CompletionItemKind::Class
            | CompletionItemKind::Struct
            | CompletionItemKind::Interface
            | CompletionItemKind::NewtypeInterface
            | CompletionItemKind::Newtype
            | CompletionItemKind::TypeAlias
            | CompletionItemKind::Enum
            | CompletionItemKind::TypeParameter
            | CompletionItemKind::BuiltinType => 0,
            CompletionItemKind::AssociatedConst
            | CompletionItemKind::Variable
            | CompletionItemKind::ValueParameter
            | CompletionItemKind::Constant
            | CompletionItemKind::Value => 1,
            CompletionItemKind::Method
            | CompletionItemKind::Function
            | CompletionItemKind::Constructor => 2,
            CompletionItemKind::Module | CompletionItemKind::Folder | CompletionItemKind::File => 3,
            CompletionItemKind::Keyword => 5,
            _ => 4,
        }
    }

    /// Return the item-kind order for import-path completions.
    fn import_path_order(self) -> u8 {
        match self {
            CompletionItemKind::Module => 0,
            CompletionItemKind::Folder => 1,
            _ => 2,
        }
    }

    /// Return the item-kind order for value-position completions.
    fn value_position_order(self) -> u8 {
        match self {
            CompletionItemKind::AssociatedConst
            | CompletionItemKind::Variable
            | CompletionItemKind::ValueParameter
            | CompletionItemKind::Constant
            | CompletionItemKind::Value => 0,
            CompletionItemKind::EnumMember => 1,
            CompletionItemKind::Field | CompletionItemKind::Property => 2,
            CompletionItemKind::Method
            | CompletionItemKind::Function
            | CompletionItemKind::Constructor => 3,
            CompletionItemKind::Class
            | CompletionItemKind::Struct
            | CompletionItemKind::Enum
            | CompletionItemKind::Newtype => 4,
            CompletionItemKind::AssociatedType
            | CompletionItemKind::Interface
            | CompletionItemKind::NewtypeInterface
            | CompletionItemKind::TypeAlias
            | CompletionItemKind::TypeParameter
            | CompletionItemKind::BuiltinType => 5,
            CompletionItemKind::Keyword => 7,
            _ => 6,
        }
    }

    /// Return true when this completion kind fits type positions well.
    fn is_type_like(self) -> bool {
        matches!(
            self,
            CompletionItemKind::AssociatedType
                | CompletionItemKind::Class
                | CompletionItemKind::Struct
                | CompletionItemKind::Interface
                | CompletionItemKind::NewtypeInterface
                | CompletionItemKind::Newtype
                | CompletionItemKind::TypeAlias
                | CompletionItemKind::Enum
                | CompletionItemKind::TypeParameter
                | CompletionItemKind::BuiltinType
        )
    }

    /// Return true when this completion kind fits member positions well.
    fn is_member_like(self) -> bool {
        matches!(
            self,
            CompletionItemKind::Method
                | CompletionItemKind::Function
                | CompletionItemKind::Constructor
                | CompletionItemKind::Field
                | CompletionItemKind::Property
                | CompletionItemKind::EnumMember
                | CompletionItemKind::AssociatedConst
                | CompletionItemKind::AssociatedType
        )
    }
}

/// Rank candidates by lexical and contextual relevance.
pub(crate) fn rank_completions(
    completions: Vec<CompletionCandidate>,
    context: &CompletionContext,
    token: Option<&CursorToken>,
) -> Vec<CompletionCandidate> {
    let prefix = token.map_or("", |token| token.text.as_str());
    let scorer = CompletionScorer::new(context, prefix);
    let mut scored: Vec<ScoredCompletion> = completions
        .into_iter()
        .enumerate()
        .filter_map(|(stable_index, completion)| scorer.score_completion(stable_index, completion))
        .collect();

    scored.sort_by(|left, right| scorer.compare(left, right));

    let has_unique_context_match = matches!(
        context,
        CompletionContext::CallArgument {
            expected_type: Some(_),
            ..
        }
    ) && scored
        .first()
        .is_some_and(|first| first.score.context_order == 0)
        && scored
            .get(1)
            .is_none_or(|second| second.score.context_order != 0);
    let mut results = scored
        .into_iter()
        .map(|scored| {
            let mut completion = scored.completion;
            let score = scored.score;
            completion.match_positions = score.lexical.matched_indices;
            completion
        })
        .collect::<Vec<_>>();

    let has_exact_label_match = !prefix.is_empty()
        && results
            .first()
            .is_some_and(|completion| completion.label == prefix);

    if (results.len() == 1 || has_exact_label_match || has_unique_context_match)
        && let Some(first) = results.first_mut()
    {
        first.preselect = true;
    }

    results
}
