/// Render generated documentation blocks.
pub(super) struct GeneratedDocumentation;

impl GeneratedDocumentation {
    /// Render one documentation block for generated output.
    pub(super) fn render(documentation: Option<&str>, fallback: &str) -> String {
        let Some(documentation) = documentation else {
            return format!("/// {fallback}\n");
        };

        let lines = documentation
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>();
        let first_non_empty = lines.iter().position(|line| !line.trim().is_empty());
        let last_non_empty = lines.iter().rposition(|line| !line.trim().is_empty());
        let Some(first_non_empty) = first_non_empty else {
            return format!("/// {fallback}\n");
        };
        let last_non_empty = last_non_empty.unwrap_or(first_non_empty);

        let lines = lines[first_non_empty..=last_non_empty].to_vec();
        let mut normalized = Vec::with_capacity(lines.len() + 8);
        let mut saw_title = false;

        for line in lines {
            let is_empty = line.trim().is_empty();
            let is_heading = line.trim_start().starts_with('#');
            let last_is_empty = normalized
                .last()
                .is_some_and(|last: &String| last.trim().is_empty());

            // title spacing
            if !saw_title && !is_empty {
                normalized.push(line);
                saw_title = true;
                continue;
            }

            // body and section spacing
            if !is_empty && !last_is_empty {
                let is_description = !is_heading;
                let after_title = normalized.len() == 1;
                let starts_section = is_heading;
                if is_description && after_title {
                    normalized.push(String::new());
                } else if starts_section {
                    normalized.push(String::new());
                }
            }

            normalized.push(line);
        }

        let mut docs = String::new();
        for line in normalized {
            if line.trim().is_empty() {
                docs.push_str("///\n");
            } else {
                docs.push_str(&format!("/// {line}\n"));
            }
        }

        if docs.is_empty() {
            return format!("/// {fallback}\n");
        }

        docs
    }
}
