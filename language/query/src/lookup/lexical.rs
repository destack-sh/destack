use std::cmp::Reverse;

const SCORE_EXACT_WHOLE: u32 = 9_000;
const SCORE_CASE_INSENSITIVE_WHOLE: u32 = 8_000;
const SCORE_EXACT_PREFIX: u32 = 7_000;
const SCORE_CASE_INSENSITIVE_PREFIX: u32 = 6_000;
const SCORE_EXACT_BOUNDARY: u32 = 5_000;
const SCORE_CASE_INSENSITIVE_BOUNDARY: u32 = 4_000;
const SCORE_SUBSEQUENCE: u32 = 3_000;
const SCORE_NO_FILTER: u32 = 1_000;

/// The lexical match kind for one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MatchKind {
    /// No lexical filter was applied.
    NoFilter,
    /// The query matched as one subsequence.
    Subsequence,
    /// The query matched word boundaries with case folded.
    CaseInsensitiveBoundary,
    /// The query matched word boundaries exactly.
    ExactBoundary,
    /// The query matched the prefix with case folded.
    CaseInsensitivePrefix,
    /// The query matched the prefix exactly.
    ExactPrefix,
    /// The query matched the whole candidate with case folded.
    CaseInsensitiveWhole,
    /// The query matched the whole candidate exactly.
    ExactWhole,
}

/// The lexical match quality for one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatchQuality {
    /// The match tier.
    pub(crate) kind: MatchKind,
    /// The ranking score within the tier.
    pub(crate) score: u32,
    /// The matched byte indices in the candidate.
    pub(crate) matched_indices: Vec<usize>,
}

/// The stable order for one lexical match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MatchOrder {
    /// The coarse lexical tier in descending quality order.
    pub(crate) kind: Reverse<MatchKind>,
    /// The within-tier score in descending quality order.
    pub(crate) score: Reverse<u32>,
}

/// One lexical query scored against one candidate.
struct LexicalQuery<'a> {
    /// The candidate text being scored.
    candidate: &'a str,
    /// The query text to match.
    query: &'a str,
    /// The lowercase query used by folded matches.
    query_lower: String,
}

impl MatchQuality {
    /// Return the stable order for this lexical match.
    pub(crate) fn order(&self) -> MatchOrder {
        MatchOrder {
            kind: Reverse(self.kind),
            score: Reverse(self.score),
        }
    }
}

impl<'a> LexicalQuery<'a> {
    /// Create one lexical query.
    fn new(candidate: &'a str, query: &'a str) -> Self {
        Self {
            candidate,
            query,
            query_lower: query.to_lowercase(),
        }
    }

    /// Score this lexical query.
    fn quality(&self) -> Option<MatchQuality> {
        // no filter
        if self.query.is_empty() {
            return Some(MatchQuality {
                kind: MatchKind::NoFilter,
                score: SCORE_NO_FILTER,
                matched_indices: Vec::new(),
            });
        }

        // exact whole
        if self.candidate == self.query {
            return Some(MatchQuality {
                kind: MatchKind::ExactWhole,
                score: SCORE_EXACT_WHOLE,
                matched_indices: self.prefix_indices(self.query.chars().count()),
            });
        }

        // case insensitive whole
        if self.candidate.to_lowercase() == self.query_lower {
            let mismatch_count = self.prefix_case_mismatch_count();

            return Some(MatchQuality {
                kind: MatchKind::CaseInsensitiveWhole,
                score: SCORE_CASE_INSENSITIVE_WHOLE
                    .saturating_sub(mismatch_count.saturating_mul(6)),
                matched_indices: self.prefix_indices(self.query.chars().count()),
            });
        }

        // exact prefix
        if self.candidate.starts_with(self.query) {
            return Some(MatchQuality {
                kind: MatchKind::ExactPrefix,
                score: SCORE_EXACT_PREFIX.saturating_sub(self.candidate.chars().count() as u32),
                matched_indices: self.prefix_indices(self.query.chars().count()),
            });
        }

        // case insensitive prefix
        if self.candidate.to_lowercase().starts_with(&self.query_lower) {
            let mismatch_count = self.prefix_case_mismatch_count();

            return Some(MatchQuality {
                kind: MatchKind::CaseInsensitivePrefix,
                score: SCORE_CASE_INSENSITIVE_PREFIX
                    .saturating_sub(self.candidate.chars().count() as u32)
                    .saturating_sub(mismatch_count.saturating_mul(6)),
                matched_indices: self.prefix_indices(self.query.chars().count()),
            });
        }

        // exact boundary
        if let Some(matched_indices) = self.boundary_match_indices(true) {
            return Some(MatchQuality {
                kind: MatchKind::ExactBoundary,
                score: SCORE_EXACT_BOUNDARY.saturating_sub(self.candidate.chars().count() as u32),
                matched_indices,
            });
        }

        // case insensitive boundary
        if let Some(matched_indices) = self.boundary_match_indices(false) {
            let mismatch_count = self.matched_case_mismatch_count(&matched_indices);

            return Some(MatchQuality {
                kind: MatchKind::CaseInsensitiveBoundary,
                score: SCORE_CASE_INSENSITIVE_BOUNDARY
                    .saturating_sub(self.candidate.chars().count() as u32)
                    .saturating_sub(mismatch_count.saturating_mul(6)),
                matched_indices,
            });
        }

        // subsequence
        if let Some(subsequence) = self.subsequence_match() {
            let mismatch_penalty = subsequence.case_mismatches.saturating_mul(2);

            return Some(MatchQuality {
                kind: MatchKind::Subsequence,
                score: SCORE_SUBSEQUENCE
                    .saturating_sub(self.candidate.chars().count() as u32)
                    .saturating_sub(subsequence.spread)
                    .saturating_sub(mismatch_penalty),
                matched_indices: subsequence.matched_indices,
            });
        }

        None
    }

    /// Return one case-mismatch count for the query prefix.
    fn prefix_case_mismatch_count(&self) -> u32 {
        self.candidate
            .chars()
            .zip(self.query.chars())
            .filter(|(candidate, query)| candidate != query)
            .count() as u32
    }

    /// Return the byte indices for the first `count` candidate characters.
    fn prefix_indices(&self, count: usize) -> Vec<usize> {
        self.candidate
            .char_indices()
            .take(count)
            .map(|(index, _)| index)
            .collect()
    }

    /// Return one boundary match when the query matches boundary characters.
    fn boundary_match_indices(&self, is_case_sensitive: bool) -> Option<Vec<usize>> {
        let prefix: Vec<char> = self.query.chars().collect();
        let mut prefix_index = 0;
        let mut matched_indices = Vec::new();

        for (index, character) in self.candidate.char_indices() {
            if prefix_index >= prefix.len() {
                break;
            }

            if !is_boundary(self.candidate, index, character) {
                continue;
            }

            let query_character = prefix[prefix_index];
            let is_match = if is_case_sensitive {
                character == query_character
            } else {
                chars_equal_fold(character, query_character)
            };

            if !is_match {
                continue;
            }

            matched_indices.push(index);
            prefix_index += 1;
        }

        (prefix_index == prefix.len()).then_some(matched_indices)
    }

    /// Return one case-mismatch count across matched byte indices.
    fn matched_case_mismatch_count(&self, matched_indices: &[usize]) -> u32 {
        self.query
            .chars()
            .zip(matched_indices.iter().copied())
            .filter_map(|(query_character, index)| {
                let candidate_character = self.candidate[index..].chars().next()?;
                Some((candidate_character, query_character))
            })
            .filter(|(candidate_character, query_character)| candidate_character != query_character)
            .count() as u32
    }

    /// Return one subsequence match result when every query character appears in order.
    fn subsequence_match(&self) -> Option<SubsequenceMatch> {
        let query: Vec<char> = self.query.chars().collect();
        let candidate: Vec<(usize, char)> = self.candidate.char_indices().collect();
        let mut matched_indices = Vec::new();
        let mut first_ordinal = None;
        let mut last_ordinal = 0usize;
        let mut case_mismatches = 0u32;
        let mut candidate_index = 0usize;

        for query_character in query {
            let mut found = None;

            while candidate_index < candidate.len() {
                let (byte_index, candidate_character) = candidate[candidate_index];
                candidate_index += 1;

                let is_match = chars_equal_fold(candidate_character, query_character);

                if !is_match {
                    continue;
                }

                found = Some((candidate_index - 1, byte_index, candidate_character));
                break;
            }

            let (ordinal, byte_index, candidate_character) = found?;

            if candidate_character != query_character {
                case_mismatches += 1;
            }

            if first_ordinal.is_none() {
                first_ordinal = Some(ordinal);
            }

            last_ordinal = ordinal;
            matched_indices.push(byte_index);
        }

        let spread = if let Some(first_ordinal) = first_ordinal {
            last_ordinal.saturating_sub(first_ordinal) as u32
        } else {
            0
        };

        Some(SubsequenceMatch {
            matched_indices,
            spread,
            case_mismatches,
        })
    }
}

/// Score one lexical query against one candidate name.
pub(crate) fn match_quality(candidate: &str, query: &str) -> Option<MatchQuality> {
    LexicalQuery::new(candidate, query).quality()
}

/// Return true when two characters match after lowercase folding.
fn chars_equal_fold(left: char, right: char) -> bool {
    let mut left = left.to_lowercase();
    let mut right = right.to_lowercase();

    loop {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) if left == right => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

/// Return true when one character starts one lexical boundary.
fn is_boundary(candidate: &str, index: usize, character: char) -> bool {
    if index == 0 {
        return true;
    }

    if character.is_uppercase() {
        return true;
    }

    let previous = candidate[..index].chars().next_back();
    let Some(previous) = previous else {
        return true;
    };

    previous == '_'
        || previous == '-'
        || (!previous.is_alphanumeric() && character.is_alphanumeric())
}

/// One subsequence match result.
struct SubsequenceMatch {
    /// The matched byte indices.
    matched_indices: Vec<usize>,
    /// The span between first and last matched character.
    spread: u32,
    /// The number of case-folded, not exact, matches.
    case_mismatches: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prefer exact prefixes over weaker lexical matches.
    #[test]
    fn test_prefers_exact_prefix_matches() {
        let exact = match_quality("toString", "to").unwrap();
        let boundary = match_quality("toString", "tS").unwrap();

        assert!(exact.kind > boundary.kind);
        assert!(exact.score > boundary.score);
    }

    /// Preserve case quality inside prefix matches.
    #[test]
    fn test_prefers_exact_case_prefix_matches() {
        let exact = match_quality("toString", "to").unwrap();
        let folded = match_quality("ToString", "to").unwrap();

        assert_eq!(exact.kind, MatchKind::ExactPrefix);
        assert_eq!(folded.kind, MatchKind::CaseInsensitivePrefix);
        assert!(exact.score > folded.score);
    }

    /// Prefer exact whole-name matches over longer prefixes.
    #[test]
    fn test_prefers_exact_whole_matches() {
        let exact = match_quality("format", "format").unwrap();
        let prefix = match_quality("formatName", "format").unwrap();

        assert_eq!(exact.kind, MatchKind::ExactWhole);
        assert_eq!(prefix.kind, MatchKind::ExactPrefix);
        assert!(exact.kind > prefix.kind);
        assert!(exact.score > prefix.score);
    }

    /// Prefer case-insensitive whole-name matches over longer prefixes.
    #[test]
    fn test_prefers_case_insensitive_whole_matches() {
        let exact = match_quality("Format", "format").unwrap();
        let prefix = match_quality("formatName", "format").unwrap();

        assert_eq!(exact.kind, MatchKind::CaseInsensitiveWhole);
        assert_eq!(prefix.kind, MatchKind::ExactPrefix);
        assert!(exact.kind > prefix.kind);
    }

    /// Preserve case quality inside boundary matches.
    #[test]
    fn test_prefers_exact_case_boundary_matches() {
        let exact = match_quality("HTTPServer", "HS").unwrap();
        let folded = match_quality("httpServer", "HS").unwrap();

        assert_eq!(exact.kind, MatchKind::ExactBoundary);
        assert_eq!(folded.kind, MatchKind::CaseInsensitiveBoundary);
        assert!(exact.score > folded.score);
    }

    /// Match camel case boundaries.
    #[test]
    fn test_matches_camel_case_boundaries() {
        let matched = match_quality("getElementsByAttribute", "gEA").unwrap();

        assert_eq!(matched.kind, MatchKind::ExactBoundary);
        assert_eq!(matched.matched_indices, vec![0, 3, 13]);
    }

    /// Match snake case boundaries.
    #[test]
    fn test_matches_snake_case_boundaries() {
        let matched = match_quality("get_element_by_id", "gebi").unwrap();

        assert_eq!(matched.kind, MatchKind::ExactBoundary);
        assert_eq!(matched.matched_indices, vec![0, 4, 12, 15]);
    }

    /// Preserve subsequence matches when prefixes are unavailable.
    #[test]
    fn test_matches_subsequences() {
        let matched = match_quality("completion", "cmpl").unwrap();

        assert_eq!(matched.kind, MatchKind::Subsequence);
        assert_eq!(matched.matched_indices, vec![0, 2, 3, 4]);
    }

    /// Reject unmatched lexical queries.
    #[test]
    fn test_rejects_non_matches() {
        assert!(match_quality("toString", "xyz").is_none());
    }

    /// Return empty match positions for empty queries.
    #[test]
    fn test_empty_query_has_no_positions() {
        let matched = match_quality("anything", "").unwrap();

        assert_eq!(matched.kind, MatchKind::NoFilter);
        assert!(matched.matched_indices.is_empty());
    }
}
