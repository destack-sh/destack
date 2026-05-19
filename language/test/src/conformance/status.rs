use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

use serde::{Deserialize, Serialize};

use super::{ConformanceCapability, ConformanceEnvironment};

/// The `status.json` file name.
pub const STATUS_JSON_FILE_NAME: &str = "status.json";

/// One case status value.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum CaseStatus {
    /// The case is maintained as one direct sound translation.
    Translated,
    /// The case is intentionally excluded from execution.
    Excluded,
    /// The case is maintained as one adapted native case.
    Adapted,
    /// The case is a known failure.
    KnownFail,
    /// The case is a known idempotence-only failure.
    KnownFailIdempotence,
    /// The case is blocked on environment availability.
    EnvBlocked,
    /// The case is flaky.
    Flaky,
    /// The case is manually managed.
    Manual,
}

impl CaseStatus {
    /// Return the stable case status identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Translated => "translated",
            Self::Excluded => "excluded",
            Self::Adapted => "adapted",
            Self::KnownFail => "known-fail",
            Self::KnownFailIdempotence => "known-fail-idempotence",
            Self::EnvBlocked => "env-blocked",
            Self::Flaky => "flaky",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for CaseStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One semantic source selector kind.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum SourceSelectorKind {
    /// The full source file is the logical test.
    File,
    /// The logical test is keyed by one explicit title.
    Title,
    /// The logical test is keyed by one explicit message string.
    Message,
    /// The logical test is keyed by one statement-group anchor.
    StatementGroup,
    /// The logical test is keyed by one loop or template item.
    TemplateItem,
    /// The logical test is keyed by one table row.
    TableRow,
}

impl SourceSelectorKind {
    /// Return the stable selector identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Title => "title",
            Self::Message => "message",
            Self::StatementGroup => "statement-group",
            Self::TemplateItem => "template-item",
            Self::TableRow => "table-row",
        }
    }
}

impl fmt::Display for SourceSelectorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One case status entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatusEntry {
    /// The case patterns or identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<String>,
    /// The source-relative file path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_file: String,
    /// The semantic source selector kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<SourceSelectorKind>,
    /// The semantic source selector key.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_key: String,
    /// The canonical hash of the extracted source fragment.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source_hash: String,
    /// The source-relative ordinal suffix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ordinal: Option<usize>,
    /// The target-relative file path.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_file: String,
    /// The target-relative subcase label.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_subcase: String,
    /// The target-relative ordinal suffix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ordinal: Option<usize>,
    /// The case status.
    pub status: CaseStatus,
    /// The reason for the status.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// One human note describing the status.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Optional capability filters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<ConformanceCapability>,
    /// Optional environment filters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<ConformanceEnvironment>,
}

impl StatusEntry {
    /// Return whether this entry uses pattern selectors.
    pub fn has_pattern_selectors(&self) -> bool {
        !self.patterns.is_empty()
    }

    /// Return the canonical source selector string for this entry.
    pub fn source_selector(&self) -> Option<String> {
        let source_kind = self.source_kind?;
        if self.source_file.is_empty() || self.source_key.is_empty() {
            return None;
        }

        let mut selector = self.source_file.clone();
        selector.push_str("::");
        selector.push_str(source_kind.as_str());
        selector.push('=');
        selector.push_str(&self.source_key);

        if let Some(source_ordinal) = self.source_ordinal {
            selector.push('#');
            selector.push_str(&source_ordinal.to_string());
        }

        Some(selector)
    }

    /// Return the canonical selectors for this entry.
    pub fn selectors(&self) -> Vec<String> {
        if !self.patterns.is_empty() {
            return self.patterns.clone();
        }

        if let Some(selector) =
            self.explicit_selector(&self.target_file, &self.target_subcase, self.target_ordinal)
        {
            return vec![selector];
        }

        Vec::new()
    }

    /// Return the canonical selector string for this entry.
    pub fn selector(&self) -> Option<String> {
        let mut selectors = self.selectors().into_iter();
        let first_selector = selectors.next()?;

        if selectors.next().is_some() {
            return None;
        }

        Some(first_selector)
    }

    /// Return whether this entry applies to one case under the provided filters.
    pub fn matches(
        &self,
        case_name: &str,
        capability: Option<ConformanceCapability>,
        environment: Option<ConformanceEnvironment>,
    ) -> bool {
        let selectors = self.selectors();
        if selectors.is_empty() {
            return false;
        }

        // reject mismatched case names first
        if !selectors
            .iter()
            .any(|selector| pattern_matches(selector, case_name))
        {
            return false;
        }

        // reject mismatched capabilities
        if !self.capabilities.is_empty()
            && capability.is_none_or(|capability| !self.capabilities.contains(&capability))
        {
            return false;
        }

        // reject mismatched environments
        if !self.environments.is_empty()
            && environment.is_none_or(|environment| !self.environments.contains(&environment))
        {
            return false;
        }

        true
    }

    /// Build one explicit selector from file and subcase parts.
    fn explicit_selector(
        &self,
        file: &str,
        subcase: &str,
        ordinal: Option<usize>,
    ) -> Option<String> {
        // reject entries without one concrete file selector
        if file.is_empty() {
            return None;
        }

        let mut selector = file.to_string();

        // append one subcase label when present
        if !subcase.is_empty() {
            selector.push_str("::");
            selector.push_str(subcase);

            // disambiguate repeated labels within one file
            if let Some(ordinal) = ordinal {
                selector.push('#');
                selector.push_str(&ordinal.to_string());
            }
        }

        Some(selector)
    }
}

/// One status file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StatusSet {
    /// The status entries in declaration order.
    #[serde(default)]
    pub entries: Vec<StatusEntry>,
}

impl StatusSet {
    /// Load one status file.
    pub fn load(path: &Path) -> Result<Self, String> {
        // read the status file
        let content = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        // parse the json payload
        serde_json::from_str(&content)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    /// Save one status file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        // serialize the json payload with stable formatting
        let content = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;

        // persist the status set
        fs::write(path, format!("{content}\n"))
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    /// Return whether the set has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the first matching status entry for one case under the provided filters.
    pub fn match_entry(
        &self,
        case_name: &str,
        capability: Option<ConformanceCapability>,
        environment: Option<ConformanceEnvironment>,
    ) -> Option<&StatusEntry> {
        self.entries
            .iter()
            .find(|entry| entry.matches(case_name, capability, environment))
    }

    /// Count entries by case status.
    pub fn status_counts(&self) -> BTreeMap<CaseStatus, usize> {
        let mut counts = BTreeMap::new();

        // count each declared status entry
        for entry in &self.entries {
            let count = counts.entry(entry.status).or_insert(0);
            *count += 1;
        }

        counts
    }

    /// Return all entries with one exact status.
    pub fn entries_with_status(&self, status: CaseStatus) -> impl Iterator<Item = &StatusEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.status == status)
    }

    /// Return one normalized status set with merged pattern entries.
    pub fn normalized(&self) -> Self {
        let mut groups = BTreeMap::<String, (StatusEntry, BTreeSet<String>)>::new();
        let mut order = Vec::<NormalizationItem>::new();

        // preserve explicit entries and merge pattern groups
        for entry in &self.entries {
            if !entry.has_pattern_selectors() {
                order.push(NormalizationItem::Entry(Box::new(entry.clone())));
                continue;
            }

            let mut key_entry = entry.clone();
            key_entry.patterns.clear();

            let key = serde_json::to_string(&key_entry)
                .expect("status entry normalization must serialize");

            if !groups.contains_key(&key) {
                order.push(NormalizationItem::PatternGroup(key.clone()));
                groups.insert(key.clone(), (key_entry, BTreeSet::new()));
            }

            let (_, selectors) = groups
                .get_mut(&key)
                .expect("status entry normalization group must exist");

            selectors.extend(entry.selectors());
        }

        let mut entries = Vec::with_capacity(order.len());

        // rebuild the normalized entry list in first occurrence order
        for item in order {
            match item {
                NormalizationItem::Entry(entry) => entries.push(*entry),
                NormalizationItem::PatternGroup(key) => {
                    let (mut entry, selectors) = groups
                        .remove(&key)
                        .expect("status entry normalization group must exist");

                    entry.patterns = selectors.into_iter().collect::<Vec<_>>();
                    entries.push(entry);
                }
            }
        }

        Self { entries }
    }
}

/// Return the `status.json` path for one suite directory.
pub fn status_json_path_for_dir(directory: &Path) -> PathBuf {
    directory.join(STATUS_JSON_FILE_NAME)
}

/// One normalized output item.
#[derive(Debug, Clone)]
enum NormalizationItem {
    /// One explicit entry kept in place.
    Entry(Box<StatusEntry>),
    /// One merged pattern group.
    PatternGroup(String),
}

/// Compress exact case selectors into safe wildcard selectors.
pub fn compress_exact_selectors(
    exact_selectors: &HashSet<String>,
    discovered_selectors: &HashSet<String>,
) -> Vec<String> {
    let mut root = SelectorDirectory::default();

    // build one trie over discovered selectors
    for selector in discovered_selectors {
        root.insert(selector, exact_selectors.contains(selector));
    }

    let mut selectors = Vec::new();
    root.collect_patterns(None, &mut selectors);
    selectors
}

/// Return whether one pattern matches one test name.
fn pattern_matches(pattern: &str, text: &str) -> bool {
    pattern_matches_bytes(pattern.as_bytes(), text.as_bytes())
}

/// One directory node inside the selector trie.
#[derive(Debug, Default)]
struct SelectorDirectory {
    /// The direct file selectors under this directory.
    direct_files: BTreeSet<String>,
    /// The direct failing file selectors under this directory.
    direct_failed_files: BTreeSet<String>,
    /// The child directories under this directory.
    children: BTreeMap<String, SelectorDirectory>,
    /// The total discovered selectors under this directory subtree.
    discovered_count: usize,
    /// The total failing selectors under this directory subtree.
    failed_count: usize,
}

impl SelectorDirectory {
    /// Insert one discovered selector into this directory trie.
    fn insert(&mut self, selector: &str, is_failed: bool) {
        self.discovered_count += 1;
        if is_failed {
            self.failed_count += 1;
        }

        let mut parts = selector.split('/').peekable();
        self.insert_parts(selector, &mut parts, is_failed);
    }

    /// Insert the remaining selector path parts.
    fn insert_parts<'a, I>(
        &mut self,
        selector: &str,
        parts: &mut std::iter::Peekable<I>,
        is_failed: bool,
    ) where
        I: Iterator<Item = &'a str>,
    {
        let Some(part) = parts.next() else {
            return;
        };

        // file leaf
        if parts.peek().is_none() {
            self.direct_files.insert(selector.to_string());
            if is_failed {
                self.direct_failed_files.insert(selector.to_string());
            }
            return;
        }

        // child directory
        let child = self.children.entry(part.to_string()).or_default();
        child.discovered_count += 1;
        if is_failed {
            child.failed_count += 1;
        }
        child.insert_parts(selector, parts, is_failed);
    }

    /// Collect compressed selectors for this subtree.
    fn collect_patterns(&self, prefix: Option<&str>, selectors: &mut Vec<String>) {
        // whole subtree
        if let Some(prefix) = prefix
            && self.failed_count > 1
            && self.failed_count == self.discovered_count
        {
            selectors.push(format!("{prefix}/**"));
            return;
        }

        // direct file layer
        if let Some(prefix) = prefix
            && self.direct_failed_files.len() > 1
            && self.direct_failed_files.len() == self.direct_files.len()
        {
            selectors.push(format!("{prefix}/*"));
        } else {
            selectors.extend(self.direct_failed_files.iter().cloned());
        }

        // child subtrees
        for (name, child) in &self.children {
            let child_prefix = match prefix {
                Some(prefix) => format!("{prefix}/{name}"),
                None => name.clone(),
            };
            child.collect_patterns(Some(&child_prefix), selectors);
        }
    }
}

/// Return whether one wildcard byte pattern matches one byte string.
fn pattern_matches_bytes(pattern: &[u8], text: &[u8]) -> bool {
    // finish once the pattern is exhausted
    if pattern.is_empty() {
        return text.is_empty();
    }

    // `**` matches any remainder, including path separators
    if let Some(rest) = pattern.strip_prefix(b"**") {
        if rest.is_empty() {
            return true;
        }

        // try the zero width match first
        if pattern_matches_bytes(rest, text) {
            return true;
        }

        // otherwise consume one byte at a time
        if let Some((_, tail)) = text.split_first() {
            return pattern_matches_bytes(pattern, tail);
        }

        return false;
    }

    // `*` matches within one path segment
    if let Some(rest) = pattern.strip_prefix(b"*") {
        if pattern_matches_bytes(rest, text) {
            return true;
        }

        if let Some((&byte, tail)) = text.split_first()
            && byte != b'/'
        {
            return pattern_matches_bytes(pattern, tail);
        }

        return false;
    }

    // literals must match exactly
    match (pattern.split_first(), text.split_first()) {
        (Some((&expected, pattern_tail)), Some((&actual, text_tail))) if expected == actual => {
            pattern_matches_bytes(pattern_tail, text_tail)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        CaseStatus, SourceSelectorKind, StatusEntry, StatusSet, compress_exact_selectors,
        pattern_matches,
    };
    use crate::conformance::{ConformanceCapability, ConformanceEnvironment};

    fn status_entry(status: CaseStatus) -> StatusEntry {
        StatusEntry {
            patterns: Vec::new(),
            source_file: String::new(),
            source_kind: None,
            source_key: String::new(),
            source_hash: String::new(),
            source_ordinal: None,
            target_file: String::new(),
            target_subcase: String::new(),
            target_ordinal: None,
            status,
            reason: String::new(),
            note: String::new(),
            capabilities: Vec::new(),
            environments: Vec::new(),
        }
    }

    #[test]
    fn test_pattern_matches_exact_names() {
        assert!(pattern_matches("path/to/case", "path/to/case"));
        assert!(!pattern_matches("path/to/case", "path/to/other"));
    }

    #[test]
    fn test_pattern_matches_single_segment_wildcards() {
        assert!(pattern_matches("path/*.js", "path/example.js"));
        assert!(!pattern_matches("path/*.js", "path/deep/example.js"));
    }

    #[test]
    fn test_pattern_matches_double_star_wildcards() {
        assert!(pattern_matches("path/**/case.js", "path/deep/tree/case.js"));
        assert!(pattern_matches("path/**", "path/deep/tree/case.js"));
    }

    #[test]
    fn test_match_entry_respects_filters() {
        let statuses = StatusSet {
            entries: vec![StatusEntry {
                patterns: vec!["tests/**".to_string()],
                reason: "requires runtime support".to_string(),
                capabilities: vec![ConformanceCapability::Run],
                environments: vec![ConformanceEnvironment::Hostless],
                ..status_entry(CaseStatus::KnownFail)
            }],
        };

        let entry = statuses.match_entry(
            "tests/number/basic.ds",
            Some(ConformanceCapability::Run),
            Some(ConformanceEnvironment::Hostless),
        );
        assert!(entry.is_some());

        let missing = statuses.match_entry(
            "tests/number/basic.ds",
            Some(ConformanceCapability::Parse),
            Some(ConformanceEnvironment::Hostless),
        );
        assert!(missing.is_none());
    }

    #[test]
    fn test_selector_uses_file_and_subcase_when_present() {
        let entry = StatusEntry {
            target_file: "api/headers/headers-basic.any.ts".to_string(),
            target_subcase: "Create headers with 1 should throw".to_string(),
            target_ordinal: Some(2),
            reason: "unsound".to_string(),
            ..status_entry(CaseStatus::Translated)
        };

        assert_eq!(
            entry.selector().as_deref(),
            Some("api/headers/headers-basic.any.ts::Create headers with 1 should throw#2")
        );
    }

    #[test]
    fn test_match_entry_accepts_structured_selectors() {
        let statuses = StatusSet {
            entries: vec![StatusEntry {
                target_file: "pseudo-tty/test-tty-isatty.ts".to_string(),
                target_subcase: "{} reported to be a tty, but it is not".to_string(),
                reason: "unsound".to_string(),
                ..status_entry(CaseStatus::Excluded)
            }],
        };

        let entry = statuses.match_entry(
            "pseudo-tty/test-tty-isatty.ts::{} reported to be a tty, but it is not",
            None,
            None,
        );

        assert!(entry.is_some());
    }

    #[test]
    fn test_selector_prefers_explicit_target_fields() {
        let entry = StatusEntry {
            source_file: "source.js".to_string(),
            source_kind: Some(SourceSelectorKind::Title),
            source_key: "source".to_string(),
            source_hash: "fnv1a64:1234".to_string(),
            target_file: "target.ts".to_string(),
            target_subcase: "target".to_string(),
            target_ordinal: Some(2),
            ..status_entry(CaseStatus::Translated)
        };

        assert_eq!(entry.selector().as_deref(), Some("target.ts::target#2"));
        assert_eq!(
            entry.source_selector().as_deref(),
            Some("source.js::title=source")
        );
    }

    #[test]
    fn test_match_entry_accepts_grouped_patterns() {
        let statuses = StatusSet {
            entries: vec![StatusEntry {
                patterns: vec!["pass*/alpha".to_string(), "fail/beta".to_string()],
                reason: "scope".to_string(),
                ..status_entry(CaseStatus::Excluded)
            }],
        };

        let alpha_entry = statuses.match_entry("pass-explicit/alpha", None, None);
        let beta_entry = statuses.match_entry("fail/beta", None, None);
        let missing = statuses.match_entry("pass/gamma", None, None);

        assert!(alpha_entry.is_some());
        assert!(beta_entry.is_some());
        assert!(missing.is_none());
    }

    #[test]
    fn test_compress_exact_selectors_uses_subtree_glob_when_safe() {
        let discovered = HashSet::from([
            "js/arrays/a.js".to_string(),
            "js/arrays/b.js".to_string(),
            "js/arrays/c.js".to_string(),
            "js/other/pass.js".to_string(),
        ]);
        let failures = HashSet::from([
            "js/arrays/a.js".to_string(),
            "js/arrays/b.js".to_string(),
            "js/arrays/c.js".to_string(),
        ]);

        let selectors = compress_exact_selectors(&failures, &discovered);
        assert_eq!(selectors, vec!["js/arrays/**".to_string()]);
    }

    #[test]
    fn test_compress_exact_selectors_uses_direct_file_glob_when_subtree_is_mixed() {
        let discovered = HashSet::from([
            "js/arrays/a.js".to_string(),
            "js/arrays/b.js".to_string(),
            "js/arrays/deep/pass.js".to_string(),
        ]);
        let failures = HashSet::from(["js/arrays/a.js".to_string(), "js/arrays/b.js".to_string()]);

        let selectors = compress_exact_selectors(&failures, &discovered);
        assert_eq!(selectors, vec!["js/arrays/*".to_string()]);
    }

    #[test]
    fn test_compress_exact_selectors_keeps_exact_names_when_glob_would_overmatch() {
        let discovered = HashSet::from([
            "js/arrays/a.js".to_string(),
            "js/arrays/b.js".to_string(),
            "js/arrays/c.js".to_string(),
        ]);
        let failures = HashSet::from(["js/arrays/a.js".to_string(), "js/arrays/c.js".to_string()]);

        let selectors = compress_exact_selectors(&failures, &discovered);
        assert_eq!(
            selectors,
            vec!["js/arrays/a.js".to_string(), "js/arrays/c.js".to_string()]
        );
    }

    #[test]
    fn test_normalized_merges_pattern_entries_with_same_metadata() {
        let statuses = StatusSet {
            entries: vec![
                StatusEntry {
                    patterns: vec!["js/a.js".to_string()],
                    reason: "auto".to_string(),
                    ..status_entry(CaseStatus::KnownFail)
                },
                StatusEntry {
                    patterns: vec!["js/b.js".to_string()],
                    reason: "auto".to_string(),
                    ..status_entry(CaseStatus::KnownFail)
                },
            ],
        };

        let normalized = statuses.normalized();
        assert_eq!(normalized.entries.len(), 1);
        assert_eq!(
            normalized.entries[0].patterns,
            vec!["js/a.js".to_string(), "js/b.js".to_string()]
        );
    }

    #[test]
    fn test_normalized_preserves_explicit_entries() {
        let statuses = StatusSet {
            entries: vec![
                StatusEntry {
                    patterns: vec!["js/a.js".to_string()],
                    reason: "auto".to_string(),
                    ..status_entry(CaseStatus::KnownFail)
                },
                StatusEntry {
                    source_file: "source.js".to_string(),
                    source_kind: Some(SourceSelectorKind::Title),
                    source_key: "alpha".to_string(),
                    source_hash: "fnv1a64:123".to_string(),
                    target_file: "target.ts".to_string(),
                    target_subcase: "alpha".to_string(),
                    ..status_entry(CaseStatus::Translated)
                },
                StatusEntry {
                    patterns: vec!["js/b.js".to_string()],
                    reason: "auto".to_string(),
                    ..status_entry(CaseStatus::KnownFail)
                },
            ],
        };

        let normalized = statuses.normalized();
        assert_eq!(normalized.entries.len(), 2);
        assert_eq!(
            normalized.entries[0].patterns,
            vec!["js/a.js".to_string(), "js/b.js".to_string()]
        );
        assert_eq!(normalized.entries[1].target_file, "target.ts");
    }
}
