use std::fmt::Write;

/// One generated conformance catalog row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceCatalogRow {
    /// The domain label.
    pub domain: String,
    /// The corpus label.
    pub corpus: String,
    /// The capability label.
    pub capability: String,
    /// The environment label.
    pub environment: String,
    /// The gate label.
    pub gate: String,
    /// The number of passed cases.
    pub passed: usize,
    /// The number of failed cases.
    pub failed: usize,
    /// The number of skipped cases.
    pub skipped: usize,
    /// The upstream reference string.
    pub upstream_ref: String,
}

impl ConformanceCatalogRow {
    /// Return the total number of accounted cases.
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }

    /// Return the pass rate percentage.
    pub fn rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }
}

/// Render one conformance catalog markdown table.
pub fn render_conformance_catalog_table(rows: &[ConformanceCatalogRow]) -> String {
    let mut output = String::new();

    // write the header
    output.push_str(
        "| Domain | Corpus | Capability | Env | Gate | Passed | Failed | Skipped | Total | Rate | Upstream Ref |\n",
    );
    output.push_str("| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |\n");

    // write each row
    for row in rows {
        let _ = writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2}% | {} |",
            row.domain,
            row.corpus,
            row.capability,
            row.environment,
            row.gate,
            row.passed,
            row.failed,
            row.skipped,
            row.total(),
            row.rate(),
            row.upstream_ref
        );
    }

    output
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

#[cfg(test)]
mod tests {
    use super::{
        ConformanceCatalogRow, render_conformance_catalog_table, replace_generated_section,
    };

    #[test]
    fn test_render_conformance_catalog_table() {
        let rows = vec![ConformanceCatalogRow {
            domain: "ecmascript".to_string(),
            corpus: "test262".to_string(),
            capability: "run".to_string(),
            environment: "hostless".to_string(),
            gate: "quick".to_string(),
            passed: 10,
            failed: 2,
            skipped: 1,
            upstream_ref: "deadbeef".to_string(),
        }];

        let table = render_conformance_catalog_table(&rows);
        assert!(table.contains("| ecmascript | test262 | run | hostless | quick | 10 | 2 | 1 | 13 | 76.92% | deadbeef |"));
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
