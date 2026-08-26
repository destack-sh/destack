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
    /// The matched character positions in the candidate.
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

impl MatchOrder {
    /// Return the coarse lexical tier.
    pub(crate) fn kind(self) -> MatchKind {
        self.kind.0
    }
}

/// One lexical query scored against one candidate.
struct LexicalQuery<'a> {
    /// The candidate text being scored.
    candidate: &'a str,
    /// The query text to match.
    query: &'a str,
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
        Self { candidate, query }
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
        if self.is_equal_folded() {
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
        if self.starts_with_folded() {
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

    /// Return whether the candidate and query are equal after lowercase folding.
    fn is_equal_folded(&self) -> bool {
        let mut candidate = self.candidate.chars().flat_map(char::to_lowercase);
        let mut query = self.query.chars().flat_map(char::to_lowercase);

        // compare every folded character
        loop {
            match (candidate.next(), query.next()) {
                (Some(candidate), Some(query)) if candidate == query => {}
                (None, None) => return true,
                _ => return false,
            }
        }
    }

    /// Return whether the candidate starts with the query after lowercase folding.
    fn starts_with_folded(&self) -> bool {
        let mut candidate = self.candidate.chars().flat_map(char::to_lowercase);
        let query = self.query.chars().flat_map(char::to_lowercase);

        // compare every folded query character
        for expected in query {
            if candidate.next() != Some(expected) {
                return false;
            }
        }

        true
    }

    /// Return one case-mismatch count for the query prefix.
    fn prefix_case_mismatch_count(&self) -> u32 {
        self.candidate
            .chars()
            .zip(self.query.chars())
            .filter(|(candidate, query)| candidate != query)
            .count() as u32
    }

    /// Return the first `count` candidate character positions.
    fn prefix_indices(&self, count: usize) -> Vec<usize> {
        (0..count).collect()
    }

    /// Return one boundary match when the query matches boundary characters.
    fn boundary_match_indices(&self, is_case_sensitive: bool) -> Option<Vec<usize>> {
        let mut query = self.query.chars();
        let mut query_character = query.next();
        let mut matched_indices = Vec::new();

        // match query characters at successive candidate boundaries
        for (ordinal, (index, character)) in self.candidate.char_indices().enumerate() {
            let Some(expected) = query_character else {
                break;
            };

            if !self.is_boundary(index, character) {
                continue;
            }

            let is_match = if is_case_sensitive {
                character == expected
            } else {
                character.to_lowercase().eq(expected.to_lowercase())
            };

            if !is_match {
                continue;
            }

            matched_indices.push(ordinal);
            query_character = query.next();
        }

        query_character.is_none().then_some(matched_indices)
    }

    /// Return whether one character starts a lexical boundary.
    fn is_boundary(&self, index: usize, character: char) -> bool {
        if index == 0 || character.is_uppercase() {
            return true;
        }

        let Some(previous) = self.candidate[..index].chars().next_back() else {
            return true;
        };

        previous == '_'
            || previous == '-'
            || (!previous.is_alphanumeric() && character.is_alphanumeric())
    }

    /// Return one case-mismatch count across matched character positions.
    fn matched_case_mismatch_count(&self, matched_indices: &[usize]) -> u32 {
        let mut query = self.query.chars();
        let mut positions = matched_indices.iter().copied();
        let mut next_position = positions.next();
        let mut mismatches = 0;

        // compare only the matched candidate characters
        for (position, candidate) in self.candidate.chars().enumerate() {
            if next_position != Some(position) {
                continue;
            }
            let Some(expected) = query.next() else {
                break;
            };
            if candidate != expected {
                mismatches += 1;
            }
            next_position = positions.next();
        }

        mismatches
    }

    /// Return one subsequence match result when every query character appears in order.
    fn subsequence_match(&self) -> Option<SubsequenceMatch> {
        let mut matched_indices = Vec::new();
        let mut first_ordinal = None;
        let mut last_ordinal = 0usize;
        let mut case_mismatches = 0u32;
        let mut candidate = self.candidate.char_indices().enumerate();

        // match each query character against the remaining candidate
        for query_character in self.query.chars() {
            let mut found = None;
            for (ordinal, (_, candidate_character)) in candidate.by_ref() {
                if candidate_character
                    .to_lowercase()
                    .eq(query_character.to_lowercase())
                {
                    found = Some((ordinal, candidate_character));

                    break;
                }
            }

            let (ordinal, candidate_character) = found?;

            if candidate_character != query_character {
                case_mismatches += 1;
            }

            if first_ordinal.is_none() {
                first_ordinal = Some(ordinal);
            }

            last_ordinal = ordinal;
            matched_indices.push(ordinal);
        }

        let spread = if let Some(first_ordinal) = first_ordinal {
            (last_ordinal - first_ordinal) as u32
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

/// One subsequence match result.
struct SubsequenceMatch {
    /// The matched character positions.
    matched_indices: Vec<usize>,
    /// The span between first and last matched character.
    spread: u32,
    /// The number of case-folded, not exact, matches.
    case_mismatches: u32,
}
