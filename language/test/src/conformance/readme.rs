use std::collections::BTreeMap;
use std::path::Path;

use crate::core::print::color;

use super::{CategoryStats, ConformanceSuiteResult};

/// Row data for one suite in the formatter conformance README.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadmeRow {
    /// The suite name.
    pub name: String,
    /// The passed case count.
    pub passed: usize,
    /// The failed case count.
    pub failed: usize,
    /// The skipped case count.
    pub skipped: usize,
    /// The total run case count.
    pub total: usize,
    /// The pass rate without skipped cases.
    pub rate: f64,
    /// The pass rate including skipped cases.
    pub rate_with_skipped: f64,
}

impl ReadmeRow {
    /// Build one row from one suite result.
    fn from_suite_result(result: &ConformanceSuiteResult) -> Self {
        Self {
            name: result.name.clone(),
            passed: result.result.passed,
            failed: result.result.failed + result.result.timedout,
            skipped: result.result.skipped,
            total: result.result.total_run(),
            rate: result.result.pass_rate(),
            rate_with_skipped: result.result.pass_rate_with_skipped(),
        }
    }

    /// Format one markdown table row.
    fn format(&self) -> String {
        let skipped = if self.skipped > 0 {
            format!("{:>5}", self.skipped)
        } else {
            "-".to_string()
        };

        format!(
            "| {:<8} | {:>5}  | {:>5}  | {:>5}  | {:>5} | {:>6.2}% | {:>6.2}% |",
            self.name,
            self.passed,
            self.failed,
            skipped,
            self.total,
            self.rate,
            self.rate_with_skipped
        )
    }
}

/// Parsed results from the formatter conformance README.
#[derive(Debug, Clone)]
pub struct ReadmeResults {
    /// The parsed table rows.
    pub rows: Vec<ReadmeRow>,
}

impl ReadmeResults {
    /// Parse one table row from the current README format.
    fn parse_row(line: &str) -> Result<Option<ReadmeRow>, String> {
        let line = line.trim();
        if !line.starts_with('|') || !line.ends_with('|') {
            return Ok(None);
        }

        let parts: Vec<&str> = line.split('|').map(|part| part.trim()).collect();
        if parts.len() != 9 {
            return Err(format!(
                "expected current conformance README row with 7 data columns, got {}: {line}",
                parts.len().saturating_sub(2)
            ));
        }

        let name = parts[1].to_lowercase();
        if name.is_empty()
            || name == "suite"
            || name.starts_with(':')
            || name.starts_with('-')
            || name == "total"
        {
            return Ok(None);
        }

        let passed = parts[2]
            .parse()
            .map_err(|_| format!("invalid passed count in README row: {line}"))?;
        let failed = parts[3]
            .parse()
            .map_err(|_| format!("invalid failed count in README row: {line}"))?;
        let skipped = parse_readme_skipped_cell(parts[4], line)?;
        let total = parts[5]
            .parse()
            .map_err(|_| format!("invalid total count in README row: {line}"))?;
        let rate = parse_readme_percent_cell(parts[6], line)?;
        let rate_with_skipped = parse_readme_percent_cell(parts[7], line)?;

        Ok(Some(ReadmeRow {
            name,
            passed,
            failed,
            skipped,
            total,
            rate,
            rate_with_skipped,
        }))
    }

    /// Parse the summary section from one README document.
    fn parse(content: &str) -> Result<Self, String> {
        let begin_marker = "<!-- begin:summary-results -->";
        let end_marker = "<!-- end:summary-results -->";

        let begin_index = content
            .find(begin_marker)
            .ok_or_else(|| "README.md is missing begin:summary-results marker".to_string())?;
        let end_index = content
            .find(end_marker)
            .ok_or_else(|| "README.md is missing end:summary-results marker".to_string())?;
        let section = &content[begin_index + begin_marker.len()..end_index];

        let mut rows = Vec::new();
        for line in section.lines() {
            if let Some(row) = Self::parse_row(line)? {
                rows.push(row);
            }
        }

        Ok(Self { rows })
    }

    /// Find one row by suite name.
    pub fn find(&self, name: &str) -> Option<&ReadmeRow> {
        self.rows.iter().find(|row| row.name == name.to_lowercase())
    }
}

/// Load the formatter conformance README baseline.
pub fn load_readme_baseline() -> Option<ReadmeResults> {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("formatter")
        .join("conformance")
        .join("README.md");

    let content = std::fs::read_to_string(&readme_path).ok()?;
    ReadmeResults::parse(&content).ok()
}

/// Update the formatter conformance README from one result set.
pub fn update_readme(results: &[ConformanceSuiteResult], is_partial: bool) -> bool {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("formatter")
        .join("conformance")
        .join("README.md");

    let content = match std::fs::read_to_string(&readme_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("{}: failed to read README.md: {error}", color::red("error"));
            return false;
        }
    };

    let old_results = match ReadmeResults::parse(&content) {
        Ok(results) => results,
        Err(error) => {
            eprintln!(
                "{}: failed to parse README.md: {error}",
                color::red("error")
            );
            return false;
        }
    };

    let filtered_suites: Vec<&ConformanceSuiteResult> = results
        .iter()
        .filter(|suite| suite.result.is_filtered())
        .collect();
    let eligible_results: Vec<&ConformanceSuiteResult> = results
        .iter()
        .filter(|suite| !suite.result.is_filtered())
        .collect();

    if eligible_results.is_empty() {
        if !filtered_suites.is_empty() {
            println!(
                "  {} README.md update skipped: all suites were filtered",
                color::yellow("warning")
            );
        }

        return false;
    }

    if !filtered_suites.is_empty() {
        let filtered_names = filtered_suites
            .iter()
            .map(|suite| suite.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  {} README.md skipping filtered suites: {filtered_names}",
            color::yellow("warning")
        );
    }

    let new_rows: Vec<ReadmeRow> = eligible_results
        .iter()
        .map(|suite| ReadmeRow::from_suite_result(suite))
        .collect();

    let mut final_rows = if is_partial {
        merge_partial_rows(&old_results, &new_rows)
    } else {
        new_rows.clone()
    };
    final_rows.sort_by(|left, right| left.name.cmp(&right.name));

    let deltas: Vec<ResultDelta> = new_rows
        .iter()
        .map(|new_row| ResultDelta::new(&new_row.name, old_results.find(&new_row.name), new_row))
        .collect();
    let any_changes = deltas.iter().any(ResultDelta::has_changes);

    let summary_section = format_results_section(&final_rows);
    let Some(mut new_content) = replace_section(&content, "summary-results", &summary_section)
    else {
        eprintln!(
            "{}: README.md missing summary-results markers",
            color::red("error")
        );
        return false;
    };

    // per suite categories
    for suite in eligible_results {
        if suite.result.categories.len() > 1 {
            let section_name = format!("{}-results", suite.name);
            let category_section = format_category_section(&suite.result.categories);

            if let Some(updated) = replace_section(&new_content, &section_name, &category_section) {
                new_content = updated;
            }
        }
    }

    if new_content == content {
        let total_passed: usize = final_rows.iter().map(|row| row.passed).sum();
        let total_total: usize = final_rows.iter().map(|row| row.total).sum();
        let total_rate = if total_total > 0 {
            total_passed as f64 / total_total as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "\n  {} README.md unchanged: {:.2}% ({}/{})",
            color::dim("(no changes)"),
            total_rate,
            total_passed,
            total_total
        );
        return false;
    }

    if let Err(error) = std::fs::write(&readme_path, &new_content) {
        eprintln!(
            "{}: failed to write README.md: {error}",
            color::red("error")
        );
        return false;
    }

    println!();
    if any_changes {
        println!("  {} README.md updated:", color::green("UPDATED"));
        for delta in &deltas {
            if delta.has_changes() {
                println!(
                    "    {}: passed {} | failed {} | rate {}",
                    color::cyan(&delta.name),
                    format_delta(delta.passed_delta),
                    format_delta(delta.failed_delta),
                    format_rate_delta(delta.rate_delta)
                );
            }
        }
    } else {
        println!(
            "  {} README.md updated (formatting only)",
            color::dim("UPDATED")
        );
    }

    true
}

#[derive(Debug)]
struct ResultDelta {
    name: String,
    passed_delta: i64,
    failed_delta: i64,
    total_delta: i64,
    rate_delta: f64,
}

impl ResultDelta {
    /// Build one delta between one old and new README row.
    fn new(name: &str, old: Option<&ReadmeRow>, new: &ReadmeRow) -> Self {
        match old {
            Some(old) => Self {
                name: name.to_string(),
                passed_delta: new.passed as i64 - old.passed as i64,
                failed_delta: new.failed as i64 - old.failed as i64,
                total_delta: new.total as i64 - old.total as i64,
                rate_delta: new.rate - old.rate,
            },
            None => Self {
                name: name.to_string(),
                passed_delta: new.passed as i64,
                failed_delta: new.failed as i64,
                total_delta: new.total as i64,
                rate_delta: new.rate,
            },
        }
    }

    /// Return whether the delta changed any main counters.
    fn has_changes(&self) -> bool {
        self.passed_delta != 0 || self.failed_delta != 0 || self.total_delta != 0
    }
}

/// Merge one partial update into the existing README rows.
fn merge_partial_rows(old_results: &ReadmeResults, new_rows: &[ReadmeRow]) -> Vec<ReadmeRow> {
    let mut all_names: Vec<&str> = old_results
        .rows
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    for row in new_rows {
        if !all_names.contains(&row.name.as_str()) {
            all_names.push(&row.name);
        }
    }

    all_names
        .into_iter()
        .filter_map(|name| {
            if let Some(new_row) = new_rows.iter().find(|row| row.name == name) {
                Some(new_row.clone())
            } else {
                old_results.find(name).cloned()
            }
        })
        .collect()
}

/// Replace one generated section in the README document.
fn replace_section(content: &str, section_name: &str, new_section: &str) -> Option<String> {
    let begin_marker = format!("<!-- begin:{section_name} -->");
    let end_marker = format!("<!-- end:{section_name} -->");

    let begin_index = content.find(&begin_marker)?;
    let end_index = content.find(&end_marker)?;

    Some(format!(
        "{}\n{}\n{}{}",
        &content[..begin_index + begin_marker.len()],
        new_section,
        end_marker,
        &content[end_index + end_marker.len()..]
    ))
}

/// Format one per-suite category section.
fn format_category_section(categories: &BTreeMap<String, CategoryStats>) -> String {
    // totals
    let total_passed: usize = categories.values().map(|stats| stats.passed).sum();
    let total_failed: usize = categories.values().map(|stats| stats.failed).sum();
    let total_skipped: usize = categories.values().map(|stats| stats.skipped).sum();
    let total_total: usize = categories.values().map(CategoryStats::total).sum();
    let total_rate = if total_total > 0 {
        total_passed as f64 / total_total as f64 * 100.0
    } else {
        0.0
    };
    let total_total_with_skipped = total_total + total_skipped;
    let total_rate_with_skipped = if total_total_with_skipped > 0 {
        total_passed as f64 / total_total_with_skipped as f64 * 100.0
    } else {
        0.0
    };

    // table header
    let mut lines = Vec::new();
    lines.push(
        "| Category             | Passed | Failed | Skipped | Total |  Rate   | Incl. Rate |"
            .to_string(),
    );
    lines.push(
        "|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|"
            .to_string(),
    );

    // rows
    for (category, stats) in categories {
        let skipped = if stats.skipped > 0 {
            stats.skipped.to_string()
        } else {
            "-".to_string()
        };

        lines.push(format!(
            "| {:<20} | {:>5}  | {:>5}  | {:>7}  | {:>5} | {:>6.2}% | {:>9.2}% |",
            category,
            stats.passed,
            stats.failed,
            skipped,
            stats.total(),
            stats.pass_rate(),
            stats.pass_rate_with_skipped()
        ));
    }

    // total row
    lines.push(
        "|----------------------|--------|--------|---------|-------|---------|------------|"
            .to_string(),
    );

    let skipped = if total_skipped > 0 {
        total_skipped.to_string()
    } else {
        "-".to_string()
    };

    lines.push(format!(
        "| {:<20} | {:>5}  | {:>5}  | {:>7}  | {:>5} | {:>6.2}% | {:>9.2}% |",
        "total",
        total_passed,
        total_failed,
        skipped,
        total_total,
        total_rate,
        total_rate_with_skipped
    ));

    lines.join("\n")
}

/// Format the main README results section.
fn format_results_section(rows: &[ReadmeRow]) -> String {
    // totals
    let total_passed: usize = rows.iter().map(|row| row.passed).sum();
    let total_failed: usize = rows.iter().map(|row| row.failed).sum();
    let total_skipped: usize = rows.iter().map(|row| row.skipped).sum();
    let total_total: usize = rows.iter().map(|row| row.total).sum();
    let total_rate = if total_total > 0 {
        total_passed as f64 / total_total as f64 * 100.0
    } else {
        0.0
    };
    let total_total_with_skipped = total_total + total_skipped;
    let total_rate_with_skipped = if total_total_with_skipped > 0 {
        total_passed as f64 / total_total_with_skipped as f64 * 100.0
    } else {
        0.0
    };

    // table header
    let mut lines = Vec::new();
    lines.push(
        "| Suite    | Passed | Failed | Skipped | Total |  Rate   | Incl. Rate |".to_string(),
    );
    lines.push(
        "|:---------|-------:|-------:|--------:|------:|--------:|-----------:|".to_string(),
    );

    // rows
    for row in rows {
        lines.push(row.format());
    }

    // total row
    lines.push(
        "|----------|--------|--------|---------|-------|---------|------------|".to_string(),
    );

    let skipped = if total_skipped > 0 {
        format!("{total_skipped:>5}")
    } else {
        "-".to_string()
    };

    lines.push(format!(
        "| {:<8} | {:>5}  | {:>5}  | {:>6}  | {:>5} | {:>6.2}% | {:>9.2}% |",
        "total",
        total_passed,
        total_failed,
        skipped,
        total_total,
        total_rate,
        total_rate_with_skipped
    ));
    lines.push(String::new());
    lines.push(format!(
        "Total Blended Pass Rate: **{total_rate:.2}%** ({total_rate_with_skipped:.2}% incl. skipped)"
    ));

    lines.join("\n")
}

/// Parse one skipped cell from the README table.
fn parse_readme_skipped_cell(value: &str, line: &str) -> Result<usize, String> {
    if value == "-" {
        return Ok(0);
    }

    value
        .parse()
        .map_err(|_| format!("invalid skipped count in README row: {line}"))
}

/// Parse one percent cell from the README table.
fn parse_readme_percent_cell(value: &str, line: &str) -> Result<f64, String> {
    value
        .trim_end_matches('%')
        .parse()
        .map_err(|_| format!("invalid percent in README row: {line}"))
}

/// Format one integer delta for terminal output.
fn format_delta(delta: i64) -> String {
    if delta > 0 {
        color::green(&format!("+{delta}"))
    } else if delta < 0 {
        color::red(&format!("{delta}"))
    } else {
        color::dim("±0")
    }
}

/// Format one percent delta for terminal output.
fn format_rate_delta(delta: f64) -> String {
    if delta > 0.005 {
        color::green(&format!("+{delta:.2}%"))
    } else if delta < -0.005 {
        color::red(&format!("{delta:.2}%"))
    } else {
        color::dim("±0.00%")
    }
}
