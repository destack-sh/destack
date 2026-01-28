// scoring tier base values, higher = better match
// each tier is separated by 200 to prevent overlap after length and spread penalties
const SCORE_EXACT_PREFIX: u32 = 2000;
const SCORE_CASE_INSENSITIVE_PREFIX: u32 = 1800;
const SCORE_WORD_BOUNDARY: u32 = 1600;
const SCORE_FUZZY_SUBSEQUENCE: u32 = 1400;
const SCORE_NO_FILTER: u32 = 1000;

/// Result of a fuzzy match.
#[derive(Debug, Clone)]
pub(crate) struct FuzzyMatch {
    /// Score for ranking (higher = better match).
    pub score: u32,
    /// Match tier for ordering.
    pub tier: MatchTier,
    /// Byte indices of matched characters in the candidate (for UI highlighting).
    pub matched_indices: Vec<usize>,
}

/// Match tiers for completion ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MatchTier {
    /// No filter applied.
    NoFilter,
    /// Fuzzy subsequence match.
    FuzzySubsequence,
    /// Word boundary match.
    WordBoundary,
    /// Case insensitive prefix match.
    CaseInsensitivePrefix,
    /// Exact prefix match.
    ExactPrefix,
}

/// Score a candidate against a typed prefix.
///
/// Returns `None` if no match, `Some(FuzzyMatch)` otherwise.
/// Higher score = better match.
/// `matched_indices` contains byte positions for UI highlighting.
pub(crate) fn score_completion(candidate: &str, prefix: &str) -> Option<FuzzyMatch> {
    // no filter: base score adjusted by length, shorter = better
    if prefix.is_empty() {
        return Some(FuzzyMatch {
            score: SCORE_NO_FILTER.saturating_sub(candidate.len() as u32),
            tier: MatchTier::NoFilter,
            matched_indices: Vec::new(),
        });
    }

    // exact prefix match, highest priority
    if candidate.starts_with(prefix) {
        let indices = prefix_indices(candidate, prefix.chars().count());
        return Some(FuzzyMatch {
            score: SCORE_EXACT_PREFIX.saturating_sub(candidate.len() as u32),
            tier: MatchTier::ExactPrefix,
            matched_indices: indices,
        });
    }

    // case insensitive prefix match
    if candidate.to_lowercase().starts_with(&prefix.to_lowercase()) {
        let indices = prefix_indices(candidate, prefix.chars().count());
        return Some(FuzzyMatch {
            score: SCORE_CASE_INSENSITIVE_PREFIX.saturating_sub(candidate.len() as u32),
            tier: MatchTier::CaseInsensitivePrefix,
            matched_indices: indices,
        });
    }

    // camelCase or snake_case word boundary match
    if let Some(indices) = word_boundary_match_indices(candidate, prefix) {
        return Some(FuzzyMatch {
            score: SCORE_WORD_BOUNDARY.saturating_sub(candidate.len() as u32),
            tier: MatchTier::WordBoundary,
            matched_indices: indices,
        });
    }

    // fuzzy substring match, all chars in order
    if let Some((spread, indices)) = fuzzy_match_with_indices(candidate, prefix) {
        // penalize by character spread, tighter matches score higher
        return Some(FuzzyMatch {
            score: SCORE_FUZZY_SUBSEQUENCE
                .saturating_sub(spread)
                .saturating_sub(candidate.len() as u32),
            tier: MatchTier::FuzzySubsequence,
            matched_indices: indices,
        });
    }

    None
}

/// Get byte indices for the first n characters of a string.
fn prefix_indices(s: &str, n: usize) -> Vec<usize> {
    s.char_indices().take(n).map(|(i, _)| i).collect()
}

/// Check if prefix matches word boundaries in candidate.
///
/// Word boundaries are: start of string, uppercase letters, after underscore.
/// Example: "gEA" matches "getElementsByAttribute" (g E A are at boundaries).
fn word_boundary_match_indices(candidate: &str, prefix: &str) -> Option<Vec<usize>> {
    // collect word boundary positions
    let boundaries: Vec<usize> = candidate
        .char_indices()
        .filter(|(i, c)| {
            *i == 0
                || c.is_uppercase()
                || (*i > 0 && candidate.as_bytes().get(i - 1) == Some(&b'_'))
        })
        .map(|(i, _)| i)
        .collect();

    // match prefix chars against boundary positions
    let prefix_chars: Vec<char> = prefix.chars().collect();
    let mut prefix_idx = 0;
    let mut matched_indices = Vec::new();

    // walk boundaries and match prefix chars in order
    for &boundary in &boundaries {
        if prefix_idx >= prefix_chars.len() {
            break;
        }

        let Some(candidate_char) = candidate.chars().nth(boundary) else {
            continue;
        };

        // case insensitive comparison
        if candidate_char.to_lowercase().next() == prefix_chars[prefix_idx].to_lowercase().next() {
            matched_indices.push(boundary);
            prefix_idx += 1;
        }
    }

    if prefix_idx == prefix_chars.len() {
        Some(matched_indices)
    } else {
        None
    }
}

/// Fuzzy match: check if all prefix chars appear in candidate in order.
///
/// Returns (spread, indices) where spread is the distance between first and last match.
fn fuzzy_match_with_indices(candidate: &str, prefix: &str) -> Option<(u32, Vec<usize>)> {
    // lowercase inputs for case insensitive matching
    let candidate_lower = candidate.to_lowercase();
    let prefix_lower = prefix.to_lowercase();

    // prepare iterators and match state
    let mut candidate_iter = candidate_lower.char_indices();
    let mut matched_indices = Vec::new();
    let mut first_pos: Option<usize> = None;
    let mut last_pos: usize = 0;

    for prefix_char in prefix_lower.chars() {
        // find next matching character
        let found = candidate_iter.find(|(_, c)| *c == prefix_char);

        match found {
            Some((pos, _)) => {
                if first_pos.is_none() {
                    first_pos = Some(pos);
                }
                last_pos = pos;
                matched_indices.push(pos);
            }
            None => return None,
        }
    }

    // compute spread between first and last matches
    let first = first_pos.unwrap_or(0);
    Some(((last_pos - first) as u32, matched_indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prefer exact prefix matches over lower tiers.
    #[test]
    fn test_exact_prefix_match() {
        let m = score_completion("toString", "to").unwrap();
        assert!(m.score > SCORE_CASE_INSENSITIVE_PREFIX);
        assert_eq!(m.matched_indices, vec![0, 1]);
    }

    /// Rank case sensitive prefixes above case insensitive matches.
    #[test]
    fn test_case_insensitive_prefix() {
        // compare exact and case insensitive matches
        let exact = score_completion("toString", "to").unwrap();
        let case_insensitive = score_completion("ToString", "to").unwrap();
        assert!(exact.score > case_insensitive.score);
        assert_eq!(case_insensitive.matched_indices, vec![0, 1]);
    }

    /// Allow matching on word boundary initials.
    #[test]
    fn test_word_boundary_match() {
        let m = score_completion("getElementsByAttribute", "gEA").unwrap();
        assert!(m.matched_indices.contains(&0));
    }

    /// Match across snake case boundaries.
    #[test]
    fn test_snake_case_boundaries() {
        let m = score_completion("get_element_by_id", "gebi").unwrap();
        assert!(!m.matched_indices.is_empty());
    }

    /// Support fuzzy substrings when prefixes are missing.
    #[test]
    fn test_fuzzy_substring() {
        let m = score_completion("completion", "cmpl").unwrap();
        assert_eq!(m.matched_indices.len(), 4);
        assert!(score_completion("completion", "nlpc").is_none());
    }

    /// Return none when there is no match.
    #[test]
    fn test_no_match() {
        assert!(score_completion("toString", "xyz").is_none());
        assert!(score_completion("foo", "bar").is_none());
    }

    /// Allow empty prefixes and return no match positions.
    #[test]
    fn test_empty_prefix() {
        let m = score_completion("anything", "").unwrap();
        assert!(m.matched_indices.is_empty());
    }

    /// Rank exact matches above boundary and fuzzy matches.
    #[test]
    fn test_score_ordering() {
        // compare score tiers across match types
        let exact = score_completion("toString", "to").unwrap();
        let boundary = score_completion("toString", "tS").unwrap();
        let fuzzy = score_completion("toString", "trg").unwrap();

        assert!(exact.score > boundary.score);
        assert!(boundary.score > fuzzy.score);
    }

    /// Record match positions for exact prefixes.
    #[test]
    fn test_match_positions_exact() {
        let m = score_completion("hello", "hel").unwrap();
        assert_eq!(m.matched_indices, vec![0, 1, 2]);
    }

    /// Record match positions for fuzzy matches.
    #[test]
    fn test_match_positions_fuzzy() {
        let m = score_completion("hello", "hlo").unwrap();
        assert_eq!(m.matched_indices, vec![0, 2, 4]);
    }
}
