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
