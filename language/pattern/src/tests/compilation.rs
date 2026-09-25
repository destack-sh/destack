use std::fmt::Write;
use std::sync::Arc;

use tspp_core::StringPool;
use tspp_source::{DiagnosticCollection, File, Span};

use super::{fixture_text, render_diagnostics};
use crate::{Metavariable, MetavariableUse, Pattern, Predicate};

/// One completed pattern compilation under test.
pub(crate) struct TestCompilation {
    /// The strings containing pattern names.
    strings: Arc<StringPool>,
    /// The authored files available to assertions.
    files: Vec<Arc<File>>,
    /// The compiled pattern or its diagnostics.
    result: Result<Pattern, DiagnosticCollection>,
}

impl TestCompilation {
    /// Create one completed pattern compilation.
    pub(crate) fn new(
        strings: Arc<StringPool>,
        files: Vec<Arc<File>>,
        result: Result<Pattern, DiagnosticCollection>,
    ) -> Self {
        Self {
            strings,
            files,
            result,
        }
    }

    /// Assert the complete source-oriented pattern snapshot.
    #[track_caller]
    pub(crate) fn assert(self, expected: &str) {
        let pattern = self.result.as_ref().expect("pattern should compile");
        let actual = self.snapshot(pattern);
        let expected = fixture_text(expected).trim_end_matches('\n');

        assert_eq!(actual, expected);
    }

    /// Assert the complete rendered compilation diagnostics.
    #[track_caller]
    pub(crate) fn assert_diagnostics(self, expected: &str) {
        let diagnostics = self.result.expect_err("pattern should fail");
        let actual = render_diagnostics(&self.files, &diagnostics);

        assert_eq!(actual.trim_matches('\n'), expected.trim_matches('\n'));
    }

    /// Render the structural pattern and predicates with tripleslash rows.
    fn snapshot(&self, pattern: &Pattern) -> String {
        let mut output = self.structural_snapshot(pattern);

        // render each predicate beside its authored source
        for (index, predicate) in pattern.predicates().iter().enumerate() {
            output.push_str("\n\n");
            output.push_str(&self.predicate_snapshot(pattern, predicate, index));
        }

        output
    }

    /// Render the structural pattern with compiled model rows.
    fn structural_snapshot(&self, pattern: &Pattern) -> String {
        let fragment = pattern
            .fragments()
            .first()
            .expect("compiled structural fragment");
        let root = fragment.root();
        let root_span = fragment.span();
        let root_source = self.files[0].get_span_str(root_span).expect("root source");
        let mut rows = vec![SnapshotRow::new(
            root_span,
            format!(
                "/// @pattern.root node={:?} source={:?}",
                root.ty, root_source
            ),
        )];

        // render declarations at their first structural use
        for (variable_id, variable) in pattern.metavariables().iter() {
            let use_entry = fragment
                .uses()
                .iter()
                .find(|use_entry| use_entry.variable() == Some(variable_id))
                .expect("metavariable declaration should have one structural use");
            let name = self.strings.get(variable.name());
            let value = Self::metavariable_fields(*variable);
            rows.push(SnapshotRow::new(
                use_entry.span(),
                format!("/// @pattern.metavariable name={name} {value}"),
            ));
        }

        // render every structural use in authored order
        for use_entry in fragment.uses().iter() {
            let name = self.use_name(pattern, use_entry);
            let value = Self::use_fields(*use_entry);
            rows.push(SnapshotRow::new(
                use_entry.span(),
                format!("/// @pattern.use name={name} {value}"),
            ));
        }

        Self::render_rows(self.files[0].text(), rows)
    }

    /// Render one predicate with its metavariable reads.
    fn predicate_snapshot(&self, pattern: &Pattern, predicate: &Predicate, index: usize) -> String {
        let file = &self.files[index + 1];
        let root = predicate.root();
        let root_span = predicate.tree().get_span(root);
        let root_source = file.get_span_str(root_span).expect("predicate root source");
        let mut rows = vec![SnapshotRow::new(
            root_span,
            format!("/// @predicate.root node=Expression source={root_source:?}"),
        )];

        // render every predicate read in authored order
        for use_entry in predicate.uses().iter() {
            let variable = pattern.metavariables().get(use_entry.variable);
            let name = self.strings.get(variable.name());
            let span = predicate.tree().get_span(use_entry.expression);
            rows.push(SnapshotRow::new(
                span,
                format!("/// @predicate.use name={name} node=Expression"),
            ));
        }

        Self::render_rows(file.text(), rows)
    }

    /// Overlay tripleslash rows after their source lines.
    fn render_rows(source: &str, mut rows: Vec<SnapshotRow>) -> String {
        rows.sort_by_key(|row| row.span.start);
        let source = source.trim_matches('\n');
        let mut output = String::new();
        let mut row_index = 0;
        let mut line_start = 0;

        // copy each source line and append its rows
        for line in source.split('\n') {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(line);

            let line_end = line_start + line.len() as u32;
            while let Some(row) = rows.get(row_index) {
                if row.span.start > line_end {
                    break;
                }

                let indent = line
                    .chars()
                    .take_while(|character| character.is_whitespace())
                    .collect::<String>();
                write!(output, "\n{indent}{}", row.text).expect("write pattern snapshot");
                row_index += 1;
            }

            line_start = line_end + 1;
        }

        output
    }

    /// Return one structural use's authored name.
    fn use_name<'a>(&'a self, pattern: &'a Pattern, use_entry: &MetavariableUse) -> &'a str {
        match use_entry.variable() {
            Some(variable) => {
                let variable = pattern.metavariables().get(variable);
                self.strings.get(variable.name())
            }
            None => "_",
        }
    }

    /// Render one metavariable declaration as row fields.
    fn metavariable_fields(metavariable: Metavariable) -> String {
        match metavariable {
            Metavariable::Node { node_type, .. } => {
                format!("kind=node node={node_type:?}")
            }
            Metavariable::Nodes { node_type, .. } => {
                format!("kind=nodes node={node_type:?}")
            }
            Metavariable::Name { .. } => "kind=name".to_string(),
        }
    }

    /// Render one structural use as row fields.
    fn use_fields(metavariable_use: MetavariableUse) -> String {
        match metavariable_use {
            MetavariableUse::Node { node, .. } => format!("kind=node node={:?}", node.ty),
            MetavariableUse::Nodes { node, .. } => format!("kind=nodes node={:?}", node.ty),
            MetavariableUse::Name {
                node, span_type, ..
            } => format!("kind=name node={:?} span={span_type:?}", node.ty),
        }
    }
}

/// One snapshot row anchored to authored source.
struct SnapshotRow {
    /// The source span anchoring the row.
    span: Span,
    /// The rendered tripleslash row.
    text: String,
}

impl SnapshotRow {
    /// Create one snapshot row.
    fn new(span: Span, text: String) -> Self {
        Self { span, text }
    }
}
