use std::collections::HashMap;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use libtest_mimic::{Failed, Trial};
use tspp_query::QueryMethod;
use tspp_repository::TraceReport;

use crate::{QueryCase, QueryTrace, QueryWorkspace, parse_query_document};

/// Environment variable enabling exact response replacement.
const BLESS_ENV: &str = "TSPP_BLESS";
/// Environment variable selecting the number of slow artifacts in timing reports.
const TRACE_SLOW_ARTIFACTS_ENV: &str = "TSPP_TEST_TRACE_SLOW_ARTIFACTS";
/// Default number of slow artifacts in timing reports.
const DEFAULT_TRACE_SLOW_ARTIFACTS: usize = 8;

/// Query fixtures executed through one shared workspace.
#[derive(Debug)]
pub(super) struct QuerySuite {
    /// The workspace shared by isolated fixture revisions.
    workspace: QueryWorkspace,
    /// The parsed cases keyed by test name.
    cases: HashMap<String, QueryCase>,
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
    pub(super) fn load() -> Result<Arc<Self>, String> {
        let fixture_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixture");
        Self::require_method_files(&fixture_directory)?;
        let cases = Self::parse_cases(&fixture_directory)?;
        let workspace = QueryWorkspace::new("/query")?;

        Ok(Arc::new(Self {
            workspace,
            cases,
            is_blessing: Self::read_blessing(),
            response_deltas: Mutex::new(HashMap::new()),
        }))
    }

    /// Return whether mismatched response rows are replaced.
    pub(super) fn is_blessing(&self) -> bool {
        self.is_blessing
    }

    /// Build one test trial for every parsed fixture.
    pub(super) fn trials(self: &Arc<Self>) -> Vec<Trial> {
        let mut names = self.cases.keys().cloned().collect::<Vec<_>>();
        names.sort();

        names
            .into_iter()
            .map(|name| {
                let suite = self.clone();
                Trial::test(name.clone(), move || suite.run(&name).map_err(Failed::from))
            })
            .collect()
    }

    /// Parse every Markdown fixture into one exact index.
    fn parse_cases(directory: &Path) -> Result<HashMap<String, QueryCase>, String> {
        let mut paths = std::fs::read_dir(directory)
            .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
        paths
            .retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("md"));
        paths.sort();
        let mut cases = HashMap::new();

        // index every parsed test by its stable fixture name
        for path in paths {
            let markdown_cases = parse_query_document(&path)
                .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
            for markdown_case in markdown_cases {
                let case = QueryCase::parse(&path, markdown_case)
                    .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
                let name = case.test_name(directory)?;
                if cases.insert(name.clone(), case).is_some() {
                    return Err(format!("query fixture repeats test '{name}'"));
                }
            }
        }

        Ok(cases)
    }

    /// Execute and verify one named query fixture.
    fn run(&self, name: &str) -> Result<(), String> {
        let case = self
            .cases
            .get(name)
            .ok_or_else(|| format!("query fixture '{name}' is missing"))?;
        let result = case.run(&self.workspace, self.is_blessing)?;
        Self::print_traces(name, result.traces)?;

        self.bless(&case.document, result.response_updates)
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

    /// Print complete query operation traces as one report.
    fn print_traces(name: &str, traces: Vec<QueryTrace>) -> Result<(), String> {
        if traces.is_empty() {
            return Ok(());
        }
        let slow_attempts = match std::env::var(TRACE_SLOW_ARTIFACTS_ENV) {
            Ok(value) => value.parse::<usize>().map_err(|error| {
                format!("invalid {TRACE_SLOW_ARTIFACTS_ENV} value '{value}': {error}")
            })?,
            Err(std::env::VarError::NotPresent) => DEFAULT_TRACE_SLOW_ARTIFACTS,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(format!("{TRACE_SLOW_ARTIFACTS_ENV} is not valid UTF-8"));
            }
        };
        let mut report = TraceReport::new()
            .color()
            .timelines()
            .span_totals()
            .slow_attempts(slow_attempts);
        for trace in traces {
            report = report.row(trace.name, trace.trace);
        }
        let output = format!("\ntimings {name}\n{}", report.render());
        io::stdout()
            .lock()
            .write_all(output.as_bytes())
            .map_err(|error| format!("failed to print query timings: {error}"))
    }

    /// Read whether query response rows should be blessed.
    fn read_blessing() -> bool {
        std::env::var_os(BLESS_ENV).is_some_and(|value| !value.is_empty() && value != "0")
    }

    /// Require one fixture file for every registered query method.
    fn require_method_files(directory: &Path) -> Result<(), String> {
        let mut expected = QueryMethod::ALL
            .iter()
            .map(|method| format!("{}.md", method.name()))
            .collect::<Vec<_>>();
        expected.sort();
        let entries = std::fs::read_dir(directory)
            .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
        let mut actual = Vec::new();

        // require an exact one-to-one file mapping
        for entry in entries {
            let entry = entry
                .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?;
            let path = entry.path();
            if !entry
                .file_type()
                .map_err(|error| format!("failed to inspect '{}': {error}", path.display()))?
                .is_file()
            {
                return Err(format!(
                    "query fixture directory contains non-file '{}'",
                    path.display()
                ));
            }
            actual.push(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_| format!("query fixture path '{}' is not UTF-8", path.display()))?,
            );
        }
        actual.sort();
        if actual != expected {
            return Err(format!(
                "query fixture files differ from registered methods\nexpected: {expected:?}\nactual: {actual:?}"
            ));
        }

        Ok(())
    }
}
