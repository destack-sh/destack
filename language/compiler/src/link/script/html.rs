use crate::Compiler;

impl Compiler {
    /// Build one minimal HTML document that references the linked script entry.
    pub(crate) fn linked_html_document(&self, entry_specifier: &str) -> String {
        format!(
            "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n</head>\n<body>\n<script type=\"module\" src=\"{entry_specifier}\"></script>\n</body>\n</html>\n"
        )
    }
}
