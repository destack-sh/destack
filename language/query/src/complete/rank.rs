use std::cmp::Ordering;

use crate::{
    CompletionCandidate, CompletionCandidates, CompletionItemKind, CompletionOrigin, MatchQuality,
    match_quality,
};

use super::{CompletionContext, CursorToken};

/// The ranker for one completion query.
struct CompletionRanker<'a> {
    /// The completion context being ranked.
    context: &'a CompletionContext,
    /// The typed lexical prefix.
    prefix: &'a str,
}

/// One candidate matched against the typed prefix.
struct CompletionMatch {
    /// The original candidate ordinal.
    ordinal: usize,
    /// The completion candidate.
    candidate: CompletionCandidate,
    /// The lexical match quality against the typed prefix.
    lexical: MatchQuality,
}

impl CompletionRanker<'_> {
    /// Build one completion ranker.
    fn new<'a>(context: &'a CompletionContext, prefix: &'a str) -> CompletionRanker<'a> {
        CompletionRanker { context, prefix }
    }

    /// Match one completion candidate against the typed prefix.
    fn match_candidate(
        &self,
        ordinal: usize,
        candidate: CompletionCandidate,
    ) -> Option<CompletionMatch> {
        let lexical = match_quality(&candidate.label, self.prefix)?;

        Some(CompletionMatch {
            ordinal,
            candidate,
            lexical,
        })
    }

    /// Compare two matched completion candidates.
    fn compare(&self, left: &CompletionMatch, right: &CompletionMatch) -> Ordering {
        let expected_type = self.context.expected_type();
        let left_type_order =
            u8::from(expected_type.is_some() && left.candidate.type_id != expected_type);
        let right_type_order =
            u8::from(expected_type.is_some() && right.candidate.type_id != expected_type);

        // compare auto imports by their structured import order
        let left_is_auto_import = left.candidate.origin == CompletionOrigin::AutoImport;
        let right_is_auto_import = right.candidate.origin == CompletionOrigin::AutoImport;
        let import_order = if left_is_auto_import && right_is_auto_import {
            left.candidate
                .import_order
                .cmp(&right.candidate.import_order)
        } else {
            Ordering::Equal
        };

        left.lexical
            .order()
            .cmp(&right.lexical.order())
            .then(left_type_order.cmp(&right_type_order))
            .then(
                left.candidate
                    .origin
                    .order()
                    .cmp(&right.candidate.origin.order()),
            )
            .then(
                self.kind_order(&left.candidate)
                    .cmp(&self.kind_order(&right.candidate)),
            )
            .then(import_order)
            .then(
                left.candidate
                    .is_deprecated
                    .cmp(&right.candidate.is_deprecated),
            )
            .then_with(|| self.compare_ordering_text(left, right))
            .then(left.ordinal.cmp(&right.ordinal))
    }

    /// Return whether this candidate has the exact expected type.
    fn is_exact_type_match(&self, candidate: &CompletionCandidate) -> bool {
        let expected_type = self.context.expected_type();

        expected_type.is_some() && candidate.type_id == expected_type
    }

    /// Compare two candidates by explicit ordering text.
    fn compare_ordering_text(&self, left: &CompletionMatch, right: &CompletionMatch) -> Ordering {
        let left_uses_text = self.uses_ordering_text(&left.candidate);
        let right_uses_text = self.uses_ordering_text(&right.candidate);

        if !self.prefix.is_empty() || (left_uses_text && right_uses_text) {
            return left
                .candidate
                .ordering_text()
                .cmp(right.candidate.ordering_text());
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

    /// Return the item-kind order for the active context.
    fn kind_order(&self, completion: &CompletionCandidate) -> u8 {
        match self.context {
            CompletionContext::TypePosition { .. } => completion.kind.type_position_order(),
            CompletionContext::NewExpression { .. } => completion.kind.new_expression_order(),
            CompletionContext::ObjectLiteralKey { .. } => completion.kind.object_literal_order(),
            CompletionContext::ImportClause { .. } => completion.kind.import_clause_order(),
            CompletionContext::MemberAccess { .. } => completion.kind.member_access_order(),
            CompletionContext::ImportPath { .. } => completion.kind.import_path_order(),
            CompletionContext::ControlLabel { .. } => {
                u8::from(completion.kind != CompletionItemKind::Label)
            }
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
}

impl CompletionCandidates {
    /// Rank these candidates by lexical and contextual relevance.
    pub(crate) fn rank(self, context: &CompletionContext, token: Option<&CursorToken>) -> Self {
        let prefix = token.map_or("", |token| token.text.as_str());
        let ranker = CompletionRanker::new(context, prefix);
        let mut matches = self
            .items
            .into_iter()
            .enumerate()
            .filter_map(|(ordinal, candidate)| ranker.match_candidate(ordinal, candidate))
            .collect::<Vec<_>>();

        // rank every matching candidate
        matches.sort_by(|left, right| ranker.compare(left, right));
        let first_has_expected_type = matches
            .first()
            .is_some_and(|completion| ranker.is_exact_type_match(&completion.candidate));
        let mut items = matches
            .into_iter()
            .map(|completion| {
                let mut candidate = completion.candidate;
                candidate.match_positions = completion.lexical.matched_indices;

                candidate
            })
            .collect::<Vec<_>>();

        // preselect the first candidate when its type exactly matches the expectation
        if first_has_expected_type && let Some(first) = items.first_mut() {
            first.preselect = true;
        }

        Self {
            items,
            is_incomplete: self.is_incomplete,
        }
    }
}
