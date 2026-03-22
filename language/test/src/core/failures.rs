use std::collections::HashSet;
use std::path::Path;

/// Load case names from one failures file:
/// - `# full line comments`
/// - `case-name # inline comments`
/// - blank lines (ignored)
pub fn load_expected_failures(path: &Path) -> HashSet<String> {
    // treat missing files as an empty baseline
    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                // inline comments
                match line.find('#') {
                    Some(idx) => line[..idx].trim(),
                    None => line,
                }
            })
            .filter(|line| !line.is_empty())
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Save case names to one failures file.
pub fn save_expected_failures(path: &Path, failures: &HashSet<String>) -> std::io::Result<()> {
    // keep the file stable for readable diffs
    let mut sorted: Vec<_> = failures.iter().cloned().collect();
    sorted.sort();

    // terminate the file with one trailing newline
    let content = sorted.join("\n") + "\n";
    std::fs::write(path, content)
}
