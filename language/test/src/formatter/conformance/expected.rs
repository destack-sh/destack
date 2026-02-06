use std::path::Path;

use super::runner::ExpectedOutput;

/// Load expected output content for a test case.
pub(super) fn load_expected_output(
    root: &Path,
    expected_output: &ExpectedOutput,
) -> Option<String> {
    match expected_output {
        ExpectedOutput::None => None,
        ExpectedOutput::PlainFile(path) => {
            let absolute_path = root.join(path);
            std::fs::read_to_string(absolute_path).ok()
        }
        ExpectedOutput::OxfmtSnapshot(path) => {
            let absolute_path = root.join(path);
            let snapshot = std::fs::read_to_string(absolute_path).ok()?;
            parse_oxfmt_snapshot_output(&snapshot)
        }
    }
}

/// Parse the output section from an oxfmt snapshot fixture.
fn parse_oxfmt_snapshot_output(snapshot: &str) -> Option<String> {
    let marker = "==================== Output ====================";
    let marker_index = snapshot.find(marker)?;
    let output = snapshot[marker_index + marker.len()..].trim_start_matches('\n');
    let mut lines = output.lines().peekable();

    // optional options block between dashed separators
    if let Some(first_line) = lines.peek()
        && is_separator_line(first_line)
    {
        lines.next();
        for line in lines.by_ref() {
            if is_separator_line(line) {
                break;
            }
        }
    }

    // capture first output variant only
    let mut output_lines = Vec::new();
    for line in lines {
        if line.starts_with("=====================") || is_separator_line(line) {
            break;
        }
        output_lines.push(line);
    }

    while output_lines.last().is_some_and(|line| line.is_empty()) {
        output_lines.pop();
    }

    let mut output = output_lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }

    Some(output)
}

/// Return whether a line is a dashed separator.
fn is_separator_line(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|character| character == '-')
}

#[cfg(test)]
mod tests {
    use super::parse_oxfmt_snapshot_output;

    #[test]
    fn test_parse_oxfmt_snapshot_output_with_options_block() {
        let snapshot = "==================== Output ====================\n------------------\n{ printWidth: 80 }\n------------------\nconst answer = 42;\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }

    #[test]
    fn test_parse_oxfmt_snapshot_output_without_options_block() {
        let snapshot = "==================== Output ====================\nconst answer = 42;\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }

    #[test]
    fn test_parse_oxfmt_snapshot_output_with_multiple_options_blocks() {
        let snapshot = "==================== Output ====================\n------------------\n{ printWidth: 80 }\n------------------\nconst answer = 42;\n\n-------------------\n{ printWidth: 100 }\n-------------------\nconst answer = 42;\n\n===================== End =====================\n";
        let output = parse_oxfmt_snapshot_output(snapshot).expect("output should parse");
        assert_eq!(output, "const answer = 42;\n");
    }
}
