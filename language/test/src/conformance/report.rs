use std::fmt::Write;

use super::{CaseStatus, ConformanceCatalog};

/// One generated conformance catalog row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceCatalogRow {
    /// The domain label.
    pub domain: String,
    /// The suite label.
    pub suite: String,
    /// The human readable suite title.
    pub title: String,
    /// The joined status counts.
    pub statuses: String,
    /// The origin reference string.
    pub origin_ref: String,
}

/// Render one conformance catalog markdown table.
pub fn render_conformance_catalog_table(rows: &[ConformanceCatalogRow]) -> String {
    let mut output = String::new();

    // write the header
    output.push_str("| Domain | Suite | Title | Status | Origin Ref |\n");
    output.push_str("| --- | --- | --- | --- | --- |\n");

    // write each row
    for row in rows {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} |",
            row.domain, row.suite, row.title, row.statuses, row.origin_ref
        );
    }

    output
}

/// Build report rows from one catalog.
pub fn build_conformance_catalog_rows(catalog: &ConformanceCatalog) -> Vec<ConformanceCatalogRow> {
    let mut rows = Vec::with_capacity(catalog.suites.len());

    // flatten each suite into one rendered row
    for suite in &catalog.suites {
        let statuses = render_status_counts(&suite.statuses.status_counts());

        rows.push(ConformanceCatalogRow {
            domain: suite.suite.domain.to_string(),
            suite: suite.suite.suite.clone(),
            title: suite.suite.title.clone(),
            statuses,
            origin_ref: suite.suite.origin.ref_.clone(),
        });
    }

    rows
}

/// Replace one generated markdown section in a document.
pub fn replace_generated_section(
    document: &str,
    section: &str,
    body: &str,
) -> Result<String, String> {
    let begin_marker = format!("<!-- begin:{section} -->");
    let end_marker = format!("<!-- end:{section} -->");

    // locate the begin marker
    let Some(begin_index) = document.find(&begin_marker) else {
        return Err(format!("missing begin marker for section '{section}'"));
    };
    let body_start = begin_index + begin_marker.len();

    // locate the end marker
    let Some(end_index) = document.find(&end_marker) else {
        return Err(format!("missing end marker for section '{section}'"));
    };
    if end_index < body_start {
        return Err(format!(
            "end marker precedes begin marker for section '{section}'"
        ));
    }

    // splice the document with normalized body content
    let mut updated = String::with_capacity(document.len() + body.len());
    updated.push_str(&document[..body_start]);
    updated.push('\n');
    updated.push_str(body);
    if !body.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&document[end_index..]);

    Ok(updated)
}

/// Render one compact case status summary.
fn render_status_counts(counts: &std::collections::BTreeMap<CaseStatus, usize>) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }

    let labels = counts
        .iter()
        .map(|(status, count)| format!("{status} {count}"))
        .collect::<Vec<_>>();

    labels.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{
        ConformanceCatalogRow, render_conformance_catalog_table, replace_generated_section,
    };

    #[test]
    fn test_render_conformance_catalog_table() {
        let rows = vec![ConformanceCatalogRow {
            domain: "ecma".to_string(),
            suite: "test262".to_string(),
            title: "ECMA Test262".to_string(),
            statuses: "excluded 1, known-fail 2".to_string(),
            origin_ref: "deadbeef".to_string(),
        }];

        let table = render_conformance_catalog_table(&rows);
        let expected = "\
| Domain | Suite | Title | Status | Origin Ref |
| --- | --- | --- | --- | --- |
| ecma | test262 | ECMA Test262 | excluded 1, known-fail 2 | deadbeef |
";

        assert_eq!(table, expected);
    }

    #[test]
    fn test_replace_generated_section() {
        let document = "\
before
<!-- begin:summary -->
old
<!-- end:summary -->
after
";

        let updated = replace_generated_section(document, "summary", "new body").expect("section");
        let expected = "\
before
<!-- begin:summary -->
new body
<!-- end:summary -->
after
";

        assert_eq!(updated, expected);
    }
}
