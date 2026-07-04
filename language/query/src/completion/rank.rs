use std::cmp::Ordering;

use crate::{MatchKind, MatchQuality, match_quality};

use super::{Completion, CompletionContext, CompletionKind, CompletionOrigin, CursorToken};

/// The semantic and lexical relevance for one completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CompletionScore {
    /// The lexical match quality for the label.
    lexical: MatchQuality,
    /// The context-fit order bucket.
    context_order: u8,
    /// The active-parameter name-fit order bucket.
    parameter_order: u8,
    /// The expected nominal type-fit order bucket.
    type_order: u8,
    /// The expected callable and constructable order bucket.
    callability_order: u8,
    /// The semantic origin order bucket.
    origin_order: u8,
    /// The context-shaped semantic order bucket.
    semantic_order: u8,
    /// The producer order bucket.
    producer_order: u32,
    /// The direct-member preference order bucket.
    member_order: u8,
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
    completion: Completion,
    /// The derived lexical and semantic relevance.
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
    fn new(scorer: &CompletionScorer<'_>, completion: &Completion, lexical: MatchQuality) -> Self {
        Self {
            lexical,
            context_order: scorer.context_order(completion),
            parameter_order: scorer.parameter_order(completion),
            type_order: scorer.type_order(completion),
            callability_order: scorer.callability_order(completion),
            origin_order: scorer.origin_order(completion),
            semantic_order: scorer.semantic_order(completion),
            producer_order: completion.sort_order,
            member_order: scorer.member_order(completion),
            deprecated_order: u8::from(completion.deprecated),
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
        completion: Completion,
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
            .then(left.score.parameter_order.cmp(&right.score.parameter_order))
            .then(left.score.type_order.cmp(&right.score.type_order))
            .then(
                left.score
                    .callability_order
                    .cmp(&right.score.callability_order),
            )
            .then(left.score.origin_order.cmp(&right.score.origin_order))
            .then(left.score.semantic_order.cmp(&right.score.semantic_order))
            .then(
                left.score
                    .lexical
                    .kind
                    .completion_order()
                    .cmp(&right.score.lexical.kind.completion_order()),
            )
            .then(left.score.producer_order.cmp(&right.score.producer_order))
            .then_with(|| self.compare_auto_imports(left, right))
            .then(right.score.lexical.score.cmp(&left.score.lexical.score))
            .then(left.score.member_order.cmp(&right.score.member_order))
            .then(
                left.score
                    .deprecated_order
                    .cmp(&right.score.deprecated_order),
            )
            .then_with(|| self.compare_sort_text(left, right))
            .then(left.stable_index.cmp(&right.stable_index))
            .then_with(|| {
                left.completion
                    .ordering_text()
                    .cmp(right.completion.ordering_text())
            })
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

    /// Compare two candidates by explicit sort-text tie breaks.
    fn compare_sort_text(&self, left: &ScoredCompletion, right: &ScoredCompletion) -> Ordering {
        let left_uses_text = self.prefers_sort_text_tiebreak(&left.completion);
        let right_uses_text = self.prefers_sort_text_tiebreak(&right.completion);

        if !self.prefix.is_empty() || (left_uses_text && right_uses_text) {
            return left
                .completion
                .ordering_text()
                .cmp(right.completion.ordering_text());
        }

        Ordering::Equal
    }

    /// Return whether this completion prefers sort-text tie breaking.
    fn prefers_sort_text_tiebreak(&self, completion: &Completion) -> bool {
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

    /// Return the active-parameter name-fit order for this completion.
    fn parameter_order(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 1;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 1;
        };
        let Some(expected_name) = expected_parameter.name.as_ref() else {
            return 1;
        };

        u8::from(!completion.label.eq_ignore_ascii_case(expected_name))
    }

    /// Return the expected-type fit order for this completion.
    fn type_order(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 0;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 0;
        };
        if expected_parameter.related_nominals.is_empty() {
            return 0;
        }

        let expected_nominal = expected_parameter.nominal_symbol;
        match completion.nominal_symbol {
            Some(nominal_symbol) if Some(nominal_symbol) == expected_nominal => 0,
            Some(_)
                if completion.related_nominals.iter().any(|nominal_symbol| {
                    expected_parameter.related_nominals.contains(nominal_symbol)
                }) =>
            {
                1
            }
            None => 2,
            Some(_) => 3,
        }
    }

    /// Return the expected-value-shape fit order for this completion.
    fn callability_order(&self, completion: &Completion) -> u8 {
        let CompletionContext::CallArgument {
            expected_parameter, ..
        } = self.context
        else {
            return 0;
        };
        let Some(expected_parameter) = expected_parameter.as_ref() else {
            return 0;
        };

        if expected_parameter.prefers_constructible {
            return u8::from(!completion.value_shape.is_constructable);
        }

        if expected_parameter.prefers_callable {
            return u8::from(!completion.value_shape.is_callable);
        }

        0
    }

    /// Return the ordering bucket for this completion origin.
    fn origin_order(&self, completion: &Completion) -> u8 {
        match completion.origin {
            CompletionOrigin::Contextual => 0,
            CompletionOrigin::Local => 1,
            CompletionOrigin::Builtin => 2,
            CompletionOrigin::AutoImport => 3,
            CompletionOrigin::Keyword => 4,
            CompletionOrigin::Base => 5,
        }
    }

    /// Return the member-source order for this completion.
    fn member_order(&self, completion: &Completion) -> u8 {
        if !matches!(self.context, CompletionContext::MemberAccess { .. }) {
            return 0;
        }

        u8::from(completion.is_extension_member)
    }

    /// Return the context-fit order for this completion.
    fn context_order(&self, completion: &Completion) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => u8::from(!completion.kind.is_type_like()),
            CompletionContext::NewExpression { .. } => {
                u8::from(!completion.kind.is_constructable())
            }
            CompletionContext::ObjectLiteral { .. } => {
                u8::from(completion.kind != CompletionKind::Field)
            }
            CompletionContext::MemberAccess { .. } => u8::from(!completion.kind.is_member_like()),
            CompletionContext::ImportPath { .. } => u8::from(!matches!(
                completion.kind,
                CompletionKind::Folder | CompletionKind::Module
            )),
            _ => 0,
        }
    }

    /// Return the context-shaped semantic order for this completion.
    fn semantic_order(&self, completion: &Completion) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => completion.kind.type_position_order(),
            CompletionContext::NewExpression { .. } => completion.kind.new_expression_order(),
            CompletionContext::ObjectLiteral { .. } => completion.kind.object_literal_order(),
            CompletionContext::ImportClause { .. } => completion.kind.import_clause_order(),
            CompletionContext::MemberAccess { .. } => completion.kind.member_access_order(),
            CompletionContext::ImportPath { .. } => completion.kind.import_path_order(),
            _ => completion.kind.value_position_order(),
        }
    }
}

impl CompletionKind {
    /// Return the semantic order for type-position completions.
    fn type_position_order(self) -> u8 {
        match self {
            CompletionKind::Class
            | CompletionKind::Struct
            | CompletionKind::Interface
            | CompletionKind::Enum
            | CompletionKind::TypeParameter => 0,
            CompletionKind::Module | CompletionKind::Folder | CompletionKind::File => 1,
            CompletionKind::Keyword => 3,
            _ => 2,
        }
    }

    /// Return the semantic order for new-expression completions.
    fn new_expression_order(self) -> u8 {
        match self {
            CompletionKind::Struct => 0,
            CompletionKind::Class => 1,
            CompletionKind::Function | CompletionKind::Constructor => 2,
            CompletionKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the semantic order for object-literal completions.
    fn object_literal_order(self) -> u8 {
        match self {
            CompletionKind::Field => 0,
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 1,
            CompletionKind::EnumMember => 2,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 3,
            CompletionKind::Class | CompletionKind::Struct | CompletionKind::Enum => 4,
            CompletionKind::Keyword => 6,
            _ => 5,
        }
    }

    /// Return the semantic order for member completions.
    fn member_access_order(self) -> u8 {
        match self {
            CompletionKind::Field | CompletionKind::Property => 0,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 1,
            CompletionKind::EnumMember => 2,
            CompletionKind::Keyword => 4,
            _ => 3,
        }
    }

    /// Return the semantic order for import-clause completions.
    fn import_clause_order(self) -> u8 {
        match self {
            CompletionKind::Class
            | CompletionKind::Struct
            | CompletionKind::Interface
            | CompletionKind::Enum
            | CompletionKind::TypeParameter => 0,
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 1,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 2,
            CompletionKind::Module | CompletionKind::Folder | CompletionKind::File => 3,
            CompletionKind::Keyword => 5,
            _ => 4,
        }
    }

    /// Return the semantic order for import-path completions.
    fn import_path_order(self) -> u8 {
        match self {
            CompletionKind::Module => 0,
            CompletionKind::Folder => 1,
            _ => 2,
        }
    }

    /// Return the semantic order for value-position completions.
    fn value_position_order(self) -> u8 {
        match self {
            CompletionKind::Variable | CompletionKind::Constant | CompletionKind::Value => 0,
            CompletionKind::EnumMember => 1,
            CompletionKind::Field | CompletionKind::Property => 2,
            CompletionKind::Method | CompletionKind::Function | CompletionKind::Constructor => 3,
            CompletionKind::Class | CompletionKind::Struct | CompletionKind::Enum => 4,
            CompletionKind::Interface | CompletionKind::TypeParameter => 5,
            CompletionKind::Keyword => 7,
            _ => 6,
        }
    }

    /// Return true when this completion kind fits type positions well.
    fn is_type_like(self) -> bool {
        matches!(
            self,
            CompletionKind::Class
                | CompletionKind::Struct
                | CompletionKind::Interface
                | CompletionKind::Enum
                | CompletionKind::TypeParameter
        )
    }

    /// Return true when this completion kind fits member positions well.
    fn is_member_like(self) -> bool {
        matches!(
            self,
            CompletionKind::Method
                | CompletionKind::Function
                | CompletionKind::Constructor
                | CompletionKind::Field
                | CompletionKind::Property
                | CompletionKind::EnumMember
        )
    }

    /// Return whether this completion kind is constructable with `new`.
    pub(crate) fn is_constructable(self) -> bool {
        matches!(self, CompletionKind::Class | CompletionKind::Struct)
    }
}

/// Filter completions using lexical and semantic relevance.
pub(crate) fn filter_completions(
    completions: Vec<Completion>,
    context: &CompletionContext,
    token: Option<&CursorToken>,
) -> Vec<Completion> {
    let prefix = token.map_or("", |token| token.text.as_str());
    let scorer = CompletionScorer::new(context, prefix);
    let mut scored: Vec<ScoredCompletion> = completions
        .into_iter()
        .enumerate()
        .filter_map(|(stable_index, completion)| scorer.score_completion(stable_index, completion))
        .collect();

    scored.sort_by(|left, right| scorer.compare(left, right));

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

    if results.len() == 1 || has_exact_label_match {
        if let Some(first) = results.first_mut() {
            first.preselect = true;
        }
    }

    results
}
