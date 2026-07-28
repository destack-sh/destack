use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use destack_query::QueryMethodId;

use super::{QueryFixture, QueryWorkspace};
use crate::core::{
    Case, CaseResult, MarkdownSuiteIndex, RunContext, RunOptions, Suite, discover_markdown_suite,
    fixtures_dir,
};

/// Environment variable enabling exact response replacement.
const BLESS_ENV: &str = "DESTACK_BLESS";

/// Query fixtures executed through one shared workspace.
#[derive(Debug)]
pub struct QuerySuite {
    /// The workspace shared by isolated case revisions.
    workspace: QueryWorkspace,
    /// The parsed fixtures keyed by full case name.
    fixtures: HashMap<String, QueryFixture>,
    /// The discovered runnable cases.
    cases: Vec<Case>,
    /// Whether mismatched response rows should be replaced.
    is_blessing: bool,
    /// Earlier response length changes in each Markdown file.
    response_deltas: Mutex<HashMap<PathBuf, Vec<ResponseDelta>>>,
}

/// One canonical query response update.
#[derive(Debug)]
pub(super) struct ResponseUpdate {
    /// The response content range in the Markdown file.
    pub(super) content_range: Range<usize>,
    /// The original fenced response body.
    pub(super) original: String,
    /// The canonical response source returned by the query.
    pub(super) replacement: String,
}

/// One response length change at an original Markdown offset.
#[derive(Debug, Clone, Copy)]
struct ResponseDelta {
    /// The original response start.
    start: usize,
    /// The replacement length minus the original length.
    length_delta: isize,
}

impl QuerySuite {
    /// Load every query fixture.
    pub fn load() -> Result<Self, String> {
        let query_directory = fixtures_dir().join("query");
        require_method_files(&query_directory)?;
        let MarkdownSuiteIndex { cases, entries } =
            discover_markdown_suite(&query_directory, "destack_test::query", |path, markdown| {
                QueryFixture::parse(path, markdown).map(Some)
            })?;
        let workspace = QueryWorkspace::new("/query")?;

        Ok(Self {
            workspace,
            fixtures: entries,
            cases,
            is_blessing: is_blessing(),
            response_deltas: Mutex::new(HashMap::new()),
        })
    }

    /// Replace canonical response rows for one fixture case.
    fn bless(&self, path: &Path, mut updates: Vec<ResponseUpdate>) -> Result<(), String> {
        if updates.is_empty() {
            return Ok(());
        }
        updates.sort_by_key(|update| update.content_range.start);
        let mut deltas = self
            .response_deltas
            .lock()
            .map_err(|_| "query response blessing state is poisoned".to_string())?;
        let previous = deltas.entry(path.to_path_buf()).or_default();
        let mut staged = Vec::with_capacity(updates.len());
        let mut source = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
        let mut previous_end = None;

        // replace verified response contents in source order
        for update in updates {
            if previous_end.is_some_and(|end| update.content_range.start < end) {
                return Err(format!(
                    "query response updates overlap in '{}'",
                    path.display()
                ));
            }
            previous_end = Some(update.content_range.end);
            let shift = previous
                .iter()
                .chain(&staged)
                .filter(|delta| delta.start < update.content_range.start)
                .map(|delta| delta.length_delta)
                .sum::<isize>();
            let start = update
                .content_range
                .start
                .checked_add_signed(shift)
                .ok_or_else(|| "query response start overflowed".to_string())?;
            let end = update
                .content_range
                .end
                .checked_add_signed(shift)
                .ok_or_else(|| "query response end overflowed".to_string())?;
            let current = source.get(start..end).ok_or_else(|| {
                format!(
                    "query response range {start}..{end} is invalid in '{}'",
                    path.display()
                )
            })?;
            if current != update.original {
                return Err(format!(
                    "query response changed before blessing in '{}'",
                    path.display()
                ));
            }
            let replacement_length = update.replacement.len() as isize;
            let source_length = update.original.len() as isize;
            let length_delta = replacement_length - source_length;
            source.replace_range(start..end, &update.replacement);
            staged.push(ResponseDelta {
                start: update.content_range.start,
                length_delta,
            });
        }

        // publish the file and its committed offset changes together
        std::fs::write(path, source)
            .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;
        previous.extend(staged);

        Ok(())
    }
}

impl Suite for QuerySuite {
    /// Return the suite name.
    fn name(&self) -> &'static str {
        "query"
    }

    /// Return whether cases can execute in parallel.
    fn runs_in_parallel(&self) -> bool {
        !self.is_blessing
    }

    /// Return the discovered query cases.
    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    /// Run one query fixture.
    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let Some(fixture) = self.fixtures.get(&case.full_name()) else {
            return CaseResult::Failed {
                message: format!("query fixture '{}' is missing", case.full_name()),
            };
        };

        match fixture
            .run(&self.workspace, self.is_blessing)
            .and_then(|updates| self.bless(&case.path, updates))
        {
            Ok(()) => CaseResult::Passed,
            Err(message) => CaseResult::Failed { message },
        }
    }
}

/// Return whether query response rows should be blessed.
fn is_blessing() -> bool {
    std::env::var_os(BLESS_ENV).is_some_and(|value| !value.is_empty() && value != "0")
}

/// Require one fixture file for every registered query method.
fn require_method_files(directory: &Path) -> Result<(), String> {
    // collect the canonical method files
    let mut expected = QueryMethodId::ALL
        .iter()
        .map(|method| format!("{}.md", method.name()))
        .collect::<Vec<_>>();
    expected.sort();

    // collect the declared fixture files
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
    let mut actual = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "failed to inspect query fixture '{}': {error}",
                path.display()
            )
        })?;
        if !file_type.is_file() {
            return Err(format!(
                "query fixture directory contains non-file entry '{}'",
                path.display()
            ));
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| format!("query fixture path '{}' is not UTF-8", path.display()))?;
        actual.push(name);
    }
    actual.sort();

    // require an exact one-to-one registry mapping
    if actual != expected {
        return Err(format!(
            "query fixture files differ from registered methods\nexpected: {expected:?}\nactual: {actual:?}"
        ));
    }

    Ok(())
}
