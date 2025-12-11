use std::collections::HashSet;
use std::path::Path;

pub fn load_expected_failures(path: &Path) -> HashSet<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

pub fn save_expected_failures(path: &Path, failures: &HashSet<String>) -> std::io::Result<()> {
    let mut sorted: Vec<_> = failures.iter().cloned().collect();
    sorted.sort();
    let content = sorted.join("\n") + "\n";
    std::fs::write(path, content)
}
