use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::mdtest::{MdTestCase, discover_md_files, parse_mdtest_file, slug};

use super::{Case, CaseResult, save_expected_failures};

/// One Markdown-backed suite entry.
pub trait MarkdownSuiteEntry {
    /// Return the source section name.
    fn section(&self) -> &str;

    /// Return the source entry name.
    fn name(&self) -> &str;

    /// Return whether this entry should be skipped by default.
    fn is_skipped(&self) -> bool;
}

impl MarkdownSuiteEntry for MdTestCase {
    fn section(&self) -> &str {
        &self.section
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_skipped(&self) -> bool {
        self.skip
    }
}

/// One discovered Markdown suite index.
#[derive(Debug)]
pub struct MarkdownSuiteIndex<T> {
    /// The discovered runnable cases.
    pub cases: Vec<Case>,
    /// The parsed entries keyed by full case name.
    pub entries: HashMap<String, T>,
}

/// Discover one Markdown-backed suite.
pub fn discover_markdown_suite<T, F>(
    suite_dir: &Path,
    category: &str,
    mut convert: F,
) -> Result<MarkdownSuiteIndex<T>, String>
where
    T: MarkdownSuiteEntry,
    F: FnMut(&Path, MdTestCase) -> Result<Option<T>, String>,
{
    let mut cases = Vec::new();
    let mut entries = HashMap::new();

    // discover all Markdown fixture files up front
    let md_paths = discover_md_files(suite_dir).map_err(|error| {
        format!(
            "failed to discover markdown fixtures in {}: {error}",
            suite_dir.display()
        )
    })?;

    // parse each Markdown fixture into suite entries
    for md_path in md_paths {
        let parsed_cases = match parse_mdtest_file(&md_path) {
            Ok(parsed_cases) => parsed_cases,
            Err(error) => {
                return Err(format!("failed to parse {}: {error}", md_path.display()));
            }
        };

        let relative_path = md_path.strip_prefix(suite_dir).map_err(|error| {
            format!(
                "markdown fixture '{}' is outside '{}': {error}",
                md_path.display(),
                suite_dir.display()
            )
        })?;
        let relative_name = relative_path.to_str().ok_or_else(|| {
            format!(
                "markdown fixture path '{}' is not UTF-8",
                relative_path.display()
            )
        })?;

        // convert each Markdown case into one suite entry
        for parsed_case in parsed_cases {
            let entry = convert(relative_path, parsed_case)
                .map_err(|error| format!("failed to convert {}: {error}", md_path.display()))?;
            let Some(entry) = entry else {
                continue;
            };

            let name = format!(
                "{relative_name}/{}/{}",
                slug(entry.section()),
                slug(entry.name())
            );
            let case = Case::file(name, md_path.clone(), category).with_skipped(entry.is_skipped());

            if entries.insert(case.full_name(), entry).is_some() {
                return Err(format!(
                    "markdown fixture '{}' repeats case '{}'",
                    md_path.display(),
                    case.full_name()
                ));
            }
            cases.push(case);
        }
    }

    Ok(MarkdownSuiteIndex { cases, entries })
}

/// Return an expected failure set view when it is not empty.
pub fn expected_failures_view(expected_failures: &HashSet<String>) -> Option<&HashSet<String>> {
    // keep callers simple when there is no baseline
    if expected_failures.is_empty() {
        None
    } else {
        Some(expected_failures)
    }
}

/// Update one known failure baseline file from run results.
pub fn update_failure_baseline(
    expected_failures_path: &Path,
    results: &[(Case, CaseResult)],
) -> Result<usize, String> {
    let mut failures = HashSet::new();

    // collect the currently failing case names
    for (case, result) in results {
        if result.is_failed() {
            failures.insert(case.full_name());
        }
    }

    // rewrite the baseline file from the collected failures
    save_expected_failures(expected_failures_path, &failures).map_err(|error| {
        format!(
            "failed to update {}: {error}",
            expected_failures_path.display()
        )
    })?;

    Ok(failures.len())
}
