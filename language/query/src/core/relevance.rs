#![allow(clippy::too_many_arguments)]

use std::cmp::Reverse;
use std::path::Path;

use destack_dir::SymbolSpace;
use destack_source::{FileId, ModuleId, PackageId, PathExt};
use destack_workspace::{Repository, Revision, SymbolIndexEntry, SymbolIndexKind};

use crate::core::path::{path_component_count, path_distance};
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
    pub kind: MatchKind,
    /// The ranking score within the tier.
    pub score: u32,
    /// The matched byte indices in the candidate.
    pub matched_indices: Vec<usize>,
}

/// The stable ordering key for one lexical match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MatchSortKey {
    /// The coarse lexical tier in descending quality order.
    kind: Reverse<MatchKind>,
    /// The within-tier score in descending quality order.
    score: Reverse<u32>,
}

/// The lexical and structural relevance for one workspace symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolRelevance {
    /// The lexical match quality for the symbol name.
    pub lexical: MatchQuality,
    /// The coarse symbol kind rank.
    pub kind_rank: u8,
    /// The container-name lexical match quality when available.
    pub container_lexical: Option<MatchQuality>,
    /// Whether this is a nested symbol rather than one top-level declaration.
    pub has_container: bool,
}

/// The lexical and path relevance for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportRelevance {
    /// The lexical match quality for the exported name.
    pub lexical: MatchQuality,
    /// The space preference rank.
    pub space_rank: u8,
    /// The same-directory preference rank.
    pub directory_rank: u8,
    /// The same-package preference rank.
    pub package_rank: u8,
    /// The relative path distance rank.
    pub distance_rank: u32,
    /// The relative depth difference rank.
    pub depth_rank: u32,
}

/// The stable structured ordering key for one import candidate.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ImportSortKey {
    /// The coarse space ordering rank.
    pub space_rank: u8,
    /// The same-directory ordering rank.
    pub directory_rank: u8,
    /// The same-package ordering rank.
    pub package_rank: u8,
    /// The path distance ordering rank.
    pub distance_rank: u32,
    /// The path depth ordering rank.
    pub depth_rank: u32,
    /// The display-path length ordering rank.
    pub display_path_length: usize,
    /// The display path.
    pub display_path: String,
    /// The export name.
    pub export_name: String,
}

impl MatchQuality {
    /// Return the stable ordering key for this lexical match.
    pub(crate) fn sort_key(&self) -> MatchSortKey {
        MatchSortKey {
            kind: Reverse(self.kind),
            score: Reverse(self.score),
        }
    }
}

/// Score one lexical query against one candidate name.
pub(crate) fn match_quality(candidate: &str, query: &str) -> Option<MatchQuality> {
    // no filter
    if query.is_empty() {
        return Some(MatchQuality {
            kind: MatchKind::NoFilter,
            score: SCORE_NO_FILTER,
            matched_indices: Vec::new(),
        });
    }

    let query_lower = lowercased(query);

    // exact whole
    if candidate == query {
        return Some(MatchQuality {
            kind: MatchKind::ExactWhole,
            score: SCORE_EXACT_WHOLE,
            matched_indices: prefix_indices(candidate, query.chars().count()),
        });
    }

    // case insensitive whole
    if lowercased(candidate) == query_lower {
        let mismatch_count = prefix_case_mismatch_count(candidate, query);

        return Some(MatchQuality {
            kind: MatchKind::CaseInsensitiveWhole,
            score: SCORE_CASE_INSENSITIVE_WHOLE.saturating_sub(mismatch_count.saturating_mul(6)),
            matched_indices: prefix_indices(candidate, query.chars().count()),
        });
    }

    // exact prefix
    if candidate.starts_with(query) {
        return Some(MatchQuality {
            kind: MatchKind::ExactPrefix,
            score: SCORE_EXACT_PREFIX.saturating_sub(candidate.chars().count() as u32),
            matched_indices: prefix_indices(candidate, query.chars().count()),
        });
    }

    // case insensitive prefix
    if lowercased(candidate).starts_with(&query_lower) {
        let mismatch_count = prefix_case_mismatch_count(candidate, query);

        return Some(MatchQuality {
            kind: MatchKind::CaseInsensitivePrefix,
            score: SCORE_CASE_INSENSITIVE_PREFIX
                .saturating_sub(candidate.chars().count() as u32)
                .saturating_sub(mismatch_count.saturating_mul(6)),
            matched_indices: prefix_indices(candidate, query.chars().count()),
        });
    }

    // exact boundary
    if let Some(matched_indices) = boundary_match_indices(candidate, query, true) {
        return Some(MatchQuality {
            kind: MatchKind::ExactBoundary,
            score: SCORE_EXACT_BOUNDARY.saturating_sub(candidate.chars().count() as u32),
            matched_indices,
        });
    }

    // case insensitive boundary
    if let Some(matched_indices) = boundary_match_indices(candidate, query, false) {
        let mismatch_count = matched_case_mismatch_count(candidate, query, &matched_indices);

        return Some(MatchQuality {
            kind: MatchKind::CaseInsensitiveBoundary,
            score: SCORE_CASE_INSENSITIVE_BOUNDARY
                .saturating_sub(candidate.chars().count() as u32)
                .saturating_sub(mismatch_count.saturating_mul(6)),
            matched_indices,
        });
    }

    // subsequence
    if let Some(subsequence) = subsequence_match(candidate, query) {
        let mismatch_penalty = subsequence.case_mismatches.saturating_mul(2);

        return Some(MatchQuality {
            kind: MatchKind::Subsequence,
            score: SCORE_SUBSEQUENCE
                .saturating_sub(candidate.chars().count() as u32)
                .saturating_sub(subsequence.spread)
                .saturating_sub(mismatch_penalty),
            matched_indices: subsequence.matched_indices,
        });
    }

    None
}

/// Compute workspace-symbol relevance for one indexed entry.
pub(crate) fn symbol_relevance(entry: &SymbolIndexEntry, query: &str) -> Option<SymbolRelevance> {
    let lexical = match_quality(&entry.name, query)?;
    let kind_rank = symbol_kind_rank(entry.kind);
    let container_lexical = entry
        .container_name
        .as_ref()
        .and_then(|container_name| match_quality(container_name, query));
    let has_container = entry.container_name.is_some();

    Some(SymbolRelevance {
        lexical,
        kind_rank,
        container_lexical,
        has_container,
    })
}

/// Compute import relevance for one candidate.
pub(crate) fn import_relevance(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    query: &str,
    export_name: &str,
    expected_space: Option<SymbolSpace>,
    space: SymbolSpace,
    module_id: ModuleId,
    module_path: &str,
) -> Option<ImportRelevance> {
    let lexical = match_quality(export_name, query)?;
    let path_relevance = import_path_relevance(
        repository,
        revision,
        file_id,
        current_package_id,
        module_id,
        module_path,
    );
    let space_rank = import_space_rank(space, expected_space);

    Some(ImportRelevance {
        lexical,
        space_rank,
        directory_rank: path_relevance.directory_rank,
        package_rank: path_relevance.package_rank,
        distance_rank: path_relevance.distance_rank,
        depth_rank: path_relevance.depth_rank,
    })
}

/// Build the stable sort text for one import candidate.
pub(crate) fn import_sort_text(
    relevance: &ImportRelevance,
    display_path: &str,
    export_name: &str,
) -> String {
    let sort_key = import_sort_key(relevance, display_path, export_name);

    format!(
        "{}:{}:{}:{:04}:{:04}:{:04}:{display_path}:{export_name}",
        sort_key.space_rank,
        sort_key.directory_rank,
        sort_key.package_rank,
        sort_key.distance_rank,
        sort_key.depth_rank,
        sort_key.display_path_length,
    )
}

/// Build the stable structured sort key for one import candidate.
pub(crate) fn import_sort_key(
    relevance: &ImportRelevance,
    display_path: &str,
    export_name: &str,
) -> ImportSortKey {
    ImportSortKey {
        space_rank: relevance.space_rank,
        directory_rank: relevance.directory_rank,
        package_rank: relevance.package_rank,
        distance_rank: relevance.distance_rank,
        depth_rank: relevance.depth_rank,
        display_path_length: display_path.chars().count(),
        display_path: display_path.to_string(),
        export_name: export_name.to_string(),
    }
}

/// Return one stable sort key for one workspace symbol.
pub(crate) fn workspace_symbol_sort_key(
    relevance: &SymbolRelevance,
    entry: &SymbolIndexEntry,
) -> (
    MatchSortKey,
    u8,
    u8,
    u8,
    Option<MatchSortKey>,
    Option<String>,
    usize,
    String,
    u128,
    u32,
    u32,
) {
    (
        relevance.lexical.sort_key(),
        relevance.kind_rank,
        u8::from(relevance.container_lexical.is_none()),
        u8::from(relevance.has_container),
        relevance
            .container_lexical
            .as_ref()
            .map(MatchQuality::sort_key),
        entry.container_name.clone(),
        entry.name.chars().count(),
        entry.name.to_lowercase(),
        entry.file_id.0,
        entry.range.start,
        entry.range.end,
    )
}

/// Return one coarse symbol kind rank for workspace-symbol ordering.
fn symbol_kind_rank(kind: SymbolIndexKind) -> u8 {
    match kind {
        SymbolIndexKind::Namespace => 0,
        SymbolIndexKind::Class
        | SymbolIndexKind::Struct
        | SymbolIndexKind::Interface
        | SymbolIndexKind::Enum => 1,
        SymbolIndexKind::Function => 2,
        SymbolIndexKind::Constant | SymbolIndexKind::Variable => 3,
        SymbolIndexKind::Method | SymbolIndexKind::Field | SymbolIndexKind::EnumMember => 4,
        SymbolIndexKind::TypeParameter => 5,
    }
}

/// Return one coarse import space rank.
fn import_space_rank(space: SymbolSpace, expected_space: Option<SymbolSpace>) -> u8 {
    match expected_space {
        Some(SymbolSpace::Type) => match space {
            SymbolSpace::Type => 0,
            SymbolSpace::TypeValue => 1,
            SymbolSpace::Value => 2,
            SymbolSpace::Label => 3,
        },
        _ => match space {
            SymbolSpace::Value => 0,
            SymbolSpace::TypeValue => 1,
            SymbolSpace::Type => 2,
            SymbolSpace::Label => 3,
        },
    }
}

/// The structural import-path relevance for one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImportPathRelevance {
    /// The same-directory preference rank.
    directory_rank: u8,
    /// The same-package preference rank.
    package_rank: u8,
    /// The relative path distance rank.
    distance_rank: u32,
    /// The relative depth difference rank.
    depth_rank: u32,
}

/// Compute one structural path relevance for an import candidate.
fn import_path_relevance(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    target_module_id: ModuleId,
    module_path: &str,
) -> ImportPathRelevance {
    let target_package_id = {
        let Some(module) = repository.module(revision, target_module_id).ok().flatten() else {
            return ImportPathRelevance {
                directory_rank: 2,
                package_rank: import_package_rank(current_package_id, None),
                distance_rank: u32::MAX,
                depth_rank: u32::MAX,
            };
        };
        module.package_id
    };

    let Some(source_file) = repository.file(revision, file_id).ok().flatten() else {
        return ImportPathRelevance {
            directory_rank: 2,
            package_rank: import_package_rank(current_package_id, Some(target_package_id)),
            distance_rank: u32::MAX,
            depth_rank: u32::MAX,
        };
    };
    let Some(source_path) = source_file.path.as_ref() else {
        return ImportPathRelevance {
            directory_rank: 2,
            package_rank: import_package_rank(current_package_id, Some(target_package_id)),
            distance_rank: u32::MAX,
            depth_rank: u32::MAX,
        };
    };

    let Some(source_dir) = source_path.parent() else {
        return ImportPathRelevance {
            directory_rank: 2,
            package_rank: import_package_rank(current_package_id, Some(target_package_id)),
            distance_rank: u32::MAX,
            depth_rank: u32::MAX,
        };
    };

    let source_dir = source_dir.normalize();
    let target_path = Path::new(module_path).normalize();
    let target_dir = target_path.parent().unwrap_or(&target_path).to_path_buf();

    ImportPathRelevance {
        directory_rank: u8::from(source_dir != target_dir),
        package_rank: import_package_rank(current_package_id, Some(target_package_id)),
        distance_rank: path_distance(&source_dir, &target_path),
        depth_rank: path_component_count(&target_dir)
            .saturating_sub(path_component_count(&source_dir)),
    }
}

/// Return one package preference rank for import ordering.
fn import_package_rank(
    current_package_id: Option<PackageId>,
    target_package_id: Option<PackageId>,
) -> u8 {
    match current_package_id {
        Some(current_package_id) if Some(current_package_id) == target_package_id => 0,
        Some(_) => 2,
        None => 1,
    }
}

/// Return lowercase text for lexical matching.
fn lowercased(text: &str) -> String {
    text.to_lowercase()
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

/// Return one case-mismatch count for one prefix.
fn prefix_case_mismatch_count(candidate: &str, query: &str) -> u32 {
    candidate
        .chars()
        .zip(query.chars())
        .filter(|(candidate, query)| candidate != query)
        .count() as u32
}

/// Return the byte indices for the first `count` characters.
fn prefix_indices(text: &str, count: usize) -> Vec<usize> {
    text.char_indices()
        .take(count)
        .map(|(index, _)| index)
        .collect()
}

/// Return one boundary match when one query matches boundary characters.
fn boundary_match_indices(
    candidate: &str,
    query: &str,
    is_case_sensitive: bool,
) -> Option<Vec<usize>> {
    let prefix: Vec<char> = query.chars().collect();
    let mut prefix_index = 0;
    let mut matched_indices = Vec::new();

    for (index, character) in candidate.char_indices() {
        if prefix_index >= prefix.len() {
            break;
        }

        if !is_boundary(candidate, index, character) {
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

/// Return one case-mismatch count across matched byte indices.
fn matched_case_mismatch_count(candidate: &str, query: &str, matched_indices: &[usize]) -> u32 {
    query
        .chars()
        .zip(matched_indices.iter().copied())
        .filter_map(|(query_character, index)| {
            let candidate_character = candidate[index..].chars().next()?;
            Some((candidate_character, query_character))
        })
        .filter(|(candidate_character, query_character)| candidate_character != query_character)
        .count() as u32
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

/// Return one subsequence match result when every query character appears in order.
fn subsequence_match(candidate: &str, query: &str) -> Option<SubsequenceMatch> {
    let query: Vec<char> = query.chars().collect();
    let candidate: Vec<(usize, char)> = candidate.char_indices().collect();
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

    let first_ordinal = first_ordinal.unwrap_or(0);
    let spread = last_ordinal.saturating_sub(first_ordinal) as u32;
    Some(SubsequenceMatch {
        matched_indices,
        spread,
        case_mismatches,
    })
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
