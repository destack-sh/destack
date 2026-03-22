use crate::core::CaseResult;

/// Normalize a snapshot expectation into a newline separated string.
pub fn normalize_expected_snapshot(expected: &str) -> String {
    // filter empty lines and trim trailing whitespace
    expected
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Compare one actual snapshot against the expected snapshot.
pub fn compare_snapshot(label: &str, actual: &str, expected: &str) -> CaseResult {
    let actual_snapshot = normalize_expected_snapshot(actual);
    let expected_snapshot = normalize_expected_snapshot(expected);

    if actual_snapshot == expected_snapshot {
        return CaseResult::Passed;
    }

    CaseResult::Failed {
        message: format!(
            "{label} snapshot mismatch\n\nexpected:\n{expected_snapshot}\n\nactual:\n{actual_snapshot}"
        ),
    }
}

/// Compare one actual snapshot line list against the expected snapshot.
pub fn compare_snapshot_lines(label: &str, actual_lines: &[String], expected: &str) -> CaseResult {
    compare_snapshot(label, &actual_lines.join("\n"), expected)
}

/// Decide whether an expectation is snapshot shaped.
pub fn looks_like_snapshot(expected: &str, markers: &[&str]) -> bool {
    expected
        .lines()
        .map(str::trim)
        .any(|line| markers.iter().any(|marker| line.contains(marker)))
}

/// Decide whether an expectation is snapshot shaped and span like.
pub fn looks_like_span_snapshot(expected: &str, markers: &[&str]) -> bool {
    expected.lines().map(str::trim).any(|line| {
        let has_line_range = line.split_once('-').is_some_and(|(start, end)| {
            !start.is_empty()
                && !end.is_empty()
                && start.chars().all(|character| character.is_ascii_digit())
                && end.chars().all(|character| character.is_ascii_digit())
        });
        let has_position = line
            .split_once(':')
            .and_then(|(_, tail)| tail.chars().next())
            .is_some_and(|character| character.is_ascii_digit());

        has_line_range || has_position || markers.iter().any(|marker| line.contains(marker))
    })
}

/// Parse one optional `top:` snapshot directive from the first non empty line.
pub fn parse_snapshot_top_directive(content: &str) -> (Option<usize>, String) {
    let mut top_limit = None;
    let mut lines = Vec::new();
    let mut directive_consumed = false;

    // parse the first non empty line as a potential top directive
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !directive_consumed && let Some(rest) = trimmed.strip_prefix("top:") {
            top_limit = rest.trim().parse::<usize>().ok();
            directive_consumed = true;
            continue;
        }

        directive_consumed = true;
        lines.push(trimmed.to_string());
    }

    (top_limit, lines.join("\n"))
}
