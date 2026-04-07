use destack_source::{FileId, Span};

use crate::mdtest::validate_query_range_markers;

/// A cursor marker (`$0`, `$1`, etc.) in test source.
#[derive(Debug, Clone)]
pub struct CursorMarker {
    /// The cursor index.
    pub index: usize,
    /// The file that owns the cursor.
    pub file_id: FileId,
    /// The offset in the cleaned source.
    pub offset: u32,
}

/// A range marker (`^^^ name`) in test source.
#[derive(Debug, Clone)]
pub struct RangeMarker {
    /// The marker name.
    pub name: String,
    /// The span in the cleaned source.
    pub span: Span,
    /// The optional explicit target name.
    pub target: Option<String>,
}

/// Parsed markers from a test source file.
#[derive(Debug, Clone, Default)]
pub struct TestMarkers {
    /// The cursor markers in the file.
    pub cursors: Vec<CursorMarker>,
    /// The range markers in the file.
    pub ranges: Vec<RangeMarker>,
}

impl TestMarkers {
    /// Get a cursor by index.
    pub fn cursor(&self, index: usize) -> Option<&CursorMarker> {
        self.cursors.iter().find(|cursor| cursor.index == index)
    }

    /// Get a range marker by name.
    pub fn range(&self, name: &str) -> Option<&RangeMarker> {
        self.ranges.iter().find(|range| range.name == name)
    }
}

/// Parse test source and extract markers.
///
/// Returns the cleaned source without marker syntax and the extracted markers.
pub fn parse_markers(
    file_id: FileId,
    file_path: &str,
    source: &str,
) -> Result<(String, TestMarkers), String> {
    let mut markers = TestMarkers::default();
    let mut clean_lines = Vec::new();

    // track the cleaned start offset for the previous source line
    let mut previous_line_start = 0u32;

    for line in source.lines() {
        // strip caret marker lines from the cleaned source
        if let Some((column, length, name, target)) = parse_caret_marker(line) {
            let start = previous_line_start + column as u32;
            let end = start + length as u32;

            markers.ranges.push(RangeMarker {
                name,
                span: Span::new(file_id, start, end),
                target,
            });
            continue;
        }

        // strip cursor markers from the current source line
        let (clean_line, cursors) = extract_cursors(line);
        let line_start = clean_lines
            .iter()
            .map(|clean_line: &String| clean_line.len() as u32 + 1)
            .sum::<u32>();

        for (index, column) in cursors {
            markers.cursors.push(CursorMarker {
                index,
                file_id,
                offset: line_start + column as u32,
            });
        }

        previous_line_start = line_start;
        clean_lines.push(clean_line);
    }

    let clean_source = clean_lines.join("\n");
    let range_markers = markers
        .ranges
        .iter()
        .map(|range| {
            (
                range.name.as_str(),
                range.span.start as usize..range.span.end as usize,
            )
        })
        .collect::<Vec<_>>();

    validate_query_range_markers(file_path, &clean_source, &range_markers)?;

    Ok((clean_source, markers))
}

/// Parse a caret marker line.
///
/// Returns the column, caret length, marker name, and optional target.
fn parse_caret_marker(line: &str) -> Option<(usize, usize, String, Option<String>)> {
    // locate the trailing comment
    let comment_start = line.find("//")?;
    let comment = &line[comment_start + 2..];

    // locate the caret run inside the comment
    let caret_start = comment.find('^')?;
    let caret_length = comment[caret_start..]
        .chars()
        .take_while(|character| *character == '^')
        .count();

    if caret_length == 0 {
        return None;
    }

    // parse the marker payload after the carets
    let marker = comment[caret_start + caret_length..].trim();
    let (name, target) = if let Some(arrow) = marker.find("->") {
        (
            marker[..arrow].trim().to_string(),
            Some(marker[arrow + 2..].trim().to_string()),
        )
    } else {
        (marker.to_string(), None)
    };

    Some((comment_start + caret_start, caret_length, name, target))
}

/// Extract cursor markers from one source line.
fn extract_cursors(line: &str) -> (String, Vec<(usize, usize)>) {
    let mut clean_line = String::new();
    let mut cursors = Vec::new();
    let mut characters = line.chars().peekable();
    let mut column = 0usize;

    while let Some(character) = characters.next() {
        if character == '$'
            && let Some(next) = characters.peek()
            && next.is_ascii_digit()
        {
            let index =
                next.to_digit(10)
                    .unwrap_or_else(|| unreachable!("peeked ascii digit")) as usize;
            let _ = characters.next();
            cursors.push((index, column));
            continue;
        }

        clean_line.push(character);
        column += 1;
    }

    (clean_line, cursors)
}

#[cfg(test)]
mod tests {
    use destack_source::FileId;

    use super::parse_markers;

    #[test]
    fn test_parse_simple_markers() {
        let source = r#"
const foo = 1;
//      ^^^ def:foo
const bar = foo;
//            ^^^ use:foo -> def:foo
"#;
        let file_id = FileId(0);
        let (clean, markers) = parse_markers(file_id, "test.ds", source).expect("markers");

        assert!(!clean.contains("^^^"));
        assert_eq!(markers.ranges.len(), 2);
        assert_eq!(markers.ranges[0].name, "def:foo");
        assert_eq!(markers.ranges[1].name, "use:foo");
        assert_eq!(markers.ranges[1].target, Some("def:foo".to_string()));
    }

    #[test]
    fn test_parse_cursor_markers() {
        let source = "point.$0x";
        let file_id = FileId(0);
        let (clean, markers) = parse_markers(file_id, "test.ds", source).expect("markers");

        assert_eq!(clean, "point.x");
        assert_eq!(markers.cursors.len(), 1);
        assert_eq!(markers.cursors[0].index, 0);
        assert_eq!(markers.cursors[0].file_id, file_id);
        assert_eq!(markers.cursors[0].offset, 6);
    }

    #[test]
    fn test_reject_partial_identifier_range() {
        let source = r#"
const value = 1;
//      ^^^^ use:value
"#;
        let error = parse_markers(FileId(0), "test.ds", source)
            .expect_err("expected strict anchor failure");

        assert!(error.contains("must exactly cover word token `value`"));
    }

    #[test]
    fn test_parse_column_zero_marker() {
        let source = r#"
stableLater();
//^^^^^^^^^^^ use:stableLater
"#;
        let file_id = FileId(0);
        let (_, markers) = parse_markers(file_id, "test.ds", source).expect("markers");

        assert_eq!(markers.ranges.len(), 1);
        assert_eq!(markers.ranges[0].span.start, 1);
        assert_eq!(markers.ranges[0].span.end, 12);
    }
}
